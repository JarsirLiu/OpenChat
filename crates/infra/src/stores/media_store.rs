use std::sync::Arc;

use anyhow::Context;

use crate::db::DatabasePool;

#[derive(Clone)]
pub struct PersistedMediaObject {
    pub object_key: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone)]
pub struct MediaObjectRecord {
    pub object_key: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
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
                    INSERT INTO media_objects (object_key, user_id, session_id, turn_id, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT(object_key) DO UPDATE SET
                      user_id = excluded.user_id,
                      session_id = excluded.session_id,
                      turn_id = excluded.turn_id
                    "#,
                )
                .bind(record.object_key)
                .bind(record.user_id)
                .bind(record.session_id)
                .bind(record.turn_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO media_objects (object_key, user_id, session_id, turn_id, created_at)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT(object_key) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      session_id = EXCLUDED.session_id,
                      turn_id = EXCLUDED.turn_id
                    "#,
                )
                .bind(record.object_key)
                .bind(record.user_id)
                .bind(record.session_id)
                .bind(record.turn_id)
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
                sqlx::query_as::<_, (String, String, Option<String>, Option<String>, i64)>(
                    r#"
                    SELECT object_key, user_id, session_id, turn_id, created_at
                    FROM media_objects
                    WHERE object_key = ?1
                    "#,
                )
                .bind(object_key)
                .fetch_optional(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, Option<String>, i64)>(
                    r#"
                    SELECT object_key, user_id, session_id, turn_id, created_at
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
            |(object_key, user_id, session_id, turn_id, created_at)| PersistedMediaObject {
                object_key,
                user_id,
                session_id,
                turn_id,
                created_at: created_at.to_string(),
            },
        ))
    }

    pub async fn list_media_object_keys_for_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<String>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT object_key
                    FROM media_objects
                    WHERE user_id = ?1 AND session_id = ?2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT object_key
                    FROM media_objects
                    WHERE user_id = $1 AND session_id = $2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list media objects for session")
    }

    pub async fn list_media_object_keys_for_turns(
        &self,
        user_id: &str,
        turn_ids: &[String],
    ) -> anyhow::Result<Vec<String>> {
        if turn_ids.is_empty() {
            return Ok(Vec::new());
        }

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let mut query = sqlx::QueryBuilder::new(
                    "SELECT object_key FROM media_objects WHERE user_id = ",
                );
                query.push_bind(user_id);
                query.push(" AND turn_id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query.build_query_scalar::<String>().fetch_all(pool).await
            }
            DatabasePool::Postgres(pool) => {
                let mut query = sqlx::QueryBuilder::new(
                    "SELECT object_key FROM media_objects WHERE user_id = ",
                );
                query.push_bind(user_id);
                query.push(" AND turn_id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query.build_query_scalar::<String>().fetch_all(pool).await
            }
        }
        .context("failed to list media objects for turns")
    }

    pub async fn assign_session_to_objects(
        &self,
        user_id: &str,
        session_id: &str,
        object_keys: &[String],
    ) -> anyhow::Result<()> {
        if object_keys.is_empty() {
            return Ok(());
        }

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let mut query = sqlx::QueryBuilder::new("UPDATE media_objects SET session_id = ");
                query.push_bind(session_id);
                query.push(" WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND object_key IN (");
                {
                    let mut separated = query.separated(", ");
                    for object_key in object_keys {
                        separated.push_bind(object_key);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut query = sqlx::QueryBuilder::new("UPDATE media_objects SET session_id = ");
                query.push_bind(session_id);
                query.push(" WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND object_key IN (");
                {
                    let mut separated = query.separated(", ");
                    for object_key in object_keys {
                        separated.push_bind(object_key);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;
            }
        }

        Ok(())
    }
}

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
}
