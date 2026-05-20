mod adapters;
mod application;
mod domain;
mod error;
mod media;
mod runtime;
mod tools;
pub mod streaming;

pub use adapters::context::{
    append_image_media_parts, assistant_text_to_content_json, build_session_context,
    collect_attached_tool_calls, format_persisted_tool_result_text, format_tool_result_text,
    normalize_session_history, sanitize_tool_result_json, session_history_window_size,
    user_content_to_json, user_content_to_outbound_parts, value_to_outbound_content_parts,
    OutboundContentPart, OutboundMessage, OutboundToolCall, SessionContext,
};
pub use application::{
    ActiveTurnRegistryPort, ChatRepository, ChatService, SessionMessagesSnapshotPage,
    SessionRuntimeRegistry, TurnBuilder, TurnRunner,
};
pub use domain::{
    ChatRequest, SelectedTextModel, SelectedTool, TurnAccepted, TurnAttachment, TurnContext,
    TurnModelRef, TurnPlan, TurnTerminalReason, TurnTerminalReasonCode, TurnToolRef,
    UploadedAttachment,
};
pub use error::{ChatServiceError, PROVIDER_API_KEY_REQUIRED, PROVIDER_AUTHENTICATION_FAILED};
pub use media::{
    parse_media_assets_json, MediaAsset, MediaStore, ModelMediaUrlResolver, StoredMedia,
};
pub use runtime::{
    ActiveTurnHandle, ActiveTurnRegistry, ImageModelAccessResolver, ImageProviderRuntime,
    ModelEventStream, ModelProviderRuntime, ModelStreamEvent, OpenAiCompatibleRuntime,
    OpenChatTurnExecutor, ResolveImageAccessFuture, ResolveTextAccessFuture,
    ResolvedImageModelAccess, ResolvedTextModelAccess, TextModelAccessResolver,
};
pub use tools::{
    CatalogTool, GeneratedImage, ImageGenerationToolHandler, ImageRuntime, ResolveToolAccessFuture,
    ToolAccessDecision, ToolAccessOutcome, ToolAccessRequirement, ToolAccessResolver,
    ToolAccessService, ToolCapability, ToolExecutionResult, ToolExecutor, ToolFunctionSpec,
    ToolInvocation, ToolRegistry, ToolSpec,
};
pub use streaming::{InMemorySessionStore, SessionRuntime, StreamEventPayload};

pub mod protocol {
    pub use crate::streaming::protocol::*;
}
