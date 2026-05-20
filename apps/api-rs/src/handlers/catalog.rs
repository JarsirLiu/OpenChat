use axum::{extract::State, response::IntoResponse, Json};
use openchat_catalog_core::CatalogModel;

use crate::{
    http::catalog::{CatalogModelDto, CatalogToolDto},
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
        Err(error) => {
            return (
                axum::http::StatusCode::from_u16(error.status_code)
                    .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                Json(serde_json::json!({ "message": error.message })),
            )
                .into_response();
        }
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

pub async fn list_tools(_current_user: CurrentUser) -> impl IntoResponse {
    Json(Vec::<CatalogToolDto>::new()).into_response()
}
