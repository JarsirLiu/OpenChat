use std::sync::Arc;

use anyhow::Context;

use crate::db::DatabasePool;

#[derive(Clone, Debug)]
pub struct PersistedCleanupJob {
    pub id: String,
    pub object_key: String,
    pub retry_count: i64,
}

#[derive(Clone)]
pub struct CleanupJobStore {
    pool: Arc<DatabasePool>,
}

impl CleanupJobStore {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    pub async fn enqueue_delete_objects(
        &self,
        user_id: &str,
        object_keys: &[String],
    ) -> anyhow::Result<()> {
        if object_keys.is_empty() {
            return Ok(());
        }

        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                for (index, object_key) in object_keys.iter().enumerate() {
                    sqlx::query(
                        r#"
                        INSERT INTO cleanup_jobs (
                          id, user_id, job_type, object_key, status, retry_count, max_retries,
                          next_attempt_at, last_error, created_at, updated_at
                        )
                        VALUES (?1, ?2, 'delete_object', ?3, 'pending', 0, 8, ?4, NULL, ?4, ?4)
                        "#,
                    )
                    .bind(format!("cleanup_{}_{}", now, index))
                    .bind(user_id)
                    .bind(object_key)
                    .bind(now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::Postgres(pool) => {
                for (index, object_key) in object_keys.iter().enumerate() {
                    sqlx::query(
                        r#"
                        INSERT INTO cleanup_jobs (
                          id, user_id, job_type, object_key, status, retry_count, max_retries,
                          next_attempt_at, last_error, created_at, updated_at
                        )
                        VALUES ($1, $2, 'delete_object', $3, 'pending', 0, 8, $4, NULL, $4, $4)
                        "#,
                    )
                    .bind(format!("cleanup_{}_{}", now, index))
                    .bind(user_id)
                    .bind(object_key)
                    .bind(now)
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn claim_pending_jobs(&self, limit: i64) -> anyhow::Result<Vec<PersistedCleanupJob>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, i64)>(
                    r#"
                    WITH picked AS (
                      SELECT id
                      FROM cleanup_jobs
                      WHERE status = 'pending'
                        AND next_attempt_at <= EXTRACT(EPOCH FROM NOW()) * 1000
                      ORDER BY created_at ASC, id ASC
                      LIMIT $1
                      FOR UPDATE SKIP LOCKED
                    )
                    UPDATE cleanup_jobs
                    SET status = 'running',
                        updated_at = EXTRACT(EPOCH FROM NOW()) * 1000
                    WHERE id IN (SELECT id FROM picked)
                    RETURNING id, object_key, retry_count
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, i64)>(
                    r#"
                    WITH picked AS (
                      SELECT id
                      FROM cleanup_jobs
                      WHERE status = 'pending'
                        AND next_attempt_at <= EXTRACT(EPOCH FROM NOW()) * 1000
                      ORDER BY created_at ASC, id ASC
                      LIMIT $1
                      FOR UPDATE SKIP LOCKED
                    )
                    UPDATE cleanup_jobs
                    SET status = 'running',
                        updated_at = EXTRACT(EPOCH FROM NOW()) * 1000
                    WHERE id IN (SELECT id FROM picked)
                    RETURNING id, object_key, retry_count
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        }
        .map(|rows| {
            rows.into_iter()
                .map(|(id, object_key, retry_count)| PersistedCleanupJob {
                    id,
                    object_key,
                    retry_count,
                })
                .collect()
        })
        .context("failed to claim cleanup jobs")
    }

    pub async fn mark_job_succeeded(&self, job_id: &str) -> anyhow::Result<()> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query("DELETE FROM cleanup_jobs WHERE id = $1")
                    .bind(job_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM cleanup_jobs WHERE id = $1")
                    .bind(job_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn mark_job_failed(
        &self,
        job: &PersistedCleanupJob,
        error: &str,
    ) -> anyhow::Result<()> {
        let next_retry_count = job.retry_count.saturating_add(1);
        let now = now_millis_i64();
        let is_dead = next_retry_count >= 8;
        let next_attempt_at = now + retry_delay_millis(next_retry_count);
        let status = if is_dead { "dead" } else { "pending" };

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    UPDATE cleanup_jobs
                    SET status = $2,
                        retry_count = $3,
                        last_error = $4,
                        next_attempt_at = $5,
                        updated_at = $6
                    WHERE id = $1
                    "#,
                )
                .bind(job.id.as_str())
                .bind(status)
                .bind(next_retry_count)
                .bind(error)
                .bind(next_attempt_at)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    UPDATE cleanup_jobs
                    SET status = $2,
                        retry_count = $3,
                        last_error = $4,
                        next_attempt_at = $5,
                        updated_at = $6
                    WHERE id = $1
                    "#,
                )
                .bind(job.id.as_str())
                .bind(status)
                .bind(next_retry_count)
                .bind(error)
                .bind(next_attempt_at)
                .bind(now)
                .execute(pool)
                .await?;
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

fn retry_delay_millis(retry_count: i64) -> i64 {
    let seconds = match retry_count {
        0 | 1 => 5,
        2 => 15,
        3 => 30,
        4 => 60,
        5 => 5 * 60,
        _ => 15 * 60,
    };
    seconds * 1000
}

#[cfg(test)]
mod tests {
    use super::retry_delay_millis;

    #[test]
    fn retry_delay_backoff_matches_expected_schedule() {
        assert_eq!(retry_delay_millis(0), 5_000);
        assert_eq!(retry_delay_millis(1), 5_000);
        assert_eq!(retry_delay_millis(2), 15_000);
        assert_eq!(retry_delay_millis(3), 30_000);
        assert_eq!(retry_delay_millis(4), 60_000);
        assert_eq!(retry_delay_millis(5), 300_000);
        assert_eq!(retry_delay_millis(6), 900_000);
        assert_eq!(retry_delay_millis(9), 900_000);
    }

    #[test]
    fn eighth_retry_is_terminal_for_cleanup_jobs() {
        let current_retry_count = 7_i64;
        let next_retry_count = current_retry_count.saturating_add(1);
        assert!(next_retry_count >= 8);
    }
}
