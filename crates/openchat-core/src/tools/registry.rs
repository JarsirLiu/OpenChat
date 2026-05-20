use serde_json::json;

use crate::{CatalogTool, ChatServiceError, ToolFunctionSpec, ToolSpec, TurnToolRef};

use super::access::{ToolAccessRequirement, ToolCapability};

#[derive(Clone, Default)]
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn validate(&self, tool: &TurnToolRef) -> Result<(), ChatServiceError> {
        match tool.tool_type.as_str() {
            "image" => Ok(()),
            other => Err(ChatServiceError::new(
                501,
                format!("Tool type `{other}` is not implemented yet"),
            )),
        }
    }

    pub fn requirement_for_catalog_tool(
        &self,
        tool: &CatalogTool,
    ) -> Result<ToolAccessRequirement, ChatServiceError> {
        let capability = self.capability_for_tool_type(tool.tool_type.as_str())?;
        Ok(ToolAccessRequirement {
            model_config_id: tool.model_config_id.clone(),
            source: tool.source.clone(),
            tool_type: tool.tool_type.clone(),
            capability,
            provider_key: tool.provider.clone(),
            runtime_provider: tool.runtime_provider.clone(),
        })
    }

    pub fn requirement_for_turn_tool(
        &self,
        tool: &TurnToolRef,
    ) -> Result<ToolAccessRequirement, ChatServiceError> {
        let capability = self.capability_for_tool_type(tool.tool_type.as_str())?;
        Ok(ToolAccessRequirement {
            model_config_id: tool.model_config_id.clone(),
            source: tool.source.clone(),
            tool_type: tool.tool_type.clone(),
            capability,
            provider_key: tool.provider.clone(),
            runtime_provider: tool.runtime_provider.clone(),
        })
    }

    pub fn spec_for_turn_tool(&self, tool: &TurnToolRef) -> Result<ToolSpec, ChatServiceError> {
        match tool.tool_type.as_str() {
            "image" => Ok(ToolSpec {
                kind: "function",
                function: ToolFunctionSpec {
                    name: tool.id.clone(),
                    description: format!(
                        "Generate or edit images with {}. Use this when the user explicitly asks to create, draw, generate, or modify an image.",
                        tool.display_name
                    ),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": "Required. The image generation or editing prompt."
                            },
                            "size": {
                                "type": "string",
                                "description": "Optional. Output size such as 1024x1024, 1536x1024, or 1024x1536."
                            },
                            "aspect_ratio": {
                                "type": "string",
                                "description": "Optional. Preferred aspect ratio such as 1:1, 16:9, 4:3, 3:4, or 9:16."
                            },
                            "quality": {
                                "type": "string",
                                "description": "Optional. Image quality hint such as low, medium, high, or hd."
                            },
                            "background": {
                                "type": "string",
                                "description": "Optional. Background hint such as transparent, white, or solid."
                            },
                            "count": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 4,
                                "description": "Optional. Number of images to generate. Defaults to 1."
                            }
                        },
                        "required": ["prompt"],
                        "additionalProperties": false
                    }),
                },
            }),
            other => Err(ChatServiceError::new(
                501,
                format!("Tool type `{other}` is not implemented yet"),
            )),
        }
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

    fn capability_for_tool_type(
        &self,
        tool_type: &str,
    ) -> Result<ToolCapability, ChatServiceError> {
        match tool_type {
            "image" => Ok(ToolCapability::ImageGeneration),
            other => Err(ChatServiceError::new(
                501,
                format!("Tool type `{other}` is not implemented yet"),
            )),
        }
    }
}
