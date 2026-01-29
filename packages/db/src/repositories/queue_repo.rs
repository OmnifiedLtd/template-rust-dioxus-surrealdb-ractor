//! Queue repository for CRUD operations.

use queue_core::{Queue, QueueId, QueueState, QueueStats};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use crate::{get_db, DbError};

/// Repository for queue persistence operations.
pub struct QueueRepository;

/// Internal record type for SurrealDB.
#[derive(Debug, Serialize, Deserialize)]
struct QueueRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    #[serde(flatten)]
    queue: Queue,
}

impl QueueRepository {
    /// Create a new queue in the database.
    pub async fn create(queue: &Queue) -> Result<Queue, DbError> {
        let db = get_db();
        let queue_id = queue.id.to_string();
        let queue_data = queue.clone();

        let record: Option<QueueRecord> = db
            .create(("queue", queue_id))
            .content(queue_data)
            .await?;

        record
            .map(|r| r.queue)
            .ok_or_else(|| DbError::Query("Failed to create queue".into()))
    }

    /// Get a queue by ID.
    pub async fn get(id: QueueId) -> Result<Queue, DbError> {
        let db = get_db();

        let record: Option<QueueRecord> = db.select(("queue", id.to_string())).await?;

        record
            .map(|r| r.queue)
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Get a queue by name.
    pub async fn get_by_name(name: &str) -> Result<Queue, DbError> {
        let db = get_db();
        let name_owned = name.to_string();

        let mut result = db
            .query("SELECT * FROM queue WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.queue)
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", name)))
    }

    /// List all queues.
    pub async fn list() -> Result<Vec<Queue>, DbError> {
        let db = get_db();

        let records: Vec<QueueRecord> = db.select("queue").await?;

        Ok(records.into_iter().map(|r| r.queue).collect())
    }

    /// List queues by state.
    pub async fn list_by_state(state: QueueState) -> Result<Vec<Queue>, DbError> {
        let db = get_db();

        let mut result = db
            .query("SELECT * FROM queue WHERE state = $state ORDER BY created_at DESC")
            .bind(("state", state.to_string()))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        Ok(records.into_iter().map(|r| r.queue).collect())
    }

    /// Update a queue's state.
    pub async fn update_state(id: QueueId, state: QueueState) -> Result<Queue, DbError> {
        let db = get_db();

        let record: Option<QueueRecord> = db
            .update(("queue", id.to_string()))
            .merge(serde_json::json!({
                "state": state,
                "updated_at": chrono::Utc::now()
            }))
            .await?;

        record
            .map(|r| r.queue)
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Update a queue's statistics.
    pub async fn update_stats(id: QueueId, stats: &QueueStats) -> Result<Queue, DbError> {
        let db = get_db();

        let record: Option<QueueRecord> = db
            .update(("queue", id.to_string()))
            .merge(serde_json::json!({
                "stats": stats,
                "updated_at": chrono::Utc::now()
            }))
            .await?;

        record
            .map(|r| r.queue)
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Update a queue.
    pub async fn update(queue: &Queue) -> Result<Queue, DbError> {
        let db = get_db();

        let mut updated = queue.clone();
        updated.updated_at = chrono::Utc::now();
        let queue_id = queue.id.to_string();

        let record: Option<QueueRecord> = db
            .update(("queue", queue_id))
            .content(updated)
            .await?;

        record
            .map(|r| r.queue)
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", queue.id)))
    }

    /// Delete a queue.
    pub async fn delete(id: QueueId) -> Result<(), DbError> {
        let db = get_db();

        let _: Option<QueueRecord> = db.delete(("queue", id.to_string())).await?;

        Ok(())
    }

    /// Check if a queue exists.
    pub async fn exists(id: QueueId) -> Result<bool, DbError> {
        let db = get_db();

        let record: Option<QueueRecord> = db.select(("queue", id.to_string())).await?;

        Ok(record.is_some())
    }

    /// Check if a queue name exists.
    pub async fn name_exists(name: &str) -> Result<bool, DbError> {
        let db = get_db();
        let name_owned = name.to_string();

        let mut result = db
            .query("SELECT count() FROM queue WHERE name = $name GROUP ALL")
            .bind(("name", name_owned))
            .await?;

        #[derive(Deserialize)]
        struct CountResult {
            count: i64,
        }

        let counts: Vec<CountResult> = result.take(0)?;

        Ok(counts.first().map_or(false, |c| c.count > 0))
    }
}
