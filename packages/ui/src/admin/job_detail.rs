//! Job detail panel component.

use dioxus::prelude::*;
use queue_core::{Job, JobStatus};

use super::StatusBadge;

/// Props for JobDetail component.
#[derive(Props, Clone, PartialEq)]
pub struct JobDetailProps {
    /// The job to display.
    pub job: Job,
    /// Callback when close is clicked.
    pub on_close: EventHandler<()>,
    /// Callback when cancel is clicked.
    pub on_cancel: EventHandler<Job>,
    /// Callback when retry is clicked.
    pub on_retry: EventHandler<Job>,
}

/// Detail panel component for displaying full job information.
#[component]
pub fn JobDetail(props: JobDetailProps) -> Element {
    let job = props.job.clone();
    let status_str = job.status.as_str().to_string();
    let can_cancel = !job.status.is_terminal();
    let can_retry = job.status.can_retry();

    // Clone job for each closure that needs it
    let job_for_cancel = job.clone();
    let job_for_retry = job.clone();

    // Format timestamps
    let created = job.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let updated = job.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    // Extract status details
    let status_details = match &job.status {
        JobStatus::Running {
            started_at,
            worker_id,
        } => Some(format!(
            "Started at {} by {}",
            started_at.format("%H:%M:%S"),
            worker_id
        )),
        JobStatus::Completed {
            started_at,
            completed_at,
            result,
        } => {
            let duration = (*completed_at - *started_at).num_seconds();
            Some(format!("Completed in {}s: {}", duration, result.summary))
        }
        JobStatus::Failed {
            started_at,
            failed_at,
            error,
            attempts,
        } => {
            let duration = (*failed_at - *started_at).num_seconds();
            Some(format!(
                "Failed after {}s (attempt {}): {}",
                duration, attempts, error
            ))
        }
        JobStatus::Cancelled {
            cancelled_at,
            reason,
        } => {
            let reason_str = reason.as_deref().unwrap_or("No reason");
            Some(format!(
                "Cancelled at {}: {}",
                cancelled_at.format("%H:%M:%S"),
                reason_str
            ))
        }
        _ => None,
    };

    // Format payload
    let payload_json =
        serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| "{}".to_string());

    rsx! {
        div { class: "job-detail-panel",
            div { class: "job-detail-header",
                h3 { "Job Details" }
                button {
                    class: "btn-close",
                    onclick: move |_| props.on_close.call(()),
                    "x"
                }
            }

            div { class: "job-detail-content",
                div { class: "detail-row",
                    span { class: "detail-label", "ID" }
                    span { class: "detail-value", "{job.id}" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Type" }
                    span { class: "detail-value", "{job.job_type}" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Priority" }
                    span { class: "detail-value", "{job.priority}" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Status" }
                    StatusBadge { status: status_str }
                }

                if let Some(details) = status_details {
                    div { class: "detail-row",
                        span { class: "detail-label", "Details" }
                        span { class: "detail-value", "{details}" }
                    }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Created" }
                    span { class: "detail-value", "{created}" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Updated" }
                    span { class: "detail-value", "{updated}" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Timeout" }
                    span { class: "detail-value", "{job.timeout_secs}s" }
                }

                div { class: "detail-row",
                    span { class: "detail-label", "Max Retries" }
                    span { class: "detail-value", "{job.max_retries}" }
                }

                if !job.tags.is_empty() {
                    div { class: "detail-row",
                        span { class: "detail-label", "Tags" }
                        div { class: "tags",
                            for tag in job.tags.iter() {
                                span { class: "tag", "{tag}" }
                            }
                        }
                    }
                }

                div { class: "detail-section",
                    h4 { "Payload" }
                    pre { class: "payload-json", "{payload_json}" }
                }
            }

            div { class: "job-detail-actions",
                if can_cancel {
                    button {
                        class: "btn btn-cancel",
                        onclick: move |_| props.on_cancel.call(job_for_cancel.clone()),
                        "Cancel Job"
                    }
                }
                if can_retry {
                    button {
                        class: "btn btn-retry",
                        onclick: move |_| props.on_retry.call(job_for_retry.clone()),
                        "Retry Job"
                    }
                }
            }
        }
    }
}
