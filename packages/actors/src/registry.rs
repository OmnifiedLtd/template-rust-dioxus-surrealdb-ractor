//! Actor registry for discovering actors by name.

use ractor::ActorRef;
use std::collections::HashMap;
use std::sync::RwLock;

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
        match self.supervisor.write() {
            Ok(mut guard) => {
                *guard = Some(supervisor);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(supervisor);
            }
        }
    }

    /// Get the supervisor.
    pub fn get_supervisor(&self) -> Option<ActorRef<SupervisorMessage>> {
        match self.supervisor.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Register a queue actor.
    pub fn register_queue(&self, name: &str, queue: ActorRef<QueueMessage>) {
        match self.queues.write() {
            Ok(mut guard) => {
                guard.insert(name.to_string(), queue);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.insert(name.to_string(), queue);
            }
        }
    }

    /// Unregister a queue actor.
    pub fn unregister_queue(&self, name: &str) {
        match self.queues.write() {
            Ok(mut guard) => {
                guard.remove(name);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.remove(name);
            }
        }
    }

    /// Get a queue actor by name.
    pub fn get_queue(&self, name: &str) -> Option<ActorRef<QueueMessage>> {
        match self.queues.read() {
            Ok(guard) => guard.get(name).cloned(),
            Err(poisoned) => poisoned.into_inner().get(name).cloned(),
        }
    }

    /// List all registered queue names.
    pub fn list_queues(&self) -> Vec<String> {
        match self.queues.read() {
            Ok(guard) => guard.keys().cloned().collect(),
            Err(poisoned) => poisoned.into_inner().keys().cloned().collect(),
        }
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
