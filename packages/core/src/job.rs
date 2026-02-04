//! Job domain types for work items in the queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a job, using ULID for chronological sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub Ulid);

impl JobId {
    /// Create a new unique job ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Parse a job ID from a string.
    pub fn parse(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Priority level for job execution order.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

/// Current status of a job in its lifecycle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is waiting to be processed.
    #[default]
    Pending,
    /// Job is currently being executed by a worker.
    Running {
        started_at: DateTime<Utc>,
        worker_id: String,
    },
    /// Job completed successfully.
    Completed {
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        result: JobResult,
    },
    /// Job failed with an error.
    Failed {
        started_at: DateTime<Utc>,
        failed_at: DateTime<Utc>,
        error: String,
        attempts: u32,
    },
    /// Job was cancelled before completion.
    Cancelled {
        cancelled_at: DateTime<Utc>,
        reason: Option<String>,
    },
    /// Job is paused and won't be picked up.
    Paused,
}

impl JobStatus {
    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Completed { .. } | JobStatus::Failed { .. } | JobStatus::Cancelled { .. }
        )
    }

    /// Check if the job can be retried.
    pub fn can_retry(&self) -> bool {
        matches!(self, JobStatus::Failed { .. } | JobStatus::Cancelled { .. })
    }

    /// Get a simple status string for display.
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running { .. } => "running",
            JobStatus::Completed { .. } => "completed",
            JobStatus::Failed { .. } => "failed",
            JobStatus::Cancelled { .. } => "cancelled",
            JobStatus::Paused => "paused",
        }
    }
}

/// Result of a completed job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobResult {
    /// Human-readable summary of the result.
    pub summary: String,
    /// Optional structured output data as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

impl JobResult {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            output: None,
        }
    }

    pub fn with_output(summary: impl Into<String>, output: serde_json::Value) -> Self {
        Self {
            summary: summary.into(),
            output: Some(output),
        }
    }
}

/// A job represents a unit of work to be executed by the queue system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier for this job.
    pub id: JobId,
    /// The queue this job belongs to.
    pub queue_id: super::QueueId,
    /// Type of job (used for routing to handlers).
    pub job_type: String,
    /// Job payload as JSON.
    pub payload: serde_json::Value,
    /// Execution priority.
    pub priority: Priority,
    /// Current status.
    pub status: JobStatus,
    /// Number of attempts so far.
    #[serde(default)]
    pub attempts: u32,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Timeout in seconds for job execution.
    pub timeout_secs: u64,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// When the job was last updated.
    pub updated_at: DateTime<Utc>,
    /// Optional tags for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Job {
    /// Create a new pending job.
    pub fn new(
        queue_id: super::QueueId,
        job_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: JobId::new(),
            queue_id,
            job_type: job_type.into(),
            payload,
            priority: Priority::default(),
            status: JobStatus::Pending,
            attempts: 0,
            max_retries: 3,
            timeout_secs: 300, // 5 minutes default
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }

    /// Set the priority for this job.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the max retries for this job.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the timeout for this job.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Add tags to this job.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}
