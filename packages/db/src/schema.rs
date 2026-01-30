//! Database schema definitions using SurrealQL.

use crate::{get_db, DbError};

/// Initialize the database schema.
///
/// This creates all necessary tables, fields, and indexes.
pub async fn init_schema() -> Result<(), DbError> {
    let db = get_db();

    tracing::info!("Initializing database schema...");

    // Queue table
    db.query(QUEUE_SCHEMA).await?;

    // Job table
    db.query(JOB_SCHEMA).await?;

    // Job history table (for analytics)
    db.query(JOB_HISTORY_SCHEMA).await?;

    tracing::info!("Database schema initialized");

    Ok(())
}

/// Queue table schema.
const QUEUE_SCHEMA: &str = r#"
-- Queue table for storing queue metadata
DEFINE TABLE IF NOT EXISTS queue SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS name ON queue TYPE string;
DEFINE FIELD IF NOT EXISTS description ON queue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS state ON queue TYPE string DEFAULT "running";
DEFINE FIELD IF NOT EXISTS config ON queue TYPE object;
DEFINE FIELD IF NOT EXISTS config.concurrency ON queue TYPE int DEFAULT 4;
DEFINE FIELD IF NOT EXISTS config.default_timeout_secs ON queue TYPE int DEFAULT 300;
DEFINE FIELD IF NOT EXISTS config.default_max_retries ON queue TYPE int DEFAULT 3;
DEFINE FIELD IF NOT EXISTS config.max_queue_size ON queue TYPE option<int>;
DEFINE FIELD IF NOT EXISTS config.rate_limit ON queue TYPE option<float>;
DEFINE FIELD IF NOT EXISTS stats ON queue TYPE object DEFAULT {};
DEFINE FIELD IF NOT EXISTS created_at ON queue TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON queue TYPE datetime DEFAULT time::now();

-- Indexes for efficient lookups
DEFINE INDEX IF NOT EXISTS queue_name ON queue FIELDS name UNIQUE;
DEFINE INDEX IF NOT EXISTS queue_state ON queue FIELDS state;
"#;

/// Job table schema.
const JOB_SCHEMA: &str = r#"
-- Job table for storing active jobs
DEFINE TABLE IF NOT EXISTS job SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS queue_id ON job TYPE string;
DEFINE FIELD IF NOT EXISTS job_type ON job TYPE string;
DEFINE FIELD IF NOT EXISTS payload ON job TYPE object;
DEFINE FIELD IF NOT EXISTS priority ON job TYPE string DEFAULT "normal";
DEFINE FIELD IF NOT EXISTS status ON job TYPE object;
DEFINE FIELD IF NOT EXISTS max_retries ON job TYPE int DEFAULT 3;
DEFINE FIELD IF NOT EXISTS timeout_secs ON job TYPE int DEFAULT 300;
DEFINE FIELD IF NOT EXISTS tags ON job TYPE array DEFAULT [];
DEFINE FIELD IF NOT EXISTS created_at ON job TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON job TYPE datetime DEFAULT time::now();

-- Indexes for efficient job queries
DEFINE INDEX IF NOT EXISTS job_queue ON job FIELDS queue_id;
DEFINE INDEX IF NOT EXISTS job_status ON job FIELDS status.status;
DEFINE INDEX IF NOT EXISTS job_priority ON job FIELDS priority;
DEFINE INDEX IF NOT EXISTS job_type ON job FIELDS job_type;
DEFINE INDEX IF NOT EXISTS job_created ON job FIELDS created_at;

-- Compound index for queue polling (pending jobs by priority)
DEFINE INDEX IF NOT EXISTS job_queue_pending ON job FIELDS queue_id, status.status, priority;
"#;

/// Job history table schema for analytics and auditing.
const JOB_HISTORY_SCHEMA: &str = r#"
-- Job history for completed/failed jobs (archival)
DEFINE TABLE IF NOT EXISTS job_history SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS job_id ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS queue_id ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS job_type ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS priority ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS final_status ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS attempts ON job_history TYPE int DEFAULT 1;
DEFINE FIELD IF NOT EXISTS duration_ms ON job_history TYPE option<int>;
DEFINE FIELD IF NOT EXISTS error ON job_history TYPE option<string>;
DEFINE FIELD IF NOT EXISTS result_summary ON job_history TYPE option<string>;
DEFINE FIELD IF NOT EXISTS tags ON job_history TYPE array DEFAULT [];
DEFINE FIELD IF NOT EXISTS created_at ON job_history TYPE string;
DEFINE FIELD IF NOT EXISTS completed_at ON job_history TYPE datetime DEFAULT time::now();

-- Indexes for analytics queries
DEFINE INDEX IF NOT EXISTS history_queue ON job_history FIELDS queue_id;
DEFINE INDEX IF NOT EXISTS history_type ON job_history FIELDS job_type;
DEFINE INDEX IF NOT EXISTS history_status ON job_history FIELDS final_status;
DEFINE INDEX IF NOT EXISTS history_completed ON job_history FIELDS completed_at;
"#;
