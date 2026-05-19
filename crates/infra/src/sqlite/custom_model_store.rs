use std::sync::Arc;

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::Row;

use super::db::DatabasePool;

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
}

#[derive(Clone)]
pub struct PersistedCustomModel {
    pub model_config_id: String,
    pub model_name: String,
    pub model_type: String,
    pub base_url: String,
    pub api_key: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct CustomModelCreate {
    pub user_id: String,
    pub model_config_id: String,
    pub model_name: String,
    pub model_type: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Clone)]
pub struct SqliteCustomModelStore {
    pool: Arc<DatabasePool>,
    cipher_key: Arc<[u8; 32]>,
}

impl SqliteCustomModelStore {
    pub fn new(pool: Arc<DatabasePool>, secret: &str) -> Self {
        Self {
            pool,
            cipher_key: Arc::new(derive_cipher_key(secret)),
        }
    }

    pub async fn list_user_models(
        &self,
        user_id: &str,
    ) -> anyhow::Result<Vec<PersistedCustomModel>> {
        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at
                    FROM user_custom_models
                    WHERE user_id = ?1
                    ORDER BY model_name ASC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("failed to list custom models")?;

                Ok(rows
                    .into_iter()
                    .map(|row| {
                        let encrypted = row.get::<String, _>("api_key_ciphertext");
                        PersistedCustomModel {
                            model_config_id: row.get("model_config_id"),
                            model_name: row.get("model_name"),
                            model_type: row.get("model_type"),
                            base_url: row.get("base_url"),
                            api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())
                                .unwrap_or_default(),
                            created_at: row.get::<i64, _>("created_at").to_string(),
                            updated_at: row.get::<i64, _>("updated_at").to_string(),
                        }
                    })
                    .collect())
            }
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT model_config_id, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at
                    FROM user_custom_models
                    WHERE user_id = $1
                    ORDER BY model_name ASC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("failed to list custom models")?;

                Ok(rows
                    .into_iter()
                    .map(|row| {
                        let encrypted = row.get::<String, _>("api_key_ciphertext");
                        PersistedCustomModel {
                            model_config_id: row.get("model_config_id"),
                            model_name: row.get("model_name"),
                            model_type: row.get("model_type"),
                            base_url: row.get("base_url"),
                            api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())
                                .unwrap_or_default(),
                            created_at: row.get::<i64, _>("created_at").to_string(),
                            updated_at: row.get::<i64, _>("updated_at").to_string(),
                        }
                    })
                    .collect())
            }
        }
    }

    pub async fn find_user_model(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> anyhow::Result<Option<PersistedCustomModel>> {
        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT model_config_id, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at
                    FROM user_custom_models
                    WHERE user_id = ?1 AND model_config_id = ?2
                    "#,
                )
                .bind(user_id)
                .bind(model_config_id)
                .fetch_optional(pool)
                .await
                .context("failed to read custom model")?;

                row.map(|row| {
                    let encrypted = row.get::<String, _>("api_key_ciphertext");
                    Ok(PersistedCustomModel {
                        model_config_id: row.get("model_config_id"),
                        model_name: row.get("model_name"),
                        model_type: row.get("model_type"),
                        base_url: row.get("base_url"),
                        api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                        created_at: row.get::<i64, _>("created_at").to_string(),
                        updated_at: row.get::<i64, _>("updated_at").to_string(),
                    })
                })
                .transpose()
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT model_config_id, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at
                    FROM user_custom_models
                    WHERE user_id = $1 AND model_config_id = $2
                    "#,
                )
                .bind(user_id)
                .bind(model_config_id)
                .fetch_optional(pool)
                .await
                .context("failed to read custom model")?;

                row.map(|row| {
                    let encrypted = row.get::<String, _>("api_key_ciphertext");
                    Ok(PersistedCustomModel {
                        model_config_id: row.get("model_config_id"),
                        model_name: row.get("model_name"),
                        model_type: row.get("model_type"),
                        base_url: row.get("base_url"),
                        api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                        created_at: row.get::<i64, _>("created_at").to_string(),
                        updated_at: row.get::<i64, _>("updated_at").to_string(),
                    })
                })
                .transpose()
            }
        }
    }

    pub async fn create_user_model(
        &self,
        create: CustomModelCreate,
    ) -> anyhow::Result<PersistedCustomModel> {
        let now = now_millis_i64();
        let ciphertext = encrypt_secret(self.cipher_key.as_ref(), create.api_key.trim())?;

        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO user_custom_models
                    (user_id, model_config_id, provider_key, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                )
                .bind(create.user_id.as_str())
                .bind(create.model_config_id.as_str())
                .bind("openai")
                .bind(create.model_name.trim())
                .bind(create.model_type.as_str())
                .bind(create.base_url.trim())
                .bind(&ciphertext)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await
                .context("failed to create custom model")?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO user_custom_models
                    (user_id, model_config_id, provider_key, model_name, model_type, base_url, api_key_ciphertext, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(create.user_id.as_str())
                .bind(create.model_config_id.as_str())
                .bind("openai")
                .bind(create.model_name.trim())
                .bind(create.model_type.as_str())
                .bind(create.base_url.trim())
                .bind(&ciphertext)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await
                .context("failed to create custom model")?;
            }
        }

        Ok(PersistedCustomModel {
            model_config_id: create.model_config_id,
            model_name: create.model_name.trim().to_string(),
            model_type: create.model_type,
            base_url: create.base_url.trim().to_string(),
            api_key: create.api_key.trim().to_string(),
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
    }

    pub async fn delete_user_model(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> anyhow::Result<bool> {
        let rows_affected = match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    DELETE FROM user_custom_models
                    WHERE user_id = ?1 AND model_config_id = ?2
                    "#,
                )
                .bind(user_id)
                .bind(model_config_id)
                .execute(pool)
                .await
                .context("failed to delete custom model")?
                .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    DELETE FROM user_custom_models
                    WHERE user_id = $1 AND model_config_id = $2
                    "#,
                )
                .bind(user_id)
                .bind(model_config_id)
                .execute(pool)
                .await
                .context("failed to delete custom model")?
                .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }
}

fn derive_cipher_key(secret: &str) -> [u8; 32] {
    let digest = Sha256::digest(secret.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn encrypt_secret(key_bytes: &[u8; 32], plaintext: &str) -> anyhow::Result<String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let mut nonce = [0_u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|_| anyhow!("failed to encrypt custom model secret"))?;
    Ok(format!(
        "{}.{}",
        BASE64.encode(nonce),
        BASE64.encode(ciphertext)
    ))
}

fn decrypt_secret(key_bytes: &[u8; 32], encoded: &str) -> anyhow::Result<String> {
    if encoded.trim().is_empty() {
        return Ok(String::new());
    }
    let (nonce_part, ciphertext_part) = encoded
        .split_once('.')
        .ok_or_else(|| anyhow!("custom model secret payload is malformed"))?;
    let nonce = BASE64
        .decode(nonce_part)
        .context("failed to decode custom model nonce")?;
    let ciphertext = BASE64
        .decode(ciphertext_part)
        .context("failed to decode custom model ciphertext")?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow!("failed to decrypt custom model secret"))?;
    String::from_utf8(plaintext).context("custom model secret is not valid utf-8")
}
