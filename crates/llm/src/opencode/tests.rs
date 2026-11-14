use super::*;
use crate::{Cancellation, PrivacyClass, fake::FakeBudget};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode, header::LOCATION},
    response::IntoResponse,
    routing::post,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

const KEY: &str = "synthetic-opencode-key-for-local-fixtures-only";
type CapturedRequest = Arc<Mutex<Option<(HeaderMap, serde_json::Value)>>>;
fn config() -> ModelsConfig {
    let mut c: ModelsConfig =
        toml::from_str(include_str!("../../../../config/models.example.toml")).unwrap();
    c.enabled = true;
    c.allow_global_processing = true;
    c
}
fn personal_config() -> ModelsConfig {
    let mut c = config();
    c.allow_personal_go = true;
    c.allowed_privacy = vec![PrivacyClass::Personal];
    c.max_attempts = 1;
    c
}
fn request() -> ModelRequest {
    ModelRequest {
        model: crate::DEEPSEEK_MODEL.into(),
        context: CallContext {
            task_id: "task-1".into(),
            operation_id: "operation-1".into(),
            step: 1,
            deadline_at: i64::MAX,
            privacy: PrivacyClass::PublicCoding,
        },
        request_id: "operation-1:1:1".into(),
        messages: vec![Message::user("Synthetic coding environment diagnostic")],
        max_output_tokens: 512,
    }
}
fn answer() -> serde_json::Value {
    serde_json::json!({"model":crate::DEEPSEEK_MODEL,"choices":[{"message":{"content":"{\"action\":\"tool\",\"tool\":\"diagnostic.v1\"}","reasoning_content":"This private reasoning is deliberately ignored."},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"cost":0,"opencode":{"allowance":0.00003042}}})
}

async fn serve(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}"), task)
}

#[tokio::test]
async fn sends_verified_chat_contract_session_and_product_identity() {
    let received = Arc::new(Mutex::new(None));
    let router = Router::new()
        .route(
            "/chat",
            post(
                |State(received): State<CapturedRequest>,
                 headers: HeaderMap,
                 Json(body): Json<serde_json::Value>| async move {
                    *received.lock().unwrap() = Some((headers, body));
                    Json(answer())
                },
            ),
        )
        .with_state(received.clone());
    let (base, server) = serve(router).await;
    let client = OpenCodeGoClient::at(&config(), KEY.into(), format!("{base}/chat")).unwrap();
    let response = client.complete(request()).await.unwrap();
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 20);
    assert_eq!(response.usage.cost_microusd, Some(0));
    assert!(!response.content.contains("private reasoning"));
    let observed = received.lock().unwrap();
    let (headers, body) = observed.as_ref().unwrap();
    assert_eq!(headers["authorization"], format!("Bearer {KEY}"));
    assert_eq!(headers["x-opencode-session"], "operation-1");
    assert!(
        headers["user-agent"]
            .to_str()
            .unwrap()
            .starts_with("ZaturoBot/")
    );
    assert_eq!(body["model"], crate::DEEPSEEK_MODEL);
    assert_eq!(body["max_tokens"], 512);
    assert_eq!(body["stream"], false);
    assert!(body.get("tools").is_none());
    assert!(body.get("response_format").is_none());
    assert!(body.get("thinking").is_none());
    assert!(!format!("{client:?}").contains(KEY));
    server.abort();
}

#[tokio::test]
async fn provider_errors_never_echo_response_bodies_or_credentials() {
    let (base, server) = serve(Router::new().route(
        "/chat",
        post(|| async {
            (
                StatusCode::UNAUTHORIZED,
                format!("echoed credential: {KEY}"),
            )
        }),
    ))
    .await;
    let client = OpenCodeGoClient::at(&config(), KEY.into(), format!("{base}/chat")).unwrap();
    let error = client.complete(request()).await.unwrap_err();
    assert_eq!(error, Error::Authentication);
    assert!(!format!("{error:?}").contains(KEY));
    server.abort();
}

