#![allow(dead_code, unused_variables)]
//! Core engine — connection management, Flight SQL client, data layer.
//!
//! This crate houses:
//! - `ConnectionProfile` and `ConnectionManager`
//! - Flight SQL client wrapper (future: `arrow_flight::sql::FlightSqlServiceClient`)
//! - Schema introspection types
//! - Query execution types

pub mod config;
pub mod connection;
