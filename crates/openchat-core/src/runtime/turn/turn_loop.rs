use std::{collections::BTreeMap, sync::Arc};

use futures_util::StreamExt;
use openchat_infra::stores::{ChatStore, PersistedToolCall};
use tokio::time::{timeout, Duration};

use crate::{
    runtime::turn::{
        event_builder::{
            build_image_generated_event, build_message_delta_event, build_reasoning_delta_event,
            build_reasoning_item, build_reasoning_started_event,
            build_tool_call_arguments_delta_event, build_tool_call_started_event, send_event,
        },
        helpers::now_string,
        history_assembler::HistoryAssembler,
        loop_step_result::{InProgressToolCall, LoopStepResult},
        message_writer::MessageWriter,
        tool_call_coordinator::ToolCallCoordinator,
    },
    ActiveTurnHandle, ImageModelAccessResolver, ModelEventStream, ModelProviderRuntime,
    ModelStreamEvent, SessionRuntime, TextModelAccessResolver, ToolAccessResolver, TurnPlan,
    TurnTerminalReason,
};

const MAX_MODEL_STEPS_PER_TURN: usize = 8;
const MODEL_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MODEL_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct TurnLoopRunResult {
    pub assistant_text: String,
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
    chat_store: Arc<ChatStore>,
    model_provider_runtime: ModelProviderRuntime<R>,
    tool_call_coordinator: ToolCallCoordinator<R>,
    message_writer: MessageWriter,
}

