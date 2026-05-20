use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use openchat_infra::stores::{AuthStore, StoredAuthUser, StoredUser};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{AuthError, AuthSession, AuthUser};

pub const ACCESS_TOKEN_TTL_MILLIS: i64 = 15 * 60 * 1000;
pub const REFRESH_TOKEN_TTL_MILLIS: i64 = 30 * 24 * 60 * 60 * 1000;
const TOKEN_VERSION: &str = "v1";

type HmacSha256 = Hmac<Sha256>;

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[derive(Clone)]
pub struct AuthService {
    store: Arc<AuthStore>,
    token_signer: Arc<TokenSigner>,
}

impl AuthService {
    pub fn new(store: Arc<AuthStore>, secret: &str) -> Self {
        Self {
            store,
            token_signer: Arc::new(TokenSigner::new(secret)),
        }
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

        let password_hash = hash_password(password).map_err(internal_error)?;
        self.store
            .insert_user(to_stored_user(&user), password_hash)
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

        self.verify_password_and_upgrade(&stored, password).await?;
        self.issue_session(from_stored_user(stored.user)).await
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<AuthSession, AuthError> {
        let Some(refresh_record) = self
            .store
            .find_refresh_token(refresh_token.as_str())
            .await
            .map_err(internal_error)?
        else {
            return Err(AuthError::new(401, "Authentication required"));
        };

        self.ensure_refresh_token_not_expired(&refresh_record)
            .await?;
        self.store
            .revoke_refresh_token(refresh_token.as_str())
            .await
            .map_err(internal_error)?;
        self.issue_session(from_stored_user(refresh_record.user))
            .await
    }

    pub async fn current_user(&self, access_token: &str) -> Result<AuthUser, AuthError> {
        let claims = self.token_signer.verify(access_token)?;
        let Some(user) = self
            .store
            .find_user_by_id(claims.sub.as_str())
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
        let issued_at = now_millis();
        let token = self
            .token_signer
            .sign(&AccessTokenClaims {
                sub: user.id.clone(),
                email: user.email.clone(),
                username: user.username.clone(),
                is_admin: user.is_admin,
                iat: issued_at,
                exp: issued_at.saturating_add(ACCESS_TOKEN_TTL_MILLIS),
                version: TOKEN_VERSION.to_string(),
            })
            .map_err(internal_error)?;

        let refresh_token = format!("rt_{}", random_token_suffix());

        self.store
            .store_refresh_token(user.id.as_str(), refresh_token.clone())
            .await
            .map_err(internal_error)?;
        Ok(AuthSession::new(user, token, refresh_token))
    }

    async fn verify_password_and_upgrade(
        &self,
        stored: &StoredUser,
        password: &str,
    ) -> Result<(), AuthError> {
        let parsed = PasswordHash::new(stored.password_hash.as_str())
            .map_err(|_| AuthError::new(500, "Stored password hash is invalid"))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AuthError::new(401, "Invalid email or password"))
    }

    async fn ensure_refresh_token_not_expired(
        &self,
        refresh_record: &openchat_infra::stores::StoredRefreshToken,
    ) -> Result<(), AuthError> {
        if now_millis().saturating_sub(refresh_record.created_at) > REFRESH_TOKEN_TTL_MILLIS {
            return Err(AuthError::new(401, "Authentication required"));
        }

        Ok(())
    }
}

#[derive(Clone)]
struct TokenSigner {
    secret: Vec<u8>,
}

impl TokenSigner {
    fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    fn sign(&self, claims: &AccessTokenClaims) -> anyhow::Result<String> {
        let payload = serde_json::to_vec(claims)?;
        let encoded_payload = URL_SAFE_NO_PAD.encode(payload);
        let signature = self.signature(encoded_payload.as_bytes())?;
        Ok(format!("{TOKEN_VERSION}.{encoded_payload}.{signature}"))
    }

    fn verify(&self, token: &str) -> Result<AccessTokenClaims, AuthError> {
        let mut parts = token.split('.');
        let Some(version) = parts.next() else {
            return Err(AuthError::new(401, "Authentication required"));
        };
        let Some(payload) = parts.next() else {
            return Err(AuthError::new(401, "Authentication required"));
        };
        let Some(signature) = parts.next() else {
            return Err(AuthError::new(401, "Authentication required"));
        };
        if parts.next().is_some() || version != TOKEN_VERSION {
            return Err(AuthError::new(401, "Authentication required"));
        }

        let expected_signature = self.signature(payload.as_bytes()).map_err(internal_error)?;
        if expected_signature != signature {
            return Err(AuthError::new(401, "Authentication required"));
        }

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AuthError::new(401, "Authentication required"))?;
        let claims: AccessTokenClaims = serde_json::from_slice(payload_bytes.as_slice())
            .map_err(|_| AuthError::new(401, "Authentication required"))?;

        if claims.exp < now_millis() {
            return Err(AuthError::new(401, "Authentication required"));
        }

        Ok(claims)
    }

    fn signature(&self, payload: &[u8]) -> anyhow::Result<String> {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_slice())?;
        mac.update(TOKEN_VERSION.as_bytes());
        mac.update(b".");
        mac.update(payload);
        Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    email: String,
    username: String,
    is_admin: bool,
    iat: i64,
    exp: i64,
    version: String,
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

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(hash.to_string())
}

fn random_token_suffix() -> String {
    let bytes: [u8; 24] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
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

#[cfg(test)]
mod tests {
    use super::{TokenSigner, ACCESS_TOKEN_TTL_MILLIS, TOKEN_VERSION};

    #[test]
    fn signed_token_round_trip() {
        let signer = TokenSigner::new("test-secret");
        let now = super::now_millis();
        let token = signer
            .sign(&super::AccessTokenClaims {
                sub: "user_1".to_string(),
                email: "test@example.com".to_string(),
                username: "tester".to_string(),
                is_admin: false,
                iat: now,
                exp: now + ACCESS_TOKEN_TTL_MILLIS,
                version: TOKEN_VERSION.to_string(),
            })
            .unwrap();

        let claims = signer.verify(token.as_str()).unwrap();
        assert_eq!(claims.sub, "user_1");
    }
}
