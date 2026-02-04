use std::sync::LazyLock;

use tokio::sync::{Mutex, MutexGuard};

use db::{DbConfig, DbError};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub async fn setup_db() -> Result<MutexGuard<'static, ()>, DbError> {
    let guard = TEST_LOCK.lock().await;
    db::init(DbConfig::memory()).await?;
    let db_conn = db::get_db()?;
    db_conn
        .query("DELETE job_history; DELETE job; DELETE queue;")
        .await?;
    Ok(guard)
}
