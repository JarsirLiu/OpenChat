use std::sync::Arc;
use std::collections::HashSet;

use openchat_account_core::{
    AccountService, AuthService, CustomModelService, ModelProviderService,
};
use openchat_catalog_core::{CatalogModel, CatalogService, CatalogTurnBuilder};
use openchat_core::{
    ActiveTurnRegistry, ChatService, ChatServiceError, ImageProviderRuntime, ImageRuntime,
    InMemorySessionStore, MediaStore, ModelMediaUrlResolver, ModelProviderRuntime, ModelRuntime,
    OpenChatTurnExecutor, StoredMedia, ToolAccessService, ToolExecutor,
};
use openchat_infra::db::Database;
use openchat_infra::stores::{
    CatalogModelRecord, CatalogToolRecord, MediaObjectRecord, AuthStore,
    CatalogStore, ChatStore, CustomModelStore, MediaObjectStore,
    UserProviderApiKeyStore,
};
use openchat_infra::storage::{
    build_object_store, DynObjectStore, LocalStorageConfig, RetrievedObject, S3StorageConfig,
    StorageBackendConfig,
};
use openchat_security_core::{
    AccessTokenAuthenticator, Authorizer, OwnershipAuthorizer, ResourceTokenService,
};

use crate::config::{AppConfig, MediaStorageConfig};
use crate::security::authenticator::AccountAuthenticator;
use crate::security::cookies::AuthCookieManager;
use crate::security::resource_access::ResourceAccessService;
use crate::system_provider_registry::build_system_provider_registry;

#[derive(Clone)]
pub(crate) struct AppMediaStore {
    inner: DynObjectStore,
    browser_media_base_url: String,
    model_media_base_url: String,
    resource_token_service: Arc<ResourceTokenService>,
    media_objects: Arc<MediaObjectStore>,
}

impl AppMediaStore {
    fn new(
        inner: DynObjectStore,
        browser_media_base_url: String,
        model_media_base_url: String,
        resource_token_service: Arc<ResourceTokenService>,
        media_objects: Arc<MediaObjectStore>,
    ) -> Self {
        Self {
            inner,
            browser_media_base_url,
            model_media_base_url,
            resource_token_service,
            media_objects,
        }
    }

    pub async fn get_bytes(&self, key: &str) -> anyhow::Result<Option<RetrievedObject>> {
        self.inner.get_bytes(key).await
    }

    pub async fn put_owned_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
        owner_user_id: &str,
        session_id: Option<&str>,
    ) -> Result<StoredMedia, ChatServiceError> {
        let stored = self
            .inner
            .put_bytes(key, bytes, content_type)
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;
        self.media_objects
            .upsert_media_object(MediaObjectRecord {
                object_key: stored.key.clone(),
                user_id: owner_user_id.to_string(),
                session_id: session_id.map(str::to_string),
            })
            .await
            .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

        Ok(StoredMedia {
            key: stored.key.clone(),
            browser_url: self.browser_media_url(stored.key.as_str()),
            model_url: self.signed_model_media_url(stored.key.as_str()),
            content_type: stored.content_type,
            size_bytes: stored.size_bytes,
        })
    }

    pub async fn get_media_owner(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .media_objects
            .get_media_object(key)
            .await?
            .map(|media| media.user_id))
    }

    pub fn browser_media_url(&self, key: &str) -> String {
        let normalized = key.replace('\\', "/").trim_start_matches('/').to_string();
        format!(
            "{}/{}",
            self.browser_media_base_url.trim_end_matches('/'),
            normalized
        )
    }

    pub fn signed_model_media_url(&self, key: &str) -> String {
        let normalized = key.replace('\\', "/").trim_start_matches('/').to_string();
        let signature = self
            .resource_token_service
            .sign("media", normalized.as_str(), 15 * 60);
        format!(
            "{}/{}?sig={}",
            self.model_media_base_url.trim_end_matches('/'),
            normalized,
            signature
        )
    }

    pub fn verify_media_signature(&self, key: &str, signature: &str) -> bool {
        self.resource_token_service.verify(
            "media",
            key.replace('\\', "/").trim_start_matches('/'),
            signature,
        )
    }
}

impl MediaStore for AppMediaStore {
    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
        owner_user_id: &'a str,
        session_id: Option<&'a str>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<StoredMedia, ChatServiceError>> + Send + 'a>,
    > {
        Box::pin(async move { self.put_owned_bytes(key, bytes, content_type, owner_user_id, session_id).await })
    }
}

impl ModelMediaUrlResolver for AppMediaStore {
    fn resolve_model_url(&self, media_id: &str, fallback_url: &str) -> String {
        if media_id.trim().is_empty() {
            return fallback_url.to_string();
        }
        self.signed_model_media_url(media_id)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub account_service: Arc<AccountService>,
    pub authenticator: Arc<dyn AccessTokenAuthenticator>,
    pub auth_cookies: Arc<AuthCookieManager>,
    pub resource_access: Arc<ResourceAccessService>,
    pub catalog_service: Arc<CatalogService>,
    pub chat_service: Arc<ChatService>,
    pub tool_access_service: Arc<ToolAccessService<AccountService>>,
    pub media_store: Arc<AppMediaStore>,
}

impl AppState {
    pub async fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let database = Database::connect(config.database_url.as_str()).await?;
        let pool = Arc::new(database.pool().clone());
        let auth_store = Arc::new(AuthStore::new(pool.clone()));
        auth_store.seed_demo_user().await?;

