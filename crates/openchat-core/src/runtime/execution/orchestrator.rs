use std::{collections::BTreeMap, sync::Arc};

use futures_util::StreamExt;
use openchat_infra::stores::{ChatStore, PersistedMessage, PersistedToolCall};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};

use crate::{
    assistant_text_to_content_json, append_image_media_parts, format_tool_result_text,
    user_content_to_json, user_content_to_outbound_parts,
    runtime::execution::{
        event_builder::{
            build_image_generated_event, build_message_delta_event, build_message_item,
            build_message_started_event, build_reasoning_completed_event,
            build_reasoning_delta_event, build_reasoning_item, build_reasoning_started_event,
            build_tool_call_arguments_delta_event, build_tool_call_completed_event,
            build_tool_call_item, build_tool_call_started_event, build_turn,
            build_turn_started_event, send_event,
        },
        helpers::now_string,
        lifecycle::{emit_session_updated, finalize_turn, TurnTerminalState},
        session_title::SessionTitleGenerator,
    },
    ActiveTurnHandle, ImageModelAccessResolver, MediaAsset, ModelEventStream, OutboundContentPart,
    ModelProviderRuntime, ModelStreamEvent, OutboundMessage, OutboundToolCall, SessionRuntime,
    TextModelAccessResolver, ToolAccessResolver, ToolExecutionResult, ToolExecutor,
    ToolInvocation, TurnExecution, TurnExecutionFuture, TurnPlan, TurnRunner,
    TurnTerminalReason,
};

const MAX_MODEL_STEPS_PER_TURN: usize = 8;
const MODEL_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MODEL_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(15);

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

#[derive(Clone)]
pub struct OpenChatTurnExecutor<R> {
    chat_store: Arc<ChatStore>,
    model_provider_runtime: ModelProviderRuntime<R>,
    tool_executor: ToolExecutor<R>,
    session_title_generator: SessionTitleGenerator<R>,
}

#[derive(Default)]
struct PassState {
    assistant_text: String,
    reasoning_text: String,
    reasoning_started: bool,
    completed_tool_calls: Vec<CompletedToolCall>,
}

struct CompletedToolCall {
    tool_call_id: String,
    tool_name: String,
    tool_display_name: String,
    arguments_text: String,
    result: Value,
    media: Vec<MediaAsset>,
    failed: bool,
}

#[derive(Default)]
struct InProgressToolCall {
    tool_name: String,
    arguments_text: String,
}

