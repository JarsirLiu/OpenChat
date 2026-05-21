use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::fs;

use super::{ObjectStore, RetrievedObject, StoredObject};

#[derive(Clone, Debug)]
pub struct LocalObjectStore {
    root_dir: PathBuf,
    public_base_url: String,
}

impl LocalObjectStore {
    pub async fn new(root_dir: String, public_base_url: String) -> Result<Self> {
        let root_path = PathBuf::from(root_dir);
        fs::create_dir_all(&root_path).await.with_context(|| {
            format!("failed to create local media root {}", root_path.display())
        })?;

        Ok(Self {
            root_dir: root_path,
            public_base_url: normalize_public_base_url(public_base_url),
        })
    }

    fn resolve_path(&self, key: &str) -> PathBuf {
        let sanitized = key.replace('\\', "/");
        self.root_dir.join(Path::new(&sanitized))
    }

    fn public_url_for_key(&self, key: &str) -> String {
        format!(
            "{}/{}",
            self.public_base_url.trim_end_matches('/'),
            key.replace('\\', "/").trim_start_matches('/')
        )
    }
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<StoredObject> {
        let path = self.resolve_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create local media directory {}",
                    parent.display()
                )
            })?;
        }

        fs::write(&path, &bytes)
            .await
            .with_context(|| format!("failed to write local media file {}", path.display()))?;

        Ok(StoredObject {
            key: key.to_string(),
            public_url: self.public_url_for_key(key),
            content_type: content_type.to_string(),
            size_bytes: bytes.len(),
        })
    }

    async fn get_bytes(&self, key: &str) -> Result<Option<RetrievedObject>> {
        let path = self.resolve_path(key);
        let bytes = match fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read local media file {}", path.display()))
            }
        };

        Ok(Some(RetrievedObject {
            key: key.to_string(),
            content_type: infer_content_type(key),
            size_bytes: bytes.len(),
            bytes,
        }))
    }

    async fn delete_object(&self, key: &str) -> Result<bool> {
        let path = self.resolve_path(key);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error)
                .with_context(|| format!("failed to delete local media file {}", path.display())),
        }
    }
}

fn normalize_public_base_url(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "/media".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn infer_content_type(key: &str) -> String {
    let normalized = key.to_ascii_lowercase();
    if normalized.ends_with(".png") {
        "image/png".to_string()
    } else if normalized.ends_with(".jpg") || normalized.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if normalized.ends_with(".webp") {
        "image/webp".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}
