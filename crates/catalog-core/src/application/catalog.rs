use openchat_core::CatalogTool;

use crate::domain::models::CatalogModel;

#[derive(Clone)]
pub struct CatalogService {
    models: Vec<CatalogModel>,
    tools: Vec<CatalogTool>,
}

impl CatalogService {
    pub fn new(models: Vec<CatalogModel>, tools: Vec<CatalogTool>) -> Self {
        Self { models, tools }
    }

    pub fn list_models(&self) -> Vec<CatalogModel> {
        self.models.clone()
    }

    pub fn list_tools(&self) -> Vec<CatalogTool> {
        self.tools.clone()
    }

    pub fn resolve_text_model(&self, model_config_id: &str) -> Option<CatalogModel> {
        self.models
            .iter()
            .find(|model| {
                matches!(model.model_type.as_str(), "text" | "multimodal")
                    && model.model_config_id == model_config_id
            })
            .cloned()
    }

    pub fn resolve_image_tool(&self, model_config_id: &str, tool_id: &str) -> Option<CatalogTool> {
        self.tools
            .iter()
            .find(|tool| {
                tool.tool_type == "image"
                    && tool.model_config_id == model_config_id
                    && tool.id == tool_id
            })
            .cloned()
    }
}