impl<R> OpenChatTurnExecutor<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    pub fn new(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_executor: ToolExecutor<R>,
    ) -> Self {
        let session_title_generator =
            SessionTitleGenerator::new(chat_store.clone(), model_provider_runtime.clone());
        Self {
            chat_store,
            model_provider_runtime,
            tool_executor,
            session_title_generator,
        }
    }

    async fn execute_turn(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_executor: ToolExecutor<R>,
        session_title_generator: SessionTitleGenerator<R>,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: ActiveTurnHandle,
    ) {
        let turn_started_at = now_string();
        let turn = build_turn(
            active_turn.turn_id().to_string(),
            plan.session_id.clone(),
            "running",
            Some(turn_started_at.clone()),
            None,
            None,
        );

        let user_item_id = format!("item_user_{}", now_millis());
        let assistant_item_id = format!("item_assistant_{}", now_millis());
        let reasoning_item_id = format!("item_reasoning_{}", now_millis());
        let selected_tool_id = plan.tool_list.first().map(|tool| tool.id.as_str());

        let _ = chat_store
            .begin_turn(
                plan.user_id.as_str(),
                active_turn.turn_id(),
                plan.session_id.as_str(),
                plan.prompt.as_str(),
                plan.text_model.model_config_id.as_str(),
                selected_tool_id,
            )
            .await;
        session_title_generator.ensure_initial_title(&plan).await;
        let _ = emit_session_updated(&chat_store, &session_runtime, plan.session_id.as_str()).await;

        let _ = send_event(
            &session_runtime,
            &build_turn_started_event(
                plan.session_id.clone(),
                active_turn.turn_id().to_string(),
                turn_started_at.clone(),
                turn.clone(),
            ),
        );

        let user_content = user_content_to_json(plan.prompt.as_str(), plan.attachments.as_slice());

        let _ = send_event(
            &session_runtime,
            &build_message_started_event(
                plan.session_id.clone(),
                active_turn.turn_id().to_string(),
                user_item_id.clone(),
                now_string(),
                build_message_item(
                    user_item_id.clone(),
                    active_turn.turn_id().to_string(),
                    "completed",
                    "user",
                    Some(plan.prompt.clone()),
                    Some(user_content.clone()),
                ),
            ),
        );

        let _ = upsert_message(
            &chat_store,
            PersistedMessage {
                id: user_item_id.clone(),
                user_id: plan.user_id.clone(),
                session_id: plan.session_id.clone(),
                turn_id: active_turn.turn_id().to_string(),
                role: "user".into(),
                status: "completed".into(),
                content_json: user_content.to_string(),
                tool_call_id: None,
            },
        )
        .await;

        let _ = send_event(
            &session_runtime,
            &build_message_started_event(
                plan.session_id.clone(),
                active_turn.turn_id().to_string(),
                assistant_item_id.clone(),
                now_string(),
                build_message_item(
                    assistant_item_id.clone(),
                    active_turn.turn_id().to_string(),
                    "in_progress",
                    "assistant",
                    Some(String::new()),
                    None,
                ),
            ),
        );

        let _ = upsert_message(
            &chat_store,
            PersistedMessage {
                id: assistant_item_id.clone(),
                user_id: plan.user_id.clone(),
                session_id: plan.session_id.clone(),
                turn_id: active_turn.turn_id().to_string(),
                role: "assistant".into(),
                status: "in_progress".into(),
                content_json: assistant_text_to_content_json("").to_string(),
                tool_call_id: None,
            },
        )
        .await;

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

            let mut pass_state = PassState::default();
            let connect_cancellation = active_turn.cancellation_token();
            let mut model_events: ModelEventStream = tokio::select! {
                _ = connect_cancellation.cancelled() => {
                    interrupt_turn(
                        &chat_store,
                        &session_runtime,
                        &active_turn,
                        turn_started_at.as_str(),
                    )
                    .await;
                    return;
                }
                result = timeout(MODEL_CONNECT_TIMEOUT, model_provider_runtime.stream_text(&pass_plan)) => {
                    match result {
                        Ok(Ok(stream)) => stream,
                        Ok(Err(error)) => {
                            fail_turn(
                                &chat_store,
                                &session_runtime,
                                &active_turn,
                                TurnTerminalReason::from_chat_service_error(&error),
                            )
                            .await;
                            return;
                        }
                        Err(_) => {
                            fail_turn(
                                &chat_store,
                                &session_runtime,
                                &active_turn,
                                TurnTerminalReason::model_connect_timeout(),
                            )
                            .await;
                            return;
                        }
                    }
                }
            };

            loop {
                let stream_cancellation = active_turn.cancellation_token();
                let next_event = tokio::select! {
                    _ = stream_cancellation.cancelled() => {
                        interrupt_turn(
                            &chat_store,
                            &session_runtime,
                            &active_turn,
                            turn_started_at.as_str(),
                        )
                        .await;
                        return;
                    }
                    result = timeout(MODEL_STREAM_IDLE_TIMEOUT, model_events.next()) => {
                        match result {
                            Ok(event) => event,
                            Err(_) => {
                                fail_turn(
                                    &chat_store,
                                    &session_runtime,
                                    &active_turn,
                                    TurnTerminalReason::model_stream_idle_timeout(),
                                )
                                .await;
                                return;
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
                                &session_runtime,
                                &build_reasoning_started_event(
                                    plan.session_id.clone(),
                                    active_turn.turn_id().to_string(),
                                    reasoning_item_id.clone(),
                                    now_string(),
                                    build_reasoning_item(
                                        reasoning_item_id.clone(),
                                        active_turn.turn_id().to_string(),
                                        "in_progress",
                                        Some(String::new()),
                                    ),
                                ),
                            );

                            let _ = upsert_message(
                                &chat_store,
                                PersistedMessage {
                                    id: reasoning_item_id.clone(),
                                    user_id: plan.user_id.clone(),
                                    session_id: plan.session_id.clone(),
                                    turn_id: active_turn.turn_id().to_string(),
                                    role: "reasoning".into(),
                                    status: "in_progress".into(),
                                    content_json: json!([{ "type": "text", "text": "" }])
                                        .to_string(),
                                    tool_call_id: None,
                                },
                            )
                            .await;
                        }

                        pass_state.reasoning_started = true;
                        pass_state.reasoning_text.push_str(delta.as_str());
                        reasoning_accumulator.push_str(delta.as_str());

                        let _ = send_event(
                            &session_runtime,
                            &build_reasoning_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                reasoning_item_id.clone(),
                                now_string(),
                                delta,
                            ),
                        );

                        let _ = upsert_message(
                            &chat_store,
                            PersistedMessage {
                                id: reasoning_item_id.clone(),
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                role: "reasoning".into(),
                                status: "in_progress".into(),
                                content_json: json!([{ "type": "text", "text": reasoning_accumulator.clone() }]).to_string(),
                                tool_call_id: None,
                            },
                        )
                        .await;
                    }
                    Ok(ModelStreamEvent::TextDelta(delta)) => {
                        pass_state.assistant_text.push_str(delta.as_str());
                        assistant_accumulator.push_str(delta.as_str());

                        let _ = send_event(
                            &session_runtime,
                            &build_message_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                assistant_item_id.clone(),
                                now_string(),
                                delta,
                            ),
                        );

                        let _ = upsert_message(
                            &chat_store,
                            PersistedMessage {
                                id: assistant_item_id.clone(),
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                role: "assistant".into(),
                                status: "in_progress".into(),
                                content_json: assistant_text_to_content_json(
                                    assistant_accumulator.as_str(),
                                )
                                .to_string(),
                                tool_call_id: None,
                            },
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
                            &session_runtime,
                            &build_tool_call_started_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                tool_call_id.clone(),
                                tool_call_id.clone(),
                                Some(assistant_item_id.clone()),
                                tool_name.clone(),
                                now_string(),
                                arguments,
                            ),
                        );

                        let _ = chat_store
                            .upsert_tool_call(PersistedToolCall {
                                id: tool_call_id,
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                parent_item_id: Some(assistant_item_id.clone()),
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
                            &session_runtime,
                            &build_tool_call_arguments_delta_event(
                                plan.session_id.clone(),
                                active_turn.turn_id().to_string(),
                                tool_call_id.clone(),
                                tool_call_id.clone(),
                                Some(assistant_item_id.clone()),
                                now_string(),
                                delta.clone(),
                            ),
                        );

                        let _ = chat_store
                            .upsert_tool_call(PersistedToolCall {
                                id: tool_call_id,
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                parent_item_id: Some(assistant_item_id.clone()),
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

                        let completed_tool_call = execute_tool_call(
                            &chat_store,
                            &session_runtime,
                            &tool_executor,
                            &plan,
                            active_turn.turn_id(),
                            assistant_item_id.as_str(),
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
                                &session_runtime,
                                &build_image_generated_event(
                                    plan.session_id.clone(),
                                    active_turn.turn_id().to_string(),
                                    now_string(),
                                    media,
                                    Some(assistant_item_id.clone()),
                                ),
                            );
                        }

                        pass_state.completed_tool_calls.push(completed_tool_call);

                        let _ = upsert_message(
                            &chat_store,
                            PersistedMessage {
                                id: assistant_item_id.clone(),
                                user_id: plan.user_id.clone(),
                                session_id: plan.session_id.clone(),
                                turn_id: active_turn.turn_id().to_string(),
                                role: "assistant".into(),
                                status: "in_progress".into(),
                                content_json: assistant_text_to_content_json(
                                    assistant_accumulator.as_str(),
                                )
                                .to_string(),
                                tool_call_id: None,
                            },
                        )
                        .await;
                    }
                    Err(error) => {
                        fail_turn(
                            &chat_store,
                            &session_runtime,
                            &active_turn,
                            TurnTerminalReason::from_chat_service_error(&error),
                        )
                        .await;
                        return;
                    }
                }
            }

            if !pass_state.completed_tool_calls.is_empty() {
                if !included_user_message_in_history {
                    rolling_history.push(OutboundMessage {
                        role: "user".into(),
                        item_id: user_item_id.clone(),
                        turn_id: active_turn.turn_id().to_string(),
                        content: user_content_to_outbound_parts(
                            plan.prompt.as_str(),
                            plan.attachments.as_slice(),
                        ),
                        tool_calls: Vec::new(),
                        tool_call_id: None,
                    });
                    included_user_message_in_history = true;
                }

                rolling_history.push(OutboundMessage {
                    role: "assistant".into(),
                    item_id: format!("assistant_step_{step_index}"),
                    turn_id: active_turn.turn_id().to_string(),
                    content: if pass_state.assistant_text.trim().is_empty() {
                        Vec::new()
                    } else {
                        vec![OutboundContentPart::Text {
                            text: pass_state.assistant_text.clone(),
                        }]
                    },
                    tool_calls: pass_state
                        .completed_tool_calls
                        .iter()
                        .map(|call| OutboundToolCall {
                            id: call.tool_call_id.clone(),
                            name: call.tool_name.clone(),
                            arguments_text: call.arguments_text.clone(),
                        })
                        .collect(),
                    tool_call_id: None,
                });

                for tool_call in &pass_state.completed_tool_calls {
                    rolling_history.push(OutboundMessage {
                        role: "tool".into(),
                        item_id: format!("tool_result_{}", tool_call.tool_call_id),
                        turn_id: active_turn.turn_id().to_string(),
                        content: tool_result_to_outbound_parts(tool_call),
                        tool_calls: Vec::new(),
                        tool_call_id: Some(tool_call.tool_call_id.clone()),
                    });
                }

                pass_prompt.clear();
                pass_attachments.clear();
                continue;
            }

            break;
        }

        if reasoning_started_once {
            let _ = send_event(
                &session_runtime,
                &build_reasoning_completed_event(
                    plan.session_id.clone(),
                    active_turn.turn_id().to_string(),
                    reasoning_item_id.clone(),
                    now_string(),
                    build_reasoning_item(
                        reasoning_item_id.clone(),
                        active_turn.turn_id().to_string(),
                        "completed",
                        Some(reasoning_accumulator.clone()),
                    ),
                ),
            );

            let _ = upsert_message(
                &chat_store,
                PersistedMessage {
                    id: reasoning_item_id.clone(),
                    user_id: plan.user_id.clone(),
                    session_id: plan.session_id.clone(),
                    turn_id: active_turn.turn_id().to_string(),
                    role: "reasoning".into(),
                    status: "completed".into(),
                    content_json: json!([{ "type": "text", "text": reasoning_accumulator }])
                        .to_string(),
                    tool_call_id: None,
                },
            )
            .await;
        }

        let _ = upsert_message(
            &chat_store,
            PersistedMessage {
                id: assistant_item_id.clone(),
                user_id: plan.user_id.clone(),
                session_id: plan.session_id.clone(),
                turn_id: active_turn.turn_id().to_string(),
                role: "assistant".into(),
                status: "completed".into(),
                content_json: assistant_text_to_content_json(assistant_accumulator.as_str())
                    .to_string(),
                tool_call_id: None,
            },
        )
        .await;
        finalize_turn(
            &chat_store,
            &session_runtime,
            &active_turn,
            TurnTerminalState::Completed {
                started_at: turn_started_at.clone(),
            },
        )
        .await;
        session_title_generator.spawn_generate(plan, session_runtime);
    }
}

