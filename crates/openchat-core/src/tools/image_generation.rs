use anyhow::Context;
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    ChatServiceError, ImageModelAccessResolver, ImageProviderRuntime, MediaAsset, MediaStore,
    ResolvedImageModelAccess,
};

use super::context::{ToolExecutionResult, ToolInvocation};

#[derive(Clone)]
pub struct ImageRuntime {
    client: Client,
}

#[derive(Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

pub struct ImageGenerationToolHandler<R> {
    runtime: ImageProviderRuntime<R>,
    media_store: Arc<dyn MediaStore>,
}

impl<R> Clone for ImageGenerationToolHandler<R> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            media_store: self.media_store.clone(),
        }
    }
}

impl ImageRuntime {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub(crate) async fn generate_image(
        &self,
        request: &ImageGenerationRequest,
        access: &ResolvedImageModelAccess,
    ) -> Result<Vec<GeneratedImage>, ChatServiceError> {
        let url = format!(
            "{}/images/generations",
            access.base_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(url)
            .bearer_auth(access.api_key.as_str())
            .json(&OpenAiImageRequest {
                model: access.model_name.clone(),
                prompt: request.prompt.trim().to_string(),
                size: request.size.clone(),
                n: request.count,
                response_format: "b64_json".to_string(),
                quality: request.quality.clone(),
                background: request.background.clone(),
            })
            .send()
            .await
            .with_context(|| format!("failed to call image provider `{}`", access.provider_key))
            .map_err(map_runtime_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChatServiceError::new(
                502,
                format!(
                    "Image provider `{}` request failed: {status} {body}",
                    access.provider_key
                ),
            ));
        }

        let payload: OpenAiImageResponse = response
            .json()
            .await
            .context("invalid image provider response")
            .map_err(map_runtime_error)?;

        if payload.data.is_empty() {
            return Err(ChatServiceError::new(
                502,
                "Image provider returned no image data",
            ));
        }

        let mut generated_images = Vec::with_capacity(payload.data.len());
        for image in payload.data {
            if let Some(b64_json) = image.b64_json {
                let encoded = b64_json
                    .split_once(',')
                    .map(|(_, value)| value)
                    .unwrap_or(b64_json.as_str());

                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .context("invalid image base64")
                    .map_err(map_runtime_error)?;

                generated_images.push(GeneratedImage {
                    bytes,
                    mime_type: "image/png".to_string(),
                });
                continue;
            }

            if let Some(url) = image.url.filter(|value| !value.trim().is_empty()) {
                let response = self
                    .client
                    .get(url)
                    .send()
                    .await
                    .with_context(|| "failed to download image output")
                    .map_err(map_runtime_error)?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(ChatServiceError::new(
                        502,
                        format!("Image output download failed: {status} {body}"),
                    ));
                }

                let mime_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("image/png")
                    .to_string();

                let bytes = response
                    .bytes()
                    .await
                    .context("failed to read image bytes")
                    .map_err(map_runtime_error)?
                    .to_vec();

                generated_images.push(GeneratedImage { bytes, mime_type });
                continue;
            }
        }

        if generated_images.is_empty() {
            return Err(ChatServiceError::new(
                502,
                "Image provider returned no usable image output",
            ));
        }

        Ok(generated_images)
    }
}

impl<R> ImageGenerationToolHandler<R>
where
    R: ImageModelAccessResolver,
{
    pub fn new(runtime: ImageProviderRuntime<R>, media_store: Arc<dyn MediaStore>) -> Self {
        Self {
            runtime,
            media_store,
        }
    }

    pub async fn execute(
        &self,
        invocation: ToolInvocation,
    ) -> Result<ToolExecutionResult, ChatServiceError> {
        let request = parse_image_generation_request(invocation.arguments_text.as_str())?;
        let generated_images = self
            .runtime
            .generate_image(invocation.user_id.as_str(), &invocation.tool, &request)
            .await?;

        let mut stored_images = Vec::with_capacity(generated_images.len());
        for (index, generated) in generated_images.into_iter().enumerate() {
            let stored = self
                .media_store
                .put_bytes(
                    &build_media_key(
                        invocation.user_id.as_str(),
                        invocation.session_id.as_str(),
                        invocation.turn_id.as_str(),
                        invocation.tool_call_id.as_str(),
                        index,
                        generated.mime_type.as_str(),
                    ),
                    generated.bytes,
                    generated.mime_type.as_str(),
                    invocation.user_id.as_str(),
                    Some(invocation.session_id.as_str()),
                )
                .await?;
            stored_images.push(stored);
        }

        let media = stored_images
            .iter()
            .map(|stored| MediaAsset {
                kind: "image".to_string(),
                url: stored.browser_url.clone(),
                object_key: Some(stored.key.clone()),
                mime_type: stored.content_type.clone(),
                size_bytes: stored.size_bytes,
            })
            .collect::<Vec<_>>();

        Ok(ToolExecutionResult {
            media: media.clone(),
            result: json!({
                "kind": "image",
                "message": format!("Generated {} image(s) successfully.", media.len()),
                "prompt": request.prompt,
                "provider": invocation.tool.provider,
                "model": invocation.tool.model_name,
                "count": media.len(),
            }),
        })
    }
}

#[derive(Debug)]
pub(crate) struct ImageGenerationRequest {
    prompt: String,
    size: String,
    count: u32,
    quality: Option<String>,
    background: Option<String>,
}

#[derive(Deserialize)]
struct ImageGenerationArguments {
    prompt: String,
    size: Option<String>,
    aspect_ratio: Option<String>,
    quality: Option<String>,
    background: Option<String>,
    count: Option<u32>,
}

fn parse_image_generation_request(raw: &str) -> Result<ImageGenerationRequest, ChatServiceError> {
    let args: ImageGenerationArguments = serde_json::from_str(raw).map_err(|error| {
        ChatServiceError::new(400, format!("Invalid image tool arguments: {error}"))
    })?;

    let prompt = args.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(ChatServiceError::new(
            400,
            "Image tool requires a non-empty prompt",
        ));
    }

    Ok(ImageGenerationRequest {
        prompt,
        size: args
            .size
            .or_else(|| args.aspect_ratio.as_deref().map(size_from_aspect_ratio))
            .unwrap_or_else(|| "1024x1024".to_string()),
        count: args.count.unwrap_or(1).clamp(1, 4),
        quality: args.quality.filter(|value| !value.trim().is_empty()),
        background: args.background.filter(|value| !value.trim().is_empty()),
    })
}

fn size_from_aspect_ratio(ratio: &str) -> String {
    match ratio.trim() {
        "16:9" => "1536x1024".to_string(),
        "9:16" => "1024x1536".to_string(),
        "4:3" => "1408x1056".to_string(),
        "3:4" => "1056x1408".to_string(),
        _ => "1024x1024".to_string(),
    }
}

fn build_media_key(
    user_id: &str,
    session_id: &str,
    turn_id: &str,
    tool_call_id: &str,
    image_index: usize,
    mime_type: &str,
) -> String {
    let extension = match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };

    format!(
        "media/users/{user_id}/sessions/{session_id}/turns/{turn_id}/tools/{tool_call_id}_{image_index}.{extension}"
    )
}

fn map_runtime_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::new(502, error.to_string())
}

#[derive(Serialize)]
struct OpenAiImageRequest {
    model: String,
    prompt: String,
    size: String,
    n: u32,
    response_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiImageResponse {
    #[serde(default)]
    data: Vec<OpenAiImageData>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    #[serde(default)]
    b64_json: Option<String>,
    #[serde(default)]
    url: Option<String>,
}
