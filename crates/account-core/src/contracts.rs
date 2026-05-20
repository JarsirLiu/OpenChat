use serde::{Deserialize, Serialize};

use crate::{
    AuthSession, AuthUser, CreateUserCustomModel, UpsertUserProviderApiKey, UserCustomModel,
    UserProviderApiKey, UserProviderApiKeySecret,
};

#[derive(Deserialize)]
pub struct LoginRequestDto {
    pub account: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequestDto {
    pub email: String,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct RefreshRequestDto {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequestDto {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct UserInfoDto {
    pub user_info: UserDto,
}

#[derive(Serialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
}

#[derive(Serialize)]
pub struct AuthResponseDto {
    pub status: String,
    pub token: String,
    pub refresh_token: String,
    pub user_info: UserDto,
}

#[derive(Serialize)]
pub struct UserProviderApiKeyDto {
    pub provider_key: String,
    pub has_api_key: bool,
    pub masked_api_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct UserProviderApiKeySecretDto {
    pub provider_key: String,
    pub api_key: String,
}

#[derive(Deserialize)]
pub struct UpsertUserProviderApiKeyDto {
    pub provider_key: String,
    pub api_key: Option<String>,
}

#[derive(Serialize)]
pub struct UserCustomModelDto {
    pub model_config_id: String,
    pub model_name: String,
    pub model_type: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateUserCustomModelDto {
    pub model_name: String,
    #[serde(rename = "type")]
    pub model_type: String,
    pub base_url: String,
    pub api_key: String,
}

impl From<AuthUser> for UserDto {
    fn from(value: AuthUser) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            is_admin: value.is_admin,
        }
    }
}

impl From<AuthUser> for UserInfoDto {
    fn from(value: AuthUser) -> Self {
        Self {
            user_info: UserDto::from(value),
        }
    }
}

impl From<AuthSession> for AuthResponseDto {
    fn from(value: AuthSession) -> Self {
        Self {
            status: value.status,
            token: value.token,
            refresh_token: value.refresh_token,
            user_info: UserDto::from(value.user),
        }
    }
}

impl From<UserProviderApiKey> for UserProviderApiKeyDto {
    fn from(value: UserProviderApiKey) -> Self {
        Self {
            provider_key: value.provider_key,
            has_api_key: value.has_api_key,
            masked_api_key: value.masked_api_key,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<UserProviderApiKeySecret> for UserProviderApiKeySecretDto {
    fn from(value: UserProviderApiKeySecret) -> Self {
        Self {
            provider_key: value.provider_key,
            api_key: value.api_key,
        }
    }
}

impl From<UpsertUserProviderApiKeyDto> for UpsertUserProviderApiKey {
    fn from(value: UpsertUserProviderApiKeyDto) -> Self {
        Self {
            provider_key: value.provider_key,
            api_key: value.api_key,
        }
    }
}

impl From<UserCustomModel> for UserCustomModelDto {
    fn from(value: UserCustomModel) -> Self {
        Self {
            model_config_id: value.model_config_id,
            model_name: value.model_name,
            model_type: value.model_type,
            base_url: value.base_url,
            has_api_key: value.has_api_key,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<CreateUserCustomModelDto> for CreateUserCustomModel {
    fn from(value: CreateUserCustomModelDto) -> Self {
        Self {
            model_name: value.model_name,
            model_type: value.model_type,
            base_url: value.base_url,
            api_key: value.api_key,
        }
    }
}
