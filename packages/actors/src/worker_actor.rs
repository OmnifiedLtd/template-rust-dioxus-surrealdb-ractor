//! Worker actor for executing jobs.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use queue_core::{Job, JobEvent, QueueId};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::broadcast;

use crate::handler::JobHandlerRegistry;
use crate::messages::{QueueMessage, WorkerMessage};

/// State for the worker actor.
pub struct WorkerActorState {
    /// Unique worker ID.
    pub worker_id: String,
    /// Queue ID this worker is attached to.
    pub queue_id: QueueId,
    /// Current job being processed.
    pub current_job: Option<Job>,
    /// Queue actor reference.
    pub queue: ActorRef<QueueMessage>,
    /// Handler registry.
    pub handlers: Arc<JobHandlerRegistry>,
    /// Event broadcaster.
    pub event_tx: Option<broadcast::Sender<JobEvent>>,
    /// Whether the worker should continue running.
    pub running: bool,
}

impl WorkerActorState {
    /// Create a new worker actor state.
    pub fn new(
        worker_id: impl Into<String>,
        queue_id: QueueId,
        queue: ActorRef<QueueMessage>,
        handlers: Arc<JobHandlerRegistry>,
    ) -> Self {
        Self {
            worker_id: worker_id.into(),
            queue_id,
            current_job: None,
            queue,
            handlers,
            event_tx: None,
            running: true,
        }
    }

    /// Set the event broadcaster.
    pub fn with_event_tx(mut self, tx: broadcast::Sender<JobEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Check if the worker is idle.
    pub fn is_idle(&self) -> bool {
        self.current_job.is_none()
    }
}

/// Worker actor arguments.
pub struct WorkerArgs {
    pub worker_id: String,
    pub queue_id: QueueId,
    pub queue: ActorRef<QueueMessage>,
    pub handlers: Arc<JobHandlerRegistry>,
    pub event_tx: Option<broadcast::Sender<JobEvent>>,
}

/// Worker actor that executes jobs.
pub struct WorkerActor;

impl Actor for WorkerActor {
    type Msg = WorkerMessage;
    type State = WorkerActorState;
    type Arguments = WorkerArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Starting worker: {}", args.worker_id);

        let mut state =
            WorkerActorState::new(args.worker_id, args.queue_id, args.queue, args.handlers);
        if let Some(tx) = args.event_tx {
            state = state.with_event_tx(tx);
        }

        // Start the work loop
        let myself_clone = myself.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if myself_clone.send_message(WorkerMessage::Heartbeat).is_err() {
                    break;
                }
            }
        });

        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkerMessage::ProcessJob { job } => {
                let job = *job;
                state.current_job = Some(job.clone());

                // Find handler for this job type
                if let Some(handler) = state.handlers.get(&job.job_type) {
                    let job_id = job.id;
                    let timeout = Duration::from_secs(job.timeout_secs);

                    // Execute with timeout
                    let result = tokio::time::timeout(timeout, handler.handle(&job)).await;

                    match result {
                        Ok(Ok(job_result)) => {
                            // Job succeeded
                            state.queue.send_message(QueueMessage::JobCompleted {
                                job_id,
                                worker_id: state.worker_id.clone(),
                                result: job_result,
                            })?;
                        }
                        Ok(Err(error)) => {
                            // Job failed with error
                            state.queue.send_message(QueueMessage::JobFailed {
                                job_id,
                                worker_id: state.worker_id.clone(),
                                error,
                            })?;
                        }
                        Err(_) => {
                            // Job timed out
                            state.queue.send_message(QueueMessage::JobFailed {
                                job_id,
                                worker_id: state.worker_id.clone(),
                                error: "Job timed out".into(),
                            })?;
                        }
                    }
                } else {
                    // No handler for this job type
                    state.queue.send_message(QueueMessage::JobFailed {
                        job_id: job.id,
                        worker_id: state.worker_id.clone(),
                        error: format!("No handler for job type: {}", job.job_type),
                    })?;
                }

                state.current_job = None;
            }

            WorkerMessage::StopJob { reason } => {
                if let Some(job) = state.current_job.take() {
                    state.queue.send_message(QueueMessage::JobFailed {
                        job_id: job.id,
                        worker_id: state.worker_id.clone(),
                        error: format!("Stopped: {}", reason),
                    })?;
                }
            }

            WorkerMessage::IsIdle { reply } => {
                let _ = reply.send(state.is_idle());
            }

            WorkerMessage::Shutdown => {
                tracing::info!("Shutting down worker: {}", state.worker_id);
                state.running = false;
                myself.stop(None);
                return Ok(());
            }

            WorkerMessage::Heartbeat => {
                if !state.running {
                    myself.stop(None);
                    return Ok(());
                }

                // If idle, request a job
                if state.is_idle() {
                    let timeout = std::time::Duration::from_secs(5);
                    let result = ractor::rpc::call(
                        &state.queue,
                        |reply| QueueMessage::RequestJob {
                            worker_id: state.worker_id.clone(),
                            reply,
                        },
                        Some(timeout),
                    )
                    .await;
                    // ractor::rpc::call returns Result<CallResult<T>, MessagingErr<M>>
                    // CallResult can be Success(T), Timeout, or SenderError
                    if let Ok(ractor::rpc::CallResult::Success(Some(job))) = result {
                        myself.send_message(WorkerMessage::ProcessJob { job: Box::new(job) })?;
                    }
                }

                // Broadcast heartbeat event
                if let Some(ref tx) = state.event_tx {
                    let _ = tx.send(JobEvent::WorkerHeartbeat {
                        worker_id: state.worker_id.clone(),
                        queue_id: state
                            .current_job
                            .as_ref()
                            .map_or(state.queue_id, |j| j.queue_id),
                        current_job: state.current_job.as_ref().map(|j| j.id),
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        Ok(())
    }
}
