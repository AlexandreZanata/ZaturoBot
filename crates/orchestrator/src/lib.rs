//! A bounded coding-diagnostic planner. Model proposals never grant tool permissions.
use serde::Deserialize;
use zaturo_core::{DiagnosticResult, ModelUsage, OrchestrationResult};
use zaturo_llm::{BudgetPort, CallContext, Cancellation, Message, PrivacyClass, Router};
use zaturo_security::{ActionClass, permits};

pub const PROMPT_ID: &str = "diagnostic-planner";
pub const PROMPT_VERSION: u32 = 1;
const SYSTEM_PROMPT: &str = include_str!("../../../prompts/diagnostic-planner.md");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Model(#[from] zaturo_llm::Error),
    #[error("invalid bounded orchestration input")]
    InvalidInput,
    #[error("model output violates the structured action contract")]
    InvalidProposal,
    #[error("tool proposal denied by policy")]
    PolicyDenied,
    #[error("diagnostic evidence failed acceptance")]
    ToolFailed,
    #[error("orchestration cancelled before dispatch")]
    Cancelled,
}
pub type FutureResult<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'a>>;

pub trait DiagnosticExecutor: Send + Sync {
    /// Implementations journal intent/result and independently enforce local grants.
    fn execute<'a>(&'a self, operation_id: &'a str) -> FutureResult<'a, DiagnosticResult>;
}

#[derive(Clone)]
pub struct RunInput {
    pub task_id: String,
    pub operation_id: String,
    pub owner_request: String,
    pub deadline_at: i64,
    pub privacy: PrivacyClass,
}
pub struct Outcome {
    pub diagnostic: DiagnosticResult,
    pub report: OrchestrationResult,
}
impl std::fmt::Debug for RunInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunInput")
            .field("task_id", &self.task_id)
            .field("operation_id", &self.operation_id)
            .field("owner_request", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("model", &self.report.model)
            .field("summary", &"[REDACTED]")
            .field("usage", &self.report.usage)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Proposal {
    Tool { tool: String },
    Finish { summary: String },
}

pub async fn run(
    router: &Router,
    input: &RunInput,
    budget: &dyn BudgetPort,
    executor: &dyn DiagnosticExecutor,
    cancellation: &Cancellation,
) -> Result<Outcome, Error> {
    validate_input(input)?;
    let mut messages = vec![Message::system(SYSTEM_PROMPT), Message::user(serde_json::json!({"untrusted_owner_request":input.owner_request,"operation_id":input.operation_id,"available_tools":["diagnostic.v1"],"purpose":"public coding environment diagnostic"}).to_string())];
    let first = router
        .complete(&context(input, 1), messages.clone(), budget, cancellation)
        .await?;
    match parse(&first.content)? {
        Proposal::Tool { tool }
            if permits(&tool, ActionClass::ReadOnly, &["diagnostic.v1".into()]) => {}
        Proposal::Tool { .. } => return Err(Error::PolicyDenied),
        Proposal::Finish { .. } => return Err(Error::InvalidProposal),
    }
    if cancellation.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let diagnostic = executor.execute(&input.operation_id).await?;
    if !diagnostic.validates(&input.operation_id) {
        return Err(Error::ToolFailed);
    }
    if cancellation.is_cancelled() {
        return Err(Error::Cancelled);
    }
    messages.push(Message::assistant(first.content));
    messages.push(Message::user(
        serde_json::json!({"untrusted_tool_observation":diagnostic}).to_string(),
    ));
    let second = router
        .complete(&context(input, 2), messages, budget, cancellation)
        .await?;
    let summary = match parse(&second.content)? {
        Proposal::Finish { summary }
            if !summary.trim().is_empty()
                && summary.chars().count() <= 512
                && summary.len() <= 1024 =>
        {
            summary
        }
        _ => return Err(Error::InvalidProposal),
    };
    let usage_complete = first.usage_complete && second.usage_complete;
    let mut usage = sum_usage(&first.usage, &second.usage)?;
    if !usage_complete {
        usage.cost_microusd = None;
    }
    let report = OrchestrationResult {
        model: second.model,
        steps: 2,
        summary,
        usage,
        model_calls: first
            .model_calls
            .checked_add(second.model_calls)
            .ok_or(Error::InvalidProposal)?,
        usage_complete,
    };
    if !report.validates() {
        return Err(Error::InvalidProposal);
    }
    Ok(Outcome { diagnostic, report })
}

fn context(input: &RunInput, step: u32) -> CallContext {
    CallContext {
        task_id: input.task_id.clone(),
        operation_id: input.operation_id.clone(),
        step,
        deadline_at: input.deadline_at,
        privacy: input.privacy,
    }
}
fn validate_input(input: &RunInput) -> Result<(), Error> {
    if zaturo_core::validate_identifier(&input.task_id).is_err()
        || zaturo_core::validate_identifier(&input.operation_id).is_err()
        || input.owner_request.trim().is_empty()
        || input.owner_request.len() > 4096
        || input.privacy != PrivacyClass::PublicCoding
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
fn parse(text: &str) -> Result<Proposal, Error> {
    if text.len() > 4096 {
        return Err(Error::InvalidProposal);
    }
    serde_json::from_str(text).map_err(|_| Error::InvalidProposal)
}
fn sum_usage(first: &ModelUsage, second: &ModelUsage) -> Result<ModelUsage, Error> {
    let cost_microusd = match (first.cost_microusd, second.cost_microusd) {
        (Some(a), Some(b)) => Some(a.checked_add(b).ok_or(Error::InvalidProposal)?),
        _ => None,
    };
    Ok(ModelUsage {
        input_tokens: first
            .input_tokens
            .checked_add(second.input_tokens)
            .ok_or(Error::InvalidProposal)?,
        output_tokens: first
            .output_tokens
            .checked_add(second.output_tokens)
            .ok_or(Error::InvalidProposal)?,
        cost_microusd,
    })
}

#[cfg(test)]
mod tests;
