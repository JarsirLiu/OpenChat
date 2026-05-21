use std::{future::Future, pin::Pin, sync::Arc};

use crate::{CatalogTool, ChatServiceError, TurnToolRef};

use super::registry::ToolRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCapability {
    ImageGeneration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolAccessRequirement {
    pub model_config_id: String,
    pub source: String,
    pub tool_type: String,
    pub capability: ToolCapability,
    pub provider_key: String,
    pub runtime_provider: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolAccessOutcome {
    Allowed,
    Denied { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolAccessDecision {
    pub visible: bool,
    pub enabled: bool,
    pub executable: bool,
    pub reason: Option<String>,
}

pub type ResolveToolAccessFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ToolAccessOutcome, ChatServiceError>> + Send + 'a>>;

pub trait ToolAccessResolver: Send + Sync {
    fn resolve_tool_access<'a>(
        &'a self,
        user_id: &'a str,
        requirement: &'a ToolAccessRequirement,
    ) -> ResolveToolAccessFuture<'a>;
}

pub struct ToolAccessService<R> {
    registry: ToolRegistry,
    resolver: Arc<R>,
}

impl<R> Clone for ToolAccessService<R> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            resolver: self.resolver.clone(),
        }
    }
}

impl<R> ToolAccessService<R>
where
    R: ToolAccessResolver,
{
    pub fn new(resolver: Arc<R>) -> Self {
        Self {
            registry: ToolRegistry::default(),
            resolver,
        }
    }

    pub async fn inspect_catalog_tool(
        &self,
        user_id: &str,
        tool: &CatalogTool,
    ) -> Result<ToolAccessDecision, ChatServiceError> {
        let requirement = match self.registry.requirement_for_catalog_tool(tool) {
            Ok(requirement) => requirement,
            Err(error) => return Ok(denied_visibility(error.message)),
        };

        self.resolve_decision(user_id, &requirement).await
    }

    pub async fn authorize_turn_tool(
        &self,
        user_id: &str,
        tool: &TurnToolRef,
    ) -> Result<ToolAccessDecision, ChatServiceError> {
        let requirement = self.registry.requirement_for_turn_tool(tool)?;
        let decision = self.resolve_decision(user_id, &requirement).await?;

        if decision.executable {
            Ok(decision)
        } else {
            Err(ChatServiceError::new(
                403,
                decision
                    .reason
                    .clone()
                    .unwrap_or_else(|| "The selected tool is not available".to_string()),
            ))
        }
    }

    async fn resolve_decision(
        &self,
        user_id: &str,
        requirement: &ToolAccessRequirement,
    ) -> Result<ToolAccessDecision, ChatServiceError> {
        match self
            .resolver
            .resolve_tool_access(user_id, requirement)
            .await?
        {
            ToolAccessOutcome::Allowed => Ok(ToolAccessDecision {
                visible: true,
                enabled: true,
                executable: true,
                reason: None,
            }),
            ToolAccessOutcome::Denied { reason } => Ok(ToolAccessDecision {
                visible: true,
                enabled: false,
                executable: false,
                reason: Some(reason),
            }),
        }
    }
}

fn denied_visibility(message: String) -> ToolAccessDecision {
    ToolAccessDecision {
        visible: false,
        enabled: false,
        executable: false,
        reason: Some(message),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::ResolveToolAccessFuture;

    use super::{
        ToolAccessDecision, ToolAccessOutcome, ToolAccessRequirement, ToolAccessResolver,
        ToolAccessService,
    };

    struct AllowAllResolver;

    impl ToolAccessResolver for AllowAllResolver {
        fn resolve_tool_access<'a>(
            &'a self,
            _user_id: &'a str,
            _requirement: &'a ToolAccessRequirement,
        ) -> ResolveToolAccessFuture<'a> {
            Box::pin(async { Ok(ToolAccessOutcome::Allowed) })
        }
    }

    struct DenyAllResolver;

    impl ToolAccessResolver for DenyAllResolver {
        fn resolve_tool_access<'a>(
            &'a self,
            _user_id: &'a str,
            _requirement: &'a ToolAccessRequirement,
        ) -> ResolveToolAccessFuture<'a> {
            Box::pin(async {
                Ok(ToolAccessOutcome::Denied {
                    reason: "not enabled".to_string(),
                })
            })
        }
    }

    #[tokio::test]
    async fn inspect_catalog_tool_marks_allowed_tools_executable() {
        let service = ToolAccessService::new(Arc::new(AllowAllResolver));
        let tool = crate::CatalogTool {
            model_config_id: "openchat:image:gpt-image".to_string(),
            model_name: "gpt-image-2".to_string(),
            id: "image_gen_oai".to_string(),
            provider: "openai".to_string(),
            runtime_provider: "openai_compatible".to_string(),
            display_provider: "OpenAI".to_string(),
            source: "openchat".to_string(),
            tool_type: "image".to_string(),
            display_name: "GPT Image".to_string(),
            image_defaults: None,
        };

        let decision = service
            .inspect_catalog_tool("user_1", &tool)
            .await
            .unwrap_or_else(|error| panic!("inspect tool failed: {}", error.message));

        assert_eq!(
            decision,
            ToolAccessDecision {
                visible: true,
                enabled: true,
                executable: true,
                reason: None,
            }
        );
    }

    #[tokio::test]
    async fn authorize_turn_tool_rejects_denied_tools() {
        let service = ToolAccessService::new(Arc::new(DenyAllResolver));
        let tool = crate::TurnToolRef {
            runtime_provider: "openai_compatible".to_string(),
            model_config_id: "openchat:image:gpt-image".to_string(),
            model_name: "gpt-image-2".to_string(),
            id: "image_gen_oai".to_string(),
            display_name: "GPT Image".to_string(),
            provider: "openai".to_string(),
            source: "openchat".to_string(),
            tool_type: "image".to_string(),
            image_defaults: None,
        };

        let error = service
            .authorize_turn_tool("user_1", &tool)
            .await
            .err()
            .unwrap_or_else(|| panic!("tool should be denied"));

        assert_eq!(error.status_code, 403);
        assert_eq!(error.message, "not enabled");
    }
}
