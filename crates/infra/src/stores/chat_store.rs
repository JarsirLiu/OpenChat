use std::sync::Arc;

use crate::db::DatabasePool;
use anyhow::Context;
use sqlx::QueryBuilder;

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
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
    pub transcript_version: String,
    pub transcript_migration_status: String,
    pub transcript_migration_error: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct PersistedThreadItem {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub item_type: String,
    pub status: String,
    pub seq: Option<i64>,
    pub parent_id: Option<String>,
    pub content_json: Option<String>,
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub revised_prompt: Option<String>,
    pub model: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub count: Option<i64>,
    pub source_tool_call_id: Option<String>,
    pub source_tool_name: Option<String>,
    pub images_json: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PersistedThreadItemRow {
    id: String,
    user_id: String,
    session_id: String,
    turn_id: String,
    item_type: String,
    status: String,
    seq: i64,
    parent_id: Option<String>,
    content_json: Option<String>,
    text: Option<String>,
    prompt: Option<String>,
    revised_prompt: Option<String>,
    model: Option<String>,
    size: Option<String>,
    quality: Option<String>,
    count: Option<i64>,
    source_tool_call_id: Option<String>,
    source_tool_name: Option<String>,
    images_json: Option<String>,
}

pub struct PersistedTurnPage {
    pub turn_ids: Vec<String>,
    pub has_more: bool,
    pub next_before_turn_id: Option<String>,
}

#[derive(Clone)]
pub struct PersistedTurnRef {
    pub id: String,
    pub session_id: String,
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
                    INSERT INTO sessions (
                      id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
                    )
                    VALUES (?1, ?2, 'v2', 'succeeded', NULL, NULL, 'idle', ?3, ?3)
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
                    INSERT INTO sessions (
                      id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
                    )
                    VALUES ($1, $2, 'v2', 'succeeded', NULL, NULL, 'idle', $3, $3)
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

    pub async fn promote_session_transcript_to_v2(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<()> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    WITH message_items AS (
                      INSERT INTO thread_items (
                        id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                        content_json, text, prompt, revised_prompt, model, size, quality, count,
                        source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                      )
                      SELECT
                        messages.id,
                        messages.user_id,
                        messages.session_id,
                        messages.turn_id,
                        CASE
                          WHEN messages.role = 'user' THEN 'userMessage'
                          WHEN messages.role = 'reasoning' THEN 'reasoning'
                          ELSE 'agentMessage'
                        END,
                        messages.status,
                        CASE
                          WHEN messages.role = 'user' THEN 10
                          WHEN messages.role = 'reasoning' THEN 20
                          ELSE 30
                        END,
                        NULL,
                        messages.content_json,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        messages.created_at,
                        messages.updated_at
                      FROM messages
                      WHERE messages.user_id = ?1
                        AND messages.session_id = ?2
                      ON CONFLICT(id) DO NOTHING
                    ),
                    image_items AS (
                      INSERT INTO thread_items (
                        id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                        content_json, text, prompt, revised_prompt, model, size, quality, count,
                        source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                      )
                      SELECT
                        'image:' || tool_calls.id,
                        tool_calls.user_id,
                        tool_calls.session_id,
                        tool_calls.turn_id,
                        'imageGeneration',
                        tool_calls.status,
                        40 + ROW_NUMBER() OVER (PARTITION BY tool_calls.turn_id ORDER BY tool_calls.created_at, tool_calls.id),
                        tool_calls.parent_item_id,
                        NULL,
                        NULL,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'prompt'
                        END,
                        NULL,
                        tool_calls.tool_name,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'size'
                        END,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'quality'
                        END,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          WHEN jsonb_typeof(tool_calls.arguments_text::jsonb -> 'n') = 'number'
                            THEN (tool_calls.arguments_text::jsonb ->> 'n')::BIGINT
                          ELSE NULL
                        END,
                        tool_calls.id,
                        tool_calls.tool_name,
                        COALESCE(
                          (
                            SELECT jsonb_agg(
                              jsonb_build_object(
                                'url', media_entry->>'url',
                                'mimeType', media_entry->>'mimeType',
                                'sizeBytes',
                                  CASE
                                    WHEN jsonb_typeof(media_entry->'sizeBytes') = 'number'
                                      THEN (media_entry->>'sizeBytes')::BIGINT
                                    ELSE NULL
                                  END
                              )
                            )
                            FROM jsonb_array_elements(
                              CASE
                                WHEN tool_calls.media_json IS NULL OR trim(tool_calls.media_json) = '' THEN '[]'::jsonb
                                ELSE tool_calls.media_json::jsonb
                              END
                            ) AS media_entry
                            WHERE media_entry->>'kind' = 'image'
                              AND COALESCE(media_entry->>'url', '') <> ''
                          ),
                          '[]'::jsonb
                        )::text,
                        tool_calls.created_at,
                        tool_calls.updated_at
                      FROM tool_calls
                      WHERE tool_calls.user_id = ?1
                        AND tool_calls.session_id = ?2
                        AND EXISTS (
                          SELECT 1
                          FROM jsonb_array_elements(
                            CASE
                              WHEN tool_calls.media_json IS NULL OR trim(tool_calls.media_json) = '' THEN '[]'::jsonb
                              ELSE tool_calls.media_json::jsonb
                            END
                          ) AS media_entry
                          WHERE media_entry->>'kind' = 'image'
                            AND COALESCE(media_entry->>'url', '') <> ''
                        )
                      ON CONFLICT(id) DO NOTHING
                    )
                    UPDATE sessions
                    SET transcript_version = 'v2',
                        transcript_migration_status = 'succeeded',
                        transcript_migration_error = NULL
                    WHERE user_id = ?1 AND id = ?2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    WITH message_items AS (
                      INSERT INTO thread_items (
                        id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                        content_json, text, prompt, revised_prompt, model, size, quality, count,
                        source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                      )
                      SELECT
                        messages.id,
                        messages.user_id,
                        messages.session_id,
                        messages.turn_id,
                        CASE
                          WHEN messages.role = 'user' THEN 'userMessage'
                          WHEN messages.role = 'reasoning' THEN 'reasoning'
                          ELSE 'agentMessage'
                        END,
                        messages.status,
                        CASE
                          WHEN messages.role = 'user' THEN 10
                          WHEN messages.role = 'reasoning' THEN 20
                          ELSE 30
                        END,
                        NULL,
                        messages.content_json,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        messages.created_at,
                        messages.updated_at
                      FROM messages
                      WHERE messages.user_id = $1
                        AND messages.session_id = $2
                      ON CONFLICT(id) DO NOTHING
                    ),
                    image_items AS (
                      INSERT INTO thread_items (
                        id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                        content_json, text, prompt, revised_prompt, model, size, quality, count,
                        source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                      )
                      SELECT
                        'image:' || tool_calls.id,
                        tool_calls.user_id,
                        tool_calls.session_id,
                        tool_calls.turn_id,
                        'imageGeneration',
                        tool_calls.status,
                        40 + ROW_NUMBER() OVER (PARTITION BY tool_calls.turn_id ORDER BY tool_calls.created_at, tool_calls.id),
                        tool_calls.parent_item_id,
                        NULL,
                        NULL,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'prompt'
                        END,
                        NULL,
                        tool_calls.tool_name,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'size'
                        END,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          ELSE tool_calls.arguments_text::jsonb ->> 'quality'
                        END,
                        CASE
                          WHEN tool_calls.arguments_text IS NULL OR trim(tool_calls.arguments_text) = '' THEN NULL
                          WHEN jsonb_typeof(tool_calls.arguments_text::jsonb -> 'n') = 'number'
                            THEN (tool_calls.arguments_text::jsonb ->> 'n')::BIGINT
                          ELSE NULL
                        END,
                        tool_calls.id,
                        tool_calls.tool_name,
                        COALESCE(
                          (
                            SELECT jsonb_agg(
                              jsonb_build_object(
                                'url', media_entry->>'url',
                                'mimeType', media_entry->>'mimeType',
                                'sizeBytes',
                                  CASE
                                    WHEN jsonb_typeof(media_entry->'sizeBytes') = 'number'
                                      THEN (media_entry->>'sizeBytes')::BIGINT
                                    ELSE NULL
                                  END
                              )
                            )
                            FROM jsonb_array_elements(
                              CASE
                                WHEN tool_calls.media_json IS NULL OR trim(tool_calls.media_json) = '' THEN '[]'::jsonb
                                ELSE tool_calls.media_json::jsonb
                              END
                            ) AS media_entry
                            WHERE media_entry->>'kind' = 'image'
                              AND COALESCE(media_entry->>'url', '') <> ''
                          ),
                          '[]'::jsonb
                        )::text,
                        tool_calls.created_at,
                        tool_calls.updated_at
                      FROM tool_calls
                      WHERE tool_calls.user_id = $1
                        AND tool_calls.session_id = $2
                        AND EXISTS (
                          SELECT 1
                          FROM jsonb_array_elements(
                            CASE
                              WHEN tool_calls.media_json IS NULL OR trim(tool_calls.media_json) = '' THEN '[]'::jsonb
                              ELSE tool_calls.media_json::jsonb
                            END
                          ) AS media_entry
                          WHERE media_entry->>'kind' = 'image'
                            AND COALESCE(media_entry->>'url', '') <> ''
                        )
                      ON CONFLICT(id) DO NOTHING
                    )
                    UPDATE sessions
                    SET transcript_version = 'v2',
                        transcript_migration_status = 'succeeded',
                        transcript_migration_error = NULL
                    WHERE user_id = $1 AND id = $2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn mark_session_transcript_migration_failed(
        &self,
        user_id: &str,
        session_id: &str,
        error: &str,
    ) -> anyhow::Result<()> {
        let now = now_millis_i64();
        let normalized_error = error.trim();
        let bounded_error = if normalized_error.len() > 2000 {
            &normalized_error[..2000]
        } else {
            normalized_error
        };

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    UPDATE sessions
                    SET transcript_migration_status = 'failed',
                        transcript_migration_error = ?3,
                        updated_at = ?4
                    WHERE user_id = ?1 AND id = ?2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .bind(bounded_error)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    UPDATE sessions
                    SET transcript_migration_status = 'failed',
                        transcript_migration_error = $3,
                        updated_at = $4
                    WHERE user_id = $1 AND id = $2
                    "#,
                )
                .bind(user_id)
                .bind(session_id)
                .bind(bounded_error)
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

                sqlx::query(
                    "UPDATE sessions SET status = 'running', updated_at = ?2 WHERE id = ?1",
                )
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

                sqlx::query(
                    "UPDATE sessions SET status = 'running', updated_at = $2 WHERE id = $1",
                )
                .bind(session_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_thread_item(&self, item: PersistedThreadItem) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO thread_items (
                      id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                      content_json, text, prompt, revised_prompt, model, size, quality, count,
                      source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                    )
                    VALUES (
                      ?1, ?2, ?3, ?4, ?5, ?6,
                      COALESCE(?7, (SELECT COALESCE(MAX(seq) + 1, 0) FROM thread_items WHERE turn_id = ?4)),
                      ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?20
                    )
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = excluded.user_id,
                      status = excluded.status,
                      parent_id = excluded.parent_id,
                      content_json = excluded.content_json,
                      text = excluded.text,
                      prompt = excluded.prompt,
                      revised_prompt = excluded.revised_prompt,
                      model = excluded.model,
                      size = excluded.size,
                      quality = excluded.quality,
                      count = excluded.count,
                      source_tool_call_id = excluded.source_tool_call_id,
                      source_tool_name = excluded.source_tool_name,
                      images_json = excluded.images_json,
                      updated_at = excluded.updated_at
                    "#,
                )
                .bind(item.id)
                .bind(item.user_id)
                .bind(item.session_id)
                .bind(item.turn_id)
                .bind(item.item_type)
                .bind(item.status)
                .bind(item.seq)
                .bind(item.parent_id)
                .bind(item.content_json)
                .bind(item.text)
                .bind(item.prompt)
                .bind(item.revised_prompt)
                .bind(item.model)
                .bind(item.size)
                .bind(item.quality)
                .bind(item.count)
                .bind(item.source_tool_call_id)
                .bind(item.source_tool_name)
                .bind(item.images_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO thread_items (
                      id, user_id, session_id, turn_id, item_type, status, seq, parent_id,
                      content_json, text, prompt, revised_prompt, model, size, quality, count,
                      source_tool_call_id, source_tool_name, images_json, created_at, updated_at
                    )
                    VALUES (
                      $1, $2, $3, $4, $5, $6,
                      COALESCE($7, (SELECT COALESCE(MAX(seq) + 1, 0) FROM thread_items WHERE turn_id = $4)),
                      $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $20
                    )
                    ON CONFLICT(id) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      status = EXCLUDED.status,
                      parent_id = EXCLUDED.parent_id,
                      content_json = EXCLUDED.content_json,
                      text = EXCLUDED.text,
                      prompt = EXCLUDED.prompt,
                      revised_prompt = EXCLUDED.revised_prompt,
                      model = EXCLUDED.model,
                      size = EXCLUDED.size,
                      quality = EXCLUDED.quality,
                      count = EXCLUDED.count,
                      source_tool_call_id = EXCLUDED.source_tool_call_id,
                      source_tool_name = EXCLUDED.source_tool_name,
                      images_json = EXCLUDED.images_json,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(item.id)
                .bind(item.user_id)
                .bind(item.session_id)
                .bind(item.turn_id)
                .bind(item.item_type)
                .bind(item.status)
                .bind(item.seq)
                .bind(item.parent_id)
                .bind(item.content_json)
                .bind(item.text)
                .bind(item.prompt)
                .bind(item.revised_prompt)
                .bind(item.model)
                .bind(item.size)
                .bind(item.quality)
                .bind(item.count)
                .bind(item.source_tool_call_id)
                .bind(item.source_tool_name)
                .bind(item.images_json)
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

                sqlx::query(
                    "UPDATE thread_items SET status = ?2, updated_at = ?3 WHERE turn_id = ?1 AND status = 'in_progress'",
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

                sqlx::query(
                    "UPDATE thread_items SET status = $2, updated_at = $3 WHERE turn_id = $1 AND status = 'in_progress'",
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

    pub async fn reconcile_session_items(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> anyhow::Result<()> {
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

                sqlx::query(
                    r#"
                    UPDATE thread_items
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = thread_items.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = thread_items.turn_id
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

                sqlx::query(
                    r#"
                    UPDATE thread_items
                    SET status = (
                        CASE
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = thread_items.turn_id
                            ) = 'completed' THEN 'completed'
                            WHEN (
                                SELECT status FROM turns
                                WHERE turns.id = thread_items.turn_id
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
                sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
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
                sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
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
                |(
                    id,
                    user_id,
                    transcript_version,
                    transcript_migration_status,
                    transcript_migration_error,
                    title,
                    status,
                    created_at,
                    updated_at,
                )| PersistedSession {
                    id,
                    user_id,
                    transcript_version,
                    transcript_migration_status,
                    transcript_migration_error,
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
        self.get_session_with_clause(session_id, Some(ScopedSessionFilter { user_id }))
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
                sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
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
                sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, i64, i64)>(
                    r#"
                    SELECT id, user_id, transcript_version, transcript_migration_status, transcript_migration_error, title, status, created_at, updated_at
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
            |(
                id,
                user_id,
                transcript_version,
                transcript_migration_status,
                transcript_migration_error,
                title,
                status,
                created_at,
                updated_at,
            )| PersistedSession {
                id,
                user_id,
                transcript_version,
                transcript_migration_status,
                transcript_migration_error,
                title,
                status,
                created_at: created_at.to_string(),
                updated_at: updated_at.to_string(),
            },
        ))
    }

    pub async fn delete_session(&self, user_id: &str, session_id: &str) -> anyhow::Result<bool> {
        let rows_affected = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => sqlx::query(
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
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
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
            .rows_affected(),
        };

        Ok(rows_affected > 0)
    }

    pub async fn list_stale_sessions_for_user(
        &self,
        user_id: &str,
        keep_latest: usize,
    ) -> anyhow::Result<Vec<String>> {
        let offset = i64::try_from(keep_latest).unwrap_or(i64::MAX);
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT id
                    FROM sessions
                    WHERE user_id = ?1
                    ORDER BY updated_at DESC, created_at DESC, id DESC
                    OFFSET ?2
                    "#,
                )
                .bind(user_id)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT id
                    FROM sessions
                    WHERE user_id = $1
                    ORDER BY updated_at DESC, created_at DESC, id DESC
                    OFFSET $2
                    "#,
                )
                .bind(user_id)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list stale user sessions")
    }

    pub async fn delete_sessions_for_user(
        &self,
        user_id: &str,
        session_ids: &[String],
    ) -> anyhow::Result<()> {
        if session_ids.is_empty() {
            return Ok(());
        }

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let mut query = QueryBuilder::new("DELETE FROM sessions WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND id IN (");
                {
                    let mut separated = query.separated(", ");
                    for session_id in session_ids {
                        separated.push_bind(session_id);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut query = QueryBuilder::new("DELETE FROM sessions WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND id IN (");
                {
                    let mut separated = query.separated(", ");
                    for session_id in session_ids {
                        separated.push_bind(session_id);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;
            }
        }

        Ok(())
    }

    pub async fn list_session_turns_page(
        &self,
        user_id: &str,
        session_id: &str,
        before_turn_id: Option<&str>,
        turn_limit: usize,
    ) -> anyhow::Result<PersistedTurnPage> {
        let fetch_limit = i64::try_from(turn_limit.saturating_add(1)).unwrap_or(i64::MAX);

        let rows: Vec<(String, i64)> = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, i64)>(
                    r#"
                    SELECT id, started_at
                    FROM turns
                    WHERE session_id = ?1
                      AND user_id = ?2
                      AND (
                        ?3 IS NULL OR (
                          started_at < COALESCE(
                            (SELECT started_at FROM turns WHERE id = ?3 AND session_id = ?1 AND user_id = ?2),
                            9223372036854775807
                          )
                          OR (
                            started_at = COALESCE(
                              (SELECT started_at FROM turns WHERE id = ?3 AND session_id = ?1 AND user_id = ?2),
                              9223372036854775807
                            )
                            AND id < ?3
                          )
                        )
                      )
                    ORDER BY started_at DESC, id DESC
                    LIMIT ?4
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(before_turn_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, i64)>(
                    r#"
                    SELECT id, started_at
                    FROM turns
                    WHERE session_id = $1
                      AND user_id = $2
                      AND (
                        $3::TEXT IS NULL OR (
                          started_at < COALESCE(
                            (SELECT started_at FROM turns WHERE id = $3 AND session_id = $1 AND user_id = $2),
                            9223372036854775807
                          )
                          OR (
                            started_at = COALESCE(
                              (SELECT started_at FROM turns WHERE id = $3 AND session_id = $1 AND user_id = $2),
                              9223372036854775807
                            )
                            AND id < $3
                          )
                        )
                      )
                    ORDER BY started_at DESC, id DESC
                    LIMIT $4
                    "#,
                )
                .bind(session_id)
                .bind(user_id)
                .bind(before_turn_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list session turns page")?;

        let has_more = rows.len() > turn_limit;
        let mut kept_rows = rows;
        if has_more {
            kept_rows.truncate(turn_limit);
        }

        let next_before_turn_id = kept_rows.last().map(|(id, _)| id.clone());
        let turn_ids = kept_rows
            .into_iter()
            .rev()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        Ok(PersistedTurnPage {
            turn_ids,
            has_more,
            next_before_turn_id,
        })
    }

    pub async fn list_stale_turns_for_user(
        &self,
        user_id: &str,
        keep_latest: usize,
    ) -> anyhow::Result<Vec<PersistedTurnRef>> {
        let offset = i64::try_from(keep_latest).unwrap_or(i64::MAX);
        let rows: Vec<(String, String)> = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query_as::<_, (String, String)>(
                    r#"
                    SELECT id, session_id
                    FROM turns
                    WHERE user_id = ?1
                    ORDER BY started_at DESC, id DESC
                    OFFSET ?2
                    "#,
                )
                .bind(user_id)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_as::<_, (String, String)>(
                    r#"
                    SELECT id, session_id
                    FROM turns
                    WHERE user_id = $1
                    ORDER BY started_at DESC, id DESC
                    OFFSET $2
                    "#,
                )
                .bind(user_id)
                .bind(offset)
                .fetch_all(pool)
                .await
            }
        }
        .context("failed to list stale user turns")?;

        Ok(rows
            .into_iter()
            .map(|(id, session_id)| PersistedTurnRef { id, session_id })
            .collect())
    }

    pub async fn delete_turns_for_user(
        &self,
        user_id: &str,
        turn_ids: &[String],
    ) -> anyhow::Result<()> {
        if turn_ids.is_empty() {
            return Ok(());
        }

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let mut query = QueryBuilder::new("DELETE FROM turns WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;

                sqlx::query(
                    r#"
                    DELETE FROM sessions
                    WHERE user_id = ?1
                      AND NOT EXISTS (
                        SELECT 1 FROM turns WHERE turns.session_id = sessions.id
                      )
                    "#,
                )
                .bind(user_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut query = QueryBuilder::new("DELETE FROM turns WHERE user_id = ");
                query.push_bind(user_id);
                query.push(" AND id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query.build().execute(pool).await?;

                sqlx::query(
                    r#"
                    DELETE FROM sessions
                    WHERE user_id = $1
                      AND NOT EXISTS (
                        SELECT 1 FROM turns WHERE turns.session_id = sessions.id
                      )
                    "#,
                )
                .bind(user_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_session_thread_items_for_turns(
        &self,
        user_id: &str,
        session_id: &str,
        turn_ids: &[String],
    ) -> anyhow::Result<Vec<PersistedThreadItem>> {
        if turn_ids.is_empty() {
            return Ok(Vec::new());
        }

        let turn_order = turn_ids
            .iter()
            .enumerate()
            .map(|(index, turn_id)| (turn_id.clone(), index))
            .collect::<std::collections::HashMap<_, _>>();

        let rows = match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let mut query = QueryBuilder::<sqlx::Postgres>::new(
                    "SELECT id, user_id, session_id, turn_id, item_type, status, seq, parent_id, content_json, text, prompt, revised_prompt, model, size, quality, count, source_tool_call_id, source_tool_name, images_json FROM thread_items WHERE user_id = ",
                );
                query.push_bind(user_id);
                query.push(" AND session_id = ");
                query.push_bind(session_id);
                query.push(" AND turn_id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query
                    .build_query_as::<PersistedThreadItemRow>()
                    .fetch_all(pool)
                    .await
            }
            DatabasePool::Postgres(pool) => {
                let mut query = QueryBuilder::<sqlx::Postgres>::new(
                    "SELECT id, user_id, session_id, turn_id, item_type, status, seq, parent_id, content_json, text, prompt, revised_prompt, model, size, quality, count, source_tool_call_id, source_tool_name, images_json FROM thread_items WHERE user_id = ",
                );
                query.push_bind(user_id);
                query.push(" AND session_id = ");
                query.push_bind(session_id);
                query.push(" AND turn_id IN (");
                {
                    let mut separated = query.separated(", ");
                    for turn_id in turn_ids {
                        separated.push_bind(turn_id);
                    }
                }
                query.push(")");
                query
                    .build_query_as::<PersistedThreadItemRow>()
                    .fetch_all(pool)
                    .await
            }
        }
        .context("failed to list session thread items for turns")?;

        let mut items = rows
            .into_iter()
            .map(|row| PersistedThreadItem {
                id: row.id,
                user_id: row.user_id,
                session_id: row.session_id,
                turn_id: row.turn_id,
                item_type: row.item_type,
                status: row.status,
                seq: Some(row.seq),
                parent_id: row.parent_id,
                content_json: row.content_json,
                text: row.text,
                prompt: row.prompt,
                revised_prompt: row.revised_prompt,
                model: row.model,
                size: row.size,
                quality: row.quality,
                count: row.count,
                source_tool_call_id: row.source_tool_call_id,
                source_tool_name: row.source_tool_name,
                images_json: row.images_json,
            })
            .collect::<Vec<_>>();

        items.sort_by(|left, right| {
            let left_order = turn_order.get(&left.turn_id).copied().unwrap_or(usize::MAX);
            let right_order = turn_order
                .get(&right.turn_id)
                .copied()
                .unwrap_or(usize::MAX);
            left_order
                .cmp(&right_order)
                .then_with(|| left.seq.cmp(&right.seq))
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(items)
    }
}

struct ScopedSessionFilter<'a> {
    user_id: &'a str,
}
