use std::sync::Arc;

use openchat_infra::stores::ChatStore;

use crate::{
    user_content_to_json,
    runtime::turn::{
        event_builder::{
            build_message_item,
            build_message_started_event, build_reasoning_completed_event,
            build_reasoning_item, build_turn, build_turn_started_event, send_event,
        },
        helpers::now_string,
        lifecycle::{emit_session_updated, finalize_turn, TurnTerminalState},
        message_writer::MessageWriter,
        session_title::SessionTitleGenerator,
        tool_call_coordinator::ToolCallCoordinator,
        turn_loop::{TurnLoop, TurnLoopExit},
    },
    ActiveTurnHandle, ImageModelAccessResolver, ModelProviderRuntime, SessionRuntime,
    TextModelAccessResolver, ToolAccessResolver,
    ToolExecutor, TurnPlan, TurnRunner, TurnTerminalReason,
};

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
    tool_call_coordinator: ToolCallCoordinator<R>,
    session_title_generator: SessionTitleGenerator<R>,
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
            tool_call_coordinator: ToolCallCoordinator::new(tool_executor),
            session_title_generator,
        }
    }

    async fn execute_turn(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_call_coordinator: ToolCallCoordinator<R>,
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

        let message_writer = MessageWriter::new(chat_store.clone());
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

        message_writer
            .write_user_completed(
                user_item_id.as_str(),
                plan.user_id.as_str(),
                plan.session_id.as_str(),
                active_turn.turn_id(),
                &user_content,
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

        message_writer
            .write_assistant_started(
                assistant_item_id.as_str(),
                plan.user_id.as_str(),
                plan.session_id.as_str(),
                active_turn.turn_id(),
            )
            .await;

        let turn_loop = TurnLoop::new(
            chat_store.clone(),
            model_provider_runtime,
            tool_call_coordinator,
            message_writer.clone(),
        );

        let loop_result = turn_loop
            .run(
                &plan,
                &session_runtime,
                &active_turn,
                user_item_id.as_str(),
                assistant_item_id.as_str(),
                reasoning_item_id.as_str(),
            )
            .await;

        let loop_result = match loop_result {
            TurnLoopExit::Completed(result) => result,
            TurnLoopExit::Interrupted => {
                interrupt_turn(
                    &chat_store,
                    &session_runtime,
                    &active_turn,
                    turn_started_at.as_str(),
                )
                .await;
                return;
            }
            TurnLoopExit::Failed(reason) => {
                fail_turn(&chat_store, &session_runtime, &active_turn, reason).await;
                return;
            }
        };

        if loop_result.reasoning_started_once {
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
                        Some(loop_result.reasoning_text.clone()),
                    ),
                ),
            );

            message_writer
                .write_reasoning_completed(
                    reasoning_item_id.as_str(),
                    plan.user_id.as_str(),
                    plan.session_id.as_str(),
                    active_turn.turn_id(),
                    loop_result.reasoning_text.as_str(),
                )
                .await;
        }

        message_writer
            .write_assistant_completed(
                assistant_item_id.as_str(),
                plan.user_id.as_str(),
                plan.session_id.as_str(),
                active_turn.turn_id(),
                loop_result.assistant_text.as_str(),
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
        let tool_call_coordinator = self.tool_call_coordinator.clone();
        let session_title_generator = self.session_title_generator.clone();
        tokio::spawn(async move {
            Self::execute_turn(
                chat_store,
                model_provider_runtime,
                tool_call_coordinator,
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
