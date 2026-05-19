use std::sync::Arc;

use openchat_account_core::{
    AccountService, AuthService, CustomModelService, ModelProviderService,
};
use openchat_catalog_core::{CatalogModel, CatalogService, CatalogTurnBuilder};
use openchat_core::{
    ActiveTurnRegistry, ChatService, ChatServiceError, ImageProviderRuntime, ImageRuntime,
    InMemorySessionStore, MediaStore, ModelProviderRuntime, ModelRuntime, OpenChatTurnExecutor, StoredMedia,
    ToolAccessService, ToolExecutor,
};
use openchat_infra::sqlite::{CatalogModelRecord, CatalogToolRecord, Database, SqliteCatalogStore};
use openchat_infra::sqlite::{
    SqliteAuthStore, SqliteChatStore, SqliteCustomModelStore, SqliteProviderSettingsStore,
};
use openchat_infra::storage::{
    build_object_store, DynObjectStore, LocalStorageConfig, S3StorageConfig, StorageBackendConfig,
};

use crate::config::{AppConfig, MediaStorageConfig};

#[derive(Clone)]
pub(crate) struct AppMediaStore {
    inner: DynObjectStore,
}

impl AppMediaStore {
    fn new(inner: DynObjectStore) -> Self {
        Self { inner }
    }
}

impl MediaStore for AppMediaStore {
    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<StoredMedia, ChatServiceError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let stored = self
                .inner
                .put_bytes(key, bytes, content_type)
                .await
                .map_err(|error| ChatServiceError::new(500, error.to_string()))?;

            Ok(StoredMedia {
                key: stored.key,
                public_url: stored.public_url,
                content_type: stored.content_type,
                size_bytes: stored.size_bytes,
            })
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub account_service: Arc<AccountService>,
    pub catalog_service: Arc<CatalogService>,
    pub chat_service: Arc<ChatService>,
    pub tool_access_service: Arc<ToolAccessService<AccountService>>,
    pub media_storage: MediaStorageConfig,
    pub media_store: Arc<AppMediaStore>,
}

impl AppState {
    pub async fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let database = Database::connect(config.database_url.as_str()).await?;
        let pool = Arc::new(database.pool().clone());
        let auth_store = Arc::new(SqliteAuthStore::new(pool.clone()));
        auth_store.seed_demo_user().await?;

        let catalog_store = SqliteCatalogStore::new(pool.clone());
        catalog_store
            .sync_from_file(config.catalog_path.as_str())
            .await?;
        let models = catalog_store
            .list_models()
            .await?
            .into_iter()
            .map(map_catalog_model)
            .collect();
        let tools = catalog_store
            .list_tools()
            .await?
            .into_iter()
            .map(map_catalog_tool)
            .collect();
        let provider_settings_store = Arc::new(SqliteProviderSettingsStore::new(
            pool.clone(),
            config.provider_secret_key.as_str(),
        ));
        let custom_model_store = Arc::new(SqliteCustomModelStore::new(
            pool.clone(),
            config.provider_secret_key.as_str(),
        ));
        let chat_store = Arc::new(SqliteChatStore::new(pool));
        let storage_config = build_storage_config(config);
        let object_store = build_object_store(&storage_config).await?;
        let media_store = Arc::new(AppMediaStore::new(object_store));
        let session_store = Arc::new(InMemorySessionStore::new());
        let active_turns = Arc::new(ActiveTurnRegistry::new());
        let model_provider_service = Arc::new(ModelProviderService::new(
            provider_settings_store,
            std::collections::HashMap::new(),
        ));
        let auth_service = Arc::new(AuthService::new(auth_store));
        let custom_model_service = Arc::new(CustomModelService::new(custom_model_store));
        let account_service = Arc::new(AccountService::new(
            auth_service.clone(),
            model_provider_service.clone(),
            custom_model_service,
        ));
        let model_runtime = ModelRuntime::new();
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
            catalog_service,
            chat_service,
            tool_access_service,
            media_storage: config.media_storage.clone(),
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
