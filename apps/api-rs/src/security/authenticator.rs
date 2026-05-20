use std::sync::Arc;

use openchat_account_core::AccountService;
use openchat_security_core::{
    AccessTokenAuthenticator, AuthContext, AuthMethod, AuthSubject, AuthenticationError,
    AuthenticationErrorKind, AuthenticationFuture,
};

#[derive(Clone)]
pub struct AccountAuthenticator {
    account_service: Arc<AccountService>,
}

impl AccountAuthenticator {
    pub fn new(account_service: Arc<AccountService>) -> Self {
        Self { account_service }
    }
}

impl AccessTokenAuthenticator for AccountAuthenticator {
    fn authenticate_access_token<'a>(&'a self, token: &'a str) -> AuthenticationFuture<'a> {
        Box::pin(async move {
            let user = self
                .account_service
                .current_user(token)
                .await
                .map_err(|error| {
                    let kind = if error.status_code == 401 {
                        AuthenticationErrorKind::InvalidCredentials
                    } else {
                        AuthenticationErrorKind::Internal
                    };
                    AuthenticationError::new(kind, error.message)
                })?;

            Ok(AuthContext::new(
                AuthSubject::new(user.id)
                    .with_profile(Some(user.username), Some(user.email))
                    .with_admin(user.is_admin),
                AuthMethod::AccessToken,
            ))
        })
    }
}