impl<R> TurnExecution for OpenChatTurnExecutor<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    fn run_turn(
        &self,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: ActiveTurnHandle,
    ) -> TurnExecutionFuture {
        let chat_store = self.chat_store.clone();
        let model_provider_runtime = self.model_provider_runtime.clone();
        let tool_executor = self.tool_executor.clone();

        Box::pin(Self::execute_turn(
            chat_store,
            model_provider_runtime,
            tool_executor,
            self.session_title_generator.clone(),
            plan,
            session_runtime,
            active_turn,
        ))
    }
}

impl<R> TurnRunner for OpenChatTurnExecutor<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    fn spawn_run(
        &self,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: ActiveTurnHandle,
    ) {
        let chat_store = self.chat_store.clone();
        let model_provider_runtime = self.model_provider_runtime.clone();
        let tool_executor = self.tool_executor.clone();
        let session_title_generator = self.session_title_generator.clone();
        tokio::spawn(async move {
            Self::execute_turn(
                chat_store,
                model_provider_runtime,
                tool_executor,
                session_title_generator,
                plan,
                session_runtime,
                active_turn,
            )
            .await;
        });
    }
}

async fn fail_turn(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    active_turn: &ActiveTurnHandle,
    reason: TurnTerminalReason,
) {
    finalize_turn(
        chat_store,
        session_runtime,
        active_turn,
        TurnTerminalState::Failed { reason },
    )
    .await;
}

