use openchat_account_core::{SystemProviderDefinition, SystemProviderRegistry};

use crate::config::AppConfig;

pub fn build_system_provider_registry(config: &AppConfig) -> SystemProviderRegistry {
    let mut providers = Vec::new();

    if let Some(base_url) = config.default_model_runtime.base_url.clone() {
        providers.push(SystemProviderDefinition {
            key: "openai".to_string(),
            runtime_provider: "openai_compatible".to_string(),
            base_url,
        });
    }

    SystemProviderRegistry::new(providers)
}
