use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};

use super::{ObjectStore, RetrievedObject, S3StorageConfig, StoredObject};

#[derive(Clone)]
pub struct S3CompatibleObjectStore {
    bucket: String,
    client: Client,
    public_base_url: String,
}

impl S3CompatibleObjectStore {
    pub async fn new(config: S3StorageConfig) -> Result<Self> {
        let credentials = Credentials::new(
            config.access_key_id,
            config.secret_access_key,
            None,
            None,
            "openchat-config",
        );

        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(config.region.clone()))
            .force_path_style(config.force_path_style);

        if let Some(endpoint) = config
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            builder = builder.endpoint_url(endpoint);
        }

        let client = Client::from_conf(builder.build());

        Ok(Self {
            bucket: config.bucket,
            client,
            public_base_url: config.public_base_url.trim_end_matches('/').to_string(),
        })
    }

    fn public_url_for_key(&self, key: &str) -> String {
        format!(
            "{}/{}",
            self.public_base_url,
            key.replace('\\', "/").trim_start_matches('/')
        )
    }
}

#[async_trait]
impl ObjectStore for S3CompatibleObjectStore {
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<StoredObject> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(bytes.clone()))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to upload object `{key}` to bucket `{}`",
                    self.bucket
                )
            })?;

        Ok(StoredObject {
            key: key.to_string(),
            public_url: self.public_url_for_key(key),
            content_type: content_type.to_string(),
            size_bytes: bytes.len(),
        })
    }

    async fn get_bytes(&self, key: &str) -> Result<Option<RetrievedObject>> {
        let response = match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let message = error.to_string();
                if message.contains("NoSuchKey") || message.contains("not found") {
                    return Ok(None);
                }
                return Err(anyhow::anyhow!(error)).with_context(|| {
                    format!(
                        "failed to download object `{key}` from bucket `{}`",
                        self.bucket
                    )
                });
            }
        };

        let content_type = response
            .content_type()
            .map(str::to_string)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let bytes = response.body.collect().await?.into_bytes().to_vec();

        Ok(Some(RetrievedObject {
            key: key.to_string(),
            size_bytes: bytes.len(),
            content_type,
            bytes,
        }))
    }
}
