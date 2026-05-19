use std::sync::Arc;

use openchat_core::ChatServiceError;
use openchat_infra::sqlite::{CustomModelCreate, SqliteCustomModelStore};

use crate::{CreateUserCustomModel, UserCustomModel};

#[derive(Clone)]
pub struct CustomModelService {
    store: Arc<SqliteCustomModelStore>,
}

impl CustomModelService {
    pub fn new(store: Arc<SqliteCustomModelStore>) -> Self {
        Self { store }
    }

    pub async fn list_user_models(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserCustomModel>, ChatServiceError> {
        self.store
            .list_user_models(user_id)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| UserCustomModel {
                        model_config_id: item.model_config_id,
                        model_name: item.model_name,
                        model_type: item.model_type,
                        base_url: item.base_url,
                        has_api_key: !item.api_key.trim().is_empty(),
                        created_at: item.created_at,
                        updated_at: item.updated_at,
                    })
                    .collect()
            })
            .map_err(internal_error)
    }

    pub async fn create_user_model(
        &self,
        user_id: &str,
        create: CreateUserCustomModel,
    ) -> Result<UserCustomModel, ChatServiceError> {
        validate_custom_model(
            create.model_name.as_str(),
            create.model_type.as_str(),
            create.base_url.as_str(),
            create.api_key.as_str(),
        )?;
        let model_config_id = build_custom_model_config_id(create.model_name.as_str());

        self.store
            .create_user_model(CustomModelCreate {
                user_id: user_id.to_string(),
                model_config_id,
                model_name: create.model_name,
                model_type: create.model_type,
                base_url: create.base_url,
                api_key: create.api_key,
            })
            .await
            .map(|item| UserCustomModel {
                model_config_id: item.model_config_id,
                model_name: item.model_name,
                model_type: item.model_type,
                base_url: item.base_url,
                has_api_key: !item.api_key.trim().is_empty(),
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .map_err(internal_error)
    }

    pub async fn find_user_model(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> Result<Option<UserCustomModel>, ChatServiceError> {
        self.store
            .find_user_model(user_id, model_config_id)
            .await
            .map(|item| {
                item.map(|value| UserCustomModel {
                    model_config_id: value.model_config_id,
                    model_name: value.model_name,
                    model_type: value.model_type,
                    base_url: value.base_url,
                    has_api_key: !value.api_key.trim().is_empty(),
                    created_at: value.created_at,
                    updated_at: value.updated_at,
                })
            })
            .map_err(internal_error)
    }

    pub async fn resolve_user_model_credentials(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> Result<Option<(String, String)>, ChatServiceError> {
        self.store
            .find_user_model(user_id, model_config_id)
            .await
            .map(|item| item.map(|value| (value.base_url, value.api_key)))
            .map_err(internal_error)
    }

    pub async fn delete_user_model(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> Result<bool, ChatServiceError> {
        self.store
            .delete_user_model(user_id, model_config_id)
            .await
            .map_err(internal_error)
    }
}

fn validate_custom_model(
    model_name: &str,
    model_type: &str,
    base_url: &str,
    api_key: &str,
) -> Result<(), ChatServiceError> {
    if model_name.trim().is_empty() {
        return Err(ChatServiceError::new(
            400,
            "A custom model name is required",
        ));
    }
    if !matches!(model_type, "text" | "multimodal") {
        return Err(ChatServiceError::new(400, "Unsupported custom model type"));
    }
    if base_url.trim().is_empty() {
        return Err(ChatServiceError::new(
            400,
            "A custom model base URL is required",
        ));
    }
    if !(base_url.trim().starts_with("http://") || base_url.trim().starts_with("https://")) {
        return Err(ChatServiceError::new(
            400,
            "Custom model base URL must start with http:// or https://",
        ));
    }
    if api_key.trim().is_empty() {
        return Err(ChatServiceError::new(
            400,
            "A custom model API key is required",
        ));
    }
    Ok(())
}

fn build_custom_model_config_id(model_name: &str) -> String {
    let slug = model_name
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    format!("custom:model:{slug}")
}

fn internal_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::new(500, error.to_string())
}
