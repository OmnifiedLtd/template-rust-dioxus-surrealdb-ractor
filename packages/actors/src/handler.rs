//! Job handler trait and registry.

use queue_core::{Job, JobResult};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Result type for job handlers.
pub type HandlerResult = Result<JobResult, String>;

/// Future type for async job handlers.
pub type HandlerFuture = Pin<Box<dyn Future<Output = HandlerResult> + Send>>;

/// Trait for job handlers.
///
/// Implement this trait to define how jobs of a specific type are processed.
pub trait JobHandler: Send + Sync + 'static {
    /// The job type this handler processes.
    fn job_type(&self) -> &str;

    /// Process a job and return the result.
    fn handle(&self, job: &Job) -> HandlerFuture;
}

/// Registry for job handlers.
///
/// Maps job types to their handlers for dynamic dispatch.
#[derive(Default)]
pub struct JobHandlerRegistry {
    handlers: HashMap<String, Arc<dyn JobHandler>>,
}

impl JobHandlerRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a job type.
    pub fn register<H: JobHandler>(&mut self, handler: H) {
        let job_type = handler.job_type().to_string();
        self.handlers.insert(job_type, Arc::new(handler));
    }

    /// Get a handler for a job type.
    pub fn get(&self, job_type: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(job_type).cloned()
    }

    /// Check if a handler exists for a job type.
    pub fn has_handler(&self, job_type: &str) -> bool {
        self.handlers.contains_key(job_type)
    }

    /// List all registered job types.
    pub fn job_types(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

/// A simple function-based job handler.
pub struct FnHandler<F>
where
    F: Fn(&Job) -> HandlerFuture + Send + Sync + 'static,
{
    job_type: String,
    handler: F,
}

impl<F> FnHandler<F>
where
    F: Fn(&Job) -> HandlerFuture + Send + Sync + 'static,
{
    /// Create a new function-based handler.
    pub fn new(job_type: impl Into<String>, handler: F) -> Self {
        Self {
            job_type: job_type.into(),
            handler,
        }
    }
}

impl<F> JobHandler for FnHandler<F>
where
    F: Fn(&Job) -> HandlerFuture + Send + Sync + 'static,
{
    fn job_type(&self) -> &str {
        &self.job_type
    }

    fn handle(&self, job: &Job) -> HandlerFuture {
        (self.handler)(job)
    }
}

/// Helper macro for creating job handlers from async closures.
#[macro_export]
macro_rules! job_handler {
    ($job_type:expr, |$job:ident| $body:expr) => {
        $crate::FnHandler::new($job_type, |$job: &core::Job| {
            let $job = $job.clone();
            Box::pin(async move { $body })
        })
    };
}
