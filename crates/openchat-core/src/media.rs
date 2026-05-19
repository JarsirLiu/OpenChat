use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use crate::ChatServiceError;

#[derive(Clone, Debug)]
pub struct StoredMedia {
    pub key: String,
    pub public_url: String,
    pub content_type: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub kind: String,
    pub url: String,
    pub mime_type: String,
    pub size_bytes: usize,
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

pub trait MediaStore: Send + Sync {
    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
    ) -> PutMediaFuture<'a>;
}
