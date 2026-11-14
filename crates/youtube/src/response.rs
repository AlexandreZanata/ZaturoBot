//! Whitelisted bounded metadata; arbitrary backend objects never enter task/feed payloads.
use crate::{Error, canonical_operation, text};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zaturo_core::{ServiceRequest, ServiceResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Accepted,
    Running,
    Completed,
    Failed,
    Uncertain,
}
impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Uncertain => "uncertain",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub operation_id: String,
    pub state: JobState,
    pub channel: String,
    pub account: String,
    pub count: u32,
    pub mode: String,
    pub generated_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub summary: String,
    pub scheduled_count: u32,
    pub failed_count: u32,
    pub runs: Vec<RunSummary>,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub state: String,
    pub slot: String,
    pub video_id: String,
    pub artifacts: Vec<String>,
}
#[derive(Deserialize, Serialize)]
struct Profile {
    id: String,
    name: String,
    channel_id: String,
    account_id: String,
    browser_configured: bool,
}
#[derive(Deserialize)]
struct Profiles {
    schema_version: u32,
    profiles: Vec<Profile>,
}
#[derive(Deserialize)]
struct Runs {
    schema_version: u32,
    channel: String,
    runs: Vec<RunSummary>,
}
#[derive(Deserialize)]
struct RawJob {
    schema_version: u32,
    operation_id: String,
    state: JobState,
    channel: String,
    account: String,
    count: u32,
    #[serde(default = "schedule")]
    mode: String,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    readiness: Option<std::collections::BTreeMap<String, String>>,
    created_at: i64,
    updated_at: i64,
    #[serde(default)]
    result: Option<RawResult>,
}
#[derive(Deserialize)]
struct RawResult {
    scheduled_count: u32,
    #[serde(default)]
    generated_count: u32,
    failed_count: u32,
    runs: Vec<RunSummary>,
}
fn id(value: &str) -> bool {
    !value.starts_with('-') && zaturo_core::validate_identifier(value).is_ok()
}
fn clean_run(mut run: RunSummary) -> Result<(RunSummary, bool), Error> {
    if !zaturo_core::safe_filename(&run.id)
        || !matches!(
            run.state.as_str(),
            "scheduled" | "ready" | "error" | "in_progress"
        )
        || run.artifacts.len() > 32
        || run.artifacts.iter().any(|v| !zaturo_core::safe_filename(v))
    {
        return Err(Error::InvalidOutput);
    }
    let mut truncated = false;
    for (field, limit) in [
        (&mut run.title, 200),
        (&mut run.topic, 240),
        (&mut run.slot, 64),
        (&mut run.video_id, 32),
    ] {
        let (value, changed) = text(field, limit);
        *field = value;
        truncated |= changed;
    }
    Ok((run, truncated))
}
pub(crate) fn job_status(raw: Value) -> Result<JobStatus, Error> {
    let job: RawJob = serde_json::from_value(raw).map_err(|_| Error::InvalidOutput)?;
    canonical_operation(&job.operation_id).map_err(|_| Error::InvalidOutput)?;
    if job.schema_version != 1
        || !id(&job.channel)
        || !id(&job.account)
        || !(1..=10).contains(&job.count)
        || job.created_at < 0
        || job.updated_at < job.created_at
    {
        return Err(Error::InvalidOutput);
    }
    let result = job.result.unwrap_or(RawResult {
        scheduled_count: 0,
        generated_count: 0,
        failed_count: 0,
        runs: Vec::new(),
    });
    if !matches!(job.mode.as_str(), "schedule" | "generate")
        || result.generated_count > job.count
        || result.scheduled_count > job.count
        || result.failed_count > job.count
        || result.runs.len() > 10
    {
        return Err(Error::InvalidOutput);
    }
    let mut runs = Vec::new();
    let mut truncated = false;
    for run in result.runs {
        let (run, changed) = clean_run(run)?;
        runs.push(run);
        truncated |= changed;
    }
    let state = if job.state == JobState::Completed
        && (result.failed_count != 0
            || if job.mode == "generate" {
                result.generated_count != job.count
                    || result.scheduled_count != 0
                    || runs
                        .iter()
                        .filter(|r| {
                            r.state == "ready" && r.artifacts.iter().any(|a| a == "video.mp4")
                        })
                        .count()
                        != job.count as usize
            } else {
                result.scheduled_count != job.count
                    || runs.iter().filter(|r| r.state == "scheduled").count() != job.count as usize
            }) {
        JobState::Uncertain
    } else {
        job.state
    };
    let summary=match state {
        JobState::Accepted=>"YouTube batch was durably accepted; video work is not finished.",
        JobState::Running=>"YouTube batch is still running; video work is not finished.",
        JobState::Completed=>"YouTube batch reports completion; verify its scheduled-item evidence.",
        JobState::Failed=>"YouTube batch failed; inspect the private project records before starting different work.",
        JobState::Uncertain=>"YouTube batch outcome is uncertain; automatic resubmission is fenced.",
    }.to_owned();
    let summary = if job.mode == "generate" {
        generation_summary(
            state,
            job.error_code.as_deref(),
            job.readiness.as_ref(),
            job.count,
        )?
    } else {
        summary
    };
    Ok(JobStatus {
        operation_id: job.operation_id,
        state,
        channel: job.channel,
        account: job.account,
        count: job.count,
        mode: job.mode,
        generated_count: result.generated_count,
        created_at: job.created_at,
        updated_at: job.updated_at,
        summary,
        scheduled_count: result.scheduled_count,
        failed_count: result.failed_count,
        runs,
        truncated,
    })
}
fn bounded(mut result: ServiceResult, array: &str) -> Result<ServiceResult, Error> {
    loop {
        let size = serde_json::to_vec(&result)
            .map_err(|_| Error::InvalidOutput)?
            .len();
        if size <= 4096 {
            return Ok(result);
        }
        let Some(items) = result.data.get_mut(array).and_then(Value::as_array_mut) else {
            return Err(Error::InvalidOutput);
        };
        if items.pop().is_none() {
            return Err(Error::InvalidOutput);
        }
        result.data["truncated"] = json!(true);
    }
}
pub(crate) fn submission(
    operation: &str,
    request: &ServiceRequest,
    job: JobStatus,
) -> Result<ServiceResult, Error> {
    let (channel, account, count, mode) = match request {
        ServiceRequest::YoutubeSubmit {
            channel,
            account,
            count,
            ..
        } => (channel, Some(account), count, "schedule"),
        ServiceRequest::YoutubeGenerate { channel, count } => (channel, None, count, "generate"),
        _ => return Err(Error::InvalidRequest),
    };
    if job.operation_id != operation
        || job.channel != *channel
        || account.is_some_and(|a| job.account != *a)
        || job.count != *count
        || job.mode != mode
    {
        return Err(Error::InvalidOutput);
    }
    if job.state == JobState::Uncertain {
        return Err(Error::ReconciliationRequired);
    }
    if job.state == JobState::Failed && mode != "generate" {
        return Err(Error::JobFailed);
    }
    let outcome = if job.state == JobState::Failed {
        "blocked"
    } else if job.state == JobState::Completed {
        "completed"
    } else {
        "accepted"
    };
    let summary = if mode == "generate" {
        job.summary.clone()
    } else if job.state == JobState::Completed {
        format!("YouTube batch completed with {} scheduled video(s).", count)
    } else {
        job.summary.clone()
    };
    bounded(
        ServiceResult {
            operation_id: operation.into(),
            action: request.name().into(),
            outcome: outcome.into(),
            summary,
            data: serde_json::to_value(job).map_err(|_| Error::InvalidOutput)?,
        },
        "runs",
    )
}
pub(crate) fn map_result(
    operation: &str,
    request: &ServiceRequest,
    raw: Value,
) -> Result<ServiceResult, Error> {
    match request {
        ServiceRequest::YoutubeProfiles => {
            let mut value: Profiles =
                serde_json::from_value(raw).map_err(|_| Error::InvalidOutput)?;
            if value.schema_version != 1 || value.profiles.len() > 100 {
                return Err(Error::InvalidOutput);
            }
            let mut truncated = false;
            for profile in &mut value.profiles {
                if !id(&profile.id) || !id(&profile.account_id) || !id(&profile.channel_id) {
                    return Err(Error::InvalidOutput);
                }
                let (name, changed) = text(&profile.name, 160);
                profile.name = name;
                truncated |= changed;
            }
            let count = value.profiles.len();
            bounded(
                ServiceResult {
                    operation_id: operation.into(),
                    action: request.name().into(),
                    outcome: "completed".into(),
                    summary: format!("Read {count} configured YouTube channel profile(s)."),
                    data: json!({"profiles":value.profiles,"total":count,"truncated":truncated}),
                },
                "profiles",
            )
        }
        ServiceRequest::YoutubeRuns { channel, .. } => {
            let value: Runs = serde_json::from_value(raw).map_err(|_| Error::InvalidOutput)?;
            if value.schema_version != 1 || value.channel != *channel || value.runs.len() > 100 {
                return Err(Error::InvalidOutput);
            }
            let count = value.runs.len();
            let mut runs = Vec::new();
            let mut truncated = false;
            for run in value.runs {
                let (run, changed) = clean_run(run)?;
                runs.push(run);
                truncated |= changed;
            }
            bounded(
                ServiceResult {
                    operation_id: operation.into(),
                    action: request.name().into(),
                    outcome: "completed".into(),
                    summary: format!(
                        "Read {count} YouTube run record(s); stored video titles and files were not changed."
                    ),
                    data: json!({"channel":channel,"runs":runs,"total":count,"truncated":truncated}),
                },
                "runs",
            )
        }
        ServiceRequest::YoutubeSubmit { .. } | ServiceRequest::YoutubeGenerate { .. } => {
            submission(operation, request, job_status(raw)?)
        }
        ServiceRequest::YoutubeStatus { operation_id } => {
            let job = job_status(raw)?;
            if job.operation_id != *operation_id {
                return Err(Error::InvalidOutput);
            }
            let summary = format!(
                "Observed YouTube job state: {}. {}",
                job.state.as_str(),
                job.summary
            );
            bounded(
                ServiceResult {
                    operation_id: operation.into(),
                    action: request.name().into(),
                    outcome: "completed".into(),
                    summary,
                    data: serde_json::to_value(job).map_err(|_| Error::InvalidOutput)?,
                },
                "runs",
            )
        }
        _ => Err(Error::InvalidRequest),
    }
}