impl<R> TurnLoop<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    pub fn new(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_call_coordinator: ToolCallCoordinator<R>,
        message_writer: MessageWriter,
    ) -> Self {
        Self {
            chat_store,
            model_provider_runtime,
            tool_call_coordinator,
            message_writer,
        }
    }

    pub async fn run(
        &self,
        plan: &TurnPlan,
        session_runtime: &SessionRuntime,
        active_turn: &ActiveTurnHandle,
        user_item_id: &str,
        assistant_item_id: &str,
        reasoning_item_id: &str,
    ) -> TurnLoopExit {
        let mut rolling_history = plan.history.clone();
        let mut pass_prompt = plan.prompt.clone();
        let mut pass_attachments = plan.attachments.clone();
        let mut included_user_message_in_history = false;
        let mut assistant_accumulator = String::new();
        let mut reasoning_accumulator = String::new();
        let mut reasoning_started_once = false;
        let mut in_progress_tool_calls: BTreeMap<String, InProgressToolCall> = BTreeMap::new();

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

            let mut pass_state = LoopStepResult::default();
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

            loop {
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
                    break;
                };

                match event {
                    Ok(ModelStreamEvent::ReasoningDelta(delta)) => {
                        if !reasoning_started_once {
                            reasoning_started_once = true;
                            let _ = send_event(
                                session_runtime,
                                &build_reasoning_started_event(
                                    plan.session_id.clone(),
                                    active_turn.turn_id().to_string(),
                                    reasoning_item_id.to_string(),
                                    now_string(),
                                    build_reasoning_item(
                                        reasoning_item_id.to_string(),
                                        active_turn.turn_id().to_string(),
                                        "in_progress",
                                        Some(String::new()),
                                    ),
                                ),
                            );

                            self.message_writer
                                .write_reasoning_started(
                                    reasoning_item_id,
                                    plan.user_id.as_str(),
                                    plan.session_id.as_str(),
                                    active_turn.turn_id(),
                                )
                                .await;
                        }

                        pass_state.reasoning_started = true;
                        pass_state.reasoning_text.push_str(delta.as_str());
                        reasoning_accumulator.push_str(delta.as_str());

                        let _ = send_event(
                            session_runtime,
                            &build_reasoning_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                reasoning_item_id.to_string(),
                                now_string(),
                                delta,
                            ),
                        );

                        self.message_writer
                            .write_reasoning_in_progress(
                                reasoning_item_id,
                                plan.user_id.as_str(),
                                plan.session_id.as_str(),
                                active_turn.turn_id(),
                                reasoning_accumulator.as_str(),
                            )
                            .await;
                    }
                    Ok(ModelStreamEvent::TextDelta(delta)) => {
                        pass_state.assistant_text.push_str(delta.as_str());
                        assistant_accumulator.push_str(delta.as_str());

                        let _ = send_event(
                            session_runtime,
                            &build_message_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                assistant_item_id.to_string(),
                                now_string(),
                                delta,
                            ),
                        );

                        self.message_writer
                            .write_assistant_in_progress(
                                assistant_item_id,
                                plan.user_id.as_str(),
                                plan.session_id.as_str(),
                                active_turn.turn_id(),
                                assistant_accumulator.as_str(),
                            )
                            .await;
                    }
                    Ok(ModelStreamEvent::ToolCallStart {
                        tool_call_id,
                        tool_name,
                        arguments,
                    }) => {
                        in_progress_tool_calls.insert(
                            tool_call_id.clone(),
                            InProgressToolCall {
                                tool_name: tool_name.clone(),
                                arguments_text: String::new(),
                            },
                        );

                        let _ = send_event(
                            session_runtime,
                            &build_tool_call_started_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                tool_call_id.clone(),
                                tool_call_id.clone(),
                                Some(assistant_item_id.to_string()),
                                tool_name.clone(),
                                now_string(),
                                arguments,
                            ),
                        );

                        let _ = self
                            .chat_store
                            .upsert_tool_call(PersistedToolCall {
                                id: tool_call_id,
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                parent_item_id: Some(assistant_item_id.to_string()),
                                tool_name,
                                tool_display_name: None,
                                arguments_text: None,
                                result_json: None,
                                status: "in_progress".into(),
                                media_json: None,
                            })
                            .await;
                    }
                    Ok(ModelStreamEvent::ToolCallArgumentsDelta {
                        tool_call_id,
                        delta,
                    }) => {
                        let entry = in_progress_tool_calls
                            .entry(tool_call_id.clone())
                            .or_insert_with(InProgressToolCall::default);
                        entry.arguments_text.push_str(delta.as_str());

                        let _ = send_event(
                            session_runtime,
                            &build_tool_call_arguments_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                tool_call_id.clone(),
                                tool_call_id.clone(),
                                Some(assistant_item_id.to_string()),
                                now_string(),
                                delta.clone(),
                            ),
                        );

                        let _ = self
                            .chat_store
                            .upsert_tool_call(PersistedToolCall {
                                id: tool_call_id,
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                parent_item_id: Some(assistant_item_id.to_string()),
                                tool_name: entry.tool_name.clone(),
                                tool_display_name: None,
                                arguments_text: Some(entry.arguments_text.clone()),
                                result_json: None,
                                status: "in_progress".into(),
                                media_json: None,
                            })
                            .await;
                    }
                    Ok(ModelStreamEvent::ToolCallComplete {
                        tool_call_id,
                        tool_name,
                        arguments_text,
                    }) => {
                        let pending = in_progress_tool_calls.remove(&tool_call_id);
                        let resolved_tool_name = if tool_name.is_empty() {
                            pending
                                .as_ref()
                                .map(|call| call.tool_name.clone())
                                .unwrap_or_default()
                        } else {
                            tool_name
                        };
                        let resolved_arguments_text = if arguments_text.trim().is_empty() {
                            pending.map(|call| call.arguments_text).unwrap_or_default()
                        } else {
                            arguments_text
                        };

                        let completed_tool_call = self
                            .tool_call_coordinator
                            .execute_and_persist(
                                &self.chat_store,
                                session_runtime,
                                &pass_plan,
                                active_turn.turn_id(),
                                assistant_item_id,
                                &tool_call_id,
                                &resolved_tool_name,
                                resolved_arguments_text,
                            )
                            .await;

                        for media in completed_tool_call.media.iter().cloned() {
                            if media.kind != "image" || media.url.trim().is_empty() {
                                continue;
                            }
                            let _ = send_event(
                                session_runtime,
                                &build_image_generated_event(
                                    plan.session_id.clone(),
                                    active_turn.turn_id().to_string(),
                                    now_string(),
                                    media,
                                    Some(assistant_item_id.to_string()),
                                ),
                            );
                        }

                        pass_state.completed_tool_calls.push(completed_tool_call);

                        self.message_writer
                            .write_assistant_in_progress(
                                assistant_item_id,
                                plan.user_id.as_str(),
                                plan.session_id.as_str(),
                                active_turn.turn_id(),
                                assistant_accumulator.as_str(),
                            )
                            .await;
                    }
                    Err(error) => {
                        return TurnLoopExit::Failed(TurnTerminalReason::from_chat_service_error(
                            &error,
                        ));
                    }
                }
            }

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
            assistant_text: assistant_accumulator,
            reasoning_text: reasoning_accumulator,
            reasoning_started_once,
        })
    }
}
