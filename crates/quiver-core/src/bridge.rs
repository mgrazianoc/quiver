//! Async bridge between the synchronous TUI event loop and the
//! tokio-based Flight SQL client.
//!
//! The TUI sends [`CoreRequest`] messages through a channel and
//! polls for [`CoreResponse`] messages each tick via `try_recv`.

use std::thread;

use anyhow::Result;
use arrow::array::Array;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::catalog::{
    build_schema_tree, extract_columns, extract_tables, ColumnInfo, TableInfo, TreeNode,
};
use crate::client::{FlightClient, QueryResult};
use crate::connection::ConnectionProfile;

// ── Request / Response enums ──────────────────────────────────

/// Commands the TUI can send to the async runtime.
#[derive(Debug)]
pub enum CoreRequest {
    /// Connect to a Flight SQL server.
    Connect(ConnectionProfile),
    /// Disconnect from the current server.
    Disconnect,
    /// Execute a SQL query.
    ExecuteQuery(String),
    /// Cancel the currently running query.
    CancelQuery,
    /// Refresh the schema browser tree.
    RefreshSchema,
    /// Test a connection without persisting it.
    TestConnection(ConnectionProfile),
}

/// Responses the async runtime sends back to the TUI.
#[derive(Debug)]
pub enum CoreResponse {
    /// Connection state changed.
    Connected {
        profile: ConnectionProfile,
        server_info: Vec<(String, String)>,
    },
    /// Disconnected from the server.
    Disconnected,
    /// Query completed successfully.
    QueryCompleted(QueryResult),
    /// Schema tree loaded.
    SchemaLoaded(Vec<TreeNode>),
    /// An operation failed.
    Error { operation: String, message: String },
    /// Test connection result.
    TestResult { success: bool, message: String },
}

// ── Core handle ───────────────────────────────────────────────

/// Handle held by the TUI to communicate with the async core.
pub struct CoreHandle {
    tx: mpsc::UnboundedSender<CoreRequest>,
    rx: mpsc::UnboundedReceiver<CoreResponse>,
}

impl CoreHandle {
    /// Spawn the async runtime on a background thread and return a handle.
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::unbounded_channel::<CoreRequest>();
        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<CoreResponse>();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            rt.block_on(core_loop(req_rx, resp_tx));
        });

        Self {
            tx: req_tx,
            rx: resp_rx,
        }
    }

    /// Send a request to the async core.
    pub fn send(&self, request: CoreRequest) {
        // If the receiver is dropped the runtime has panicked; ignore.
        let _ = self.tx.send(request);
    }

    /// Non-blocking poll for a response.
    pub fn try_recv(&mut self) -> Option<CoreResponse> {
        self.rx.try_recv().ok()
    }
}

// ── Async core loop ───────────────────────────────────────────

