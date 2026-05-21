use openchat_infra::stores::{PersistedSession, PersistedTurnPage};

use crate::{ActiveTurnHandle, SessionRuntime};

pub trait ChatRepository: Send + Sync {
    fn ensure_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn get_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = anyhow::Result<Option<PersistedSession>>> + Send + 'a,
        >,
    >;

    fn list_sessions<'a>(
        &'a self,
        user_id: &'a str,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<Vec<PersistedSession>>> + Send + 'a>,
    >;

    fn delete_session<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<bool>> + Send + 'a>>;

    fn update_session_title<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        title: &'a str,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = anyhow::Result<Option<PersistedSession>>> + Send + 'a,
        >,
    >;

    fn interrupt_running_turn<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<bool>> + Send + 'a>>;

    fn list_session_turns_page<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        before_turn_id: Option<&'a str>,
        limit: usize,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<PersistedTurnPage>> + Send + 'a>,
    >;

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
    >;

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
    >;

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
    >;

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
    >;

    fn reconcile_session_runtime_state<'a>(
        &'a self,
        user_id: &'a str,
        session_id: &'a str,
        active_turn_ids: &'a [String],
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<()>> + Send + 'a>>;
}

pub trait SessionRuntimeRegistry: Send + Sync {
    fn session_runtime<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = SessionRuntime> + Send + 'a>>;

    fn remove_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = ()> + Send + 'a>>;
}

pub trait ActiveTurnRegistryPort: Send + Sync {
    fn register<'a>(
        &'a self,
        session_id: &'a str,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = ActiveTurnHandle> + Send + 'a>>;

    fn interrupt<'a>(
        &'a self,
        turn_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = bool> + Send + 'a>>;

    fn active_turn_ids_for_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Vec<String>> + Send + 'a>>;
}
