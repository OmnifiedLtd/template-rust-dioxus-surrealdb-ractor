//! Job detail page - displays a single job with full details.

use dioxus::prelude::*;
use queue_core::{Job, JobStatus, Queue};

use crate::admin::StatusBadge;

/// Refresh interval in milliseconds (5 seconds).
const REFRESH_INTERVAL_MS: u32 = 5000;

/// Props for AdminJobDetailPage.
#[derive(Props, Clone, PartialEq)]
pub struct AdminJobDetailPageProps {
    pub queue_id: String,
    pub job_id: String,
}

/// Job detail page component.
#[component]
pub fn AdminJobDetailPage(props: AdminJobDetailPageProps) -> Element {
    let queue_id = props.queue_id.clone();
    let job_id = props.job_id.clone();

    let mut queue = use_signal(|| None::<Queue>);
    let mut job = use_signal(|| None::<Job>);
    let mut error = use_signal(|| None::<String>);

    // Auto-refresh: fetch job every 5 seconds
    let qid = queue_id.clone();
    let jid = job_id.clone();
    let _refresh = use_coroutine(move |_rx: UnboundedReceiver<()>| {
        let qid = qid.clone();
        let jid = jid.clone();
        async move {
            loop {
                // Load queue details for breadcrumb
                if let Ok(queues) = api::list_queues().await
                    && let Some(q) = queues.into_iter().find(|q| q.id.to_string() == qid)
                {
                    queue.set(Some(q));
                }

                // Load job
                if let Ok(Some(j)) = api::get_job(jid.clone()).await {
                    job.set(Some(j));
                }

                // Wait before next refresh
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(REFRESH_INTERVAL_MS).await;

                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(REFRESH_INTERVAL_MS as u64))
                    .await;
            }
        }
    });

    // Cancel job handler
    let job_id_for_cancel = job_id.clone();
    let on_cancel = move |_| {
        let jid = job_id_for_cancel.clone();
        spawn(async move {
            if let Err(e) =
                api::cancel_job(jid.clone(), Some("Cancelled from admin".to_string())).await
            {
                error.set(Some(format!("Failed to cancel job: {}", e)));
            } else if let Ok(Some(j)) = api::get_job(jid).await {
                job.set(Some(j));
            }
        });
    };

    rsx! {
        div { class: "page-container",
            // Breadcrumb
            nav { class: "breadcrumb",
                Link { to: "/admin/queues", class: "breadcrumb-link", "Queues" }
                span { class: "breadcrumb-separator", "/" }
                Link {
                    to: "/admin/queues/{queue_id}",
                    class: "breadcrumb-link",
                    {queue().map(|q| q.name.clone()).unwrap_or_else(|| "Queue".to_string())}
                }
                span { class: "breadcrumb-separator", "/" }
                span { class: "breadcrumb-current", "Job Details" }
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

            if let Some(j) = job() {
                {
                    let status_str = j.status.as_str().to_string();
                    let can_cancel = !j.status.is_terminal();
                    let created = j.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                    let updated = j.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                    let payload_json = serde_json::to_string_pretty(&j.payload).unwrap_or_else(|_| "{}".to_string());

                    // Extract status details
                    let status_details = match &j.status {
                        JobStatus::Running { started_at, worker_id } => Some(format!(
                            "Started at {} by worker {}",
                            started_at.format("%H:%M:%S"),
                            worker_id
                        )),
                        JobStatus::Completed { started_at, completed_at, result } => {
                            let duration = (*completed_at - *started_at).num_seconds();
                            Some(format!("Completed in {}s — {}", duration, result.summary))
                        }
                        JobStatus::Failed { started_at, failed_at, error, attempts } => {
                            let duration = (*failed_at - *started_at).num_seconds();
                            Some(format!("Failed after {}s (attempt {}) — {}", duration, attempts, error))
                        }
                        JobStatus::Cancelled { cancelled_at, reason } => {
                            let reason_str = reason.as_deref().unwrap_or("No reason provided");
                            Some(format!("Cancelled at {} — {}", cancelled_at.format("%H:%M:%S"), reason_str))
                        }
                        _ => None,
                    };

                    rsx! {
                        // Page header
                        div { class: "page-header",
                            div { class: "page-header-content",
                                div { class: "page-header-title-row",
                                    h1 { class: "page-title", "Job Details" }
                                    StatusBadge { status: status_str.clone() }
                                    span { class: "auto-refresh-indicator", "Auto-refreshing" }
                                }
                                p { class: "page-description job-id-display", "{j.id}" }
                            }
                            div { class: "page-header-actions",
                                if can_cancel {
                                    button {
                                        class: "btn btn-cancel",
                                        onclick: on_cancel,
                                        "Cancel Job"
                                    }
                                }
                            }
                        }

                        // Status message (if any)
                        if let Some(details) = status_details {
                            div { class: "status-message status-message-{status_str}",
                                "{details}"
                            }
                        }

                        // Job details cards
                        div { class: "detail-grid",
                            // Basic info card
                            div { class: "card",
                                div { class: "card-header",
                                    h2 { class: "card-title", "Basic Information" }
                                }
                                div { class: "card-body",
                                    div { class: "detail-list",
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Job Type" }
                                            span { class: "detail-value", "{j.job_type}" }
                                        }
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Priority" }
                                            span { class: "detail-value capitalize", "{j.priority}" }
                                        }
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Queue" }
                                            span { class: "detail-value", "{j.queue_id}" }
                                        }
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Created" }
                                            span { class: "detail-value tabular-nums", "{created}" }
                                        }
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Updated" }
                                            span { class: "detail-value tabular-nums", "{updated}" }
                                        }
                                    }
                                }
                            }

                            // Configuration card
                            div { class: "card",
                                div { class: "card-header",
                                    h2 { class: "card-title", "Configuration" }
                                }
                                div { class: "card-body",
                                    div { class: "detail-list",
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Timeout" }
                                            span { class: "detail-value", "{j.timeout_secs} seconds" }
                                        }
                                        div { class: "detail-item",
                                            span { class: "detail-label", "Max Retries" }
                                            span { class: "detail-value", "{j.max_retries}" }
                                        }
                                        if !j.tags.is_empty() {
                                            div { class: "detail-item",
                                                span { class: "detail-label", "Tags" }
                                                div { class: "tags",
                                                    for tag in j.tags.iter() {
                                                        span { class: "tag", "{tag}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Payload card (full width)
                        div { class: "card",
                            div { class: "card-header",
                                h2 { class: "card-title", "Payload" }
                            }
                            div { class: "card-body",
                                pre { class: "payload-json", "{payload_json}" }
                            }
                        }
                    }
                }
            } else {
                div { class: "loading", "Loading job..." }
            }
        }
    }
}
