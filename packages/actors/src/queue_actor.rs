//! Queue actor for managing jobs in a single queue.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use chrono::Utc;
use queue_core::{Job, JobEvent, JobId, JobStatus, Queue, QueueState, QueueStats};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::broadcast;

use crate::messages::{QueueMessage, SupervisorMessage};

/// Wrapper for priority queue ordering (higher priority first, older jobs first).
#[derive(Debug, Clone)]
struct PriorityJob {
    job: Job,
}

impl PartialEq for PriorityJob {
    fn eq(&self, other: &Self) -> bool {
        self.job.id == other.job.id
    }
}

impl Eq for PriorityJob {}

impl PartialOrd for PriorityJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        match self.job.priority.cmp(&other.job.priority) {
            Ordering::Equal => {
                // Older jobs first (earlier created_at)
                other.job.created_at.cmp(&self.job.created_at)
            }
            other => other,
        }
    }
}

/// State for the queue actor.
pub struct QueueActorState {
    /// Queue metadata.
    pub queue: Queue,
    /// Pending jobs (priority queue).
    pending: BinaryHeap<PriorityJob>,
    /// Running jobs by ID.
    running: HashMap<JobId, Job>,
    /// All jobs by ID for quick lookup.
    jobs: HashMap<JobId, Job>,
    /// Event broadcaster.
    event_tx: Option<broadcast::Sender<JobEvent>>,
    /// Supervisor reference for event forwarding.
    supervisor: Option<ActorRef<SupervisorMessage>>,
}

impl QueueActorState {
    /// Create a new queue actor state.
    pub fn new(queue: Queue) -> Self {
        Self {
            queue,
            pending: BinaryHeap::new(),
            running: HashMap::new(),
            jobs: HashMap::new(),
            event_tx: None,
            supervisor: None,
        }
    }

    /// Set the supervisor reference.
    pub fn with_supervisor(mut self, supervisor: ActorRef<SupervisorMessage>) -> Self {
        self.supervisor = Some(supervisor);
        self
    }

    /// Set the event broadcaster.
    pub fn with_event_tx(mut self, tx: broadcast::Sender<JobEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Broadcast an event.
    fn broadcast(&self, event: JobEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event.clone());
        }
        if let Some(ref supervisor) = self.supervisor {
            let _ = supervisor.send_message(SupervisorMessage::BroadcastEvent { event });
        }
    }

    /// Update and broadcast stats.
    fn update_stats(&mut self) {
        self.queue.stats = QueueStats {
            pending: self.pending.len() as u64,
            running: self.running.len() as u64,
            completed: self.queue.stats.completed,
            failed: self.queue.stats.failed,
            avg_duration_ms: self.queue.stats.avg_duration_ms,
            throughput_per_min: self.queue.stats.throughput_per_min,
        };

        self.broadcast(JobEvent::QueueStatsUpdated {
            queue_id: self.queue.id,
            stats: self.queue.stats.clone(),
            timestamp: Utc::now(),
        });
    }
}

/// Queue actor that manages a single queue.
pub struct QueueActor;

impl Actor for QueueActor {
    type Msg = QueueMessage;
    type State = QueueActorState;
    type Arguments = QueueActorState;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Starting queue actor: {}", args.queue.name);
        Ok(args)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            QueueMessage::Enqueue { job, reply } => {
                let job = *job;
                if !state.queue.is_accepting_jobs() {
                    let _ = reply.send(Err("Queue is not accepting jobs".into()));
                    return Ok(());
                }

                // Check queue size limit
                if let Some(max_size) = state.queue.config.max_queue_size
                    && state.pending.len() >= max_size
                {
                    let _ = reply.send(Err("Queue is full".into()));
                    return Ok(());
                }

                if let Err(e) = db::repositories::JobRepository::create(&job).await {
                    let _ = reply.send(Err(format!("Failed to persist job: {}", e)));
                    return Ok(());
                }

                let job_id = job.id;
                state.jobs.insert(job_id, job.clone());
                state.pending.push(PriorityJob { job: job.clone() });

                state.broadcast(JobEvent::JobEnqueued {
                    job: job.clone(),
                    timestamp: Utc::now(),
                });
                state.update_stats();

                let _ = reply.send(Ok(job));
            }

