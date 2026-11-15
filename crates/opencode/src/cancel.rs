//! Stop only a proved generation actor belonging to this executor's durable operation.
use super::{OpenCodeConfig, private_file, state};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{os::unix::fs::MetadataExt, path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};
use zaturo_core::OpenCodeWorkOrder;
use zaturo_youtube::YoutubeClient;

#[derive(Deserialize)]
struct Actor {
    operation_id: String,
    request: Value,
    pid: Option<u32>,
    process_identity: Option<String>,
}
fn identity(
    actor: &Actor,
    order: &OpenCodeWorkOrder,
    youtube: &YoutubeClient,
) -> Result<(u32, String, String)> {
    let (channel, count) = order.generation().context("generation order required")?;
    ensure!(
        actor.operation_id == order.operation_id
            && actor.request["channel"] == channel
            && actor.request["count"] == count
            && actor.request["mode"] == "generate"
            && actor.request["approved_generate"] == true
            && actor.request["approved_schedule"] == false,
        "actor request differs"
    );
    let pid = actor
        .pid
        .filter(|v| *v > 1)
        .context("actor has not published a process identity")?;
    let expected = actor
        .process_identity
        .as_deref()
        .context("actor identity missing")?;
    let root = PathBuf::from(format!("/proc/{pid}"));
    ensure!(
        std::fs::metadata(&root)?.uid() == std::fs::metadata(&youtube.config().project_root)?.uid(),
        "actor owner differs"
    );
    let stat = std::fs::read_to_string(root.join("stat"))?;
    let fields = stat
        .rsplit_once(')')
        .context("invalid actor process")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    ensure!(
        fields.len() > 19 && fields[0] != "Z" && fields[19] == expected,
        "actor process changed"
    );
    ensure!(
        std::fs::read_link(root.join("exe"))? == std::fs::canonicalize(&youtube.config().python)?
            && std::fs::read_link(root.join("cwd"))?
                == std::fs::canonicalize(&youtube.config().project_root)?,
        "actor executable/project differs"
    );
    let command = std::fs::read(root.join("cmdline"))?;
    ensure!(command.len() <= 4096, "actor command too large");
    let argv = command
        .split(|b| *b == 0)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    ensure!(
        argv.len() == 6
            && argv[1] == b"-m"
            && argv[2] == b"tools.ytbot.service"
            && argv[3] == b"_execute"
            && argv[4] == b"--operation-id"
            && argv[5] == order.operation_id.as_bytes(),
        "actor argv differs"
    );
    let unit = format!("youtubebot-{}.service", order.operation_id);
    ensure!(
        std::fs::read_to_string(root.join("cgroup"))?
            .lines()
            .any(|l| l.ends_with(&format!("/{unit}"))),
        "actor cgroup differs"
    );
    Ok((pid, expected.into(), unit))
}
async fn systemctl(args: &[&str]) -> Result<String> {
    let mut command = Command::new("/usr/bin/systemctl");
    command.env_clear().env("PATH", "/usr/bin:/bin");
    for key in ["HOME", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"] {
        if let Some(v) = std::env::var_os(key) {
            command.env(key, v);
        }
    }
    let mut child = command
        .arg("--user")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut bytes = Vec::new();
    let stream = child.stdout.take().context("missing unit output")?;
    tokio::time::timeout(Duration::from_secs(20), async {
        stream.take(4097).read_to_end(&mut bytes).await?;
        ensure!(
            bytes.len() <= 4096 && child.wait().await?.success(),
            "unit operation failed"
        );
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    Ok(String::from_utf8(bytes)?)
}
/// This never deletes or rewrites YoutubeBot admission/execution markers or media.
/// A stopped actor remains fenced and is reconciled by the service as uncertain.
pub async fn cancel_owned(
    config: &OpenCodeConfig,
    order: &OpenCodeWorkOrder,
    youtube: &YoutubeClient,
) -> Result<Value> {
    let directory = state::directory(&config.state_root, &order.operation_id)?;
    let intent: state::Intent =
        serde_json::from_slice(&private_file(&directory.join("intent.json"), 16384)?)?;
    ensure!(
        intent.order == *order && intent.executable_sha256 == config.executable_sha256,
        "not this executor's operation"
    );
    let receipt: state::Receipt =
        serde_json::from_slice(&private_file(&directory.join("receipt.json"), 16384)?)?;
    let (channel, count) = order.generation().context("generation required")?;
    ensure!(
        receipt
            .proof
            .is_some_and(|p| p.validates(&order.operation_id, channel, count)),
        "no owned media dispatch proof"
    );
    let record = youtube
        .config()
        .project_root
        .join(".local/ytbot-service/jobs")
        .join(format!("{}.json", order.operation_id));
    let actor: Actor = serde_json::from_slice(&private_file(&record, 65536)?)?;
    let (pid, start, unit) = identity(&actor, order, youtube)?;
    let main = systemctl(&["show", &unit, "-p", "MainPID", "--value"]).await?;
    ensure!(
        main.trim().parse::<u32>()? == pid,
        "unit main actor differs"
    );
    // Recheck immediately before the sole bounded mutation.
    ensure!(
        identity(&actor, order, youtube)? == (pid, start.clone(), unit.clone()),
        "actor identity changed before stop"
    );
    state::atomic(
        &directory.join("cancel-intent.json"),
        &serde_json::to_vec(
            &serde_json::json!({"operation_id":order.operation_id,"pid":pid,"process_start":start,"unit":unit,"created_at":super::now_ms()}),
        )?,
    )?;
    systemctl(&["stop", &unit]).await?;
    let active = systemctl(&["show", &unit, "-p", "ActiveState", "--value"]).await?;
    ensure!(
        matches!(active.trim(), "inactive" | "failed"),
        "media unit still active"
    );
    ensure!(
        identity(&actor, order, youtube).is_err(),
        "media actor still active"
    );
    let result = serde_json::json!({"operation_id":order.operation_id,"media_actor_stopped":true,"unit":unit,"fences_preserved":true,"reconciliation_required":true,"verified_at":super::now_ms()});
    state::atomic(
        &directory.join("cancelled.json"),
        &serde_json::to_vec(&result)?,
    )?;
    Ok(result)
}
