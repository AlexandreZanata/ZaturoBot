//! Launch only a hash-pinned installed program; monitor an owned unit and exact broker receipt.
use super::{
    OpenCodeConfig,
    gateway::{Gate, ProcessProof, Progress},
    state,
};
use anyhow::{Context, Result, ensure};
use axum::{Router, extract::DefaultBodyLimit, routing::post};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::Path,
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};
use zaturo_core::{OpenCodeEvidence, OpenCodeWorkOrder, ServiceRequest, ServiceResult};
use zaturo_llm::{BudgetPort, Cancellation};
use zaturo_youtube::YoutubeClient;

static DISPATCH: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

fn blocked(order: &OpenCodeWorkOrder, code: &str, summary: &str) -> ServiceResult {
    ServiceResult {
        operation_id: order.operation_id.clone(),
        action: if order.interpret() {
            "assistant_route"
        } else {
            "youtube_opencode_generate"
        }
        .into(),
        outcome: "blocked".into(),
        summary: summary.into(),
        data: json!({"code":code}),
    }
}
fn wrapped(
    order: &OpenCodeWorkOrder,
    proof: OpenCodeEvidence,
    result: ServiceResult,
    observed: bool,
) -> Result<ServiceResult> {
    let (channel, count) = order
        .generation()
        .context("generation work order required")?;
    ensure!(
        proof.validates(&order.operation_id, channel, count),
        "invalid execution proof"
    );
    let request = ServiceRequest::YoutubeGenerate {
        channel: channel.into(),
        count,
    };
    ensure!(
        result.validates(&order.operation_id, &request),
        "invalid generation receipt"
    );
    let data = &result.data;
    let output = ServiceResult {
        operation_id: order.operation_id.clone(),
        action: "youtube_opencode_generate".into(),
        outcome: if matches!(result.outcome.as_str(), "accepted" | "completed") {
            "accepted".into()
        } else {
            result.outcome.clone()
        },
        summary: if result.outcome == "blocked" {
            result.summary
        } else if result.summary.is_empty() {
            "A operação exige observação do mesmo UUID.".into()
        } else if !observed {
            "A geração possui recibo do serviço, mas a confirmação de entrega do resultado ao OpenCode não foi observada. Acompanharei o mesmo UUID, sem nova chamada de IA nem outro lote.".into()
        } else {
            "O OpenCode acionou a geração aprovada. A verificação dos arquivos e da decodificação determinará a conclusão; nenhum agendamento foi autorizado.".into()
        },
        data: json!({"opencode":proof,"opencode_result_observed":observed,"service_code":data.get("code"),"mode":"generate","channel":channel,"count":count,
            "generated_count":data.get("generated_count"),"scheduled_count":data.get("scheduled_count"),"failed_count":data.get("failed_count"),
            "run_ids":data.get("runs").and_then(Value::as_array).map(|rows|rows.iter().filter_map(|v|v.get("id").cloned()).collect::<Vec<_>>())}),
    };
    ensure!(
        output.validates(
            &order.operation_id,
            &ServiceRequest::YoutubeOpenCodeGenerate {
                channel: channel.into(),
                count
            }
        ),
        "bounded installed executor receipt required"
    );
    Ok(output)
}

