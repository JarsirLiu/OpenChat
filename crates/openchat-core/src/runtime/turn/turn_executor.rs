use std::sync::Arc;

use openchat_infra::stores::ChatStore;
use tracing::warn;

use crate::{
    runtime::turn::{
        event_builder::{build_turn, build_turn_started_event, send_event},
        helpers::{now_millis, now_string},
        lifecycle::{emit_session_updated, finalize_turn, TurnTerminalState},
        session_title::SessionTitleGenerator,
        tool_call_coordinator::ToolCallCoordinator,
        transcript_projector::{ProjectionContext, TranscriptProjector},
        turn_loop::{TurnLoop, TurnLoopExit},
    },
    user_content_to_json, ActiveTurnHandle, ImageModelAccessResolver, ModelProviderRuntime,
    SessionRuntime, TextModelAccessResolver, ToolAccessResolver, ToolExecutor, TurnPlan,
    TurnRunner, TurnTerminalReason, UserSessionRetentionPort, UserTurnRetentionPort,
};

const MAX_PERSISTED_TURNS_PER_USER: usize = 200;
const MAX_PERSISTED_SESSIONS_PER_USER: usize = 30;

#[derive(Clone)]
pub struct OpenChatTurnExecutor<R> {
    chat_store: Arc<ChatStore>,
    model_provider_runtime: ModelProviderRuntime<R>,
    tool_call_coordinator: ToolCallCoordinator<R>,
    session_title_generator: SessionTitleGenerator<R>,
    turn_retention: Arc<dyn UserTurnRetentionPort>,
    session_retention: Arc<dyn UserSessionRetentionPort>,
}