            QueueMessage::RequestJob { worker_id, reply } => {
                if !state.queue.is_processing() {
                    let _ = reply.send(None);
                    return Ok(());
                }

                // Check concurrency limit
                if state.running.len() >= state.queue.config.concurrency as usize {
                    let _ = reply.send(None);
                    return Ok(());
                }

                if let Some(priority_job) = state.pending.pop() {
                    let mut job = priority_job.job;
                    let now = Utc::now();
                    let previous_attempts = job.attempts;

                    job.attempts = job.attempts.saturating_add(1);
                    job.status = JobStatus::Running {
                        started_at: now,
                        worker_id: worker_id.clone(),
                    };
                    job.updated_at = now;

                    if let Err(e) = db::repositories::JobRepository::update_status(
                        job.id,
                        &job.status,
                        job.attempts,
                    )
                    .await
                    {
                        tracing::warn!("Failed to mark job {} running: {}", job.id, e);
                        job.attempts = previous_attempts;
                        job.status = JobStatus::Pending;
                        job.updated_at = now;
                        state.pending.push(PriorityJob { job });
                        state.update_stats();
                        let _ = reply.send(None);
                        return Ok(());
                    }

                    state.jobs.insert(job.id, job.clone());
                    state.running.insert(job.id, job.clone());

                    state.broadcast(JobEvent::JobStarted {
                        job_id: job.id,
                        queue_id: state.queue.id,
                        worker_id,
                        timestamp: now,
                    });
                    state.update_stats();

                    let _ = reply.send(Some(job));
                } else {
                    let _ = reply.send(None);
                }
            }

            QueueMessage::JobCompleted {
                job_id,
                worker_id: _,
                result,
            } => {
                if let Some(mut job) = state.running.remove(&job_id) {
                    let now = Utc::now();
                    let started_at = match &job.status {
                        JobStatus::Running { started_at, .. } => *started_at,
                        _ => now,
                    };
                    let duration_ms = (now - started_at).num_milliseconds() as u64;

                    job.status = JobStatus::Completed {
                        started_at,
                        completed_at: now,
                        result,
                    };
                    job.updated_at = now;

                    if let Err(e) = db::repositories::JobRepository::update_status(
                        job_id,
                        &job.status,
                        job.attempts,
                    )
                    .await
                    {
                        tracing::warn!("Failed to update job {} status: {}", job_id, e);
                    }

                    state.jobs.insert(job_id, job.clone());
                    state.queue.stats.completed += 1;

                    state.broadcast(JobEvent::JobCompleted {
                        job_id,
                        queue_id: state.queue.id,
                        duration_ms,
                        timestamp: now,
                    });
                    state.update_stats();

                    // Archive to database
                    if let Err(e) = db::repositories::JobRepository::archive(&job).await {
                        tracing::warn!("Failed to archive job {}: {}", job_id, e);
                    }
                }
            }

            QueueMessage::JobFailed {
                job_id,
                worker_id: _,
                error,
            } => {
                if let Some(mut job) = state.running.remove(&job_id) {
                    let now = Utc::now();
                    let started_at = match &job.status {
                        JobStatus::Running { started_at, .. } => *started_at,
                        _ => now,
                    };

                    let attempts = job.attempts;
                    let will_retry = attempts < job.max_retries;

                    job.status = JobStatus::Failed {
                        started_at,
                        failed_at: now,
                        error: error.clone(),
                        attempts,
                    };
                    job.updated_at = now;

                    state.broadcast(JobEvent::JobFailed {
                        job_id,
                        queue_id: state.queue.id,
                        error,
                        attempts,
                        will_retry,
                        timestamp: now,
                    });

                    if will_retry {
                        job.status = JobStatus::Pending;
                        job.updated_at = now;

                        if let Err(e) = db::repositories::JobRepository::update_status(
                            job_id,
                            &job.status,
                            job.attempts,
                        )
                        .await
                        {
                            tracing::warn!("Failed to mark job {} pending: {}", job_id, e);
                        }

                        // Re-enqueue for retry
                        state.pending.push(PriorityJob { job: job.clone() });

                        state.broadcast(JobEvent::JobRetrying {
                            job_id,
                            queue_id: state.queue.id,
                            attempt: attempts + 1,
                            timestamp: now,
                        });
                    } else {
                        if let Err(e) = db::repositories::JobRepository::update_status(
                            job_id,
                            &job.status,
                            job.attempts,
                        )
                        .await
                        {
                            tracing::warn!("Failed to update job {} status: {}", job_id, e);
                        }

                        state.queue.stats.failed += 1;

                        // Archive failed job
                        if let Err(e) = db::repositories::JobRepository::archive(&job).await {
                            tracing::warn!("Failed to archive job {}: {}", job_id, e);
                        }
                    }

                    state.jobs.insert(job_id, job);
                    state.update_stats();
                }
            }

