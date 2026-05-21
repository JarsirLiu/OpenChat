use anyhow::Context;
use base64::Engine;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

use crate::{
    normalize_generated_image_bytes, ChatServiceError, ImageModelAccessResolver,
    ImageProviderRuntime, ImageToolDefaults, MediaAsset, MediaStore, OutboundContentPart,
    OutboundMessage, ResolvedImageModelAccess, TurnAttachment, TurnToolRef,
};

use super::context::{ToolExecutionResult, ToolInvocation};

const SUPPORTED_IMAGE_SIZES: &[&str] = &[
    "1024x1024",
    "1536x1024",
    "1024x1536",
    "1408x1056",
    "1056x1408",
    "2048x1152",
    "1152x2048",
    "2048x2048",
    "2048x1024",
    "1024x2048",
    "3840x2160",
    "2160x3840",
    "4096x4096",
    "4096x2048",
    "2048x4096",
];
const FALLBACK_DEFAULT_SIZE: &str = "1024x1024";
const FALLBACK_DEFAULT_QUALITY: &str = "auto";
const FALLBACK_DEFAULT_N: u32 = 1;
const UPSTREAM_RETRY_DELAY: Duration = Duration::from_millis(350);
const MAX_SEND_ATTEMPTS: usize = 2;

#[derive(Clone)]
pub struct ImageRuntime {
    client: Client,
}

#[derive(Clone, Debug)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

pub struct ImageGenerationToolHandler<R> {
    client: Client,
    runtime: ImageProviderRuntime<R>,
    media_store: Arc<dyn MediaStore>,
}

impl<R> Clone for ImageGenerationToolHandler<R> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
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
        debug!(
            provider = access.provider_key.as_str(),
            model = access.model_name.as_str(),
            base_url = access.base_url.as_str(),
            operation = "generate",
            size = request.size.as_str(),
            count = request.count,
            quality = request.quality.as_deref().unwrap_or("none"),
            prompt_preview = %truncate_for_log(request.prompt.as_str(), 120),
            "sending image generation request"
        );

        let request_body = OpenAiImageRequest {
            model: access.model_name.clone(),
            prompt: request.prompt.trim().to_string(),
            size: request.size.clone(),
            n: request.count,
            response_format: Some("b64_json".to_string()),
            quality: request.quality.clone(),
        };

        let response = send_image_generation_request_with_retry(
            &self.client,
            url.as_str(),
            access,
            &request_body,
        )
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                provider = access.provider_key.as_str(),
                model = access.model_name.as_str(),
                base_url = access.base_url.as_str(),
                operation = "generate",
                status_code = status.as_u16(),
                response_summary = %summarize_openai_image_response_body(body.as_str()),
                "image provider returned a non-success generation response"
            );
            return Err(map_provider_response_error(
                access.provider_key.as_str(),
                status.as_u16(),
                body,
            ));
        }

        decode_openai_image_response(
            response,
            access.provider_key.as_str(),
            access.model_name.as_str(),
        )
        .await
    }

    pub(crate) async fn edit_image(
        &self,
        request: &ImageGenerationRequest,
        inputs: &[ResolvedToolImageInput],
        access: &ResolvedImageModelAccess,
    ) -> Result<Vec<GeneratedImage>, ChatServiceError> {
        let url = format!("{}/images/edits", access.base_url.trim_end_matches('/'));
        debug!(
            provider = access.provider_key.as_str(),
            model = access.model_name.as_str(),
            base_url = access.base_url.as_str(),
            operation = "edit",
            size = request.size.as_str(),
            count = request.count,
            quality = request.quality.as_deref().unwrap_or("none"),
            input_count = inputs.len(),
            prompt_preview = %truncate_for_log(request.prompt.as_str(), 120),
            "sending image edit request"
        );
        let response =
            send_image_edit_request_with_retry(&self.client, url.as_str(), access, request, inputs)
                .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                provider = access.provider_key.as_str(),
                model = access.model_name.as_str(),
                base_url = access.base_url.as_str(),
                operation = "edit",
                status_code = status.as_u16(),
                response_summary = %summarize_openai_image_response_body(body.as_str()),
                "image provider returned a non-success edit response"
            );
            return Err(map_provider_response_error(
                access.provider_key.as_str(),
                status.as_u16(),
                body,
            ));
        }

        decode_openai_image_response(
            response,
            access.provider_key.as_str(),
            access.model_name.as_str(),
        )
        .await
    }
}

