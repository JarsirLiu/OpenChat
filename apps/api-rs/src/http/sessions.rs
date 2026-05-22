use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItemDto {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub transcript_version: Option<String>,
    pub transcript_migration_status: Option<String>,
    pub transcript_migration_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadItemImageDto {
    pub url: String,
    pub object_key: Option<String>,
    pub mime_type: String,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadItemDto {
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
    pub content: Option<Value>,
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub revised_prompt: Option<String>,
    pub model: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub count: Option<u32>,
    pub source_tool_call_id: Option<String>,
    pub source_tool_name: Option<String>,
    pub images: Vec<SessionThreadItemImageDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTurnDto {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub terminal_reason: Option<openchat_core::protocol::TerminalReasonDto>,
    pub items: Vec<SessionThreadItemDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetailDto {
    pub session: SessionListItemDto,
    pub turns: Vec<SessionTurnDto>,
    pub history_page: SessionHistoryPageDto,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryPageDto {
    pub has_more: bool,
    pub next_before_turn_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct RenameSessionDto {
    pub title: String,
}

#[derive(Clone, Deserialize)]
pub struct SessionHistoryQueryDto {
    pub before_turn_id: Option<String>,
}