pub async fn reconcile(
    config: &OpenCodeConfig,
    order: &OpenCodeWorkOrder,
    youtube: Option<&YoutubeClient>,
) -> Result<ServiceResult> {
    let directory = state::directory(&config.state_root, &order.operation_id)?;
    let intent: state::Intent =
        serde_json::from_slice(&super::private_file(&directory.join("intent.json"), 16384)?)?;
    ensure!(
        intent.order == *order && intent.executable_sha256 == config.executable_sha256,
        "operation intent differs"
    );
    let receipt: state::Receipt = serde_json::from_slice(&super::private_file(
        &directory.join("receipt.json"),
        16384,
    )?)?;
    if order.interpret() {
        if let Some(result) = receipt.result
            && result.validates(&order.operation_id, &route_request(order)?)
        {
            return Ok(result);
        }
        return Ok(blocked(
            order,
            "opencode_reconciliation_required",
            "A interpretação anterior não possui um recibo completo. A chamada de IA não será repetida.",
        ));
    }
    let (channel, count) = order
        .generation()
        .context("generation work order required")?;
    let youtube = youtube.context("generation adapter required")?;
    if let Some(proof) = receipt.proof {
        let result = match youtube
            .reconcile(
                &order.operation_id,
                &ServiceRequest::YoutubeGenerate {
                    channel: channel.into(),
                    count,
                },
            )
            .await
        {
            Ok(value) => value,
            Err(_) => {
                return Ok(blocked(
                    order,
                    "opencode_reconciliation_required",
                    "A operação já possui intenção durável, mas o estado da geração ainda não pôde ser confirmado. O mesmo UUID foi preservado; nenhum lote foi reenviado.",
                ));
            }
        };
        return wrapped(
            order,
            proof,
            result,
            receipt.code.as_deref() == Some("installed_tool_result_observed"),
        );
    }
    Ok(blocked(
        order,
        "opencode_reconciliation_required",
        "O OpenCode já recebeu uma intenção desta operação. Sem recibo de execução verificável, não iniciarei outra sessão nem outro lote. É necessária reconciliação da mesma operação.",
    ))
}

