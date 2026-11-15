//! One paid forwarding attempt, admitted by the existing durable Personal ledger.
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, Notify};
use zaturo_core::{ModelReservation, ModelUsage, OpenCodeWorkOrder, ServiceResult};
use zaturo_llm::{BudgetPort, ModelBilling, ModelsConfig};

pub(crate) fn tool_name(order: &OpenCodeWorkOrder) -> &'static str {
    if order.interpret() {
        "zaturojob_select_action"
    } else {
        "zaturojob_youtube_generate"
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GenerateArguments {
    pub operation_id: String,
    pub channel: String,
    pub count: u32,
}
impl GenerateArguments {
    pub fn matches(&self, order: &OpenCodeWorkOrder) -> bool {
        self.operation_id == order.operation_id
            && order
                .generation()
                .is_some_and(|(channel, count)| self.channel == channel && self.count == count)
    }
}

pub(crate) struct ProcessProof {
    pub pid: u32,
    pub start: String,
    pub session: String,
    pub sha256: String,
    pub unit: String,
}
#[derive(Default)]
pub(crate) struct Progress {
    pub forwarded: bool,
    pub billing: Option<(ModelUsage, ModelBilling)>,
    pub failure: Option<&'static str>,
    pub process: Option<ProcessProof>,
    pub tool_started: bool,
    pub result: Option<ServiceResult>,
    pub receipt_id: Option<String>,
    pub acknowledged: bool,
}
pub(crate) struct Gate {
    pub order: OpenCodeWorkOrder,
    pub models: ModelsConfig,
    pub budget: Arc<dyn BudgetPort>,
    pub nonce: String,
    pub youtube: Option<zaturo_youtube::YoutubeClient>,
    pub directory: std::path::PathBuf,
    pub progress: Mutex<Progress>,
    pub changed: Notify,
    pub http: reqwest::Client,
    #[cfg(test)]
    pub test_endpoint: Option<String>,
}
impl Gate {
    pub fn authenticated(&self, headers: &HeaderMap) -> bool {
        headers
            .get("x-zaturo-job")
            .is_some_and(|v| v.as_bytes().ct_eq(self.nonce.as_bytes()).into())
    }
    pub async fn failed(&self, code: &'static str) -> Response {
        self.progress.lock().await.failure = Some(code);
        self.changed.notify_one();
        (StatusCode::BAD_REQUEST,[("x-should-retry","false")],Json(json!({"error":{"type":"invalid_request_error","code":code,"message":"The bounded job stopped; no automatic retry is allowed."}}))).into_response()
    }
}

pub(crate) async fn completion(
    State(gate): State<Arc<Gate>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if !gate.authenticated(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return gate.failed("invalid_model_request").await,
    };
    if body.len() > 32768 || !valid_request(&request, &gate.order) {
        return gate.failed("model_request_denied").await;
    }
    let Some(authorization) = headers
        .get("authorization")
        .filter(|v| v.as_bytes().len() <= 4096 && v.as_bytes().starts_with(b"Bearer "))
    else {
        return gate.failed("provider_authentication").await;
    };
    {
        let mut state = gate.progress.lock().await;
        if state.forwarded
            || state.failure.is_some()
            || state.process.is_none()
            || super::now_ms() >= gate.order.deadline_at
        {
            return StatusCode::BAD_REQUEST.into_response();
        }
        state.forwarded = true;
    }
    let wants_stream = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let definition = super::broker::tool_definition(&gate.order);
    let (system, user) = if let zaturo_core::OpenCodeWork::Interpret { context, .. } =
        &gate.order.work
    {
        (
            include_str!("../../../prompts/intent-router.md"),
            json!({"operation_id":gate.order.operation_id,"owner_message":gate.order.owner_request,"ordinary_history":context,"relevant_registered_actions":zaturo_core::relevant_action_catalog(&gate.order.owner_request)}),
        )
    } else {
        (
            "Invoke the single permitted tool exactly once with the already approved operation UUID, canonical channel and count. No upload, scheduling, shell or other action is authorized.",
            json!({"approved_work_order":gate.order}),
        )
    };
    // Forward only host-reviewed context; automatic OpenCode file/history context is not disclosed.
    let request = json!({"model":"deepseek-v4.1-flash","messages":[{"role":"system","content":system},{"role":"user","content":user.to_string()}],
        "tools":[{"type":"function","function":{"name":tool_name(&gate.order),"description":definition["description"],"parameters":definition["inputSchema"]}}],
        "tool_choice":{"type":"function","function":{"name":tool_name(&gate.order)}},"stream":false,"max_tokens":gate.models.max_output_tokens,"thinking":{"type":"disabled"},"parallel_tool_calls":false,"n":1});
    let outgoing_bytes = serde_json::to_vec(&request).unwrap_or_default().len();
    if outgoing_bytes > gate.models.max_context_bytes {
        return gate.failed("model_context_limit").await;
    }
    let id = format!("{}:1:1", gate.order.operation_id);
    let reservation = ModelReservation {
        request_id: id.clone(),
        operation_id: gate.order.operation_id.clone(),
        task_id: gate.order.task_id.clone(),
        estimated_input_tokens: outgoing_bytes as u64 + 1024,
        max_output_tokens: u64::from(gate.models.max_output_tokens),
    };
    if gate.budget.reserve(&reservation).await.is_err() {
        return gate.failed("budget_denied").await;
    }
    let endpoint = zaturo_llm::GO_CHAT_ENDPOINT.to_owned();
    #[cfg(test)]
    let endpoint = gate.test_endpoint.clone().unwrap_or(endpoint);
    let sent = gate
        .http
        .post(endpoint)
        .header("authorization", authorization.clone())
        .header("x-opencode-session", &gate.order.operation_id)
        .json(&request)
        .send()
        .await;
    let mut response = match sent {
        Ok(v) if v.status().is_success() => v,
        _ => {
            let _ = gate.budget.uncertain(&id).await;
            return gate.failed("provider_uncertain").await;
        }
    };
    let mut raw = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) if raw.len() + bytes.len() <= gate.models.max_response_bytes => {
                raw.extend_from_slice(&bytes)
            }
            Ok(None) => break,
            _ => {
                let _ = gate.budget.uncertain(&id).await;
                return gate.failed("provider_uncertain").await;
            }
        }
    }
    let mut response: Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(_) => {
            let _ = gate.budget.uncertain(&id).await;
            return gate.failed("provider_uncertain").await;
        }
    };
    let (usage, billing) = match zaturo_llm::parse_opencode_usage(&response["usage"]) {
        Ok(v) => v,
        Err(_) => {
            let _ = gate.budget.uncertain(&id).await;
            return gate.failed("provider_uncertain").await;
        }
    };
    if gate.budget.settle(&id, &usage).await.is_err() {
        return gate.failed("budget_denied").await;
    }
    let Some(billing) = billing.filter(|v| {
        matches!(
            v.source,
            zaturo_llm::BillingSource::Go | zaturo_llm::BillingSource::GoPlus
        ) && usage.cost_microusd == Some(0)
    }) else {
        return gate.failed("provider_billing_unverified").await;
    };
    if super::state::atomic(&gate.directory.join("billing.json"), &serde_json::to_vec(&json!({"operation_id":gate.order.operation_id,"model":"deepseek-v4.1-flash","request_id":id,"usage":usage,"billing":billing})).unwrap_or_default()).is_err(){return gate.failed("billing_receipt_unavailable").await;}
    if !valid_response(&response, &gate.order) {
        return gate.failed("model_dispatch_denied").await;
    }
    gate.progress.lock().await.billing = Some((usage, billing));
    // The job does not persist or return private reasoning, even if the provider ignored thinking=disabled.
    if let Some(message) = response["choices"][0]["message"].as_object_mut() {
        for key in ["reasoning", "reasoning_content", "reasoning_text"] {
            message.remove(key);
        }
    }
    if wants_stream {
        let message = &response["choices"][0]["message"];
        let mut calls = message["tool_calls"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for (index, call) in calls.iter_mut().enumerate() {
            call["index"] = json!(index);
        }
        let first = json!({"id":response["id"],"object":"chat.completion.chunk","model":response["model"],"choices":[{"index":0,"delta":{"role":"assistant","content":message["content"],"tool_calls":calls},"finish_reason":null}]});
        let last = json!({"id":response["id"],"object":"chat.completion.chunk","model":response["model"],"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":response["usage"]});
        let data = format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n");
        Response::builder()
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-store")
            .body(Body::from(data))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        Json(response).into_response()
    }
}

