//! Status and state badge components.

use dioxus::prelude::*;
use queue_core::QueueState;

/// Badge for displaying job status.
#[component]
pub fn StatusBadge(status: String) -> Element {
    let (bg_class, text) = match status.as_str() {
        "pending" => ("badge-pending", "Pending"),
        "running" => ("badge-running", "Running"),
        "completed" => ("badge-completed", "Completed"),
        "failed" => ("badge-failed", "Failed"),
        "cancelled" => ("badge-cancelled", "Cancelled"),
        "paused" => ("badge-paused", "Paused"),
        _ => ("badge-default", status.as_str()),
    };

    rsx! {
        span {
            class: "status-badge {bg_class}",
            {text}
        }
    }
}

/// Badge for displaying queue state.
#[component]
pub fn StateBadge(state: QueueState) -> Element {
    let (bg_class, text) = match state {
        QueueState::Running => ("badge-running", "Running"),
        QueueState::Paused => ("badge-paused", "Paused"),
        QueueState::Draining => ("badge-draining", "Draining"),
        QueueState::Stopped => ("badge-stopped", "Stopped"),
    };

    rsx! {
        span {
            class: "state-badge {bg_class}",
            {text}
        }
    }
}
