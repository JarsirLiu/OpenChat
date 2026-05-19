mod auth_service;
mod contracts;
mod custom_model_service;
mod domain;
mod model_provider_service;
mod service;

pub use auth_service::AuthService;
pub use contracts::{
    AuthResponseDto, CreateUserCustomModelDto, LoginRequestDto, LogoutRequestDto,
    RefreshRequestDto, RegisterRequestDto, UpsertUserProviderSettingDto, UserCustomModelDto,
    UserDto, UserInfoDto, UserProviderSettingDto,
};
pub use custom_model_service::CustomModelService;
pub use domain::{
    AuthError, AuthSession, AuthUser, CreateUserCustomModel, UpsertUserProviderSetting,
    UserCustomModel, UserProviderSetting,
};
pub use model_provider_service::{ModelProviderService, ProviderRuntimeFallback};
pub use service::AccountService;
