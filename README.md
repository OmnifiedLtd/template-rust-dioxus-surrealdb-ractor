# Dioxus + Ractor + SurrealDB Job Queue Template

A full-stack Rust application template featuring a job queue system with an admin dashboard. Built with Dioxus 0.7 for the frontend, Ractor for actor-based concurrency, and SurrealDB for persistence.

## Features

- **Job Queue System**: Priority-based job scheduling with retry support
- **Admin Dashboard**: Real-time monitoring and management UI at `/admin`
- **Durable Persistence**: Jobs survive server restarts with SurrealDB
- **Actor-based Architecture**: Scalable, fault-tolerant design with Ractor

---

## Persistence Strategy

The job queue system uses SurrealDB for persistence with two modes:

### Development Mode (In-Memory)

By default, local development uses an **in-memory database** for fast iteration:

```rust
DbConfig::memory()  // Data is lost when server stops
```

This is ideal for development because:
- Instant startup (no disk I/O)
- Clean slate on each restart
- No leftover test data

### Production Mode (File-Based)

When deployed (detected via `RAILWAY_ENVIRONMENT` env var), the system automatically switches to **file-based persistence**:

```rust
DbConfig::file("./data/surrealdb")  // Data persists to disk
```

You can also force file-based storage locally by setting:

```bash
export RAILWAY_ENVIRONMENT=production
```

### Job Rehydration on Restart

When the server restarts, the system automatically:

1. **Resets stale "running" jobs** → Jobs that were mid-execution when the server stopped are reset to "pending" status
2. **Loads pending jobs** → All pending jobs are loaded from the database into memory for processing
3. **Preserves queue state** → Paused/running state and configurations are restored

This ensures **no jobs are lost** during deployments or crashes.

---

## Volume Setup for Cloud Deployment

To make your job queue durable in production, you need to configure a persistent volume.

### Railway

1. **Create a volume** in your Railway project:
   ```
   Railway Dashboard → Your Service → Volumes → Add Volume
   ```

2. **Mount path**: `/app/data`

3. **Update the storage path** in `packages/api/src/init.rs` if needed:
   ```rust
   DbConfig::file("/app/data/surrealdb")
   ```

4. Railway will automatically persist data across deployments.

### Fly.io

1. **Create a volume**:
   ```bash
   fly volumes create data --region <your-region> --size 1
   ```

2. **Add to `fly.toml`**:
   ```toml
   [mounts]
     source = "data"
     destination = "/app/data"
   ```

3. **Update the storage path** in your code:
   ```rust
   DbConfig::file("/app/data/surrealdb")
   ```

4. **Deploy**:
   ```bash
   fly deploy
   ```

### Docker (Self-Hosted)

```bash
docker run -v /path/on/host:/app/data your-image
```

Or with Docker Compose:

```yaml
services:
  app:
    volumes:
      - app_data:/app/data

volumes:
  app_data:
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RAILWAY_ENVIRONMENT` | Triggers file-based persistence when set | (unset) |
| `DATABASE_PATH` | Custom database path (future) | `./data/surrealdb` |
| `STORAGE_BACKEND` | Object storage backend: `s3`, `filesystem`, `memory` | `filesystem` |
| `STORAGE_FS_ROOT` | Root directory when `STORAGE_BACKEND=filesystem` | `./data/object_store` |
| `STORAGE_PREFIX` | Optional key prefix applied to all objects | (unset) |
| `S3_BUCKET` | Bucket name when `STORAGE_BACKEND=s3` | (required for `s3`) |
| `AWS_REGION` | Region when `STORAGE_BACKEND=s3` | `us-east-1` |
| `S3_ENDPOINT` | Custom endpoint for S3-compatible storage (MinIO, R2, etc.) | (unset) |
| `S3_ALLOW_HTTP` | Allow plain HTTP for custom endpoints (`true`/`false`) | auto (true for `http://` endpoint) |
| `S3_VIRTUAL_HOSTED_STYLE` | Use virtual-hosted style requests (`true`/`false`) | `false` |
| `AWS_ACCESS_KEY_ID` | Access key for S3 (optional if using ambient AWS auth) | (unset) |
| `AWS_SECRET_ACCESS_KEY` | Secret key for S3 (optional if using ambient AWS auth) | (unset) |
| `AWS_SESSION_TOKEN` | Session token for S3 (optional) | (unset) |

