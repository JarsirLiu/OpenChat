use serde_json::json;

use crate::{
    CatalogTool, ChatServiceError, ToolFunctionSpec, ToolSpec, TurnToolRef,
};

use super::access::{ToolAccessRequirement, ToolCapability};
use super::definition::{ToolDefinition, ToolHandlerKind, ToolInputMode};

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
                    parameters: image_generation_parameters(definition.input_mode),
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

    fn definition_for_turn_tool(&self, tool: &TurnToolRef) -> Result<ToolDefinition, ChatServiceError> {
        self.definition_for_tool_type(tool.tool_type.as_str())
    }

    fn definition_for_tool_type(&self, tool_type: &str) -> Result<ToolDefinition, ChatServiceError> {
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

fn image_generation_parameters(input_mode: ToolInputMode) -> serde_json::Value {
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
                "description": "Optional. Output size such as 1024x1024, 1536x1024, or 1024x1536."
            }),
        ),
        (
            "aspect_ratio".to_string(),
            json!({
                "type": "string",
                "description": "Optional. Preferred aspect ratio such as 1:1, 16:9, 4:3, 3:4, or 9:16."
            }),
        ),
        (
            "quality".to_string(),
            json!({
                "type": "string",
                "description": "Optional. Image quality hint such as low, medium, high, or hd."
            }),
        ),
        (
            "background".to_string(),
            json!({
                "type": "string",
                "description": "Optional. Background hint such as transparent, white, or solid."
            }),
        ),
        (
            "count".to_string(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 4,
                "description": "Optional. Number of images to generate. Defaults to 1."
            }),
        ),
    ]);

    if !matches!(input_mode, ToolInputMode::TextOnly) {
        let description = if matches!(input_mode, ToolInputMode::RequiredImages) {
            "Required. Reference image ids or URLs to use as edit inputs."
        } else {
            "Optional. Reference image ids or URLs to use as edit inputs."
        };
        properties.insert(
            "input_images".to_string(),
            json!({
                "type": "array",
                "items": { "type": "string" },
                "description": description
            }),
        );
        properties.insert(
            "image".to_string(),
            json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "Alias for input_images."
            }),
        );
    }

    let required = if matches!(input_mode, ToolInputMode::RequiredImages) {
        json!(["prompt", "input_images"])
    } else {
        json!(["prompt"])
    };

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}
