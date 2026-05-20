use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ActiveTurnHandle {
    registry: Arc<ActiveTurnRegistry>,
    session_id: String,
    turn_id: String,
    cancellation_token: CancellationToken,
}

impl ActiveTurnHandle {
    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn turn_id(&self) -> &str {
        self.turn_id.as_str()
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub async fn finish(&self) {
        self.registry
            .unregister(self.session_id(), self.turn_id())
            .await;
    }
}

#[derive(Default)]
struct ActiveTurnState {
    by_turn_id: HashMap<String, CancellationToken>,
    by_session_id: HashMap<String, HashSet<String>>,
}

#[derive(Clone, Default)]
pub struct ActiveTurnRegistry {
    state: Arc<RwLock<ActiveTurnState>>,
}

impl ActiveTurnRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, session_id: &str, turn_id: &str) -> ActiveTurnHandle {
        let cancellation_token = CancellationToken::new();
        let mut state = self.state.write().await;
        state
            .by_turn_id
            .insert(turn_id.to_string(), cancellation_token.clone());
        state
            .by_session_id
            .entry(session_id.to_string())
            .or_default()
            .insert(turn_id.to_string());

        ActiveTurnHandle {
            registry: Arc::new(self.clone()),
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
            cancellation_token,
        }
    }

    pub async fn interrupt(&self, turn_id: &str) -> bool {
        let state = self.state.read().await;
        let Some(token) = state.by_turn_id.get(turn_id) else {
            return false;
        };
        token.cancel();
        true
    }

    pub async fn active_turn_ids_for_session(&self, session_id: &str) -> Vec<String> {
        let state = self.state.read().await;
        state
            .by_session_id
            .get(session_id)
            .map(|items| items.iter().cloned().collect())
            .unwrap_or_default()
    }

    async fn unregister(&self, session_id: &str, turn_id: &str) {
        let mut state = self.state.write().await;
        state.by_turn_id.remove(turn_id);

        if let Some(turn_ids) = state.by_session_id.get_mut(session_id) {
            turn_ids.remove(turn_id);
            if turn_ids.is_empty() {
                state.by_session_id.remove(session_id);
            }
        }
    }
}
