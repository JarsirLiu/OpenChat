use crate::{Action, AuthContext, AuthorizationError, ResourceDescriptor, ResourceOwner, ResourceVisibility};

pub trait Authorizer: Send + Sync {
    fn authorize(
        &self,
        auth: &AuthContext,
        action: Action,
        resource: &ResourceDescriptor,
    ) -> Result<(), AuthorizationError>;
}

#[derive(Clone, Debug, Default)]
pub struct OwnershipAuthorizer;

impl OwnershipAuthorizer {
    pub fn new() -> Self {
        Self
    }

    fn is_owner(&self, auth: &AuthContext, resource: &ResourceDescriptor) -> bool {
        match &resource.owner {
            ResourceOwner::User(user_id) => user_id == auth.user_id(),
            ResourceOwner::System => false,
        }
    }
}

impl Authorizer for OwnershipAuthorizer {
    fn authorize(
        &self,
        auth: &AuthContext,
        action: Action,
        resource: &ResourceDescriptor,
    ) -> Result<(), AuthorizationError> {
        if auth.is_admin() {
            return Ok(());
        }

        if matches!(resource.visibility, ResourceVisibility::Public) && action.is_read() {
            return Ok(());
        }

        if self.is_owner(auth, resource) {
            return Ok(());
        }

        Err(AuthorizationError::forbidden(action, resource.kind))
    }
}

#[cfg(test)]
mod tests {
    use super::{Authorizer, OwnershipAuthorizer};
    use crate::{Action, AuthContext, AuthMethod, AuthSubject, ResourceDescriptor, ResourceKind, ResourceOwner, ResourceVisibility};

    fn user_auth(user_id: &str) -> AuthContext {
        AuthContext::new(
            AuthSubject::new(user_id.to_string()).with_admin(false),
            AuthMethod::AccessToken,
        )
    }

    #[test]
    fn owner_can_read_private_resource() {
        let authorizer = OwnershipAuthorizer::new();
        let resource = ResourceDescriptor::new(
            ResourceKind::Session,
            ResourceOwner::User("user_a".to_string()),
            ResourceVisibility::Private,
        );

        assert!(authorizer
            .authorize(&user_auth("user_a"), Action::Read, &resource)
            .is_ok());
    }

    #[test]
    fn non_owner_cannot_read_private_resource() {
        let authorizer = OwnershipAuthorizer::new();
        let resource = ResourceDescriptor::new(
            ResourceKind::Session,
            ResourceOwner::User("user_a".to_string()),
            ResourceVisibility::Private,
        );

        assert!(authorizer
            .authorize(&user_auth("user_b"), Action::Read, &resource)
            .is_err());
    }

    #[test]
    fn public_resource_allows_read() {
        let authorizer = OwnershipAuthorizer::new();
        let resource = ResourceDescriptor::new(
            ResourceKind::MediaObject,
            ResourceOwner::User("user_a".to_string()),
            ResourceVisibility::Public,
        );

        assert!(authorizer
            .authorize(&user_auth("user_b"), Action::Read, &resource)
            .is_ok());
    }
}
