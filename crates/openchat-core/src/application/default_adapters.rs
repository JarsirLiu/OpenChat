use openchat_infra::stores::{ChatStore, PersistedSession, PersistedTurnPage};

use crate::{ActiveTurnHandle, ActiveTurnRegistry, InMemorySessionStore, SessionRuntime};

use super::ports::{ActiveTurnRegistryPort, ChatRepository, SessionRuntimeRegistry};

impl ChatRepository for ChatStore {
    fn ensure_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move { ChatStore::ensure_session(self, user_id, session_id).await })
    }

    fn get_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = anyhow::Result<Option<PersistedSession>>> + Send + 'a,
        >,
    > {
        Box::pin(async move { ChatStore::get_session(self, user_id, session_id).await })
    }

    fn list_sessions<'a>(
        &'a self,
        user_id: &'a str,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<Vec<PersistedSession>>> + Send + 'a>,
    > {
        Box::pin(async move { ChatStore::list_sessions(self, user_id).await })
    }

    fn delete_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<bool>> + Send + 'a>>
    {
        Box::pin(async move { ChatStore::delete_session(self, user_id, session_id).await })
    }

    fn update_session_title<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        title: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = anyhow::Result<Option<PersistedSession>>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            ChatStore::update_session_title(self, user_id, session_id, title).await
        })
    }

    fn interrupt_running_turn<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<bool>> + Send + 'a>>
    {
        Box::pin(async move {
            ChatStore::interrupt_running_turn(self, user_id, session_id, turn_id).await
        })
    }

    fn list_session_turns_page<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        before_turn_id: Option<&'a str>,
        limit: usize,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<PersistedTurnPage>> + Send + 'a>,
    > {
        Box::pin(async move {
            ChatStore::list_session_turns_page(self, user_id, session_id, before_turn_id, limit)
                .await
        })
    }

    fn list_session_messages<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<
                    Output = anyhow::Result<Vec<openchat_infra::stores::PersistedSessionMessage>>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move { ChatStore::list_session_messages(self, user_id, session_id).await })
    }

    fn list_session_tool_calls<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<
                    Output = anyhow::Result<Vec<openchat_infra::stores::PersistedSessionToolCall>>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            ChatStore::list_session_tool_calls(self, user_id, session_id).await
        })
    }

    fn list_session_messages_for_turns<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        turn_ids: &'a [String],
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<
                    Output = anyhow::Result<Vec<openchat_infra::stores::PersistedSessionMessage>>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            ChatStore::list_session_messages_for_turns(self, user_id, session_id, turn_ids).await
        })
    }

    fn list_session_tool_calls_for_turns<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        turn_ids: &'a [String],
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<
                    Output = anyhow::Result<Vec<openchat_infra::stores::PersistedSessionToolCall>>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            ChatStore::list_session_tool_calls_for_turns(self, user_id, session_id, turn_ids)
                .await
        })
    }

    fn reconcile_session_runtime_state<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        active_turn_ids: &'a [String],
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            ChatStore::reconcile_session_runtime_state(
                self,
                user_id,
                session_id,
                active_turn_ids,
            )
            .await
        })
    }
}

impl SessionRuntimeRegistry for InMemorySessionStore {
    fn session_runtime<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = SessionRuntime> + Send + 'a>> {
        Box::pin(async move { InMemorySessionStore::session_runtime(self, session_id).await })
    }

    fn remove_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move { InMemorySessionStore::remove_session(self, session_id).await })
    }
}

impl ActiveTurnRegistryPort for ActiveTurnRegistry {
    fn register<'a>(
        &'a self,
        session_id: &'a str,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = ActiveTurnHandle> + Send + 'a>> {
        Box::pin(async move { ActiveTurnRegistry::register(self, session_id, turn_id).await })
    }

    fn interrupt<'a>(
        &'a self,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move { ActiveTurnRegistry::interrupt(self, turn_id).await })
    }

    fn active_turn_ids_for_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Vec<String>> + Send + 'a>> {
        Box::pin(async move {
            ActiveTurnRegistry::active_turn_ids_for_session(self, session_id).await
        })
    }
}
