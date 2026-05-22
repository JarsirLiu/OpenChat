use serde::Serialize;
use serde_json::Value;

use crate::StreamEventPayload;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnDto {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub terminal_reason: Option<TerminalReasonDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageItemDto {
    pub id: String,
    pub turn_id: String,
    pub kind: &'static str,
    pub status: String,
    pub role: String,
    pub text: Option<String>,
    pub content: Option<Value>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningItemDto {
    pub id: String,
    pub turn_id: String,
    pub kind: &'static str,
    pub status: String,
    pub text: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallItemDto {
    pub id: String,
    pub turn_id: String,
    pub kind: &'static str,
    pub status: String,
    pub tool_call_id: String,
    pub parent_item_id: Option<String>,
    pub tool_name: String,
    pub tool_display_name: Option<String>,
    pub arguments_text: Option<String>,
    pub content: Value,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImageAssetDto {
    pub url: String,
    pub object_key: Option<String>,
    pub mime_type: String,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadItemSnapshotDto {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub session_id: String,
    pub turn_id: String,
    pub status: String,
    pub seq: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<GeneratedImageAssetDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSnapshotDto {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub terminal_reason: Option<TerminalReasonDto>,
    pub items: Vec<ThreadItemSnapshotDto>,
}

#[derive(Clone, Serialize)]
pub struct ErrorDto {
    pub code: Option<String>,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct TerminalReasonDto {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreatedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub at: String,
    pub session: SessionDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdatedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub at: String,
    pub session: SessionDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub at: String,
    pub turn: TurnDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStartedEvent<T> {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub item: T,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemMessageDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub delta: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningStartedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub item: ReasoningItemDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub delta: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningCompletedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub item: ReasoningItemDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemToolCallStartedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub tool_call_id: String,
    pub parent_item_id: Option<String>,
    pub tool_name: String,
    pub at: String,
    pub arguments: Option<Value>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemToolCallArgumentsDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub tool_call_id: String,
    pub parent_item_id: Option<String>,
    pub at: String,
    pub delta: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemToolCallCompletedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub at: String,
    pub item: ToolCallItemDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompletedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub at: String,
    pub turn: TurnDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnFailedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub at: String,
    pub error: ErrorDto,
}

pub fn serialize_event<T: Serialize>(event: &T) -> Result<StreamEventPayload, serde_json::Error> {
    Ok(StreamEventPayload {
        data: serde_json::to_string(event)?,
    })
}

pub struct ChatEventEnvelope {
    pub data: String,
}

impl ChatEventEnvelope {
    pub fn new(payload: StreamEventPayload) -> Self {
        Self { data: payload.data }
    }
}
