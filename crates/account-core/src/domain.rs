#[derive(Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
}

impl AuthUser {
    pub fn demo() -> Self {
        Self {
            id: "user_openchat_demo".into(),
            username: "OpenChat Demo".into(),
            email: "demo@openchat.local".into(),
            is_admin: true,
        }
    }
}

#[derive(Clone)]
pub struct AuthSession {
    pub status: String,
    pub token: String,
    pub refresh_token: String,
    pub user: AuthUser,
}

impl AuthSession {
    pub fn new(user: AuthUser, token: String, refresh_token: String) -> Self {
        Self {
            status: "ok".into(),
            token,
            refresh_token,
            user,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthError {
    pub status_code: u16,
    pub message: String,
}

impl AuthError {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
        }
    }
}

#[derive(Clone)]
pub struct UserProviderApiKey {
    pub provider_key: String,
    pub has_api_key: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct UpsertUserProviderApiKey {
    pub provider_key: String,
    pub api_key: Option<String>,
}

#[derive(Clone)]
pub struct UserCustomModel {
    pub model_config_id: String,
    pub model_name: String,
    pub model_type: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct CreateUserCustomModel {
    pub model_name: String,
    pub model_type: String,
    pub base_url: String,
    pub api_key: String,
}
