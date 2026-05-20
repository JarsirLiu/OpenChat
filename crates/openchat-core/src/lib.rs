mod adapters;
mod application;
mod domain;
mod error;
mod media;
mod runtime;
pub mod streaming;

pub use adapters::context::{
    build_session_context, normalize_session_history, session_history_window_size,
    OutboundContentPart, OutboundMessage, OutboundToolCall, SessionContext,
};
pub use application::{ChatService, SessionMessagesSnapshotPage, TurnBuilder, TurnRunner};
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
    ActiveTurnHandle, ActiveTurnRegistry, CatalogTool, GeneratedImage,
    ImageGenerationToolHandler, ImageModelAccessResolver, ImageProviderRuntime, ImageRuntime,
    ModelEventStream, ModelProviderRuntime, ModelRuntime, ModelStreamEvent,
    OpenChatTurnExecutor, ResolveImageAccessFuture, ResolveTextAccessFuture,
    ResolveToolAccessFuture, ResolvedImageModelAccess, ResolvedTextModelAccess,
    TextModelAccessResolver, ToolAccessDecision, ToolAccessOutcome, ToolAccessRequirement,
    ToolAccessResolver, ToolAccessService, ToolCapability, ToolExecutionResult, ToolExecutor,
    ToolFunctionSpec, ToolInvocation, ToolRegistry, ToolSpec, TurnExecution,
    TurnExecutionFuture,
};
pub use streaming::{InMemorySessionStore, SessionRuntime, StreamEventPayload};

pub mod events {
    pub use crate::streaming::events::*;
}
