#[derive(Clone)]
pub struct CatalogModel {
    pub model_config_id: String,
    pub provider: String,
    pub runtime_provider: String,
    pub display_provider: String,
    pub model: String,
    pub display_name: String,
    pub source: String,
    pub model_type: String,
    pub input_modalities: Vec<String>,
    pub official: bool,
    pub custom: bool,
}
