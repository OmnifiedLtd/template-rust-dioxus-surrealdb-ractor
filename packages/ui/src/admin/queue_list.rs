//! Queue list component.

use dioxus::prelude::*;
use queue_core::Queue;

use super::QueueCard;

/// Props for QueueList component.
#[derive(Props, Clone, PartialEq)]
pub struct QueueListProps {
    /// List of queues to display.
    pub queues: Vec<Queue>,
    /// Currently selected queue ID (as string).
    #[props(default)]
    pub selected_id: Option<String>,
    /// Callback when a queue is selected.
    pub on_select: EventHandler<Queue>,
    /// Callback when pause is clicked.
    pub on_pause: EventHandler<Queue>,
    /// Callback when resume is clicked.
    pub on_resume: EventHandler<Queue>,
}

/// List component for displaying all queues.
#[component]
pub fn QueueList(props: QueueListProps) -> Element {
    let selected_id = props.selected_id.clone();

    rsx! {
        div { class: "queue-list",
            h2 { class: "queue-list-title", "Queues" }

            if props.queues.is_empty() {
                div { class: "empty-state",
                    p { "No queues found" }
                    p { class: "hint", "Create a queue to get started" }
                }
            } else {
                for queue in props.queues.iter() {
                    QueueCard {
                        key: "{queue.id}",
                        queue: queue.clone(),
                        selected: selected_id.as_ref().map_or(false, |id| id == &queue.id.to_string()),
                        on_select: props.on_select.clone(),
                        on_pause: props.on_pause.clone(),
                        on_resume: props.on_resume.clone(),
                    }
                }
            }
        }
    }
}
