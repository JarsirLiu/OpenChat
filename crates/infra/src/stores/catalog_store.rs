use std::{fs, path::Path, sync::Arc};

use anyhow::Context;
use serde::Deserialize;
use sqlx::Row;

use crate::db::DatabasePool;

fn default_runtime_provider() -> String {
    "openai_compatible".to_string()
}

fn default_input_modalities() -> Vec<String> {
    vec!["text".to_string()]
}

#[derive(Clone, Deserialize)]
pub struct CatalogConfig {
    pub providers: Vec<CatalogProvider>,
    pub models: Vec<CatalogModelRecord>,
    pub tools: Vec<CatalogToolRecord>,
}

#[derive(Clone, Deserialize)]
pub struct CatalogProvider {
    pub key: String,
    pub label: String,
}

#[derive(Clone, Deserialize)]
pub struct CatalogModelRecord {
    pub model_config_id: String,
    pub provider: String,
    #[serde(default = "default_runtime_provider")]
    pub runtime_provider: String,
    pub display_provider: String,
    pub model: String,
    pub display_name: String,
    pub source: String,
    #[serde(rename = "type")]
    pub model_type: String,
    #[serde(default = "default_input_modalities")]
    pub input_modalities: Vec<String>,
    pub official: bool,
    pub custom: bool,
}

#[derive(Clone, Deserialize)]
pub struct CatalogToolRecord {
    pub model_config_id: String,
    pub model: String,
    pub id: String,
    pub provider: String,
    #[serde(default = "default_runtime_provider")]
    pub runtime_provider: String,
    pub display_provider: String,
    pub source: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub display_name: String,
}

#[derive(Clone)]
pub struct CatalogStore {
    pool: Arc<DatabasePool>,
}

