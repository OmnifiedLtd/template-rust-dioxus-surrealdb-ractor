//! Job repository for CRUD operations.

use chrono::{DateTime, Utc};
use queue_core::{Job, JobId, JobStatus, Priority, QueueId, QueueStats};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use surrealdb::sql::Thing;

use crate::{DbError, get_db};

/// Repository for job persistence operations.
pub struct JobRepository;

/// Internal record type for reading from SurrealDB.
#[derive(Debug, Deserialize)]
struct JobRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    queue_id: String,
    job_type: String,
    payload: JsonValue,
    priority: Priority,
    status: JobStatus,
    attempts: u32,
    max_retries: u32,
    timeout_secs: u64,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl JobRecord {
    fn into_job(self, job_id: JobId) -> Job {
        let queue_id = QueueId::parse(&self.queue_id).unwrap_or_else(|_| QueueId::new());
        Job {
            id: job_id,
            queue_id,
            job_type: self.job_type,
            payload: self.payload,
            priority: self.priority,
            status: self.status,
            attempts: self.attempts,
            max_retries: self.max_retries,
            timeout_secs: self.timeout_secs,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Struct for creating/updating jobs - omits datetime fields to use SurrealDB defaults.
#[derive(Debug, Clone, Serialize)]
struct JobCreate {
    queue_id: String,
    job_type: String,
    payload: JsonValue,
    priority: Priority,
    status: JobStatus,
    attempts: u32,
    max_retries: u32,
    timeout_secs: u64,
    tags: Vec<String>,
}

/// Job history record for archival - omits completed_at to use SurrealDB default.
#[derive(Debug, Serialize)]
pub struct JobHistoryCreate {
    pub job_id: String,
    pub queue_id: String,
    pub job_type: String,
    pub priority: String,
    pub final_status: String,
    pub attempts: u32,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub result_summary: Option<String>,
    pub tags: Vec<String>,
    // Note: created_at from original job is stored as ISO string for reference
    pub created_at: String,
    // completed_at uses SurrealDB DEFAULT time::now()
}

/// Filter options for listing jobs.
#[derive(Debug, Default, Clone)]
pub struct JobFilter {
    pub queue_id: Option<QueueId>,
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[allow(clippy::result_large_err)]
fn to_json<T: Serialize>(value: T) -> Result<serde_json::Value, DbError> {
    serde_json::to_value(value).map_err(|e| DbError::Serialization(e.to_string()))
}

impl JobRepository {
    /// Create a new job in the database.
    pub async fn create(job: &Job) -> Result<Job, DbError> {
        let db = get_db()?;

        // Use JobCreate to omit datetime fields - let SurrealDB use defaults
        let create_data = JobCreate {
            queue_id: job.queue_id.to_string(),
            job_type: job.job_type.clone(),
            payload: job.payload.clone(),
            priority: job.priority,
            status: job.status.clone(),
            attempts: job.attempts,
            max_retries: job.max_retries,
            timeout_secs: job.timeout_secs,
            tags: job.tags.clone(),
        };

        let record: Option<JobRecord> = db
            .create(("job", job.id.to_string()))
            .content(create_data)
            .await?;

        record
            .map(|r| r.into_job(job.id))
            .ok_or_else(|| DbError::Query("Failed to create job".into()))
    }

    /// Get a job by ID.
    pub async fn get(id: JobId) -> Result<Job, DbError> {
        let db = get_db()?;

        let record: Option<JobRecord> = db.select(("job", id.to_string())).await?;

        record
            .map(|r| r.into_job(id))
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", id)))
    }

    /// List jobs with optional filtering.
    pub async fn list(filter: JobFilter) -> Result<Vec<Job>, DbError> {
        let db = get_db()?;

        let mut conditions = Vec::new();
        let mut bindings: Vec<(&str, serde_json::Value)> = Vec::new();

        if let Some(queue_id) = &filter.queue_id {
            conditions.push("queue_id = $queue_id");
            bindings.push(("queue_id", to_json(queue_id.to_string())?));
        }

        if let Some(status) = &filter.status {
            conditions.push("status.status = $status");
            bindings.push(("status", to_json(status)?));
        }

        if let Some(job_type) = &filter.job_type {
            conditions.push("job_type = $job_type");
            bindings.push(("job_type", to_json(job_type)?));
        }

        if let Some(priority) = &filter.priority {
            conditions.push("priority = $priority");
            bindings.push(("priority", to_json(priority.to_string())?));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_clause = filter
            .limit
            .map(|l| format!("LIMIT {}", l))
            .unwrap_or_default();

        let offset_clause = filter
            .offset
            .map(|o| format!("START {}", o))
            .unwrap_or_default();

        let query = format!(
            "SELECT * FROM job {} ORDER BY priority DESC, created_at ASC {} {}",
            where_clause, limit_clause, offset_clause
        );

        let mut result = db.query(&query);

        for (name, value) in bindings {
            result = result.bind((name, value));
        }

        let mut response = result.await?;
        let records: Vec<JobRecord> = response.take(0)?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id_str = r.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
                let job_id = JobId::parse(&id_str).unwrap_or_else(|_| JobId::new());
                r.into_job(job_id)
            })
            .collect())
    }

    /// Get pending jobs for a queue, ordered by priority and creation time.
    pub async fn get_pending_for_queue(
        queue_id: QueueId,
        limit: usize,
    ) -> Result<Vec<Job>, DbError> {
        let db = get_db()?;

        let mut result = db
            .query(
                r#"
                SELECT * FROM job
                WHERE queue_id = $queue_id AND status.status = "pending"
                ORDER BY priority DESC, created_at ASC
                LIMIT $limit
                "#,
            )
            .bind(("queue_id", queue_id.to_string()))
            .bind(("limit", limit as i64))
            .await?;

        let records: Vec<JobRecord> = result.take(0)?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id_str = r.id.as_ref().map(|t| t.id.to_raw()).unwrap_or_default();
                let job_id = JobId::parse(&id_str).unwrap_or_else(|_| JobId::new());
                r.into_job(job_id)
            })
            .collect())
    }

    /// Update a job's status and attempts.
    pub async fn update_status(
        id: JobId,
        status: &JobStatus,
        attempts: u32,
    ) -> Result<Job, DbError> {
        let db = get_db()?;
        let status_clone = status.clone();

        // Use SurrealQL to set updated_at with time::now()
        let mut result = db
            .query(
                "UPDATE type::thing('job', $id) SET status = $status, attempts = $attempts, updated_at = time::now() RETURN AFTER",
            )
            .bind(("id", id.to_string()))
            .bind(("status", status_clone))
            .bind(("attempts", attempts))
            .await?;

        let records: Vec<JobRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.into_job(id))
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", id)))
    }

    /// Update a job.
    pub async fn update(job: &Job) -> Result<Job, DbError> {
        let db = get_db()?;

        let mut result = db
            .query(
                "UPDATE type::thing('job', $id) SET queue_id = $queue_id, job_type = $job_type, payload = $payload, priority = $priority, status = $status, attempts = $attempts, max_retries = $max_retries, timeout_secs = $timeout_secs, tags = $tags, updated_at = time::now() RETURN AFTER",
            )
            .bind(("id", job.id.to_string()))
            .bind(("queue_id", job.queue_id.to_string()))
            .bind(("job_type", job.job_type.clone()))
            .bind(("payload", job.payload.clone()))
            .bind(("priority", job.priority))
            .bind(("status", job.status.clone()))
            .bind(("attempts", job.attempts))
            .bind(("max_retries", job.max_retries))
            .bind(("timeout_secs", job.timeout_secs))
            .bind(("tags", job.tags.clone()))
            .await?;

        let records: Vec<JobRecord> = result.take(0)?;

        records
            .into_iter()
            .next()
            .map(|r| r.into_job(job.id))
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", job.id)))
    }

    /// Delete a job.
    pub async fn delete(id: JobId) -> Result<(), DbError> {
        let db = get_db()?;

        let _: Option<JobRecord> = db.delete(("job", id.to_string())).await?;

        Ok(())
    }

    /// Archive a completed/failed job to history and delete from active jobs.
    pub async fn archive(job: &Job) -> Result<(), DbError> {
        let db = get_db()?;

        // Determine final status and extract details
        let (final_status, attempts, duration_ms, error, result_summary) = match &job.status {
            JobStatus::Completed {
                started_at,
                completed_at,
                result,
            } => {
                let duration = (*completed_at - *started_at).num_milliseconds() as u64;
                (
                    "completed",
                    job.attempts.max(1),
                    Some(duration),
                    None,
                    Some(result.summary.clone()),
                )
            }
            JobStatus::Failed { error, .. } => {
                ("failed", job.attempts, None, Some(error.clone()), None)
            }
            JobStatus::Cancelled { reason, .. } => {
                ("cancelled", job.attempts, None, reason.clone(), None)
            }
            _ => return Ok(()), // Don't archive non-terminal jobs
        };

        let history = JobHistoryCreate {
            job_id: job.id.to_string(),
            queue_id: job.queue_id.to_string(),
            job_type: job.job_type.clone(),
            priority: job.priority.to_string(),
            final_status: final_status.to_string(),
            attempts,
            duration_ms,
            error,
            result_summary,
            tags: job.tags.clone(),
            created_at: job.created_at.to_rfc3339(),
        };

        // Create history record
        db.query("CREATE job_history CONTENT $data")
            .bind(("data", history))
            .await?;

        // Delete active job
        Self::delete(job.id).await?;

        Ok(())
    }

    /// Count jobs by status for a queue.
    pub async fn count_by_status(
        queue_id: QueueId,
    ) -> Result<std::collections::HashMap<String, u64>, DbError> {
        let db = get_db()?;

        let mut result = db
            .query(
                r#"
                SELECT status.status AS status_value, count() as count
                FROM job
                WHERE queue_id = $queue_id
                GROUP BY status_value
                "#,
            )
            .bind(("queue_id", queue_id.to_string()))
            .await?;

        #[derive(Deserialize)]
        struct StatusCount {
            status_value: Option<String>,
            count: i64,
        }

        let counts: Vec<StatusCount> = result.take(0)?;

        let mut map = std::collections::HashMap::new();
        for count in counts {
            if let Some(status) = count.status_value {
                map.insert(status, count.count as u64);
            }
        }

        Ok(map)
    }

    /// Get queue statistics from job counts.
    pub async fn get_queue_stats(queue_id: QueueId) -> Result<QueueStats, DbError> {
        let counts = Self::count_by_status(queue_id).await?;

        Ok(QueueStats {
            pending: counts.get("pending").copied().unwrap_or(0),
            running: counts.get("running").copied().unwrap_or(0),
            completed: counts.get("completed").copied().unwrap_or(0),
            failed: counts.get("failed").copied().unwrap_or(0),
            avg_duration_ms: None,    // TODO: Calculate from history
            throughput_per_min: None, // TODO: Calculate from history
        })
    }
}
