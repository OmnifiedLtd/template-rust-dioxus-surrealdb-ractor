//! Queue management server functions.

use dioxus::prelude::*;
use queue_core::Queue;
#[cfg(feature = "server")]
use queue_core::QueueId;

/// Create a new queue.
#[post("/api/queues/create")]
pub async fn create_queue(
    name: String,
    description: Option<String>,
) -> Result<Queue, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::CreateQueue {
                name,
                description,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// List all queues.
#[get("/api/queues")]
pub async fn list_queues() -> Result<Vec<Queue>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        // Ensure job queue is initialized before accessing supervisor
        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::ListQueues { reply: tx.into() })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Get a queue by ID.
#[get("/api/queues/:id")]
pub async fn get_queue(id: String) -> Result<Option<Queue>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let queue_id = QueueId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::GetQueue {
                queue_id,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Get a queue by name.
#[get("/api/queues/by-name/:name")]
pub async fn get_queue_by_name(name: String) -> Result<Option<Queue>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::GetQueueByName {
                name,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Pause a queue.
#[post("/api/queues/:id/pause")]
pub async fn pause_queue(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let queue_id = QueueId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::PauseQueue {
                queue_id,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Resume a paused queue.
#[post("/api/queues/:id/resume")]
pub async fn resume_queue(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let queue_id = QueueId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::ResumeQueue {
                queue_id,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Delete a queue.
#[post("/api/queues/:id/delete")]
pub async fn delete_queue(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use actors::SupervisorMessage;
        use actors::global_registry;

        crate::ensure_initialized()
            .await
            .map_err(|e| ServerFnError::new(format!("Initialization failed: {}", e)))?;

        let queue_id = QueueId::parse(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid queue ID: {}", e)))?;

        let supervisor = global_registry()
            .get_supervisor()
            .ok_or_else(|| ServerFnError::new("Supervisor not available"))?;

        let (tx, rx) = actors::concurrency::oneshot();
        supervisor
            .send_message(SupervisorMessage::DeleteQueue {
                queue_id,
                reply: tx.into(),
            })
            .map_err(|e| ServerFnError::new(format!("Failed to send message: {}", e)))?;

        rx.await
            .map_err(|_| ServerFnError::new("Failed to receive response"))?
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}
