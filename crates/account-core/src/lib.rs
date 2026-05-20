mod auth_service;
mod contracts;
mod custom_model_service;
mod domain;
mod model_provider_service;
mod service;
mod system_provider_registry;

pub use auth_service::{AuthService, ACCESS_TOKEN_TTL_MILLIS, REFRESH_TOKEN_TTL_MILLIS};
pub use contracts::{
    AuthResponseDto, CreateUserCustomModelDto, LoginRequestDto, LogoutRequestDto,
    RefreshRequestDto, RegisterRequestDto, UpsertUserProviderApiKeyDto, UserCustomModelDto,
    UserDto, UserInfoDto, UserProviderApiKeyDto, UserProviderApiKeySecretDto,
};
pub use custom_model_service::CustomModelService;
pub use domain::{
    AuthError, AuthSession, AuthUser, CreateUserCustomModel, UpsertUserProviderApiKey,
    UserCustomModel, UserProviderApiKey, UserProviderApiKeySecret,
};
pub use model_provider_service::ModelProviderService;
pub use service::AccountService;
pub use system_provider_registry::{SystemProviderDefinition, SystemProviderRegistry};
