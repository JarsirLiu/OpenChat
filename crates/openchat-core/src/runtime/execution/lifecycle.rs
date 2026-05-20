use openchat_infra::stores::ChatStore;
use openchat_infra::stores::PersistedTurnTerminalReason;

use crate::{
    runtime::execution::{
        event_builder::{
            build_session, build_session_updated_event, build_turn, build_turn_completed_event,
            build_turn_failed_event, send_event,
        },
        helpers::now_string,
    },
    SessionRuntime, TurnTerminalReason,
};

use super::super::turn_control::ActiveTurnHandle;

pub enum TurnTerminalState {
    Completed {
        started_at: String,
    },
    Interrupted {
        started_at: String,
        reason: TurnTerminalReason,
    },
    Failed {
        reason: TurnTerminalReason,
    },
}

pub async fn finalize_turn(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    active_turn: &ActiveTurnHandle,
    state: TurnTerminalState,
) {
    match &state {
        TurnTerminalState::Completed { started_at } => {
            let completed_at = now_string();
            let turn = build_turn(
                active_turn.turn_id().to_string(),
                active_turn.session_id().to_string(),
                "completed",
                Some(started_at.clone()),
                Some(completed_at.clone()),
                None,
            );
            let _ = send_event(
                session_runtime,
                &build_turn_completed_event(
                    active_turn.session_id().to_string(),
                    active_turn.turn_id().to_string(),
                    completed_at,
                    turn,
                ),
            );
            let _ = chat_store
                .complete_turn(
                    active_turn.turn_id(),
                    active_turn.session_id(),
                    "completed",
                    None,
                )
                .await;
        }
        TurnTerminalState::Interrupted { started_at, reason } => {
            let completed_at = now_string();
            let turn = build_turn(
                active_turn.turn_id().to_string(),
                active_turn.session_id().to_string(),
                "interrupted",
                Some(started_at.clone()),
                Some(completed_at.clone()),
                Some(reason.to_event_dto()),
            );
            let _ = send_event(
                session_runtime,
                &build_turn_completed_event(
                    active_turn.session_id().to_string(),
                    active_turn.turn_id().to_string(),
                    completed_at,
                    turn,
                ),
            );
            let _ = chat_store
                .complete_turn(
                    active_turn.turn_id(),
                    active_turn.session_id(),
                    "interrupted",
                    Some(&PersistedTurnTerminalReason {
                        code: reason.code_str().to_string(),
                        message: Some(reason.message().to_string()),
                    }),
                )
                .await;
        }
        TurnTerminalState::Failed { reason } => {
            let _ = send_event(
                session_runtime,
                &build_turn_failed_event(
                    active_turn.session_id().to_string(),
                    active_turn.turn_id().to_string(),
                    now_string(),
                    reason.code_str().to_string(),
                    reason.message().to_string(),
                ),
            );
            let _ = chat_store
                .complete_turn(
                    active_turn.turn_id(),
                    active_turn.session_id(),
                    "failed",
                    Some(&PersistedTurnTerminalReason {
                        code: reason.code_str().to_string(),
                        message: Some(reason.message().to_string()),
                    }),
                )
                .await;
        }
    }

    active_turn.finish().await;
    let _ = emit_session_updated(chat_store, session_runtime, active_turn.session_id()).await;
}

pub async fn emit_session_updated(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    session_id: &str,
) -> anyhow::Result<()> {
    let Some(session) = chat_store.get_session_unscoped(session_id).await? else {
        return Ok(());
    };

    let _ = send_event(
        session_runtime,
        &build_session_updated_event(
            session.id.clone(),
            now_string(),
            build_session(
                session.id,
                session.title,
                session.status,
                session.created_at,
                session.updated_at,
            ),
        ),
    );

    Ok(())
}
