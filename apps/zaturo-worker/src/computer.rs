//! Home plans through the existing paid ledger; the work PC executes independently granted steps.
use crate::{config::WorkerConfig, now_ms};
use std::sync::Arc;
use zaturo_core::{
    Assignment, ComputerAction, ComputerLease, ComputerObservation, ComputerStep,
    ComputerStepRequest, ComputerStepStatus, FailureCode, ServiceRequest, ServiceResult,
};
use zaturo_db::{ServiceIntent, WorkerJournal};
use zaturo_llm::{CallContext, Cancellation, PrivacyClass};
use zaturo_protocol::ClientMessage;
pub async fn execute(
    work: Assignment,
    journal: Arc<WorkerJournal>,
    config: WorkerConfig,
    token: String,
    cancel: Cancellation,
) -> ClientMessage {
    let result = tokio::select! {biased;_=cancel.cancelled()=>Err(FailureCode::ReconciliationRequired),result=run(&work,journal,&config,&token,&cancel)=>result};
    match result {
        Ok(result) => ClientMessage::CompletedService {
            task_id: work.task.id,
            lease_token: work.lease_token,
            result,
        },
        Err(code) => ClientMessage::Failed {
            task_id: work.task.id,
            lease_token: work.lease_token,
            code,
        },
    }
}
async fn run(
    work: &Assignment,
    journal: Arc<WorkerJournal>,
    config: &WorkerConfig,
    token: &str,
    cancel: &Cancellation,
) -> Result<ServiceResult, FailureCode> {
    let request = work
        .task
        .service
        .as_ref()
        .ok_or(FailureCode::PolicyDenied)?;
    if !request.validates()
        || work.task.kind != zaturo_core::TaskKind::Service
        || work.task.assigned_worker.as_deref() != Some(&config.worker_id)
        || work.task.lease_token.as_deref() != Some(&work.lease_token)
        || work.lease_expires_at <= now_ms()
        || work.task.deadline_at <= now_ms()
        || !config
            .capabilities
            .iter()
            .any(|c| c == request.capability())
    {
        return Err(FailureCode::PolicyDenied);
    }
    let expected = request.clone();
    let local = journal.clone();
    let operation = work.task.operation_id.clone();
    let id = operation.clone();
    let task = work.task.id.clone();
    match tokio::task::spawn_blocking(move || local.begin_service(&id, &task, &expected, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?
    {
        ServiceIntent::Cached(result) => {
            return if result.validates(&operation, request) {
                Ok(result)
            } else {
                Err(FailureCode::ReconciliationRequired)
            };
        }
        ServiceIntent::Reconcile => return Err(FailureCode::ReconciliationRequired),
        ServiceIntent::Execute => {}
    }
    let mut result=ServiceResult{operation_id:operation.clone(),action:request.name().into(),outcome:"blocked".into(),summary:"O adaptador local de computador não está configurado. Verifique o perfil e as permissões no PC de trabalho.".into(),data:serde_json::json!({"code":"computer_unavailable"})};
    match request {
        ServiceRequest::ComputerTask { request: goal, .. } => {
            let mut model_config = config.clone();
            model_config.assistant_models_config = config.computer_models_config.clone();
            if let Ok(Some(router)) = crate::assistant::load(&model_config) {
                let budget = crate::assistant::AssistantBudget {
                    journal: journal.clone(),
                    limits: router.config().budget.clone(),
                };
                let port = RemoteComputer::new(config, work, token)
                    .map_err(|_| FailureCode::ExecutionFailed)?;
                let context = CallContext {
                    task_id: work.task.id.clone(),
                    operation_id: operation.clone(),
                    step: 1,
                    deadline_at: work.task.deadline_at,
                    privacy: PrivacyClass::Personal,
                };
                match zaturo_orchestrator::computer_run(
                    &router, &context, goal, &budget, &port, cancel,
                )
                .await
                {
                    Ok(out) => {
                        result.outcome = out.outcome;
                        result.summary = out.summary;
                        result.data = serde_json::json!({"verified_actions":out.actions,"model_calls":out.model_calls,"target_worker":match request{ServiceRequest::ComputerTask{target_worker,..}=>target_worker,_=>unreachable!()}})
                    }
                    Err(zaturo_orchestrator::Error::Model(error)) => {
                        let (code, text) = crate::assistant::failure(&error);
                        result.summary = text.into();
                        result.data = serde_json::json!({"code":code});
                    }
                    Err(_) => {
                        result.summary="A tarefa parou sem comprovar a conclusão. Uma etapa pode exigir confirmação ou reconciliação; não repetirei ações incertas.".into();
                        result.data = serde_json::json!({"code":"computer_stopped"});
                    }
                }
            } else {
                result.summary="O planejamento de computador não tem um perfil pessoal Go autorizado no worker da IA. Verifique o perfil local do computador da IA.".into();
            }
        }
        ServiceRequest::ComputerStep {
            action,
            owner_approved,
            ..
        } => {
            let computer = config.computer.as_ref().ok_or(FailureCode::PolicyDenied)?;
            let remote = RemoteComputer::new(config, work, token)
                .map_err(|_| FailureCode::ExecutionFailed)?;
            let lease: ComputerLease = RemoteComputer::read(
                remote
                    .client
                    .get(format!(
                        "{}/v1/device/computer-actions/{}/grant",
                        config.hub_url, work.task.id
                    ))
                    .send()
                    .await
                    .map_err(|_| FailureCode::ExecutionFailed)?,
            )
            .await
            .map_err(|_| FailureCode::PolicyDenied)?;
            if lease.operation_id != operation
                || lease.lease_token != work.lease_token
                || lease.owner_approved != *owner_approved
            {
                return Err(FailureCode::PolicyDenied);
            }
            let observation=zaturo_computer::execute(computer,action,&lease).await.unwrap_or_else(|_|ComputerObservation{outcome:if action.requires_approval(){"uncertain"}else{"blocked"}.into(),text:"O adaptador não confirmou a etapa. Verifique o computador; nenhuma ação incerta será repetida automaticamente.".into(),data:serde_json::json!({"code":"adapter_not_confirmed"})});
            result.outcome = if observation.outcome == "completed" {
                "completed"
            } else if observation.outcome == "partial" {
                "partial"
            } else {
                "blocked"
            }
            .into();
            result.summary = observation.text.chars().take(180).collect();
            result.data = serde_json::json!({"observation":observation});
        }
        ServiceRequest::PriceSearch { query } => {
            let computer = config.computer.as_ref().ok_or(FailureCode::PolicyDenied)?;
            let action = ComputerAction {
                backend: zaturo_core::ComputerBackend::Bitlupa,
                tool: "search".into(),
                arguments: serde_json::json!({"query":query}),
                target: serde_json::json!({}),
                expected_snapshot_sha256: None,
            };
            let lease = ComputerLease {
                operation_id: operation.clone(),
                lease_token: work.lease_token.clone(),
                lease_expires_at: work.lease_expires_at,
                deadline_at: work.task.deadline_at,
                owner_approved: false,
            };
            let observation = zaturo_computer::execute(computer, &action, &lease)
                .await
                .map_err(|_| FailureCode::ExecutionFailed)?;
            result.summary = observation.text;
            result.outcome = if observation.outcome == "completed" {
                "completed"
            } else if observation.outcome == "partial" {
                "partial"
            } else {
                "blocked"
            }
            .into();
            result.data = observation.data;
        }
        _ => return Err(FailureCode::PolicyDenied),
    }
    if !result.validates(&operation, request) {
        return Err(FailureCode::ModelRejected);
    }
    let local = journal;
    let saved = result.clone();
    tokio::task::spawn_blocking(move || local.record_service(&operation, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?;
    Ok(result)
}
struct RemoteComputer {
    client: reqwest::Client,
    base: String,
    parent: String,
    lease: String,
}
impl RemoteComputer {
    fn new(config: &WorkerConfig, work: &Assignment, token: &str) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))?;
        auth.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth);
        let mut lease = reqwest::header::HeaderValue::from_str(&work.lease_token)?;
        lease.set_sensitive(true);
        headers.insert("x-zaturo-lease", lease);
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .default_headers(headers)
                .timeout(std::time::Duration::from_secs(12))
                .build()?,
            base: config.hub_url.clone(),
            parent: work.task.id.clone(),
            lease: work.lease_token.clone(),
        })
    }
    async fn read<T: serde::de::DeserializeOwned>(
        mut response: reqwest::Response,
    ) -> Result<T, zaturo_orchestrator::Error> {
        if !response.status().is_success() {
            return Err(zaturo_orchestrator::Error::ToolFailed);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?
        {
            if body.len() + chunk.len() > 32768 {
                return Err(zaturo_orchestrator::Error::ToolFailed);
            }
            body.extend_from_slice(&chunk)
        }
        serde_json::from_slice(&body).map_err(|_| zaturo_orchestrator::Error::ToolFailed)
    }
}
impl zaturo_orchestrator::ComputerExecutor for RemoteComputer {
    fn execute<'a>(
        &'a self,
        sequence: u32,
        action: ComputerAction,
    ) -> zaturo_orchestrator::FutureResult<'a, ComputerObservation> {
        Box::pin(async move {
            let _ = &self.lease; // Kept private; never expose the active lease in a URL or model observation.
            let step: ComputerStep = Self::read(
                self.client
                    .post(format!(
                        "{}/v1/device/computer/{}/steps",
                        self.base, self.parent
                    ))
                    .json(&ComputerStepRequest { sequence, action })
                    .send()
                    .await
                    .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?,
            )
            .await?;
            loop {
                let status: ComputerStepStatus = Self::read(
                    self.client
                        .get(format!(
                            "{}/v1/device/computer/{}/steps/{}",
                            self.base, self.parent, step.id
                        ))
                        .send()
                        .await
                        .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?,
                )
                .await?;
                if let Some(observation) = status.observation {
                    return Ok(observation);
                }
                tokio::time::sleep(std::time::Duration::from_millis(700)).await;
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};
    use tokio::task::JoinHandle;
    use zaturo_core::{ChatAction, ChatReply, ChatRequest, FeedPage, NewTask, TaskKind, TaskState};
    use zaturo_hub::{AppState, config::HubConfig};
    use zaturo_llm::{
        ModelResponse, ModelsConfig, Router,
        fake::{FakeBudget, FakeClient, FakeReply},
    };

    const OWNER: &str = "synthetic-owner-for-computer-broker-test";
    const HOME: &str = "synthetic-home-for-computer-broker-test";
    const WORK: &str = "synthetic-work-for-computer-broker-test";
    const APP: &str = "synthetic-native-app-for-computer-broker";

    struct Running(Vec<JoinHandle<()>>);
    impl Drop for Running {
        fn drop(&mut self) {
            for task in &self.0 {
                task.abort();
            }
        }
    }
    fn private_file(path: &Path, value: impl AsRef<[u8]>) {
        fs::write(path, value).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn answer(proposal: Value) -> FakeReply {
        FakeReply::Answer(ModelResponse {
            billing: Some(zaturo_llm::ModelBilling {
                source: zaturo_llm::BillingSource::Go,
                allowance_microusd: Some(0),
            }),
            model: zaturo_llm::DEEPSEEK_MODEL.into(),
            content: proposal.to_string(),
            usage: zaturo_core::ModelUsage {
                input_tokens: 20,
                output_tokens: 20,
                cost_microusd: Some(0),
            },
            finish_reason: "stop".into(),
            duration_ms: 1,
            model_calls: 1,
            usage_complete: true,
        })
    }

    /// Real authenticated broker routes, worker WebSocket, grant fetch and independent journal.
    /// The fixed private helper and model replies are synthetic; no live UI or account is used.
    #[tokio::test]
    async fn brokered_computer_steps_require_native_owner_approval_and_observed_verification() {
        tokio::time::timeout(Duration::from_secs(15), async {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path();
            fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
            let mut devices = String::new();
            for (id, token) in [("local-worker", HOME), ("work-pc", WORK)] {
                let recipient = zaturo_transfer::crypto::generate_identity(
                    &root.join(format!("{id}.identity")),
                )
                .unwrap();
                devices += &format!(
                    "[[devices]]\nid={id:?}\ntoken_sha256='{:x}'\nencryption_recipient={recipient:?}\n",
                    Sha256::digest(token)
                );
            }
            private_file(&root.join("devices.toml"), devices);
            private_file(
                &root.join("apps.toml"),
                format!(
                    "[[clients]]\nid='native-owner-app'\ntoken_sha256='{:x}'\ncan_submit=true\nviewer_device='local-worker'\n",
                    Sha256::digest(APP)
                ),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let base = format!("http://{address}");
            let hub = AppState::new(
                HubConfig {
                    bind: address,
                    client_url: base.clone(),
                    database: root.join("hub.db"),
                    worker_id: "local-worker".into(),
                    max_connections: 3,
                    max_queued_tasks: 8,
                    max_page_size: 100,
                    lease_seconds: 30,
                    max_workers: 2,
                    devices_file: Some(root.join("devices.toml")),
                    apps_file: Some(root.join("apps.toml")),
                    transfer_root: Some(root.join("spool")),
                    transfer_limits: Default::default(),
                },
                OWNER,
                HOME,
            )
            .unwrap();
            let router = zaturo_hub::router(hub.clone());
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let mut running = Running(vec![server]);
            let scripts = root.join("helpers");
            fs::create_dir(&scripts).unwrap();
            fs::set_permissions(&scripts, fs::Permissions::from_mode(0o700)).unwrap();
            let profile = root.join("linux.toml");
            private_file(&profile, "enabled=true\n");
            let trace = root.join("trace.jsonl");
            private_file(&trace, "");
            // Assert transport authority while recording only nonsecret synthetic metadata.
            private_file(
                &scripts.join("linux_mcp.py"),
                r#"import json, os, pathlib, stat, sys, time
assert sys.argv[1] == '--config' and sys.argv[3:5] == ['action', '--request-json']
profile, request_path = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[5])
assert stat.S_IMODE(request_path.stat().st_mode) == 0o600
assert not any(k.startswith(('ZATURO_', 'OPENCODE_')) for k in os.environ)
r = json.loads(request_path.read_text())
assert r['lease_token'] and r['operation_id'] and r['approved_plan_sha256']
assert min(r['lease_expires_at'], r['deadline_at']) > int(time.time() * 1000)
target = {'window_id': 7, 'pid': 777, 'app_id': 'synthetic-editor'}
assert r['target'] == target
marker, trace = profile.parent / 'synthetic-clicked', profile.parent / 'trace.jsonl'
if r['tool'] == 'click':
    assert r['owner_confirmed'] is True
    assert r['arguments'] == {'element_index': 1}
    assert r['expected_snapshot_sha256'] == 'a' * 64
    marker.write_text('synthetic input only')
    os.chmod(marker, 0o600)
    data = {'dispatched': True, 'verified_external_effect': False, 'target': target}
    result = {'schema': 1, 'status': 'completed', 'tool': 'click', 'data': data}
else:
    assert r['tool'] == 'get_app_state' and r['owner_confirmed'] is False
    label = 'Ação sintética concluída' if marker.exists() else 'Confirmar exemplo sintético'
    node = {'index': 1, 'role': 'button', 'name': label, 'text': '', 'actions': ['click'],
            'states': ['enabled'], 'editable': False, 'protected': False}
    data = {'target': target, 'nodes': [node], 'total_nodes': 1, 'offset': 0,
            'next_offset': None, 'truncated': False}
    result = {'schema': 1, 'status': 'completed', 'tool': 'get_app_state', 'data': data,
              'snapshot_sha256': ('b' if marker.exists() else 'a') * 64}
with trace.open('a') as f:
    f.write(json.dumps({'tool': r['tool'], 'operation_id': r['operation_id'],
                        'owner_confirmed': r['owner_confirmed'], 'target': target}) + '\n')
print(json.dumps(result))
"#,
            );
            let config = WorkerConfig {
                hub_url: base.clone(),
                worker_id: "work-pc".into(),
                database: root.join("work.db"),
                workspace_root: root.join("workspaces"),
                artifact_root: root.join("artifacts"),
                capabilities: vec!["diagnostic.v1".into(), "computer.action.v1".into()],
                max_concurrent_tasks: 1,
                heartbeat_seconds: 1,
                reconnect_max_seconds: 2,
                models_config: None,
                assistant_models_config: None,
                computer_models_config: None,
                opencode: None,
                computer: Some(zaturo_computer::ComputerConfig {
                    python: "/usr/bin/python3".into(),
                    scripts_root: scripts,
                    linux_profile: Some(profile),
                    browser_profile: None,
                    bitlupa_project: None,
                    bitlupa_database: None,
                    social_profile: None,
                    capture_target: None,
                    wayland_portal_capture: false,
                }),
                files: None,
                youtube: None,
                android: None,
            };
            let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
            let local = journal.clone();
            running.0.push(tokio::spawn(async move {
                crate::connect_once(&config, WORK, local).await.unwrap()
            }));
            while !hub.connected.lock().unwrap().contains("work-pc")
                || !hub.engine.workers(100, 0).unwrap().iter().any(|w| w.id == "work-pc")
            {
                assert!(!running.0[1].is_finished(), "synthetic executor disconnected");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            hub.engine
                .register(
                    "local-worker",
                    &["diagnostic.v1".into(), "computer.agent.v1".into()],
                    now_ms(),
                )
                .unwrap();
            let goal = "Confirme o botão sintético autorizado e verifique o novo estado.";
            let mut new = NewTask::diagnostic("local-worker", "synthetic-computer-broker");
            new.kind = TaskKind::Service;
            new.max_attempts = 1;
            new.timeout_seconds = 15;
            new.request = goal.into();
            new.service = Some(ServiceRequest::ComputerTask {
                request: goal.into(),
                target_worker: "work-pc".into(),
            });
            let parent = hub.engine.create(new, now_ms()).unwrap();
            let mut assignment = hub.engine.claim("local-worker", now_ms()).unwrap().unwrap();
            assert_eq!(assignment.task.id, parent.id);
            assignment.task = hub
                .engine
                .start(&parent.id, "local-worker", &assignment.lease_token, now_ms())
                .unwrap();
            let mut home: WorkerConfig =
                toml::from_str(include_str!("../../../config/worker.example.toml")).unwrap();
            home.hub_url = base.clone();
            let remote = RemoteComputer::new(&home, &assignment, HOME).unwrap();
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap();
            let target = json!({"window_id":7,"pid":777,"app_id":"synthetic-editor"});
            let read = json!({"type":"action","action":{"backend":"desktop","tool":"get_app_state","arguments":{},"target":target}});
            let click = json!({"type":"action","action":{"backend":"desktop","tool":"click","arguments":{"element_index":1},"target":target,"expected_snapshot_sha256":"a".repeat(64)}});
            let fake = Arc::new(FakeClient::new([
                answer(read.clone()),
                answer(click),
                answer(read),
                answer(json!({"type":"finish","summary":"O novo estado do botão sintético foi observado."})),
            ]));
            let mut models: ModelsConfig =
                toml::from_str(include_str!("../../../config/computer-models.example.toml")).unwrap();
            models.enabled = true;
            models.allow_global_processing = true;
            let router = Router::new(models, fake.clone()).unwrap();
            let context = CallContext {
                task_id: parent.id.clone(),
                operation_id: parent.operation_id.clone(),
                step: 1,
                deadline_at: parent.deadline_at,
                privacy: PrivacyClass::Personal,
            };
            let budget = FakeBudget::default();
            let cancel = Cancellation::new();
            let planner = zaturo_orchestrator::computer_run(
                &router, &context, goal, &budget, &remote, &cancel,
            );
            let approve = async {
                let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let approval = loop {
                    let feed: FeedPage = client
                        .get(format!("{base}/v1/feed"))
                        .query(&[("day", day.as_str()), ("offset_minutes", "0"), ("limit", "100")])
                        .bearer_auth(APP)
                        .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
                    if let Some(item) = feed.items.into_iter().find(|i| {
                        i.task_id.as_deref() == Some(parent.id.as_str())
                            && i.text.contains("Ação aguardando sua confirmação")
                    }) {
                        assert!(item.text.contains("Confirmar exemplo sintético"));
                        break item.text.split("/aprovar ").nth(1).unwrap()
                            .split_whitespace().next().unwrap().to_owned();
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                };
                let before = fs::read_to_string(&trace).unwrap();
                assert_eq!(before.lines().count(), 1);
                assert!(!before.contains("click"), "input ran before native owner approval");
                // Old MCP bot_chat fixes action=assistant, even with the same app token.
                let inert: ChatReply = client
                    .post(format!("{base}/v1/chat"))
                    .bearer_auth(APP)
                    .json(&ChatRequest {
                        message: format!("/aprovar {approval}"),
                        idempotency_key: format!("mcp-{approval}"),
                        target_worker: Some("work-pc".into()),
                        action: Some(ChatAction::Assistant),
                    })
                    .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
                assert!(inert.task.is_none());
                assert!(inert.message.text.contains("interface nativa"));
                let waiting: ComputerStepStatus = client
                    .get(format!("{base}/v1/device/computer/{}/steps/{approval}", parent.id))
                    .bearer_auth(HOME)
                    .header("x-zaturo-lease", &assignment.lease_token)
                    .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
                assert_eq!(waiting.step.state, "waiting_approval");
                assert!(waiting.step.child_id.is_none() && waiting.observation.is_none());
                assert_eq!(fs::read_to_string(&trace).unwrap(), before);
                let reply: ChatReply = client
                    .post(format!("{base}/v1/chat"))
                    .bearer_auth(APP)
                    .json(&ChatRequest {
                        message: format!("/aprovar {approval}"),
                        idempotency_key: format!("approve-{approval}"),
                        target_worker: Some("work-pc".into()),
                        action: Some(ChatAction::ComputerConfirm),
                    })
                    .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
                assert!(reply.task.is_none());
                assert!(reply.message.text.contains("autorizada"));
            };
            let (outcome, ()) = tokio::join!(planner, approve);
            let outcome = outcome.unwrap();
            assert_eq!(outcome.outcome, "completed");
            assert_eq!(outcome.actions, 3);
            assert_eq!(outcome.model_calls, 4);
            assert_eq!(fake.calls(), 4);
            let trace_text = fs::read_to_string(&trace).unwrap();
            let calls: Vec<Value> = trace_text.lines()
                .map(|line| serde_json::from_str(line).unwrap()).collect();
            assert_eq!(calls.iter().map(|v| v["tool"].as_str().unwrap()).collect::<Vec<_>>(),
                ["get_app_state", "click", "get_app_state"]);
            assert_eq!(calls.iter().map(|v| v["owner_confirmed"].as_bool().unwrap()).collect::<Vec<_>>(),
                [false, true, false]);
            assert!(!trace_text.contains(&assignment.lease_token));
            assert!(!trace_text.contains(HOME) && !trace_text.contains(WORK));
            let children = hub.engine.list(100, 0).unwrap().into_iter().filter(|t| {
                matches!(&t.service, Some(ServiceRequest::ComputerStep { parent_id, .. }) if parent_id == &parent.id)
            }).collect::<Vec<_>>();
            assert_eq!(children.len(), 3);
            for child in children {
                assert_eq!(child.target_worker, "work-pc");
                assert_eq!(child.assigned_worker.as_deref(), Some("work-pc"));
                assert_eq!(child.state, TaskState::Completed);
                let result = child.service_result.as_ref().unwrap();
                assert_eq!(journal.begin_service(&child.operation_id, &child.id,
                    child.service.as_ref().unwrap(), now_ms()).unwrap(),
                    ServiceIntent::Cached(result.clone()));
                assert_eq!(client.get(format!("{base}/v1/device/computer-actions/{}/grant", child.id))
                    .bearer_auth(APP).send().await.unwrap().status(), 401);
            }
        })
        .await
        .expect("real local broker fixture exceeded its 15-second deadline");
    }
}
