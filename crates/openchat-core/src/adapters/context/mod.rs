mod history;
mod content_parts_mapper;
mod normalize;
mod tool_call_attachment;
mod tool_result_formatter;
mod types;

pub use content_parts_mapper::{
    append_image_media_parts, assistant_text_to_content_json, user_content_to_json,
    user_content_to_outbound_parts, value_to_outbound_content_parts,
};
pub use history::{build_session_context, session_history_window_size};
pub use normalize::normalize_session_history;
pub use tool_call_attachment::collect_attached_tool_calls;
pub use tool_result_formatter::{
    format_persisted_tool_result_text, format_tool_result_text, sanitize_tool_result_json,
};
pub use types::{OutboundContentPart, OutboundMessage, OutboundToolCall, SessionContext};
