use std::{future::Future, io::Cursor, pin::Pin};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ChatServiceError;

#[derive(Clone, Debug)]
pub struct StoredMedia {
    pub key: String,
    pub browser_url: String,
    pub model_url: String,
    pub content_type: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct RetrievedMedia {
    pub key: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub kind: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_key: Option<String>,
    pub mime_type: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct NormalizedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

pub fn parse_media_assets_json(raw: Option<&str>) -> Vec<MediaAsset> {
    raw.filter(|value| !value.trim().is_empty())
        .and_then(|value| serde_json::from_str::<Vec<MediaAsset>>(value).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|asset| !asset.url.trim().is_empty())
        .collect()
}

pub type PutMediaFuture<'a> =
    Pin<Box<dyn Future<Output = Result<StoredMedia, ChatServiceError>> + Send + 'a>>;

pub fn normalize_generated_image_bytes(
    bytes: &[u8],
    source_label: &str,
) -> Result<NormalizedImage, ChatServiceError> {
    let image = image::load_from_memory(bytes).map_err(|error| {
        ChatServiceError::new(
            502,
            format!("Image provider returned an invalid {source_label} image payload: {error}"),
        )
    })?;

    let width = image.width();
    let height = image.height();
    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|error| {
            ChatServiceError::new(
                502,
                format!("Failed to normalize generated image payload: {error}"),
            )
        })?;

    let bytes = buffer.into_inner();
    if bytes.is_empty() {
        return Err(ChatServiceError::new(
            502,
            "Image provider returned an image that became empty after normalization",
        ));
    }

    Ok(NormalizedImage {
        bytes,
        mime_type: "image/png".to_string(),
        width,
        height,
    })
}

#[async_trait]
pub trait ModelMediaUrlResolver: Send + Sync {
    async fn resolve_model_url(&self, media_id: &str, fallback_url: &str) -> String;
}

pub trait MediaStore: Send + Sync {
    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
        owner_user_id: &'a str,
        session_id: Option<&'a str>,
    ) -> PutMediaFuture<'a>;

    fn get_bytes<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<RetrievedMedia>, ChatServiceError>> + Send + 'a>>;
}
