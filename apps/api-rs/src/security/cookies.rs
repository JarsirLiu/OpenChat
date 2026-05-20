use axum_extra::extract::cookie::{Cookie, SameSite};
use openchat_account_core::{AuthSession, ACCESS_TOKEN_TTL_MILLIS, REFRESH_TOKEN_TTL_MILLIS};
use time::Duration;

const ACCESS_COOKIE_NAME: &str = "openchat_access";
const REFRESH_COOKIE_NAME: &str = "openchat_refresh";
const CSRF_COOKIE_NAME: &str = "openchat_csrf";

#[derive(Clone)]
pub struct AuthCookieManager {
    secure: bool,
    domain: Option<String>,
}

impl AuthCookieManager {
    pub fn new(secure: bool, domain: Option<String>) -> Self {
        Self { secure, domain }
    }

    pub fn access_cookie_name(&self) -> &'static str {
        ACCESS_COOKIE_NAME
    }

    pub fn refresh_cookie_name(&self) -> &'static str {
        REFRESH_COOKIE_NAME
    }

    pub fn csrf_cookie_name(&self) -> &'static str {
        CSRF_COOKIE_NAME
    }

    pub fn build_access_cookie(&self, token: &str) -> Cookie<'static> {
        self.build_cookie(ACCESS_COOKIE_NAME, token.to_string(), true, "/", ACCESS_TOKEN_TTL_MILLIS)
    }

    pub fn build_refresh_cookie(&self, token: &str) -> Cookie<'static> {
        self.build_cookie(
            REFRESH_COOKIE_NAME,
            token.to_string(),
            true,
            "/api/auth",
            REFRESH_TOKEN_TTL_MILLIS,
        )
    }

    pub fn build_csrf_cookie(&self, token: &str) -> Cookie<'static> {
        self.build_cookie(CSRF_COOKIE_NAME, token.to_string(), false, "/", REFRESH_TOKEN_TTL_MILLIS)
    }

    pub fn clear_access_cookie(&self) -> Cookie<'static> {
        self.expired_cookie(ACCESS_COOKIE_NAME, true, "/")
    }

    pub fn clear_refresh_cookie(&self) -> Cookie<'static> {
        self.expired_cookie(REFRESH_COOKIE_NAME, true, "/api/auth")
    }

    pub fn clear_csrf_cookie(&self) -> Cookie<'static> {
        self.expired_cookie(CSRF_COOKIE_NAME, false, "/")
    }

    pub fn session_cookies(&self, session: &AuthSession) -> [Cookie<'static>; 2] {
        [
            self.build_access_cookie(session.token.as_str()),
            self.build_refresh_cookie(session.refresh_token.as_str()),
        ]
    }

    fn build_cookie(
        &self,
        name: &'static str,
        value: String,
        http_only: bool,
        path: &'static str,
        ttl_millis: i64,
    ) -> Cookie<'static> {
        let mut builder = Cookie::build((name, value))
            .path(path)
            .http_only(http_only)
            .same_site(SameSite::Lax)
            .secure(self.secure)
            .max_age(Duration::milliseconds(ttl_millis));

        if let Some(domain) = self.domain.as_ref() {
            builder = builder.domain(domain.clone());
        }

        builder.build()
    }

    fn expired_cookie(
        &self,
        name: &'static str,
        http_only: bool,
        path: &'static str,
    ) -> Cookie<'static> {
        let mut builder = Cookie::build((name, ""))
            .path(path)
            .http_only(http_only)
            .same_site(SameSite::Lax)
            .secure(self.secure)
            .max_age(Duration::seconds(0));

        if let Some(domain) = self.domain.as_ref() {
            builder = builder.domain(domain.clone());
        }

        builder.build()
    }
}
