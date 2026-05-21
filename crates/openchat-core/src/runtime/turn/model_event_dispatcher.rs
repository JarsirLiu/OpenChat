use std::collections::BTreeMap;

use crate::{
    runtime::turn::{
        loop_step_result::{InProgressToolCall, LoopStepResult},
        tool_call_coordinator::ToolCallCoordinator,
        transcript_projector::{ProjectionContext, TranscriptProjector},
    },
    ImageModelAccessResolver, ModelStreamEvent, SessionRuntime, TextModelAccessResolver,
    ToolAccessResolver, TurnPlan, TurnTerminalReason,
};

pub(crate) struct ModelEventDispatcher<'a, R> {
    transcript_projector: &'a TranscriptProjector,
    tool_call_coordinator: &'a ToolCallCoordinator<R>,
    session_runtime: &'a SessionRuntime,
    pass_plan: &'a TurnPlan,
    projection_context: ProjectionContext<'a>,
    reasoning_item_id: &'a str,
    reasoning_accumulator: &'a mut String,
    reasoning_started_once: &'a mut bool,
    current_assistant_item_id: Option<String>,
    current_assistant_text: String,
    in_progress_tool_calls: BTreeMap<String, InProgressToolCall>,
    pass_state: LoopStepResult,
}

impl<'a, R> ModelEventDispatcher<'a, R>
where
    R: TextModelAccessResolver + ImageModelAccessResolver + ToolAccessResolver + 'static,
{
    pub fn new(
        transcript_projector: &'a TranscriptProjector,
        tool_call_coordinator: &'a ToolCallCoordinator<R>,
        session_runtime: &'a SessionRuntime,
        pass_plan: &'a TurnPlan,
        projection_context: ProjectionContext<'a>,
        reasoning_item_id: &'a str,
        reasoning_accumulator: &'a mut String,
        reasoning_started_once: &'a mut bool,
    ) -> Self {
        Self {
            transcript_projector,
            tool_call_coordinator,
            session_runtime,
            pass_plan,
            projection_context,
            reasoning_item_id,
            reasoning_accumulator,
            reasoning_started_once,
            current_assistant_item_id: None,
            current_assistant_text: String::new(),
            in_progress_tool_calls: BTreeMap::new(),
            pass_state: LoopStepResult::default(),
        }
    }

    pub async fn handle_event(
        &mut self,
        event: ModelStreamEvent,
    ) -> Result<(), TurnTerminalReason> {
        match event {
            ModelStreamEvent::ReasoningDelta(delta) => {
                if !*self.reasoning_started_once {
                    *self.reasoning_started_once = true;
                    self.transcript_projector
                        .project_reasoning_started(
                            self.session_runtime,
                            self.projection_context,
                            self.reasoning_item_id,
                        )
                        .await
                        .map_err(|error| {
                            TurnTerminalReason::transcript_projection_failed(error.to_string())
                        })?;
                }

                self.pass_state.reasoning_started = true;
                self.pass_state.reasoning_text.push_str(delta.as_str());
                self.reasoning_accumulator.push_str(delta.as_str());

                self.transcript_projector
                    .project_reasoning_delta(
                        self.session_runtime,
                        self.projection_context,
                        self.reasoning_item_id,
                        delta.as_str(),
                        self.reasoning_accumulator.as_str(),
                    )
                    .await
                    .map_err(|error| {
                        TurnTerminalReason::transcript_projection_failed(error.to_string())
                    })?;
            }
            ModelStreamEvent::TextDelta(delta) => {
                self.pass_state.assistant_text.push_str(delta.as_str());
                if self.current_assistant_item_id.is_none() {
                    let assistant_item_id = self.transcript_projector.new_assistant_item_id();
                    self.current_assistant_item_id = Some(assistant_item_id.clone());
                    self.current_assistant_text.clear();

                    self.transcript_projector
                        .project_assistant_started(
                            self.session_runtime,
                            self.projection_context,
                            assistant_item_id.as_str(),
                        )
                        .await
                        .map_err(|error| {
                            TurnTerminalReason::transcript_projection_failed(error.to_string())
                        })?;
                }

                self.current_assistant_text.push_str(delta.as_str());
                let assistant_item_id = self
                    .current_assistant_item_id
                    .as_deref()
                    .expect("assistant item id must exist before text delta");

                self.transcript_projector
                    .project_assistant_delta(
                        self.session_runtime,
                        self.projection_context,
                        assistant_item_id,
                        delta.as_str(),
                        self.current_assistant_text.as_str(),
                    )
                    .await
                    .map_err(|error| {
                        TurnTerminalReason::transcript_projection_failed(error.to_string())
                    })?;
            }
            ModelStreamEvent::ToolCallStart {
                tool_call_id,
                tool_name,
                arguments,
            } => {
                let is_image_generation = self
                    .pass_plan
                    .tool_list
                    .iter()
                    .find(|tool| tool.id == tool_name)
                    .map(|tool| tool.tool_type.eq_ignore_ascii_case("image"))
                    .unwrap_or(false);

                self.in_progress_tool_calls.insert(
                    tool_call_id.clone(),
                    InProgressToolCall {
                        tool_name: tool_name.clone(),
                        arguments_text: String::new(),
                    },
                );

                self.transcript_projector
                    .project_tool_call_started(
                        self.session_runtime,
                        self.projection_context,
                        tool_call_id.as_str(),
                        self.current_assistant_item_id.clone(),
                        tool_name.as_str(),
                        is_image_generation,
                        arguments,
                    )
                    .await
                    .map_err(|error| {
                        TurnTerminalReason::transcript_projection_failed(error.to_string())
                    })?;
            }
            ModelStreamEvent::ToolCallArgumentsDelta {
                tool_call_id,
                delta,
            } => {
                let entry = self
                    .in_progress_tool_calls
                    .entry(tool_call_id.clone())
                    .or_insert_with(InProgressToolCall::default);
                entry.arguments_text.push_str(delta.as_str());

                self.transcript_projector
                    .project_tool_call_arguments_delta(
                        self.session_runtime,
                        self.projection_context,
                        tool_call_id.as_str(),
                        self.current_assistant_item_id.clone(),
                        delta.as_str(),
                    )
                    .await
                    .map_err(|error| {
                        TurnTerminalReason::transcript_projection_failed(error.to_string())
                    })?;
            }
            ModelStreamEvent::ToolCallComplete {
                tool_call_id,
                tool_name,
                arguments_text,
            } => {
                let pending = self.in_progress_tool_calls.remove(&tool_call_id);
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
                    .execute_and_project(
                        self.transcript_projector,
                        self.session_runtime,
                        self.pass_plan,
                        &tool_call_id,
                        self.projection_context,
                        self.current_assistant_item_id.as_deref(),
                        &resolved_tool_name,
                        resolved_arguments_text,
                    )
                    .await
                    .map_err(|error| {
                        TurnTerminalReason::transcript_projection_failed(error.to_string())
                    })?;

                self.pass_state
                    .completed_tool_calls
                    .push(completed_tool_call);
                self.current_assistant_item_id = None;
                self.current_assistant_text.clear();
            }
        }

        Ok(())
    }

    pub fn into_step_result(self) -> LoopStepResult {
        self.pass_state
    }
}
