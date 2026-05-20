use crate::{
    ChatServiceError, ImageModelAccessResolver, ImageProviderRuntime, ToolAccessResolver,
    ToolAccessService,
};

use std::sync::Arc;

use super::{
    context::{ToolExecutionResult, ToolInvocation},
    definition::ToolHandlerKind,
    image_generation::ImageGenerationToolHandler,
    registry::ToolRegistry,
};

pub struct ToolExecutor<R> {
    registry: ToolRegistry,
    access_service: ToolAccessService<R>,
    image_generation: ImageGenerationToolHandler<R>,
}

impl<R> Clone for ToolExecutor<R> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            access_service: self.access_service.clone(),
            image_generation: self.image_generation.clone(),
        }
    }
}

impl<R> ToolExecutor<R>
where
    R: ImageModelAccessResolver + ToolAccessResolver,
{
    pub fn new(
        image_runtime: ImageProviderRuntime<R>,
        access_service: ToolAccessService<R>,
        media_store: Arc<dyn crate::MediaStore>,
    ) -> Self {
        Self {
            registry: ToolRegistry::default(),
            access_service,
            image_generation: ImageGenerationToolHandler::new(image_runtime, media_store),
        }
    }

    pub async fn execute(
        &self,
        invocation: ToolInvocation,
    ) -> Result<ToolExecutionResult, ChatServiceError> {
        self.access_service
            .authorize_turn_tool(invocation.user_id.as_str(), &invocation.tool)
            .await?;

        match self.registry.handler_kind_for_turn_tool(&invocation.tool)? {
            ToolHandlerKind::ImageGeneration => self.image_generation.execute(invocation).await,
        }
    }
}
