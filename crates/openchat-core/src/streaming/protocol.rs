use serde::Serialize;
use serde_json::Value;

use crate::{MediaAsset, StreamEventPayload};

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
    pub result: Option<Value>,
    pub media: Option<Vec<MediaAsset>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallSummaryDto {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub parent_item_id: Option<String>,
    pub arguments_text: Option<String>,
    pub result: Option<Value>,
    pub status: Option<String>,
    pub media: Option<Vec<MediaAsset>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSnapshotDto {
    pub id: String,
    pub role: String,
    pub turn_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub content: Value,
    pub tool_calls: Option<Vec<ToolCallSummaryDto>>,
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
pub struct ImageGeneratedEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub session_id: String,
    pub turn_id: String,
    pub at: String,
    pub media: MediaAsset,
    pub target_item_id: Option<String>,
    pub canvas_id: Option<String>,
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
