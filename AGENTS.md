# Agents Architecture Guide

This document provides an overview of the agentic backend architecture for building autonomous, resumable job processing systems.

## Overview

This template provides a production-ready foundation for agentic backends:

- **Pausable/Resumable Jobs** - Jobs persist state and can be paused, resumed, or cancelled
- **Priority Queues** - Jobs are processed by priority (Critical > High > Normal > Low)
- **Automatic Retries** - Failed jobs retry with exponential backoff (configurable)
- **Real-time Monitoring** - SSE-based event streaming for live dashboard updates
- **Supervisor Hierarchy** - Erlang-style supervision for fault tolerance

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                      Dioxus Frontend                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ AdminDash   │  │ QueueList   │  │ JobDetail           │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │ Server Functions
┌───────────────────────────┴─────────────────────────────────┐
│                      API Layer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ queues.rs   │  │ jobs.rs     │  │ realtime.rs (SSE)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │ Actor Messages
┌───────────────────────────┴─────────────────────────────────┐
│                    Actor System                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                   Supervisor                            ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐    ││
│  │  │ QueueActor 1 │  │ QueueActor 2 │  │ ...        │    ││
│  │  │  Workers...  │  │  Workers...  │  │            │    ││
│  │  └──────────────┘  └──────────────┘  └────────────┘    ││
│  └─────────────────────────────────────────────────────────┘│
└───────────────────────────┬─────────────────────────────────┘
                            │ DB Operations
┌───────────────────────────┴─────────────────────────────────┐
│                    SurrealDB                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ queue       │  │ job         │  │ job_history         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Adding New Job Types

### 1. Define the Job Handler

Create a handler in `packages/api/src/init.rs`:

```rust
handlers.register(FnHandler::new("my-job-type", |job: &Job| {
    let payload = job.payload.clone();
    Box::pin(async move {
        // Extract parameters from payload
        let param = payload.get("param")
            .and_then(|v| v.as_str())
            .ok_or("Missing param")?;

        // Do the work
        let result = do_work(param).await?;

        // Return success with optional output
        Ok(JobResult::with_output(
            "Job completed successfully",
            serde_json::json!({ "result": result }),
        ))
    })
}));
```

### 2. Create Jobs via API

```rust
let request = CreateJobRequest {
    queue_id: queue_id.to_string(),
    job_type: "my-job-type".to_string(),
    payload: json!({ "param": "value" }),
    priority: Some("normal".to_string()),
    max_retries: Some(3),
    timeout_secs: Some(300),
    tags: vec!["my-tag".to_string()],
};

let job = api::enqueue_job(request).await?;
```

### 3. Add UI for the Job Type (Optional)

Update `packages/ui/src/admin/create_job_form.rs` to add the new job type to the dropdown:

```rust
option { value: "my-job-type", "My Job Type" }
```

## Job Lifecycle

```
Created → Pending → Running → Completed
                  ↘         ↗
                   → Failed → (Retry) → Pending
                          ↘
                           → (Max Retries) → Archived
```

1. **Created** - Job is created via API
2. **Pending** - Job is in the queue waiting for a worker
3. **Running** - Worker is executing the job
4. **Completed** - Job succeeded, moved to history
5. **Failed** - Job failed, may retry or archive
6. **Cancelled** - Job was manually cancelled

## Quality Gates (REQUIRED)

**IMPORTANT**: Before returning completed work to the user, you MUST run these quality checks:

### Pre-Completion Checklist

```bash
# 1. Format check (must pass)
cargo fmt --all -- --check

# 2. Type check
cargo check --all-targets --all-features

# 3. Standard lint
cargo clippy --all-targets --all-features -- -D warnings

# 4. Strict no-panic lint (REQUIRED for all production code)
cargo clippy --all-targets --all-features -- \
    -D clippy::unwrap_used \
    -D clippy::expect_used \
    -D clippy::panic \
    -D clippy::unimplemented \
    -D clippy::todo \
    -D clippy::unreachable \
    -D clippy::indexing_slicing

# 5. Run tests
cargo test --all-features --workspace

# 6. Build WASM (for web target)
cd packages/web && npm run tailwind && dx build
```

### No-Panic Policy

This codebase enforces a **strict no-panic policy**. The following are **forbidden** in production code:

| Forbidden | Safe Alternative |
|-----------|------------------|
| `.unwrap()` | `.ok_or()`, `.unwrap_or()`, `.unwrap_or_default()`, `?` |
| `.expect()` | `.ok_or_else()`, `.map_err()`, `?` with context |
| `panic!()` | Return `Result<T, E>` or `Option<T>` |
| `todo!()` | Return `Err(...)` or implement properly |
| `unimplemented!()` | Return `Err(...)` describing why |
| `unreachable!()` | Return `Err(...)` or handle the case |
| `array[i]` | `.get(i)`, `.get_mut(i)`, iterators |

