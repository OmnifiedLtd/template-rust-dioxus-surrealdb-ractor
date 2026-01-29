//! Queue card component for displaying queue info.

use dioxus::prelude::*;
use queue_core::{Queue, QueueState};

use super::StateBadge;

/// Props for QueueCard component.
#[derive(Props, Clone, PartialEq)]
pub struct QueueCardProps {
    /// The queue to display.
    pub queue: Queue,
    /// Whether this queue is selected.
    #[props(default = false)]
    pub selected: bool,
    /// Callback when the queue is clicked.
    pub on_select: EventHandler<Queue>,
    /// Callback when pause is clicked.
    pub on_pause: EventHandler<Queue>,
    /// Callback when resume is clicked.
    pub on_resume: EventHandler<Queue>,
}

/// Card component for displaying a single queue.
#[component]
pub fn QueueCard(props: QueueCardProps) -> Element {
    let queue = props.queue.clone();
    let is_paused = queue.state == QueueState::Paused;
    let selected_class = if props.selected { "selected" } else { "" };

    // Clone queue for each closure that needs it
    let queue_for_select = queue.clone();
    let queue_for_resume = queue.clone();
    let queue_for_pause = queue.clone();

    rsx! {
        div {
            class: "queue-card {selected_class}",
            onclick: move |_| props.on_select.call(queue_for_select.clone()),

            div { class: "queue-card-header",
                h3 { class: "queue-name", "{queue.name}" }
                StateBadge { state: queue.state }
            }

            if let Some(ref desc) = queue.description {
                p { class: "queue-description", "{desc}" }
            }

            div { class: "queue-stats",
                div { class: "stat",
                    span { class: "stat-value", "{queue.stats.pending}" }
                    span { class: "stat-label", "Pending" }
                }
                div { class: "stat",
                    span { class: "stat-value", "{queue.stats.running}" }
                    span { class: "stat-label", "Running" }
                }
                div { class: "stat",
                    span { class: "stat-value", "{queue.stats.completed}" }
                    span { class: "stat-label", "Completed" }
                }
                div { class: "stat",
                    span { class: "stat-value", "{queue.stats.failed}" }
                    span { class: "stat-label", "Failed" }
                }
            }

            div { class: "queue-actions",
                if is_paused {
                    button {
                        class: "btn btn-resume",
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_resume.call(queue_for_resume.clone());
                        },
                        "Resume"
                    }
                } else {
                    button {
                        class: "btn btn-pause",
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_pause.call(queue_for_pause.clone());
                        },
                        "Pause"
                    }
                }
            }
        }
    }
}
