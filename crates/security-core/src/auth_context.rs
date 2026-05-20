#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthContext {
    subject: AuthSubject,
    method: AuthMethod,
}

impl AuthContext {
    pub fn new(subject: AuthSubject, method: AuthMethod) -> Self {
        Self { subject, method }
    }

    pub fn subject(&self) -> &AuthSubject {
        &self.subject
    }

    pub fn method(&self) -> &AuthMethod {
        &self.method
    }

    pub fn user_id(&self) -> &str {
        self.subject.user_id.as_str()
    }

    pub fn is_admin(&self) -> bool {
        self.subject.is_admin
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthSubject {
    pub user_id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub is_admin: bool,
}

impl AuthSubject {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            username: None,
            email: None,
            is_admin: false,
        }
    }

    pub fn with_profile(mut self, username: Option<String>, email: Option<String>) -> Self {
        self.username = username;
        self.email = email;
        self
    }

    pub fn with_admin(mut self, is_admin: bool) -> Self {
        self.is_admin = is_admin;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthMethod {
    AccessToken,
    SessionCookie,
    ApiKey,
    Internal,
}
