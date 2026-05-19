use std::sync::Arc;

use openchat_infra::sqlite::{SqliteAuthStore, StoredAuthUser};

use crate::{AuthError, AuthSession, AuthUser};

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Clone)]
pub struct AuthService {
    store: Arc<SqliteAuthStore>,
}

impl AuthService {
    pub fn new(store: Arc<SqliteAuthStore>) -> Self {
        Self { store }
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        username: Option<&str>,
    ) -> Result<AuthSession, AuthError> {
        let email = normalize_email(email)?;
        validate_password(password)?;

        if self
            .store
            .email_exists(email.as_str())
            .await
            .map_err(internal_error)?
        {
            return Err(AuthError::new(409, "Email is already registered"));
        }

        let user = AuthUser {
            id: format!("user_{}", now_millis()),
            username: username
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| email.split('@').next().unwrap_or("OpenChat User"))
                .to_string(),
            email: email.clone(),
            is_admin: false,
        };

        self.store
            .insert_user(to_stored_user(&user), password.to_string())
            .await
            .map_err(internal_error)?;
        self.issue_session(user).await
    }

    pub async fn login(&self, account: &str, password: &str) -> Result<AuthSession, AuthError> {
        let email = normalize_email(account)?;
        let Some(stored) = self
            .store
            .find_user_by_email(email.as_str())
            .await
            .map_err(internal_error)?
        else {
            return Err(AuthError::new(401, "Invalid email or password"));
        };

        if stored.password_hash != password {
            return Err(AuthError::new(401, "Invalid email or password"));
        }

        self.issue_session(from_stored_user(stored.user)).await
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<AuthSession, AuthError> {
        let Some(user) = self
            .store
            .user_for_refresh_token(refresh_token.as_str())
            .await
            .map_err(internal_error)?
        else {
            return Err(AuthError::new(401, "Authentication required"));
        };
        self.issue_session(from_stored_user(user)).await
    }

    pub async fn current_user(&self, access_token: &str) -> Result<AuthUser, AuthError> {
        let Some(user) = self
            .store
            .user_for_access_token(access_token)
            .await
            .map_err(internal_error)?
        else {
            return Err(AuthError::new(401, "Authentication required"));
        };
        Ok(from_stored_user(user))
    }

    pub async fn logout(&self, refresh_token: &str) {
        let _ = self.store.revoke_refresh_token(refresh_token).await;
    }

    async fn issue_session(&self, user: AuthUser) -> Result<AuthSession, AuthError> {
        let token = format!("token_{}_{}", user.id, now_millis());
        let refresh_token = format!("refresh_{}_{}", user.id, now_millis());
        self.store
            .store_tokens(user.id.as_str(), token.clone(), refresh_token.clone())
            .await
            .map_err(internal_error)?;
        Ok(AuthSession::new(user, token, refresh_token))
    }
}

fn normalize_email(value: &str) -> Result<String, AuthError> {
    let email = value.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AuthError::new(400, "A valid email is required"));
    }
    Ok(email)
}

fn validate_password(value: &str) -> Result<(), AuthError> {
    if value.len() < 6 {
        return Err(AuthError::new(
            400,
            "Password must be at least 6 characters",
        ));
    }
    Ok(())
}

fn internal_error(error: anyhow::Error) -> AuthError {
    AuthError::new(500, error.to_string())
}

fn to_stored_user(user: &AuthUser) -> StoredAuthUser {
    StoredAuthUser {
        id: user.id.clone(),
        username: user.username.clone(),
        email: user.email.clone(),
        is_admin: user.is_admin,
    }
}

fn from_stored_user(user: StoredAuthUser) -> AuthUser {
    AuthUser {
        id: user.id,
        username: user.username,
        email: user.email,
        is_admin: user.is_admin,
    }
}
