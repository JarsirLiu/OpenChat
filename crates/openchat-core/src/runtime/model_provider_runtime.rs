use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    runtime::tools::ImageGenerationRequest, ChatServiceError, GeneratedImage, ImageRuntime,
    ModelEventStream, ModelRuntime, TurnModelRef, TurnPlan, TurnToolRef,
};

pub type ResolveTextAccessFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ResolvedTextModelAccess, ChatServiceError>> + Send + 'a>>;

pub trait TextModelAccessResolver: Send + Sync {
    fn resolve_text_access<'a>(
        &'a self,
        user_id: &'a str,
        model: &'a TurnModelRef,
    ) -> ResolveTextAccessFuture<'a>;
}

pub type ResolveImageAccessFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ResolvedImageModelAccess, ChatServiceError>> + Send + 'a>>;

pub trait ImageModelAccessResolver: Send + Sync {
    fn resolve_image_access<'a>(
        &'a self,
        user_id: &'a str,
        tool: &'a TurnToolRef,
    ) -> ResolveImageAccessFuture<'a>;
}

#[derive(Clone)]
pub struct ResolvedTextModelAccess {
    pub provider_key: String,
    pub runtime_provider: String,
    pub model_name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key: String,
    pub input_modalities: Vec<String>,
}

#[derive(Clone)]
pub struct ResolvedImageModelAccess {
    pub provider_key: String,
    pub runtime_provider: String,
    pub model_name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key: String,
}

pub struct ModelProviderRuntime<R> {
    access_resolver: Arc<R>,
    openai_compatible_runtime: ModelRuntime,
}

pub struct ImageProviderRuntime<R> {
    access_resolver: Arc<R>,
    openai_compatible_runtime: ImageRuntime,
}

impl<R> Clone for ModelProviderRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            access_resolver: self.access_resolver.clone(),
            openai_compatible_runtime: self.openai_compatible_runtime.clone(),
        }
    }
}

impl<R> Clone for ImageProviderRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            access_resolver: self.access_resolver.clone(),
            openai_compatible_runtime: self.openai_compatible_runtime.clone(),
        }
    }
}

impl<R> ModelProviderRuntime<R>
where
    R: TextModelAccessResolver,
{
    pub fn new(access_resolver: Arc<R>, openai_compatible_runtime: ModelRuntime) -> Self {
        Self {
            access_resolver,
            openai_compatible_runtime,
        }
    }

    pub async fn stream_text(&self, plan: &TurnPlan) -> Result<ModelEventStream, ChatServiceError> {
        let access = self
            .access_resolver
            .resolve_text_access(plan.user_id.as_str(), &plan.text_model)
            .await?;

        match access.runtime_provider.as_str() {
            "openai_compatible" => {
                self.openai_compatible_runtime
                    .stream_text(plan, &access)
                    .await
            }
            provider => Err(ChatServiceError::new(
                501,
                format!("Runtime provider `{provider}` is not implemented yet"),
            )),
        }
    }
}

impl<R> ImageProviderRuntime<R>
where
    R: ImageModelAccessResolver,
{
    pub fn new(access_resolver: Arc<R>, openai_compatible_runtime: ImageRuntime) -> Self {
        Self {
            access_resolver,
            openai_compatible_runtime,
        }
    }

    pub(crate) async fn generate_image(
        &self,
        user_id: &str,
        tool: &TurnToolRef,
        request: &ImageGenerationRequest,
    ) -> Result<Vec<GeneratedImage>, ChatServiceError> {
        let access = self
            .access_resolver
            .resolve_image_access(user_id, tool)
            .await?;

        match access.runtime_provider.as_str() {
            "openai_compatible" => {
                self.openai_compatible_runtime
                    .generate_image(request, &access)
                    .await
            }
            provider => Err(ChatServiceError::new(
                501,
                format!("Image runtime provider `{provider}` is not implemented yet"),
            )),
        }
    }
}