async fn core_loop(
    mut rx: mpsc::UnboundedReceiver<CoreRequest>,
    tx: mpsc::UnboundedSender<CoreResponse>,
) {
    let mut client: Option<FlightClient> = None;
    let mut cancel_token: Option<CancellationToken> = None;

    while let Some(req) = rx.recv().await {
        match req {
            CoreRequest::Connect(profile) => {
                let max_attempts = (profile.max_retries as usize) + 1;
                let mut last_err = String::new();
                let mut connected = false;

                for attempt in 1..=max_attempts {
                    match FlightClient::connect(&profile).await {
                        Ok(c) => {
                            client = Some(c);
                            let _ = tx.send(CoreResponse::Connected {
                                profile: profile.clone(),
                                server_info: Vec::new(),
                            });
                            connected = true;
                            break;
                        }
                        Err(e) => {
                            last_err = e.to_string();
                            if attempt < max_attempts {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                        }
                    }
                }

                if !connected {
                    let _ = tx.send(CoreResponse::Error {
                        operation: "connect".into(),
                        message: last_err,
                    });
                }
            }

            CoreRequest::Disconnect => {
                if let Some(mut c) = client.take() {
                    let _ = c.close().await;
                }
                let _ = tx.send(CoreResponse::Disconnected);
            }

            CoreRequest::ExecuteQuery(sql) => {
                let token = CancellationToken::new();
                cancel_token = Some(token.clone());

                if let Some(ref mut c) = client {
                    match c.execute_query_cancellable(&sql, token).await {
                        Ok(result) => {
                            let _ = tx.send(CoreResponse::QueryCompleted(result));
                        }
                        Err(e) => {
                            let _ = tx.send(CoreResponse::Error {
                                operation: "execute_query".into(),
                                message: e.to_string(),
                            });
                        }
                    }
                } else {
                    let _ = tx.send(CoreResponse::Error {
                        operation: "execute_query".into(),
                        message: "not connected".into(),
                    });
                }
                let _ = cancel_token.take();
            }

            CoreRequest::CancelQuery => {
                if let Some(token) = cancel_token.take() {
                    token.cancel();
                }
            }

            CoreRequest::RefreshSchema => {
                if let Some(ref mut c) = client {
                    match fetch_schema_tree(c).await {
                        Ok(tree) => {
                            let _ = tx.send(CoreResponse::SchemaLoaded(tree));
                        }
                        Err(e) => {
                            let _ = tx.send(CoreResponse::Error {
                                operation: "refresh_schema".into(),
                                message: e.to_string(),
                            });
                        }
                    }
                } else {
                    let _ = tx.send(CoreResponse::Error {
                        operation: "refresh_schema".into(),
                        message: "not connected".into(),
                    });
                }
            }

            CoreRequest::TestConnection(profile) => match FlightClient::connect(&profile).await {
                Ok(mut c) => {
                    let _ = c.close().await;
                    let _ = tx.send(CoreResponse::TestResult {
                        success: true,
                        message: format!("Connected to {}", profile.endpoint_uri()),
                    });
                }
                Err(e) => {
                    let _ = tx.send(CoreResponse::TestResult {
                        success: false,
                        message: e.to_string(),
                    });
                }
            },
        }
    }
}

/// Fetch tables from the server and build a schema tree.
async fn fetch_schema_tree(client: &mut FlightClient) -> Result<Vec<TreeNode>> {
    let table_batches = client.get_tables(None, None, None, vec![], true).await?;

    let tables = extract_tables(&table_batches);

    // Try to extract column info from the table_schema binary column,
    // or fall back to no columns if the server doesn't provide schema.
    let mut tables_with_cols: Vec<(TableInfo, Option<Vec<ColumnInfo>>)> = Vec::new();

    for batch in &table_batches {
        if let Ok(schema_idx) = batch.schema().index_of("table_schema") {
            let schema_col = batch.column(schema_idx);
            let names_col = arrow::array::as_string_array(
                batch.column(batch.schema().index_of("table_name").unwrap_or(0)),
            );

            // table_schema is IPC-encoded binary
            if let Some(binary_arr) = schema_col
                .as_any()
                .downcast_ref::<arrow::array::BinaryArray>()
            {
                for row in 0..batch.num_rows() {
                    let table_name = names_col.value(row);
                    // Find the matching TableInfo
                    if let Some(pos) = tables.iter().position(|t| t.table_name == table_name) {
                        let cols = if !binary_arr.is_null(row) {
                            let ipc_bytes = binary_arr.value(row);
                            decode_ipc_schema(ipc_bytes).map(|s| extract_columns(&s))
                        } else {
                            None
                        };
                        tables_with_cols.push((tables[pos].clone(), cols));
                    }
                }
            } else {
                // Not binary — just add tables without columns
                for t in &tables {
                    if !tables_with_cols
                        .iter()
                        .any(|(tc, _)| tc.table_name == t.table_name)
                    {
                        tables_with_cols.push((t.clone(), None));
                    }
                }
            }
        }
    }

    // If we couldn't extract from table_schema column, fall back
    if tables_with_cols.is_empty() {
        tables_with_cols = tables.into_iter().map(|t| (t, None)).collect();
    }

    Ok(build_schema_tree(&tables_with_cols))
}

/// Decode an IPC-serialized Arrow schema from bytes.
fn decode_ipc_schema(bytes: &[u8]) -> Option<arrow_schema::Schema> {
    arrow::ipc::root_as_schema(bytes)
        .ok()
        .map(|fb| arrow::ipc::convert::fb_to_schema(fb))
}