impl<R> ImageGenerationToolHandler<R>
where
    R: ImageModelAccessResolver,
{
    pub fn new(runtime: ImageProviderRuntime<R>, media_store: Arc<dyn MediaStore>) -> Self {
        Self {
            client: Client::new(),
            runtime,
            media_store,
        }
    }

    pub async fn execute(
        &self,
        invocation: ToolInvocation,
    ) -> Result<ToolExecutionResult, ChatServiceError> {
        let request =
            parse_image_generation_request(invocation.arguments_text.as_str(), &invocation.tool)?;
        let resolved_inputs = self.resolve_input_images(&invocation, &request).await?;
        let generated_images = self
            .runtime
            .generate_image(
                invocation.user_id.as_str(),
                &invocation.tool,
                &request,
                resolved_inputs.as_slice(),
            )
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
                "operation": request.operation.as_str(),
                "prompt": request.prompt,
                "provider": invocation.tool.provider,
                "model": invocation.tool.model_name,
                "count": media.len(),
                "inputImageCount": resolved_inputs.len(),
            }),
        })
    }

    async fn resolve_input_images(
        &self,
        invocation: &ToolInvocation,
        request: &ImageGenerationRequest,
    ) -> Result<Vec<ResolvedToolImageInput>, ChatServiceError> {
        let mut resolved = Vec::with_capacity(request.input_images.len());

        for input in &request.input_images {
            let value = input.trim();
            if value.is_empty() {
                continue;
            }

            if let Some(data_uri) = parse_data_uri(value)? {
                resolved.push(data_uri);
                continue;
            }

            if let Some(attachment) =
                find_current_attachment(value, invocation.current_attachments.as_slice())
            {
                resolved.push(self.load_owned_media(attachment.id.as_str(), value).await?);
                continue;
            }

            if let Some(media_id) = find_history_media_id(value, invocation.history.as_slice()) {
                resolved.push(self.load_owned_media(media_id.as_str(), value).await?);
                continue;
            }

            if looks_like_external_url(value) {
                resolved.push(self.download_remote_image(value).await?);
                continue;
            }

            if let Some(media) = self.media_store.get_bytes(value).await? {
                resolved.push(ResolvedToolImageInput {
                    bytes: media.bytes,
                    mime_type: media.content_type,
                });
                continue;
            }

            return Err(ChatServiceError::new(
                400,
                format!("Unsupported input image reference `{value}`"),
            ));
        }

        match request.operation {
            ImageToolOperation::Generate if !resolved.is_empty() => Err(ChatServiceError::new(
                400,
                "Generate operation does not accept input_images",
            )),
            ImageToolOperation::Reference if resolved.is_empty() => Err(ChatServiceError::new(
                400,
                "Reference image generation requires at least one input image",
            )),
            _ => Ok(resolved),
        }
    }

    async fn load_owned_media(
        &self,
        media_id: &str,
        reference: &str,
    ) -> Result<ResolvedToolImageInput, ChatServiceError> {
        let media = self.media_store.get_bytes(media_id).await?.ok_or_else(|| {
            ChatServiceError::new(404, format!("Input image `{reference}` was not found"))
        })?;

        Ok(ResolvedToolImageInput {
            bytes: media.bytes,
            mime_type: media.content_type,
        })
    }

    async fn download_remote_image(
        &self,
        url: &str,
    ) -> Result<ResolvedToolImageInput, ChatServiceError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to download input image `{url}`"))
            .map_err(map_runtime_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChatServiceError::new(
                400,
                format!("Input image download failed: {status} {body}"),
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
            .context("failed to read input image bytes")
            .map_err(map_runtime_error)?
            .to_vec();

        Ok(ResolvedToolImageInput { bytes, mime_type })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ImageToolOperation {
    Generate,
    Reference,
}

impl ImageToolOperation {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Reference => "reference",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedToolImageInput {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug)]
pub(crate) struct ImageGenerationRequest {
    pub(crate) operation: ImageToolOperation,
    prompt: String,
    size: String,
    count: u32,
    quality: Option<String>,
    input_images: Vec<String>,
}

#[derive(Deserialize)]
struct ImageGenerationArguments {
    prompt: String,
    size: Option<String>,
    quality: Option<String>,
    n: Option<u32>,
    #[serde(default)]
    input_images: Option<ImageInputValue>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImageInputValue {
    One(String),
    Many(Vec<String>),
}

const MAX_IMAGE_COUNT: u32 = 8;

fn parse_image_generation_request(
    raw: &str,
    tool: &TurnToolRef,
) -> Result<ImageGenerationRequest, ChatServiceError> {
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

    let input_images = match args.input_images {
        Some(ImageInputValue::One(value)) => vec![value],
        Some(ImageInputValue::Many(values)) => values,
        None => Vec::new(),
    };
    let operation = if input_images.is_empty() {
        ImageToolOperation::Generate
    } else {
        ImageToolOperation::Reference
    };
    let defaults = image_tool_defaults(tool.image_defaults.as_ref())?;
    let size = normalize_requested_size(args.size, defaults.size.as_str())?;
    let count = args.n.unwrap_or(defaults.n).clamp(1, MAX_IMAGE_COUNT);

    Ok(ImageGenerationRequest {
        operation,
        prompt,
        size,
        count,
        quality: Some(normalize_quality(args.quality, defaults.quality.as_str())?),
        input_images,
    })
}

pub(crate) fn supported_image_size_description(default_size: &str) -> String {
    let mut values = vec!["auto".to_string()];
    values.extend(SUPPORTED_IMAGE_SIZES.iter().map(|value| value.to_string()));
    format!(
        "Optional. Output size such as {}. Defaults to {default_size}. Use explicit dimensions for predictable results.",
        values.join(", ")
    )
}

fn normalize_requested_size(
    requested: Option<String>,
    default_size: &str,
) -> Result<String, ChatServiceError> {
    let value = requested
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_size)
        .to_ascii_lowercase();

    let normalized = match value.as_str() {
        "auto" => default_size,
        "1024" | "1024x1024" | "square" => "1024x1024",
        "1536x1024" | "landscape" => "1536x1024",
        "1024x1536" | "portrait" => "1024x1536",
        "1408x1056" => "1408x1056",
        "1056x1408" => "1056x1408",
        "2048x1152" => "2048x1152",
        "1152x2048" => "1152x2048",
        "2048x1024" => "2048x1024",
        "1024x2048" => "1024x2048",
        "3840x2160" => "3840x2160",
        "2160x3840" => "2160x3840",
        "4096x2048" => "4096x2048",
        "2048x4096" => "2048x4096",
        other if SUPPORTED_IMAGE_SIZES.iter().any(|size| size == &other) => other,
        other => {
            return Err(ChatServiceError::new(
                400,
                format!("Unsupported image size `{other}`"),
            ))
        }
    };

    Ok(normalized.to_string())
}

fn normalize_quality(
    requested: Option<String>,
    default_quality: &str,
) -> Result<String, ChatServiceError> {
    let value = requested
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_quality);

    let normalized = value.to_ascii_lowercase();
    match normalized.as_str() {
        "auto" | "low" | "medium" | "high" => Ok(normalized),
        other => Err(ChatServiceError::new(
            400,
            format!("Unsupported image quality `{other}`"),
        )),
    }
}

fn image_tool_defaults(
    defaults: Option<&ImageToolDefaults>,
) -> Result<ImageToolDefaults, ChatServiceError> {
    let defaults = defaults.cloned().unwrap_or(ImageToolDefaults {
        size: FALLBACK_DEFAULT_SIZE.to_string(),
        quality: FALLBACK_DEFAULT_QUALITY.to_string(),
        n: FALLBACK_DEFAULT_N,
    });

    Ok(ImageToolDefaults {
        size: normalize_requested_size(Some(defaults.size), FALLBACK_DEFAULT_SIZE)?,
        quality: normalize_quality(Some(defaults.quality), FALLBACK_DEFAULT_QUALITY)?,
        n: defaults.n.clamp(1, MAX_IMAGE_COUNT),
    })
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

fn build_input_file_name(index: usize, mime_type: &str) -> String {
    let extension = match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };

    format!("input_{index}.{extension}")
}

async fn decode_openai_image_response(
    response: reqwest::Response,
    provider_key: &str,
    model_name: &str,
) -> Result<Vec<GeneratedImage>, ChatServiceError> {
    let status = response.status().as_u16();
    let raw_body = response
        .text()
        .await
        .context("failed to read image provider response body")
        .map_err(map_runtime_error)?;

    debug!(
        provider = provider_key,
        model = model_name,
        status_code = status,
        response_summary = %summarize_openai_image_response_body(raw_body.as_str()),
        "received image provider response"
    );

    let payload: OpenAiImageResponse = serde_json::from_str(raw_body.as_str())
        .context("invalid image provider response")
        .map_err(map_runtime_error)?;

    decode_openai_image_payload(payload).await
}

async fn send_image_generation_request_with_retry(
    client: &Client,
    url: &str,
    access: &ResolvedImageModelAccess,
    body: &OpenAiImageRequest,
) -> Result<reqwest::Response, ChatServiceError> {
    for attempt in 1..=MAX_SEND_ATTEMPTS {
        let response = client
            .post(url)
            .bearer_auth(access.api_key.as_str())
            .json(body)
            .send()
            .await;

        match response {
            Ok(response) => return Ok(response),
            Err(error) if attempt < MAX_SEND_ATTEMPTS && should_retry_transport_error(&error) => {
                warn!(
                    provider = access.provider_key.as_str(),
                    model = access.model_name.as_str(),
                    base_url = access.base_url.as_str(),
                    operation = "generate",
                    attempt,
                    max_attempts = MAX_SEND_ATTEMPTS,
                    error = %error,
                    "retrying image generation request after transient transport failure"
                );
                sleep(UPSTREAM_RETRY_DELAY).await;
            }
            Err(error) => {
                warn!(
                    provider = access.provider_key.as_str(),
                    model = access.model_name.as_str(),
                    base_url = access.base_url.as_str(),
                    operation = "generate",
                    attempt,
                    max_attempts = MAX_SEND_ATTEMPTS,
                    error = %error,
                    debug_error = ?error,
                    "image generation request failed before receiving a response"
                );
                return Err(map_runtime_error(anyhow::Error::new(error).context(
                    format!("failed to call image provider `{}`", access.provider_key),
                )));
            }
        }
    }

    Err(ChatServiceError::upstream(
        "image generation request exhausted retry attempts",
    ))
}

async fn send_image_edit_request_with_retry(
    client: &Client,
    url: &str,
    access: &ResolvedImageModelAccess,
    request: &ImageGenerationRequest,
    inputs: &[ResolvedToolImageInput],
) -> Result<reqwest::Response, ChatServiceError> {
    for attempt in 1..=MAX_SEND_ATTEMPTS {
        let current_form = build_image_edit_form(access, request, inputs)?;

        let response = client
            .post(url)
            .bearer_auth(access.api_key.as_str())
            .multipart(current_form)
            .send()
            .await;

        match response {
            Ok(response) => return Ok(response),
            Err(error) if attempt < MAX_SEND_ATTEMPTS && should_retry_transport_error(&error) => {
                warn!(
                    provider = access.provider_key.as_str(),
                    model = access.model_name.as_str(),
                    base_url = access.base_url.as_str(),
                    operation = "edit",
                    attempt,
                    max_attempts = MAX_SEND_ATTEMPTS,
                    error = %error,
                    "retrying image edit request after transient transport failure"
                );
                sleep(UPSTREAM_RETRY_DELAY).await;
            }
            Err(error) => {
                warn!(
                    provider = access.provider_key.as_str(),
                    model = access.model_name.as_str(),
                    base_url = access.base_url.as_str(),
                    operation = "edit",
                    attempt,
                    max_attempts = MAX_SEND_ATTEMPTS,
                    error = %error,
                    debug_error = ?error,
                    "image edit request failed before receiving a response"
                );
                return Err(map_runtime_error(anyhow::Error::new(error).context(
                    format!("failed to call image provider `{}`", access.provider_key),
                )));
            }
        }
    }

    Err(ChatServiceError::upstream(
        "image edit request exhausted retry attempts",
    ))
}

fn build_image_edit_form(
    access: &ResolvedImageModelAccess,
    request: &ImageGenerationRequest,
    inputs: &[ResolvedToolImageInput],
) -> Result<Form, ChatServiceError> {
    let mut form = Form::new()
        .text("model", access.model_name.clone())
        .text("prompt", request.prompt.trim().to_string())
        .text("size", request.size.clone())
        .text("n", request.count.to_string());

    if let Some(quality) = request.quality.clone() {
        form = form.text("quality", quality);
    }

    for (index, input) in inputs.iter().enumerate() {
        let file_name = build_input_file_name(index, input.mime_type.as_str());
        let part = Part::bytes(input.bytes.clone())
            .file_name(file_name)
            .mime_str(input.mime_type.as_str())
            .map_err(|error| ChatServiceError::upstream(error.to_string()))?;
        form = form.part("image", part);
    }

    Ok(form)
}

fn should_retry_transport_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() || error.is_request() {
        let text = error.to_string().to_ascii_lowercase();
        return text.contains("unexpected eof")
            || text.contains("tls handshake eof")
            || text.contains("connection reset")
            || text.contains("connection aborted")
            || text.contains("http2")
            || text.contains("broken pipe")
            || text.contains("timed out");
    }

    false
}

