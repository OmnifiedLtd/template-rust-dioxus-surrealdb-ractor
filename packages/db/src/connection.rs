//! Database connection management with lazy initialization.

use std::sync::LazyLock;
use surrealdb::engine::any::{Any, connect};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use thiserror::Error;
use tokio::sync::OnceCell;

/// Global database instance using lazy initialization.
static DB: LazyLock<OnceCell<Surreal<Any>>> = LazyLock::new(OnceCell::new);

/// Database connection wrapper.
pub type Database = Surreal<Any>;

/// Database configuration.
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// Connection mode: "memory" or "file://path"
    pub endpoint: String,
    /// Namespace to use
    pub namespace: String,
    /// Database name to use
    pub database: String,
    /// Optional root credentials for authentication
    pub credentials: Option<(String, String)>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            endpoint: "mem://".to_string(),
            namespace: "jobqueue".to_string(),
            database: "main".to_string(),
            credentials: None,
        }
    }
}

impl DbConfig {
    /// Create a config for in-memory testing.
    pub fn memory() -> Self {
        Self::default()
    }

    /// Create a config for file-based persistence.
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            endpoint: format!("file://{}", path.into()),
            ..Default::default()
        }
    }

    /// Create a config for RocksDB persistence (requires rocksdb feature).
    pub fn rocksdb(path: impl Into<String>) -> Self {
        Self {
            endpoint: format!("rocksdb://{}", path.into()),
            ..Default::default()
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set the database name.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }

    /// Set root credentials for authentication.
    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }
}

/// Database errors.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database not initialized - call init_db first")]
    NotInitialized,
    #[error("Database already initialized")]
    AlreadyInitialized,
    #[error("Connection error: {0}")]
    Connection(#[from] surrealdb::Error),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Initialize the database connection.
///
/// This should be called once at application startup before any database operations.
pub async fn init_db(config: DbConfig) -> Result<&'static Database, DbError> {
    DB.get_or_try_init(|| async {
        tracing::info!("Connecting to database: {}", config.endpoint);

        let db = connect(&config.endpoint).await?;

        // Authenticate if credentials provided
        if let Some((username, password)) = &config.credentials {
            db.signin(Root {
                username,
                password,
            })
            .await?;
        }

        // Select namespace and database
        db.use_ns(&config.namespace).use_db(&config.database).await?;

        tracing::info!(
            "Connected to database: {}/{}",
            config.namespace,
            config.database
        );

        Ok(db)
    })
    .await
}

/// Get the database connection.
///
/// Panics if the database hasn't been initialized yet.
pub fn get_db() -> &'static Database {
    DB.get()
        .expect("Database not initialized - call init_db first")
}

/// Try to get the database connection.
///
/// Returns None if the database hasn't been initialized yet.
pub fn try_get_db() -> Option<&'static Database> {
    DB.get()
}