            QueueMessage::CancelJob {
                job_id,
                reason,
                reply,
            } => {
                if let Some(mut job) = state.jobs.get(&job_id).cloned() {
                    let now = Utc::now();

                    // Remove from pending or running
                    state.running.remove(&job_id);
                    state.pending.retain(|pj| pj.job.id != job_id);

                    job.status = JobStatus::Cancelled {
                        cancelled_at: now,
                        reason: reason.clone(),
                    };
                    job.updated_at = now;

                    if let Err(e) = db::repositories::JobRepository::update_status(
                        job_id,
                        &job.status,
                        job.attempts,
                    )
                    .await
                    {
                        tracing::warn!("Failed to update job {} status: {}", job_id, e);
                    }

                    state.jobs.insert(job_id, job.clone());

                    state.broadcast(JobEvent::JobCancelled {
                        job_id,
                        queue_id: state.queue.id,
                        reason,
                        timestamp: now,
                    });
                    state.update_stats();

                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("Job not found".into()));
                }
            }

            QueueMessage::RetryJob { job_id, reply } => {
                if let Some(mut job) = state.jobs.get(&job_id).cloned() {
                    if !job.status.can_retry() {
                        let _ = reply.send(Err("Job cannot be retried".into()));
                        return Ok(());
                    }

                    let now = Utc::now();
                    job.status = JobStatus::Pending;
                    job.updated_at = now;

                    if let Err(e) = db::repositories::JobRepository::update_status(
                        job_id,
                        &job.status,
                        job.attempts,
                    )
                    .await
                    {
                        let _ = reply.send(Err(format!("Failed to update job: {}", e)));
                        return Ok(());
                    }

                    state.jobs.insert(job_id, job.clone());
                    state.pending.push(PriorityJob { job: job.clone() });
                    state.update_stats();

                    let _ = reply.send(Ok(job));
                } else {
                    let _ = reply.send(Err("Job not found".into()));
                }
            }

            QueueMessage::GetJob { job_id, reply } => {
                let _ = reply.send(state.jobs.get(&job_id).cloned());
            }

            QueueMessage::ListJobs {
                status_filter,
                limit,
                reply,
            } => {
                let jobs: Vec<Job> = state
                    .jobs
                    .values()
                    .filter(|j| {
                        status_filter
                            .as_ref()
                            .is_none_or(|s| j.status.as_str() == s)
                    })
                    .take(limit)
                    .cloned()
                    .collect();
                let _ = reply.send(jobs);
            }

            QueueMessage::Pause => {
                let old_state = state.queue.state;
                state.queue.state = QueueState::Paused;
                state.queue.updated_at = Utc::now();

                if let Err(e) = db::repositories::QueueRepository::update_state(
                    state.queue.id,
                    state.queue.state,
                )
                .await
                {
                    tracing::warn!("Failed to persist queue state: {}", e);
                }

                state.broadcast(JobEvent::QueueStateChanged {
                    queue_id: state.queue.id,
                    old_state,
                    new_state: QueueState::Paused,
                    timestamp: Utc::now(),
                });
            }

            QueueMessage::Resume => {
                let old_state = state.queue.state;
                state.queue.state = QueueState::Running;
                state.queue.updated_at = Utc::now();

                if let Err(e) = db::repositories::QueueRepository::update_state(
                    state.queue.id,
                    state.queue.state,
                )
                .await
                {
                    tracing::warn!("Failed to persist queue state: {}", e);
                }

                state.broadcast(JobEvent::QueueStateChanged {
                    queue_id: state.queue.id,
                    old_state,
                    new_state: QueueState::Running,
                    timestamp: Utc::now(),
                });
            }

            QueueMessage::GetInfo { reply } => {
                let _ = reply.send(state.queue.clone());
            }

            QueueMessage::GetStats { reply } => {
                let _ = reply.send(state.queue.stats.clone());
            }

            QueueMessage::Shutdown => {
                tracing::info!("Shutting down queue: {}", state.queue.name);
                // Could persist state here
                myself.stop(None);
                return Ok(());
            }

            QueueMessage::Tick => {
                // Periodic housekeeping
                // TODO: Check for timed-out jobs, persist state, etc.
            }
        }

        Ok(())
    }
}