pub async fn dispatch(
    config: OpenCodeConfig,
    order: OpenCodeWorkOrder,
    youtube: Option<YoutubeClient>,
    budget: Arc<dyn BudgetPort>,
    cancel: Cancellation,
) -> Result<ServiceResult> {
    let _dispatch = DISPATCH
        .try_acquire()
        .context("another installed dispatch is active")?;
    config.validate()?;
    ensure!(
        order.validates(super::now_ms()),
        "invalid or expired approved work order"
    );
    let models = config.models()?;
    let directory = state::directory(&config.state_root, &order.operation_id)?;
    if directory.join("intent.json").exists() {
        return reconcile(&config, &order, youtube.as_ref()).await;
    }
    let binary = config.executable.clone();
    let expected = config.executable_sha256.clone();
    tokio::task::spawn_blocking(move || verify_binary(&binary, &expected)).await??;
    let version =
        bounded_program(&config.executable, &["--version".into()], &directory, 4096).await?;
    ensure!(
        String::from_utf8(version)?.trim() == format!("opencode v{}", super::VERSION),
        "installed OpenCode version differs"
    );
    let session = format!("ses_{}", order.operation_id.replace('-', ""));
    state::create_intent(
        &directory,
        &state::Intent {
            order: order.clone(),
            session_id: session.clone(),
            executable_sha256: config.executable_sha256.clone(),
            created_at: super::now_ms(),
        },
    )?;
    state::receipt(
        &directory,
        &state::Receipt {
            phase: "prepared".into(),
            updated_at: super::now_ms(),
            proof: None,
            result: None,
            code: None,
        },
    )?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let nonce =
        uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(models.timeout_seconds))
        .build()?;
    let gate = Arc::new(Gate {
        order: order.clone(),
        models,
        budget,
        nonce: nonce.clone(),
        youtube,
        directory: directory.clone(),
        progress: tokio::sync::Mutex::new(Progress::default()),
        changed: tokio::sync::Notify::new(),
        http,
        #[cfg(test)]
        test_endpoint: None,
    });
    let application = Router::new()
        .route("/v1/chat/completions", post(super::gateway::completion))
        .route("/mcp", post(super::broker::mcp))
        .layer(DefaultBodyLimit::max(32768))
        .layer(tower::limit::ConcurrencyLimitLayer::new(4))
        .with_state(gate.clone());
    let server = ServerGuard(tokio::spawn(async move {
        axum::serve(listener, application).await
    }));
    let opencode_directory = directory.join(".opencode");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&opencode_directory)?;
    state::atomic(
        &opencode_directory.join("opencode.json"),
        &serde_json::to_vec(&state::job_configuration(&order, &origin, &nonce))?,
    )?;
    let unit = format!("zaturo-opencode-{}.service", order.operation_id);
    let environment = clean_environment()?;
    let mut arguments = vec![
        "--user".into(),
        "--quiet".into(),
        "--wait".into(),
        "--collect".into(),
        "--pipe".into(),
        format!("--unit={unit}"),
        format!("--working-directory={}", directory.display()),
        "--property=CPUQuota=100%".into(),
        "--property=MemoryHigh=512M".into(),
        "--property=MemoryMax=2G".into(),
        "--property=TasksMax=128".into(),
        format!("--property=RuntimeMaxSec={}", order.runtime_seconds),
        "--property=KillMode=control-group".into(),
        "--property=TimeoutStopSec=5".into(),
        "--property=Restart=no".into(),
        "--property=UMask=0077".into(),
        "--property=NoNewPrivileges=yes".into(),
        "--".into(),
        "/usr/bin/env".into(),
        "-i".into(),
    ];
    arguments.extend(environment.iter().map(|(k, v)| format!("{k}={v}")));
    arguments.extend([
        config.executable.to_string_lossy().into_owned(),
        "--standalone".into(),
        "run".into(),
        "--format".into(),
        "json".into(),
        "--model".into(),
        super::MODEL.into(),
        "--agent".into(),
        "zaturo-job".into(),
        "--session".into(),
        session.clone(),
        "--title".into(),
        format!("Zaturo generation {}", order.operation_id),
        "--archived".into(),
        json!({"approved_work_order":order}).to_string(),
    ]);
    state::receipt(
        &directory,
        &state::Receipt {
            phase: "launching".into(),
            updated_at: super::now_ms(),
            proof: None,
            result: None,
            code: None,
        },
    )?;
    let mut child = Command::new("/usr/bin/systemd-run")
        .args(arguments)
        .env_clear()
        .envs(environment.clone())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let identity = process_proof(&unit, &config.executable).await;
    let (pid, start) = match identity {
        Ok(value) => value,
        Err(error) => {
            server.0.abort();
            return Err(error);
        }
    };
    state::atomic(
        &directory.join("process.json"),
        &serde_json::to_vec(
            &json!({"pid":pid,"start":start,"session":session,"unit":unit,"sha256":config.executable_sha256}),
        )?,
    )?;
    let verified_start = start.clone();
    gate.progress.lock().await.process = Some(ProcessProof {
        pid,
        start,
        session,
        sha256: config.executable_sha256.clone(),
        unit: unit.clone(),
    });
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("OpenCode stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("OpenCode stderr unavailable"))?;
    let reader_gate = gate.clone();
    let output_limit = order.max_output_bytes;
    let reader = tokio::spawn(async move { drain(stdout, reader_gate, output_limit, true).await });
    let reader_gate = gate.clone();
    let errors = tokio::spawn(async move { drain(stderr, reader_gate, output_limit, false).await });
    let deadline = tokio::time::Instant::now()
        + Duration::from_millis((order.deadline_at - super::now_ms()).max(0) as u64)
            .min(Duration::from_secs(order.runtime_seconds));
    let mut cancelled = false;
    loop {
        tokio::select! {biased;
            _=cancel.cancelled()=>{cancelled=true;break;},
            _=tokio::time::sleep_until(deadline)=>break,
            _=gate.changed.notified()=>{},
            _=tokio::time::sleep(Duration::from_millis(100))=>{},
        }
        {
            let state = gate.progress.lock().await;
            if state.failure.is_some() || state.acknowledged {
                break;
            }
        }
        if child.try_wait()?.is_some() {
            break;
        }
    }
    // Existing keep-alive connections cannot admit a tool after this dispatch closes.
    gate.progress
        .lock()
        .await
        .failure
        .get_or_insert(if cancelled {
            "owner_cancelled"
        } else {
            "dispatch_closed"
        });
    // This stops the owned dispatch unit, never the detached media actor or another OpenCode session.
    let dispatch_stopped = stop_unit(&unit, &config.executable, pid, &verified_start).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), reader).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), errors).await;
    server.0.abort();
    let state = gate.progress.lock().await;
    if state.forwarded && state.billing.is_none() {
        let _ = gate
            .budget
            .uncertain(&format!("{}:1:1", order.operation_id))
            .await;
    }
    if cancelled {
        let tool_started = state.tool_started;
        drop(state);
        if tool_started && let Some(youtube) = gate.youtube.as_ref() {
            let stopped = super::cancel_owned(&config, &order, youtube).await;
            return Ok(blocked(
                &order,
                if stopped.is_ok() && dispatch_stopped {
                    "owned_media_actor_stopped"
                } else {
                    "owned_operation_reconciliation_required"
                },
                if stopped.is_ok() && dispatch_stopped {
                    "O processo próprio do OpenCode e o ator de geração foram interrompidos, com identidade e unidade verificadas. As cercas e os arquivos foram preservados; a operação permanece em reconciliação."
                } else if stopped.is_ok() {
                    "O ator de geração foi interrompido, mas a parada do processo próprio do OpenCode não foi comprovada. O mesmo UUID e os recibos foram preservados para reconciliação."
                } else {
                    "A interrupção completa dos processos próprios não foi comprovada. O mesmo UUID foi preservado; a geração exige reconciliação."
                },
            ));
        }
        return Ok(blocked(
            &order,
            if dispatch_stopped {
                "owned_dispatch_stopped"
            } else {
                "owned_dispatch_stop_unconfirmed"
            },
            if dispatch_stopped {
                "O despacho próprio foi interrompido antes de chamar a ferramenta. Nenhum lote foi reenviado."
            } else {
                "A ferramenta de geração não foi chamada, mas a parada do processo próprio do OpenCode não foi comprovada. A operação foi preservada para reconciliação."
            },
        ));
    }
    let acknowledged = state.acknowledged;
    let code = state.failure;
    drop(state);
    let receipt_path = directory.join("receipt.json");
    let mut receipt: state::Receipt =
        serde_json::from_slice(&super::private_file(&receipt_path, 16384)?)?;
    if acknowledged
        && order.interpret()
        && let Some(result) = receipt.result.clone()
    {
        ensure!(
            result.validates(&order.operation_id, &route_request(&order)?),
            "invalid interpretation receipt"
        );
        receipt.phase = "dispatched".into();
        receipt.updated_at = super::now_ms();
        receipt.code = Some("installed_tool_result_observed".into());
        state::receipt(&directory, &receipt)?;
        return Ok(result);
    }
    if acknowledged
        && let (Some(proof), Some(result)) = (receipt.proof.clone(), receipt.result.clone())
    {
        receipt.phase = "dispatched".into();
        receipt.updated_at = super::now_ms();
        receipt.code = Some("installed_tool_result_observed".into());
        state::receipt(&directory, &receipt)?;
        return wrapped(&order, proof, result, true);
    }
    if !order.interpret() && receipt.proof.is_some() {
        // A missing CLI acknowledgment cannot orphan a positively admitted media actor.
        // This is one safe status read for the SAME operation, never another generate call.
        if let Ok(result) = reconcile(&config, &order, gate.youtube.as_ref()).await
            && result.outcome == "accepted"
        {
            return Ok(result);
        }
    }
    receipt.phase = "uncertain".into();
    receipt.code = Some(code.unwrap_or("opencode_receipt_unconfirmed").into());
    receipt.updated_at = super::now_ms();
    state::receipt(&directory, &receipt)?;
    Ok(blocked(
        &order,
        code.unwrap_or("opencode_receipt_unconfirmed"),
        "O despacho do OpenCode não produziu todos os recibos exigidos. A operação foi preservada para reconciliação, sem nova chamada de IA nem outro lote.",
    ))
}