async fn interrupt_turn(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    active_turn: &ActiveTurnHandle,
    turn_started_at: &str,
) {
    finalize_turn(
        chat_store,
        session_runtime,
        active_turn,
        TurnTerminalState::Interrupted {
            started_at: turn_started_at.to_string(),
            reason: TurnTerminalReason::user_requested(),
        },
    )
    .await;
}

async fn upsert_message(chat_store: &ChatStore, message: PersistedMessage) -> anyhow::Result<()> {
    chat_store.upsert_message(message).await
}

fn tool_result_to_history_text(call: &CompletedToolCall) -> String {
    format_tool_result_text(
        call.tool_display_name.as_str(),
        if call.failed { "failed" } else { "completed" },
        Some(call.arguments_text.as_str()),
        Some(&call.result),
        call.media.as_slice(),
    )
}

fn tool_result_to_outbound_parts(call: &CompletedToolCall) -> Vec<OutboundContentPart> {
    let mut parts = vec![OutboundContentPart::Text {
        text: tool_result_to_history_text(call),
    }];
    append_image_media_parts(&mut parts, call.media.as_slice());

    parts
}

async fn execute_tool_call<R>(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    tool_executor: &ToolExecutor<R>,
    plan: &TurnPlan,
    turn_id: &str,
    assistant_item_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    arguments_text: String,
) -> CompletedToolCall
where
    R: ImageModelAccessResolver + ToolAccessResolver,
{
    let tool = match plan.tool_list.iter().find(|tool| tool.id == tool_name) {
        Some(tool) => tool.clone(),
        None => {
            let result = json!({
                "kind": "tool_error",
                "message": format!("Tool `{tool_name}` is not enabled for this turn"),
            });
            let completed = CompletedToolCall {
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
                tool_display_name: tool_name.to_string(),
                arguments_text: arguments_text.clone(),
                result: result.clone(),
                media: Vec::new(),
                failed: true,
            };
            persist_completed_tool_call(
                chat_store,
                session_runtime,
                plan,
                turn_id,
                assistant_item_id,
                &completed,
            )
            .await;
            return completed;
        }
    };

    let execution = tool_executor
        .execute(ToolInvocation {
            user_id: plan.user_id.clone(),
            session_id: plan.session_id.clone(),
            turn_id: turn_id.to_string(),
            tool_call_id: tool_call_id.to_string(),
            arguments_text: arguments_text.clone(),
            tool: tool.clone(),
        })
        .await;

    let completed = match execution {
        Ok(ToolExecutionResult { media, result }) => CompletedToolCall {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool.id.clone(),
            tool_display_name: tool.display_name.clone(),
            arguments_text,
            result: json!({
                "kind": "tool_result",
                "tool": tool.id,
                "output": result,
            }),
            media,
            failed: false,
        },
        Err(error) => CompletedToolCall {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool.id.clone(),
            tool_display_name: tool.display_name.clone(),
            arguments_text,
            result: json!({
                "kind": "tool_error",
                "message": error.message,
            }),
            media: Vec::new(),
            failed: true,
        },
    };

    persist_completed_tool_call(
        chat_store,
        session_runtime,
        plan,
        turn_id,
        assistant_item_id,
        &completed,
    )
    .await;

    completed
}

