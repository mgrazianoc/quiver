#![allow(dead_code, unused_variables)]
//! Core engine — connection management, Flight SQL client, data layer.
//!
//! This crate houses:
//! - `ConnectionProfile` and `ConnectionManager`
//! - `FlightClient` — async Flight SQL client wrapper
//! - Schema introspection types (`TreeNode`, catalog RPCs)
//! - Query execution types (`QueryResult`)

pub mod bridge;
pub mod catalog;
pub mod client;
pub mod config;
pub mod connection;