### Why No Panics?

Actors must handle errors gracefully to maintain system stability:
- Panicking actors crash and need supervisor intervention
- User-facing errors should be informative, not stack traces
- File-based persistence can fail - handle it gracefully
- Network/DB operations can timeout - handle it gracefully

---

## Testing Strategies

### SurrealDB Testing Requirements

**IMPORTANT**: All SurrealDB queries and mutations must have corresponding tests.

For every repository method, write tests that verify:
1. **Happy path** - Normal operation succeeds
2. **Not found** - Handles missing records gracefully
3. **Validation** - Rejects invalid data
4. **Edge cases** - Empty results, large data, special characters

```rust
#[tokio::test]
async fn test_job_create() {
    // Setup in-memory DB
    db::init(DbConfig::memory()).await.unwrap();

    let queue = Queue::new("test-queue");
    QueueRepository::create(&queue).await.unwrap();

    let job = Job::new(queue.id, "echo", json!({"msg": "hello"}));
    JobRepository::create(&job).await.unwrap();

    // Verify
    let loaded = JobRepository::get(job.id).await.unwrap();
    assert_eq!(loaded.job_type, "echo");
}

#[tokio::test]
async fn test_job_not_found() {
    db::init(DbConfig::memory()).await.unwrap();

    let result = JobRepository::get(JobId::new()).await;
    assert!(result.is_err()); // Should return error, not panic
}

#[tokio::test]
async fn test_job_list_with_filters() {
    db::init(DbConfig::memory()).await.unwrap();

    // Create test data...

    let filter = JobFilter {
        status: Some(JobStatus::Pending),
        ..Default::default()
    };

    let jobs = JobRepository::list(Some(filter)).await.unwrap();
    assert!(jobs.iter().all(|j| j.status == JobStatus::Pending));
}
```

### Unit Tests

Test individual components in isolation:

```rust
#[tokio::test]
async fn test_job_creation() {
    // Initialize in-memory DB
    db::init(DbConfig::memory()).await.unwrap();

    let queue = Queue::new("test-queue");
    QueueRepository::create(&queue).await.unwrap();

    let job = Job::new(queue.id, "echo", json!({"msg": "hello"}));
    JobRepository::create(&job).await.unwrap();

    let loaded = JobRepository::get(job.id).await.unwrap();
    assert_eq!(loaded.job_type, "echo");
}
```

### Integration Tests

Test the full actor system:

```rust
#[tokio::test]
async fn test_job_processing() {
    // Setup
    db::init(DbConfig::memory()).await.unwrap();

    let mut handlers = JobHandlerRegistry::new();
    handlers.register(FnHandler::new("test", |_| {
        Box::pin(async { Ok(JobResult::new("done")) })
    }));

    let (supervisor, _) = start_supervisor(handlers).await.unwrap();

    // Create queue
    let (tx, rx) = ractor::call::create_oneshot();
    supervisor.send_message(SupervisorMessage::CreateQueue {
        name: "test".into(),
        description: None,
        reply: tx,
    }).unwrap();
    let queue = rx.await.unwrap().unwrap();

    // Enqueue job
    let job = Job::new(queue.id, "test", json!({}));
    let (tx, rx) = ractor::call::create_oneshot();
    supervisor.send_message(SupervisorMessage::EnqueueJob {
        queue_id: queue.id,
        job,
        reply: tx,
    }).unwrap();
    rx.await.unwrap().unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify completion
    // ...
}
```

### E2E Tests with Playwright

```typescript
test('admin dashboard loads', async ({ page }) => {
    await page.goto('/admin');
    await expect(page.locator('h1')).toContainText('Job Queue Admin');
});

test('can create and view job', async ({ page }) => {
    await page.goto('/admin');

    // Select queue
    await page.click('.queue-card');

    // Create job
    await page.click('text=+ New Job');
    await page.selectOption('select', 'echo');
    await page.click('text=Create Job');

    // Verify job appears
    await expect(page.locator('.job-table')).toContainText('echo');
});
```

## CI/CD Pipeline

The project includes a comprehensive GitHub Actions CI pipeline (`.github/workflows/ci.yml`) that runs on every push and pull request:

