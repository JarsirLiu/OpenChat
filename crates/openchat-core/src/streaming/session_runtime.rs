use std::{collections::HashMap, sync::Arc};

use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct StreamEventPayload {
    pub data: String,
}

#[derive(Clone)]
pub struct SessionRuntime {
    pub sender: broadcast::Sender<StreamEventPayload>,
}

#[derive(Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionRuntime>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn session_runtime(&self, session_id: &str) -> SessionRuntime {
        let mut sessions = self.sessions.write().await;
        if let Some(runtime) = sessions.get(session_id) {
            return runtime.clone();
        }

        let (sender, _) = broadcast::channel(256);
        let runtime = SessionRuntime { sender };
        sessions.insert(session_id.to_string(), runtime.clone());
        runtime
    }

    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }
}
