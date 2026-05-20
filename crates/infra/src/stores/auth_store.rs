use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::Row;

use crate::db::DatabasePool;

fn now_millis_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_millis().try_into().unwrap_or(i64::MAX)
}

#[derive(Clone)]
pub struct StoredUser {
    pub user: StoredAuthUser,
    pub password_hash: String,
}

#[derive(Clone)]
pub struct StoredRefreshToken {
    pub user: StoredAuthUser,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct StoredAuthUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
}

#[derive(Clone)]
pub struct AuthStore {
    pool: Arc<DatabasePool>,
}

impl AuthStore {
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self { pool }
    }

    pub async fn seed_demo_user(&self) -> anyhow::Result<()> {
        if let Some(stored) = self.find_user_by_email("demo@openchat.local").await? {
            if PasswordHash::new(stored.password_hash.as_str()).is_err() {
                self.update_password_hash(
                    stored.user.id.as_str(),
                    hash_password("openchat-demo")?.as_str(),
                )
                .await?;
            }
            return Ok(());
        }

        let demo = StoredAuthUser {
            id: "user_openchat_demo".into(),
            username: "OpenChat Demo".into(),
            email: "demo@openchat.local".into(),
            is_admin: true,
        };
        self.insert_user(demo, hash_password("openchat-demo")?)
            .await
    }

    pub async fn insert_user(
        &self,
        user: StoredAuthUser,
        password_hash: String,
    ) -> anyhow::Result<()> {
        let now = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, username, password_hash, is_admin, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                )
                .bind(user.id)
                .bind(user.email)
                .bind(user.username)
                .bind(password_hash)
                .bind(if user.is_admin { 1_i64 } else { 0_i64 })
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, username, password_hash, is_admin, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                )
                .bind(user.id)
                .bind(user.email)
                .bind(user.username)
                .bind(password_hash)
                .bind(user.is_admin)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn find_user_by_email(&self, email: &str) -> anyhow::Result<Option<StoredUser>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT id, email, username, is_admin, password_hash
                    FROM users
                    WHERE email = ?1
                    "#,
                )
                .bind(email.to_lowercase())
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredUser {
                    user: StoredAuthUser {
                        id: row.get::<String, _>("id"),
                        email: row.get::<String, _>("email"),
                        username: row.get::<String, _>("username"),
                        is_admin: row.get::<i32, _>("is_admin") != 0,
                    },
                    password_hash: row.get::<String, _>("password_hash"),
                }))
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT id, email, username, is_admin, password_hash
                    FROM users
                    WHERE email = $1
                    "#,
                )
                .bind(email.to_lowercase())
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredUser {
                    user: StoredAuthUser {
                        id: row.get::<String, _>("id"),
                        email: row.get::<String, _>("email"),
                        username: row.get::<String, _>("username"),
                        is_admin: row.get::<bool, _>("is_admin"),
                    },
                    password_hash: row.get::<String, _>("password_hash"),
                }))
            }
        }
    }

    pub async fn find_user_by_id(&self, user_id: &str) -> anyhow::Result<Option<StoredAuthUser>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT id, email, username, is_admin
                    FROM users
                    WHERE id = ?1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredAuthUser {
                    id: row.get::<String, _>("id"),
                    email: row.get::<String, _>("email"),
                    username: row.get::<String, _>("username"),
                    is_admin: row.get::<i32, _>("is_admin") != 0,
                }))
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT id, email, username, is_admin
                    FROM users
                    WHERE id = $1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredAuthUser {
                    id: row.get::<String, _>("id"),
                    email: row.get::<String, _>("email"),
                    username: row.get::<String, _>("username"),
                    is_admin: row.get::<bool, _>("is_admin"),
                }))
            }
        }
    }

    pub async fn email_exists(&self, email: &str) -> anyhow::Result<bool> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let row = sqlx::query("SELECT 1 FROM users WHERE email = ?1")
                    .bind(email.to_lowercase())
                    .fetch_optional(pool)
                    .await?;
                Ok(row.is_some())
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT 1 FROM users WHERE email = $1")
                    .bind(email.to_lowercase())
                    .fetch_optional(pool)
                    .await?;
                Ok(row.is_some())
            }
        }
    }

    pub async fn store_refresh_token(
        &self,
        user_id: &str,
        refresh_token: String,
    ) -> anyhow::Result<()> {
        let created_at = now_millis_i64();
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query("INSERT OR REPLACE INTO refresh_tokens (token, user_id, created_at) VALUES (?1, ?2, ?3)")
                    .bind(refresh_token)
                    .bind(user_id)
                    .bind(created_at)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO refresh_tokens (token, user_id, created_at)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (token) DO UPDATE SET
                      user_id = EXCLUDED.user_id,
                      created_at = EXCLUDED.created_at
                    "#,
                )
                .bind(refresh_token)
                .bind(user_id)
                .bind(created_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn update_password_hash(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> anyhow::Result<()> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query(
                    r#"
                    UPDATE users
                    SET password_hash = ?2
                    WHERE id = ?1
                    "#,
                )
                .bind(user_id)
                .bind(password_hash)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    UPDATE users
                    SET password_hash = $2
                    WHERE id = $1
                    "#,
                )
                .bind(user_id)
                .bind(password_hash)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn find_refresh_token(
        &self,
        token: &str,
    ) -> anyhow::Result<Option<StoredRefreshToken>> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT u.id, u.email, u.username, u.is_admin, t.created_at
                    FROM refresh_tokens t
                    JOIN users u ON u.id = t.user_id
                    WHERE t.token = ?1
                    "#,
                )
                .bind(token)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredRefreshToken {
                    user: StoredAuthUser {
                        id: row.get::<String, _>("id"),
                        email: row.get::<String, _>("email"),
                        username: row.get::<String, _>("username"),
                        is_admin: row.get::<i32, _>("is_admin") != 0,
                    },
                    created_at: row.get::<i64, _>("created_at"),
                }))
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    r#"
                    SELECT u.id, u.email, u.username, u.is_admin, t.created_at
                    FROM refresh_tokens t
                    JOIN users u ON u.id = t.user_id
                    WHERE t.token = $1
                    "#,
                )
                .bind(token)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| StoredRefreshToken {
                    user: StoredAuthUser {
                        id: row.get::<String, _>("id"),
                        email: row.get::<String, _>("email"),
                        username: row.get::<String, _>("username"),
                        is_admin: row.get::<bool, _>("is_admin"),
                    },
                    created_at: row.get::<i64, _>("created_at"),
                }))
            }
        }
    }

    pub async fn revoke_refresh_token(&self, refresh_token: &str) -> anyhow::Result<()> {
        match self.pool.as_ref() {
            DatabasePool::Compat(pool) => {
                sqlx::query("DELETE FROM refresh_tokens WHERE token = ?1")
                    .bind(refresh_token)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM refresh_tokens WHERE token = $1")
                    .bind(refresh_token)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(hash.to_string())
}
