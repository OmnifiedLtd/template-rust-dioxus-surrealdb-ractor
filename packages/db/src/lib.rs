//! SurrealDB integration for the job queue system.
//!
//! This crate provides database connectivity and repositories for
//! persisting jobs and queues.
//!
//! # Features
//!
//! - `memory` (default): Use in-memory storage for testing
//! - `rocksdb`: Use RocksDB for persistent file-based storage

mod connection;
mod schema;
pub mod repositories;

pub use connection::{Database, DbConfig, DbError, get_db, init_db};
pub use schema::init_schema;

/// Initialize the database with the given configuration.
///
/// This should be called once at application startup.
pub async fn init(config: DbConfig) -> Result<(), DbError> {
    init_db(config).await?;
    init_schema().await?;
    Ok(())
}
