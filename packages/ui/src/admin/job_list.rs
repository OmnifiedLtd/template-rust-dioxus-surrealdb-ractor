//! Job list component for displaying jobs in a queue.

use dioxus::prelude::*;
use queue_core::Job;

use super::JobRow;

/// Props for JobList component.
#[derive(Props, Clone, PartialEq)]
pub struct JobListProps {
    /// List of jobs to display.
    pub jobs: Vec<Job>,
    /// Callback when a job is selected.
    pub on_select: EventHandler<Job>,
    /// Callback when cancel is clicked.
    pub on_cancel: EventHandler<Job>,
    /// Whether loading.
    #[props(default = false)]
    pub loading: bool,
}

/// List component for displaying jobs.
#[component]
pub fn JobList(props: JobListProps) -> Element {
    rsx! {
        div { class: "job-list",
            h3 { class: "job-list-title", "Jobs" }

            if props.loading {
                div { class: "loading", "Loading jobs..." }
            } else if props.jobs.is_empty() {
                div { class: "empty-state",
                    p { "No jobs in this queue" }
                }
            } else {
                table { class: "job-table",
                    thead {
                        tr {
                            th { "ID" }
                            th { "Type" }
                            th { "Priority" }
                            th { "Status" }
                            th { "Created" }
                            th { "Actions" }
                        }
                    }
                    tbody {
                        for job in props.jobs.iter() {
                            JobRow {
                                key: "{job.id}",
                                job: job.clone(),
                                on_select: props.on_select.clone(),
                                on_cancel: props.on_cancel.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
