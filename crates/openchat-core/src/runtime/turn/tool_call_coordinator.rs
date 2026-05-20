use openchat_infra::stores::{ChatStore, PersistedToolCall};
use serde_json::json;

use crate::{
    tool_result_to_content_json, OutboundToolResult,
    runtime::turn::{
        event_builder::{build_tool_call_completed_event, build_tool_call_item, send_event},
        helpers::now_string,
        loop_step_result::CompletedToolCall,
    },
    ImageModelAccessResolver, SessionRuntime, ToolAccessResolver, ToolExecutionResult,
    ToolExecutor, ToolInvocation, TurnPlan,
};

pub(crate) struct ToolCallCoordinator<R> {
    tool_executor: ToolExecutor<R>,
}

impl<R> Clone for ToolCallCoordinator<R> {
    fn clone(&self) -> Self {
        Self {
            tool_executor: self.tool_executor.clone(),
        }
    }
}

impl<R> ToolCallCoordinator<R>
where
    R: ImageModelAccessResolver + ToolAccessResolver,
{
    pub fn new(tool_executor: ToolExecutor<R>) -> Self {
        Self { tool_executor }
    }

    pub async fn execute_and_persist(
        &self,
        chat_store: &ChatStore,
        session_runtime: &SessionRuntime,
        plan: &TurnPlan,
        turn_id: &str,
        assistant_item_id: &str,
        tool_call_id: &str,
        tool_name: &str,
        arguments_text: String,
    ) -> CompletedToolCall {
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
                self.persist_completed_tool_call(
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

        let execution = self
            .tool_executor
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

        self.persist_completed_tool_call(
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
        &self,
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
                    tool_result_to_content_json(&OutboundToolResult {
                        tool_call_id: completed.tool_call_id.clone(),
                        tool_name: completed.tool_name.clone(),
                        tool_display_name: Some(completed.tool_display_name.clone()),
                        status: if completed.failed {
                            "failed".into()
                        } else {
                            "completed".into()
                        },
                        arguments_text: Some(completed.arguments_text.clone()),
                        result: completed.result.clone(),
                        media: completed.media.clone(),
                    }),
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
                    Some(
                        serde_json::to_string(&completed.media)
                            .unwrap_or_else(|_| "[]".to_string()),
                    )
                },
            })
            .await;
    }
}
