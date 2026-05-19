use std::path::Path;

use anyhow::Context;
use sqlx::{
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    PgPool, Row, SqlitePool,
};

#[derive(Clone)]
pub enum DatabasePool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
pub struct Database {
    pool: DatabasePool,
}

impl Database {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        if is_postgres_url(database_url) {
            let pool = PgPoolOptions::new()
                .max_connections(20)
                .connect(database_url)
                .await
                .context("failed to connect to postgres")?;

            let database = Self {
                pool: DatabasePool::Postgres(pool),
            };
            database.migrate().await?;
            return Ok(database);
        }

        ensure_parent_dir(database_url)?;

        let options = database_url
            .parse::<SqliteConnectOptions>()
            .context("failed to parse sqlite connection string")?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("failed to connect to sqlite")?;

        let database = Self {
            pool: DatabasePool::Sqlite(pool),
        };
        database.migrate().await?;
        Ok(database)
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                for statement in SQLITE_MIGRATIONS {
                    sqlx::query(statement).execute(pool).await?;
                }
                ensure_sqlite_column(
                    pool,
                    "catalog_models",
                    "runtime_provider",
                    "TEXT NOT NULL DEFAULT 'openai_compatible'",
                )
                .await?;
                ensure_sqlite_column(
                    pool,
                    "catalog_models",
                    "input_modalities",
                    "TEXT NOT NULL DEFAULT '[\"text\"]'",
                )
                .await?;
                ensure_sqlite_column(
                    pool,
                    "catalog_tools",
                    "runtime_provider",
                    "TEXT NOT NULL DEFAULT 'openai_compatible'",
                )
                .await?;
                ensure_sqlite_column(pool, "catalog_tools", "model", "TEXT NOT NULL DEFAULT ''")
                    .await?;
                ensure_sqlite_column(pool, "tool_calls", "parent_item_id", "TEXT").await?;
                ensure_sqlite_column(pool, "tool_calls", "media_json", "TEXT").await?;
                ensure_sqlite_column(pool, "turns", "terminal_reason_code", "TEXT").await?;
                ensure_sqlite_column(pool, "turns", "terminal_reason_message", "TEXT").await?;
                ensure_sqlite_column(
                    pool,
                    "user_custom_models",
                    "provider_key",
                    "TEXT NOT NULL DEFAULT 'openai'",
                )
                .await?;
                ensure_sqlite_column(
                    pool,
                    "user_custom_models",
                    "model_type",
                    "TEXT NOT NULL DEFAULT 'multimodal'",
                )
                .await?;
                ensure_sqlite_column(
                    pool,
                    "user_custom_models",
                    "base_url",
                    "TEXT NOT NULL DEFAULT ''",
                )
                .await?;
                ensure_sqlite_column(
                    pool,
                    "user_custom_models",
                    "api_key_ciphertext",
                    "TEXT NOT NULL DEFAULT ''",
                )
                .await?;
                normalize_user_custom_models_table(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
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
                ensure_postgres_column(pool, "catalog_tools", "model", "TEXT NOT NULL DEFAULT ''")
                    .await?;
                ensure_postgres_column(pool, "tool_calls", "parent_item_id", "TEXT").await?;
                ensure_postgres_column(pool, "tool_calls", "media_json", "TEXT").await?;
                ensure_postgres_column(pool, "turns", "terminal_reason_code", "TEXT").await?;
                ensure_postgres_column(pool, "turns", "terminal_reason_message", "TEXT").await?;
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
            }
        }

        Ok(())
    }
}

fn is_postgres_url(database_url: &str) -> bool {
    database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")
}