fn schedule() -> String {
    "schedule".into()
}
fn generation_summary(
    state: JobState,
    code: Option<&str>,
    checks: Option<&std::collections::BTreeMap<String, String>>,
    count: u32,
) -> Result<String, Error> {
    Ok(match state {
        JobState::Accepted=>"Lote de geração aceito; os logins ainda serão verificados. Nenhum vídeo foi concluído ou publicado.".into(),
        JobState::Running=>"O computador está preparando o lote. A conclusão depende dos logins e dos arquivos realmente gerados.".into(),
        JobState::Completed=>format!("{count} vídeo(s) gerado(s) no computador de trabalho. Não foram publicados nem agendados."),
        JobState::Uncertain=>"O resultado da geração é incerto. Verifique o computador; o lote não será reenviado automaticamente.".into(),
        JobState::Failed=>match code {
            Some("profile_missing")=>"O perfil do YoutubeBot não está configurado. Entre no computador de trabalho e conecte Google e ChatGPT. Nenhum vídeo foi criado.".into(),
            Some("login_required"|"readiness_unknown")=>{
                let values=checks.ok_or(Error::InvalidOutput)?;
                if values.len()!=2 || !values.contains_key("google") || !values.contains_key("chatgpt") || values.values().any(|v|!matches!(v.as_str(),"ready"|"login_required"|"unknown"|"unavailable")){return Err(Error::InvalidOutput);}
                let missing=["google","chatgpt"].into_iter().filter(|k|values[*k].as_str()=="login_required").map(|k|if k=="google"{"Google"}else{"ChatGPT"}).collect::<Vec<_>>();
                let unknown=["google","chatgpt"].into_iter().filter(|k|matches!(values[*k].as_str(),"unknown"|"unavailable")).map(|k|if k=="google"{"Google"}else{"ChatGPT"}).collect::<Vec<_>>();
                let mut parts=vec![];
                if !missing.is_empty(){parts.push(format!("Sua conta não está logada em {}. Entre no computador de trabalho e faça login no perfil dedicado do YoutubeBot.",missing.join(" e ")));}
                if !unknown.is_empty(){parts.push(format!("Não foi possível confirmar o acesso a {}. Verifique a rede, o navegador e eventuais desafios.",unknown.join(" e ")));}
                parts.push("Nenhum vídeo foi criado ou publicado.".into());parts.join(" ")
            },
            Some("setup_unavailable")=>"O serviço de mídia do Linux não está disponível. Nenhum processo de vídeo foi iniciado; o Codex do computador deve verificar o systemd de usuário.".into(),
            _=>"A geração falhou. Consulte os registros privados do YoutubeBot no PC de trabalho; o lote não será repetido automaticamente.".into(),
        }
    })
}
