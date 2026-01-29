//! Server initialization for the job queue system.

use actors::{JobHandlerRegistry, start_supervisor, FnHandler};
use actors::global_registry;
use queue_core::{Job, JobResult};
use db::{DbConfig, init as init_db};

/// Initialize the job queue system.
///
/// This should be called once at server startup before handling requests.
pub async fn init_job_queue() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing job queue system...");

    // Initialize database
    let db_config = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        // Railway deployment - use file-based storage
        DbConfig::file("./data/surrealdb")
    } else {
        // Local development - use in-memory
        DbConfig::memory()
    };

    init_db(db_config).await?;

    // Create handler registry with demo handlers
    let mut handlers = JobHandlerRegistry::new();

    // Demo: Echo handler
    handlers.register(FnHandler::new("echo", |job: &Job| {
        let payload = job.payload.clone();
        Box::pin(async move {
            tracing::info!("Echo job: {:?}", payload);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Ok(JobResult::with_output(
                "Echo completed",
                payload,
            ))
        })
    }));

    // Demo: Sleep handler
    handlers.register(FnHandler::new("sleep", |job: &Job| {
        let seconds = job.payload.get("seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        Box::pin(async move {
            tracing::info!("Sleeping for {} seconds", seconds);
            tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
            Ok(JobResult::new(format!("Slept for {} seconds", seconds)))
        })
    }));

    // Demo: Failing handler (for testing retries)
    handlers.register(FnHandler::new("fail", |job: &Job| {
        let should_fail = job.payload.get("fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        Box::pin(async move {
            if should_fail {
                Err("Intentional failure".into())
            } else {
                Ok(JobResult::new("Success"))
            }
        })
    }));

    // Start supervisor
    let (supervisor, _handle) = start_supervisor(handlers).await?;

    // Register globally
    global_registry().register_supervisor(supervisor.clone());

    // Create a default "demo" queue if none exist
    let queues = db::repositories::QueueRepository::list().await.unwrap_or_default();
    if queues.is_empty() {
        tracing::info!("Creating demo queue...");
        let (tx, rx) = actors::concurrency::oneshot();
        supervisor.send_message(actors::SupervisorMessage::CreateQueue {
            name: "demo".to_string(),
            description: Some("Demo queue for testing".to_string()),
            reply: tx.into(),
        })?;

        match rx.await {
            Ok(Ok(queue)) => tracing::info!("Created demo queue: {}", queue.id),
            Ok(Err(e)) => tracing::warn!("Failed to create demo queue: {}", e),
            Err(_) => tracing::warn!("Timeout creating demo queue"),
        }
    }

    tracing::info!("Job queue system initialized");
    Ok(())
}
