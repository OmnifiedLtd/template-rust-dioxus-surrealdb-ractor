//! Job management server functions.

use queue_core::{Job, JobId, Priority, QueueId};
use dioxus::prelude::*;
use serde_json::Value as JsonValue;

/// Request type for creating a job.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateJobRequest {
    pub queue_id: String,
    pub job_type: String,
    pub payload: JsonValue,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Enqueue a new job.
#[post("/api/jobs/enqueue")]
pub async fn enqueue_job(request: CreateJobRequest) -> Result<Job, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        let queue_id = QueueId::parse(&request.queue_id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let priority = request.priority
            .as_deref()
            .map(|p| match p {
                "low" => Priority::Low,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => Priority::Normal,
            })
            .unwrap_or(Priority::Normal);

        let mut job = Job::new(queue_id, &request.job_type, request.payload.clone())
            .with_priority(priority)
            .with_tags(request.tags);

        if let Some(max_retries) = request.max_retries {
            job = job.with_max_retries(max_retries);
        }
        if let Some(timeout) = request.timeout_secs {
            job = job.with_timeout(timeout);
        }

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::EnqueueJob {
                queue_id,
                job,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(|e| ServerFnError::new(e))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Get a job by ID.
#[get("/api/jobs/:id")]
pub async fn get_job(id: String) -> Result<Option<Job>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        let job_id = JobId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid job ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::GetJob { job_id, reply: tx.into() })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Cancel a job.
#[post("/api/jobs/:id/cancel")]
pub async fn cancel_job(id: String, reason: Option<String>) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        let job_id = JobId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid job ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::CancelJob { job_id, reason, reply: tx.into() })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(|e| ServerFnError::new(e))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// List jobs in a queue.
#[get("/api/queues/:queue_id/jobs")]
pub async fn list_queue_jobs(
    queue_id: String,
    status: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<Job>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use db::repositories::JobRepository;

        let queue_id = QueueId::parse(&queue_id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let filter = db::repositories::JobFilter {
            queue_id: Some(queue_id),
            status,
            limit: Some(limit.unwrap_or(100)),
            ..Default::default()
        };

        JobRepository::list(filter)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}
