//! Event types for real-time updates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Job, JobId, JobStatus, Queue, QueueId, QueueState, QueueStats};

/// Events emitted by the job queue system for real-time updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum JobEvent {
    // Queue events
    /// A new queue was created.
    QueueCreated {
        queue: Queue,
        timestamp: DateTime<Utc>,
    },
    /// A queue's state changed (running, paused, etc.).
    QueueStateChanged {
        queue_id: QueueId,
        old_state: QueueState,
        new_state: QueueState,
        timestamp: DateTime<Utc>,
    },
    /// A queue's statistics were updated.
    QueueStatsUpdated {
        queue_id: QueueId,
        stats: QueueStats,
        timestamp: DateTime<Utc>,
    },
    /// A queue was deleted.
    QueueDeleted {
        queue_id: QueueId,
        timestamp: DateTime<Utc>,
    },

    // Job events
    /// A new job was enqueued.
    JobEnqueued { job: Job, timestamp: DateTime<Utc> },
    /// A job started executing.
    JobStarted {
        job_id: JobId,
        queue_id: QueueId,
        worker_id: String,
        timestamp: DateTime<Utc>,
    },
    /// A job completed successfully.
    JobCompleted {
        job_id: JobId,
        queue_id: QueueId,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// A job failed.
    JobFailed {
        job_id: JobId,
        queue_id: QueueId,
        error: String,
        attempts: u32,
        will_retry: bool,
        timestamp: DateTime<Utc>,
    },
    /// A job's status changed.
    JobStatusChanged {
        job_id: JobId,
        queue_id: QueueId,
        old_status: JobStatus,
        new_status: JobStatus,
        timestamp: DateTime<Utc>,
    },
    /// A job was cancelled.
    JobCancelled {
        job_id: JobId,
        queue_id: QueueId,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// A job is being retried.
    JobRetrying {
        job_id: JobId,
        queue_id: QueueId,
        attempt: u32,
        timestamp: DateTime<Utc>,
    },

    // Worker events
    /// A worker connected to a queue.
    WorkerConnected {
        worker_id: String,
        queue_id: QueueId,
        timestamp: DateTime<Utc>,
    },
    /// A worker disconnected from a queue.
    WorkerDisconnected {
        worker_id: String,
        queue_id: QueueId,
        timestamp: DateTime<Utc>,
    },
    /// A worker sent a heartbeat.
    WorkerHeartbeat {
        worker_id: String,
        queue_id: QueueId,
        current_job: Option<JobId>,
        timestamp: DateTime<Utc>,
    },
}

impl JobEvent {
    /// Get the timestamp of the event.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            JobEvent::QueueCreated { timestamp, .. } => *timestamp,
            JobEvent::QueueStateChanged { timestamp, .. } => *timestamp,
            JobEvent::QueueStatsUpdated { timestamp, .. } => *timestamp,
            JobEvent::QueueDeleted { timestamp, .. } => *timestamp,
            JobEvent::JobEnqueued { timestamp, .. } => *timestamp,
            JobEvent::JobStarted { timestamp, .. } => *timestamp,
            JobEvent::JobCompleted { timestamp, .. } => *timestamp,
            JobEvent::JobFailed { timestamp, .. } => *timestamp,
            JobEvent::JobStatusChanged { timestamp, .. } => *timestamp,
            JobEvent::JobCancelled { timestamp, .. } => *timestamp,
            JobEvent::JobRetrying { timestamp, .. } => *timestamp,
            JobEvent::WorkerConnected { timestamp, .. } => *timestamp,
            JobEvent::WorkerDisconnected { timestamp, .. } => *timestamp,
            JobEvent::WorkerHeartbeat { timestamp, .. } => *timestamp,
        }
    }

    /// Get the queue ID associated with this event, if any.
    pub fn queue_id(&self) -> Option<QueueId> {
        match self {
            JobEvent::QueueCreated { queue, .. } => Some(queue.id),
            JobEvent::QueueStateChanged { queue_id, .. } => Some(*queue_id),
            JobEvent::QueueStatsUpdated { queue_id, .. } => Some(*queue_id),
            JobEvent::QueueDeleted { queue_id, .. } => Some(*queue_id),
            JobEvent::JobEnqueued { job, .. } => Some(job.queue_id),
            JobEvent::JobStarted { queue_id, .. } => Some(*queue_id),
            JobEvent::JobCompleted { queue_id, .. } => Some(*queue_id),
            JobEvent::JobFailed { queue_id, .. } => Some(*queue_id),
            JobEvent::JobStatusChanged { queue_id, .. } => Some(*queue_id),
            JobEvent::JobCancelled { queue_id, .. } => Some(*queue_id),
            JobEvent::JobRetrying { queue_id, .. } => Some(*queue_id),
            JobEvent::WorkerConnected { queue_id, .. } => Some(*queue_id),
            JobEvent::WorkerDisconnected { queue_id, .. } => Some(*queue_id),
            JobEvent::WorkerHeartbeat { queue_id, .. } => Some(*queue_id),
        }
    }

    /// Get the job ID associated with this event, if any.
    pub fn job_id(&self) -> Option<JobId> {
        match self {
            JobEvent::JobEnqueued { job, .. } => Some(job.id),
            JobEvent::JobStarted { job_id, .. } => Some(*job_id),
            JobEvent::JobCompleted { job_id, .. } => Some(*job_id),
            JobEvent::JobFailed { job_id, .. } => Some(*job_id),
            JobEvent::JobStatusChanged { job_id, .. } => Some(*job_id),
            JobEvent::JobCancelled { job_id, .. } => Some(*job_id),
            JobEvent::JobRetrying { job_id, .. } => Some(*job_id),
            JobEvent::WorkerHeartbeat { current_job, .. } => *current_job,
            _ => None,
        }
    }

    /// Get a short description of this event for logging.
    pub fn description(&self) -> String {
        match self {
            JobEvent::QueueCreated { queue, .. } => format!("Queue '{}' created", queue.name),
            JobEvent::QueueStateChanged {
                new_state,
                queue_id,
                ..
            } => format!("Queue {} -> {}", queue_id, new_state),
            JobEvent::QueueStatsUpdated {
                queue_id, stats, ..
            } => {
                format!("Queue {} stats: {} pending", queue_id, stats.pending)
            }
            JobEvent::QueueDeleted { queue_id, .. } => format!("Queue {} deleted", queue_id),
            JobEvent::JobEnqueued { job, .. } => format!("Job {} enqueued", job.id),
            JobEvent::JobStarted {
                job_id, worker_id, ..
            } => format!("Job {} started by {}", job_id, worker_id),
            JobEvent::JobCompleted {
                job_id,
                duration_ms,
                ..
            } => format!("Job {} completed in {}ms", job_id, duration_ms),
            JobEvent::JobFailed {
                job_id,
                error,
                will_retry,
                ..
            } => {
                let retry = if *will_retry { " (will retry)" } else { "" };
                format!("Job {} failed: {}{}", job_id, error, retry)
            }
            JobEvent::JobStatusChanged {
                job_id, new_status, ..
            } => format!("Job {} -> {}", job_id, new_status.as_str()),
            JobEvent::JobCancelled { job_id, reason, .. } => {
                let reason = reason.as_deref().unwrap_or("no reason");
                format!("Job {} cancelled: {}", job_id, reason)
            }
            JobEvent::JobRetrying {
                job_id, attempt, ..
            } => {
                format!("Job {} retrying (attempt {})", job_id, attempt)
            }
            JobEvent::WorkerConnected {
                worker_id,
                queue_id,
                ..
            } => format!("Worker {} connected to {}", worker_id, queue_id),
            JobEvent::WorkerDisconnected {
                worker_id,
                queue_id,
                ..
            } => format!("Worker {} disconnected from {}", worker_id, queue_id),
            JobEvent::WorkerHeartbeat { worker_id, .. } => {
                format!("Worker {} heartbeat", worker_id)
            }
        }
    }
}
