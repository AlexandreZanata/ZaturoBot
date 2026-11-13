use super::*;
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
    body["usage"].as_object_mut().unwrap().remove("cost");
    let response = parse_response(&serde_json::to_vec(&body).unwrap(), 1).unwrap();
    assert_eq!(response.usage.cost_microusd, None);
    assert_eq!(cost_microusd(0.00003042).unwrap(), 31);
}
