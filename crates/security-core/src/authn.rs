use std::{future::Future, pin::Pin};

use crate::{AuthContext, AuthenticationError};

pub type AuthenticationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AuthContext, AuthenticationError>> + Send + 'a>>;

pub trait AccessTokenAuthenticator: Send + Sync {
    fn authenticate_access_token<'a>(&'a self, token: &'a str) -> AuthenticationFuture<'a>;
}