impl<R> OpenChatTurnExecutor<R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    pub fn new(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_executor: ToolExecutor<R>,
        turn_retention: Arc<dyn UserTurnRetentionPort>,
        session_retention: Arc<dyn UserSessionRetentionPort>,
    ) -> Self {
        let session_title_generator =
            SessionTitleGenerator::new(chat_store.clone(), model_provider_runtime.clone());
        Self {
            chat_store,
            model_provider_runtime,
            tool_call_coordinator: ToolCallCoordinator::new(tool_executor),
            session_title_generator,
            turn_retention,
            session_retention,
        }
    }

    async fn execute_turn(
        chat_store: Arc<ChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
        tool_call_coordinator: ToolCallCoordinator<R>,
        session_title_generator: SessionTitleGenerator<R>,
        turn_retention: Arc<dyn UserTurnRetentionPort>,
        session_retention: Arc<dyn UserSessionRetentionPort>,
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

        let transcript_projector = TranscriptProjector::new(chat_store.clone());
        let user_content = user_content_to_json(plan.prompt.as_str(), plan.attachments.as_slice());

        if let Err(reason) = transcript_projector
            .project_user_message(
                &session_runtime,
                ProjectionContext {
                    user_id: plan.user_id.as_str(),
                    session_id: plan.session_id.as_str(),
                    turn_id: active_turn.turn_id(),
                },
                user_item_id.as_str(),
                plan.prompt.as_str(),
                &user_content,
            )
            .await
            .map_err(|error| TurnTerminalReason::transcript_projection_failed(error.to_string()))
        {
            fail_turn(&chat_store, &session_runtime, &active_turn, reason).await;
            spawn_retention_cleanup(
                turn_retention.clone(),
                session_retention.clone(),
                plan.user_id.clone(),
            );
            return;
        }

        let turn_loop = TurnLoop::new(
            model_provider_runtime,
            tool_call_coordinator,
            transcript_projector.clone(),
        );
        session_title_generator.spawn_generate(plan.clone(), session_runtime.clone());

        let loop_result = turn_loop
            .run(
                &plan,
                &session_runtime,
                &active_turn,
                user_item_id.as_str(),
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
                spawn_retention_cleanup(
                    turn_retention.clone(),
                    session_retention.clone(),
                    plan.user_id.clone(),
                );
                return;
            }
            TurnLoopExit::Failed(reason) => {
                fail_turn(&chat_store, &session_runtime, &active_turn, reason).await;
                spawn_retention_cleanup(
                    turn_retention.clone(),
                    session_retention.clone(),
                    plan.user_id.clone(),
                );
                return;
            }
        };

        if loop_result.reasoning_started_once {
            if let Err(reason) = transcript_projector
                .project_reasoning_completed(
                    &session_runtime,
                    ProjectionContext {
                        user_id: plan.user_id.as_str(),
                        session_id: plan.session_id.as_str(),
                        turn_id: active_turn.turn_id(),
                    },
                    reasoning_item_id.as_str(),
                    loop_result.reasoning_text.as_str(),
                )
                .await
                .map_err(|error| {
                    TurnTerminalReason::transcript_projection_failed(error.to_string())
                })
            {
                fail_turn(&chat_store, &session_runtime, &active_turn, reason).await;
                spawn_retention_cleanup(
                    turn_retention.clone(),
                    session_retention.clone(),
                    plan.user_id.clone(),
                );
                return;
            }
        }

        finalize_turn(
            &chat_store,
            &session_runtime,
            &active_turn,
            TurnTerminalState::Completed {
                started_at: turn_started_at.clone(),
            },
        )
        .await;
        spawn_retention_cleanup(turn_retention, session_retention, plan.user_id.clone());
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
        let turn_retention = self.turn_retention.clone();
        let session_retention = self.session_retention.clone();
        tokio::spawn(async move {
            Self::execute_turn(
                chat_store,
                model_provider_runtime,
                tool_call_coordinator,
                session_title_generator,
                turn_retention,
                session_retention,
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

fn spawn_retention_cleanup(
    turn_retention: Arc<dyn UserTurnRetentionPort>,
    session_retention: Arc<dyn UserSessionRetentionPort>,
    user_id: String,
) {
    tokio::spawn(async move {
        if let Err(error) = turn_retention
            .enforce_user_turn_limit(user_id.as_str(), MAX_PERSISTED_TURNS_PER_USER)
            .await
        {
            warn!(user_id = %user_id, error = %error, "failed to enforce user turn retention");
        }
        if let Err(error) = session_retention
            .enforce_user_session_limit(user_id.as_str(), MAX_PERSISTED_SESSIONS_PER_USER)
            .await
        {
            warn!(user_id = %user_id, error = %error, "failed to enforce user session retention");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        spawn_retention_cleanup, MAX_PERSISTED_SESSIONS_PER_USER, MAX_PERSISTED_TURNS_PER_USER,
    };
    use crate::{UserSessionRetentionPort, UserTurnRetentionPort};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use tokio::sync::Notify;

    #[derive(Default)]
    struct MockTurnRetention {
        calls: AtomicUsize,
        max_turns: Mutex<Vec<usize>>,
        notify: Notify,
    }

    impl UserTurnRetentionPort for MockTurnRetention {
        fn enforce_user_turn_limit<'a>(
            &'a self,
            _user_id: &'a str,
            max_turns: usize,
        ) -> core::pin::Pin<
            Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.max_turns.lock().unwrap().push(max_turns);
                self.notify.notify_waiters();
                Ok(())
            })
        }
    }

    #[derive(Default)]
    struct MockSessionRetention {
        calls: AtomicUsize,
        max_sessions: Mutex<Vec<usize>>,
        notify: Notify,
    }

    impl UserSessionRetentionPort for MockSessionRetention {
        fn enforce_user_session_limit<'a>(
            &'a self,
            _user_id: &'a str,
            max_sessions: usize,
        ) -> core::pin::Pin<
            Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.max_sessions.lock().unwrap().push(max_sessions);
                self.notify.notify_waiters();
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn retention_cleanup_triggers_turn_and_session_limits() {
        let turn_retention = Arc::new(MockTurnRetention::default());
        let session_retention = Arc::new(MockSessionRetention::default());

        spawn_retention_cleanup(
            turn_retention.clone(),
            session_retention.clone(),
            "user_1".to_string(),
        );

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            async {
                loop {
                    if turn_retention.calls.load(Ordering::SeqCst) > 0
                        && session_retention.calls.load(Ordering::SeqCst) > 0
                    {
                        break;
                    }
                    tokio::select! {
                        _ = turn_retention.notify.notified() => {}
                        _ = session_retention.notify.notified() => {}
                    }
                }
            },
        )
        .await
        .expect("retention cleanup should complete");

        assert_eq!(turn_retention.calls.load(Ordering::SeqCst), 1);
        assert_eq!(session_retention.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            turn_retention.max_turns.lock().unwrap().as_slice(),
            &[MAX_PERSISTED_TURNS_PER_USER]
        );
        assert_eq!(
            session_retention.max_sessions.lock().unwrap().as_slice(),
            &[MAX_PERSISTED_SESSIONS_PER_USER]
        );
    }
}
