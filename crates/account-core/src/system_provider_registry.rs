use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct SystemProviderDefinition {
    pub key: String,
    pub runtime_provider: String,
    pub base_url: String,
}

#[derive(Clone, Debug, Default)]
pub struct SystemProviderRegistry {
    providers: HashMap<String, SystemProviderDefinition>,
}

impl SystemProviderRegistry {
    pub fn new(providers: impl IntoIterator<Item = SystemProviderDefinition>) -> Self {
        let providers = providers
            .into_iter()
            .map(|provider| (provider.key.clone(), provider))
            .collect();
        Self { providers }
    }

    pub fn get(&self, provider_key: &str) -> Option<&SystemProviderDefinition> {
        self.providers.get(provider_key)
    }
}
