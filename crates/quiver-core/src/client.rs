//! Flight SQL client wrapper.
//!
//! Wraps [`FlightSqlServiceClient`] with connection-profile-aware
//! construction, authentication, and typed error mapping.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use arrow::array::RecordBatch;
use arrow_flight::decode::FlightRecordBatchStream;
use arrow_flight::sql::client::FlightSqlServiceClient;
use arrow_flight::sql::{
    CommandGetCrossReference, CommandGetDbSchemas, CommandGetExportedKeys, CommandGetImportedKeys,
    CommandGetPrimaryKeys, CommandGetTables, CommandGetXdbcTypeInfo, SqlInfo,
};
use arrow_schema::SchemaRef;
use futures::TryStreamExt;
use tonic::transport::Channel;

use crate::connection::{AuthMethod, ConnectionProfile, ConnectionState};

// ── Query result ──────────────────────────────────────────────

/// The outcome of a SQL query execution.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub batches: Vec<RecordBatch>,
    pub schema: SchemaRef,
    pub total_rows: usize,
    pub elapsed: Duration,
}

// ── Flight SQL client ─────────────────────────────────────────

/// High-level Flight SQL client.
///
/// Wraps `arrow_flight::sql::FlightSqlServiceClient<Channel>` with
/// profile-based connection, authentication, and convenience methods
/// that cover the full Flight SQL command surface.
#[derive(Debug)]
pub struct FlightClient {
    inner: FlightSqlServiceClient<Channel>,
    profile: ConnectionProfile,
    state: ConnectionState,
}

impl FlightClient {
    /// Connect to a Flight SQL server described by `profile`.
    ///
    /// Performs gRPC channel setup and, when the profile specifies
    /// authentication, runs handshake / sets bearer token before
    /// returning.
    pub async fn connect(profile: &ConnectionProfile) -> Result<Self> {
        let uri = profile.endpoint_uri();
        let channel = Channel::from_shared(uri.clone())
            .with_context(|| format!("invalid endpoint URI: {uri}"))?
            .connect()
            .await
            .with_context(|| format!("failed to connect to {uri}"))?;

        let mut inner = FlightSqlServiceClient::new(channel);

        match &profile.auth {
            AuthMethod::None => {}
            AuthMethod::Basic { username, password } => {
                inner
                    .handshake(username, password)
                    .await
                    .context("basic-auth handshake failed")?;
            }
            AuthMethod::BearerToken { token } => {
                inner.set_token(token.clone());
            }
        }

        Ok(Self {
            inner,
            profile: profile.clone(),
            state: ConnectionState::Connected,
        })
    }

    /// Build from an already-established inner client (useful for tests).
    pub fn from_inner(inner: FlightSqlServiceClient<Channel>, profile: ConnectionProfile) -> Self {
        Self {
            inner,
            profile,
            state: ConnectionState::Connected,
        }
    }

    // ── Accessors ─────────────────────────────────────────────

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn profile(&self) -> &ConnectionProfile {
        &self.profile
    }

    pub fn inner_mut(&mut self) -> &mut FlightSqlServiceClient<Channel> {
        &mut self.inner
    }

    // ── Query execution ───────────────────────────────────────

    /// Execute a SQL query and collect all result batches.
    pub async fn execute_query(&mut self, sql: &str) -> Result<QueryResult> {
        let start = Instant::now();

        let flight_info = self
            .inner
            .execute(sql.to_string(), None)
            .await
            .context("execute failed")?;

        let mut batches = Vec::new();
        let mut schema: Option<SchemaRef> = None;

        for endpoint in &flight_info.endpoint {
            let ticket = endpoint
                .ticket
                .as_ref()
                .context("endpoint missing ticket")?;

            let stream: FlightRecordBatchStream = self
                .inner
                .do_get(ticket.clone())
                .await
                .context("do_get failed")?;

            let collected: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("failed to collect record batches")?;

            if schema.is_none() {
                if let Some(first) = collected.first() {
                    schema = Some(first.schema());
                }
            }
            batches.extend(collected);
        }

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        let resolved_schema = schema.unwrap_or_else(|| Arc::new(arrow_schema::Schema::empty()));

        Ok(QueryResult {
            batches,
            schema: resolved_schema,
            total_rows,
            elapsed: start.elapsed(),
        })
    }

    /// Execute a SQL update (INSERT/UPDATE/DELETE/DDL) and return
    /// the number of affected rows.
    pub async fn execute_update(&mut self, sql: &str) -> Result<i64> {
        self.inner
            .execute_update(sql.to_string(), None)
            .await
            .context("execute_update failed")
    }

    // ── Prepared statements ───────────────────────────────────

    /// Create a prepared statement and execute it, returning results.
    pub async fn execute_prepared(&mut self, sql: &str) -> Result<QueryResult> {
        let start = Instant::now();

        let mut stmt = self
            .inner
            .prepare(sql.to_string(), None)
            .await
            .context("prepare failed")?;

        let flight_info = stmt.execute().await.context("prepared execute failed")?;

        let mut batches = Vec::new();
        let mut schema: Option<SchemaRef> = None;

        for endpoint in &flight_info.endpoint {
            let ticket = endpoint
                .ticket
                .as_ref()
                .context("endpoint missing ticket")?;

            let stream = self
                .inner
                .do_get(ticket.clone())
                .await
                .context("do_get on prepared stmt failed")?;

            let collected: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("failed to collect prepared stmt batches")?;

            if schema.is_none() {
                if let Some(first) = collected.first() {
                    schema = Some(first.schema());
                }
            }
            batches.extend(collected);
        }

        stmt.close().await.context("closing prepared stmt failed")?;

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        let resolved_schema = schema.unwrap_or_else(|| Arc::new(arrow_schema::Schema::empty()));

        Ok(QueryResult {
            batches,
            schema: resolved_schema,
            total_rows,
            elapsed: start.elapsed(),
        })
    }

