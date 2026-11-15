//! One job-scoped typed tool; proposals never add owner authority.
use super::{
    gateway::{Gate, GenerateArguments},
    state,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use zaturo_core::{
    IntentDecision, OpenCodeEvidence, OpenCodeWorkOrder, ServiceRequest, ServiceResult,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectArguments {
    pub decision: IntentDecision,
}

pub(crate) fn tool_definition(order: &OpenCodeWorkOrder) -> Value {
    if let Some((channel, count)) = order.generation() {
        return json!({"name":"youtube_generate","description":"Dispatch the exact approved generation order once. No upload or scheduling. The host observes completion.","inputSchema":{"type":"object","additionalProperties":false,"required":["operation_id","channel","count"],"properties":{"operation_id":{"type":"string","const":order.operation_id},"channel":{"type":"string","const":channel},"count":{"type":"integer","const":count}}}});
    }
    let catalog = zaturo_core::relevant_action_catalog(&order.owner_request);
    let mut decision: Value =
        serde_json::from_str(include_str!("../../../prompts/intent-router.schema.json"))
            .expect("compiled routing schema");
    if let Some(variants) = decision["oneOf"][2]["properties"]["action"]["oneOf"].as_array_mut() {
        variants.retain(|v| {
            catalog.as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r["action"] == v["properties"]["action"]["const"])
            })
        });
        if variants.is_empty() {
            decision["oneOf"]
                .as_array_mut()
                .expect("compiled routing union")
                .pop();
        }
    }
    json!({"name":"select_action","description":"Return a reply, a clarification, or one relevant registered action. This only proposes a typed decision; the Hub independently validates owner authority and capabilities.","inputSchema":{"type":"object","additionalProperties":false,"required":["decision"],"properties":{"decision":decision}}})
}

