use crate::{
    CallContext, Error, FutureResult, LlmClient, Message, ModelRequest, ModelResponse,
    ModelsConfig, PrivacyClass,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{Duration, Instant},
};
use zaturo_core::ModelUsage;

/// Verified at the official Go endpoint table on 2026-11-13. No arbitrary remote origin.
pub const GO_CHAT_ENDPOINT: &str = "https://opencode.ai/inference/go/openai/v1/chat/completions";
const USER_AGENT: &str = concat!(
    "ZaturoBot/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/AlexandreZanata/ZaturoBot)"
);

pub struct OpenCodeGoClient {
    http: reqwest::Client,
    api_key: String,
    endpoint: String,
    config: ModelsConfig,
}
impl fmt::Debug for OpenCodeGoClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenCodeGoClient")
            .field("api_key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}
impl OpenCodeGoClient {
    pub fn new(config: &ModelsConfig, api_key: String) -> Result<Self, Error> {
        Self::at(config, api_key, GO_CHAT_ENDPOINT.into())
    }
    fn at(config: &ModelsConfig, api_key: String, endpoint: String) -> Result<Self, Error> {
        config.validate()?;
        if !config.enabled {
            return Err(Error::Disabled);
        }
        if api_key.len() < 16
            || api_key.len() > 4096
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(Error::InvalidConfig(
                "provider API key is unavailable or invalid",
            ));
        }
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|_| Error::Transport)?;
        Ok(Self {
            http,
            api_key,
            endpoint,
            config: config.clone(),
        })
    }
    fn validate_request(&self, request: &ModelRequest) -> Result<(), Error> {
        let CallContext {
            privacy,
            operation_id,
            ..
        } = &request.context;
        if request.model != self.config.default_model
            || *privacy != PrivacyClass::PublicCoding
            || !self.config.allow_global_processing
            || zaturo_core::validate_identifier(operation_id).is_err()
            || request.max_output_tokens > self.config.max_output_tokens
            || request.messages.is_empty()
            || request.messages.len() > 8
        {
            return Err(Error::Denied(
                "provider request is outside its configured coding route",
            ));
        }
        if request
            .messages
            .iter()
            .map(|m| m.content.len())
            .sum::<usize>()
            > self.config.max_context_bytes
        {
            return Err(Error::InvalidRequest);
        }
        Ok(())
    }
}
impl LlmClient for OpenCodeGoClient {
    fn complete(&self, request: ModelRequest) -> FutureResult<'_, ModelResponse> {
        Box::pin(async move {
            self.validate_request(&request)?;
            let started = Instant::now();
            let mut response = self
                .http
                .post(&self.endpoint)
                .bearer_auth(&self.api_key)
                .header("x-opencode-session", &request.context.operation_id)
                .json(&ChatRequest {
                    model: &request.model,
                    messages: &request.messages,
                    max_tokens: request.max_output_tokens,
                    stream: false,
                })
                .send()
                .await
                .map_err(transport_error)?;
            let status = response.status().as_u16();
            if !(200..300).contains(&status) {
                return Err(match status {
                    401 | 403 => Error::Authentication,
                    429 => Error::RateLimited {
                        retry_after_seconds: response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse().ok()),
                    },
                    500..=599 => Error::Server(status),
                    _ => Error::Rejected(status),
                });
            }
            if response
                .content_length()
                .is_some_and(|size| size > self.config.max_response_bytes as u64)
            {
                return Err(Error::ResponseTooLarge);
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
                if body.len().saturating_add(chunk.len()) > self.config.max_response_bytes {
                    return Err(Error::ResponseTooLarge);
                }
                body.extend_from_slice(&chunk);
            }
            parse_response(
                &body,
                started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
            )
        })
    }
}

fn transport_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Transport
    }
}
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    max_tokens: u32,
    stream: bool,
}
#[derive(Deserialize)]
struct ChatResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}
#[derive(Deserialize)]
struct Choice {
    message: Answer,
    finish_reason: Option<String>,
}
#[derive(Deserialize)]
struct Answer {
    content: Option<String>,
}
#[derive(Deserialize)]
struct Usage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    cost: Option<f64>,
}

fn parse_response(body: &[u8], duration_ms: u64) -> Result<ModelResponse, Error> {
    let response: ChatResponse =
        serde_json::from_slice(body).map_err(|_| Error::InvalidResponse)?;
    if response.choices.len() != 1 || response.model.len() > 128 {
        return Err(Error::InvalidResponse);
    }
    let answer = response
        .choices
        .into_iter()
        .next()
        .ok_or(Error::InvalidResponse)?;
    let content = answer
        .message
        .content
        .filter(|s| !s.trim().is_empty())
        .ok_or(Error::InvalidResponse)?;
    let usage = response.usage.ok_or(Error::InvalidResponse)?;
    // Go allowance describes subscription quota consumption, not the request charge.
    // It must not be relabeled as a monetary cost or inferred when cost is absent.
    let cost = usage.cost;
    let usage = ModelUsage {
        input_tokens: usage.prompt_tokens.ok_or(Error::InvalidResponse)?,
        output_tokens: usage.completion_tokens.ok_or(Error::InvalidResponse)?,
        cost_microusd: cost.map(cost_microusd).transpose()?,
    };
    if !usage.validates() {
        return Err(Error::InvalidResponse);
    }
    Ok(ModelResponse {
        model: response.model,
        content,
        usage,
        finish_reason: answer.finish_reason.ok_or(Error::InvalidResponse)?,
        duration_ms,
        model_calls: 1,
        usage_complete: true,
    })
}
fn cost_microusd(usd: f64) -> Result<u64, Error> {
    if !usd.is_finite() || !(0.0..=1_000_000.0).contains(&usd) {
        return Err(Error::InvalidResponse);
    }
    Ok((usd * 1_000_000.0).ceil() as u64)
}

#[cfg(test)]
mod tests;
