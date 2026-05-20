use openchat_infra::stores::PersistedSessionToolCall;
use serde_json::Value;

use crate::{parse_media_assets_json, MediaAsset};

pub fn format_tool_result_text(
    display_name: &str,
    status: &str,
    arguments_text: Option<&str>,
    result: Option<&Value>,
    media: &[MediaAsset],
) -> String {
    let mut lines = vec![
        format!("[Tool Result: {display_name}]"),
        format!("status: {status}"),
    ];

    if let Some(arguments_text) = arguments_text
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("arguments: {arguments_text}"));
    }

    if let Some(result_text) = result.map(sanitize_tool_result_json) {
        lines.push(format!("result: {result_text}"));
    }

    let image_count = count_image_attachments(media);
    if image_count > 0 {
        lines.push(format!(
            "image_attachment: {image_count} image(s) available"
        ));
    }

    lines.join("\n")
}

pub fn format_persisted_tool_result_text(tool_call: &PersistedSessionToolCall) -> String {
    let display_name = tool_call
        .tool_display_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(tool_call.tool_name.as_str());
    let result = tool_call
        .result_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    let media = parse_media_assets_json(tool_call.media_json.as_deref());

    format_tool_result_text(
        display_name,
        tool_call.status.as_str(),
        tool_call.arguments_text.as_deref(),
        result.as_ref(),
        media.as_slice(),
    )
}

pub fn sanitize_tool_result_json(result: &Value) -> Value {
    let mut sanitized = result.clone();

    if let Some(output) = sanitized.get_mut("output").and_then(Value::as_object_mut) {
        output.remove("downloadUrl");
    }

    sanitized
}

pub fn count_image_attachments(media: &[MediaAsset]) -> usize {
    media.iter().filter(|media| media.kind == "image").count()
}
