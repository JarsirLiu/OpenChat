use anyhow::Result;
use serde_json::json;

use crate::{
    runtime::turn::{
        loop_step_result::CompletedToolCall,
        transcript_projector::{ProjectionContext, TranscriptProjector},
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

    pub async fn execute_and_project(
        &self,
        transcript_projector: &TranscriptProjector,
        session_runtime: &SessionRuntime,
        plan: &TurnPlan,
        tool_call_id: &str,
        context: ProjectionContext<'_>,
        assistant_item_id: Option<&str>,
        tool_name: &str,
        arguments_text: String,
    ) -> Result<CompletedToolCall> {
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
                transcript_projector
                    .project_tool_call_completed(
                        session_runtime,
                        ProjectionContext {
                            user_id: context.user_id,
                            session_id: context.session_id,
                            turn_id: context.turn_id,
                        },
                        assistant_item_id,
                        &completed,
                        false,
                    )
                    .await?;
                return Ok(completed);
            }
        };
        let is_image_generation = tool.tool_type.eq_ignore_ascii_case("image");

        let execution = self
            .tool_executor
            .execute(ToolInvocation {
                user_id: plan.user_id.clone(),
                session_id: plan.session_id.clone(),
                turn_id: context.turn_id.to_string(),
                tool_call_id: tool_call_id.to_string(),
                arguments_text: arguments_text.clone(),
                current_attachments: plan.attachments.clone(),
                history: plan.history.clone(),
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

        transcript_projector
            .project_tool_call_completed(
                session_runtime,
                ProjectionContext {
                    user_id: context.user_id,
                    session_id: context.session_id,
                    turn_id: context.turn_id,
                },
                assistant_item_id,
                &completed,
                is_image_generation,
            )
            .await?;

        Ok(completed)
    }
}
