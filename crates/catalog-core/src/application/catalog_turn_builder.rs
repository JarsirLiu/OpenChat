use openchat_core::{
    ChatRequest, ChatServiceError, SessionContext, TurnAttachment, TurnBuilder, TurnModelRef,
    TurnPlan, TurnToolRef,
};

use crate::application::catalog::CatalogService;

#[derive(Clone)]
pub struct CatalogTurnBuilder {
    catalog_service: CatalogService,
}

impl CatalogTurnBuilder {
    pub fn new(catalog_service: CatalogService) -> Self {
        Self { catalog_service }
    }
}

impl TurnBuilder for CatalogTurnBuilder {
    fn build_turn(
        &self,
        request: ChatRequest,
        context: SessionContext,
    ) -> Result<TurnPlan, ChatServiceError> {
        let prompt = request.prompt.trim().to_string();
        if prompt.is_empty() && request.attachments.is_empty() {
            return Err(ChatServiceError::validation(
                "A user prompt or image attachment is required",
            ));
        }

        let selected_text_model = request
            .text_model
            .as_ref()
            .ok_or_else(|| ChatServiceError::validation("A text model must be selected"))?;

        let text_model = self
            .catalog_service
            .resolve_text_model(selected_text_model.model_config_id.as_str())
            .or_else(|| resolve_custom_text_model(selected_text_model))
            .ok_or_else(|| {
                ChatServiceError::model_unavailable("Selected text model is not available")
            })?;

        let tool_list = request
            .tool_list
            .iter()
            .map(|tool| {
                self.catalog_service
                    .resolve_image_tool(tool.model_config_id.as_str(), tool.id.as_str())
                    .or_else(|| resolve_custom_image_tool(tool))
                    .ok_or_else(|| {
                        ChatServiceError::model_unavailable(
                            "Selected image tool model is not available",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TurnPlan {
            user_id: request.user_id,
            session_id: request.session_id,
            prompt,
            attachments: request
                .attachments
                .into_iter()
                .map(|attachment| TurnAttachment {
                    id: attachment.id,
                    url: attachment.url,
                    name: attachment.name,
                    mime_type: attachment.mime_type,
                    size_bytes: attachment.size_bytes,
                })
                .collect(),
            history: context.history,
            text_model: TurnModelRef {
                runtime_provider: text_model.runtime_provider,
                model_config_id: text_model.model_config_id,
                model_name: text_model.model,
                display_name: text_model.display_name,
                provider: text_model.provider,
                source: text_model.source,
                input_modalities: text_model.input_modalities,
            },
            tool_list: tool_list
                .into_iter()
                .map(|tool| TurnToolRef {
                    runtime_provider: tool.runtime_provider,
                    model_config_id: tool.model_config_id,
                    model_name: tool.model_name,
                    id: tool.id,
                    display_name: tool.display_name,
                    provider: tool.provider,
                    source: tool.source,
                    tool_type: tool.tool_type,
                })
                .collect(),
        })
    }
}

fn resolve_custom_text_model(
    selected_text_model: &openchat_core::SelectedTextModel,
) -> Option<crate::domain::models::CatalogModel> {
    if selected_text_model.source.as_deref() != Some("custom") {
        return None;
    }

    let model_type = selected_text_model.model_type.clone()?;
    if !matches!(model_type.as_str(), "text" | "multimodal") {
        return None;
    }

    let provider = selected_text_model.provider.clone()?;
    let runtime_provider = selected_text_model.runtime_provider.clone()?;
    let model_name = selected_text_model.model_name.clone()?;

    Some(crate::domain::models::CatalogModel {
        model_config_id: selected_text_model.model_config_id.clone(),
        provider: provider.clone(),
        runtime_provider,
        display_provider: provider,
        model: model_name.clone(),
        display_name: selected_text_model
            .display_name
            .clone()
            .unwrap_or(model_name),
        source: "custom".to_string(),
        model_type,
        input_modalities: if selected_text_model.input_modalities.is_empty() {
            vec!["text".to_string()]
        } else {
            selected_text_model.input_modalities.clone()
        },
        official: false,
        custom: true,
    })
}

fn resolve_custom_image_tool(
    selected_tool: &openchat_core::SelectedTool,
) -> Option<openchat_core::CatalogTool> {
    if selected_tool.source.as_deref() != Some("custom") || selected_tool.tool_type != "image" {
        return None;
    }

    let provider = selected_tool.provider.clone()?;
    let runtime_provider = selected_tool.runtime_provider.clone()?;
    let model_name = selected_tool.model_name.clone()?;

    Some(openchat_core::CatalogTool {
        model_config_id: selected_tool.model_config_id.clone(),
        model_name: model_name.clone(),
        id: selected_tool.id.clone(),
        provider: provider.clone(),
        runtime_provider,
        display_provider: provider,
        source: "custom".to_string(),
        tool_type: "image".to_string(),
        display_name: selected_tool.display_name.clone().unwrap_or(model_name),
    })
}
