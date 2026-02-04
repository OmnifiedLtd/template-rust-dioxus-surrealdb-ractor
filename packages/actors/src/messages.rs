//! Message types for actor communication.

use queue_core::{Job, JobEvent, JobId, JobResult, Queue, QueueId, QueueStats};
use ractor::RpcReplyPort;

/// Messages for the QueueActor.
#[derive(Debug)]
pub enum QueueMessage {
    /// Enqueue a new job.
    Enqueue {
        job: Box<Job>,
        reply: RpcReplyPort<Result<Job, String>>,
    },

    /// Request the next job for a worker.
    RequestJob {
        worker_id: String,
        reply: RpcReplyPort<Option<Job>>,
    },

    /// Report job completion.
    JobCompleted {
        job_id: JobId,
        worker_id: String,
        result: JobResult,
    },

    /// Report job failure.
    JobFailed {
        job_id: JobId,
        worker_id: String,
        error: String,
    },

    /// Cancel a job.
    CancelJob {
        job_id: JobId,
        reason: Option<String>,
        reply: RpcReplyPort<Result<(), String>>,
    },

    /// Retry a failed job.
    RetryJob {
        job_id: JobId,
        reply: RpcReplyPort<Result<Job, String>>,
    },

    /// Get a job by ID.
    GetJob {
        job_id: JobId,
        reply: RpcReplyPort<Option<Job>>,
    },

    /// List jobs in this queue.
    ListJobs {
        status_filter: Option<String>,
        limit: usize,
        reply: RpcReplyPort<Vec<Job>>,
    },

    /// Pause the queue.
    Pause,

    /// Resume the queue.
    Resume,

    /// Get queue info.
    GetInfo { reply: RpcReplyPort<Queue> },

    /// Get queue stats.
    GetStats { reply: RpcReplyPort<QueueStats> },

    /// Shutdown the queue gracefully.
    Shutdown,

    /// Periodic tick for housekeeping.
    Tick,
}

/// Messages for the WorkerActor.
#[derive(Debug)]
pub enum WorkerMessage {
    /// Start working on a job.
    ProcessJob { job: Box<Job> },

    /// Stop current job (cancel).
    StopJob { reason: String },

    /// Check if worker is idle.
    IsIdle { reply: RpcReplyPort<bool> },

    /// Shutdown the worker.
    Shutdown,

    /// Heartbeat tick.
    Heartbeat,
}

/// Messages for the Supervisor.
#[derive(Debug)]
pub enum SupervisorMessage {
    /// Create a new queue.
    CreateQueue {
        name: String,
        description: Option<String>,
        reply: RpcReplyPort<Result<Queue, String>>,
    },
    /// Register an existing queue from persistence.
    RegisterQueue {
        queue: Queue,
        reply: RpcReplyPort<Result<Queue, String>>,
    },

    /// Get a queue by ID.
    GetQueue {
        queue_id: QueueId,
        reply: RpcReplyPort<Option<Queue>>,
    },

    /// Get a queue by name.
    GetQueueByName {
        name: String,
        reply: RpcReplyPort<Option<Queue>>,
    },

    /// List all queues.
    ListQueues { reply: RpcReplyPort<Vec<Queue>> },

    /// Pause a queue.
    PauseQueue {
        queue_id: QueueId,
        reply: RpcReplyPort<Result<(), String>>,
    },

    /// Resume a queue.
    ResumeQueue {
        queue_id: QueueId,
        reply: RpcReplyPort<Result<(), String>>,
    },

    /// Delete a queue.
    DeleteQueue {
        queue_id: QueueId,
        reply: RpcReplyPort<Result<(), String>>,
    },

    /// Enqueue a job to a specific queue.
    EnqueueJob {
        queue_id: QueueId,
        job: Job,
        reply: RpcReplyPort<Result<Job, String>>,
    },

    /// Get a job from any queue.
    GetJob {
        job_id: JobId,
        reply: RpcReplyPort<Option<Job>>,
    },

    /// Cancel a job.
    CancelJob {
        job_id: JobId,
        reason: Option<String>,
        reply: RpcReplyPort<Result<(), String>>,
    },

    /// Subscribe to events.
    Subscribe {
        sender: tokio::sync::broadcast::Sender<JobEvent>,
    },

    /// Broadcast an event to all subscribers.
    BroadcastEvent { event: JobEvent },

    /// Shutdown all queues.
    Shutdown,

    /// Periodic tick for housekeeping.
    Tick,
}

/// Result type for internal operations.
#[allow(dead_code)]
pub type ActorResult<T> = Result<T, ActorError>;

/// Error type for actor operations.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("Queue not found: {0}")]
    QueueNotFound(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Queue is paused")]
    QueuePaused,

    #[error("Queue is full")]
    QueueFull,

    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Timeout")]
    Timeout,
}
