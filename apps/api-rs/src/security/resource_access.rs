use std::sync::Arc;

use openchat_infra::stores::ChatStore;
use openchat_security_core::{
    Action, AuthContext, AuthorizationError, Authorizer, ResourceDescriptor, ResourceKind,
    ResourceOwner, ResourceVisibility,
};

#[derive(Clone)]
pub struct ResourceAccessService {
    authorizer: Arc<dyn Authorizer>,
    chat_store: Arc<ChatStore>,
}

impl ResourceAccessService {
    pub fn new(authorizer: Arc<dyn Authorizer>, chat_store: Arc<ChatStore>) -> Self {
        Self {
            authorizer,
            chat_store,
        }
    }

    pub async fn authorize_session(
        &self,
        auth: &AuthContext,
        action: Action,
        session_id: &str,
    ) -> Result<(), AuthorizationError> {
        let session = self
            .chat_store
            .get_session_unscoped(session_id)
            .await
            .map_err(|error| AuthorizationError {
                message: error.to_string(),
            })?;

        let Some(session) = session else {
            return Err(AuthorizationError {
                message: "Session not found".to_string(),
            });
        };

        let resource = ResourceDescriptor::new(
            ResourceKind::Session,
            ResourceOwner::User(session.user_id),
            ResourceVisibility::Private,
        );

        self.authorizer.authorize(auth, action, &resource)
    }
}
