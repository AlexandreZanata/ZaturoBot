//! Private durable intent/receipts, never provider credentials or raw model output.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use zaturo_core::{OpenCodeEvidence, OpenCodeWorkOrder, ServiceResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Intent {
    pub order: OpenCodeWorkOrder,
    pub session_id: String,
    pub executable_sha256: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    pub phase: String,
    pub updated_at: i64,
    pub proof: Option<OpenCodeEvidence>,
    pub result: Option<ServiceResult>,
    pub code: Option<String>,
}

pub(crate) fn directory(root: &Path, operation: &str) -> Result<PathBuf> {
    ensure!(
        uuid::Uuid::parse_str(operation).is_ok_and(|u| u.to_string() == operation),
        "canonical operation required"
    );
    ensure!(
        root.is_absolute() && !root.ancestors().any(|p| p.is_symlink()),
        "unlinked private state root required"
    );
    if !root.exists() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(root)?;
    }
    ensure!(
        root.is_dir() && std::fs::metadata(root)?.permissions().mode() & 0o077 == 0,
        "private state root required"
    );
    let path = root.join(operation);
    if !path.exists() {
        std::fs::create_dir(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    ensure!(
        !path.is_symlink() && std::fs::metadata(&path)?.permissions().mode() & 0o077 == 0,
        "private operation directory required"
    );
    Ok(path)
}

pub(crate) fn create_intent(path: &Path, intent: &Intent) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path.join("intent.json"))?;
    file.write_all(&serde_json::to_vec(intent)?)?;
    file.sync_all()?;
    std::fs::File::open(path)?.sync_all()?;
    Ok(())
}
pub(crate) fn receipt(path: &Path, value: &Receipt) -> Result<()> {
    atomic(&path.join("receipt.json"), &serde_json::to_vec(value)?)
}

pub(crate) fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    ensure!(!path.is_symlink(), "linked receipt rejected");
    let temporary = path.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(data)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    std::fs::File::open(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("missing state parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

pub(crate) fn job_configuration(
    order: &OpenCodeWorkOrder,
    origin: &str,
    nonce: &str,
) -> serde_json::Value {
    use serde_json::json;
    let tool = super::gateway::tool_name(order);
    let permissions = json!([
        {"action":"*","resource":"*","effect":"deny"},
        {"action":tool,"resource":"*","effect":"allow"}
    ]);
    json!({"$schema":"https://opencode.ai/config.json","model":super::MODEL,"default_agent":"zaturo-job",
        "update":"disable","share":"disabled","snapshots":false,"lsp":false,"formatter":false,"warming":false,
        "watcher":{"ignore":["**"]},"compaction":{"auto":false},"instructions":[],"skills":[],"plugins":[],"websearch":false,
        "permissions":permissions,
        "policies":[
            {"action":"provider.use","resource":"*","effect":"deny"},
            {"action":"provider.use","resource":"opencode-go","effect":"allow"},
            {"action":"provider.use","resource":"opencode-go/deepseek-v4.1-flash","effect":"allow"},
            {"action":"tool.use","resource":"*","effect":"deny"},
            {"action":"tool.use","resource":format!("{tool}:*"),"effect":"allow"},
            {"action":"mcp.use","resource":"*","effect":"deny"},
            {"action":"mcp.use","resource":"zaturojob","effect":"allow"},
            {"action":"plugin.use","resource":"*","effect":"deny"},
            {"action":"websearch.use","resource":"*","effect":"deny"}
        ],
        "providers":{"opencode-go":{"settings":{"baseURL":format!("{origin}/v1")},"headers":{"x-zaturo-job":nonce},"body":{"thinking":{"type":"disabled"}}}},
        "agents":{"zaturo-job":{"model":super::MODEL,"mode":"primary","steps":1,"permissions":permissions,
            "description":"One approved generation dispatch through a fixed job broker",
            "system":if order.interpret(){"Interpret the owner request and invoke zaturojob_select_action once with a validated decision. Do not execute an action or poll."}else{"Dispatch this approved generation work order using zaturojob_youtube_generate once. Do not poll. A trusted host monitors actual completion; upload and scheduling are forbidden."},
            "request":{"body":{"thinking":{"type":"disabled"},"max_tokens":384}}}},
        "mcp":{"servers":{"zaturojob":{"type":"remote","url":format!("{origin}/mcp"),"headers":{"x-zaturo-job":nonce},"oauth":false,"codemode":false,"protocol":"legacy","timeout":{"startup":10000,"catalog":10000,"execution":25000}}}},
        "tool_output":{"max_lines":32,"max_bytes":4096},"experimental":{"subagent_depth":0},
        "username":format!("ZaturoBot-{}",order.target_worker)
    })
}
