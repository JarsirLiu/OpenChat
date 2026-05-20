use std::sync::Arc;

use anyhow::Context;
use serde_json::Value;
use crate::db::DatabasePool;

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
}

#[derive(Clone)]
pub struct PersistedMessage {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub role: String,
    pub status: String,
    pub content_json: String,
    pub tool_call_id: Option<String>,
}

#[derive(Clone)]
pub struct PersistedToolCall {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub parent_item_id: Option<String>,
    pub tool_name: String,
    pub tool_display_name: Option<String>,
    pub arguments_text: Option<String>,
    pub result_json: Option<String>,
    pub status: String,
    pub media_json: Option<String>,
}

#[derive(Clone)]
pub struct PersistedTurnTerminalReason {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct ChatStore {
    pool: Arc<DatabasePool>,
}

#[derive(Clone)]
pub struct PersistedSession {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct PersistedSessionToolCall {
    pub id: String,
    pub turn_id: String,
    pub parent_item_id: Option<String>,
    pub tool_name: String,
    pub tool_display_name: Option<String>,
    pub arguments_text: Option<String>,
    pub result_json: Option<String>,
    pub status: String,
    pub media_json: Option<String>,
}

#[derive(Clone)]
pub struct PersistedSessionMessage {
    pub id: String,
    pub session_id: String,
    pub turn_id: String,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub content: Value,
    pub tool_call_id: Option<String>,
}

impl ChatStore {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    pub async fn ensure_session(&self, user_id: &str, session_id: &str) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, title, status, created_at, updated_at)
                    VALUES (?1, ?2, NULL, 'idle', ?3, ?3)
                    ON CONFLICT(id) DO UPDATE SET
                      updated_at = excluded.updated_at
                    WHERE sessions.user_id = excluded.user_id
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, title, status, created_at, updated_at)
                    VALUES ($1, $2, NULL, 'idle', $3, $3)
                    ON CONFLICT(id) DO UPDATE SET
                      updated_at = EXCLUDED.updated_at
                    WHERE sessions.user_id = EXCLUDED.user_id
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_session_title(
        &self,
        user_id: &str,
        session_id: &str,
        title: &str,
    ) -> anyhow::Result<Option<PersistedSession>> {
        let now = now_millis_i64();
        let normalized_title = title.trim();
        if normalized_title.is_empty() {
            return self.get_session(user_id, session_id).await;
        }

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query("UPDATE sessions SET title = ?3, updated_at = ?4 WHERE id = ?1 AND user_id = ?2")
                    .bind(session_id)
                    .bind(user_id)
                    .bind(normalized_title)
                    .bind(now)
                    .execute(pool)
                    .await
                    .context("failed to update session title")?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE sessions SET title = $3, updated_at = $4 WHERE id = $1 AND user_id = $2")
                    .bind(session_id)
                    .bind(user_id)
                    .bind(normalized_title)
                    .bind(now)
                    .execute(pool)
                    .await
                    .context("failed to update session title")?;
            }
        }

        self.get_session(user_id, session_id).await
    }

    pub async fn begin_turn(
        &self,
        user_id: &str,
        turn_id: &str,
        session_id: &str,
        prompt: &str,
        text_model_config_id: &str,
        image_tool_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = now_millis_i64();
        // The session row must already be owned by the current user before a turn starts.

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO turns (id, session_id, user_id, prompt, text_model_config_id, image_tool_id, status, started_at, completed_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7, NULL)
                    "#,
                )
                .bind(turn_id)
                .bind(session_id)
                .bind(user_id)
                .bind(prompt)
                .bind(text_model_config_id)
                .bind(image_tool_id)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query("UPDATE sessions SET status = 'running', updated_at = ?2 WHERE id = ?1")
                    .bind(session_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO turns (id, session_id, user_id, prompt, text_model_config_id, image_tool_id, status, started_at, completed_at)
                    VALUES ($1, $2, $3, $4, $5, $6, 'running', $7, NULL)
                    "#,
                )
                .bind(turn_id)
                .bind(session_id)
                .bind(user_id)
                .bind(prompt)
                .bind(text_model_config_id)
                .bind(image_tool_id)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query("UPDATE sessions SET status = 'running', updated_at = $2 WHERE id = $1")
                    .bind(session_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_message(&self, message: PersistedMessage) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO messages (id, user_id, session_id, turn_id, role, status, content_json, tool_call_id, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = excluded.user_id,
                      status = excluded.status,
                      content_json = excluded.content_json,
                      tool_call_id = excluded.tool_call_id,
                      updated_at = excluded.updated_at
                    "#,
                )
                .bind(message.id)
                .bind(message.user_id)
                .bind(message.session_id)
                .bind(message.turn_id)
                .bind(message.role)
                .bind(message.status)
                .bind(message.content_json)
                .bind(message.tool_call_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO messages (id, user_id, session_id, turn_id, role, status, content_json, tool_call_id, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      status = EXCLUDED.status,
                      content_json = EXCLUDED.content_json,
                      tool_call_id = EXCLUDED.tool_call_id,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(message.id)
                .bind(message.user_id)
                .bind(message.session_id)
                .bind(message.turn_id)
                .bind(message.role)
                .bind(message.status)
                .bind(message.content_json)
                .bind(message.tool_call_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn upsert_tool_call(&self, tool_call: PersistedToolCall) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO tool_calls (id, user_id, session_id, turn_id, parent_item_id, tool_name, tool_display_name, arguments_text, result_json, status, media_json, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = excluded.user_id,
                      parent_item_id = excluded.parent_item_id,
                      tool_display_name = excluded.tool_display_name,
                      arguments_text = excluded.arguments_text,
                      result_json = excluded.result_json,
                      status = excluded.status,
                      media_json = excluded.media_json,
                      updated_at = excluded.updated_at
                    "#,
                )
                .bind(tool_call.id)
                .bind(tool_call.user_id)
                .bind(tool_call.session_id)
                .bind(tool_call.turn_id)
                .bind(tool_call.parent_item_id)
                .bind(tool_call.tool_name)
                .bind(tool_call.tool_display_name)
                .bind(tool_call.arguments_text)
                .bind(tool_call.result_json)
                .bind(tool_call.status)
                .bind(tool_call.media_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO tool_calls (id, user_id, session_id, turn_id, parent_item_id, tool_name, tool_display_name, arguments_text, result_json, status, media_json, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      parent_item_id = EXCLUDED.parent_item_id,
                      tool_display_name = EXCLUDED.tool_display_name,
                      arguments_text = EXCLUDED.arguments_text,
                      result_json = EXCLUDED.result_json,
                      status = EXCLUDED.status,
                      media_json = EXCLUDED.media_json,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(tool_call.id)
                .bind(tool_call.user_id)
                .bind(tool_call.session_id)
                .bind(tool_call.turn_id)
                .bind(tool_call.parent_item_id)
                .bind(tool_call.tool_name)
                .bind(tool_call.tool_display_name)
                .bind(tool_call.arguments_text)
                .bind(tool_call.result_json)
                .bind(tool_call.status)
                .bind(tool_call.media_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn complete_turn(
        &self,
        turn_id: &str,
        session_id: &str,
        status: &str,
        terminal_reason: Option<&PersistedTurnTerminalReason>,
    ) -> anyhow::Result<()> {
        let now = now_millis_i64();
        let session_status = match status {
            "completed" => "completed",
            "interrupted" => "interrupted",
            _ => "failed",
        };
        let item_status = match status {
            "completed" => "completed",
            "interrupted" => "interrupted",
            _ => "failed",
        };
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    "UPDATE turns SET status = ?2, completed_at = ?3, terminal_reason_code = ?4, terminal_reason_message = ?5 WHERE id = ?1",
                )
                    .bind(turn_id)
                    .bind(status)
                    .bind(now)
                    .bind(terminal_reason.map(|reason| reason.code.as_str()))
                    .bind(terminal_reason.and_then(|reason| reason.message.as_deref()))
                    .execute(pool)
                    .await?;

                sqlx::query("UPDATE sessions SET status = ?2, updated_at = ?3 WHERE id = ?1")
                    .bind(session_id)
                    .bind(session_status)
                    .bind(now)
                    .execute(pool)
                    .await?;

                sqlx::query(
                    "UPDATE messages SET status = ?2, updated_at = ?3 WHERE turn_id = ?1 AND status = 'in_progress'",
                )
                .bind(turn_id)
                .bind(item_status)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE tool_calls SET status = ?2, updated_at = ?3 WHERE turn_id = ?1 AND status = 'in_progress'",
                )
                .bind(turn_id)
                .bind(item_status)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE turns SET status = $2, completed_at = $3, terminal_reason_code = $4, terminal_reason_message = $5 WHERE id = $1",
                )
                    .bind(turn_id)
                    .bind(status)
                    .bind(now)
                    .bind(terminal_reason.map(|reason| reason.code.as_str()))
                    .bind(terminal_reason.and_then(|reason| reason.message.as_deref()))
                    .execute(pool)
                    .await?;

                sqlx::query("UPDATE sessions SET status = $2, updated_at = $3 WHERE id = $1")
                    .bind(session_id)
                    .bind(session_status)
                    .bind(now)
                    .execute(pool)
                    .await?;

                sqlx::query(
                    "UPDATE messages SET status = $2, updated_at = $3 WHERE turn_id = $1 AND status = 'in_progress'",
                )
                .bind(turn_id)
                .bind(item_status)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE tool_calls SET status = $2, updated_at = $3 WHERE turn_id = $1 AND status = 'in_progress'",
                )
                .bind(turn_id)
                .bind(item_status)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn reconcile_session_items(&self, user_id: &str, session_id: &str) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    UPDATE messages
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = messages.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = messages.turn_id
                            ) = 'interrupted' THEN 'interrupted'
                            ELSE 'failed'
                        END
                    ),
                    updated_at = ?3
                    WHERE session_id = ?1
                      AND user_id = ?2
                      AND status = 'in_progress'
                      AND turn_id IN (
                          SELECT id FROM turns
                          WHERE session_id = ?1 AND user_id = ?2 AND status != 'running'
                      )
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query(
                    r#"
                    UPDATE tool_calls
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = tool_calls.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = tool_calls.turn_id
                            ) = 'interrupted' THEN 'interrupted'
                            ELSE 'failed'
                        END
                    ),
                    updated_at = ?3
                    WHERE session_id = ?1
                      AND user_id = ?2
                      AND status = 'in_progress'
                      AND turn_id IN (
                          SELECT id FROM turns
                          WHERE session_id = ?1 AND user_id = ?2 AND status != 'running'
                      )
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    UPDATE messages
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = messages.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = messages.turn_id
                            ) = 'interrupted' THEN 'interrupted'
                            ELSE 'failed'
                        END
                    ),
                    updated_at = $3
                    WHERE session_id = $1
                      AND user_id = $2
                      AND status = 'in_progress'
                      AND turn_id IN (
                          SELECT id FROM turns
                          WHERE session_id = $1 AND user_id = $2 AND status != 'running'
                      )
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;

                sqlx::query(
                    r#"
                    UPDATE tool_calls
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = tool_calls.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = tool_calls.turn_id
                            ) = 'interrupted' THEN 'interrupted'
                            ELSE 'failed'
                        END
                    ),
                    updated_at = $3
                    WHERE session_id = $1
                      AND user_id = $2
                      AND status = 'in_progress'
                      AND turn_id IN (
                          SELECT id FROM turns
                          WHERE session_id = $1 AND user_id = $2 AND status != 'running'
                      )
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn interrupt_running_turn(
        &self,
        user_id: &str,
        session_id: &str,
        turn_id: &str,
    ) -> anyhow::Result<bool> {
        let rows_affected = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    "UPDATE turns SET status = 'interrupted', completed_at = ?4 WHERE id = ?1 AND session_id = ?2 AND user_id = ?3 AND status = 'running'",
                )
                .bind(turn_id)
                .bind(session_id)
                .bind(user_id)
                .bind(now_millis_i64())
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE turns SET status = 'interrupted', completed_at = $4 WHERE id = $1 AND session_id = $2 AND user_id = $3 AND status = 'running'",
                )
                .bind(turn_id)
                .bind(session_id)
                .bind(user_id)
                .bind(now_millis_i64())
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        if rows_affected == 0 {
            return Ok(false);
        }

        self.complete_turn(
            turn_id,
            session_id,
            "interrupted",
            Some(&PersistedTurnTerminalReason {
                code: "user_requested".into(),
                message: Some("用户已停止本轮回复".into()),
            }),
        )
        .await?;
        Ok(true)
    }

    pub async fn reconcile_session_runtime_state(
        &self,
        user_id: &str,
        session_id: &str,
        active_turn_ids: &[String],
    ) -> anyhow::Result<()> {
        let now = now_millis_i64();
        let active_turn_id = active_turn_ids.first().map(String::as_str);

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                if let Some(active_turn_id) = active_turn_id {
                    sqlx::query(
                        r#"
                        UPDATE turns
                        SET status = 'interrupted', completed_at = ?4, terminal_reason_code = 'session_recovered', terminal_reason_message = '服务已恢复，上一轮响应已中止'
                        WHERE session_id = ?1
                          AND user_id = ?2
                          AND status = 'running'
                          AND id != ?3
                        "#,
                    )
                    .bind(session_id)
                    .bind(user_id)
                    .bind(active_turn_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        UPDATE turns
                        SET status = 'interrupted', completed_at = ?3, terminal_reason_code = 'session_recovered', terminal_reason_message = '服务已恢复，上一轮响应已中止'
                        WHERE session_id = ?1
                          AND user_id = ?2
                          AND status = 'running'
                        "#,
                    )
                    .bind(session_id)
                    .bind(user_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
                }

                sqlx::query(
                    r#"
                    UPDATE sessions
                    SET status = (
                        CASE
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = ?1 AND user_id = ?2 AND status = 'running'
                            ) THEN 'running'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = ?1 AND user_id = ?2 AND status = 'failed'
                            ) THEN 'failed'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = ?1 AND user_id = ?2 AND status = 'interrupted'
                            ) THEN 'interrupted'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = ?1 AND user_id = ?2 AND status = 'completed'
                            ) THEN 'completed'
                            ELSE status
                        END
                    ),
                    updated_at = ?3
                    WHERE id = ?1 AND user_id = ?2
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                if let Some(active_turn_id) = active_turn_id {
                    sqlx::query(
                        r#"
                        UPDATE turns
                        SET status = 'interrupted', completed_at = $4, terminal_reason_code = 'session_recovered', terminal_reason_message = '服务已恢复，上一轮响应已中止'
                        WHERE session_id = $1
                          AND user_id = $2
                          AND status = 'running'
                          AND id != $3
                        "#,
                    )
                    .bind(session_id)
                    .bind(user_id)
                    .bind(active_turn_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        UPDATE turns
                        SET status = 'interrupted', completed_at = $3, terminal_reason_code = 'session_recovered', terminal_reason_message = '服务已恢复，上一轮响应已中止'
                        WHERE session_id = $1
                          AND user_id = $2
                          AND status = 'running'
                        "#,
                    )
                    .bind(session_id)
                    .bind(user_id)
                    .bind(now)
                    .execute(pool)
                    .await?;
                }

                sqlx::query(
                    r#"
                    UPDATE sessions
                    SET status = (
                        CASE
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = $1 AND user_id = $2 AND status = 'running'
                            ) THEN 'running'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = $1 AND user_id = $2 AND status = 'failed'
                            ) THEN 'failed'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = $1 AND user_id = $2 AND status = 'interrupted'
                            ) THEN 'interrupted'
                            WHEN EXISTS (
                                SELECT 1 FROM turns
                                WHERE session_id = $1 AND user_id = $2 AND status = 'completed'
                            ) THEN 'completed'
                            ELSE status
                        END
                    ),
                    updated_at = $3
                    WHERE id = $1 AND user_id = $2
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        self.reconcile_session_items(user_id, session_id).await
    }

    pub async fn list_sessions(&self, user_id: &str) -> anyhow::Result<Vec<PersistedSession>> {
        let rows = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, title, status, created_at, updated_at
                    FROM sessions
                    WHERE user_id = ?1
                    ORDER BY updated_at DESC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, title, status, created_at, updated_at
                    FROM sessions
                    WHERE user_id = $1
                    ORDER BY updated_at DESC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list sessions")?;

        Ok(rows
            .into_iter()
            .map(
                |(id, user_id, title, status, created_at, updated_at)| PersistedSession {
                    id,
                    user_id,
                    title,
                    status,
                    created_at: created_at.to_string(),
                    updated_at: updated_at.to_string(),
                },
            )
            .collect())
    }

    pub async fn get_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Option<PersistedSession>> {
        self.get_session_with_clause(
            session_id,
            Some(ScopedSessionFilter {
                user_id,
            }),
        )
        .await
    }

    pub async fn get_session_unscoped(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<PersistedSession>> {
        self.get_session_with_clause(session_id, None).await
    }

    async fn get_session_with_clause(
        &self,
        session_id: &str,
        filter: Option<ScopedSessionFilter<'_>>,
    ) -> anyhow::Result<Option<PersistedSession>> {
        let row = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, title, status, created_at, updated_at
                    FROM sessions
                    WHERE id = ?1
                      AND (?2 IS NULL OR user_id = ?2)
                    "#,
                )
                .bind(session_id)
                .bind(filter.as_ref().map(|filter| filter.user_id))
                .fetch_optional(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, title, status, created_at, updated_at
                    FROM sessions
                    WHERE id = $1
                      AND ($2::TEXT IS NULL OR user_id = $2)
                    "#,
                )
                .bind(session_id)
                .bind(filter.as_ref().map(|filter| filter.user_id))
                .fetch_optional(pool)
                .await
            }
        }
        .context("failed to get session")?;

        Ok(row.map(
            |(id, user_id, title, status, created_at, updated_at)| PersistedSession {
                id,
                user_id,
                title,
                status,
                created_at: created_at.to_string(),
                updated_at: updated_at.to_string(),
            },
        ))
    }

    pub async fn delete_session(&self, user_id: &str, session_id: &str) -> anyhow::Result<bool> {
        let rows_affected = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    DELETE FROM sessions
                    WHERE id = ?1 AND user_id = ?2
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .execute(pool)
                .await
                .context("failed to delete session")?
                .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    DELETE FROM sessions
                    WHERE id = $1 AND user_id = $2
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .execute(pool)
                .await
                .context("failed to delete session")?
                .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }

    pub async fn list_session_messages(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<PersistedSessionMessage>> {
        self.reconcile_session_items(user_id, session_id).await?;

        let rows = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, String, String, String, String, i64, i64, String, Option<String>)>(
                    r#"
                    SELECT id, user_id, session_id, turn_id, role, status, created_at, updated_at, content_json, tool_call_id
                    FROM messages
                    WHERE user_id = ?1 AND session_id = ?2
                    ORDER BY created_at ASC, id ASC
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, String, String, String, String, i64, i64, String, Option<String>)>(
                    r#"
                    SELECT id, user_id, session_id, turn_id, role, status, created_at, updated_at, content_json, tool_call_id
                    FROM messages
                    WHERE user_id = $1 AND session_id = $2
                    ORDER BY created_at ASC, id ASC
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list session messages")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    _user_id,
                    session_id,
                    turn_id,
                    role,
                    status,
                    created_at,
                    updated_at,
                    content_json,
                    tool_call_id,
                ): (
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    i64,
                    i64,
                    String,
                    Option<String>,
                )| {
                    let content = serde_json::from_str::<Value>(&content_json)
                        .unwrap_or_else(|_| Value::Array(Vec::new()));
                    PersistedSessionMessage {
                        id,
                        session_id,
                        turn_id,
                        role,
                        status,
                        created_at: created_at.to_string(),
                        updated_at: updated_at.to_string(),
                        content,
                        tool_call_id,
                    }
                },
            )
            .collect())
    }

    pub async fn list_session_tool_calls(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Vec<PersistedSessionToolCall>> {
        let rows = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, Option<String>, Option<String>, String, Option<String>)>(
                    r#"
                    SELECT id, turn_id, parent_item_id, tool_name, tool_display_name, arguments_text, result_json, status, media_json
                    FROM tool_calls
                    WHERE user_id = ?1 AND session_id = ?2
                    ORDER BY created_at ASC, id ASC
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, Option<String>, Option<String>, String, Option<String>)>(
                    r#"
                    SELECT id, turn_id, parent_item_id, tool_name, tool_display_name, arguments_text, result_json, status, media_json
                    FROM tool_calls
                    WHERE user_id = $1 AND session_id = $2
                    ORDER BY created_at ASC, id ASC
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list session tool calls")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    turn_id,
                    parent_item_id,
                    tool_name,
                    tool_display_name,
                    arguments_text,
                    result_json,
                    status,
                    media_json,
                )| PersistedSessionToolCall {
                    id,
                    turn_id,
                    parent_item_id,
                    tool_name,
                    tool_display_name,
                    arguments_text,
                    result_json,
                    status,
                    media_json,
                },
            )
            .collect())
    }
}

struct ScopedSessionFilter<'a> {
    user_id: &'a str,
}

