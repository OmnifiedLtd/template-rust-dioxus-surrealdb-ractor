//! Queue repository for CRUD operations.

use chrono::{DateTime, Utc};
use queue_core::{Queue, QueueConfig, QueueId, QueueState, QueueStats};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use crate::{DbError, get_db};

/// Repository for queue persistence operations.
pub struct QueueRepository;

/// Internal record type for SurrealDB reads.
#[derive(Debug, Deserialize)]
struct QueueRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    name: String,
    description: Option<String>,
    state: QueueState,
    config: QueueConfig,
    stats: QueueStats,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl QueueRecord {
    fn into_queue(self, queue_id: QueueId) -> Queue {
        Queue {
            id: queue_id,
            name: self.name,
            description: self.description,
            state: self.state,
            config: self.config,
            stats: self.stats,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Struct for creating/updating queues - omits datetime fields to use SurrealDB defaults.
#[derive(Debug, Clone, Serialize)]
struct QueueCreate {
    name: String,
    description: Option<String>,
    state: QueueState,
    config: QueueConfig,
    stats: QueueStats,
}

impl QueueRepository {
    /// Create a new queue in the database.
    pub async fn create(queue: &Queue) -> Result<Queue, DbError> {
        let db = get_db()?;
        let queue_id = queue.id.to_string();

        // Use QueueCreate to omit datetime fields - let SurrealDB use defaults
        let create_data = QueueCreate {
            name: queue.name.clone(),
            description: queue.description.clone(),
            state: queue.state,
            config: queue.config.clone(),
            stats: queue.stats.clone(),
        };

        let record: Option<QueueRecord> =
            db.create(("queue", &queue_id)).content(create_data).await?;

        record
            .map(|r| r.into_queue(queue.id))
            .ok_or_else(|| DbError::Query("Failed to create queue".into()))
    }

    /// Get a queue by ID.
    pub async fn get(id: QueueId) -> Result<Queue, DbError> {
        let db = get_db()?;

        let record: Option<QueueRecord> = db.select(("queue", id.to_string())).await?;

        record
            .map(|r| r.into_queue(id))
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Get a queue by name.
    pub async fn get_by_name(name: &str) -> Result<Queue, DbError> {
        let db = get_db()?;
        let name_owned = name.to_string();

        let mut result = db
            .query("SELECT * FROM queue WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| {
                // Parse queue ID from SurrealDB record id
                let id_str = r.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
                let queue_id = QueueId::parse(&id_str).unwrap_or_else(|_| QueueId::new());
                r.into_queue(queue_id)
            })
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", name)))
    }

    /// List all queues.
    pub async fn list() -> Result<Vec<Queue>, DbError> {
        let db = get_db()?;

        let records: Vec<QueueRecord> = db.select("queue").await?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id_str = r.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
                let queue_id = QueueId::parse(&id_str).unwrap_or_else(|_| QueueId::new());
                r.into_queue(queue_id)
            })
            .collect())
    }

    /// List queues by state.
    pub async fn list_by_state(state: QueueState) -> Result<Vec<Queue>, DbError> {
        let db = get_db()?;

        let mut result = db
            .query("SELECT * FROM queue WHERE state = $state ORDER BY created_at DESC")
            .bind(("state", state.to_string()))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id_str = r.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
                let queue_id = QueueId::parse(&id_str).unwrap_or_else(|_| QueueId::new());
                r.into_queue(queue_id)
            })
            .collect())
    }

    /// Update a queue's state.
    pub async fn update_state(id: QueueId, state: QueueState) -> Result<Queue, DbError> {
        let db = get_db()?;

        // Use SurrealQL to set updated_at with time::now()
        let mut result = db
            .query("UPDATE type::thing('queue', $id) SET state = $state, updated_at = time::now() RETURN AFTER")
            .bind(("id", id.to_string()))
            .bind(("state", state))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.into_queue(id))
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Update a queue's statistics.
    pub async fn update_stats(id: QueueId, stats: &QueueStats) -> Result<Queue, DbError> {
        let db = get_db()?;
        let stats_clone = stats.clone();

        // Use SurrealQL to set updated_at with time::now()
        let mut result = db
            .query("UPDATE type::thing('queue', $id) SET stats = $stats, updated_at = time::now() RETURN AFTER")
            .bind(("id", id.to_string()))
            .bind(("stats", stats_clone))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.into_queue(id))
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", id)))
    }

    /// Update a queue.
    pub async fn update(queue: &Queue) -> Result<Queue, DbError> {
        let db = get_db()?;

        let mut result = db
            .query(
                "UPDATE type::thing('queue', $id) SET name = $name, description = $description, state = $state, config = $config, stats = $stats, updated_at = time::now() RETURN AFTER",
            )
            .bind(("id", queue.id.to_string()))
            .bind(("name", queue.name.clone()))
            .bind(("description", queue.description.clone()))
            .bind(("state", queue.state))
            .bind(("config", queue.config.clone()))
            .bind(("stats", queue.stats.clone()))
            .await?;

        let records: Vec<QueueRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.into_queue(queue.id))
            .ok_or_else(|| DbError::NotFound(format!("Queue not found: {}", queue.id)))
    }

    /// Delete a queue.
    pub async fn delete(id: QueueId) -> Result<(), DbError> {
        let db = get_db()?;

        let _: Option<QueueRecord> = db.delete(("queue", id.to_string())).await?;

        Ok(())
    }

    /// Check if a queue exists.
    pub async fn exists(id: QueueId) -> Result<bool, DbError> {
        let db = get_db()?;

        let record: Option<QueueRecord> = db.select(("queue", id.to_string())).await?;

        Ok(record.is_some())
    }

    /// Check if a queue name exists.
    pub async fn name_exists(name: &str) -> Result<bool, DbError> {
        let db = get_db()?;
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

        Ok(counts.first().is_some_and(|c| c.count > 0))
    }
}
