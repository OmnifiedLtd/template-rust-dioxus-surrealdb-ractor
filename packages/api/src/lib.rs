//! Server API functions for the job queue system.
//!
//! This crate contains all shared fullstack server functions for:
//! - Queue management (create, list, pause, resume)
//! - Job management (enqueue, get, cancel, retry)
//! - Real-time events (SSE streaming)

mod echo;
mod jobs;
mod queues;

pub use echo::echo;

#[cfg(feature = "server")]
mod init;

#[cfg(feature = "server")]
mod realtime;

// Re-export all server functions
pub use jobs::*;
pub use queues::*;

#[cfg(feature = "server")]
pub use init::*;

#[cfg(feature = "server")]
pub use realtime::*;

// Re-export core types for convenience
pub use queue_core::{
    Job, JobEvent, JobId, JobStatus, Priority, Queue, QueueId, QueueState, QueueStats,
};
