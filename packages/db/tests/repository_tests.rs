#![allow(clippy::disallowed_methods)]

mod common;

use chrono::Utc;
use queue_core::{Job, JobResult, JobStatus, Priority, Queue, QueueConfig, QueueState, QueueStats};
use serde_json::{Map, Value};
use std::error::Error;

use db::{DbError, repositories::JobRepository, repositories::QueueRepository};

fn payload_with_message(message: &str) -> Value {
    let mut map = Map::new();
    map.insert("msg".to_string(), Value::String(message.to_string()));
    Value::Object(map)
}

async fn reset_db() -> Result<(), DbError> {
    let db_conn = db::get_db()?;
    db_conn
        .query("DELETE job_history; DELETE job; DELETE queue;")
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_repositories() -> Result<(), Box<dyn Error>> {
    let _guard = common::setup_db().await?;

    // QueueRepository: create/get/update/delete/exists
    let mut queue = Queue::new("alpha");
    let created = QueueRepository::create(&queue).await?;
    assert_eq!(created.name, "alpha");

    let loaded = QueueRepository::get(queue.id).await?;
    assert_eq!(loaded.id, queue.id);

    let updated_state = QueueRepository::update_state(queue.id, QueueState::Paused).await?;
    assert_eq!(updated_state.state, QueueState::Paused);

    let stats = QueueStats {
        pending: 1,
        running: 2,
        completed: 3,
        failed: 4,
        avg_duration_ms: Some(10.5),
        throughput_per_min: Some(2.25),
    };
    let updated_stats = QueueRepository::update_stats(queue.id, &stats).await?;
    assert_eq!(updated_stats.stats.pending, 1);
    assert_eq!(updated_stats.stats.completed, 3);

    queue.description = Some("updated".to_string());
    queue.config = QueueConfig {
        concurrency: 2,
        default_timeout_secs: 120,
        default_max_retries: 1,
        max_queue_size: Some(10),
        rate_limit: Some(5.0),
    };
    let updated = QueueRepository::update(&queue).await?;
    assert_eq!(updated.description.as_deref(), Some("updated"));
    assert_eq!(updated.config.concurrency, 2);

    let exists = QueueRepository::exists(queue.id).await?;
    assert!(exists);

    QueueRepository::delete(queue.id).await?;
    let exists_after = QueueRepository::exists(queue.id).await?;
    assert!(!exists_after);

    let missing = QueueRepository::get(queue.id).await;
    assert!(matches!(missing, Err(DbError::NotFound(_))));

    // QueueRepository: list, get_by_name, name_exists, list_by_state, duplicate name
    reset_db().await?;
    let queue_a = Queue::new("queue-a");
    let queue_b = Queue::new("queue-b");
    QueueRepository::create(&queue_a).await?;
    QueueRepository::create(&queue_b).await?;

    let list = QueueRepository::list().await?;
    assert!(list.len() >= 2);

    let by_name = QueueRepository::get_by_name("queue-a").await?;
    assert_eq!(by_name.name, "queue-a");

    let name_exists = QueueRepository::name_exists("queue-a").await?;
    assert!(name_exists);

    let missing_name = QueueRepository::name_exists("missing").await?;
    assert!(!missing_name);

    QueueRepository::update_state(queue_b.id, QueueState::Paused).await?;
    let paused = QueueRepository::list_by_state(QueueState::Paused).await?;
    assert!(paused.iter().all(|q| q.state == QueueState::Paused));

    let duplicate = QueueRepository::create(&Queue::new("queue-a")).await;
    assert!(duplicate.is_err());

    // JobRepository: create/get/update_status/update/delete
    reset_db().await?;
    let queue = Queue::new("jobs");
    QueueRepository::create(&queue).await?;

    let mut job = Job::new(queue.id, "echo", payload_with_message("hi"));
    let created_job = JobRepository::create(&job).await?;
    assert_eq!(created_job.job_type, "echo");

    let loaded_job = JobRepository::get(job.id).await?;
    assert_eq!(loaded_job.id, job.id);

    let running_status = JobStatus::Running {
        started_at: Utc::now(),
        worker_id: "worker-1".to_string(),
    };
    let updated_status = JobRepository::update_status(job.id, &running_status, 1).await?;
    assert!(matches!(updated_status.status, JobStatus::Running { .. }));
    assert_eq!(updated_status.attempts, 1);

    job = updated_status;
    job.tags = vec!["tag-a".to_string()];
    job.priority = Priority::High;
    let updated_job = JobRepository::update(&job).await?;
    assert_eq!(updated_job.tags.len(), 1);
    assert_eq!(updated_job.priority, Priority::High);

    JobRepository::delete(job.id).await?;
    let missing_job = JobRepository::get(job.id).await;
    assert!(matches!(missing_job, Err(DbError::NotFound(_))));

    // JobRepository: list/pending filters
    reset_db().await?;
    let queue = Queue::new("pending");
    QueueRepository::create(&queue).await?;

    let mut pending_job = Job::new(queue.id, "pending", payload_with_message("p"));
    pending_job.priority = Priority::Low;
    JobRepository::create(&pending_job).await?;

    let mut running_job = Job::new(queue.id, "running", payload_with_message("r"));
    running_job.status = JobStatus::Running {
        started_at: Utc::now(),
        worker_id: "worker-1".to_string(),
    };
    running_job.attempts = 1;
    JobRepository::create(&running_job).await?;

    let mut failed_job = Job::new(queue.id, "failed", payload_with_message("f"));
    failed_job.status = JobStatus::Failed {
        started_at: Utc::now(),
        failed_at: Utc::now(),
        error: "fail".to_string(),
        attempts: 1,
    };
    failed_job.attempts = 1;
    JobRepository::create(&failed_job).await?;

    let filter = db::repositories::JobFilter {
        queue_id: Some(queue.id),
        status: Some("pending".to_string()),
        ..Default::default()
    };
    let pending = JobRepository::list(filter).await?;
    assert!(pending.iter().all(|j| j.status.as_str() == "pending"));

    let pending_for_queue = JobRepository::get_pending_for_queue(queue.id, 10).await?;
    assert!(
        pending_for_queue
            .iter()
            .all(|j| j.status.as_str() == "pending")
    );

    // JobRepository: archive and stats
    reset_db().await?;
    let queue = Queue::new("archive");
    QueueRepository::create(&queue).await?;

    let mut completed_job = Job::new(queue.id, "completed", payload_with_message("c"));
    completed_job.status = JobStatus::Completed {
        started_at: Utc::now(),
        completed_at: Utc::now(),
        result: JobResult::new("done"),
    };
    completed_job.attempts = 1;
    JobRepository::create(&completed_job).await?;

    let mut failed_job = Job::new(queue.id, "failed", payload_with_message("f2"));
    failed_job.status = JobStatus::Failed {
        started_at: Utc::now(),
        failed_at: Utc::now(),
        error: "boom".to_string(),
        attempts: 2,
    };
    failed_job.attempts = 2;
    JobRepository::create(&failed_job).await?;

    let counts = JobRepository::count_by_status(queue.id).await?;
    assert_eq!(counts.get("completed").copied().unwrap_or(0), 1);
    assert_eq!(counts.get("failed").copied().unwrap_or(0), 1);

    let stats = JobRepository::get_queue_stats(queue.id).await?;
    assert_eq!(stats.completed, 1);
    assert_eq!(stats.failed, 1);

    JobRepository::archive(&completed_job).await?;
    let missing = JobRepository::get(completed_job.id).await;
    assert!(matches!(missing, Err(DbError::NotFound(_))));

    let db_conn = db::get_db()?;
    let mut response = db_conn
        .query("SELECT job_id FROM job_history WHERE job_id = $job_id")
        .bind(("job_id", completed_job.id.to_string()))
        .await?;
    let records: Vec<Value> = response.take(0)?;
    assert!(!records.is_empty());

    Ok(())
}
