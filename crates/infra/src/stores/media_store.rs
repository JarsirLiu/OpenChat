use std::sync::Arc;

use anyhow::Context;

use crate::db::DatabasePool;

#[derive(Clone)]
pub struct PersistedMediaObject {
    pub object_key: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone)]
pub struct MediaObjectRecord {
    pub object_key: String,
    pub user_id: String,
    pub session_id: Option<String>,
}

#[derive(Clone)]
pub struct MediaObjectStore {
    pool: Arc<DatabasePool>,
}

impl MediaObjectStore {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    pub async fn upsert_media_object(&self, record: MediaObjectRecord) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO media_objects (object_key, user_id, session_id, created_at)
                    VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT(object_key) DO UPDATE SET
                      user_id = excluded.user_id,
                      session_id = excluded.session_id
                    "#,
                )
                .bind(record.object_key)
                .bind(record.user_id)
                .bind(record.session_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO media_objects (object_key, user_id, session_id, created_at)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT(object_key) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      session_id = EXCLUDED.session_id
                    "#,
                )
                .bind(record.object_key)
                .bind(record.user_id)
                .bind(record.session_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_media_object(
        &self,
        object_key: &str,
    ) -> anyhow::Result<Option<PersistedMediaObject>> {
        let row = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, i64)>(
                    r#"
                    SELECT object_key, user_id, session_id, created_at
                    FROM media_objects
                    WHERE object_key = ?1
                    "#,
                )
                .bind(object_key)
                .fetch_optional(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, i64)>(
                    r#"
                    SELECT object_key, user_id, session_id, created_at
                    FROM media_objects
                    WHERE object_key = $1
                    "#,
                )
                .bind(object_key)
                .fetch_optional(pool)
                .await
            }
        }
        .context("failed to get media object")?;

        Ok(row.map(
            |(object_key, user_id, session_id, created_at)| PersistedMediaObject {
                object_key,
                user_id,
                session_id,
                created_at: created_at.to_string(),
            },
        ))
    }
}

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
}