### Object Storage Example (S3-Compatible)

The example app includes a demo job type `persist-object` that writes a JSON value to object storage and reads it back to prove persistence.

Example payload:

```json
{
  "key": "demo/hello.json",
  "data": { "message": "Hello from object storage" }
}
```

Local dev (filesystem):

```bash
export STORAGE_BACKEND=filesystem
export STORAGE_FS_ROOT=./data/object_store
```

S3-compatible (example: MinIO on localhost):

```bash
export STORAGE_BACKEND=s3
export S3_BUCKET=demo
export AWS_REGION=us-east-1
export S3_ENDPOINT=http://localhost:9000
export S3_ALLOW_HTTP=true
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
```

### Queue Configuration

Queues are configured in code with sensible defaults:

```rust
let queue = Queue::new("my-queue")
    .with_description("Processing queue")
    .with_concurrency(4)           // Max parallel workers
    .with_max_retries(3)           // Retry failed jobs
    .with_timeout(Duration::from_secs(300));
```

---

# Development

Your new workspace contains a member crate for each of the web, desktop and mobile platforms, a `ui` crate for shared components and a `api` crate for shared backend logic:

```
your_project/
├─ README.md
├─ Cargo.toml
└─ packages/
   ├─ web/
   │  └─ ... # Web specific UI/logic
   ├─ desktop/
   │  └─ ... # Desktop specific UI/logic
   ├─ mobile/
   │  └─ ... # Mobile specific UI/logic
   ├─ api/
   │  └─ ... # All shared server logic
   └─  ui/
      └─ ... # Component shared between multiple platforms
```

## Platform crates

Each platform crate contains the entry point for the platform, and any assets, components and dependencies that are specific to that platform. For example, the desktop crate in the workspace looks something like this:

```
desktop/ # The desktop crate contains all platform specific UI, logic and dependencies for the desktop app
├─ assets/ # Assets used by the desktop app - Any platform specific assets should go in this folder
├─ src/
│  ├─ main.rs # The entrypoint for the desktop app. It also defines the routes for the desktop platform
│  ├─ views/ # The views each route will render in the desktop version of the app
│  │  ├─ mod.rs # Defines the module for the views route and re-exports the components for each route
│  │  ├─ blog.rs # The component that will render at the /blog/:id route
│  │  ├─ home.rs # The component that will render at the / route
├─ Cargo.toml # The desktop crate's Cargo.toml - This should include all desktop specific dependencies
```

When you start developing with the workspace setup each of the platform crates will look almost identical. The UI starts out exactly the same on all platforms. However, as you continue developing your application, this setup makes it easy to let the views for each platform change independently.

## Shared UI crate

The workspace contains a `ui` crate with components that are shared between multiple platforms. You should put any UI elements you want to use in multiple platforms in this crate. You can also put some shared client side logic in this crate, but be careful to not pull in platform specific dependencies. The `ui` crate starts out something like this:

```
ui/
├─ src/
│  ├─ lib.rs # The entrypoint for the ui crate
│  ├─ hero.rs # The Hero component that will be used in every platform
│  ├─ echo.rs # The shared echo component that communicates with the server
│  ├─ navbar.rs # The Navbar component that will be used in the layout of every platform's router
```

## Shared backend logic

The workspace contains a `api` crate with shared backend logic. This crate defines all of the shared server functions for all platforms. Server functions are async functions that expose a public API on the server. They can be called like a normal async function from the client. When you run `dx serve`, all of the server functions will be collected in the server build and hosted on a public API for the client to call. The `api` crate starts out something like this:

```
api/
├─ src/
│  ├─ lib.rs # Exports a server function that echos the input string
```

### Serving Your App

Navigate to the platform crate of your choice:
```bash
cd web
```

and serve:

```bash
dx serve
```
