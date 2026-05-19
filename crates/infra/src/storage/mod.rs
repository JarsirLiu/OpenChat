mod local;
mod s3;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

pub use local::LocalObjectStore;
pub use s3::S3CompatibleObjectStore;

#[derive(Clone, Debug)]
pub struct StoredObject {
    pub key: String,
    pub public_url: String,
    pub content_type: String,
    pub size_bytes: usize,
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<StoredObject>;
}

pub type DynObjectStore = Arc<dyn ObjectStore>;

#[derive(Clone, Debug)]
pub enum StorageBackendConfig {
    Local(LocalStorageConfig),
    S3(S3StorageConfig),
}

#[derive(Clone, Debug)]
pub struct LocalStorageConfig {
    pub root_dir: String,
    pub public_base_url: String,
}

#[derive(Clone, Debug)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub public_base_url: String,
    pub force_path_style: bool,
}

pub async fn build_object_store(config: &StorageBackendConfig) -> Result<DynObjectStore> {
    match config {
        StorageBackendConfig::Local(local) => Ok(Arc::new(
            LocalObjectStore::new(local.root_dir.clone(), local.public_base_url.clone()).await?,
        )),
        StorageBackendConfig::S3(s3) => {
            Ok(Arc::new(S3CompatibleObjectStore::new(s3.clone()).await?))
        }
    }
}