async fn decode_openai_image_payload(
    payload: OpenAiImageResponse,
) -> Result<Vec<GeneratedImage>, ChatServiceError> {
    if let Some(error) = payload.error {
        let message = error
            .message
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Image provider returned an embedded error".to_string());
        let code_suffix = error
            .code
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!(" ({value})"))
            .unwrap_or_default();
        warn!(message = %message, code = %code_suffix, "image provider returned embedded error");
        return Err(ChatServiceError::new(
            502,
            format!("{message}{code_suffix}"),
        ));
    }

    if payload.data.is_empty() {
        return Err(ChatServiceError::new(
            502,
            "Image provider returned no image data",
        ));
    }

    let client = Client::new();
    let mut generated_images = Vec::with_capacity(payload.data.len());
    for image in payload.data {
        let OpenAiImageData {
            b64_json,
            url,
            revised_prompt,
        } = image;

        if let Some(b64_json) = b64_json {
            match decode_base64_generated_image(b64_json.as_str()) {
                Ok(Some(generated_image)) => {
                    generated_images.push(generated_image);
                    continue;
                }
                Ok(None) => {
                    warn!(
                        "image provider returned an empty base64 payload; falling back to URL output if available"
                    );
                }
                Err(error) => {
                    if url.as_deref().is_some_and(|value| !value.trim().is_empty()) {
                        warn!(
                            error = %error.message,
                            "image provider returned an unusable base64 payload; falling back to URL output"
                        );
                    } else {
                        return Err(error);
                    }
                }
            }
        }

        if let Some(url) = url.filter(|value| !value.trim().is_empty()) {
            debug!(image_url = %url, "downloading image output from provider URL");

            let response = client
                .get(&url)
                .send()
                .await
                .with_context(|| "failed to download image output")
                .map_err(map_runtime_error)?;

            let status = response.status();
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_string();

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                warn!(
                    image_url = %url,
                    status_code = status.as_u16(),
                    content_type = %content_type,
                    body_preview = %truncate_for_log(body.as_str(), 200),
                    "image output download returned a non-success response"
                );
                return Err(ChatServiceError::new(
                    502,
                    format!("Image output download failed: {status} {body}"),
                ));
            }

            let bytes = response
                .bytes()
                .await
                .context("failed to read image bytes")
                .map_err(map_runtime_error)?
                .to_vec();

            debug!(
                image_url = %url,
                status_code = status.as_u16(),
                content_type = %content_type,
                byte_len = bytes.len(),
                body_preview = %describe_downloaded_image_payload(bytes.as_slice(), content_type.as_str()),
                "downloaded image output from provider URL"
            );

            if bytes.is_empty() {
                return Err(ChatServiceError::new(
                    502,
                    "Image provider returned an empty downloaded image payload",
                ));
            }

            let normalized = normalize_generated_image_bytes(bytes.as_slice(), "downloaded")
                .map_err(|error| {
                    warn!(
                        image_url = %url,
                        status_code = status.as_u16(),
                        content_type = %content_type,
                        byte_len = bytes.len(),
                        body_preview = %describe_downloaded_image_payload(bytes.as_slice(), content_type.as_str()),
                        error = %error.message,
                        "downloaded provider URL payload could not be decoded as an image"
                    );
                    error
                })?;
            generated_images.push(GeneratedImage {
                bytes: normalized.bytes,
                mime_type: normalized.mime_type,
            });
            continue;
        }

        if let Some(revised_prompt) = revised_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Err(ChatServiceError::new(
                502,
                format!(
                    "Image provider returned no usable image output. Revised prompt: {revised_prompt}"
                ),
            ));
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

