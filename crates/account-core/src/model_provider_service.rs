use std::sync::Arc;

use openchat_core::{
    ChatServiceError, ResolvedImageModelAccess, ResolvedTextModelAccess, ToolAccessOutcome,
    ToolAccessRequirement, TurnModelRef, TurnToolRef,
};
use openchat_infra::stores::{
    PersistedUserProviderApiKey, UpdateUserProviderApiKey, UserProviderApiKeyStore,
};

use crate::{
    SystemProviderRegistry, UpsertUserProviderApiKey, UserProviderApiKey,
};

#[derive(Clone)]
pub struct ModelProviderService {
    user_provider_api_key_store: Arc<UserProviderApiKeyStore>,
    system_provider_registry: Arc<SystemProviderRegistry>,
}

impl ModelProviderService {
    pub fn new(
        user_provider_api_key_store: Arc<UserProviderApiKeyStore>,
        system_provider_registry: SystemProviderRegistry,
    ) -> Self {
        Self {
            user_provider_api_key_store,
            system_provider_registry: Arc::new(system_provider_registry),
        }
    }

    pub async fn list_user_api_keys(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserProviderApiKey>, ChatServiceError> {
        self.user_provider_api_key_store
            .list_user_api_keys(user_id)
            .await
            .map_err(internal_error)
            .map(|items| {
                items
                    .into_iter()
                    .map(Self::build_user_api_key)
                    .collect::<Vec<_>>()
            })
    }

    pub async fn upsert_user_api_key(
        &self,
        user_id: &str,
        update: UpsertUserProviderApiKey,
    ) -> Result<UserProviderApiKey, ChatServiceError> {
        validate_provider_key(update.provider_key.as_str())?;
        self.user_provider_api_key_store
            .upsert_user_api_key(UpdateUserProviderApiKey {
                user_id: user_id.to_string(),
                provider_key: update.provider_key,
                api_key: update.api_key,
            })
            .await
            .map(Self::build_user_api_key)
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
        let Some(system_provider) = self.system_provider_registry.get(provider_key) else {
            return Ok(ProviderCredentialsState::Denied {
                reason: "当前模型绑定的系统 Provider 不存在".to_string(),
            });
        };

        if let Some(api_key_record) = self
            .user_provider_api_key_store
            .find_user_api_key(user_id, provider_key)
            .await
            .map_err(internal_error)?
        {
            let base_url = system_provider.base_url.clone();
            let api_key = api_key_record.api_key;

            if base_url.trim().is_empty() || api_key.trim().is_empty() {
                return Ok(ProviderCredentialsState::Denied {
                    reason: "请先在右侧参数中填写 API Key，然后再使用这个模型".to_string(),
                });
            }

            return Ok(ProviderCredentialsState::Ready {
                base_url,
                api_key,
            });
        }

        Ok(ProviderCredentialsState::Denied {
            reason: "请先在右侧参数中填写 API Key，然后再使用这个模型".to_string(),
        })
    }

    fn build_user_api_key(stored: PersistedUserProviderApiKey) -> UserProviderApiKey {
        UserProviderApiKey {
            provider_key: stored.provider_key,
            has_api_key: !stored.api_key.trim().is_empty(),
            created_at: stored.created_at,
            updated_at: stored.updated_at,
        }
    }
}

enum ProviderCredentialsState {
    Ready { base_url: String, api_key: String },
    Denied { reason: String },
}

fn validate_provider_key(provider_key: &str) -> Result<(), ChatServiceError> {
    if provider_key.trim().is_empty() {
        return Err(ChatServiceError::new(400, "A provider key is required"));
    }
    Ok(())
}

fn internal_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::new(500, error.to_string())
}

