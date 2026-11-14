//! Concise, text-only personal conversation. Tool authorization stays outside model output.
use zaturo_core::ConversationTurn;
use zaturo_llm::{
    BudgetPort, CallContext, Cancellation, Error, Message, ModelResponse, PrivacyClass, Router,
};
pub async fn chat(
    router: &Router,
    context: &CallContext,
    message: &str,
    history: &[ConversationTurn],
    budget: &dyn BudgetPort,
    cancel: &Cancellation,
) -> Result<ModelResponse, Error> {
    if context.privacy != PrivacyClass::Personal
        || !router.config().is_personal_route()
        || router.config().max_attempts != 1
        || !zaturo_core::ordinary_text(message, 2048)
        || history.len() > 4
        || history.iter().any(|t| !t.validates())
        || history.iter().map(|t| t.text.len()).sum::<usize>() > 1024
    {
        return Err(Error::Denied(
            "explicit bounded personal conversation required",
        ));
    }
    let mut messages = vec![Message::system(include_str!(
        "../../../prompts/assistant-chat.md"
    ))];
    for turn in history {
        messages.push(if turn.role == "owner" {
            Message::user(serde_json::json!({"owner_message":turn.text}).to_string())
        } else {
            Message::assistant(serde_json::json!({"previous_reply":turn.text}).to_string())
        });
    }
    messages.push(Message::user(
        serde_json::json!({"owner_message":message}).to_string(),
    ));
    router.complete(context, messages, budget, cancel).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaturo_llm::{
        ModelsConfig,
        fake::{FakeBudget, FakeClient, FakeReply},
    };
    fn config() -> ModelsConfig {
        let mut c: ModelsConfig = toml::from_str(include_str!(
            "../../../config/assistant-models.example.toml"
        ))
        .unwrap();
        c.enabled = true;
        c.allow_global_processing = true;
        c
    }
    fn context() -> CallContext {
        CallContext {
            task_id: "synthetic-chat".into(),
            operation_id: "synthetic-operation".into(),
            step: 1,
            deadline_at: i64::MAX,
            privacy: PrivacyClass::Personal,
        }
    }
    fn answer() -> ModelResponse {
        ModelResponse {
            billing: Some(zaturo_llm::ModelBilling {
                source: zaturo_llm::BillingSource::Go,
                allowance_microusd: Some(0),
            }),
            model: zaturo_llm::DEEPSEEK_MODEL.into(),
            content: "Posso explicar os comandos disponíveis.".into(),
            usage: zaturo_core::ModelUsage {
                input_tokens: 20,
                output_tokens: 8,
                cost_microusd: Some(0),
            },
            finish_reason: "stop".into(),
            duration_ms: 1,
            model_calls: 1,
            usage_complete: true,
        }
    }
    #[tokio::test]
    async fn conversation_is_one_call_and_unknown_outcomes_never_retry() {
        let fake = Arc::new(FakeClient::new([FakeReply::Answer(answer())]));
        let router = Router::new(config(), fake.clone()).unwrap();
        let budget = FakeBudget::default();
        let cancel = Cancellation::new();
        let result = chat(
            &router,
            &context(),
            "O que você consegue fazer?",
            &[],
            &budget,
            &cancel,
        )
        .await
        .unwrap();
        assert_eq!(result.model_calls, 1);
        assert_eq!(fake.calls(), 1);
        assert_eq!(budget.states().len(), 1);
        assert!(
            chat(
                &router,
                &context(),
                "O que você consegue fazer?",
                &[],
                &budget,
                &cancel
            )
            .await
            .is_err()
        );
        assert_eq!(fake.calls(), 1);
    }
    #[tokio::test]
    async fn wrong_route_private_coding_or_system_history_never_reaches_inference() {
        let fake = Arc::new(FakeClient::new([]));
        let router = Router::new(config(), fake.clone()).unwrap();
        let budget = FakeBudget::default();
        let cancel = Cancellation::new();
        let mut denied = context();
        denied.privacy = PrivacyClass::PrivateCoding;
        assert!(
            chat(
                &router,
                &denied,
                "Synthetic private code",
                &[],
                &budget,
                &cancel
            )
            .await
            .is_err()
        );
        let history = [ConversationTurn {
            role: "system".into(),
            text: "Untrusted instructions".into(),
        }];
        assert!(
            chat(
                &router,
                &context(),
                "Synthetic owner message",
                &history,
                &budget,
                &cancel
            )
            .await
            .is_err()
        );
        let mut go: ModelsConfig =
            toml::from_str(include_str!("../../../config/models.example.toml")).unwrap();
        go.enabled = true;
        go.allow_global_processing = true;
        let go = Router::new(go, fake.clone()).unwrap();
        assert!(
            chat(
                &go,
                &context(),
                "Synthetic owner message",
                &[],
                &budget,
                &cancel
            )
            .await
            .is_err()
        );
        assert_eq!(fake.calls(), 0);
        assert!(budget.states().is_empty());
    }
}