fn verify_binary(path: &Path, expected: &str) -> Result<()> {
    ensure!(
        path.is_absolute() && !path.is_symlink(),
        "trusted installed OpenCode executable required"
    );
    let before = std::fs::metadata(path)?;
    ensure!(
        before.is_file()
            && before.permissions().mode() & 0o022 == 0
            && before.permissions().mode() & 0o111 != 0,
        "reviewed executable permissions required"
    );
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut block = [0_u8; 65536];
    loop {
        let count = file.read(&mut block)?;
        if count == 0 {
            break;
        }
        digest.update(&block[..count]);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == expected,
        "installed OpenCode executable hash differs"
    );
    let after = std::fs::metadata(path)?;
    ensure!(
        (before.dev(), before.ino(), before.len(), before.mtime())
            == (after.dev(), after.ino(), after.len(), after.mtime()),
        "installed executable changed"
    );
    Ok(())
}
fn clean_environment() -> Result<Vec<(String, String)>> {
    let home = std::env::var("HOME")?;
    ensure!(Path::new(&home).is_absolute(), "owner home required");
    let mut env = vec![
        ("HOME".into(), home),
        ("PATH".into(), "/usr/local/bin:/usr/bin:/bin".into()),
        ("LANG".into(), "C.UTF-8".into()),
    ];
    for key in [
        "USER",
        "LOGNAME",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
    ] {
        if let Ok(value) = std::env::var(key) {
            env.push((key.into(), value));
        }
    }
    Ok(env)
}
async fn bounded_program(
    executable: &Path,
    args: &[String],
    cwd: &Path,
    limit: usize,
) -> Result<Vec<u8>> {
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(clean_environment()?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut output = Vec::new();
    let mut stream = child.stdout.take().context("program output unavailable")?;
    tokio::time::timeout(Duration::from_secs(10), async {
        (&mut stream)
            .take((limit + 1) as u64)
            .read_to_end(&mut output)
            .await?;
        ensure!(output.len() <= limit, "program output limit");
        ensure!(child.wait().await?.success(), "program rejected request");
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    Ok(output)
}
async fn process_proof(unit: &str, executable: &Path) -> Result<(u32, String)> {
    for _ in 0..50 {
        let bytes = bounded_program(
            Path::new("/usr/bin/systemctl"),
            &[
                "--user".into(),
                "show".into(),
                unit.into(),
                "-p".into(),
                "MainPID".into(),
                "--value".into(),
            ],
            Path::new("/"),
            64,
        )
        .await?;
        if let Ok(pid) = String::from_utf8(bytes)?.trim().parse::<u32>()
            && pid > 1
        {
            let root = std::path::PathBuf::from(format!("/proc/{pid}"));
            if std::fs::read_link(root.join("exe")).ok().as_deref() == Some(executable)
                && std::fs::read_to_string(root.join("cgroup"))?.contains(unit)
            {
                let stat = std::fs::read_to_string(root.join("stat"))?;
                let start = stat
                    .rsplit_once(')')
                    .context("invalid owned process identity")?
                    .1
                    .split_whitespace()
                    .nth(19)
                    .context("owned start identity missing")?
                    .to_owned();
                return Ok((pid, start));
            }
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    anyhow::bail!("owned OpenCode process could not be verified")
}
struct ServerGuard(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn stop_unit(unit: &str, executable: &Path, pid: u32, start: &str) -> bool {
    let root = std::path::PathBuf::from(format!("/proc/{pid}"));
    let identity = std::fs::read_to_string(root.join("stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(')').map(|(_, tail)| {
                tail.split_whitespace()
                    .nth(19)
                    .unwrap_or_default()
                    .to_owned()
            })
        });
    let owned = identity.as_deref() == Some(start)
        && std::fs::read_link(root.join("exe")).ok().as_deref() == Some(executable)
        && std::fs::read_to_string(root.join("cgroup")).is_ok_and(|value| value.contains(unit));
    if owned && unit.starts_with("zaturo-opencode-") && unit.ends_with(".service") {
        return bounded_program(
            Path::new("/usr/bin/systemctl"),
            &["--user".into(), "stop".into(), unit.into()],
            Path::new("/"),
            1024,
        )
        .await
        .is_ok();
    }
    false
}
async fn drain<R: tokio::io::AsyncRead + Unpin>(
    mut input: R,
    gate: Arc<Gate>,
    limit: usize,
    observe: bool,
) -> Result<()> {
    let mut total = 0;
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        let count = input.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        total += count;
        if total > limit {
            gate.progress.lock().await.failure = Some("opencode_output_limit");
            gate.changed.notify_one();
            anyhow::bail!("bounded program output exceeded");
        }
        if !observe {
            continue;
        }
        buffer.extend_from_slice(&chunk[..count]);
        while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
            let line = buffer.drain(..=end).collect::<Vec<_>>();
            if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                let mut state = gate.progress.lock().await;
                if value["type"] == "tool_use"
                    && state
                        .receipt_id
                        .as_ref()
                        .is_some_and(|id| value.to_string().contains(id))
                {
                    state.acknowledged = true;
                    gate.changed.notify_one();
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn route_request(order: &OpenCodeWorkOrder) -> Result<ServiceRequest> {
    let zaturo_core::OpenCodeWork::Interpret {
        context,
        planner_worker,
    } = &order.work
    else {
        anyhow::bail!("interpretation work order required")
    };
    Ok(ServiceRequest::AssistantRoute {
        message: order.owner_request.clone(),
        context: context.clone(),
        target_worker: order.target_worker.clone(),
        planner_worker: planner_worker.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovered_media_completion_is_only_admission_until_full_artifact_inspection() {
        let operation = "00000000-0000-4000-8000-000000000001";
        let order:OpenCodeWorkOrder=serde_json::from_value(json!({"operation_id":operation,"task_id":"00000000-0000-4000-8000-000000000002","target_worker":"work-pc","work":{"mode":"generate","channel":"fallem-prince","count":2},"owner_request":"faça 2 videos para o falllem price","deadline_at":120000,"allowed_operations":["generate","status","inspect_artifacts","cancel_owned"],"max_model_calls":1,"max_tool_calls":1,"max_output_bytes":65536,"runtime_seconds":120})).unwrap();
        let proof:OpenCodeEvidence=serde_json::from_value(json!({"operation_id":operation,"session_id":"ses_synthetic","executable_sha256":"a".repeat(64),"executable_version":"2.3.0","process_id":2,"process_start":"1","unit":format!("zaturo-opencode-{operation}.service"),"target_worker":"work-pc","channel":"fallem-prince","count":2,"mode":"generate","invoked_tool":"youtube_generate","model":"deepseek-v4.1-flash","model_calls":1,"billing_source":"go","usage":{"input_tokens":100,"output_tokens":50,"cost_microusd":0}})).unwrap();
        let service = ServiceResult {
            operation_id: operation.into(),
            action: "youtube_generate".into(),
            outcome: "completed".into(),
            summary: "The media service reports completion.".into(),
            data: json!({"generated_count":2,"scheduled_count":0,"failed_count":0}),
        };
        let result = wrapped(&order, proof, service, false).unwrap();
        assert_eq!(result.outcome, "accepted");
        assert_eq!(result.data["opencode_result_observed"], false);
        assert!(!result.data.to_string().contains("full_decode_verified"));
    }
}
