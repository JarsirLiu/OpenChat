use anyhow::Context;
use sqlx::PgPool;

pub(crate) async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    for statement in POSTGRES_MIGRATIONS {
        sqlx::query(statement).execute(pool).await?;
    }
    ensure_postgres_column(
        pool,
        "catalog_models",
        "runtime_provider",
        "TEXT NOT NULL DEFAULT 'openai_compatible'",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "catalog_models",
        "input_modalities",
        "TEXT NOT NULL DEFAULT '[\"text\"]'",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "catalog_tools",
        "runtime_provider",
        "TEXT NOT NULL DEFAULT 'openai_compatible'",
    )
    .await?;
    ensure_postgres_column(pool, "catalog_tools", "model", "TEXT NOT NULL DEFAULT ''").await?;
    ensure_postgres_column(pool, "catalog_tools", "default_size", "TEXT").await?;
    ensure_postgres_column(pool, "catalog_tools", "default_quality", "TEXT").await?;
    ensure_postgres_column(pool, "catalog_tools", "default_n", "BIGINT").await?;
    ensure_postgres_column(pool, "tool_calls", "parent_item_id", "TEXT").await?;
    ensure_postgres_column(pool, "tool_calls", "media_json", "TEXT").await?;
    ensure_postgres_column(pool, "turns", "terminal_reason_code", "TEXT").await?;
    ensure_postgres_column(pool, "turns", "terminal_reason_message", "TEXT").await?;
    ensure_postgres_column(pool, "sessions", "user_id", "TEXT").await?;
    ensure_postgres_column(
        pool,
        "sessions",
        "transcript_version",
        "TEXT NOT NULL DEFAULT 'legacy'",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "sessions",
        "transcript_migration_status",
        "TEXT NOT NULL DEFAULT 'pending'",
    )
    .await?;
    ensure_postgres_column(pool, "sessions", "transcript_migration_error", "TEXT").await?;
    ensure_postgres_column(pool, "turns", "user_id", "TEXT").await?;
    ensure_postgres_column(pool, "messages", "user_id", "TEXT").await?;
    ensure_postgres_column(pool, "tool_calls", "user_id", "TEXT").await?;
    sqlx::query(
        "UPDATE turns SET user_id = sessions.user_id FROM sessions WHERE sessions.id = turns.session_id AND turns.user_id IS NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE messages SET user_id = sessions.user_id FROM sessions WHERE sessions.id = messages.session_id AND messages.user_id IS NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE tool_calls SET user_id = sessions.user_id FROM sessions WHERE sessions.id = tool_calls.session_id AND tool_calls.user_id IS NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sessions_user_updated_at ON sessions(user_id, updated_at DESC)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_turns_user_session ON turns(user_id, session_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_user_session ON messages(user_id, session_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tool_calls_user_session ON tool_calls(user_id, session_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS thread_items (
          id TEXT PRIMARY KEY,
          user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
          session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
          turn_id TEXT NOT NULL REFERENCES turns(id) ON DELETE CASCADE,
          item_type TEXT NOT NULL,
          status TEXT NOT NULL,
          seq BIGINT NOT NULL,
          parent_id TEXT,
          content_json TEXT,
          text TEXT,
          prompt TEXT,
          revised_prompt TEXT,
          model TEXT,
          size TEXT,
          quality TEXT,
          count BIGINT,
          source_tool_call_id TEXT,
          source_tool_name TEXT,
          images_json TEXT,
          created_at BIGINT NOT NULL,
          updated_at BIGINT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_thread_items_user_session_turn ON thread_items(user_id, session_id, turn_id, seq ASC, id ASC)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS media_objects (
          object_key TEXT PRIMARY KEY,
          user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
          session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
          created_at BIGINT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_media_objects_user ON media_objects(user_id)")
        .execute(pool)
        .await?;
    ensure_postgres_column(
        pool,
        "user_custom_models",
        "provider_key",
        "TEXT NOT NULL DEFAULT 'openai'",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "user_custom_models",
        "model_type",
        "TEXT NOT NULL DEFAULT 'multimodal'",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "user_custom_models",
        "base_url",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    ensure_postgres_column(
        pool,
        "user_custom_models",
        "api_key_ciphertext",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    migrate_user_provider_api_keys_table(pool).await?;
    if let Err(error) = backfill_legacy_sessions_to_thread_items(pool).await {
        eprintln!("[openchat][db] legacy session backfill failed, continuing startup: {error}");
    }
    Ok(())
}

async fn ensure_postgres_column(
    pool: &PgPool,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let row = sqlx::query(
        r#"
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2
        "#,
    )
    .bind(table_name)
    .bind(column_name)
    .fetch_optional(pool)
    .await
    .with_context(|| format!("failed to inspect schema for table {table_name}"))?;

    if row.is_some() {
        return Ok(());
    }

    sqlx::query(&format!(
        "ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}"
    ))
    .execute(pool)
    .await
    .with_context(|| format!("failed to add column {column_name} to table {table_name}"))?;

    Ok(())
}

async fn migrate_user_provider_api_keys_table(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_provider_api_keys (
          user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
          provider_key TEXT NOT NULL,
          api_key_ciphertext TEXT NOT NULL,
          created_at BIGINT NOT NULL,
          updated_at BIGINT NOT NULL,
          PRIMARY KEY (user_id, provider_key)
        )
        "#,
    )
    .execute(pool)
    .await
    .context("failed to create user_provider_api_keys table")?;

    let legacy_table_exists = sqlx::query(
        r#"
        SELECT 1
        FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'provider_settings'
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("failed to inspect legacy provider_settings table")?
    .is_some();

    if !legacy_table_exists {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO user_provider_api_keys (
          user_id,
          provider_key,
          api_key_ciphertext,
          created_at,
          updated_at
        )
        SELECT
          user_id,
          provider_key,
          api_key_ciphertext,
          created_at,
          updated_at
        FROM provider_settings
        ON CONFLICT(user_id, provider_key) DO UPDATE SET
          api_key_ciphertext = EXCLUDED.api_key_ciphertext,
          created_at = EXCLUDED.created_at,
          updated_at = EXCLUDED.updated_at
        "#,
    )
    .execute(pool)
    .await
    .context("failed to migrate provider_settings into user_provider_api_keys")?;

    sqlx::query("DROP TABLE IF EXISTS provider_settings")
        .execute(pool)
        .await
        .context("failed to drop legacy provider_settings table")?;

    Ok(())
}

async fn backfill_legacy_sessions_to_thread_items(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
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
          END AS item_type,
          messages.status,
          CASE
            WHEN messages.role = 'user' THEN 10
            WHEN messages.role = 'reasoning' THEN 20
            ELSE 30
          END AS seq,
          NULL,
          messages.content_json,
          (
            SELECT string_agg(part->>'text', E'\n\n')
            FROM jsonb_array_elements(
              CASE
                WHEN messages.content_json IS NULL OR trim(messages.content_json) = '' THEN '[]'::jsonb
                ELSE messages.content_json::jsonb
              END
            ) AS part
            WHERE part->>'type' = 'text'
          ) AS text,
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
        INNER JOIN sessions ON sessions.id = messages.session_id
        WHERE sessions.transcript_version = 'legacy'
        ON CONFLICT(id) DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .context("failed to backfill legacy message items into thread_items")?;

    sqlx::query(
        r#"
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
        INNER JOIN sessions ON sessions.id = tool_calls.session_id
        WHERE sessions.transcript_version = 'legacy'
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
        "#,
    )
    .execute(pool)
    .await
    .context("failed to backfill legacy image generation items into thread_items")?;

    sqlx::query(
        "UPDATE sessions SET transcript_version = 'v2', transcript_migration_status = 'succeeded', transcript_migration_error = NULL WHERE transcript_version = 'legacy'",
    )
    .execute(pool)
    .await
    .context("failed to promote legacy sessions to transcript_version v2")?;

    Ok(())
}

const POSTGRES_MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT NOT NULL UNIQUE,
      username TEXT NOT NULL,
      password_hash TEXT NOT NULL,
      is_admin BOOLEAN NOT NULL DEFAULT FALSE,
      created_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS refresh_tokens (
      token TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      created_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS catalog_models (
      model_config_id TEXT PRIMARY KEY,
      provider TEXT NOT NULL,
      runtime_provider TEXT NOT NULL DEFAULT 'openai_compatible',
      display_provider TEXT NOT NULL,
      model TEXT NOT NULL,
      display_name TEXT NOT NULL,
      source TEXT NOT NULL,
      model_type TEXT NOT NULL,
      input_modalities TEXT NOT NULL DEFAULT '["text"]',
      official BOOLEAN NOT NULL,
      custom BOOLEAN NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS catalog_tools (
      model_config_id TEXT NOT NULL,
      model TEXT NOT NULL,
      id TEXT NOT NULL,
      provider TEXT NOT NULL,
      runtime_provider TEXT NOT NULL DEFAULT 'openai_compatible',
      display_provider TEXT NOT NULL,
      source TEXT NOT NULL,
      tool_type TEXT NOT NULL,
      display_name TEXT NOT NULL,
      default_size TEXT,
      default_quality TEXT,
      default_n BIGINT,
      PRIMARY KEY (model_config_id, id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS user_custom_models (
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      model_config_id TEXT NOT NULL,
      provider_key TEXT NOT NULL DEFAULT 'openai',
      model_name TEXT NOT NULL,
      model_type TEXT NOT NULL DEFAULT 'multimodal',
      base_url TEXT NOT NULL DEFAULT '',
      api_key_ciphertext TEXT NOT NULL DEFAULT '',
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL,
      PRIMARY KEY (user_id, model_config_id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      transcript_version TEXT NOT NULL DEFAULT 'legacy',
      transcript_migration_status TEXT NOT NULL DEFAULT 'pending',
      transcript_migration_error TEXT,
      title TEXT,
      status TEXT NOT NULL,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS turns (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      prompt TEXT NOT NULL,
      text_model_config_id TEXT NOT NULL,
      image_tool_id TEXT,
      status TEXT NOT NULL,
      started_at BIGINT NOT NULL,
      completed_at BIGINT,
      terminal_reason_code TEXT,
      terminal_reason_message TEXT
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS messages (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
      turn_id TEXT NOT NULL REFERENCES turns(id) ON DELETE CASCADE,
      role TEXT NOT NULL,
      status TEXT NOT NULL,
      content_json TEXT NOT NULL,
      tool_call_id TEXT,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS tool_calls (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
      turn_id TEXT NOT NULL REFERENCES turns(id) ON DELETE CASCADE,
      parent_item_id TEXT,
      tool_name TEXT NOT NULL,
      tool_display_name TEXT,
      arguments_text TEXT,
      result_json TEXT,
      status TEXT NOT NULL,
      media_json TEXT,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS media_objects (
      object_key TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
      created_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS thread_items (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
      turn_id TEXT NOT NULL REFERENCES turns(id) ON DELETE CASCADE,
      item_type TEXT NOT NULL,
      status TEXT NOT NULL,
      seq BIGINT NOT NULL,
      parent_id TEXT,
      content_json TEXT,
      text TEXT,
      prompt TEXT,
      revised_prompt TEXT,
      model TEXT,
      size TEXT,
      quality TEXT,
      count BIGINT,
      source_tool_call_id TEXT,
      source_tool_name TEXT,
      images_json TEXT,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS user_provider_api_keys (
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      provider_key TEXT NOT NULL,
      api_key_ciphertext TEXT NOT NULL,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL,
      PRIMARY KEY (user_id, provider_key)
    )
    "#,
];
