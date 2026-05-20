use std::sync::Arc;

use openchat_core::{
    ChatServiceError, ImageModelAccessResolver, ResolveImageAccessFuture, ResolveTextAccessFuture,
    ResolveToolAccessFuture, ResolvedImageModelAccess, ResolvedTextModelAccess,
    TextModelAccessResolver, ToolAccessOutcome, ToolAccessRequirement, ToolAccessResolver,
    TurnModelRef, TurnToolRef,
};

use crate::{
    AuthError, AuthService, AuthSession, AuthUser, CreateUserCustomModel, CustomModelService,
    ModelProviderService, UpsertUserProviderApiKey, UserCustomModel, UserProviderApiKey,
};

#[derive(Clone)]
pub struct AccountService {
    auth_service: Arc<AuthService>,
    model_provider_service: Arc<ModelProviderService>,
    custom_model_service: Arc<CustomModelService>,
}

impl AccountService {
    pub fn new(
        auth_service: Arc<AuthService>,
        model_provider_service: Arc<ModelProviderService>,
        custom_model_service: Arc<CustomModelService>,
    ) -> Self {
        Self {
            auth_service,
            model_provider_service,
            custom_model_service,
        }
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        username: Option<&str>,
    ) -> Result<AuthSession, AuthError> {
        self.auth_service.register(email, password, username).await
    }

    pub async fn login(&self, account: &str, password: &str) -> Result<AuthSession, AuthError> {
        self.auth_service.login(account, password).await
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<AuthSession, AuthError> {
        self.auth_service.refresh(refresh_token).await
    }

    pub async fn current_user(&self, access_token: &str) -> Result<AuthUser, AuthError> {
        self.auth_service.current_user(access_token).await
    }

    pub async fn logout(&self, refresh_token: &str) {
        self.auth_service.logout(refresh_token).await;
    }

    pub async fn list_user_provider_api_keys(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserProviderApiKey>, ChatServiceError> {
        self.model_provider_service
            .list_user_api_keys(user_id)
            .await
    }

    pub async fn upsert_user_provider_api_key(
        &self,
        user_id: &str,
        update: UpsertUserProviderApiKey,
    ) -> Result<UserProviderApiKey, ChatServiceError> {
        self.model_provider_service
            .upsert_user_api_key(user_id, update)
            .await
    }

    pub async fn list_user_custom_models(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserCustomModel>, ChatServiceError> {
        self.custom_model_service.list_user_models(user_id).await
    }

    pub async fn create_user_custom_model(
        &self,
        user_id: &str,
        create: CreateUserCustomModel,
    ) -> Result<UserCustomModel, ChatServiceError> {
        self.custom_model_service
            .create_user_model(user_id, create)
            .await
    }

    pub async fn delete_user_custom_model(
        &self,
        user_id: &str,
        model_config_id: &str,
    ) -> Result<bool, ChatServiceError> {
        self.custom_model_service
            .delete_user_model(user_id, model_config_id)
            .await
    }

    pub async fn resolve_text_access(
        &self,
        user_id: &str,
        model: &TurnModelRef,
    ) -> Result<ResolvedTextModelAccess, ChatServiceError> {
        if model.source == "custom" {
            let Some((base_url, api_key)) = self
                .custom_model_service
                .resolve_user_model_credentials(user_id, model.model_config_id.as_str())
                .await?
            else {
                return Err(ChatServiceError::new(
                    400,
                    "Selected custom model is not available",
                ));
            };

            return Ok(ResolvedTextModelAccess {
                provider_key: model.provider.clone(),
                runtime_provider: model.runtime_provider.clone(),
                model_name: model.model_name.clone(),
                display_name: model.display_name.clone(),
                base_url,
                api_key,
                input_modalities: model.input_modalities.clone(),
            });
        }

        self.model_provider_service
            .resolve_text_access(user_id, model)
            .await
    }

    pub async fn resolve_image_access(
        &self,
        user_id: &str,
        tool: &TurnToolRef,
    ) -> Result<ResolvedImageModelAccess, ChatServiceError> {
        if tool.source == "custom" {
            let Some((base_url, api_key)) = self
                .custom_model_service
                .resolve_user_model_credentials(user_id, tool.model_config_id.as_str())
                .await?
            else {
                return Err(ChatServiceError::new(
                    400,
                    "Selected custom image model is not available",
                ));
            };

            return Ok(ResolvedImageModelAccess {
                provider_key: tool.provider.clone(),
                runtime_provider: tool.runtime_provider.clone(),
                model_name: tool.model_name.clone(),
                display_name: tool.display_name.clone(),
                base_url,
                api_key,
            });
        }

        self.model_provider_service
            .resolve_image_access(user_id, tool)
            .await
    }

    pub async fn resolve_tool_access(
        &self,
        user_id: &str,
        requirement: &ToolAccessRequirement,
    ) -> Result<ToolAccessOutcome, ChatServiceError> {
        if requirement.source == "custom" {
            let credentials = self
                .custom_model_service
                .resolve_user_model_credentials(user_id, requirement.model_config_id.as_str())
                .await?;

            return Ok(match credentials {
                Some((base_url, api_key))
                    if !base_url.trim().is_empty() && !api_key.trim().is_empty() =>
                {
                    ToolAccessOutcome::Allowed
                }
                Some(_) => ToolAccessOutcome::Denied {
                    reason: "自定义图片模型未完整配置".to_string(),
                },
                None => ToolAccessOutcome::Denied {
                    reason: "自定义图片模型不存在或不可用".to_string(),
                },
            });
        }

        self.model_provider_service
            .resolve_tool_access(user_id, requirement)
            .await
    }
}

impl TextModelAccessResolver for AccountService {
    fn resolve_text_access<'a>(
        &'a self,
        user_id: &'a str,
        model: &'a TurnModelRef,
    ) -> ResolveTextAccessFuture<'a> {
        Box::pin(async move { self.resolve_text_access(user_id, model).await })
    }
}

impl ImageModelAccessResolver for AccountService {
    fn resolve_image_access<'a>(
        &'a self,
        user_id: &'a str,
        tool: &'a TurnToolRef,
    ) -> ResolveImageAccessFuture<'a> {
        Box::pin(async move { self.resolve_image_access(user_id, tool).await })
    }
}

impl ToolAccessResolver for AccountService {
    fn resolve_tool_access<'a>(
        &'a self,
        user_id: &'a str,
        requirement: &'a ToolAccessRequirement,
    ) -> ResolveToolAccessFuture<'a> {
        Box::pin(async move { self.resolve_tool_access(user_id, requirement).await })
    }
}