#[tokio::test]
async fn redirects_are_rejected_and_unbounded_chunked_bodies_are_stopped() {
    let router = Router::new()
        .route(
            "/redirect",
            post(|| async { (StatusCode::TEMPORARY_REDIRECT, [(LOCATION, "/chat")]) }),
        )
        .route("/chat", post(|| async { Json(answer()) }))
        .route(
            "/large",
            post(|| async {
                let chunks = futures_util::stream::iter(
                    (0..40).map(|_| Ok::<_, Infallible>(Bytes::from(vec![b'x'; 1024]))),
                );
                Body::from_stream(chunks).into_response()
            }),
        );
    let (base, server) = serve(router).await;
    let client = OpenCodeGoClient::at(&config(), KEY.into(), format!("{base}/redirect")).unwrap();
    assert_eq!(
        client.complete(request()).await.unwrap_err(),
        Error::Rejected(307)
    );
    let client = OpenCodeGoClient::at(&config(), KEY.into(), format!("{base}/large")).unwrap();
    assert_eq!(
        client.complete(request()).await.unwrap_err(),
        Error::ResponseTooLarge
    );
    server.abort();
}

#[test]
fn missing_usage_malformed_output_and_invalid_cost_fail_closed() {
    let mut body = answer();
    body.as_object_mut().unwrap().remove("usage");
    assert!(parse_response(&serde_json::to_vec(&body).unwrap(), 1).is_err());
    assert!(parse_response(b"malformed", 1).is_err());
    assert!(cost_microusd(-1.0).is_err());
    assert!(cost_microusd(f64::NAN).is_err());
}

#[test]
fn a_plan_allowance_is_never_substituted_for_a_missing_request_charge() {
    let mut body = answer();
    body["usage"]["opencode"]["billing_source"] = "go".into();
    body["usage"].as_object_mut().unwrap().remove("cost");
    let response = parse_response(&serde_json::to_vec(&body).unwrap(), 1).unwrap();
    assert_eq!(response.usage.cost_microusd, None);
    assert_eq!(response.billing.unwrap().allowance_microusd, Some(31));
    assert_eq!(cost_microusd(0.00003042).unwrap(), 31);
}

#[test]
fn explicit_personal_go_configuration_keeps_the_go_endpoint_and_privacy() {
    let personal = personal_config();
    let client = OpenCodeGoClient::new(&personal, KEY.into()).unwrap();
    assert_eq!(client.endpoint, GO_CHAT_ENDPOINT);
    assert_eq!(personal.privacy(), PrivacyClass::Personal);
    assert!(personal.is_personal_route());
    let mut unapproved = personal.clone();
    unapproved.allow_personal_go = false;
    assert!(OpenCodeGoClient::new(&unapproved, KEY.into()).is_err());
    let mut legacy = personal;
    legacy.provider = "opencode-console".into();
    legacy.allow_personal_go = false;
    let console = OpenCodeGoClient::new_console(&legacy, KEY.into()).unwrap();
    assert_eq!(console.endpoint, CONSOLE_CHAT_ENDPOINT);
    assert!(OpenCodeGoClient::new(&legacy, KEY.into()).is_err());
    assert!(OpenCodeGoClient::new_console(&personal_config(), KEY.into()).is_err());
}