pub(crate) async fn mcp(
    State(gate): State<Arc<Gate>>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if !gate.authenticated(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(id) = request.get("id").cloned() else {
        return StatusCode::ACCEPTED.into_response();
    };
    let answer=match request["method"].as_str(){
        Some("initialize")=>json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"zaturo-job","version":"1"}}),
        Some("ping")=>json!({}),
        Some("tools/list")=>json!({"tools":[tool_definition(&gate.order)]}),
        Some("tools/call")=>{
            let definition=tool_definition(&gate.order);
            if request["params"]["name"]!=definition["name"]{denied()}
            else {
                let result=if gate.order.interpret(){
                    match serde_json::from_value::<SelectArguments>(request["params"]["arguments"].clone()){
                        Ok(args) if args.decision.validates_for(&gate.order.owner_request)=>select(&gate,args.decision).await,
                        _=>Err(anyhow::anyhow!("unapproved decision")),
                    }
                }else{
                    match serde_json::from_value::<GenerateArguments>(request["params"]["arguments"].clone()){
                        Ok(args) if args.matches(&gate.order)=>generate(&gate).await,
                        _=>Err(anyhow::anyhow!("unapproved generation")),
                    }
                };
                result.unwrap_or_else(|_|denied())
            }
        },
        _=>return Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not available"}})).into_response(),
    };
    Json(json!({"jsonrpc":"2.0","id":id,"result":answer})).into_response()
}
fn denied() -> Value {
    json!({"isError":true,"content":[{"type":"text","text":"The exact approved job requires reconciliation. No automatic retry is allowed."}]})
}
async fn begin(gate: &Gate) -> anyhow::Result<()> {
    let mut progress = gate.progress.lock().await;
    anyhow::ensure!(
        progress.billing.is_some()
            && progress.process.is_some()
            && progress.failure.is_none()
            && !progress.tool_started
            && super::now_ms() < gate.order.deadline_at,
        "no active approved inference receipt"
    );
    progress.tool_started = true;
    Ok(())
}
async fn finish(
    gate: &Gate,
    result: ServiceResult,
    proof: Option<OpenCodeEvidence>,
) -> anyhow::Result<Value> {
    let receipt_id = uuid::Uuid::new_v4().to_string();
    state::receipt(
        &gate.directory,
        &state::Receipt {
            phase: "tool_result".into(),
            updated_at: super::now_ms(),
            proof,
            result: Some(result.clone()),
            code: None,
        },
    )?;
    let mut progress = gate.progress.lock().await;
    progress.result = Some(result.clone());
    progress.receipt_id = Some(receipt_id.clone());
    drop(progress);
    gate.changed.notify_one();
    Ok(
        json!({"isError":false,"content":[{"type":"text","text":json!({"receipt_id":receipt_id,"operation_id":gate.order.operation_id,"outcome":result.outcome,"summary":result.summary}).to_string()}]}),
    )
}
async fn select(gate: &Gate, decision: IntentDecision) -> anyhow::Result<Value> {
    begin(gate).await?;
    let summary = match &decision {
        IntentDecision::Reply { text } => text.clone(),
        IntentDecision::Clarify { question } => question.clone(),
        IntentDecision::Action { .. } => {
            "Pedido interpretado. O Hub verificará o destino e registrará a execução separadamente."
                .into()
        }
    };
    let progress = gate.progress.lock().await;
    let (usage, billing) = progress
        .billing
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("billing missing"))?;
    let process = progress
        .process
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("process missing"))?;
    let result = ServiceResult {
        operation_id: gate.order.operation_id.clone(),
        action: "assistant_route".into(),
        outcome: "completed".into(),
        summary,
        data: json!({"decision":decision,"model":"deepseek-v4.1-flash","model_calls":1,"usage":usage,"billing":billing,"opencode_route":{"session_id":process.session,"executable_sha256":process.sha256,"version":super::VERSION,"process_id":process.pid,"process_start":process.start,"unit":process.unit,"target_worker":gate.order.target_worker,"invoked_tool":"select_action"}}),
    };
    drop(progress);
    anyhow::ensure!(
        result.validates(
            &gate.order.operation_id,
            &super::runner::route_request(&gate.order)?
        ),
        "invalid interpretation receipt"
    );
    finish(gate, result, None).await
}
async fn generate(gate: &Gate) -> anyhow::Result<Value> {
    begin(gate).await?;
    let (channel, count) = gate
        .order
        .generation()
        .ok_or_else(|| anyhow::anyhow!("generation required"))?;
    let progress = gate.progress.lock().await;
    let (usage, billing) = progress
        .billing
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("billing missing"))?;
    let process = progress
        .process
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("process missing"))?;
    let proof = OpenCodeEvidence {
        operation_id: gate.order.operation_id.clone(),
        session_id: process.session.clone(),
        executable_sha256: process.sha256.clone(),
        executable_version: super::VERSION.into(),
        process_id: process.pid,
        process_start: process.start.clone(),
        unit: process.unit.clone(),
        target_worker: gate.order.target_worker.clone(),
        channel: channel.into(),
        count,
        mode: "generate".into(),
        invoked_tool: "youtube_generate".into(),
        model: "deepseek-v4.1-flash".into(),
        model_calls: 1,
        billing_source: match billing.source {
            zaturo_llm::BillingSource::Go => "go",
            zaturo_llm::BillingSource::GoPlus => "go-plus",
            _ => anyhow::bail!("wrong billing source"),
        }
        .into(),
        usage: usage.clone(),
    };
    drop(progress);
    anyhow::ensure!(
        proof.validates(&gate.order.operation_id, channel, count),
        "invalid executor evidence"
    );
    // Persist proof before any detached media actor can be admitted.
    state::receipt(
        &gate.directory,
        &state::Receipt {
            phase: "generation_intent".into(),
            updated_at: super::now_ms(),
            proof: Some(proof.clone()),
            result: None,
            code: None,
        },
    )?;
    let youtube = gate
        .youtube
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("adapter missing"))?;
    let request = ServiceRequest::YoutubeGenerate {
        channel: channel.into(),
        count,
    };
    let result = match youtube.execute(&gate.order.operation_id, &request).await {
        Ok(value) => value,
        Err(_) => {
            youtube
                .reconcile(&gate.order.operation_id, &request)
                .await?
        }
    };
    finish(gate, result, Some(proof)).await
}