| Job | Description | Blocking? |
|-----|-------------|-----------|
| `format` | Checks `cargo fmt` formatting | Yes |
| `check` | Runs `cargo check` for compilation errors | Yes |
| `lint` | Runs strict Clippy with no-panic policy | Yes |
| `test` | Runs all tests (debug + release) | Yes |
| `docs` | Builds documentation with warnings as errors | Yes |
| `wasm` | Builds WASM target with Dioxus CLI | Yes |
| `security` | Runs `cargo audit` for vulnerabilities | No (advisory) |

All jobs except `security` must pass for PRs to merge.

### Local CI Simulation

```bash
# Run all checks locally before pushing
cargo fmt --all -- --check && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all-features --workspace && \
cargo doc --all-features --no-deps
```

---

## Deployment (Railway)

### Environment Variables

```bash
# Railway provides these automatically
RAILWAY_ENVIRONMENT=production

# Optional: Custom database path
DATABASE_PATH=./data/surrealdb
```

### Persistence

The template automatically uses file-based storage on Railway:

```rust
let db_config = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
    DbConfig::file("./data/surrealdb")
} else {
    DbConfig::memory()
};
```

### Railway.toml

```toml
[build]
builder = "dockerfile"

[deploy]
startCommand = "dx serve --release"
healthcheckPath = "/"
healthcheckTimeout = 30
```

### Dockerfile

```dockerfile
FROM rust:1.83-bookworm as builder
WORKDIR /app
RUN cargo install dioxus-cli
COPY . .
RUN dx build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/web /app/web
COPY --from=builder /app/dist /app/dist
WORKDIR /app
EXPOSE 8080
CMD ["./web"]
```

## Monitoring & Observability

### Logging

The template uses `tracing` for structured logging:

```rust
tracing::info!("Job {} started", job.id);
tracing::warn!("Job {} failed: {}", job.id, error);
```

### Events

Subscribe to real-time events:

```rust
let mut rx = api::subscribe_events();
while let Ok(event) = rx.recv().await {
    match event {
        JobEvent::JobCompleted { job_id, duration_ms, .. } => {
            println!("Job {} completed in {}ms", job_id, duration_ms);
        }
        _ => {}
    }
}
```

## Best Practices

### 1. Idempotent Handlers

Design job handlers to be idempotent - running the same job twice should produce the same result:

```rust
handlers.register(FnHandler::new("process-order", |job| {
    Box::pin(async move {
        let order_id = job.payload["order_id"].as_str().unwrap();

        // Check if already processed
        if is_order_processed(order_id).await? {
            return Ok(JobResult::new("Already processed"));
        }

        // Process order...
        process_order(order_id).await?;
        mark_order_processed(order_id).await?;

        Ok(JobResult::new("Order processed"))
    })
}));
```

### 2. Graceful Timeouts

Set appropriate timeouts and handle them:

```rust
let job = Job::new(queue_id, "long-running", payload)
    .with_timeout(600)  // 10 minutes
    .with_max_retries(2);
```

### 3. Priority Appropriately

Use priority levels thoughtfully:

- **Critical** - User-facing, time-sensitive (password resets)
- **High** - Important but not urgent (email notifications)
- **Normal** - Standard background work
- **Low** - Maintenance, cleanup, analytics

### 4. Tag for Filtering

Use tags for organization and filtering:

```rust
let job = Job::new(queue_id, "send-email", payload)
    .with_tags(vec!["email".into(), "marketing".into()]);
```

## Extending the System

### Adding Queue Types

Create specialized queues for different workloads:

```rust
// High-throughput queue
let config = QueueConfig {
    concurrency: 16,
    default_timeout_secs: 30,
    max_queue_size: Some(10000),
    rate_limit: Some(100.0), // 100 jobs/sec
    ..Default::default()
};

let queue = Queue::new("fast-queue").with_config(config);
```

### Custom Supervision

Handle actor failures:

```rust
async fn handle_supervisor_evt(
    &self,
    _myself: ActorRef<Self::Msg>,
    message: SupervisionEvent,
    state: &mut Self::State,
) -> Result<(), ActorProcessingErr> {
    match message {
        SupervisionEvent::ActorTerminated(cell, _, reason) => {
            // Restart the actor or escalate
            if should_restart(&reason) {
                restart_actor(cell).await?;
            }
        }
        _ => {}
    }
    Ok(())
}
```

### Metrics Collection

Add custom metrics:

```rust
// In queue actor
state.queue.stats.completed += 1;

// Calculate throughput
let elapsed = now - state.last_stats_update;
state.queue.stats.throughput_per_min = Some(
    (state.jobs_since_last_update as f64 / elapsed.as_secs_f64()) * 60.0
);
```