#[tokio::test]
async fn personal_go_sends_real_client_identity_disables_thinking_and_keeps_allowance_separate() {
    let received = Arc::new(Mutex::new(None));
    let router = Router::new()
        .route(
            "/inference/go/openai/v1/chat/completions",
            post(
                |State(received): State<CapturedRequest>,
                 headers: HeaderMap,
                 Json(body): Json<serde_json::Value>| async move {
                    *received.lock().unwrap() = Some((headers, body));
                    let mut body = answer();
                    body["usage"]["opencode"]["billing_source"] = "go-plus".into();
                    Json(body)
                },
            ),
        )
        .with_state(received.clone());
    let (base, server) = serve(router).await;
    let client = OpenCodeGoClient::at(
        &personal_config(),
        KEY.into(),
        format!("{base}/inference/go/openai/v1/chat/completions"),
    )
    .unwrap();
    let client = Arc::new(client);
    let route = crate::Router::new(personal_config(), client.clone()).unwrap();
    let budget = FakeBudget::default();
    let mut request = request();
    request.context.privacy = PrivacyClass::Personal;
    let response = route
        .complete(
            &request.context,
            request.messages.clone(),
            &budget,
            &Cancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(response.usage.cost_microusd, Some(0));
    assert_eq!(budget.states()["operation-1:1:1"], "settled");
    assert_eq!(
        response.billing,
        Some(ModelBilling {
            source: BillingSource::GoPlus,
            allowance_microusd: Some(31)
        })
    );
    let (headers, body) = received.lock().unwrap().clone().unwrap();
    assert_eq!(headers["authorization"], format!("Bearer {KEY}"));
    assert_eq!(headers["x-opencode-session"], "operation-1");
    assert!(
        headers["user-agent"]
            .to_str()
            .unwrap()
            .starts_with("ZaturoBot/")
    );
    assert_eq!(body["thinking"]["type"], "disabled");
    assert_eq!(body["model"], crate::DEEPSEEK_MODEL);
    assert_eq!(body["stream"], false);
    assert!(body.get("tools").is_none());
    assert!(!response.content.contains("private reasoning"));
    request.context.privacy = PrivacyClass::PublicCoding;
    assert!(matches!(
        client.complete(request).await,
        Err(Error::Denied(_))
    ));
    server.abort();
}

#[tokio::test]
async fn personal_go_http_usage_settles_before_billing_denial_without_console_fallback() {
    let cases = [
        (None, Some(0.0)),
        (Some("balance"), Some(0.0)),
        (Some("byok"), Some(0.0)),
        (Some("free"), Some(0.0)),
        (Some("go"), None),
        (Some("go"), Some(0.000001)),
    ];
    for (source, charge) in cases {
        let mut body = answer();
        if let Some(source) = source {
            body["usage"]["opencode"]["billing_source"] = source.into();
        }
        body["usage"].as_object_mut().unwrap().remove("cost");
        if let Some(charge) = charge {
            body["usage"]["cost"] = charge.into();
        }
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let (base, server) = serve(Router::new().route(
            "/chat",
            post(move || {
                let body = body.clone();
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move { Json(body) }
            }),
        ))
        .await;
        let client =
            OpenCodeGoClient::at(&personal_config(), KEY.into(), format!("{base}/chat")).unwrap();
        let route = crate::Router::new(personal_config(), Arc::new(client)).unwrap();
        let budget = FakeBudget::default();
        let mut request = request();
        request.context.privacy = PrivacyClass::Personal;
        let error = route
            .complete(
                &request.context,
                request.messages,
                &budget,
                &Cancellation::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            Error::BillingSource,
            "source {source:?}, charge {charge:?}"
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(budget.states().len(), 1);
        assert_eq!(budget.states()["operation-1:1:1"], "settled");
        assert!(!error.to_string().contains(KEY));
        server.abort();
    }
}

#[test]
fn billing_metadata_rejects_unknown_sources_and_never_relabels_actual_charge() {
    for source in ["go", "go-plus", "balance", "byok", "free"] {
        let mut body = answer();
        body["usage"]["opencode"]["billing_source"] = source.into();
        body["usage"]["cost"] = 0.000002.into();
        let response = parse_response(&serde_json::to_vec(&body).unwrap(), 1).unwrap();
        assert_eq!(response.usage.cost_microusd, Some(2));
        let billing = response.billing.unwrap();
        assert_eq!(billing.allowance_microusd, Some(31));
        assert_eq!(serde_json::to_value(&billing).unwrap()["source"], source);
    }
    for source in ["go_plus", "go-teams", "unknown", ""] {
        let mut body = answer();
        body["usage"]["opencode"]["billing_source"] = source.into();
        assert_eq!(
            parse_response(&serde_json::to_vec(&body).unwrap(), 1).unwrap_err(),
            Error::InvalidResponse
        );
    }
    let mut body = answer();
    body["usage"]["opencode"]["allowance"] = (-1).into();
    assert_eq!(
        parse_response(&serde_json::to_vec(&body).unwrap(), 1).unwrap_err(),
        Error::InvalidResponse
    );
}
