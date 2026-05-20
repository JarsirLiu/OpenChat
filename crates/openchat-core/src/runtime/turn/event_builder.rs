use crate::{
    protocol::{
        serialize_event, ErrorDto, ImageGeneratedEvent, ItemMessageDeltaEvent, ItemStartedEvent,
        ItemToolCallArgumentsDeltaEvent, ItemToolCallCompletedEvent, ItemToolCallStartedEvent,
        MessageItemDto, ReasoningCompletedEvent, ReasoningDeltaEvent, ReasoningItemDto,
        ReasoningStartedEvent, SessionDto, SessionUpdatedEvent, TerminalReasonDto, ToolCallItemDto,
        TurnCompletedEvent, TurnDto, TurnFailedEvent, TurnStartedEvent,
    },
    MediaAsset, SessionRuntime, StreamEventPayload,
};
use serde::Serialize;
use serde_json::Value;

pub fn send_event<T: Serialize>(
    runtime: &SessionRuntime,
    event: &T,
) -> Result<usize, tokio::sync::broadcast::error::SendError<StreamEventPayload>> {
    let payload = match serialize_event(event) {
        Ok(payload) => payload,
        Err(_) => return Ok(0),
    };

    runtime.sender.send(payload)
}

pub fn build_turn(
    id: String,
    session_id: String,
    status: &str,
    started_at: Option<String>,
    completed_at: Option<String>,
    terminal_reason: Option<TerminalReasonDto>,
) -> TurnDto {
    TurnDto {
        id,
        session_id,
        status: status.into(),
        started_at,
        completed_at,
        terminal_reason,
    }
}

pub fn build_session(
    id: String,
    title: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
) -> SessionDto {
    SessionDto {
        id,
        title,
        status,
        created_at,
        updated_at,
    }
}

pub fn build_message_item(
    id: String,
    turn_id: String,
    status: &str,
    role: &str,
    text: Option<String>,
    content: Option<Value>,
) -> MessageItemDto {
    MessageItemDto {
        id,
        turn_id,
        kind: "message",
        status: status.into(),
        role: role.into(),
        text,
        content,
    }
}

pub fn build_reasoning_item(
    id: String,
    turn_id: String,
    status: &str,
    text: Option<String>,
) -> ReasoningItemDto {
    ReasoningItemDto {
        id,
        turn_id,
        kind: "reasoning",
        status: status.into(),
        text,
    }
}

pub fn build_tool_call_item(
    id: String,
    turn_id: String,
    status: &str,
    tool_call_id: String,
    parent_item_id: Option<String>,
    tool_name: String,
    tool_display_name: Option<String>,
    arguments_text: Option<String>,
    content: Value,
) -> ToolCallItemDto {
    ToolCallItemDto {
        id,
        turn_id,
        kind: "tool_call",
        status: status.into(),
        tool_call_id,
        parent_item_id,
        tool_name,
        tool_display_name,
        arguments_text,
        content,
    }
}

pub fn build_turn_started_event(
    session_id: String,
    turn_id: String,
    at: String,
    turn: TurnDto,
) -> TurnStartedEvent {
    TurnStartedEvent {
        event_type: "turn.started",
        session_id,
        turn_id,
        at,
        turn,
    }
}

pub fn build_session_updated_event(
    session_id: String,
    at: String,
    session: SessionDto,
) -> SessionUpdatedEvent {
    SessionUpdatedEvent {
        event_type: "session.updated",
        session_id,
        at,
        session,
    }
}

pub fn build_message_started_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    item: MessageItemDto,
) -> ItemStartedEvent<MessageItemDto> {
    ItemStartedEvent {
        event_type: "item.started",
        session_id,
        turn_id,
        item_id,
        at,
        item,
    }
}

pub fn build_message_delta_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    delta: String,
) -> ItemMessageDeltaEvent {
    ItemMessageDeltaEvent {
        event_type: "item.message.delta",
        session_id,
        turn_id,
        item_id,
        at,
        delta,
    }
}

pub fn build_reasoning_started_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    item: ReasoningItemDto,
) -> ReasoningStartedEvent {
    ReasoningStartedEvent {
        event_type: "reasoning.started",
        session_id,
        turn_id,
        item_id,
        at,
        item,
    }
}

pub fn build_reasoning_delta_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    delta: String,
) -> ReasoningDeltaEvent {
    ReasoningDeltaEvent {
        event_type: "reasoning.delta",
        session_id,
        turn_id,
        item_id,
        at,
        delta,
    }
}

pub fn build_reasoning_completed_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    item: ReasoningItemDto,
) -> ReasoningCompletedEvent {
    ReasoningCompletedEvent {
        event_type: "reasoning.completed",
        session_id,
        turn_id,
        item_id,
        at,
        item,
    }
}

pub fn build_tool_call_started_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    tool_call_id: String,
    parent_item_id: Option<String>,
    tool_name: String,
    at: String,
    arguments: Option<Value>,
) -> ItemToolCallStartedEvent {
    ItemToolCallStartedEvent {
        event_type: "item.tool_call.started",
        session_id,
        turn_id,
        item_id,
        tool_call_id,
        parent_item_id,
        tool_name,
        at,
        arguments,
    }
}

pub fn build_tool_call_arguments_delta_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    tool_call_id: String,
    parent_item_id: Option<String>,
    at: String,
    delta: String,
) -> ItemToolCallArgumentsDeltaEvent {
    ItemToolCallArgumentsDeltaEvent {
        event_type: "item.tool_call.arguments.delta",
        session_id,
        turn_id,
        item_id,
        tool_call_id,
        parent_item_id,
        at,
        delta,
    }
}

pub fn build_tool_call_completed_event(
    session_id: String,
    turn_id: String,
    item_id: String,
    at: String,
    item: ToolCallItemDto,
) -> ItemToolCallCompletedEvent {
    ItemToolCallCompletedEvent {
        event_type: "item.tool_call.completed",
        session_id,
        turn_id,
        item_id,
        at,
        item,
    }
}

pub fn build_image_generated_event(
    session_id: String,
    turn_id: String,
    at: String,
    media: MediaAsset,
    target_item_id: Option<String>,
) -> ImageGeneratedEvent {
    ImageGeneratedEvent {
        event_type: "image_generated",
        session_id,
        turn_id,
        at,
        media,
        target_item_id,
        canvas_id: None,
    }
}

pub fn build_turn_completed_event(
    session_id: String,
    turn_id: String,
    at: String,
    turn: TurnDto,
) -> TurnCompletedEvent {
    TurnCompletedEvent {
        event_type: "turn.completed",
        session_id,
        turn_id,
        at,
        turn,
    }
}

pub fn build_turn_failed_event(
    session_id: String,
    turn_id: String,
    at: String,
    code: String,
    message: String,
) -> TurnFailedEvent {
    TurnFailedEvent {
        event_type: "turn.failed",
        session_id,
        turn_id,
        at,
        error: ErrorDto {
            code: Some(code),
            message,
        },
    }
}