async fn ensure_sqlite_column(
    pool: &SqlitePool,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table_name})"))
        .fetch_all(pool)
        .await
        .with_context(|| format!("failed to inspect schema for table {table_name}"))?;

    let has_column = rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column_name);
    if has_column {
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

async fn normalize_user_custom_models_table(pool: &SqlitePool) -> anyhow::Result<()> {
    let table_names = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('user_custom_models', 'user_custom_models__new')",
    )
    .fetch_all(pool)
    .await
    .context("failed to inspect user_custom_models table names")?;

    let has_primary_table = table_names
        .iter()
        .any(|row| row.get::<String, _>("name") == "user_custom_models");
    let has_replacement_table = table_names
        .iter()
        .any(|row| row.get::<String, _>("name") == "user_custom_models__new");

    if !has_primary_table && has_replacement_table {
        sqlx::query("ALTER TABLE user_custom_models__new RENAME TO user_custom_models")
            .execute(pool)
            .await
            .context("failed to recover user_custom_models table from replacement table")?;
        return Ok(());
    }

    let rows = sqlx::query("PRAGMA table_info(user_custom_models)")
        .fetch_all(pool)
        .await
        .context("failed to inspect schema for table user_custom_models")?;

    if rows.is_empty() {
        return Ok(());
    }

    let provider_key_default = rows
        .iter()
        .find(|row| row.get::<String, _>("name") == "provider_key")
        .and_then(|row| row.get::<Option<String>, _>("dflt_value"));

    let should_rebuild = provider_key_default.as_deref() != Some("'openai'");
    if !should_rebuild {
        if has_replacement_table {
            sqlx::query("DROP TABLE user_custom_models__new")
                .execute(pool)
                .await
                .context("failed to drop stale replacement user_custom_models table")?;
        }
        return Ok(());
    }

    if has_replacement_table {
        sqlx::query("DROP TABLE user_custom_models__new")
            .execute(pool)
            .await
            .context("failed to clear stale replacement user_custom_models table before rebuild")?;
    }

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_custom_models__new (
          user_id TEXT NOT NULL,
          model_config_id TEXT NOT NULL,
          provider_key TEXT NOT NULL DEFAULT 'openai',
          model_name TEXT NOT NULL,
          model_type TEXT NOT NULL DEFAULT 'multimodal',
          base_url TEXT NOT NULL DEFAULT '',
          api_key_ciphertext TEXT NOT NULL DEFAULT '',
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY (user_id, model_config_id),
          FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .context("failed to create replacement user_custom_models table")?;

    sqlx::query(
        r#"
        INSERT INTO user_custom_models__new (
          user_id,
          model_config_id,
          provider_key,
          model_name,
          model_type,
          base_url,
          api_key_ciphertext,
          created_at,
          updated_at
        )
        SELECT
          user_id,
          model_config_id,
          COALESCE(NULLIF(provider_key, ''), 'openai'),
          model_name,
          model_type,
          base_url,
          api_key_ciphertext,
          created_at,
          updated_at
        FROM user_custom_models
        "#,
    )
    .execute(pool)
    .await
    .context("failed to copy user_custom_models into replacement table")?;

    sqlx::query("DROP TABLE user_custom_models")
        .execute(pool)
        .await
        .context("failed to drop legacy user_custom_models table")?;

    sqlx::query("ALTER TABLE user_custom_models__new RENAME TO user_custom_models")
        .execute(pool)
        .await
        .context("failed to rename replacement user_custom_models table")?;

    Ok(())
}

fn ensure_parent_dir(database_url: &str) -> anyhow::Result<()> {
    let Some(path) = database_url.strip_prefix("sqlite://") else {
        return Ok(());
    };

    let Some(parent) = Path::new(path).parent() else {
        return Ok(());
    };

    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create sqlite directory {}", parent.display()))?;
    Ok(())
}

const SQLITE_MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT NOT NULL UNIQUE,
      username TEXT NOT NULL,
      password_hash TEXT NOT NULL,
      is_admin INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS access_tokens (
      token TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS refresh_tokens (
      token TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
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
      official INTEGER NOT NULL,
      custom INTEGER NOT NULL
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
      PRIMARY KEY (model_config_id, id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS provider_settings (
      user_id TEXT NOT NULL,
      provider_key TEXT NOT NULL,
      base_url TEXT NOT NULL,
      api_key_ciphertext TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      PRIMARY KEY (user_id, provider_key),
      FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS user_custom_models (
      user_id TEXT NOT NULL,
      model_config_id TEXT NOT NULL,
      provider_key TEXT NOT NULL DEFAULT 'openai',
      model_name TEXT NOT NULL,
      model_type TEXT NOT NULL DEFAULT 'multimodal',
      base_url TEXT NOT NULL DEFAULT '',
      api_key_ciphertext TEXT NOT NULL DEFAULT '',
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      PRIMARY KEY (user_id, model_config_id),
      FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      title TEXT,
      status TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS turns (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL,
      prompt TEXT NOT NULL,
      text_model_config_id TEXT NOT NULL,
      image_tool_id TEXT,
      status TEXT NOT NULL,
      started_at INTEGER NOT NULL,
      completed_at INTEGER,
      terminal_reason_code TEXT,
      terminal_reason_message TEXT,
      FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS messages (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL,
      turn_id TEXT NOT NULL,
      role TEXT NOT NULL,
      status TEXT NOT NULL,
      content_json TEXT NOT NULL,
      tool_call_id TEXT,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
      FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS tool_calls (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL,
      turn_id TEXT NOT NULL,
      parent_item_id TEXT,
      tool_name TEXT NOT NULL,
      tool_display_name TEXT,
      arguments_text TEXT,
      result_json TEXT,
      status TEXT NOT NULL,
      media_json TEXT,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
      FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
    )
    "#,
];

const POSTGRES_MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT NOT NULL UNIQUE,
      username TEXT NOT NULL,
      password_hash TEXT NOT NULL,
      is_admin INTEGER NOT NULL DEFAULT 0,
      created_at BIGINT NOT NULL
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS access_tokens (
      token TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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
      official INTEGER NOT NULL,
      custom INTEGER NOT NULL
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
      PRIMARY KEY (model_config_id, id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS provider_settings (
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      provider_key TEXT NOT NULL,
      base_url TEXT NOT NULL,
      api_key_ciphertext TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1,
      created_at BIGINT NOT NULL,
      updated_at BIGINT NOT NULL,
      PRIMARY KEY (user_id, provider_key)
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
];
