use super::access::ToolCapability;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToolInputMode {
    TextOnly,
    OptionalImages,
    RequiredImages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolHandlerKind {
    ImageGeneration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolDefinition {
    pub tool_type: &'static str,
    pub capability: ToolCapability,
    pub input_mode: ToolInputMode,
    pub handler_kind: ToolHandlerKind,
}