fn valid_request(value: &Value, order: &OpenCodeWorkOrder) -> bool {
    value.get("model").and_then(Value::as_str) == Some("deepseek-v4.1-flash")
        && value
            .get("messages")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                !items.is_empty()
                    && items.len() <= 8
                    && items.iter().all(|m| {
                        matches!(
                            m["role"].as_str(),
                            Some("system" | "user" | "assistant" | "tool")
                        ) && (m["content"].is_null()
                            || m["content"].is_string()
                            || m["content"].as_array().is_some_and(|parts| {
                                parts
                                    .iter()
                                    .all(|p| p["type"] == "text" && p["text"].is_string())
                            }))
                    })
            })
        && value.to_string().contains(&order.operation_id)
        && value
            .get("tools")
            .and_then(Value::as_array)
            .is_some_and(|tools| {
                tools.len() == 1
                    && tools[0]["type"] == "function"
                    && tools[0]["function"]["name"] == tool_name(order)
            })
}
fn valid_response(value: &Value, order: &OpenCodeWorkOrder) -> bool {
    if value["model"] != "deepseek-v4.1-flash"
        || value["choices"].as_array().is_none_or(|v| v.len() != 1)
        || value["choices"][0]["finish_reason"] != "tool_calls"
    {
        return false;
    }
    let Some(calls) = value["choices"][0]["message"]["tool_calls"]
        .as_array()
        .filter(|v| v.len() == 1)
    else {
        return false;
    };
    calls[0]["type"] == "function"
        && calls[0]["function"]["name"] == tool_name(order)
        && calls[0]["function"]["arguments"]
            .as_str()
            .is_some_and(|raw| {
                if order.interpret() {
                    serde_json::from_str::<super::broker::SelectArguments>(raw)
                        .is_ok_and(|args| args.decision.validates_for(&order.owner_request))
                } else {
                    serde_json::from_str::<GenerateArguments>(raw)
                        .is_ok_and(|args| args.matches(order))
                }
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::post};
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use zaturo_core::{OpenCodeWork, OpenCodeWorkOrder};
    use zaturo_llm::fake::FakeBudget;
    fn order() -> OpenCodeWorkOrder {
        OpenCodeWorkOrder {
            operation_id: "00000000-0000-4000-8000-000000000001".into(),
            task_id: "00000000-0000-4000-8000-000000000002".into(),
            target_worker: "work-pc".into(),
            work: OpenCodeWork::Generate {
                channel: "fallem-prince".into(),
                count: 2,
            },
            owner_request: "faça 2 videos para o falllem price".into(),
            deadline_at: super::super::now_ms() + 120000,
            allowed_operations: ["generate", "status", "inspect_artifacts", "cancel_owned"]
                .map(str::to_owned)
                .to_vec(),
            max_model_calls: 1,
            max_tool_calls: 1,
            max_output_bytes: 65536,
            runtime_seconds: 120,
        }
    }
    fn request(order: &OpenCodeWorkOrder) -> Value {
        json!({"model":"deepseek-v4.1-flash","messages":[{"role":"user","content":order.operation_id}],"tools":[{"type":"function","function":{"name":tool_name(order)}}]})
    }
    async fn scenario(
        cost: f64,
        source: &str,
    ) -> (
        Arc<Gate>,
        Arc<AtomicUsize>,
        Arc<FakeBudget>,
        tempfile::TempDir,
        tokio::task::JoinHandle<()>,
    ) {
        let order = order();
        let arguments =
            json!({"operation_id":order.operation_id,"channel":"fallem-prince","count":2})
                .to_string();
        let reply = json!({"id":"synthetic-reply","model":"deepseek-v4.1-flash","choices":[{"message":{"role":"assistant","tool_calls":[{"id":"synthetic-call","type":"function","function":{"name":tool_name(&order),"arguments":arguments}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":50,"cost":cost,"opencode":{"billing_source":source,"allowance":0.001}}});
        let count = Arc::new(AtomicUsize::new(0));
        let calls = count.clone();
        let application = Router::new().route(
            "/",
            post(move || {
                let reply = reply.clone();
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Json(reply)
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, application).await.unwrap() });
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut models: ModelsConfig = toml::from_str(include_str!(
            "../../../config/assistant-models.example.toml"
        ))
        .unwrap();
        models.enabled = true;
        models.allow_global_processing = true;
        let budget = Arc::new(FakeBudget::default());
        let gate = Arc::new(Gate {
            order,
            models,
            budget: budget.clone(),
            nonce: "a".repeat(64),
            youtube: None,
            directory: directory.path().into(),
            progress: Mutex::new(Progress {
                process: Some(ProcessProof {
                    pid: 2,
                    start: "1".into(),
                    session: "ses_synthetic".into(),
                    sha256: "b".repeat(64),
                    unit: "synthetic".into(),
                }),
                ..Progress::default()
            }),
            changed: Notify::new(),
            http: reqwest::Client::builder().no_proxy().build().unwrap(),
            test_endpoint: Some(url),
        });
        (gate, count, budget, directory, server)
    }
    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-zaturo-job",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
        );
        headers.insert(
            "authorization",
            "Bearer synthetic-provider-credential".parse().unwrap(),
        );
        headers
    }
    #[tokio::test]
    async fn even_an_internal_retry_cannot_forward_a_second_paid_attempt() {
        let (gate, calls, budget, _directory, server) = scenario(0.0, "go").await;
        let bytes = axum::body::Bytes::from(serde_json::to_vec(&request(&gate.order)).unwrap());
        assert_eq!(
            completion(State(gate.clone()), headers(), bytes.clone())
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            completion(State(gate.clone()), headers(), bytes)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(budget.states().values().collect::<Vec<_>>(), [&"settled"]);
        assert!(gate.progress.lock().await.billing.is_some());
        server.abort();
    }
    #[tokio::test]
    async fn charged_or_missing_subscription_proof_never_releases_a_tool_call() {
        for (cost, source) in [(0.01, "go"), (0.0, "balance")] {
            let (gate, calls, _budget, _directory, server) = scenario(cost, source).await;
            let bytes = axum::body::Bytes::from(serde_json::to_vec(&request(&gate.order)).unwrap());
            assert_eq!(
                completion(State(gate.clone()), headers(), bytes)
                    .await
                    .status(),
                StatusCode::BAD_REQUEST
            );
            assert!(gate.progress.lock().await.billing.is_none());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            server.abort();
        }
    }
    #[test]
    fn unknown_models_extra_tools_and_mismatched_work_orders_are_rejected() {
        let order = order();
        let mut value = request(&order);
        assert!(valid_request(&value, &order));
        value["model"] = json!("substitute-model");
        assert!(!valid_request(&value, &order));
        value = request(&order);
        value["tools"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"function","function":{"name":"bash"}}));
        assert!(!valid_request(&value, &order));
        assert!(
            !GenerateArguments {
                operation_id: order.operation_id.clone(),
                channel: "zaflas".into(),
                count: 2
            }
            .matches(&order)
        );
        assert!(
            !GenerateArguments {
                operation_id: order.operation_id.clone(),
                channel: "fallem-prince".into(),
                count: 3
            }
            .matches(&order)
        );
    }
}