fn decode_base64_generated_image(
    b64_json: &str,
) -> Result<Option<GeneratedImage>, ChatServiceError> {
    let encoded = b64_json
        .split_once(',')
        .map(|(_, value)| value)
        .unwrap_or(b64_json)
        .trim();

    if encoded.is_empty() {
        return Ok(None);
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("invalid image base64")
        .map_err(map_runtime_error)?;

    if bytes.is_empty() {
        return Err(ChatServiceError::new(
            502,
            "Image provider returned an empty image payload",
        ));
    }

    let normalized = normalize_generated_image_bytes(bytes.as_slice(), "base64")?;
    Ok(Some(GeneratedImage {
        bytes: normalized.bytes,
        mime_type: normalized.mime_type,
    }))
}

fn describe_downloaded_image_payload(bytes: &[u8], content_type: &str) -> String {
    if looks_like_text_response(content_type, bytes) {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    format!("binary payload ({} bytes)", bytes.len())
}

fn looks_like_text_response(content_type: &str, bytes: &[u8]) -> bool {
    let normalized = content_type.trim().to_ascii_lowercase();
    if normalized.starts_with("text/")
        || normalized.contains("json")
        || normalized.contains("xml")
        || normalized.contains("html")
    {
        return true;
    }

    std::str::from_utf8(bytes).is_ok()
}

fn summarize_openai_image_response_body(raw_body: &str) -> String {
    let trimmed = raw_body.trim();
    if trimmed.is_empty() {
        return "empty body".to_string();
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return truncate_for_log(trimmed, 600);
    };

    let Some(object) = value.as_object() else {
        return truncate_for_log(trimmed, 600);
    };

    let data_summary = object
        .get("data")
        .and_then(|value| value.as_array())
        .map(|items| {
            items.iter()
                .enumerate()
                .map(|(index, item)| {
                    let Some(entry) = item.as_object() else {
                        return format!("item{index}=non_object");
                    };

                    let b64_len = entry
                        .get("b64_json")
                        .and_then(|value| value.as_str())
                        .map(|value| value.len())
                        .unwrap_or(0);
                    let has_url = entry
                        .get("url")
                        .and_then(|value| value.as_str())
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false);
                    let revised_prompt = entry
                        .get("revised_prompt")
                        .and_then(|value| value.as_str())
                        .map(|value| truncate_for_log(value, 80))
                        .unwrap_or_else(|| "-".to_string());

                    format!(
                        "item{index}{{b64_len={b64_len},has_url={has_url},revised_prompt={revised_prompt}}}"
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "none".to_string());

    let error_summary = object
        .get("error")
        .map(|error| truncate_for_log(error.to_string().as_str(), 200))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "keys={:?}; data={}; error={}",
        object.keys().collect::<Vec<_>>(),
        data_summary,
        error_summary
    )
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn parse_data_uri(value: &str) -> Result<Option<ResolvedToolImageInput>, ChatServiceError> {
    if !value.starts_with("data:") {
        return Ok(None);
    }

    let (metadata, encoded) = value
        .split_once(',')
        .ok_or_else(|| ChatServiceError::new(400, "Invalid data URI image input"))?;
    let mime_type = metadata
        .trim_start_matches("data:")
        .split(';')
        .next()
        .filter(|item| !item.trim().is_empty())
        .unwrap_or("image/png")
        .to_string();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| ChatServiceError::new(400, "Invalid base64 image data"))?;

    Ok(Some(ResolvedToolImageInput { bytes, mime_type }))
}

fn find_current_attachment<'a>(
    reference: &str,
    attachments: &'a [TurnAttachment],
) -> Option<&'a TurnAttachment> {
    attachments
        .iter()
        .find(|attachment| attachment.id == reference || attachment.url == reference)
}

fn find_history_media_id(reference: &str, history: &[OutboundMessage]) -> Option<String> {
    for message in history {
        for part in &message.content {
            match part {
                OutboundContentPart::ImageUrl { url, media_id } => {
                    if url == reference {
                        if let Some(media_id) = media_id {
                            return Some(media_id.clone());
                        }
                    }
                    if media_id.as_deref() == Some(reference) {
                        return Some(reference.to_string());
                    }
                }
                OutboundContentPart::ToolResult(tool_result) => {
                    for media in &tool_result.media {
                        if media.url == reference {
                            if let Some(media_id) = &media.object_key {
                                return Some(media_id.clone());
                            }
                        }
                        if media.object_key.as_deref() == Some(reference) {
                            return Some(reference.to_string());
                        }
                    }
                }
                OutboundContentPart::Text { .. } => {}
            }
        }
    }

    None
}

fn looks_like_external_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn map_runtime_error(error: anyhow::Error) -> ChatServiceError {
    ChatServiceError::upstream(error.to_string())
}

fn map_provider_response_error(
    provider_key: &str,
    status_code: u16,
    body: String,
) -> ChatServiceError {
    if matches!(status_code, 401 | 403) {
        return ChatServiceError::provider_authentication_failed(format!(
            "Provider `{provider_key}` authentication failed. Please update the API key and try again."
        ));
    }

    ChatServiceError::upstream(format!(
        "Image provider `{provider_key}` request failed: {status_code} {body}"
    ))
}

#[derive(Serialize)]
struct OpenAiImageRequest {
    model: String,
    prompt: String,
    size: String,
    n: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiImageResponse {
    #[serde(default)]
    data: Vec<OpenAiImageData>,
    #[serde(default)]
    error: Option<OpenAiImageResponseError>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    #[serde(default)]
    b64_json: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    revised_prompt: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiImageResponseError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        decode_base64_generated_image, decode_openai_image_payload, find_history_media_id,
        parse_image_generation_request, ImageToolOperation, OpenAiImageData, OpenAiImageResponse,
        OpenAiImageResponseError,
    };
    use crate::{
        ImageToolDefaults, MediaAsset, OutboundContentPart, OutboundMessage, OutboundToolCall,
        OutboundToolResult, TurnToolRef,
    };

    fn test_tool(image_defaults: Option<ImageToolDefaults>) -> TurnToolRef {
        TurnToolRef {
            runtime_provider: "openai_compatible".into(),
            model_config_id: "openchat:image:gpt-image-2".into(),
            model_name: "gpt-image-2".into(),
            id: "image_gen_gpt_image_2".into(),
            display_name: "GPT Image 2".into(),
            provider: "openai".into(),
            source: "openchat".into(),
            tool_type: "image".into(),
            image_defaults,
        }
    }

    #[test]
    fn defaults_to_generate_without_input_images() {
        let request =
            match parse_image_generation_request(r#"{"prompt":"draw a cat"}"#, &test_tool(None)) {
                Ok(request) => request,
                Err(_) => panic!("request should parse"),
            };

        assert_eq!(request.operation, ImageToolOperation::Generate);
        assert!(request.input_images.is_empty());
    }

    #[test]
    fn defaults_to_reference_when_input_images_are_present() {
        let request = match parse_image_generation_request(
            r#"{"prompt":"use this pose","input_images":["https://example.com/cat.png"]}"#,
            &test_tool(None),
        ) {
            Ok(request) => request,
            Err(_) => panic!("request should parse"),
        };

        assert_eq!(request.operation, ImageToolOperation::Reference);
        assert_eq!(request.input_images.len(), 1);
    }

    #[test]
    fn accepts_single_string_input_images_value() {
        let request = match parse_image_generation_request(
            r#"{"prompt":"edit this","input_images":"media/object-1.png"}"#,
            &test_tool(None),
        ) {
            Ok(request) => request,
            Err(_) => panic!("request should parse"),
        };

        assert_eq!(request.operation, ImageToolOperation::Reference);
        assert_eq!(request.input_images, vec!["media/object-1.png"]);
    }

    #[test]
    fn supports_n_parameter_and_explicit_size() {
        let request = match parse_image_generation_request(
            r#"{"prompt":"poster","size":"2048x1024","n":6}"#,
            &test_tool(None),
        ) {
            Ok(request) => request,
            Err(_) => panic!("request should parse"),
        };

        assert_eq!(request.size, "2048x1024");
        assert_eq!(request.count, 6);
    }

    #[test]
    fn supports_auto_and_extended_dimension_sizes() {
        let auto_request = match parse_image_generation_request(
            r#"{"prompt":"poster","size":"auto"}"#,
            &test_tool(Some(ImageToolDefaults {
                size: "2048x1152".into(),
                quality: "high".into(),
                n: 2,
            })),
        ) {
            Ok(request) => request,
            Err(_) => panic!("auto request should parse"),
        };
        let widescreen_request = match parse_image_generation_request(
            r#"{"prompt":"poster","size":"3840x2160"}"#,
            &test_tool(None),
        ) {
            Ok(request) => request,
            Err(_) => panic!("widescreen request should parse"),
        };

        assert_eq!(auto_request.size, "2048x1152");
        assert_eq!(auto_request.count, 2);
        assert_eq!(auto_request.quality.as_deref(), Some("high"));
        assert_eq!(widescreen_request.size, "3840x2160");
    }

    #[test]
    fn rejects_unsupported_sizes() {
        assert!(parse_image_generation_request(
            r#"{"prompt":"poster","size":"3000x3000"}"#,
            &test_tool(None),
        )
        .is_err());
    }

    #[test]
    fn falls_back_to_tool_profile_defaults() {
        let request = match parse_image_generation_request(
            r#"{"prompt":"poster"}"#,
            &test_tool(Some(ImageToolDefaults {
                size: "2048x2048".into(),
                quality: "medium".into(),
                n: 3,
            })),
        ) {
            Ok(request) => request,
            Err(_) => panic!("request should parse"),
        };

        assert_eq!(request.size, "2048x2048");
        assert_eq!(request.count, 3);
        assert_eq!(request.quality.as_deref(), Some("medium"));
    }

    #[test]
    fn resolves_history_media_by_url_and_object_key() {
        let history = vec![OutboundMessage {
            role: "tool".into(),
            item_id: "tool_result_1".into(),
            turn_id: "turn_1".into(),
            content: vec![OutboundContentPart::ToolResult(OutboundToolResult {
                tool_call_id: "call_1".into(),
                tool_name: "image_generation".into(),
                tool_display_name: Some("Image Generation".into()),
                status: "completed".into(),
                arguments_text: Some("{\"prompt\":\"cat\"}".into()),
                result: serde_json::json!({ "kind": "image" }),
                media: vec![MediaAsset {
                    kind: "image".into(),
                    url: "https://example.com/generated.png".into(),
                    object_key: Some("media/object-1.png".into()),
                    mime_type: "image/png".into(),
                    size_bytes: 128,
                }],
            })],
            tool_calls: vec![OutboundToolCall {
                id: "call_1".into(),
                name: "image_generation".into(),
                arguments_text: "{\"prompt\":\"cat\"}".into(),
            }],
            tool_call_id: Some("call_1".into()),
        }];

        assert_eq!(
            find_history_media_id("https://example.com/generated.png", history.as_slice()),
            Some("media/object-1.png".into())
        );
        assert_eq!(
            find_history_media_id("media/object-1.png", history.as_slice()),
            Some("media/object-1.png".into())
        );
    }

    #[test]
    fn empty_base64_payload_is_treated_as_missing_output() {
        let result = match decode_base64_generated_image("data:image/png;base64,") {
            Ok(result) => result,
            Err(_) => panic!("empty base64 payload should not hard fail"),
        };

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn rejects_non_image_base64_payload() {
        let error = decode_openai_image_payload(OpenAiImageResponse {
            data: vec![OpenAiImageData {
                b64_json: Some("aGVsbG8=".into()),
                url: None,
                revised_prompt: None,
            }],
            error: None,
        })
        .await
        .expect_err("non-image base64 payload should fail");

        assert!(error.message.contains("invalid base64 image payload"));
    }

    #[tokio::test]
    async fn surfaces_embedded_provider_error() {
        let error = decode_openai_image_payload(OpenAiImageResponse {
            data: Vec::new(),
            error: Some(OpenAiImageResponseError {
                message: Some("safety filter blocked the image".into()),
                code: Some("content_blocked".into()),
            }),
        })
        .await
        .expect_err("embedded error should fail");

        assert!(error.message.contains("safety filter blocked the image"));
        assert!(error.message.contains("content_blocked"));
    }

    #[tokio::test]
    async fn surfaces_revised_prompt_without_image_output() {
        let error = decode_openai_image_payload(OpenAiImageResponse {
            data: vec![OpenAiImageData {
                b64_json: None,
                url: None,
                revised_prompt: Some("A safer version of the prompt".into()),
            }],
            error: None,
        })
        .await
        .expect_err("missing image output should fail");

        assert!(error.message.contains("Revised prompt"));
    }
}
