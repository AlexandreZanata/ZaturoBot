//! One Personal/Go interpretation call; executable authority stays in deterministic host policy.
use zaturo_core::{ConversationTurn, IntentDecision};
use zaturo_llm::{
    BudgetPort, CallContext, Cancellation, Error, Message, ModelResponse, PrivacyClass, Router,
};

pub async fn route_intent(
    router: &Router,
    context: &CallContext,
    owner_message: &str,
    history: &[ConversationTurn],
    budget: &dyn BudgetPort,
    cancellation: &Cancellation,
) -> Result<(IntentDecision, ModelResponse), Error> {
    if context.privacy != PrivacyClass::Personal
        || !router.config().is_personal_route()
        || router.config().max_attempts != 1
        || router.config().provider != "opencode-go"
        || !router.config().allow_personal_go
        || !zaturo_core::ordinary_text(owner_message, 2048)
        || history.len() > 4
        || history.iter().any(|t| !t.validates())
        || history.iter().map(|t| t.text.len()).sum::<usize>() > 1024
    {
        return Err(Error::Denied("explicit bounded Personal routing required"));
    }
    let messages = vec![
        Message::system(include_str!("../../../prompts/intent-router.md")),
        Message::user(
            serde_json::json!({"owner_message":owner_message,"ordinary_history":history,
            "relevant_registered_actions":zaturo_core::relevant_action_catalog(owner_message)})
            .to_string(),
        ),
    ];
    let response = router
        .complete(context, messages, budget, cancellation)
        .await?;
    let decision: IntentDecision =
        serde_json::from_str(&response.content).map_err(|_| Error::InvalidResponse)?;
    if !decision.validates_for(owner_message) {
        return Err(Error::InvalidResponse);
    }
    Ok((decision, response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaturo_core::ModelUsage;
    use zaturo_llm::{
        BillingSource, ModelBilling, ModelsConfig,
        fake::{FakeBudget, FakeClient, FakeReply},
    };
    fn response(content: &str) -> ModelResponse {
        ModelResponse {
            model: zaturo_llm::DEEPSEEK_MODEL.into(),
            content: content.into(),
            usage: ModelUsage {
                input_tokens: 30,
                output_tokens: 30,
                cost_microusd: Some(0),
            },
            billing: Some(ModelBilling {
                source: BillingSource::Go,
                allowance_microusd: Some(0),
            }),
            finish_reason: "stop".into(),
            duration_ms: 1,
            model_calls: 1,
            usage_complete: true,
        }
    }
    #[tokio::test]
    async fn routing_uses_one_budgeted_call_and_cannot_invent_authority() {
        let mut config: ModelsConfig = toml::from_str(include_str!(
            "../../../config/assistant-models.example.toml"
        ))
        .unwrap();
        config.enabled = true;
        config.allow_global_processing = true;
        let fake = Arc::new(FakeClient::new([FakeReply::Answer(response(
            r#"{"kind":"action","action":{"action":"youtube_generate","channel":"fallem-prince","count":2}}"#,
        ))]));
        let router = Router::new(config, fake.clone()).unwrap();
        let context = CallContext {
            task_id: "synthetic-route".into(),
            operation_id: "synthetic-route-operation".into(),
            step: 1,
            deadline_at: i64::MAX,
            privacy: PrivacyClass::Personal,
        };
        let budget = FakeBudget::default();
        let outcome = route_intent(
            &router,
            &context,
            "não faça 2 videos para o falllem price",
            &[],
            &budget,
            &Cancellation::new(),
        )
        .await;
        assert!(matches!(outcome, Err(Error::InvalidResponse)));
        assert_eq!(fake.calls(), 1);
        assert_eq!(budget.states().len(), 1);
        assert!(
            route_intent(
                &router,
                &context,
                "não faça 2 videos para o falllem price",
                &[],
                &budget,
                &Cancellation::new()
            )
            .await
            .is_err()
        );
        assert_eq!(fake.calls(), 1);
    }
}
