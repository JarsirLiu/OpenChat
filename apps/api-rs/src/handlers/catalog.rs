use axum::{extract::State, response::IntoResponse, Json};
use openchat_catalog_core::CatalogModel;
use openchat_core::CatalogTool;

use crate::{
    http::catalog::{CatalogModelDto, CatalogToolDto},
    http::errors::chat_service_error_response_from_error,
    security::extractors::CurrentUser,
    state::AppState,
};

fn build_model_dto_with_access(
    model: CatalogModel,
    access_result: Result<openchat_core::ResolvedTextModelAccess, openchat_core::ChatServiceError>,
) -> CatalogModelDto {
    let mut dto = CatalogModelDto::from(model);
    match access_result {
        Ok(_) => {
            dto.available = true;
            dto.unavailable_reason = None;
        }
        Err(error) if error.status_code < 500 => {
            dto.available = false;
            dto.unavailable_reason = Some(error.message);
        }
        Err(error) => {
            dto.available = false;
            dto.unavailable_reason = Some("模型当前不可用，请稍后重试".to_string());
            tracing::warn!("failed to inspect model availability: {}", error.message);
        }
    }
    dto
}

fn build_tool_dto_with_access(
    tool: CatalogTool,
    access_result: Result<openchat_core::ToolAccessDecision, openchat_core::ChatServiceError>,
) -> CatalogToolDto {
    let mut dto = CatalogToolDto {
        model_config_id: tool.model_config_id,
        id: tool.id,
        provider: tool.provider,
        display_provider: tool.display_provider,
        model: tool.model_name,
        source: tool.source,
        r#type: tool.tool_type,
        display_name: tool.display_name,
        available: true,
        enabled: true,
        unavailable_reason: None,
    };

    match access_result {
        Ok(decision) => {
            dto.available = decision.visible;
            dto.enabled = decision.enabled;
            dto.unavailable_reason = decision.reason;
        }
        Err(error) if error.status_code < 500 => {
            dto.available = false;
            dto.enabled = false;
            dto.unavailable_reason = Some(error.message);
        }
        Err(error) => {
            dto.available = false;
            dto.enabled = false;
            dto.unavailable_reason = Some("工具当前不可用，请稍后重试".to_string());
            tracing::warn!("failed to inspect tool availability: {}", error.message);
        }
    }

    dto
}

fn custom_image_tool_id(model_config_id: &str) -> String {
    let mut normalized = String::with_capacity(model_config_id.len());
    for ch in model_config_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            normalized.push(ch);
        } else {
            normalized.push('_');
        }
    }
    format!("image_gen_custom_{normalized}")
}

pub async fn list_models(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> impl IntoResponse {
    let mut model_dtos = Vec::new();
    for model in state.catalog_service.list_models() {
        let access = state
            .account_service
            .resolve_text_access(
                auth.user_id(),
                &openchat_core::TurnModelRef {
                    runtime_provider: model.runtime_provider.clone(),
                    model_config_id: model.model_config_id.clone(),
                    model_name: model.model.clone(),
                    display_name: model.display_name.clone(),
                    provider: model.provider.clone(),
                    source: model.source.clone(),
                    input_modalities: model.input_modalities.clone(),
                },
            )
            .await;

        model_dtos.push(build_model_dto_with_access(model, access));
    }

    let custom_models = match state
        .account_service
        .list_user_custom_models(auth.user_id())
        .await
    {
        Ok(items) => items,
        Err(error) => return chat_service_error_response_from_error(error),
    };

    for custom in custom_models {
        if custom.model_type == "image" {
            continue;
        }

        let catalog_model = CatalogModel {
            model_config_id: custom.model_config_id,
            provider: "openai".to_string(),
            runtime_provider: "openai_compatible".to_string(),
            display_provider: String::new(),
            model: custom.model_name.clone(),
            display_name: custom.model_name,
            source: "custom".to_string(),
            model_type: custom.model_type.clone(),
            input_modalities: match custom.model_type.as_str() {
                "multimodal" => vec!["text".to_string(), "image".to_string()],
                _ => vec!["text".to_string()],
            },
            official: false,
            custom: true,
        };

        let access = state
            .account_service
            .resolve_text_access(
                auth.user_id(),
                &openchat_core::TurnModelRef {
                    runtime_provider: catalog_model.runtime_provider.clone(),
                    model_config_id: catalog_model.model_config_id.clone(),
                    model_name: catalog_model.model.clone(),
                    display_name: catalog_model.display_name.clone(),
                    provider: catalog_model.provider.clone(),
                    source: catalog_model.source.clone(),
                    input_modalities: catalog_model.input_modalities.clone(),
                },
            )
            .await;

        model_dtos.push(build_model_dto_with_access(catalog_model, access));
    }

    Json(model_dtos).into_response()
}

pub async fn list_tools(
    State(state): State<AppState>,
    CurrentUser(auth): CurrentUser,
) -> impl IntoResponse {
    let mut tools = state.catalog_service.list_tools();

    let custom_models = match state
        .account_service
        .list_user_custom_models(auth.user_id())
        .await
    {
        Ok(items) => items,
        Err(error) => return chat_service_error_response_from_error(error),
    };

    for custom in custom_models {
        if custom.model_type != "image" {
            continue;
        }

        tools.push(CatalogTool {
            model_config_id: custom.model_config_id,
            model_name: custom.model_name.clone(),
            id: custom_image_tool_id(custom.model_name.as_str()),
            provider: "openai".to_string(),
            runtime_provider: "openai_compatible".to_string(),
            display_provider: String::new(),
            source: "custom".to_string(),
            tool_type: "image".to_string(),
            display_name: custom.model_name,
        });
    }

    let mut tool_dtos = Vec::with_capacity(tools.len());
    for tool in tools {
        let access = state
            .tool_access_service
            .inspect_catalog_tool(auth.user_id(), &tool)
            .await;
        tool_dtos.push(build_tool_dto_with_access(tool, access));
    }

    Json(tool_dtos).into_response()
}