    // ── Catalog introspection ─────────────────────────────────

    /// Retrieve the list of catalogs from the server.
    pub async fn get_catalogs(&mut self) -> Result<Vec<RecordBatch>> {
        let info = self
            .inner
            .get_catalogs()
            .await
            .context("get_catalogs failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve database schemas, optionally filtered.
    pub async fn get_db_schemas(
        &mut self,
        catalog: Option<String>,
        schema_filter: Option<String>,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetDbSchemas {
            catalog,
            db_schema_filter_pattern: schema_filter,
        };
        let info = self
            .inner
            .get_db_schemas(cmd)
            .await
            .context("get_db_schemas failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve tables, optionally filtered.
    pub async fn get_tables(
        &mut self,
        catalog: Option<String>,
        schema_filter: Option<String>,
        table_filter: Option<String>,
        table_types: Vec<String>,
        include_schema: bool,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetTables {
            catalog,
            db_schema_filter_pattern: schema_filter,
            table_name_filter_pattern: table_filter,
            table_types,
            include_schema,
        };
        let info = self
            .inner
            .get_tables(cmd)
            .await
            .context("get_tables failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve table types supported by the server.
    pub async fn get_table_types(&mut self) -> Result<Vec<RecordBatch>> {
        let info = self
            .inner
            .get_table_types()
            .await
            .context("get_table_types failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve SQL metadata from the server.
    pub async fn get_sql_info(&mut self, infos: Vec<SqlInfo>) -> Result<Vec<RecordBatch>> {
        let info = self
            .inner
            .get_sql_info(infos)
            .await
            .context("get_sql_info failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve primary keys for a table.
    pub async fn get_primary_keys(
        &mut self,
        catalog: Option<String>,
        schema: Option<String>,
        table: String,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetPrimaryKeys {
            catalog,
            db_schema: schema,
            table,
        };
        let info = self
            .inner
            .get_primary_keys(cmd)
            .await
            .context("get_primary_keys failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve exported keys (foreign keys that reference a table's primary key).
    pub async fn get_exported_keys(
        &mut self,
        catalog: Option<String>,
        schema: Option<String>,
        table: String,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetExportedKeys {
            catalog,
            db_schema: schema,
            table,
        };
        let info = self
            .inner
            .get_exported_keys(cmd)
            .await
            .context("get_exported_keys failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve imported keys (foreign keys in the given table).
    pub async fn get_imported_keys(
        &mut self,
        catalog: Option<String>,
        schema: Option<String>,
        table: String,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetImportedKeys {
            catalog,
            db_schema: schema,
            table,
        };
        let info = self
            .inner
            .get_imported_keys(cmd)
            .await
            .context("get_imported_keys failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve cross-reference (foreign key relationships between two tables).
    pub async fn get_cross_reference(
        &mut self,
        pk_catalog: Option<String>,
        pk_schema: Option<String>,
        pk_table: String,
        fk_catalog: Option<String>,
        fk_schema: Option<String>,
        fk_table: String,
    ) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetCrossReference {
            pk_catalog,
            pk_db_schema: pk_schema,
            pk_table,
            fk_catalog,
            fk_db_schema: fk_schema,
            fk_table,
        };
        let info = self
            .inner
            .get_cross_reference(cmd)
            .await
            .context("get_cross_reference failed")?;
        self.do_get_endpoints(&info).await
    }

    /// Retrieve XDBC type information.
    pub async fn get_xdbc_type_info(&mut self, data_type: Option<i32>) -> Result<Vec<RecordBatch>> {
        let cmd = CommandGetXdbcTypeInfo { data_type };
        let info = self
            .inner
            .get_xdbc_type_info(cmd)
            .await
            .context("get_xdbc_type_info failed")?;
        self.do_get_endpoints(&info).await
    }

    // ── Transactions ──────────────────────────────────────────

    /// Begin a new transaction and return the transaction ID.
    pub async fn begin_transaction(&mut self) -> Result<bytes::Bytes> {
        self.inner
            .begin_transaction()
            .await
            .context("begin_transaction failed")
    }

    /// Commit or rollback a transaction.
    pub async fn end_transaction(
        &mut self,
        transaction_id: bytes::Bytes,
        action: arrow_flight::sql::EndTransaction,
    ) -> Result<()> {
        self.inner
            .end_transaction(transaction_id, action)
            .await
            .context("end_transaction failed")
    }

    // ── Custom headers ────────────────────────────────────────

    /// Set a custom header that will be sent with every request.
    pub fn set_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.inner.set_header(key, value);
    }

    // ── Graceful close ────────────────────────────────────────

    /// Close the client connection.
    pub async fn close(&mut self) -> Result<()> {
        self.inner.close().await.context("close failed")?;
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────

    /// Fetch all RecordBatches from every endpoint in a FlightInfo.
    async fn do_get_endpoints(
        &mut self,
        info: &arrow_flight::FlightInfo,
    ) -> Result<Vec<RecordBatch>> {
        let mut batches = Vec::new();
        for endpoint in &info.endpoint {
            let ticket = endpoint
                .ticket
                .as_ref()
                .context("endpoint missing ticket")?;
            let stream = self
                .inner
                .do_get(ticket.clone())
                .await
                .context("do_get failed")?;
            let collected: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("stream collect failed")?;
            batches.extend(collected);
        }
        Ok(batches)
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::sync::Arc;

    use std::str::FromStr;

    use arrow::array::{ArrayRef, Int32Array, StringArray};
    use arrow_flight::encode::FlightDataEncoderBuilder;
    use arrow_flight::flight_service_server::{FlightService, FlightServiceServer};
    use arrow_flight::sql::metadata::{
        SqlInfoData, SqlInfoDataBuilder, XdbcTypeInfo, XdbcTypeInfoData, XdbcTypeInfoDataBuilder,
    };
    use arrow_flight::sql::server::{FlightSqlService, PeekableFlightDataStream};
    use arrow_flight::sql::{
        ActionBeginTransactionRequest, ActionBeginTransactionResult,
        ActionClosePreparedStatementRequest, ActionCreatePreparedStatementRequest,
        ActionCreatePreparedStatementResult, ActionEndTransactionRequest, CommandGetCatalogs,
        CommandGetCrossReference, CommandGetDbSchemas, CommandGetExportedKeys,
        CommandGetImportedKeys, CommandGetPrimaryKeys, CommandGetSqlInfo, CommandGetTableTypes,
        CommandGetTables, CommandGetXdbcTypeInfo, CommandPreparedStatementQuery,
        CommandPreparedStatementUpdate, CommandStatementIngest, CommandStatementQuery,
        CommandStatementUpdate, DoPutPreparedStatementResult, Nullable, ProstMessageExt,
        Searchable, SqlInfo, TicketStatementQuery, XdbcDataType,
    };
    use arrow_flight::utils::batches_to_flight_data;
    use arrow_flight::{
        Action, FlightDescriptor, FlightEndpoint, FlightInfo, HandshakeRequest, HandshakeResponse,
        IpcMessage, SchemaAsIpc, Ticket,
    };
    use arrow_ipc::writer::IpcWriteOptions;
    use arrow_schema::{DataType, Field, Schema};
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use core::str;
    use futures::{stream, Stream};
    use once_cell::sync::Lazy;
    use prost::Message;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use tonic::metadata::MetadataValue;
    use tonic::{Request, Response, Status, Streaming};
    use uuid::Uuid;

    // ══════════════════════════════════════════════════════════
    //  Mock Flight SQL Server
    // ══════════════════════════════════════════════════════════
    //
    // A comprehensive in-process server implementing the full
    // FlightSqlService trait.  Each method either returns
    // deterministic test data or `Status::unimplemented`.
    // The update counter and transaction store are wrapped in
    // Arc<Mutex<>> so tests can inspect server-side state.

    const FAKE_TOKEN: &str = "test-bearer-token";

    static SQL_INFO: Lazy<SqlInfoData> = Lazy::new(|| {
        let mut b = SqlInfoDataBuilder::new();
        b.append(SqlInfo::FlightSqlServerName, "QuiverTestServer");
        b.append(SqlInfo::FlightSqlServerVersion, "0.1.0");
        b.append(SqlInfo::FlightSqlServerArrowVersion, "1.3");
        b.build().unwrap()
    });

    static XDBC_INFO: Lazy<XdbcTypeInfoData> = Lazy::new(|| {
        let mut b = XdbcTypeInfoDataBuilder::new();
        b.append(XdbcTypeInfo {
            type_name: "INTEGER".into(),
            data_type: XdbcDataType::XdbcInteger,
            column_size: Some(32),
            literal_prefix: None,
            literal_suffix: None,
            create_params: None,
            nullable: Nullable::NullabilityNullable,
            case_sensitive: false,
            searchable: Searchable::Full,
            unsigned_attribute: Some(false),
            fixed_prec_scale: false,
            auto_increment: Some(false),
            local_type_name: Some("INTEGER".into()),
            minimum_scale: None,
            maximum_scale: None,
            sql_data_type: XdbcDataType::XdbcInteger,
            datetime_subcode: None,
            num_prec_radix: Some(2),
            interval_precision: None,
        });
        b.build().unwrap()
    });

    /// Tables seeded in the mock server: "catalog.schema.table"
    static MOCK_TABLES: Lazy<Vec<&'static str>> = Lazy::new(|| {
        vec![
            "production.public.users",
            "production.public.orders",
            "analytics.reporting.daily_summary",
        ]
    });

    #[derive(Clone)]
    struct MockFlightSqlServer {
        /// Tracks active transactions.
        transactions: Arc<Mutex<HashSet<String>>>,
        /// Counts DML updates.
        update_count: Arc<Mutex<i64>>,
    }

    impl MockFlightSqlServer {
        fn new() -> Self {
            Self {
                transactions: Arc::new(Mutex::new(HashSet::new())),
                update_count: Arc::new(Mutex::new(0)),
            }
        }

        fn service(&self) -> FlightServiceServer<Self> {
            FlightServiceServer::new(self.clone())
        }

        fn fake_query_batch() -> RecordBatch {
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int32, false),
                Field::new("name", DataType::Utf8, false),
            ]));
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(Int32Array::from(vec![1, 2, 3])) as ArrayRef,
                    Arc::new(StringArray::from(vec!["alice", "bob", "carol"])) as ArrayRef,
                ],
            )
            .unwrap()
        }

        fn check_token<T>(req: &Request<T>) -> Result<(), Status> {
            if let Some(auth) = req.metadata().get("authorization") {
                let val = auth
                    .to_str()
                    .map_err(|e| Status::internal(format!("bad auth header: {e}")))?;
                if val == format!("Bearer {FAKE_TOKEN}") {
                    return Ok(());
                }
            }
            // Allow unauthenticated for methods that don't enforce it
            Ok(())
        }

        fn primary_keys_schema() -> Schema {
            Schema::new(vec![
                Field::new("catalog_name", DataType::Utf8, true),
                Field::new("db_schema_name", DataType::Utf8, true),
                Field::new("table_name", DataType::Utf8, false),
                Field::new("column_name", DataType::Utf8, false),
                Field::new("key_name", DataType::Utf8, true),
                Field::new("key_sequence", DataType::Int32, false),
            ])
        }

        fn fk_schema() -> Schema {
            Schema::new(vec![
                Field::new("pk_catalog_name", DataType::Utf8, true),
                Field::new("pk_db_schema_name", DataType::Utf8, true),
                Field::new("pk_table_name", DataType::Utf8, false),
                Field::new("pk_column_name", DataType::Utf8, false),
                Field::new("fk_catalog_name", DataType::Utf8, true),
                Field::new("fk_db_schema_name", DataType::Utf8, true),
                Field::new("fk_table_name", DataType::Utf8, false),
                Field::new("fk_column_name", DataType::Utf8, false),
                Field::new("key_sequence", DataType::Int32, false),
                Field::new("fk_key_name", DataType::Utf8, true),
                Field::new("pk_key_name", DataType::Utf8, true),
                Field::new("update_rule", DataType::UInt8, false),
                Field::new("delete_rule", DataType::UInt8, false),
            ])
        }

        fn empty_fk_batch() -> RecordBatch {
            let schema = Arc::new(Self::fk_schema());
            RecordBatch::new_empty(schema)
        }
    }

    #[tonic::async_trait]
    impl FlightSqlService for MockFlightSqlServer {
        type FlightService = MockFlightSqlServer;

        // ── Handshake (Basic auth) ────────────────────────

        async fn do_handshake(
            &self,
            request: Request<Streaming<HandshakeRequest>>,
        ) -> Result<
            Response<Pin<Box<dyn Stream<Item = Result<HandshakeResponse, Status>> + Send>>>,
            Status,
        > {
            let auth = request
                .metadata()
                .get("authorization")
                .ok_or_else(|| Status::unauthenticated("missing authorization"))?
                .to_str()
                .map_err(|e| Status::internal(format!("bad header: {e}")))?
                .to_string();

            if !auth.starts_with("Basic ") {
                return Err(Status::unauthenticated("expected Basic auth"));
            }
            let decoded = BASE64_STANDARD
                .decode(&auth["Basic ".len()..])
                .map_err(|e| Status::internal(format!("base64 error: {e}")))?;
            let creds = str::from_utf8(&decoded)
                .map_err(|e| Status::internal(format!("utf8 error: {e}")))?;
            let parts: Vec<&str> = creds.splitn(2, ':').collect();
            if parts.len() != 2 || parts[0] != "admin" || parts[1] != "password" {
                return Err(Status::unauthenticated("bad credentials"));
            }

            let resp = HandshakeResponse {
                protocol_version: 0,
                payload: FAKE_TOKEN.into(),
            };
            let output = stream::iter(vec![Ok(resp)]);
            let token = format!("Bearer {FAKE_TOKEN}");
            let mut response: Response<Pin<Box<dyn Stream<Item = _> + Send>>> =
                Response::new(Box::pin(output));
            response
                .metadata_mut()
                .append("authorization", MetadataValue::from_str(&token).unwrap());
            Ok(response)
        }

        // ── Execute query (get_flight_info_statement) ─────

        async fn get_flight_info_statement(
            &self,
            query: CommandStatementQuery,
            _request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let batch = Self::fake_query_batch();
            let schema = (*batch.schema()).clone();

            let handle = format!("stmt-{}", query.query.len());
            let tsq = TicketStatementQuery {
                statement_handle: handle.into(),
            };
            let ticket = Ticket::new(tsq.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);

            let info = FlightInfo::new()
                .try_with_schema(&schema)
                .map_err(|e| Status::internal(format!("schema error: {e}")))?
                .with_endpoint(endpoint)
                .with_total_records(batch.num_rows() as i64)
                .with_ordered(false);

            Ok(Response::new(info))
        }

        // ── do_get (returns fake query data) ──────────────

        async fn do_get_fallback(
            &self,
            _request: Request<Ticket>,
            _message: arrow_flight::sql::Any,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::fake_query_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("encode error: {e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        async fn do_get_statement(
            &self,
            _ticket: TicketStatementQuery,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::fake_query_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("encode error: {e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── DML / DDL (execute_update) ────────────────────

        async fn do_put_statement_update(
            &self,
            _ticket: CommandStatementUpdate,
            _request: Request<PeekableFlightDataStream>,
        ) -> Result<i64, Status> {
            let mut count = self.update_count.lock().await;
            *count += 1;
            Ok(*count)
        }

        // ── Catalog: get_catalogs ─────────────────────────

        async fn get_flight_info_catalogs(
            &self,
            query: CommandGetCatalogs,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&query.into_builder().schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_catalogs(
            &self,
            query: CommandGetCatalogs,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let names: HashSet<String> = MOCK_TABLES
                .iter()
                .map(|t| t.split('.').next().unwrap().to_string())
                .collect();
            let mut builder = query.into_builder();
            for name in names {
                builder.append(name);
            }
            let schema = builder.schema();
            let batch = builder.build();
            let stream = FlightDataEncoderBuilder::new()
                .with_schema(schema)
                .build(futures::stream::once(async { batch }))
                .map_err(Status::from);
            Ok(Response::new(Box::pin(stream)))
        }

        // ── Catalog: get_db_schemas ───────────────────────

        async fn get_flight_info_schemas(
            &self,
            query: CommandGetDbSchemas,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&query.into_builder().schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_schemas(
            &self,
            query: CommandGetDbSchemas,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let pairs: HashSet<(String, String)> = MOCK_TABLES
                .iter()
                .map(|t| {
                    let p: Vec<&str> = t.split('.').collect();
                    (p[0].to_string(), p[1].to_string())
                })
                .collect();
            let mut builder = query.into_builder();
            for (cat, sch) in pairs {
                builder.append(cat, sch);
            }
            let schema = builder.schema();
            let batch = builder.build();
            let stream = FlightDataEncoderBuilder::new()
                .with_schema(schema)
                .build(futures::stream::once(async { batch }))
                .map_err(Status::from);
            Ok(Response::new(Box::pin(stream)))
        }

        // ── Catalog: get_tables ───────────────────────────

        async fn get_flight_info_tables(
            &self,
            query: CommandGetTables,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&query.into_builder().schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_tables(
            &self,
            query: CommandGetTables,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let tuples: HashSet<(String, String, String)> = MOCK_TABLES
                .iter()
                .map(|t| {
                    let p: Vec<&str> = t.split('.').collect();
                    (p[0].to_string(), p[1].to_string(), p[2].to_string())
                })
                .collect();
            let dummy_schema = Schema::empty();
            let mut builder = query.into_builder();
            for (cat, sch, tbl) in tuples {
                builder
                    .append(cat, sch, tbl, "TABLE", &dummy_schema)
                    .map_err(Status::from)?;
            }
            let schema = builder.schema();
            let batch = builder.build();
            let stream = FlightDataEncoderBuilder::new()
                .with_schema(schema)
                .build(futures::stream::once(async { batch }))
                .map_err(Status::from);
            Ok(Response::new(Box::pin(stream)))
        }

        // ── Catalog: get_table_types ──────────────────────

        async fn get_flight_info_table_types(
            &self,
            query: CommandGetTableTypes,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&Schema::new(vec![Field::new(
                    "table_type",
                    DataType::Utf8,
                    false,
                )]))
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_table_types(
            &self,
            _query: CommandGetTableTypes,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let schema = Arc::new(Schema::new(vec![Field::new(
                "table_type",
                DataType::Utf8,
                false,
            )]));
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(StringArray::from(vec!["TABLE", "VIEW"])) as ArrayRef],
            )
            .unwrap();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── Catalog: get_sql_info ─────────────────────────

        async fn get_flight_info_sql_info(
            &self,
            query: CommandGetSqlInfo,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(query.into_builder(&SQL_INFO).schema().as_ref())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_sql_info(
            &self,
            query: CommandGetSqlInfo,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let builder = query.into_builder(&SQL_INFO);
            let schema = builder.schema();
            let batch = builder.build();
            let stream = FlightDataEncoderBuilder::new()
                .with_schema(schema)
                .build(futures::stream::once(async { batch }))
                .map_err(Status::from);
            Ok(Response::new(Box::pin(stream)))
        }

        // ── Catalog: get_primary_keys ─────────────────────

        async fn get_flight_info_primary_keys(
            &self,
            query: CommandGetPrimaryKeys,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&Self::primary_keys_schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_primary_keys(
            &self,
            _query: CommandGetPrimaryKeys,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let schema = Arc::new(Self::primary_keys_schema());
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(StringArray::from(vec![Some("production")])) as ArrayRef,
                    Arc::new(StringArray::from(vec![Some("public")])) as ArrayRef,
                    Arc::new(StringArray::from(vec!["users"])) as ArrayRef,
                    Arc::new(StringArray::from(vec!["id"])) as ArrayRef,
                    Arc::new(StringArray::from(vec![Some("pk_users")])) as ArrayRef,
                    Arc::new(Int32Array::from(vec![1])) as ArrayRef,
                ],
            )
            .map_err(|e| Status::internal(format!("batch build: {e}")))?;
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── Catalog: get_exported_keys ────────────────────

        async fn get_flight_info_exported_keys(
            &self,
            query: CommandGetExportedKeys,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&Self::fk_schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_exported_keys(
            &self,
            _query: CommandGetExportedKeys,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::empty_fk_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── Catalog: get_imported_keys ────────────────────

        async fn get_flight_info_imported_keys(
            &self,
            query: CommandGetImportedKeys,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&Self::fk_schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_imported_keys(
            &self,
            _query: CommandGetImportedKeys,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::empty_fk_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── Catalog: get_cross_reference ──────────────────

        async fn get_flight_info_cross_reference(
            &self,
            query: CommandGetCrossReference,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&Self::fk_schema())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_cross_reference(
            &self,
            _query: CommandGetCrossReference,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::empty_fk_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        // ── Catalog: get_xdbc_type_info ───────────────────

        async fn get_flight_info_xdbc_type_info(
            &self,
            query: CommandGetXdbcTypeInfo,
            request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let fd = request.into_inner();
            let ticket = Ticket::new(query.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(query.into_builder(&XDBC_INFO).schema().as_ref())
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_descriptor(fd);
            Ok(Response::new(info))
        }

        async fn do_get_xdbc_type_info(
            &self,
            query: CommandGetXdbcTypeInfo,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let builder = query.into_builder(&XDBC_INFO);
            let schema = builder.schema();
            let batch = builder.build();
            let stream = FlightDataEncoderBuilder::new()
                .with_schema(schema)
                .build(futures::stream::once(async { batch }))
                .map_err(Status::from);
            Ok(Response::new(Box::pin(stream)))
        }

        // ── Prepared statements ───────────────────────────

        async fn do_action_create_prepared_statement(
            &self,
            _query: ActionCreatePreparedStatementRequest,
            _request: Request<Action>,
        ) -> Result<ActionCreatePreparedStatementResult, Status> {
            let batch = Self::fake_query_batch();
            let schema = batch.schema();
            let message: IpcMessage = SchemaAsIpc::new(&schema, &IpcWriteOptions::default())
                .try_into()
                .map_err(|e| Status::internal(format!("schema encode: {e}")))?;
            let IpcMessage(schema_bytes) = message;
            Ok(ActionCreatePreparedStatementResult {
                prepared_statement_handle: b"test-prepared-handle".to_vec().into(),
                dataset_schema: schema_bytes,
                parameter_schema: Default::default(),
            })
        }

        async fn get_flight_info_prepared_statement(
            &self,
            cmd: CommandPreparedStatementQuery,
            _request: Request<FlightDescriptor>,
        ) -> Result<Response<FlightInfo>, Status> {
            let batch = Self::fake_query_batch();
            let schema = (*batch.schema()).clone();
            let ticket = Ticket::new(cmd.as_any().encode_to_vec());
            let endpoint = FlightEndpoint::new().with_ticket(ticket);
            let info = FlightInfo::new()
                .try_with_schema(&schema)
                .map_err(|e| Status::internal(format!("{e}")))?
                .with_endpoint(endpoint)
                .with_total_records(batch.num_rows() as i64);
            Ok(Response::new(info))
        }

        async fn do_get_prepared_statement(
            &self,
            _query: CommandPreparedStatementQuery,
            _request: Request<Ticket>,
        ) -> Result<Response<<Self as FlightService>::DoGetStream>, Status> {
            let batch = Self::fake_query_batch();
            let schema = batch.schema();
            let flight_data = batches_to_flight_data(&schema, vec![batch])
                .map_err(|e| Status::internal(format!("{e}")))?
                .into_iter()
                .map(Ok);
            Ok(Response::new(Box::pin(stream::iter(flight_data))))
        }

        async fn do_action_close_prepared_statement(
            &self,
            _query: ActionClosePreparedStatementRequest,
            _request: Request<Action>,
        ) -> Result<(), Status> {
            Ok(())
        }

        async fn do_put_prepared_statement_query(
            &self,
            _query: CommandPreparedStatementQuery,
            _request: Request<PeekableFlightDataStream>,
        ) -> Result<DoPutPreparedStatementResult, Status> {
            Ok(DoPutPreparedStatementResult {
                prepared_statement_handle: None,
            })
        }

        async fn do_put_prepared_statement_update(
            &self,
            _query: CommandPreparedStatementUpdate,
            _request: Request<PeekableFlightDataStream>,
        ) -> Result<i64, Status> {
            Ok(1)
        }

        // ── Transactions ─────────────────────────────────

        async fn do_action_begin_transaction(
            &self,
            _query: ActionBeginTransactionRequest,
            _request: Request<Action>,
        ) -> Result<ActionBeginTransactionResult, Status> {
            let tx_id = Uuid::new_v4().to_string();
            self.transactions.lock().await.insert(tx_id.clone());
            Ok(ActionBeginTransactionResult {
                transaction_id: tx_id.into_bytes().into(),
            })
        }

        async fn do_action_end_transaction(
            &self,
            query: ActionEndTransactionRequest,
            _request: Request<Action>,
        ) -> Result<(), Status> {
            let tx_id = String::from_utf8(query.transaction_id.to_vec())
                .map_err(|_| Status::invalid_argument("invalid tx id"))?;
            if !self.transactions.lock().await.remove(&tx_id) {
                return Err(Status::not_found("transaction not found"));
            }
            Ok(())
        }

        // ── Ingest ────────────────────────────────────────

        async fn do_put_statement_ingest(
            &self,
            _ticket: CommandStatementIngest,
            request: Request<PeekableFlightDataStream>,
        ) -> Result<i64, Status> {
            let batches: Vec<RecordBatch> = FlightRecordBatchStream::new_from_flight_data(
                request.into_inner().map_err(|e| e.into()),
            )
            .try_collect()
            .await?;
            Ok(batches.iter().map(|b| b.num_rows() as i64).sum())
        }

        async fn register_sql_info(&self, _id: i32, _result: &SqlInfo) {}
    }

    // ══════════════════════════════════════════════════════════
    //  Test Fixture
    // ══════════════════════════════════════════════════════════

    struct TestFixture {
        addr: SocketAddr,
        _shutdown: tokio::sync::oneshot::Sender<()>,
    }

    impl TestFixture {
        async fn new(server: MockFlightSqlServer) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (tx, rx) = tokio::sync::oneshot::channel();

            let svc = server.service();
            tokio::spawn(async move {
                tonic::transport::Server::builder()
                    .add_service(svc)
                    .serve_with_incoming_shutdown(
                        tokio_stream::wrappers::TcpListenerStream::new(listener),
                        async {
                            rx.await.ok();
                        },
                    )
                    .await
                    .unwrap();
            });

            // Give the server a moment to start
            tokio::time::sleep(Duration::from_millis(50)).await;

            Self {
                addr,
                _shutdown: tx,
            }
        }

        fn profile(&self) -> ConnectionProfile {
            ConnectionProfile {
                name: "test".into(),
                host: self.addr.ip().to_string(),
                port: self.addr.port(),
                tls_enabled: false,
                auth: AuthMethod::None,
            }
        }

        fn profile_with_basic_auth(&self) -> ConnectionProfile {
            ConnectionProfile {
                auth: AuthMethod::Basic {
                    username: "admin".into(),
                    password: "password".into(),
                },
                ..self.profile()
            }
        }

        fn profile_with_bearer(&self) -> ConnectionProfile {
            ConnectionProfile {
                auth: AuthMethod::BearerToken {
                    token: FAKE_TOKEN.into(),
                },
                ..self.profile()
            }
        }

        async fn client(&self) -> FlightClient {
            FlightClient::connect(&self.profile()).await.unwrap()
        }
    }

    // ══════════════════════════════════════════════════════════
    //  Tests — organised by Flight SQL capability
    // ══════════════════════════════════════════════════════════

    // ── Connection & Authentication ───────────────────────

    #[tokio::test]
    async fn connect_no_auth() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let client = FlightClient::connect(&fix.profile()).await.unwrap();
        assert_eq!(client.state(), ConnectionState::Connected);
    }

    #[tokio::test]
    async fn connect_basic_auth_handshake() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let client = FlightClient::connect(&fix.profile_with_basic_auth())
            .await
            .unwrap();
        assert_eq!(client.state(), ConnectionState::Connected);
        // Handshake should have set a bearer token on the inner client
        assert!(client.inner.token().is_some());
    }

    #[tokio::test]
    async fn connect_basic_auth_bad_credentials() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut profile = fix.profile_with_basic_auth();
        if let AuthMethod::Basic {
            ref mut password, ..
        } = profile.auth
        {
            *password = "wrong".into();
        }
        let result = FlightClient::connect(&profile).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("handshake"),
            "expected handshake error, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn connect_bearer_token() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let client = FlightClient::connect(&fix.profile_with_bearer())
            .await
            .unwrap();
        assert_eq!(client.state(), ConnectionState::Connected);
        assert_eq!(client.inner.token().unwrap(), FAKE_TOKEN);
    }

    #[tokio::test]
    async fn connect_unreachable_server() {
        let profile = ConnectionProfile {
            name: "nope".into(),
            host: "127.0.0.1".into(),
            port: 1, // unlikely to be listening
            tls_enabled: false,
            auth: AuthMethod::None,
        };
        let result = FlightClient::connect(&profile).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn close_transitions_to_disconnected() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        assert_eq!(client.state(), ConnectionState::Connected);
        client.close().await.unwrap();
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    // ── Query Execution ───────────────────────────────────

    #[tokio::test]
    async fn execute_query_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client.execute_query("SELECT 1").await.unwrap();
        assert_eq!(result.total_rows, 3); // fake batch has 3 rows
        assert_eq!(result.batches.len(), 1);
        assert_eq!(result.schema.fields().len(), 2); // id + name
    }

    #[tokio::test]
    async fn execute_query_schema_field_names() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client
            .execute_query("SELECT id, name FROM users")
            .await
            .unwrap();
        let field_names: Vec<&str> = result
            .schema
            .fields()
            .iter()
            .map(|f| f.name().as_str())
            .collect();
        assert_eq!(field_names, vec!["id", "name"]);
    }

    #[tokio::test]
    async fn execute_query_elapsed_is_positive() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client.execute_query("SELECT 1").await.unwrap();
        assert!(result.elapsed > Duration::ZERO);
    }

    #[tokio::test]
    async fn execute_update_returns_count() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let count = client
            .execute_update("INSERT INTO t VALUES (1)")
            .await
            .unwrap();
        assert!(count > 0);
    }

    // ── Prepared Statements ───────────────────────────────

    #[tokio::test]
    async fn execute_prepared_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client
            .execute_prepared("SELECT * FROM users")
            .await
            .unwrap();
        assert_eq!(result.total_rows, 3);
        assert_eq!(result.batches.len(), 1);
    }

    #[tokio::test]
    async fn execute_prepared_schema_matches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client
            .execute_prepared("SELECT * FROM users")
            .await
            .unwrap();
        let field_names: Vec<&str> = result
            .schema
            .fields()
            .iter()
            .map(|f| f.name().as_str())
            .collect();
        assert_eq!(field_names, vec!["id", "name"]);
    }

    // ── Catalog: get_catalogs ─────────────────────────────

    #[tokio::test]
    async fn get_catalogs_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_catalogs().await.unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // production, analytics
    }

    #[tokio::test]
    async fn get_catalogs_schema_has_catalog_name_column() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_catalogs().await.unwrap();
        let schema = batches[0].schema();
        assert!(schema.field_with_name("catalog_name").is_ok());
    }

    // ── Catalog: get_db_schemas ───────────────────────────

    #[tokio::test]
    async fn get_db_schemas_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_db_schemas(None, None).await.unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // public, reporting
    }

    #[tokio::test]
    async fn get_db_schemas_schema_columns() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_db_schemas(None, None).await.unwrap();
        let schema = batches[0].schema();
        assert!(schema.field_with_name("catalog_name").is_ok());
        assert!(schema.field_with_name("db_schema_name").is_ok());
    }

    // ── Catalog: get_tables ───────────────────────────────

    #[tokio::test]
    async fn get_tables_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_tables(None, None, None, vec![], false)
            .await
            .unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3); // users, orders, daily_summary
    }

    #[tokio::test]
    async fn get_tables_schema_has_required_columns() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_tables(None, None, None, vec![], false)
            .await
            .unwrap();
        let schema = batches[0].schema();
        for col in &["catalog_name", "db_schema_name", "table_name", "table_type"] {
            assert!(schema.field_with_name(col).is_ok(), "missing column: {col}");
        }
    }

    // ── Catalog: get_table_types ──────────────────────────

    #[tokio::test]
    async fn get_table_types_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_table_types().await.unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // TABLE, VIEW
    }

    // ── Catalog: get_sql_info ─────────────────────────────

    #[tokio::test]
    async fn get_sql_info_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_sql_info(vec![
                SqlInfo::FlightSqlServerName,
                SqlInfo::FlightSqlServerVersion,
            ])
            .await
            .unwrap();
        assert!(!batches.is_empty());
    }

    #[tokio::test]
    async fn get_sql_info_schema_columns() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_sql_info(vec![SqlInfo::FlightSqlServerName])
            .await
            .unwrap();
        let schema = batches[0].schema();
        assert!(schema.field_with_name("info_name").is_ok());
        assert!(schema.field_with_name("value").is_ok());
    }

    // ── Catalog: get_primary_keys ─────────────────────────

    #[tokio::test]
    async fn get_primary_keys_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_primary_keys(
                Some("production".into()),
                Some("public".into()),
                "users".into(),
            )
            .await
            .unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 1); // "id" key
    }

    #[tokio::test]
    async fn get_primary_keys_schema_columns() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_primary_keys(None, None, "users".into())
            .await
            .unwrap();
        let schema = batches[0].schema();
        assert!(schema.field_with_name("table_name").is_ok());
        assert!(schema.field_with_name("column_name").is_ok());
        assert!(schema.field_with_name("key_sequence").is_ok());
    }

    // ── Catalog: get_exported_keys ────────────────────────

    #[tokio::test]
    async fn get_exported_keys_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_exported_keys(
                Some("production".into()),
                Some("public".into()),
                "users".into(),
            )
            .await
            .unwrap();
        // Returns 0 rows (no exported FK relationships in mock) but succeeds
        assert!(!batches.is_empty());
    }

    // ── Catalog: get_imported_keys ────────────────────────

    #[tokio::test]
    async fn get_imported_keys_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_imported_keys(
                Some("production".into()),
                Some("public".into()),
                "orders".into(),
            )
            .await
            .unwrap();
        assert!(!batches.is_empty());
    }

    // ── Catalog: get_cross_reference ──────────────────────

    #[tokio::test]
    async fn get_cross_reference_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client
            .get_cross_reference(
                Some("production".into()),
                Some("public".into()),
                "users".into(),
                Some("production".into()),
                Some("public".into()),
                "orders".into(),
            )
            .await
            .unwrap();
        assert!(!batches.is_empty());
    }

    // ── Catalog: get_xdbc_type_info ───────────────────────

    #[tokio::test]
    async fn get_xdbc_type_info_returns_batches() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let batches = client.get_xdbc_type_info(None).await.unwrap();
        assert!(!batches.is_empty());
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert!(total_rows >= 1); // INTEGER from XDBC_INFO
    }

    // ── Transactions ──────────────────────────────────────

    #[tokio::test]
    async fn begin_and_commit_transaction() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let tx_id = client.begin_transaction().await.unwrap();
        assert!(!tx_id.is_empty());
        client
            .end_transaction(tx_id, arrow_flight::sql::EndTransaction::Commit)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn begin_and_rollback_transaction() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let tx_id = client.begin_transaction().await.unwrap();
        client
            .end_transaction(tx_id, arrow_flight::sql::EndTransaction::Rollback)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn end_unknown_transaction_fails() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        let result = client
            .end_transaction(
                bytes::Bytes::from("nonexistent"),
                arrow_flight::sql::EndTransaction::Commit,
            )
            .await;
        assert!(result.is_err());
    }

    // ── Custom headers ────────────────────────────────────

    #[tokio::test]
    async fn set_header_persists() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let mut client = fix.client().await;
        client.set_header("x-custom", "value");
        // After setting, queries should still work (header doesn't break anything)
        let result = client.execute_query("SELECT 1").await.unwrap();
        assert_eq!(result.total_rows, 3);
    }

    // ── Accessors ─────────────────────────────────────────

    #[tokio::test]
    async fn profile_accessor() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let client = fix.client().await;
        assert_eq!(client.profile().name, "test");
        assert_eq!(client.profile().host, fix.addr.ip().to_string());
        assert_eq!(client.profile().port, fix.addr.port());
    }

    #[tokio::test]
    async fn from_inner_constructor() {
        let fix = TestFixture::new(MockFlightSqlServer::new()).await;
        let uri = format!("http://{}", fix.addr);
        let channel = Channel::from_shared(uri).unwrap().connect().await.unwrap();
        let inner = FlightSqlServiceClient::new(channel);
        let client = FlightClient::from_inner(inner, fix.profile());
        assert_eq!(client.state(), ConnectionState::Connected);
    }
}
