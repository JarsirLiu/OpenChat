use std::{collections::HashMap, sync::Arc};

use openchat_core::{
    ChatServiceError, ResolvedImageModelAccess, ResolvedTextModelAccess, ToolAccessOutcome,
    ToolAccessRequirement, TurnModelRef, TurnToolRef,
};
use openchat_infra::sqlite::{ProviderSettingUpdate, SqliteProviderSettingsStore};

use crate::{UpsertUserProviderSetting, UserProviderSetting};

#[derive(Clone)]
pub struct ProviderRuntimeFallback {
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Clone)]
pub struct ModelProviderService {
    store: Arc<SqliteProviderSettingsStore>,
    fallbacks: Arc<HashMap<String, ProviderRuntimeFallback>>,
}

impl ModelProviderService {
    pub fn new(
        store: Arc<SqliteProviderSettingsStore>,
        fallbacks: HashMap<String, ProviderRuntimeFallback>,
    ) -> Self {
        Self {
            store,
            fallbacks: Arc::new(fallbacks),
        }
    }

    pub async fn list_user_settings(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserProviderSetting>, ChatServiceError> {
        self.store
            .list_user_settings(user_id)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| UserProviderSetting {
                        provider_key: item.provider_key,
                        base_url: item.base_url,
                        enabled: item.enabled,
                        has_api_key: !item.api_key.trim().is_empty(),
                        created_at: item.created_at,
                        updated_at: item.updated_at,
                    })
                    .collect()
            })
            .map_err(internal_error)
    }

    pub async fn upsert_user_setting(
        &self,
        user_id: &str,
        update: UpsertUserProviderSetting,
    ) -> Result<UserProviderSetting, ChatServiceError> {
        validate_setting(update.provider_key.as_str(), update.base_url.as_str())?;
        self.store
            .upsert_user_setting(ProviderSettingUpdate {
                user_id: user_id.to_string(),
                provider_key: update.provider_key,
                base_url: update.base_url,
                api_key: update.api_key,
                enabled: update.enabled,
            })
            .await
            .map(|item| UserProviderSetting {
                provider_key: item.provider_key,
                base_url: item.base_url,
                enabled: item.enabled,
                has_api_key: !item.api_key.trim().is_empty(),
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .map_err(internal_error)
    }

    pub async fn resolve_text_access(
        &self,
        user_id: &str,
        model: &TurnModelRef,
    ) -> Result<ResolvedTextModelAccess, ChatServiceError> {
        let (base_url, api_key) = self
            .resolve_provider_credentials(user_id, model.provider.as_str())
            .await?;

        Ok(ResolvedTextModelAccess {
            provider_key: model.provider.clone(),
            runtime_provider: model.runtime_provider.clone(),
            model_name: model.model_name.clone(),
            display_name: model.display_name.clone(),
            base_url,
            api_key,
            input_modalities: model.input_modalities.clone(),
        })
    }

    pub async fn resolve_image_access(
        &self,
        user_id: &str,
        tool: &TurnToolRef,
    ) -> Result<ResolvedImageModelAccess, ChatServiceError> {
        let (base_url, api_key) = self
            .resolve_provider_credentials(user_id, tool.provider.as_str())
            .await?;

        Ok(ResolvedImageModelAccess {
            provider_key: tool.provider.clone(),
            runtime_provider: tool.runtime_provider.clone(),
            model_name: tool.model_name.clone(),
            display_name: tool.display_name.clone(),
            base_url,
            api_key,
        })
    }

    pub async fn resolve_tool_access(
        &self,
        user_id: &str,
        requirement: &ToolAccessRequirement,
    ) -> Result<ToolAccessOutcome, ChatServiceError> {
        match requirement.runtime_provider.as_str() {
            "openai_compatible" => {}
            provider => {
                return Ok(ToolAccessOutcome::Denied {
                    reason: format!("Runtime provider `{provider}` is not supported yet"),
                });
            }
        }

        self.inspect_provider_access(user_id, requirement.provider_key.as_str())
            .await
    }

    async fn resolve_provider_credentials(
        &self,
        user_id: &str,
        provider_key: &str,
    ) -> Result<(String, String), ChatServiceError> {
        match self
            .provider_credentials_state(user_id, provider_key)
            .await?
        {
            ProviderCredentialsState::Ready { base_url, api_key } => Ok((base_url, api_key)),
            ProviderCredentialsState::Denied { reason } => Err(ChatServiceError::new(400, reason)),
        }
    }

    async fn inspect_provider_access(
        &self,
        user_id: &str,
        provider_key: &str,
    ) -> Result<ToolAccessOutcome, ChatServiceError> {
        match self
            .provider_credentials_state(user_id, provider_key)
            .await?
        {
            ProviderCredentialsState::Ready { .. } => Ok(ToolAccessOutcome::Allowed),
            ProviderCredentialsState::Denied { reason } => Ok(ToolAccessOutcome::Denied { reason }),
        }
    }

    async fn provider_credentials_state(
        &self,
        user_id: &str,
        provider_key: &str,
    ) -> Result<ProviderCredentialsState, ChatServiceError> {
        if let Some(setting) = self
            .store
            .find_user_setting(user_id, provider_key)
            .await
            .map_err(internal_error)?
        {
            if !setting.enabled {
                return Ok(ProviderCredentialsState::Denied {
                    reason: "当前模型接入已关闭，请先在右侧参数中启用并保存 API Key".to_string(),
                });
            }

            if setting.base_url.trim().is_empty() || setting.api_key.trim().is_empty() {
                return Ok(ProviderCredentialsState::Denied {
                    reason: "请先在右侧参数中保存 API Key，然后再使用这个模型".to_string(),
                });
            }

            return Ok(ProviderCredentialsState::Ready {
                base_url: setting.base_url,
                api_key: setting.api_key,
            });
        }

        let Some(fallback) = self.fallbacks.get(provider_key) else {
            return Ok(ProviderCredentialsState::Denied {
                reason: "请先在右侧参数中保存 API Key，然后再使用这个模型".to_string(),
            });
        };

        let Some(api_key) = fallback.api_key.clone() else {
            return Ok(ProviderCredentialsState::Denied {
                reason: "请先在右侧参数中保存 API Key，然后再使用这个模型".to_string(),
            });
        };

        Ok(ProviderCredentialsState::Ready {
            base_url: fallback.base_url.clone(),
            api_key,
        })
    }
}

enum ProviderCredentialsState {
    Ready { base_url: String, api_key: String },
    Denied { reason: String },
}

fn validate_setting(provider_key: &str, base_url: &str) -> Result<(), ChatServiceError> {
    if provider_key.trim().is_empty() {
        return Err(ChatServiceError::new(400, "A provider key is required"));
    }
    let normalized = base_url.trim();
    if normalized.is_empty() {
        return Err(ChatServiceError::new(
            400,
            "A provider base URL is required",
        ));
    }
    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        return Err(ChatServiceError::new(
            400,
            "Provider base URL must start with http:// or https://",
        ));
    }
    Ok(())
}

fn internal_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::new(500, error.to_string())
}
