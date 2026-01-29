//! Real-time event streaming via Server-Sent Events.

use queue_core::JobEvent;
use tokio::sync::broadcast;

/// Global event broadcaster.
static EVENT_TX: std::sync::LazyLock<broadcast::Sender<JobEvent>> =
    std::sync::LazyLock::new(|| {
        let (tx, _) = broadcast::channel(1024);
        tx
    });

/// Get the global event broadcaster.
pub fn event_broadcaster() -> broadcast::Sender<JobEvent> {
    EVENT_TX.clone()
}

/// Subscribe to the global event stream.
pub fn subscribe_events() -> broadcast::Receiver<JobEvent> {
    EVENT_TX.subscribe()
}

// Note: SSE endpoint would typically be implemented as a custom Axum route
// or using Dioxus's streaming capabilities. For now, we provide the
// subscription mechanism that can be used by the web server.

/// Helper to format an event for SSE.
pub fn format_sse_event(event: &JobEvent) -> String {
    let json = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
    format!("data: {}\n\n", json)
}
