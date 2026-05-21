use serde_json::json;

use crate::{
    CatalogTool, ChatServiceError, ImageToolDefaults, ToolFunctionSpec, ToolSpec, TurnToolRef,
};

use super::access::{ToolAccessRequirement, ToolCapability};
use super::definition::{ToolDefinition, ToolHandlerKind, ToolInputMode};
use super::image_generation::supported_image_size_description;

#[derive(Clone, Default)]
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn validate(&self, tool: &TurnToolRef) -> Result<(), ChatServiceError> {
        self.definition_for_turn_tool(tool).map(|_| ())
    }

    pub fn requirement_for_catalog_tool(
        &self,
        tool: &CatalogTool,
    ) -> Result<ToolAccessRequirement, ChatServiceError> {
        let definition = self.definition_for_catalog_tool(tool)?;
        Ok(ToolAccessRequirement {
            model_config_id: tool.model_config_id.clone(),
            source: tool.source.clone(),
            tool_type: tool.tool_type.clone(),
            capability: definition.capability,
            provider_key: tool.provider.clone(),
            runtime_provider: tool.runtime_provider.clone(),
        })
    }

    pub fn requirement_for_turn_tool(
        &self,
        tool: &TurnToolRef,
    ) -> Result<ToolAccessRequirement, ChatServiceError> {
        let definition = self.definition_for_turn_tool(tool)?;
        Ok(ToolAccessRequirement {
            model_config_id: tool.model_config_id.clone(),
            source: tool.source.clone(),
            tool_type: tool.tool_type.clone(),
            capability: definition.capability,
            provider_key: tool.provider.clone(),
            runtime_provider: tool.runtime_provider.clone(),
        })
    }

    pub fn spec_for_turn_tool(&self, tool: &TurnToolRef) -> Result<ToolSpec, ChatServiceError> {
        let definition = self.definition_for_turn_tool(tool)?;
        match definition.handler_kind {
            ToolHandlerKind::ImageGeneration => Ok(ToolSpec {
                kind: "function",
                function: ToolFunctionSpec {
                    name: tool.id.clone(),
                    description: format!(
                        "Generate or edit images with {}. Use this when the user explicitly asks to create, draw, generate, or modify an image.",
                        tool.display_name
                    ),
                    parameters: image_generation_parameters(definition.input_mode, tool.image_defaults.as_ref()),
                },
            }),
        }
    }

    pub fn handler_kind_for_turn_tool(
        &self,
        tool: &TurnToolRef,
    ) -> Result<ToolHandlerKind, ChatServiceError> {
        Ok(self.definition_for_turn_tool(tool)?.handler_kind)
    }

    pub fn specs_for_turn_tools(
        &self,
        tools: &[TurnToolRef],
    ) -> Result<Vec<ToolSpec>, ChatServiceError> {
        tools
            .iter()
            .map(|tool| self.spec_for_turn_tool(tool))
            .collect()
    }

    fn definition_for_catalog_tool(
        &self,
        tool: &CatalogTool,
    ) -> Result<ToolDefinition, ChatServiceError> {
        self.definition_for_tool_type(tool.tool_type.as_str())
    }

    fn definition_for_turn_tool(
        &self,
        tool: &TurnToolRef,
    ) -> Result<ToolDefinition, ChatServiceError> {
        self.definition_for_tool_type(tool.tool_type.as_str())
    }

    fn definition_for_tool_type(
        &self,
        tool_type: &str,
    ) -> Result<ToolDefinition, ChatServiceError> {
        match tool_type {
            "image" => Ok(ToolDefinition {
                tool_type: "image",
                capability: ToolCapability::ImageGeneration,
                input_mode: ToolInputMode::OptionalImages,
                handler_kind: ToolHandlerKind::ImageGeneration,
            }),
            other => Err(ChatServiceError::new(
                501,
                format!("Tool type `{other}` is not implemented yet"),
            )),
        }
    }
}

fn image_generation_parameters(
    input_mode: ToolInputMode,
    defaults: Option<&ImageToolDefaults>,
) -> serde_json::Value {
    let default_size = defaults
        .map(|item| item.size.as_str())
        .unwrap_or("1024x1024");
    let default_quality = defaults.map(|item| item.quality.as_str()).unwrap_or("auto");
    let default_n = defaults.map(|item| item.n).unwrap_or(1);
    let mut properties = serde_json::Map::from_iter([
        (
            "prompt".to_string(),
            json!({
                "type": "string",
                "description": "Required. The image generation or editing prompt."
            }),
        ),
        (
            "size".to_string(),
            json!({
                "type": "string",
                "default": default_size,
                "description": supported_image_size_description(default_size)
            }),
        ),
        (
            "quality".to_string(),
            json!({
                "type": "string",
                "enum": ["auto", "low", "medium", "high"],
                "default": default_quality,
                "description": "Optional. Image quality hint such as auto, low, medium, or high. This controls visual fidelity, not output dimensions."
            }),
        ),
        (
            "n".to_string(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 8,
                "default": default_n,
                "description": "Optional. Number of images to generate. Defaults to the model profile and supports up to 8."
            }),
        ),
    ]);

    if !matches!(input_mode, ToolInputMode::TextOnly) {
        let description = if matches!(input_mode, ToolInputMode::RequiredImages) {
            "Required. Reference image ids or URLs to use as edit inputs."
        } else {
            "Optional. Reference image ids or URLs to use as image inputs."
        };
        properties.insert(
            "input_images".to_string(),
            json!({
                "type": "array",
                "items": { "type": "string" },
                "description": format!("{description} These may be current attachment ids, previously generated image ids, external URLs, or data URLs.")
            }),
        );
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": ["prompt"],
        "additionalProperties": false
    })
}
