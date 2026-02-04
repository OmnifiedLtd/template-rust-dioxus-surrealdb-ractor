//! Core domain types for the job queue system.
//!
//! This crate contains shared types used across all packages:
//! - Job and JobStatus for work items
//! - Queue and QueueState for job containers
//! - Events for real-time updates

mod events;
mod job;
mod queue;

pub use events::JobEvent;
pub use job::{Job, JobId, JobResult, JobStatus, Priority};
pub use queue::{Queue, QueueConfig, QueueId, QueueState, QueueStats};
