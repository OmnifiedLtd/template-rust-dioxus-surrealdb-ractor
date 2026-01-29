//! Job repository for CRUD operations.

use queue_core::{Job, JobId, JobStatus, Priority, QueueId, QueueStats};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use crate::{get_db, DbError};

/// Repository for job persistence operations.
pub struct JobRepository;

/// Internal record type for SurrealDB.
#[derive(Debug, Serialize, Deserialize)]
struct JobRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Thing>,
    #[serde(flatten)]
    job: Job,
}

/// Job history record for archival.
#[derive(Debug, Serialize, Deserialize)]
pub struct JobHistoryRecord {
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
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

impl JobRepository {
    /// Create a new job in the database.
    pub async fn create(job: &Job) -> Result<Job, DbError> {
        let db = get_db();
        let job_clone = job.clone();

        let record: Option<JobRecord> = db
            .create(("job", job.id.to_string()))
            .content(job_clone)
            .await?;

        record
            .map(|r| r.job)
            .ok_or_else(|| DbError::Query("Failed to create job".into()))
    }

    /// Get a job by ID.
    pub async fn get(id: JobId) -> Result<Job, DbError> {
        let db = get_db();

        let record: Option<JobRecord> = db.select(("job", id.to_string())).await?;

        record
            .map(|r| r.job)
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", id)))
    }

    /// List jobs with optional filtering.
    pub async fn list(filter: JobFilter) -> Result<Vec<Job>, DbError> {
        let db = get_db();

        let mut conditions = Vec::new();
        let mut bindings: Vec<(&str, serde_json::Value)> = Vec::new();

        if let Some(queue_id) = &filter.queue_id {
            conditions.push("queue_id = $queue_id");
            bindings.push(("queue_id", serde_json::json!(queue_id.to_string())));
        }

        if let Some(status) = &filter.status {
            conditions.push("status.status = $status");
            bindings.push(("status", serde_json::json!(status)));
        }

        if let Some(job_type) = &filter.job_type {
            conditions.push("job_type = $job_type");
            bindings.push(("job_type", serde_json::json!(job_type)));
        }

        if let Some(priority) = &filter.priority {
            conditions.push("priority = $priority");
            bindings.push(("priority", serde_json::json!(priority.to_string())));
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

        Ok(records.into_iter().map(|r| r.job).collect())
    }

    /// Get pending jobs for a queue, ordered by priority and creation time.
    pub async fn get_pending_for_queue(queue_id: QueueId, limit: usize) -> Result<Vec<Job>, DbError> {
        let db = get_db();

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

        Ok(records.into_iter().map(|r| r.job).collect())
    }

    /// Update a job's status.
    pub async fn update_status(id: JobId, status: &JobStatus) -> Result<Job, DbError> {
        let db = get_db();

        let record: Option<JobRecord> = db
            .update(("job", id.to_string()))
            .merge(serde_json::json!({
                "status": status,
                "updated_at": chrono::Utc::now()
            }))
            .await?;

        record
            .map(|r| r.job)
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", id)))
    }

    /// Update a job.
    pub async fn update(job: &Job) -> Result<Job, DbError> {
        let db = get_db();

        let mut updated = job.clone();
        updated.updated_at = chrono::Utc::now();
        let job_id = job.id.to_string();

        let record: Option<JobRecord> = db
            .update(("job", job_id))
            .content(updated)
            .await?;

        record
            .map(|r| r.job)
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", job.id)))
    }

    /// Delete a job.
    pub async fn delete(id: JobId) -> Result<(), DbError> {
        let db = get_db();

        let _: Option<JobRecord> = db.delete(("job", id.to_string())).await?;

        Ok(())
    }

    /// Archive a completed/failed job to history and delete from active jobs.
    pub async fn archive(job: &Job) -> Result<(), DbError> {
        let db = get_db();

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
                    1u32,
                    Some(duration),
                    None,
                    Some(result.summary.clone()),
                )
            }
            JobStatus::Failed {
                error, attempts, ..
            } => ("failed", *attempts, None, Some(error.clone()), None),
            JobStatus::Cancelled { reason, .. } => {
                ("cancelled", 1u32, None, reason.clone(), None)
            }
            _ => return Ok(()), // Don't archive non-terminal jobs
        };

        let history = JobHistoryRecord {
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
            created_at: job.created_at,
            completed_at: chrono::Utc::now(),
        };

        // Create history record
        let _: Option<serde_json::Value> = db.create("job_history").content(history).await?;

        // Delete active job
        Self::delete(job.id).await?;

        Ok(())
    }

    /// Count jobs by status for a queue.
    pub async fn count_by_status(queue_id: QueueId) -> Result<std::collections::HashMap<String, u64>, DbError> {
        let db = get_db();

        let mut result = db
            .query(
                r#"
                SELECT status.status, count() as count
                FROM job
                WHERE queue_id = $queue_id
                GROUP BY status.status
                "#,
            )
            .bind(("queue_id", queue_id.to_string()))
            .await?;

        #[derive(Deserialize)]
        struct StatusCount {
            status: Option<String>,
            count: i64,
        }

        let counts: Vec<StatusCount> = result.take(0)?;

        let mut map = std::collections::HashMap::new();
        for count in counts {
            if let Some(status) = count.status {
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
            avg_duration_ms: None,  // TODO: Calculate from history
            throughput_per_min: None, // TODO: Calculate from history
        })
    }
}
