//! Job row component for displaying a single job in a table.

use dioxus::prelude::*;
use queue_core::Job;

use super::StatusBadge;

/// Props for JobRow component.
#[derive(Props, Clone, PartialEq)]
pub struct JobRowProps {
    /// The job to display.
    pub job: Job,
    /// Callback when the job is clicked.
    pub on_select: EventHandler<Job>,
    /// Callback when cancel is clicked.
    pub on_cancel: EventHandler<Job>,
}

/// Table row component for displaying a single job.
#[component]
pub fn JobRow(props: JobRowProps) -> Element {
    let job = props.job.clone();
    let created = job.created_at.format("%H:%M:%S").to_string();
    let status_str = job.status.as_str().to_string();
    let can_cancel = !job.status.is_terminal() && status_str != "cancelled";

    // Clone job for each closure that needs it
    let job_for_select = job.clone();
    let job_for_cancel = job.clone();

    rsx! {
        tr {
            class: "job-row",
            onclick: move |_| props.on_select.call(job_for_select.clone()),

            td { class: "job-id", "{job.id}" }
            td { class: "job-type", "{job.job_type}" }
            td { class: "job-priority", "{job.priority}" }
            td { class: "job-status",
                StatusBadge { status: status_str }
            }
            td { class: "job-created", "{created}" }
            td { class: "job-actions",
                if can_cancel {
                    button {
                        class: "btn btn-small btn-cancel",
                        onclick: move |e| {
                            e.stop_propagation();
                            props.on_cancel.call(job_for_cancel.clone());
                        },
                        "Cancel"
                    }
                }
            }
        }
    }
}
