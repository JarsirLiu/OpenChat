mod context;
mod error;
pub mod events;
mod execution;
mod execution_runtime;
mod media;
mod model_provider_runtime;
mod model_runtime;
mod request;
mod service;
mod session_runtime;
mod stream;
mod tools;
mod turn;
mod turn_control;
mod turn_terminal;
mod turn_plan;

pub use context::{
    build_session_context, normalize_session_history, session_history_window_size,
    OutboundContentPart, OutboundMessage, OutboundToolCall, SessionContext,
};
pub use error::ChatServiceError;
pub use execution::OpenChatTurnExecutor;
pub use execution_runtime::{TurnExecution, TurnExecutionFuture};
pub use media::{
    parse_media_assets_json, MediaAsset, MediaStore, ModelMediaUrlResolver, StoredMedia,
};
pub use model_provider_runtime::{
    ImageModelAccessResolver, ImageProviderRuntime, ModelProviderRuntime, ResolveImageAccessFuture,
    ResolveTextAccessFuture, ResolvedImageModelAccess, ResolvedTextModelAccess,
    TextModelAccessResolver,
};
pub use model_runtime::{ModelEventStream, ModelRuntime, ModelStreamEvent};
pub use request::{ChatRequest, SelectedTextModel, SelectedTool, UploadedAttachment};
pub use service::{ChatService, TurnBuilder, TurnRunner};
pub use session_runtime::{InMemorySessionStore, SessionRuntime};
pub use stream::StreamEventPayload;
pub use tools::{
    CatalogTool, GeneratedImage, ImageGenerationToolHandler, ImageRuntime, ResolveToolAccessFuture,
    ToolAccessDecision, ToolAccessOutcome, ToolAccessRequirement, ToolAccessResolver,
    ToolAccessService, ToolCapability, ToolExecutionResult, ToolExecutor, ToolFunctionSpec,
    ToolInvocation, ToolRegistry, ToolSpec,
};
pub use turn::{TurnAccepted, TurnContext};
pub use turn_control::{ActiveTurnHandle, ActiveTurnRegistry};
pub use turn_terminal::{TurnTerminalReason, TurnTerminalReasonCode};
pub use turn_plan::{TurnAttachment, TurnModelRef, TurnPlan, TurnToolRef};
