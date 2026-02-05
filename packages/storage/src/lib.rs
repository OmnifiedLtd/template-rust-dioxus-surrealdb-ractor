//! Object storage abstraction used by the example app.
//!
//! Goal:
//! - S3-compatible storage in production/staging
//! - On-disk storage for local dev
//! - In-memory storage for tests
//!
//! Implementation note:
//! This is intentionally a small wrapper around `object_store`, which already provides
//! S3, local filesystem, and in-memory backends.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use bytes::Bytes;
use object_store::ObjectStore;
use object_store::ObjectStoreExt;
use object_store::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid storage config: {0}")]
    InvalidConfig(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("object_store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    S3,
    Filesystem,
    Memory,
}

impl StorageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageKind::S3 => "s3",
            StorageKind::Filesystem => "filesystem",
            StorageKind::Memory => "memory",
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub allow_http: bool,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub virtual_hosted_style: bool,
}

#[derive(Debug, Clone)]
pub enum StorageBackendConfig {
    S3(S3Config),
    Filesystem { root: PathBuf },
    Memory,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend: StorageBackendConfig,
    /// Optional key prefix applied to all object keys.
    pub prefix: Option<String>,
}

impl StorageConfig {
    pub fn memory() -> Self {
        Self {
            backend: StorageBackendConfig::Memory,
            prefix: None,
        }
    }

    pub fn filesystem(root: impl Into<PathBuf>) -> Self {
        Self {
            backend: StorageBackendConfig::Filesystem { root: root.into() },
            prefix: None,
        }
    }

    pub fn s3(cfg: S3Config) -> Self {
        Self {
            backend: StorageBackendConfig::S3(cfg),
            prefix: None,
        }
    }

    /// Build a config from environment variables.
    ///
    /// Selection rules:
    /// - If `STORAGE_BACKEND` is set: use it (`s3`, `filesystem`, `memory`)
    /// - Otherwise: default to filesystem (`./data/object_store`)
    ///
    /// S3 env vars (S3-compatible):
    /// - `S3_BUCKET` (required when backend is `s3`)
    /// - `AWS_REGION` (default: `us-east-1`)
    /// - `S3_ENDPOINT` (optional, e.g. `http://localhost:9000`)
    /// - `S3_ALLOW_HTTP` (`true`/`false`, default: auto true if endpoint is http://)
    /// - `S3_VIRTUAL_HOSTED_STYLE` (`true`/`false`, default: false)
    /// - `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN` (optional; also picked up from the ambient AWS environment by the SDK)
    ///
    /// Filesystem env vars:
    /// - `STORAGE_FS_ROOT` (default: `./data/object_store`)
    ///
    /// Common:
    /// - `STORAGE_PREFIX` (optional, e.g. `example-app/`)
    pub fn from_env() -> Result<Self, StorageError> {
        let backend = std::env::var("STORAGE_BACKEND").ok();
        let prefix = std::env::var("STORAGE_PREFIX").ok().and_then(non_empty);

        let cfg = match backend.as_deref() {
            Some("s3") => Self::s3(read_s3_config()?),
            Some("filesystem") | Some("fs") => {
                let root = std::env::var("STORAGE_FS_ROOT")
                    .ok()
                    .and_then(non_empty)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("./data/object_store"));
                Self::filesystem(root)
            }
            Some("memory") | Some("mem") => Self::memory(),
            Some(other) => {
                return Err(StorageError::InvalidConfig(format!(
                    "unsupported STORAGE_BACKEND={other} (expected s3|filesystem|memory)"
                )));
            }
            None => {
                let root = std::env::var("STORAGE_FS_ROOT")
                    .ok()
                    .and_then(non_empty)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("./data/object_store"));
                Self::filesystem(root)
            }
        };

        Ok(Self { prefix, ..cfg })
    }
}

#[derive(Clone)]
pub struct Storage {
    kind: StorageKind,
    store: Arc<dyn ObjectStore>,
    prefix: Option<String>,
}

impl Storage {
    pub fn kind(&self) -> StorageKind {
        self.kind
    }

