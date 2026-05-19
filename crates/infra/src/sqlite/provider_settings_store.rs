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
pub struct PersistedProviderSetting {
    pub provider_key: String,
    pub base_url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub api_key: String,
}

#[derive(Clone)]
pub struct ProviderSettingUpdate {
    pub user_id: String,
    pub provider_key: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct SqliteProviderSettingsStore {
    pool: Arc<DatabasePool>,
    cipher_key: Arc<[u8; 32]>,
}

impl SqliteProviderSettingsStore {
    pub fn new(pool: Arc<DatabasePool>, secret: &str) -> Self {
        Self {
            pool,
            cipher_key: Arc::new(derive_cipher_key(secret)),
        }
    }

    pub async fn list_user_settings(
        &self,
        user_id: &str,
    ) -> anyhow::Result<Vec<PersistedProviderSetting>> {
        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT provider_key, base_url, enabled, created_at, updated_at, api_key_ciphertext
                    FROM provider_settings
                    WHERE user_id = ?1
                    ORDER BY provider_key ASC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("failed to list provider settings")?;

                rows.into_iter()
                    .map(|row| {
                        let encrypted = row.get::<String, _>("api_key_ciphertext");
                        Ok(PersistedProviderSetting {
                            provider_key: row.get("provider_key"),
                            base_url: row.get("base_url"),
                            enabled: row.get::<i32, _>("enabled") != 0,
                            created_at: row.get::<i64, _>("created_at").to_string(),
                            updated_at: row.get::<i64, _>("updated_at").to_string(),
                            api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                        })
                    })
                    .collect()
            }
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    r#"
                    SELECT provider_key, base_url, enabled, created_at, updated_at, api_key_ciphertext
                    FROM provider_settings
                    WHERE user_id = $1
                    ORDER BY provider_key ASC
                    "#,
                )
                .bind(user_id)
                .fetch_all(pool)
                .await
                .context("failed to list provider settings")?;

                rows.into_iter()
                    .map(|row| {
                        let encrypted = row.get::<String, _>("api_key_ciphertext");
                        Ok(PersistedProviderSetting {
                            provider_key: row.get("provider_key"),
                            base_url: row.get("base_url"),
                            enabled: row.get::<i32, _>("enabled") != 0,
                            created_at: row.get::<i64, _>("created_at").to_string(),
                            updated_at: row.get::<i64, _>("updated_at").to_string(),
                            api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                        })
                    })
                    .collect()
            }
        }
    }

    pub async fn find_user_setting(
        &self,
        user_id: &str,
        provider_key: &str,
    ) -> anyhow::Result<Option<PersistedProviderSetting>> {
        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT provider_key, base_url, enabled, created_at, updated_at, api_key_ciphertext
                    FROM provider_settings
                    WHERE user_id = ?1 AND provider_key = ?2
                    "#,
                )
                .bind(user_id)
                .bind(provider_key)
                .fetch_optional(pool)
                .await
                .context("failed to read provider setting")?;

                row.map(|row| {
                    let encrypted = row.get::<String, _>("api_key_ciphertext");
                    Ok(PersistedProviderSetting {
                        provider_key: row.get("provider_key"),
                        base_url: row.get("base_url"),
                        enabled: row.get::<i32, _>("enabled") != 0,
                        created_at: row.get::<i64, _>("created_at").to_string(),
                        updated_at: row.get::<i64, _>("updated_at").to_string(),
                        api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                    })
                })
                .transpose()
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT provider_key, base_url, enabled, created_at, updated_at, api_key_ciphertext
                    FROM provider_settings
                    WHERE user_id = $1 AND provider_key = $2
                    "#,
                )
                .bind(user_id)
                .bind(provider_key)
                .fetch_optional(pool)
                .await
                .context("failed to read provider setting")?;

                row.map(|row| {
                    let encrypted = row.get::<String, _>("api_key_ciphertext");
                    Ok(PersistedProviderSetting {
                        provider_key: row.get("provider_key"),
                        base_url: row.get("base_url"),
                        enabled: row.get::<i32, _>("enabled") != 0,
                        created_at: row.get::<i64, _>("created_at").to_string(),
                        updated_at: row.get::<i64, _>("updated_at").to_string(),
                        api_key: decrypt_secret(self.cipher_key.as_ref(), encrypted.as_str())?,
                    })
                })
                .transpose()
            }
        }
    }

    pub async fn upsert_user_setting(
        &self,
        update: ProviderSettingUpdate,
    ) -> anyhow::Result<PersistedProviderSetting> {
        let existing = self
            .find_user_setting(update.user_id.as_str(), update.provider_key.as_str())
            .await?;
        let api_key = update
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| existing.as_ref().map(|value| value.api_key.clone()))
            .ok_or_else(|| anyhow!("An API key is required for this provider"))?;

        let now = now_millis_i64();
        let created_at = existing
            .as_ref()
            .and_then(|value| value.created_at.parse::<i64>().ok())
            .unwrap_or(now);
        let ciphertext = encrypt_secret(self.cipher_key.as_ref(), api_key.as_str())?;

        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO provider_settings
                    (user_id, provider_key, base_url, api_key_ciphertext, enabled, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(user_id, provider_key) DO UPDATE SET
                      base_url = excluded.base_url,
                      api_key_ciphertext = excluded.api_key_ciphertext,
                      enabled = excluded.enabled,
                      updated_at = excluded.updated_at
                    "#,
                )
                .bind(update.user_id.as_str())
                .bind(update.provider_key.as_str())
                .bind(update.base_url.trim())
                .bind(&ciphertext)
                .bind(if update.enabled { 1_i32 } else { 0_i32 })
                .bind(created_at)
                .bind(now)
                .execute(pool)
                .await
                .context("failed to persist provider setting")?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO provider_settings
                    (user_id, provider_key, base_url, api_key_ciphertext, enabled, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(user_id, provider_key) DO UPDATE SET
                      base_url = EXCLUDED.base_url,
                      api_key_ciphertext = EXCLUDED.api_key_ciphertext,
                      enabled = EXCLUDED.enabled,
                      updated_at = EXCLUDED.updated_at
                    "#,
                )
                .bind(update.user_id.as_str())
                .bind(update.provider_key.as_str())
                .bind(update.base_url.trim())
                .bind(&ciphertext)
                .bind(if update.enabled { 1_i32 } else { 0_i32 })
                .bind(created_at)
                .bind(now)
                .execute(pool)
                .await
                .context("failed to persist provider setting")?;
            }
        }

        Ok(PersistedProviderSetting {
            provider_key: update.provider_key,
            base_url: update.base_url.trim().to_string(),
            enabled: update.enabled,
            created_at: created_at.to_string(),
            updated_at: now.to_string(),
            api_key,
        })
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
        .map_err(|_| anyhow!("failed to encrypt provider secret"))?;
    Ok(format!(
        "{}.{}",
        BASE64.encode(nonce),
        BASE64.encode(ciphertext)
    ))
}

fn decrypt_secret(key_bytes: &[u8; 32], encoded: &str) -> anyhow::Result<String> {
    let (nonce_part, ciphertext_part) = encoded
        .split_once('.')
        .ok_or_else(|| anyhow!("provider secret payload is malformed"))?;
    let nonce = BASE64
        .decode(nonce_part)
        .context("failed to decode provider nonce")?;
    let ciphertext = BASE64
        .decode(ciphertext_part)
        .context("failed to decode provider ciphertext")?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow!("failed to decrypt provider secret"))?;
    String::from_utf8(plaintext).context("provider secret is not valid utf-8")
}
