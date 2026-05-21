use serde_json::Value;

use crate::{MediaAsset, OutboundToolResult};

pub fn format_outbound_tool_result_text(tool_result: &OutboundToolResult) -> String {
    format_tool_result_text(
        tool_result
            .tool_display_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(tool_result.tool_name.as_str()),
        tool_result.status.as_str(),
        tool_result.arguments_text.as_deref(),
        Some(&tool_result.result),
        tool_result.media.as_slice(),
    )
}

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
        lines.extend(format_image_input_refs(media));
    }

    lines.join("\n")
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

fn format_image_input_refs(media: &[MediaAsset]) -> Vec<String> {
    media
        .iter()
        .filter(|media| media.kind == "image")
        .enumerate()
        .flat_map(|(index, media)| {
            let mut lines = Vec::new();
            let item_index = index + 1;

            if let Some(object_key) = media
                .object_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("input_image_ref_{item_index}: {object_key}"));
            }

            if !media.url.trim().is_empty() {
                lines.push(format!(
                    "input_image_url_{item_index}: {}",
                    media.url.trim()
                ));
            }

            lines
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::format_tool_result_text;
    use crate::MediaAsset;
    use serde_json::json;

    #[test]
    fn image_tool_results_include_reusable_input_refs() {
        let text = format_tool_result_text(
            "GPT Image 2",
            "completed",
            Some("{\"prompt\":\"edit this\"}"),
            Some(&json!({ "kind": "image" })),
            &[MediaAsset {
                kind: "image".into(),
                url: "https://example.com/generated.png".into(),
                object_key: Some("media/object-1.png".into()),
                mime_type: "image/png".into(),
                size_bytes: 128,
            }],
        );

        assert!(text.contains("input_image_ref_1: media/object-1.png"));
        assert!(text.contains("input_image_url_1: https://example.com/generated.png"));
    }
}
