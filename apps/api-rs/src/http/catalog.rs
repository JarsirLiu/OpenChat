use openchat_catalog_core::CatalogModel;
use serde::Serialize;

#[derive(Serialize)]
pub struct CatalogModelDto {
    pub model_config_id: String,
    pub provider: String,
    pub display_provider: String,
    pub model: String,
    pub display_name: String,
    pub source: String,
    pub r#type: String,
    pub input_modalities: Vec<String>,
    pub official: bool,
    pub custom: bool,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Serialize)]
pub struct CatalogToolDto {
    pub model_config_id: String,
    pub id: String,
    pub provider: String,
    pub display_provider: String,
    pub model: String,
    pub source: String,
    pub r#type: String,
    pub display_name: String,
    pub available: bool,
    pub enabled: bool,
    pub unavailable_reason: Option<String>,
}

impl From<CatalogModel> for CatalogModelDto {
    fn from(value: CatalogModel) -> Self {
        Self {
            model_config_id: value.model_config_id,
            provider: value.provider,
            display_provider: value.display_provider,
            model: value.model,
            display_name: value.display_name,
            source: value.source,
            r#type: value.model_type,
            input_modalities: value.input_modalities,
            official: value.official,
            custom: value.custom,
            available: true,
            unavailable_reason: None,
        }
    }
}
