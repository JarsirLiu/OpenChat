use openchat_core::MediaAsset;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItemDto {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessageDto {
    pub id: String,
    pub role: String,
    pub turn_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub content: Value,
    pub tool_calls: Vec<SessionToolCallSummaryDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionToolCallSummaryDto {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub parent_item_id: Option<String>,
    pub arguments_text: Option<String>,
    pub result: Option<Value>,
    pub status: String,
    pub media: Vec<MediaAsset>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetailDto {
    pub session: SessionListItemDto,
    pub messages: Vec<SessionMessageDto>,
}

#[derive(Clone, Deserialize)]
pub struct RenameSessionDto {
    pub title: String,
}
