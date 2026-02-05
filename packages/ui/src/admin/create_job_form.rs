//! Create job form component.

use dioxus::prelude::*;

/// Props for CreateJobForm component.
#[derive(Props, Clone, PartialEq)]
pub struct CreateJobFormProps {
    /// Queue ID to create the job in.
    pub queue_id: String,
    /// Callback when job is created.
    pub on_created: EventHandler<()>,
    /// Callback when form is cancelled.
    pub on_cancel: EventHandler<()>,
}

/// Form component for creating a new job.
#[component]
pub fn CreateJobForm(props: CreateJobFormProps) -> Element {
    let mut job_type = use_signal(|| "echo".to_string());
    let mut payload = use_signal(|| r#"{"message": "Hello, world!"}"#.to_string());
    let mut priority = use_signal(|| "normal".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let queue_id = props.queue_id.clone();

    let submit = move |_| {
        let queue_id = queue_id.clone();
        let job_type_val = job_type();
        let payload_val = payload();
        let priority_val = priority();

        spawn(async move {
            submitting.set(true);
            error.set(None);

            // Parse payload as JSON
            let payload_json: serde_json::Value = match serde_json::from_str(&payload_val) {
                Ok(v) => v,
                Err(e) => {
                    error.set(Some(format!("Invalid JSON: {}", e)));
                    submitting.set(false);
                    return;
                }
            };

            let request = api::CreateJobRequest {
                queue_id,
                job_type: job_type_val,
                payload: payload_json,
                priority: Some(priority_val),
                max_retries: None,
                timeout_secs: None,
                tags: vec![],
            };

            match api::enqueue_job(request).await {
                Ok(_job) => {
                    props.on_created.call(());
                }
                Err(e) => {
                    error.set(Some(format!("Failed to create job: {}", e)));
                }
            }

            submitting.set(false);
        });
    };

    rsx! {
        div { class: "create-job-form",
            h3 { "Create New Job" }

            if let Some(err) = error() {
                div { class: "error-message", "{err}" }
            }

            div { class: "form-group",
                label { "Job Type" }
                select {
                    value: "{job_type}",
                    onchange: move |e| job_type.set(e.value()),

                    option { value: "echo", "Echo" }
                    option { value: "sleep", "Sleep" }
                    option { value: "fail", "Fail (for testing)" }
                    option { value: "persist-object", "Persist Object (S3/filesystem/memory)" }
                }
            }

            div { class: "form-group",
                label { "Priority" }
                select {
                    value: "{priority}",
                    onchange: move |e| priority.set(e.value()),

                    option { value: "low", "Low" }
                    option { value: "normal", "Normal" }
                    option { value: "high", "High" }
                    option { value: "critical", "Critical" }
                }
            }

            div { class: "form-group",
                label { "Payload (JSON)" }
                textarea {
                    rows: 5,
                    value: "{payload}",
                    oninput: move |e| payload.set(e.value()),
                }
            }

            div { class: "form-actions",
                button {
                    class: "btn btn-primary",
                    disabled: submitting(),
                    onclick: submit,
                    if submitting() { "Creating..." } else { "Create Job" }
                }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| props.on_cancel.call(()),
                    "Cancel"
                }
            }
        }
    }
}