        let catalog_store = CatalogStore::new(pool.clone());
        catalog_store
            .sync_from_file(config.catalog_path.as_str())
            .await?;
        let models: Vec<CatalogModel> = catalog_store
            .list_models()
            .await?
            .into_iter()
            .map(map_catalog_model)
            .collect();
        let reserved_custom_model_names: HashSet<String> = models
            .iter()
            .flat_map(|model| {
                [
                    model.model.as_str(),
                    model.display_name.as_str(),
                    model.model_config_id.as_str(),
                ]
            })
            .map(|value| value.trim().to_lowercase())
            .collect::<std::collections::HashSet<_>>();
        let tools = catalog_store
            .list_tools()
            .await?
            .into_iter()
            .map(map_catalog_tool)
            .collect();
        let user_provider_api_key_store = Arc::new(UserProviderApiKeyStore::new(
            pool.clone(),
            config.provider_secret_key.as_str(),
        ));
        let custom_model_store = Arc::new(CustomModelStore::new(
            pool.clone(),
            config.provider_secret_key.as_str(),
        ));
        let chat_store = Arc::new(ChatStore::new(pool.clone()));
        let media_object_store = Arc::new(MediaObjectStore::new(pool));
        let resource_token_service = Arc::new(ResourceTokenService::new(
            config.auth_secret_key.as_str(),
        ));
        let storage_config = build_storage_config(config);
        let object_store = build_object_store(&storage_config).await?;
        let public_base_url = config
            .public_base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}", config.bind_addr));
        let media_store = Arc::new(AppMediaStore::new(
            object_store,
            "/api/media".to_string(),
            format!("{}/api/media", public_base_url.trim_end_matches('/')),
            resource_token_service,
            media_object_store,
        ));
        let session_store = Arc::new(InMemorySessionStore::new());
        let active_turns = Arc::new(ActiveTurnRegistry::new());
        let system_provider_registry = build_system_provider_registry(config);
        let model_provider_service = Arc::new(ModelProviderService::new(
            user_provider_api_key_store,
            system_provider_registry,
        ));
        let auth_service = Arc::new(AuthService::new(
            auth_store,
            config.auth_secret_key.as_str(),
        ));
        let custom_model_service = Arc::new(CustomModelService::new(
            custom_model_store,
            reserved_custom_model_names,
        ));
        let account_service = Arc::new(AccountService::new(
            auth_service.clone(),
            model_provider_service.clone(),
            custom_model_service,
        ));
        let authenticator: Arc<dyn AccessTokenAuthenticator> =
            Arc::new(AccountAuthenticator::new(account_service.clone()));
        let auth_cookies = Arc::new(AuthCookieManager::new(
            config.auth_cookie_secure,
            config.auth_cookie_domain.clone(),
        ));
        let authorizer: Arc<dyn Authorizer> = Arc::new(OwnershipAuthorizer::new());
        let resource_access = Arc::new(ResourceAccessService::new(
            authorizer.clone(),
            chat_store.clone(),
        ));
        let model_runtime = ModelRuntime::new(media_store.clone());
        let image_runtime = ImageRuntime::new();
        let provider_runtime = ModelProviderRuntime::new(account_service.clone(), model_runtime);
        let image_provider_runtime =
            ImageProviderRuntime::new(account_service.clone(), image_runtime);
        let tool_access_service = Arc::new(ToolAccessService::new(account_service.clone()));
        let tool_executor = ToolExecutor::new(
            image_provider_runtime,
            tool_access_service.as_ref().clone(),
            media_store.clone(),
        );
        let runtime = Arc::new(OpenChatTurnExecutor::new(
            chat_store.clone(),
            provider_runtime,
            tool_executor,
        ));
        let catalog_service = Arc::new(CatalogService::new(models, tools));
        let turn_builder = Arc::new(CatalogTurnBuilder::new(catalog_service.as_ref().clone()));
        let chat_service = Arc::new(ChatService::new(
            session_store,
            active_turns,
            chat_store,
            turn_builder,
            runtime,
        ));

        Ok(Self {
            account_service,
            authenticator,
            auth_cookies,
            resource_access,
            catalog_service,
            chat_service,
            tool_access_service,
            media_store,
        })
    }
}

fn build_storage_config(config: &AppConfig) -> StorageBackendConfig {
    match &config.media_storage {
        MediaStorageConfig::Local {
            root_dir,
            public_base_url,
        } => StorageBackendConfig::Local(LocalStorageConfig {
            root_dir: root_dir.clone(),
            public_base_url: public_base_url.clone(),
        }),
        MediaStorageConfig::S3 {
            bucket,
            region,
            endpoint,
            access_key_id,
            secret_access_key,
            public_base_url,
            force_path_style,
        } => StorageBackendConfig::S3(S3StorageConfig {
            bucket: bucket.clone(),
            region: region.clone(),
            endpoint: endpoint.clone(),
            access_key_id: access_key_id.clone(),
            secret_access_key: secret_access_key.clone(),
            public_base_url: public_base_url.clone(),
            force_path_style: *force_path_style,
        }),
    }
}

fn map_catalog_model(record: CatalogModelRecord) -> CatalogModel {
    CatalogModel {
        model_config_id: record.model_config_id,
        provider: record.provider,
        runtime_provider: record.runtime_provider,
        display_provider: record.display_provider,
        model: record.model,
        display_name: record.display_name,
        source: record.source,
        model_type: record.model_type,
        input_modalities: record.input_modalities,
        official: record.official,
        custom: record.custom,
    }
}

fn map_catalog_tool(record: CatalogToolRecord) -> openchat_core::CatalogTool {
    openchat_core::CatalogTool {
        model_config_id: record.model_config_id,
        model_name: record.model,
        id: record.id,
        provider: record.provider,
        runtime_provider: record.runtime_provider,
        display_provider: record.display_provider,
        source: record.source,
        tool_type: record.tool_type,
        display_name: record.display_name,
    }
}
