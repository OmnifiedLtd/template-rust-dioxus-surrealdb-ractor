//! Queues list page - displays all queues with stats.

use dioxus::prelude::*;
use queue_core::{Queue, QueueState};

use crate::admin::StateBadge;

/// Queues list page component.
#[component]
pub fn AdminQueuesPage() -> Element {
    let mut queues = use_signal(Vec::<Queue>::new);
    let mut error = use_signal(|| None::<String>);
    let mut initialized = use_signal(|| false);

    // Load queues
    let queues_resource = use_resource(move || async move { api::list_queues().await.ok() });

    use_effect(move || {
        if initialized() {
            return;
        }
        if let Some(Some(q)) = queues_resource.read().as_ref() {
            queues.set(q.clone());
            initialized.set(true);
        }
    });

    // Pause queue handler
    let on_pause = move |queue: Queue| {
        let queue_id = queue.id.to_string();
        spawn(async move {
            if let Err(e) = api::pause_queue(queue_id).await {
                error.set(Some(format!("Failed to pause queue: {}", e)));
            } else if let Ok(q) = api::list_queues().await {
                queues.set(q);
            }
        });
    };

    // Resume queue handler
    let on_resume = move |queue: Queue| {
        let queue_id = queue.id.to_string();
        spawn(async move {
            if let Err(e) = api::resume_queue(queue_id).await {
                error.set(Some(format!("Failed to resume queue: {}", e)));
            } else if let Ok(q) = api::list_queues().await {
                queues.set(q);
            }
        });
    };

    rsx! {
        div { class: "page-container",
            // Page header
            div { class: "page-header",
                div { class: "page-header-content",
                    h1 { class: "page-title", "Queues" }
                    p { class: "page-description", "Manage your job queues and monitor their status" }
                }
            }

            // Error banner
            if let Some(err) = error() {
                div { class: "error-banner",
                    span { "{err}" }
                    button {
                        onclick: move |_| error.set(None),
                        "×"
                    }
                }
            }

            // Stats summary
            div { class: "stats-grid",
                div { class: "stat-card",
                    div { class: "stat-card-value", "{queues().len()}" }
                    div { class: "stat-card-label", "Total Queues" }
                }
                div { class: "stat-card",
                    div { class: "stat-card-value",
                        {queues().iter().filter(|q| q.state == QueueState::Running).count().to_string()}
                    }
                    div { class: "stat-card-label", "Running" }
                }
                div { class: "stat-card",
                    div { class: "stat-card-value",
                        {queues().iter().map(|q| q.stats.pending).sum::<u64>().to_string()}
                    }
                    div { class: "stat-card-label", "Pending Jobs" }
                }
                div { class: "stat-card stat-card-accent",
                    div { class: "stat-card-value",
                        {queues().iter().map(|q| q.stats.running).sum::<u64>().to_string()}
                    }
                    div { class: "stat-card-label", "Running Jobs" }
                }
            }

            // Queues table
            div { class: "card",
                div { class: "card-header",
                    h2 { class: "card-title", "All Queues" }
                }

                if queues().is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", "▦" }
                        p { "No queues found" }
                        p { class: "hint", "Queues will appear here when created" }
                    }
                } else {
                    div { class: "table-container",
                        table { class: "data-table",
                            thead {
                                tr {
                                    th { "Name" }
                                    th { "Status" }
                                    th { class: "text-right", "Pending" }
                                    th { class: "text-right", "Running" }
                                    th { class: "text-right", "Completed" }
                                    th { class: "text-right", "Failed" }
                                    th { class: "text-right", "Actions" }
                                }
                            }
                            tbody {
                                for queue in queues().iter() {
                                    {
                                        let queue_for_action = queue.clone();
                                        let queue_for_pause = queue.clone();
                                        let queue_for_resume = queue.clone();
                                        let is_paused = queue.state == QueueState::Paused;
                                        let queue_id = queue.id.to_string();

                                        rsx! {
                                            tr { class: "data-row",
                                                td {
                                                    Link {
                                                        to: "/admin/queues/{queue_id}",
                                                        class: "queue-link",
                                                        div { class: "queue-link-content",
                                                            span { class: "queue-link-name", "{queue_for_action.name}" }
                                                            if let Some(ref desc) = queue_for_action.description {
                                                                span { class: "queue-link-desc", "{desc}" }
                                                            }
                                                        }
                                                    }
                                                }
                                                td {
                                                    StateBadge { state: queue_for_action.state }
                                                }
                                                td { class: "text-right tabular-nums", "{queue_for_action.stats.pending}" }
                                                td { class: "text-right tabular-nums", "{queue_for_action.stats.running}" }
                                                td { class: "text-right tabular-nums", "{queue_for_action.stats.completed}" }
                                                td { class: "text-right tabular-nums", "{queue_for_action.stats.failed}" }
                                                td { class: "text-right",
                                                    if is_paused {
                                                        button {
                                                            class: "btn btn-small btn-resume",
                                                            onclick: move |_| on_resume(queue_for_resume.clone()),
                                                            "Resume"
                                                        }
                                                    } else {
                                                        button {
                                                            class: "btn btn-small btn-pause",
                                                            onclick: move |_| on_pause(queue_for_pause.clone()),
                                                            "Pause"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
