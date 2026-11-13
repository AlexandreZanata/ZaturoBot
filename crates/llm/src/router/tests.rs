use super::*;
use crate::{
    PrivacyClass,
    fake::{FakeBudget as Budget, FakeClient, FakeReply},
};
use zaturo_core::ModelUsage;

const NOW: i64 = 1_000_000;
struct TestClock;
impl Clock for TestClock {
    fn now_ms(&self) -> i64 {
        NOW
    }
}

fn config() -> ModelsConfig {
    let mut config: ModelsConfig =
        toml::from_str(include_str!("../../../../config/models.example.toml")).unwrap();
    config.enabled = true;
    config.allow_global_processing = true;
    config.retry_base_millis = 1;
    config
}
fn context() -> CallContext {
    CallContext {
        task_id: "task-1".into(),
        operation_id: "operation-1".into(),
        step: 1,
        deadline_at: NOW + 10_000,
        privacy: PrivacyClass::PublicCoding,
    }
}
fn response() -> ModelResponse {
    ModelResponse {
        model: crate::DEEPSEEK_MODEL.into(),
        content: "{\"action\":\"tool\",\"tool\":\"diagnostic.v1\"}".into(),
        usage: ModelUsage {
            input_tokens: 10,
            output_tokens: 20,
            cost_microusd: Some(2),
        },
        finish_reason: "stop".into(),
        duration_ms: 1,
        model_calls: 1,
        usage_complete: true,
    }
}
fn messages() -> Vec<Message> {
    vec![
        Message::system("Trusted coding instructions"),
        Message::user("Synthetic public coding request"),
    ]
}
fn router(config: ModelsConfig, client: Arc<FakeClient>) -> Router {
    Router::with_clock(config, client, Arc::new(TestClock)).unwrap()
}

#[tokio::test]
async fn explicit_policy_and_budget_denials_never_call_a_model() {
    let client = Arc::new(FakeClient::new([FakeReply::Answer(response())]));
    let mut disabled = config();
    disabled.enabled = false;
    assert_eq!(
        router(disabled, client.clone())
            .complete(
                &context(),
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await
            .unwrap_err(),
        Error::Disabled
    );
    let mut global = config();
    global.allow_global_processing = false;
    assert!(matches!(
        router(global, client.clone())
            .complete(
                &context(),
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await,
        Err(Error::Denied(_))
    ));
    let route = router(config(), client.clone());
    let mut private = context();
    private.privacy = PrivacyClass::PrivateCoding;
    assert!(matches!(
        route
            .complete(
                &private,
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await,
        Err(Error::Denied(_))
    ));
    let budget = Budget::default();
    budget.deny();
    assert_eq!(
        route
            .complete(&context(), messages(), &budget, &Cancellation::new())
            .await
            .unwrap_err(),
        Error::Budget
    );
    assert_eq!(client.calls(), 0);
}

#[tokio::test]
async fn successful_reservation_cannot_authorize_replay() {
    let client = Arc::new(FakeClient::new([FakeReply::Answer(response())]));
    let route = router(config(), client.clone());
    let budget = Budget::default();
    route
        .complete(&context(), messages(), &budget, &Cancellation::new())
        .await
        .unwrap();
    assert_eq!(
        route
            .complete(&context(), messages(), &budget, &Cancellation::new())
            .await
            .unwrap_err(),
        Error::Reconciliation
    );
    assert_eq!(client.calls(), 1);
    assert_eq!(budget.states()["operation-1:1:1"], "settled");
}

#[tokio::test(start_paused = true)]
async fn retry_reserves_each_attempt_and_marks_incomplete_usage() {
    let client = Arc::new(FakeClient::new([
        FakeReply::Error(Error::RateLimited {
            retry_after_seconds: None,
        }),
        FakeReply::Answer(response()),
    ]));
    let route = router(config(), client.clone());
    let budget = Budget::default();
    let response = route
        .complete(&context(), messages(), &budget, &Cancellation::new())
        .await
        .unwrap();
    assert_eq!(response.model_calls, 2);
    assert!(!response.usage_complete);
    assert_eq!(client.calls(), 2);
    let records = budget.states();
    assert_eq!(records["operation-1:1:1"], "uncertain");
    assert_eq!(records["operation-1:1:2"], "settled");
}

#[tokio::test]
async fn a_long_retry_after_is_not_shortened_into_an_early_retry() {
    let error = Error::RateLimited {
        retry_after_seconds: Some(60),
    };
    let client = Arc::new(FakeClient::new([
        FakeReply::Error(error.clone()),
        FakeReply::Answer(response()),
    ]));
    let route = router(config(), client.clone());
    assert_eq!(
        route
            .complete(
                &context(),
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await
            .unwrap_err(),
        error
    );
    assert_eq!(client.calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn timeout_is_uncertain_and_is_never_automatically_retried() {
    let client = Arc::new(FakeClient::new([
        FakeReply::Delay(Duration::from_secs(2), response()),
        FakeReply::Answer(response()),
    ]));
    let mut config = config();
    config.timeout_seconds = 1;
    let route = router(config, client.clone());
    let budget = Budget::default();
    assert_eq!(
        route
            .complete(&context(), messages(), &budget, &Cancellation::new())
            .await
            .unwrap_err(),
        Error::Timeout
    );
    assert_eq!(client.calls(), 1);
    assert_eq!(budget.states()["operation-1:1:1"], "uncertain");
}

#[tokio::test(start_paused = true)]
async fn cancellation_during_a_call_retains_the_unknown_reservation() {
    let client = Arc::new(FakeClient::new([FakeReply::Delay(
        Duration::from_secs(2),
        response(),
    )]));
    let route = router(config(), client.clone());
    let budget = Budget::default();
    let cancellation = Cancellation::new();
    let cancel = cancellation.clone();
    let stop = async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel.cancel();
    };
    let context = context();
    let call = route.complete(&context, messages(), &budget, &cancellation);
    let (result, ()) = tokio::join!(call, stop);
    assert_eq!(result.unwrap_err(), Error::Cancelled);
    assert_eq!(client.calls(), 1);
    assert_eq!(budget.states()["operation-1:1:1"], "uncertain");
}

#[tokio::test]
async fn context_deadline_model_and_output_limits_are_enforced() {
    let mut wrong = response();
    wrong.model = "unapproved-model".into();
    let client = Arc::new(FakeClient::new([FakeReply::Answer(wrong)]));
    let route = router(config(), client.clone());
    let mut expired = context();
    expired.deadline_at = NOW;
    assert_eq!(
        route
            .complete(
                &expired,
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await
            .unwrap_err(),
        Error::Deadline
    );
    assert_eq!(
        route
            .complete(
                &context(),
                vec![Message::user("x".repeat(9000))],
                &Budget::default(),
                &Cancellation::new()
            )
            .await
            .unwrap_err(),
        Error::InvalidRequest
    );
    assert_eq!(
        route
            .complete(
                &context(),
                messages(),
                &Budget::default(),
                &Cancellation::new()
            )
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
    assert_eq!(client.calls(), 1);
}

#[test]
fn canonical_config_rejects_model_changes_and_secret_debug_is_redacted() {
    let mut config = config();
    config.allowed_models.push("unapproved-model".into());
    assert!(config.validate().is_err());
    let text = "synthetic-secret-that-must-not-appear";
    let request = Message::user(text);
    let mut answer = response();
    answer.content = text.into();
    assert!(!format!("{request:?} {answer:?}").contains(text));
}
