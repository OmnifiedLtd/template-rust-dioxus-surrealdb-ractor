//! Supervisor actor for managing all queues and workers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use queue_core::{JobEvent, Queue, QueueId};
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use tokio::sync::broadcast;

use crate::handler::JobHandlerRegistry;
use crate::messages::{QueueMessage, SupervisorMessage};
use crate::queue_actor::{QueueActor, QueueActorState};
use crate::worker_actor::{WorkerActor, WorkerArgs};

/// State for the supervisor actor.
pub struct SupervisorState {
    /// All queue actors by ID.
    pub queues: HashMap<QueueId, ActorRef<QueueMessage>>,
    /// Queue metadata by ID.
    pub queue_info: HashMap<QueueId, Queue>,
    /// Event broadcaster.
    pub event_tx: broadcast::Sender<JobEvent>,
    /// Handler registry for workers.
    pub handlers: Arc<JobHandlerRegistry>,
    /// Worker counter for unique IDs.
    worker_counter: u64,
}

impl SupervisorState {
    /// Create a new supervisor state.
    pub fn new(handlers: JobHandlerRegistry) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            queues: HashMap::new(),
            queue_info: HashMap::new(),
            event_tx,
            handlers: Arc::new(handlers),
            worker_counter: 0,
        }
    }

    /// Generate a unique worker ID.
    fn next_worker_id(&mut self) -> String {
        self.worker_counter += 1;
        format!("worker-{}", self.worker_counter)
    }
}

async fn spawn_queue_actor(
    myself: ActorRef<SupervisorMessage>,
    state: &mut SupervisorState,
    queue: Queue,
) -> Result<ActorRef<QueueMessage>, ActorProcessingErr> {
    let queue_state = QueueActorState::new(queue.clone())
        .with_supervisor(myself.clone())
        .with_event_tx(state.event_tx.clone());

    let (actor, _handle) =
        Actor::spawn(Some(format!("queue-{}", queue.id)), QueueActor, queue_state)
            .await
            .map_err(|e| ActorProcessingErr::from(format!("Failed to spawn queue: {}", e)))?;

    for _ in 0..queue.config.concurrency {
        let worker_id = state.next_worker_id();
        let args = WorkerArgs {
            worker_id,
            queue_id: queue.id,
            queue: actor.clone(),
            handlers: state.handlers.clone(),
            event_tx: Some(state.event_tx.clone()),
        };

        Actor::spawn(None, WorkerActor, args).await.ok();
    }

    state.queues.insert(queue.id, actor.clone());
    state.queue_info.insert(queue.id, queue);

    Ok(actor)
}

/// Supervisor actor that manages all queues.
pub struct Supervisor;

impl Actor for Supervisor {
    type Msg = SupervisorMessage;
    type State = SupervisorState;
    type Arguments = JobHandlerRegistry;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Starting job queue supervisor");