    pub fn kind_str(&self) -> &'static str {
        self.kind.as_str()
    }

    pub async fn new(cfg: StorageConfig) -> Result<Self, StorageError> {
        let (kind, store) = match cfg.backend {
            StorageBackendConfig::S3(s3) => (StorageKind::S3, Arc::new(build_s3(s3).await?) as _),
            StorageBackendConfig::Filesystem { root } => {
                ensure_dir(&root)?;
                let fs = object_store::local::LocalFileSystem::new_with_prefix(&root)?;
                (StorageKind::Filesystem, Arc::new(fs) as _)
            }
            StorageBackendConfig::Memory => {
                let mem = object_store::memory::InMemory::new();
                (StorageKind::Memory, Arc::new(mem) as _)
            }
        };

        Ok(Self {
            kind,
            store,
            prefix: cfg.prefix.and_then(non_empty),
        })
    }

    pub async fn from_env() -> Result<Self, StorageError> {
        Self::new(StorageConfig::from_env()?).await
    }

    fn to_path(&self, key: &str) -> Result<Path, StorageError> {
        let key = key.trim_start_matches('/');
        if key.is_empty() {
            return Err(StorageError::InvalidConfig(
                "object key must not be empty".to_string(),
            ));
        }

        let joined = match self.prefix.as_deref() {
            Some(prefix) => {
                let prefix = prefix.trim_matches('/');
                if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}/{key}")
                }
            }
            None => key.to_string(),
        };

        Ok(Path::from(joined))
    }

    pub async fn put_bytes(&self, key: &str, bytes: Bytes) -> Result<(), StorageError> {
        let path = self.to_path(key)?;
        self.store
            .put(&path, object_store::PutPayload::from(bytes))
            .await?;
        Ok(())
    }

    pub async fn get_bytes(&self, key: &str) -> Result<Bytes, StorageError> {
        let path = self.to_path(key)?;
        let res = self.store.get(&path).await?;
        Ok(res.bytes().await?)
    }

    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.to_path(key)?;
        self.store.delete(&path).await?;
        Ok(())
    }

    pub async fn put_json_value(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(value)?;
        self.put_bytes(key, Bytes::from(bytes)).await
    }

    pub async fn get_json_value(&self, key: &str) -> Result<serde_json::Value, StorageError> {
        let bytes = self.get_bytes(key).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn ensure_dir(root: &FsPath) -> Result<(), StorageError> {
    std::fs::create_dir_all(root)?;
    Ok(())
}

fn non_empty(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_bool_env(var_name: &str) -> Result<Option<bool>, StorageError> {
    let v = match std::env::var(var_name) {
        Ok(v) => v,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(e) => {
            return Err(StorageError::InvalidConfig(format!(
                "failed reading {var_name}: {e}"
            )));
        }
    };

    let normalized = v.trim().to_ascii_lowercase();
    let parsed = match normalized.as_str() {
        "1" | "true" | "yes" | "y" => true,
        "0" | "false" | "no" | "n" => false,
        _ => {
            return Err(StorageError::InvalidConfig(format!(
                "invalid boolean for {var_name}={v} (expected true/false)"
            )));
        }
    };
    Ok(Some(parsed))
}

fn read_s3_config() -> Result<S3Config, StorageError> {
    let bucket = std::env::var("S3_BUCKET")
        .ok()
        .and_then(non_empty)
        .ok_or_else(|| {
            StorageError::InvalidConfig("S3_BUCKET is required for s3 backend".into())
        })?;

    let region = std::env::var("AWS_REGION")
        .ok()
        .and_then(non_empty)
        .unwrap_or_else(|| "us-east-1".to_string());

    let endpoint = std::env::var("S3_ENDPOINT").ok().and_then(non_empty);
    let allow_http = match parse_bool_env("S3_ALLOW_HTTP")? {
        Some(v) => v,
        None => endpoint
            .as_deref()
            .is_some_and(|e| e.trim_start().to_ascii_lowercase().starts_with("http://")),
    };

    let virtual_hosted_style = parse_bool_env("S3_VIRTUAL_HOSTED_STYLE")?.unwrap_or(false);

    let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok().and_then(non_empty);
    let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .ok()
        .and_then(non_empty);
    let session_token = std::env::var("AWS_SESSION_TOKEN").ok().and_then(non_empty);

    Ok(S3Config {
        bucket,
        region,
        endpoint,
        allow_http,
        access_key_id,
        secret_access_key,
        session_token,
        virtual_hosted_style,
    })
}

async fn build_s3(cfg: S3Config) -> Result<object_store::aws::AmazonS3, StorageError> {
    let mut builder = object_store::aws::AmazonS3Builder::new()
        .with_bucket_name(cfg.bucket)
        .with_region(cfg.region)
        .with_virtual_hosted_style_request(cfg.virtual_hosted_style);

    if let Some(endpoint) = cfg.endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    if cfg.allow_http {
        builder = builder.with_allow_http(true);
    }
    if let Some(access_key_id) = cfg.access_key_id {
        builder = builder.with_access_key_id(access_key_id);
    }
    if let Some(secret_access_key) = cfg.secret_access_key {
        builder = builder.with_secret_access_key(secret_access_key);
    }
    if let Some(session_token) = cfg.session_token {
        builder = builder.with_token(session_token);
    }

    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;

    #[tokio::test]
    async fn in_memory_round_trip() -> Result<(), StorageError> {
        let storage = Storage::new(StorageConfig::memory()).await?;
        storage.put_bytes("hello.txt", Bytes::from("hi")).await?;
        let got = storage.get_bytes("hello.txt").await?;
        assert_eq!(got, Bytes::from("hi"));
        Ok(())
    }

    #[tokio::test]
    async fn filesystem_round_trip() -> Result<(), StorageError> {
        let dir = tempfile::tempdir()?;
        let storage = Storage::new(StorageConfig::filesystem(dir.path())).await?;

        let mut map = serde_json::Map::new();
        map.insert("a".to_string(), serde_json::Value::Number(1.into()));
        map.insert(
            "b".to_string(),
            serde_json::Value::String("two".to_string()),
        );
        let value = serde_json::Value::Object(map);
        storage.put_json_value("obj.json", &value).await?;
        let got = storage.get_json_value("obj.json").await?;
        assert_eq!(got, value);
        Ok(())
    }
}
