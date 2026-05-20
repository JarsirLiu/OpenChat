use crate::{
    ChatServiceError, ImageModelAccessResolver, ImageProviderRuntime, ToolAccessResolver,
    ToolAccessService,
};

use std::sync::Arc;

use super::{
    context::{ToolExecutionResult, ToolInvocation},
    image_generation::ImageGenerationToolHandler,
};

pub struct ToolExecutor<R> {
    access_service: ToolAccessService<R>,
    image_generation: ImageGenerationToolHandler<R>,
}

impl<R> Clone for ToolExecutor<R> {
    fn clone(&self) -> Self {
        Self {
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

        match invocation.tool.tool_type.as_str() {
            "image" => self.image_generation.execute(invocation).await,
            other => Err(ChatServiceError::new(
                501,
                format!("Tool type `{other}` is not implemented yet"),
            )),
        }
    }
}
