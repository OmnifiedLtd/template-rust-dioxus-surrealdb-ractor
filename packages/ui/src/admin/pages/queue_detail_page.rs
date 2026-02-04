//! Queue detail page - displays a single queue with its jobs.

use dioxus::prelude::*;
use queue_core::{Job, Queue, QueueState};

use crate::admin::{CreateJobForm, StateBadge, StatusBadge};

/// Props for AdminQueueDetailPage.
#[derive(Props, Clone, PartialEq)]
pub struct AdminQueueDetailPageProps {
    pub queue_id: String,
}

/// Queue detail page component.
#[component]
pub fn AdminQueueDetailPage(props: AdminQueueDetailPageProps) -> Element {
    let queue_id = props.queue_id.clone();
    let mut queue = use_signal(|| None::<Queue>);
    let mut jobs = use_signal(Vec::<Job>::new);
    let mut loading = use_signal(|| true);
    let mut show_create_form = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load queue and jobs
    let queue_id_for_resource = queue_id.clone();
    let _queue_resource = use_resource(move || {
        let qid = queue_id_for_resource.clone();
        async move {
            loading.set(true);

            // Load queue details
            if let Ok(queues) = api::list_queues().await
                && let Some(q) = queues.into_iter().find(|q| q.id.to_string() == qid)
            {
                queue.set(Some(q));
            }

            // Load jobs
            if let Ok(j) = api::list_queue_jobs(qid, None, Some(100)).await {
                jobs.set(j);
            }

            loading.set(false);
        }
    });

    // Refresh jobs
    let refresh_jobs = {
        let qid = queue_id.clone();
        move || {
            let qid = qid.clone();
            spawn(async move {
                if let Ok(j) = api::list_queue_jobs(qid, None, Some(100)).await {
                    jobs.set(j);
                }
            });
        }
    };

    // Job created handler
    let on_job_created = {
        let refresh = refresh_jobs.clone();
        move |_| {
            show_create_form.set(false);
            refresh();
        }
    };

    // Pause/Resume handlers
    let on_pause = {
        let qid = queue_id.clone();
        move |_| {
            let qid = qid.clone();
            spawn(async move {
                if let Err(e) = api::pause_queue(qid.clone()).await {
                    error.set(Some(format!("Failed to pause queue: {}", e)));
                } else if let Ok(queues) = api::list_queues().await
                    && let Some(q) = queues.into_iter().find(|q| q.id.to_string() == qid)
                {
                    queue.set(Some(q));
                }
            });
        }
    };

    let on_resume = {
        let qid = queue_id.clone();
        move |_| {
            let qid = qid.clone();
            spawn(async move {
                if let Err(e) = api::resume_queue(qid.clone()).await {
                    error.set(Some(format!("Failed to resume queue: {}", e)));
                } else if let Ok(queues) = api::list_queues().await
                    && let Some(q) = queues.into_iter().find(|q| q.id.to_string() == qid)
                {
                    queue.set(Some(q));
                }
            });
        }
    };

    rsx! {
        div { class: "page-container",
            // Breadcrumb
            nav { class: "breadcrumb",
                Link { to: "/admin/queues", class: "breadcrumb-link", "Queues" }
                span { class: "breadcrumb-separator", "/" }
                span { class: "breadcrumb-current",
                    {queue().map(|q| q.name.clone()).unwrap_or_else(|| "Loading...".to_string())}
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

            if loading() {
                div { class: "loading", "Loading queue..." }
            } else if let Some(q) = queue() {
                // Page header
                div { class: "page-header",
                    div { class: "page-header-content",
                        div { class: "page-header-title-row",
                            h1 { class: "page-title", "{q.name}" }
                            StateBadge { state: q.state }
                        }
                        if let Some(ref desc) = q.description {
                            p { class: "page-description", "{desc}" }
                        }
                    }
                    div { class: "page-header-actions",
                        if q.state == QueueState::Paused {
                            button {
                                class: "btn btn-resume",
                                onclick: on_resume,
                                "Resume Queue"
                            }
                        } else {
                            button {
                                class: "btn btn-pause",
                                onclick: on_pause,
                                "Pause Queue"
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_create_form.set(true),
                            "+ New Job"
                        }
                    }
                }

                // Stats cards
                div { class: "stats-grid stats-grid-sm",
                    div { class: "stat-card",
                        div { class: "stat-card-value", "{q.stats.pending}" }
                        div { class: "stat-card-label", "Pending" }
                    }
                    div { class: "stat-card stat-card-accent",
                        div { class: "stat-card-value", "{q.stats.running}" }
                        div { class: "stat-card-label", "Running" }
                    }
                    div { class: "stat-card stat-card-success",
                        div { class: "stat-card-value", "{q.stats.completed}" }
                        div { class: "stat-card-label", "Completed" }
                    }
                    div { class: "stat-card stat-card-danger",
                        div { class: "stat-card-value", "{q.stats.failed}" }
                        div { class: "stat-card-label", "Failed" }
                    }
                }

                // Create job form (expandable)
                if show_create_form() {
                    CreateJobForm {
                        queue_id: queue_id.clone(),
                        on_created: on_job_created,
                        on_cancel: move |_| show_create_form.set(false),
                    }
                }

                // Jobs table
                div { class: "card",
                    div { class: "card-header",
                        h2 { class: "card-title", "Jobs" }
                        span { class: "card-count", "{jobs().len()} total" }
                    }

                    if jobs().is_empty() {
                        div { class: "empty-state",
                            div { class: "empty-state-icon", "📋" }
                            p { "No jobs in this queue" }
                            p { class: "hint", "Create a job to get started" }
                        }
                    } else {
                        div { class: "table-container",
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "ID" }
                                        th { "Type" }
                                        th { "Priority" }
                                        th { "Status" }
                                        th { "Created" }
                                        th { class: "text-right", "Actions" }
                                    }
                                }
                                tbody {
                                    for job in jobs().iter() {
                                        {
                                            let job_for_row = job.clone();
                                            let job_for_cancel = job.clone();
                                            let job_id = job.id.to_string();
                                            let queue_id_for_link = queue_id.clone();
                                            let created = job.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                                            let status_str = job.status.as_str().to_string();
                                            let can_cancel = !job.status.is_terminal();

                                            rsx! {
                                                tr { class: "data-row",
                                                    td {
                                                        Link {
                                                            to: "/admin/queues/{queue_id_for_link}/jobs/{job_id}",
                                                            class: "job-id-link",
                                                            "{job_for_row.id}"
                                                        }
                                                    }
                                                    td { class: "job-type-cell", "{job_for_row.job_type}" }
                                                    td { class: "capitalize", "{job_for_row.priority}" }
                                                    td {
                                                        StatusBadge { status: status_str }
                                                    }
                                                    td { class: "text-muted tabular-nums", "{created}" }
                                                    td { class: "text-right",
                                                        if can_cancel {
                                                            button {
                                                                class: "btn btn-small btn-cancel",
                                                                onclick: move |_| {
                                                                    let job_id = job_for_cancel.id.to_string();
                                                                    let qid = queue_id_for_link.clone();
                                                                    spawn(async move {
                                                                        if let Err(e) = api::cancel_job(job_id, Some("Cancelled from admin".to_string())).await {
                                                                            error.set(Some(format!("Failed to cancel job: {}", e)));
                                                                        } else if let Ok(j) = api::list_queue_jobs(qid, None, Some(100)).await {
                                                                            jobs.set(j);
                                                                        }
                                                                    });
                                                                },
                                                                "Cancel"
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
            } else {
                div { class: "empty-state",
                    p { "Queue not found" }
                }
            }
        }
    }
}
