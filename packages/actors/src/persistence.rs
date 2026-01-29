//! File-based state persistence for actors.

use std::path::{Path, PathBuf};
use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// State persistence for actors.
///
/// Provides file-based persistence for actor state, suitable for
/// Railway deployment and local development.
pub struct StatePersistence {
    /// Base directory for state files.
    base_dir: PathBuf,
}

impl StatePersistence {
    /// Create a new persistence instance.
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Create persistence with default data directory.
    pub fn default_dir() -> Self {
        Self::new("./data/queues")
    }

    /// Ensure the base directory exists.
    pub async fn init(&self) -> Result<(), PersistenceError> {
        fs::create_dir_all(&self.base_dir).await?;
        Ok(())
    }

    /// Save state to a file.
    pub async fn save<T: Serialize>(&self, name: &str, state: &T) -> Result<(), PersistenceError> {
        let path = self.base_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(state)?;

        // Write to temp file first, then rename for atomicity
        let temp_path = self.base_dir.join(format!("{}.json.tmp", name));
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;
        fs::rename(&temp_path, &path).await?;

        tracing::debug!("Saved state to {:?}", path);
        Ok(())
    }

    /// Load state from a file.
    pub async fn load<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, PersistenceError> {
        let path = self.base_dir.join(format!("{}.json", name));

        if !path.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&path).await?;
        let mut json = String::new();
        file.read_to_string(&mut json).await?;

        let state: T = serde_json::from_str(&json)?;
        tracing::debug!("Loaded state from {:?}", path);

        Ok(Some(state))
    }

    /// Delete a state file.
    pub async fn delete(&self, name: &str) -> Result<(), PersistenceError> {
        let path = self.base_dir.join(format!("{}.json", name));

        if path.exists() {
            fs::remove_file(&path).await?;
            tracing::debug!("Deleted state file {:?}", path);
        }

        Ok(())
    }

    /// List all saved state names.
    pub async fn list(&self) -> Result<Vec<String>, PersistenceError> {
        let mut names = Vec::new();

        if !self.base_dir.exists() {
            return Ok(names);
        }

        let mut entries = fs::read_dir(&self.base_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(stem) = path.file_stem() {
                    names.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(names)
    }
}

/// Persistence errors.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
