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

mod messages;
mod queue_actor;
mod worker_actor;
mod supervisor;
mod persistence;
pub mod registry;
mod handler;

pub use messages::{QueueMessage, WorkerMessage, SupervisorMessage};
pub use queue_actor::QueueActor;
pub use worker_actor::WorkerActor;
pub use supervisor::{Supervisor, start_supervisor};
pub use persistence::StatePersistence;
pub use registry::{ActorRegistry, global_registry};
pub use handler::{JobHandler, JobHandlerRegistry, HandlerResult, FnHandler};

/// Re-export ractor types for convenience.
pub use ractor::{Actor, ActorRef, RpcReplyPort, concurrency};
