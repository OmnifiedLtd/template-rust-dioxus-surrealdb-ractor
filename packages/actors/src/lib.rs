//! Actor system for the job queue.
//!
//! This crate provides the Ractor-based actor system for managing
//! job queues, workers, and the supervisor.
//!
//! # Architecture
//!
//! - `Supervisor` - Top-level actor that manages queue actors
//! - `QueueActor` - Manages a single queue's jobs and workers
//! - `WorkerActor` - Executes jobs from a queue
//!
//! # Usage
//!
//! ```ignore
//! use actors::{Supervisor, SupervisorMessage, start_supervisor};
//!
//! // Start the supervisor
//! let (supervisor, handle) = start_supervisor().await?;
//!
//! // Create a queue via message
//! supervisor.send_message(SupervisorMessage::CreateQueue { ... })?;
//! ```

mod handler;
mod messages;
mod persistence;
mod queue_actor;
pub mod registry;
mod supervisor;
mod worker_actor;

pub use handler::{FnHandler, HandlerResult, JobHandler, JobHandlerRegistry};
pub use messages::{QueueMessage, SupervisorMessage, WorkerMessage};
pub use persistence::StatePersistence;
pub use queue_actor::QueueActor;
pub use registry::{ActorRegistry, global_registry};
pub use supervisor::{Supervisor, start_supervisor};
pub use worker_actor::WorkerActor;

/// Re-export ractor types for convenience.
pub use ractor::{Actor, ActorRef, RpcReplyPort, concurrency};