async fn persist_completed_tool_call(
    chat_store: &ChatStore,
    session_runtime: &SessionRuntime,
    plan: &TurnPlan,
    turn_id: &str,
    assistant_item_id: &str,
    completed: &CompletedToolCall,
) {
    let _ = send_event(
        session_runtime,
        &build_tool_call_completed_event(
            plan.session_id.clone(),
            turn_id.to_string(),
            completed.tool_call_id.clone(),
            now_string(),
            build_tool_call_item(
                completed.tool_call_id.clone(),
                turn_id.to_string(),
                if completed.failed {
                    "failed"
                } else {
                    "completed"
                },
                completed.tool_call_id.clone(),
                Some(assistant_item_id.to_string()),
                completed.tool_name.clone(),
                Some(completed.tool_display_name.clone()),
                Some(completed.arguments_text.clone()),
                Some(completed.result.clone()),
                (!completed.media.is_empty()).then_some(completed.media.clone()),
            ),
        ),
    );

    let _ = chat_store
        .upsert_tool_call(PersistedToolCall {
            id: completed.tool_call_id.clone(),
            user_id: plan.user_id.clone(),
            session_id: plan.session_id.clone(),
            turn_id: turn_id.to_string(),
            parent_item_id: Some(assistant_item_id.to_string()),
            tool_name: completed.tool_name.clone(),
            tool_display_name: Some(completed.tool_display_name.clone()),
            arguments_text: Some(completed.arguments_text.clone()),
            result_json: Some(completed.result.to_string()),
            status: if completed.failed {
                "failed".into()
            } else {
                "completed".into()
            },
            media_json: if completed.media.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&completed.media).unwrap_or_else(|_| "[]".to_string()))
            },
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::{tool_result_to_history_text, tool_result_to_outbound_parts, CompletedToolCall};
    use crate::{MediaAsset, OutboundContentPart};
    use serde_json::json;

    fn sample_completed_tool_call(image_urls: &[&str]) -> CompletedToolCall {
        let media = image_urls
            .iter()
            .map(|url| MediaAsset {
                kind: "image".into(),
                url: (*url).to_string(),
                object_key: None,
                mime_type: "image/png".into(),
                size_bytes: 128,
            })
            .collect::<Vec<_>>();
        CompletedToolCall {
            tool_call_id: "call_1".into(),
            tool_name: "image_generation".into(),
            tool_display_name: "Image Generation".into(),
            arguments_text: "{\"prompt\":\"cat\"}".into(),
            result: json!({
                "kind": "tool_result",
                "output": {
                    "count": media.len(),
                }
            }),
            media,
            failed: false,
        }
    }

    #[test]
    fn tool_history_parts_include_generated_image_url() {
        let parts = tool_result_to_outbound_parts(&sample_completed_tool_call(&[
            "https://example.com/generated.png",
            "https://example.com/generated-2.png",
        ]));

        assert_eq!(parts.len(), 3);
        assert!(matches!(parts[0], OutboundContentPart::Text { .. }));
        assert!(matches!(
            &parts[1],
            OutboundContentPart::ImageUrl { url, .. } if url == "https://example.com/generated.png"
        ));
        assert!(matches!(
            &parts[2],
            OutboundContentPart::ImageUrl { url, .. } if url == "https://example.com/generated-2.png"
        ));
    }

    #[test]
    fn tool_history_text_keeps_image_reference_out_of_plain_text() {
        let text = tool_result_to_history_text(&sample_completed_tool_call(&[
            "https://example.com/generated.png",
        ]));

        assert!(text.contains("image_attachment: 1 image(s) available"));
        assert!(!text.contains("https://example.com/generated.png"));
    }
}
