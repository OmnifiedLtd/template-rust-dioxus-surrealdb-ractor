//! Queue domain types for job containers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueueId(pub Ulid);

impl QueueId {
    /// Create a new unique queue ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Parse a queue ID from a string.
    pub fn parse(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for QueueId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for QueueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Current operational state of a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueState {
    /// Queue is active and processing jobs.
    Running,
    /// Queue is paused, no new jobs will be processed.
    Paused,
    /// Queue is draining, finishing current jobs but not accepting new ones.
    Draining,
    /// Queue is stopped and not processing.
    Stopped,
}

impl Default for QueueState {
    fn default() -> Self {
        Self::Running
    }
}

impl std::fmt::Display for QueueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueState::Running => write!(f, "running"),
            QueueState::Paused => write!(f, "paused"),
            QueueState::Draining => write!(f, "draining"),
            QueueState::Stopped => write!(f, "stopped"),
        }
    }
}

/// Configuration for queue behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QueueConfig {
    /// Number of concurrent workers for this queue.
    pub concurrency: u32,
    /// Default timeout for jobs in this queue (seconds).
    pub default_timeout_secs: u64,
    /// Default max retries for jobs in this queue.
    pub default_max_retries: u32,
    /// Maximum number of jobs that can be queued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_queue_size: Option<usize>,
    /// Rate limit: max jobs per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<f64>,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            default_timeout_secs: 300,
            default_max_retries: 3,
            max_queue_size: None,
            rate_limit: None,
        }
    }
}

/// Statistics for a queue's current state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QueueStats {
    /// Number of pending jobs.
    pub pending: u64,
    /// Number of running jobs.
    pub running: u64,
    /// Number of completed jobs (since last reset).
    pub completed: u64,
    /// Number of failed jobs (since last reset).
    pub failed: u64,
    /// Average job duration in milliseconds.
    pub avg_duration_ms: Option<f64>,
    /// Jobs processed per minute.
    pub throughput_per_min: Option<f64>,
}

impl QueueStats {
    /// Total jobs in queue (pending + running).
    pub fn active(&self) -> u64 {
        self.pending + self.running
    }

    /// Total processed jobs.
    pub fn processed(&self) -> u64 {
        self.completed + self.failed
    }

    /// Success rate as a percentage.
    pub fn success_rate(&self) -> Option<f64> {
        let total = self.processed();
        if total == 0 {
            None
        } else {
            Some((self.completed as f64 / total as f64) * 100.0)
        }
    }
}

/// A queue manages a set of jobs and their execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Queue {
    /// Unique identifier for this queue.
    pub id: QueueId,
    /// Human-readable name for the queue.
    pub name: String,
    /// Optional description of what this queue is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current operational state.
    pub state: QueueState,
    /// Queue configuration.
    pub config: QueueConfig,
    /// Current statistics.
    pub stats: QueueStats,
    /// When the queue was created.
    pub created_at: DateTime<Utc>,
    /// When the queue was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Queue {
    /// Create a new queue with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: QueueId::new(),
            name: name.into(),
            description: None,
            state: QueueState::Running,
            config: QueueConfig::default(),
            stats: QueueStats::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description for this queue.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the configuration for this queue.
    pub fn with_config(mut self, config: QueueConfig) -> Self {
        self.config = config;
        self
    }

    /// Check if the queue is accepting new jobs.
    pub fn is_accepting_jobs(&self) -> bool {
        matches!(self.state, QueueState::Running)
    }

    /// Check if the queue is processing jobs.
    pub fn is_processing(&self) -> bool {
        matches!(self.state, QueueState::Running | QueueState::Draining)
    }
}
