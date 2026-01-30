//! Main admin dashboard component.

use dioxus::prelude::*;
use queue_core::{Job, Queue};

use super::{QueueList, JobList, JobDetail, CreateJobForm};

/// Main admin dashboard component.
#[component]
pub fn AdminDashboard() -> Element {
    // State
    let mut queues = use_signal(Vec::<Queue>::new);
    let mut selected_queue = use_signal(|| None::<Queue>);
    let mut jobs = use_signal(Vec::<Job>::new);
    let mut selected_job = use_signal(|| None::<Job>);
    let mut loading_jobs = use_signal(|| false);
    let mut show_create_form = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load queues using use_resource for client-side async
    let queues_resource = use_resource(move || async move {
        api::list_queues().await.ok()
    });

    // Get queues from resource - this triggers re-render when resource updates
    let display_queues = match queues_resource.read().as_ref() {
        Some(Some(q)) => q.clone(),
        _ => Vec::new(),
    };

    // Keep signal in sync for mutations
    let queues_for_sync = display_queues.clone();
    use_effect(move || {
        if !queues_for_sync.is_empty() && queues().is_empty() {
            queues.set(queues_for_sync.clone());
        }
    });

    // Load jobs when queue is selected
    let load_jobs = move |queue: Queue| {
        let queue_id = queue.id.to_string();
        spawn(async move {
            loading_jobs.set(true);
            match api::list_queue_jobs(queue_id, None, Some(100)).await {
                Ok(j) => jobs.set(j),
                Err(e) => error.set(Some(format!("Failed to load jobs: {}", e))),
            }
            loading_jobs.set(false);
        });
    };

    // Queue selection handler
    let on_queue_select = move |queue: Queue| {
        selected_queue.set(Some(queue.clone()));
        selected_job.set(None);
        load_jobs(queue);
    };

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

    // Job selection handler
    let on_job_select = move |job: Job| {
        selected_job.set(Some(job));
    };

    // Cancel job handler
    let on_job_cancel = move |job: Job| {
        let job_id = job.id.to_string();
        let queue = selected_queue().clone();
        spawn(async move {
            if let Err(e) = api::cancel_job(job_id, Some("Cancelled from admin".to_string())).await {
                error.set(Some(format!("Failed to cancel job: {}", e)));
            } else if let Some(q) = queue {
                // Refresh jobs
                if let Ok(j) = api::list_queue_jobs(q.id.to_string(), None, Some(100)).await {
                    jobs.set(j);
                }
            }
        });
    };

    // Job created handler
    let on_job_created = move |_| {
        show_create_form.set(false);
        if let Some(q) = selected_queue().clone() {
            load_jobs(q);
        }
    };

    rsx! {
        div { class: "admin-dashboard",
            header { class: "admin-header",
                h1 { "Job Queue Admin" }
            }

            if let Some(err) = error() {
                div { class: "error-banner",
                    span { "{err}" }
                    button {
                        onclick: move |_| error.set(None),
                        "x"
                    }
                }
            }

            div { class: "admin-content",
                aside { class: "sidebar",
                    QueueList {
                        queues: display_queues.clone(),
                        selected_id: selected_queue().map(|q| q.id.to_string()),
                        on_select: on_queue_select,
                        on_pause: on_pause,
                        on_resume: on_resume,
                    }
                }

                main { class: "main-panel",
                    if let Some(queue) = selected_queue() {
                        div { class: "queue-detail",
                            div { class: "queue-detail-header",
                                h2 { "{queue.name}" }
                                button {
                                    class: "btn btn-primary",
                                    onclick: move |_| show_create_form.set(true),
                                    "+ New Job"
                                }
                            }

                            if show_create_form() {
                                CreateJobForm {
                                    queue_id: queue.id.to_string(),
                                    on_created: on_job_created,
                                    on_cancel: move |_| show_create_form.set(false),
                                }
                            }

                            JobList {
                                jobs: jobs(),
                                loading: loading_jobs(),
                                on_select: on_job_select,
                                on_cancel: on_job_cancel.clone(),
                            }
                        }
                    } else {
                        div { class: "no-selection",
                            p { "Select a queue to view jobs" }
                        }
                    }
                }

                if let Some(job) = selected_job() {
                    aside { class: "detail-panel",
                        JobDetail {
                            job: job.clone(),
                            on_close: move |_| selected_job.set(None),
                            on_cancel: on_job_cancel.clone(),
                            on_retry: move |_job: Job| {
                                // TODO: Implement retry API
                            },
                        }
                    }
                }
            }
        }
    }
}
