use std::sync::Arc;

use futures_util::StreamExt;
use openchat_infra::sqlite::SqliteChatStore;

use crate::{
    execution::{
        event_builder::{build_session, build_session_updated_event, send_event},
        helpers::now_string,
    },
    model_provider_runtime::ModelProviderRuntime,
    ModelStreamEvent, SessionRuntime, TextModelAccessResolver, TurnPlan,
};

const MAX_GENERATED_TITLE_CHARS: usize = 10;
const FALLBACK_TITLE_CHARS: usize = 5;
const DEFAULT_EMPTY_TITLE: &str = "新对话";

pub struct SessionTitleGenerator<R> {
    chat_store: Arc<SqliteChatStore>,
    model_provider_runtime: ModelProviderRuntime<R>,
}

impl<R> Clone for SessionTitleGenerator<R> {
    fn clone(&self) -> Self {
        Self {
            chat_store: self.chat_store.clone(),
            model_provider_runtime: self.model_provider_runtime.clone(),
        }
    }
}

impl<R> SessionTitleGenerator<R>
where
    R: TextModelAccessResolver + Send + Sync + 'static,
{
    pub fn new(
        chat_store: Arc<SqliteChatStore>,
        model_provider_runtime: ModelProviderRuntime<R>,
    ) -> Self {
        Self {
            chat_store,
            model_provider_runtime,
        }
    }

    pub fn spawn_generate(&self, plan: TurnPlan, session_runtime: SessionRuntime) {
        let generator = self.clone();
        tokio::spawn(async move {
            generator.generate(plan, session_runtime).await;
        });
    }

    pub async fn ensure_initial_title(&self, plan: &TurnPlan) {
        let fallback_title = fallback_title_from_prompt(plan.prompt.as_str());
        if fallback_title.trim().is_empty() {
            return;
        }

        let Ok(Some(session)) = self.chat_store.get_session(plan.session_id.as_str()).await else {
            return;
        };

        if session
            .title
            .as_deref()
            .is_some_and(|title| !title.trim().is_empty())
        {
            return;
        }

        let _ = self
            .chat_store
            .update_session_title(plan.session_id.as_str(), fallback_title.as_str())
            .await;
    }

    async fn generate(&self, plan: TurnPlan, session_runtime: SessionRuntime) {
        let fallback_title = fallback_title_from_prompt(plan.prompt.as_str());
        let Ok(Some(session)) = self.chat_store.get_session(plan.session_id.as_str()).await else {
            return;
        };

        if session.title.as_deref().is_some_and(|title| {
            let normalized = title.trim();
            !normalized.is_empty() && normalized != fallback_title
        }) {
            return;
        }

        let generated_title = self
            .generate_title_with_model(&plan)
            .await
            .and_then(|title| normalize_generated_title(title.as_str()));

        let final_title = generated_title.unwrap_or(fallback_title);
        if final_title.trim().is_empty() {
            return;
        }

        let Ok(Some(updated_session)) = self
            .chat_store
            .update_session_title(plan.session_id.as_str(), final_title.as_str())
            .await
        else {
            return;
        };

        let _ = send_event(
            &session_runtime,
            &build_session_updated_event(
                updated_session.id.clone(),
                now_string(),
                build_session(
                    updated_session.id,
                    updated_session.title,
                    updated_session.status,
                    updated_session.created_at,
                    updated_session.updated_at,
                ),
            ),
        );
    }

    async fn generate_title_with_model(&self, plan: &TurnPlan) -> Option<String> {
        let title_prompt = build_title_prompt(plan.prompt.as_str());

        let title_plan = TurnPlan {
            user_id: plan.user_id.clone(),
            session_id: plan.session_id.clone(),
            prompt: title_prompt,
            attachments: Vec::new(),
            history: Vec::new(),
            text_model: plan.text_model.clone(),
            tool_list: Vec::new(),
        };

        let mut stream = self.model_provider_runtime.stream_text(&title_plan).await.ok()?;
        let mut title = String::new();

        while let Some(event) = stream.next().await {
            match event.ok()? {
                ModelStreamEvent::TextDelta(delta) => title.push_str(delta.as_str()),
                ModelStreamEvent::ReasoningDelta(_)
                | ModelStreamEvent::ToolCallStart { .. }
                | ModelStreamEvent::ToolCallArgumentsDelta { .. }
                | ModelStreamEvent::ToolCallComplete { .. } => {}
            }
        }

        Some(title)
    }
}

fn normalize_generated_title(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '“' | '”' | '‘' | '’' | '《' | '》'))
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '“' | '”' | '‘' | '’' | '《' | '》'))
        .trim_matches(|ch: char| matches!(ch, '。' | '.' | ',' | '，' | ':' | '：' | ';' | '；'));

    if trimmed.is_empty() {
        return None;
    }

    let title = trimmed
        .chars()
        .take(MAX_GENERATED_TITLE_CHARS)
        .collect::<String>();
    if title.trim().is_empty() {
        None
    } else {
        Some(title)
    }
}

fn fallback_title_from_prompt(prompt: &str) -> String {
    let compact = prompt.split_whitespace().collect::<String>();
    let fallback = compact.chars().take(FALLBACK_TITLE_CHARS).collect::<String>();
    if fallback.is_empty() {
        DEFAULT_EMPTY_TITLE.to_string()
    } else {
        fallback
    }
}

fn build_title_prompt(user_prompt: &str) -> String {
    format!(
        "请根据下面这段用户消息，生成一个简洁的中文会话标题。\n要求：\n1. 不超过{}个字。\n2. 不加引号、句号、书名号或其他解释。\n3. 只返回标题本身。\n\n用户消息：\n{}",
        MAX_GENERATED_TITLE_CHARS,
        user_prompt.trim()
    )
}
