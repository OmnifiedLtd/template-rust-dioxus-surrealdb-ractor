//! Actor registry for discovering actors by name.

use std::collections::HashMap;
use std::sync::RwLock;
use ractor::ActorRef;

use crate::messages::{QueueMessage, SupervisorMessage};

/// Global actor registry for discovering actors.
///
/// This provides a way to look up actors by name without passing
/// references through the entire call stack.
pub struct ActorRegistry {
    supervisor: RwLock<Option<ActorRef<SupervisorMessage>>>,
    queues: RwLock<HashMap<String, ActorRef<QueueMessage>>>,
}

impl ActorRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            supervisor: RwLock::new(None),
            queues: RwLock::new(HashMap::new()),
        }
    }

    /// Register the supervisor.
    pub fn register_supervisor(&self, supervisor: ActorRef<SupervisorMessage>) {
        *self.supervisor.write().unwrap() = Some(supervisor);
    }

    /// Get the supervisor.
    pub fn get_supervisor(&self) -> Option<ActorRef<SupervisorMessage>> {
        self.supervisor.read().unwrap().clone()
    }

    /// Register a queue actor.
    pub fn register_queue(&self, name: &str, queue: ActorRef<QueueMessage>) {
        self.queues.write().unwrap().insert(name.to_string(), queue);
    }

    /// Unregister a queue actor.
    pub fn unregister_queue(&self, name: &str) {
        self.queues.write().unwrap().remove(name);
    }

    /// Get a queue actor by name.
    pub fn get_queue(&self, name: &str) -> Option<ActorRef<QueueMessage>> {
        self.queues.read().unwrap().get(name).cloned()
    }

    /// List all registered queue names.
    pub fn list_queues(&self) -> Vec<String> {
        self.queues.read().unwrap().keys().cloned().collect()
    }
}

impl Default for ActorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry instance.
static REGISTRY: std::sync::LazyLock<ActorRegistry> = std::sync::LazyLock::new(ActorRegistry::new);

/// Get the global actor registry.
pub fn global_registry() -> &'static ActorRegistry {
    &REGISTRY
}