impl CatalogStore {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    pub async fn sync_from_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let raw = fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "failed to read model catalog from {}",
                path.as_ref().display()
            )
        })?;
        let config: CatalogConfig =
            serde_json::from_str(&raw).context("failed to parse model catalog json")?;

        let _providers_count = config.providers.len();
        let _provider_keys: Vec<(&str, &str)> = config
            .providers
            .iter()
            .map(|entry| (entry.key.as_str(), entry.label.as_str()))
            .collect();

        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query("DELETE FROM catalog_models").execute(pool).await?;
                sqlx::query("DELETE FROM catalog_tools").execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM catalog_models").execute(pool).await?;
                sqlx::query("DELETE FROM catalog_tools").execute(pool).await?;
            }
        }

        for model in config.models {
            let input_modalities = serde_json::to_string(&model.input_modalities)?;
            match self.pool.as_ref() {
                DatabasePool::Compat(pool) => {
                    sqlx::query(
                        r#"
                        INSERT INTO catalog_models
                        (model_config_id, provider, runtime_provider, display_provider, model, display_name, source, model_type, input_modalities, official, custom)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                        "#,
                    )
                    .bind(model.model_config_id.clone())
                    .bind(model.provider.clone())
                    .bind(model.runtime_provider.clone())
                    .bind(model.display_provider.clone())
                    .bind(model.model.clone())
                    .bind(model.display_name.clone())
                    .bind(model.source.clone())
                    .bind(model.model_type.clone())
                    .bind(&input_modalities)
                    .bind(if model.official { 1_i64 } else { 0_i64 })
                    .bind(if model.custom { 1_i64 } else { 0_i64 })
                    .execute(pool)
                    .await?;
                }
                DatabasePool::Postgres(pool) => {
                    sqlx::query(
                        r#"
                        INSERT INTO catalog_models
                        (model_config_id, provider, runtime_provider, display_provider, model, display_name, source, model_type, input_modalities, official, custom)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                        "#,
                    )
                    .bind(model.model_config_id)
                    .bind(model.provider)
                    .bind(model.runtime_provider)
                    .bind(model.display_provider)
                    .bind(model.model)
                    .bind(model.display_name)
                    .bind(model.source)
                    .bind(model.model_type)
                    .bind(input_modalities)
                    .bind(model.official)
                    .bind(model.custom)
                    .execute(pool)
                    .await?;
                }
            }
        }

        for tool in config.tools {
            match self.pool.as_ref() {
                DatabasePool::Compat(pool) => {
                    sqlx::query(
                        r#"
                        INSERT INTO catalog_tools
                        (model_config_id, model, id, provider, runtime_provider, display_provider, source, tool_type, display_name)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                        "#,
                    )
                    .bind(tool.model_config_id.clone())
                    .bind(tool.model.clone())
                    .bind(tool.id.clone())
                    .bind(tool.provider.clone())
                    .bind(tool.runtime_provider.clone())
                    .bind(tool.display_provider.clone())
                    .bind(tool.source.clone())
                    .bind(tool.tool_type.clone())
                    .bind(tool.display_name.clone())
                    .execute(pool)
                    .await?;
                }
                DatabasePool::Postgres(pool) => {
                    sqlx::query(
                        r#"
                        INSERT INTO catalog_tools
                        (model_config_id, model, id, provider, runtime_provider, display_provider, source, tool_type, display_name)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        "#,
                    )
                    .bind(tool.model_config_id)
                    .bind(tool.model)
                    .bind(tool.id)
                    .bind(tool.provider)
                    .bind(tool.runtime_provider)
                    .bind(tool.display_provider)
                    .bind(tool.source)
                    .bind(tool.tool_type)
                    .bind(tool.display_name)
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<CatalogModelRecord>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, provider, runtime_provider, display_provider, model, display_name, source, model_type, input_modalities, official, custom
                    FROM catalog_models
                    ORDER BY display_provider, display_name
                    "#,
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| CatalogModelRecord {
                        model_config_id: row.get("model_config_id"),
                        provider: row.get("provider"),
                        runtime_provider: row.get("runtime_provider"),
                        display_provider: row.get("display_provider"),
                        model: row.get("model"),
                        display_name: row.get("display_name"),
                        source: row.get("source"),
                        model_type: row.get("model_type"),
                        input_modalities: serde_json::from_str(
                            &row.get::<String, _>("input_modalities"),
                        )
                        .unwrap_or_else(|_| vec!["text".to_string()]),
                        official: row.get::<i64, _>("official") != 0,
                        custom: row.get::<i64, _>("custom") != 0,
                    })
                    .collect())
            }
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, provider, runtime_provider, display_provider, model, display_name, source, model_type, input_modalities, official, custom
                    FROM catalog_models
                    ORDER BY display_provider, display_name
                    "#,
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| CatalogModelRecord {
                        model_config_id: row.get("model_config_id"),
                        provider: row.get("provider"),
                        runtime_provider: row.get("runtime_provider"),
                        display_provider: row.get("display_provider"),
                        model: row.get("model"),
                        display_name: row.get("display_name"),
                        source: row.get("source"),
                        model_type: row.get("model_type"),
                        input_modalities: serde_json::from_str(
                            &row.get::<String, _>("input_modalities"),
                        )
                        .unwrap_or_else(|_| vec!["text".to_string()]),
                        official: row.get::<bool, _>("official"),
                        custom: row.get::<bool, _>("custom"),
                    })
                    .collect())
            }
        }
    }

    pub async fn list_tools(&self) -> anyhow::Result<Vec<CatalogToolRecord>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, model, id, provider, runtime_provider, display_provider, source, tool_type, display_name
                    FROM catalog_tools
                    ORDER BY display_provider, display_name
                    "#,
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| CatalogToolRecord {
                        model_config_id: row.get("model_config_id"),
                        model: row.get("model"),
                        id: row.get("id"),
                        provider: row.get("provider"),
                        runtime_provider: row.get("runtime_provider"),
                        display_provider: row.get("display_provider"),
                        source: row.get("source"),
                        tool_type: row.get("tool_type"),
                        display_name: row.get("display_name"),
                    })
                    .collect())
            }
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, model, id, provider, runtime_provider, display_provider, source, tool_type, display_name
                    FROM catalog_tools
                    ORDER BY display_provider, display_name
                    "#,
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| CatalogToolRecord {
                        model_config_id: row.get("model_config_id"),
                        model: row.get("model"),
                        id: row.get("id"),
                        provider: row.get("provider"),
                        runtime_provider: row.get("runtime_provider"),
                        display_provider: row.get("display_provider"),
                        source: row.get("source"),
                        tool_type: row.get("tool_type"),
                        display_name: row.get("display_name"),
                    })
                    .collect())
            }
        }
    }
}