        // Start periodic tick
        let myself_clone = myself.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if myself_clone.send_message(SupervisorMessage::Tick).is_err() {
                    break;
                }
            }
        });

        Ok(SupervisorState::new(args))
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisorMessage::CreateQueue {
                name,
                description,
                reply,
            } => {
                // Check if queue already exists
                if state.queue_info.values().any(|q| q.name == name) {
                    let _ = reply.send(Err(format!("Queue '{}' already exists", name)));
                    return Ok(());
                }

                let mut queue = Queue::new(&name);
                if let Some(desc) = description {
                    queue = queue.with_description(desc);
                }

                // Save to database
                match db::repositories::QueueRepository::create(&queue).await {
                    Ok(_) => {}
                    Err(e) => {
                        let _ = reply.send(Err(format!("Failed to create queue: {}", e)));
                        return Ok(());
                    }
                }

                if let Err(e) = spawn_queue_actor(myself.clone(), state, queue.clone()).await {
                    let _ = reply.send(Err(format!("Failed to spawn queue: {}", e)));
                    return Ok(());
                }

                // Broadcast event
                let _ = state.event_tx.send(JobEvent::QueueCreated {
                    queue: queue.clone(),
                    timestamp: Utc::now(),
                });

                let _ = reply.send(Ok(queue));
            }

            SupervisorMessage::RegisterQueue { queue, reply } => {
                if state.queues.contains_key(&queue.id)
                    || state.queue_info.values().any(|q| q.name == queue.name)
                {
                    let _ = reply.send(Err("Queue already registered".into()));
                    return Ok(());
                }

                if let Err(e) = spawn_queue_actor(myself.clone(), state, queue.clone()).await {
                    let _ = reply.send(Err(format!("Failed to spawn queue: {}", e)));
                    return Ok(());
                }

                let _ = reply.send(Ok(queue));
            }

            SupervisorMessage::GetQueue { queue_id, reply } => {
                if let Some(queue_ref) = state.queues.get(&queue_id) {
                    let (tx, rx) = ractor::concurrency::oneshot();
                    if queue_ref
                        .send_message(QueueMessage::GetInfo { reply: tx.into() })
                        .is_ok()
                        && let Ok(queue) = rx.await
                    {
                        let _ = reply.send(Some(queue));
                        return Ok(());
                    }
                }
                let _ = reply.send(None);
            }

            SupervisorMessage::GetQueueByName { name, reply } => {
                for (id, info) in &state.queue_info {
                    if info.name == name
                        && let Some(queue_ref) = state.queues.get(id)
                    {
                        let (tx, rx) = ractor::concurrency::oneshot();
                        if queue_ref
                            .send_message(QueueMessage::GetInfo { reply: tx.into() })
                            .is_ok()
                            && let Ok(queue) = rx.await
                        {
                            let _ = reply.send(Some(queue));
                            return Ok(());
                        }
                    }
                }
                let _ = reply.send(None);
            }

            SupervisorMessage::ListQueues { reply } => {
                let mut queues = Vec::new();
                for queue_ref in state.queues.values() {
                    let (tx, rx) = ractor::concurrency::oneshot();
                    if queue_ref
                        .send_message(QueueMessage::GetInfo { reply: tx.into() })
                        .is_ok()
                        && let Ok(queue) = rx.await
                    {
                        queues.push(queue);
                    }
                }
                let _ = reply.send(queues);
            }

            SupervisorMessage::PauseQueue { queue_id, reply } => {
                if let Some(queue_ref) = state.queues.get(&queue_id) {
                    queue_ref.send_message(QueueMessage::Pause)?;
                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("Queue not found".into()));
                }
            }

            SupervisorMessage::ResumeQueue { queue_id, reply } => {
                if let Some(queue_ref) = state.queues.get(&queue_id) {
                    queue_ref.send_message(QueueMessage::Resume)?;
                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("Queue not found".into()));
                }
            }

            SupervisorMessage::DeleteQueue { queue_id, reply } => {
                if let Some(queue_ref) = state.queues.remove(&queue_id) {
                    queue_ref.send_message(QueueMessage::Shutdown)?;
                    state.queue_info.remove(&queue_id);

                    // Delete from database
                    if let Err(e) = db::repositories::QueueRepository::delete(queue_id).await {
                        tracing::warn!("Failed to delete queue from DB: {}", e);
                    }

                    let _ = state.event_tx.send(JobEvent::QueueDeleted {
                        queue_id,
                        timestamp: Utc::now(),
                    });

                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err("Queue not found".into()));
                }
            }

            SupervisorMessage::EnqueueJob {
                queue_id,
                job,
                reply,
            } => {
                if let Some(queue_ref) = state.queues.get(&queue_id) {
                    let (tx, rx) = ractor::concurrency::oneshot();
                    queue_ref.send_message(QueueMessage::Enqueue {
                        job: Box::new(job),
                        reply: tx.into(),
                    })?;
                    match rx.await {
                        Ok(result) => {
                            let _ = reply.send(result);
                        }
                        Err(_) => {
                            let _ = reply.send(Err("Failed to enqueue job".into()));
                        }
                    }
                } else {
                    let _ = reply.send(Err("Queue not found".into()));
                }
            }

            SupervisorMessage::GetJob { job_id, reply } => {
                for queue_ref in state.queues.values() {
                    let (tx, rx) = ractor::concurrency::oneshot();
                    if queue_ref
                        .send_message(QueueMessage::GetJob {
                            job_id,
                            reply: tx.into(),
                        })
                        .is_ok()
                        && let Ok(Some(job)) = rx.await
                    {
                        let _ = reply.send(Some(job));
                        return Ok(());
                    }
                }
                let _ = reply.send(None);
            }

            SupervisorMessage::CancelJob {
                job_id,
                reason,
                reply,
            } => {
                for queue_ref in state.queues.values() {
                    let (tx, rx) = ractor::concurrency::oneshot();
                    if queue_ref
                        .send_message(QueueMessage::CancelJob {
                            job_id,
                            reason: reason.clone(),
                            reply: tx.into(),
                        })
                        .is_ok()
                        && let Ok(Ok(())) = rx.await
                    {
                        let _ = reply.send(Ok(()));
                        return Ok(());
                    }
                }
                let _ = reply.send(Err("Job not found".into()));
            }

            SupervisorMessage::Subscribe { sender } => {
                // Merge event streams - forward from our channel to subscriber's
                let mut rx = state.event_tx.subscribe();
                tokio::spawn(async move {
                    while let Ok(event) = rx.recv().await {
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                });
            }

            SupervisorMessage::BroadcastEvent { event } => {
                let _ = state.event_tx.send(event);
            }

            SupervisorMessage::Shutdown => {
                tracing::info!("Shutting down supervisor");
                for queue_ref in state.queues.values() {
                    let _ = queue_ref.send_message(QueueMessage::Shutdown);
                }
                myself.stop(None);
                return Ok(());
            }

            SupervisorMessage::Tick => {
                // Periodic housekeeping
                // TODO: Persist state, check for stale workers, etc.
            }
        }

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        if let SupervisionEvent::ActorTerminated(cell, _, reason) = message {
            tracing::warn!(
                "Child actor {} terminated: {:?}",
                cell.get_name().unwrap_or_default(),
                reason
            );
            // TODO: Restart logic if needed
        }
        Ok(())
    }
}

/// Start the supervisor with the given handler registry.
pub async fn start_supervisor(
    handlers: JobHandlerRegistry,
) -> Result<(ActorRef<SupervisorMessage>, tokio::task::JoinHandle<()>), ractor::SpawnErr> {
    let (actor, handle) =
        Actor::spawn(Some("supervisor".to_string()), Supervisor, handlers).await?;

    Ok((actor, handle))
}
