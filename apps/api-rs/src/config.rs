use std::env;

#[derive(Clone, Debug)]
pub enum MediaStorageConfig {
    Local {
        root_dir: String,
        public_base_url: String,
    },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key_id: String,
        secret_access_key: String,
        public_base_url: String,
        force_path_style: bool,
    },
}

#[derive(Clone, Debug)]
pub struct DefaultModelRuntimeConfig {
    pub base_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub bind_addr: String,
    pub catalog_path: String,
    pub cors_allowed_origins: Vec<String>,
    pub database_url: String,
    pub public_base_url: Option<String>,
    pub auth_secret_key: String,
    pub provider_secret_key: String,
    pub auth_cookie_secure: bool,
    pub auth_cookie_domain: Option<String>,
    pub default_model_runtime: DefaultModelRuntimeConfig,
    pub media_storage: MediaStorageConfig,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let media_storage_backend = env::var("OPENCHAT_MEDIA_STORAGE_BACKEND")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase();

        Self {
            bind_addr: env::var("OPENCHAT_SERVER_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8787".to_string()),
            catalog_path: env::var("OPENCHAT_CATALOG_PATH")
                .unwrap_or_else(|_| "config/model-catalog.json".to_string()),
            cors_allowed_origins: env::var("OPENCHAT_CORS_ALLOWED_ORIGINS")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|origin| !origin.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            database_url: env::var("OPENCHAT_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://openchat:openchat123456@localhost:5432/openchat".to_string()),
            public_base_url: env::var("OPENCHAT_PUBLIC_BASE_URL")
                .ok()
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty()),
            auth_secret_key: env::var("OPENCHAT_AUTH_SECRET_KEY")
                .unwrap_or_else(|_| "openchat-dev-auth-secret".to_string()),
            provider_secret_key: env::var("OPENCHAT_PROVIDER_SECRET_KEY")
                .unwrap_or_else(|_| "openchat-dev-provider-secret".to_string()),
            auth_cookie_secure: env::var("OPENCHAT_AUTH_COOKIE_SECURE")
                .map(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
            auth_cookie_domain: env::var("OPENCHAT_AUTH_COOKIE_DOMAIN")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            default_model_runtime: DefaultModelRuntimeConfig {
                base_url: read_first_env(&[
                    "OPENCHAT_OPENAI_BASE_URL",
                    "OPENCHAT_DEFAULT_MODEL_BASE_URL",
                    "XIAOMI_OPENAI_BASE_URL",
                    "OPENAI_BASE_URL",
                ]),
            },
            media_storage: match media_storage_backend.as_str() {
                "s3" => MediaStorageConfig::S3 {
                    bucket: env::var("OPENCHAT_S3_BUCKET")
                        .unwrap_or_else(|_| "openchat-media".to_string()),
                    region: env::var("OPENCHAT_S3_REGION")
                        .unwrap_or_else(|_| "ap-southeast-1".to_string()),
                    endpoint: env::var("OPENCHAT_S3_ENDPOINT").ok(),
                    access_key_id: env::var("OPENCHAT_S3_ACCESS_KEY_ID")
                        .unwrap_or_else(|_| "minioadmin".to_string()),
                    secret_access_key: env::var("OPENCHAT_S3_SECRET_ACCESS_KEY")
                        .unwrap_or_else(|_| "minioadmin".to_string()),
                    public_base_url: env::var("OPENCHAT_S3_PUBLIC_BASE_URL")
                        .unwrap_or_else(|_| "http://localhost:9000/openchat-media".to_string()),
                    force_path_style: env::var("OPENCHAT_S3_FORCE_PATH_STYLE")
                        .map(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes"))
                        .unwrap_or(true),
                },
                _ => MediaStorageConfig::Local {
                    root_dir: env::var("OPENCHAT_MEDIA_ROOT_DIR")
                        .unwrap_or_else(|_| "data/media".to_string()),
                    public_base_url: env::var("OPENCHAT_MEDIA_PUBLIC_BASE_URL")
                        .unwrap_or_else(|_| "/media".to_string()),
                },
            },
        }
    }
}

fn read_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}
