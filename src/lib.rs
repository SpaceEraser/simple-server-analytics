use std::path::Path;

use sqlx::SqlitePool;

pub mod salvo_ext;

const DB_TABLE_PREFIX: &str = "_sa_";

pub struct SimpleAnalytics {
    pool: SqlitePool,
}

impl SimpleAnalytics {
    pub async fn new<P: AsRef<Path>>(path: P) -> sqlx::Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true)
            .pragma("cache_size", "-10000")
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .optimize_on_close(true, None)
            .filename(path);
        let pool = sqlx::Pool::connect_with(options).await?;

        Ok(Self { pool })
    }

    pub fn with_existing_pool(pool: &SqlitePool) -> Self {
        Self { pool: pool.clone() }
    }
}
