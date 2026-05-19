#[derive(Clone)]
pub struct CatalogTool {
    pub model_config_id: String,
    pub model_name: String,
    pub id: String,
    pub provider: String,
    pub runtime_provider: String,
    pub display_provider: String,
    pub source: String,
    pub tool_type: String,
    pub display_name: String,
}

impl CatalogTool {
    pub fn demo_catalog() -> Vec<Self> {
        Vec::new()
    }
}
