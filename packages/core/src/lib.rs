//! Core domain types for the job queue system.
//!
//! This crate contains shared types used across all packages:
//! - Job and JobStatus for work items
//! - Queue and QueueState for job containers
//! - Events for real-time updates

mod job;
mod queue;
mod events;

pub use job::{Job, JobId, JobStatus, JobResult, Priority};
pub use queue::{Queue, QueueId, QueueState, QueueConfig, QueueStats};
pub use events::JobEvent;
