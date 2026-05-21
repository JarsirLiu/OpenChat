use futures_util::StreamExt;
use tokio::time::{timeout, Duration};

use crate::{
    runtime::turn::{
        history_assembler::HistoryAssembler,
        model_event_dispatcher::ModelEventDispatcher,
        tool_call_coordinator::ToolCallCoordinator,
        transcript_projector::{ProjectionContext, TranscriptProjector},
    },
    ActiveTurnHandle, ImageModelAccessResolver, ModelEventStream, ModelProviderRuntime,
    SessionRuntime, TextModelAccessResolver, ToolAccessResolver, TurnPlan, TurnTerminalReason,
};

const MAX_MODEL_STEPS_PER_TURN: usize = 8;
const MODEL_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MODEL_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct TurnLoopRunResult {
    pub reasoning_text: String,
    pub reasoning_started_once: bool,
}

pub(crate) enum TurnLoopExit {
    Completed(TurnLoopRunResult),
    Interrupted,
    Failed(TurnTerminalReason),
}

#[derive(Clone)]
pub(crate) struct TurnLoop<R> {
    model_provider_runtime: ModelProviderRuntime<R>,
    tool_call_coordinator: ToolCallCoordinator<R>,
    transcript_projector: TranscriptProjector,
}

impl<R> TurnLoop<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    pub fn new(
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_call_coordinator: ToolCallCoordinator<R>,
        transcript_projector: TranscriptProjector,
    ) -> Self {
        Self {
            model_provider_runtime,
            tool_call_coordinator,
            transcript_projector,
        }
    }

    pub async fn run(
        &self,
        plan: &TurnPlan,
        session_runtime: &SessionRuntime,
        active_turn: &ActiveTurnHandle,
        user_item_id: &str,
        reasoning_item_id: &str,
    ) -> TurnLoopExit {
        let mut rolling_history = plan.history.clone();
        let mut pass_prompt = plan.prompt.clone();
        let mut pass_attachments = plan.attachments.clone();
        let mut included_user_message_in_history = false;
        let mut reasoning_accumulator = String::new();
        let mut reasoning_started_once = false;
        let projection_context = ProjectionContext {
            user_id: plan.user_id.as_str(),
            session_id: plan.session_id.as_str(),
            turn_id: active_turn.turn_id(),
        };

        for step_index in 0..MAX_MODEL_STEPS_PER_TURN {
            let pass_plan = TurnPlan {
                user_id: plan.user_id.clone(),
                session_id: plan.session_id.clone(),
                prompt: pass_prompt.clone(),
                attachments: pass_attachments.clone(),
                history: rolling_history.clone(),
                text_model: plan.text_model.clone(),
                tool_list: plan.tool_list.clone(),
            };

            let connect_cancellation = active_turn.cancellation_token();
            let mut model_events: ModelEventStream = tokio::select! {
                _ = connect_cancellation.cancelled() => {
                    return TurnLoopExit::Interrupted;
                }
                result = timeout(MODEL_CONNECT_TIMEOUT, self.model_provider_runtime.stream_text(&pass_plan)) => {
                    match result {
                        Ok(Ok(stream)) => stream,
                        Ok(Err(error)) => {
                            return TurnLoopExit::Failed(TurnTerminalReason::from_chat_service_error(&error));
                        }
                        Err(_) => {
                            return TurnLoopExit::Failed(TurnTerminalReason::model_connect_timeout());
                        }
                    }
                }
            };

            let mut event_dispatcher = ModelEventDispatcher::new(
                &self.transcript_projector,
                &self.tool_call_coordinator,
                session_runtime,
                &pass_plan,
                projection_context,
                reasoning_item_id,
                &mut reasoning_accumulator,
                &mut reasoning_started_once,
            );

            let pass_state = loop {
                let stream_cancellation = active_turn.cancellation_token();
                let next_event = tokio::select! {
                    _ = stream_cancellation.cancelled() => {
                        return TurnLoopExit::Interrupted;
                    }
                    result = timeout(MODEL_STREAM_IDLE_TIMEOUT, model_events.next()) => {
                        match result {
                            Ok(event) => event,
                            Err(_) => {
                                return TurnLoopExit::Failed(TurnTerminalReason::model_stream_idle_timeout());
                            }
                        }
                    }
                };

                let Some(event) = next_event else {
                    break event_dispatcher.into_step_result();
                };

                match event {
                    Ok(event) => {
                        if let Err(reason) = event_dispatcher.handle_event(event).await {
                            return TurnLoopExit::Failed(reason);
                        }
                    }
                    Err(error) => {
                        return TurnLoopExit::Failed(TurnTerminalReason::from_chat_service_error(
                            &error,
                        ));
                    }
                }
            };

            if !pass_state.completed_tool_calls.is_empty() {
                HistoryAssembler::append_step_result(
                    &mut rolling_history,
                    &mut included_user_message_in_history,
                    user_item_id,
                    active_turn.turn_id(),
                    plan.prompt.as_str(),
                    plan.attachments.as_slice(),
                    step_index,
                    &pass_state,
                );

                pass_prompt.clear();
                pass_attachments.clear();
                continue;
            }

            break;
        }

        TurnLoopExit::Completed(TurnLoopRunResult {
            reasoning_text: reasoning_accumulator,
            reasoning_started_once,
        })
    }
}
