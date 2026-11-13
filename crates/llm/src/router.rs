use crate::{
    BudgetPort, CallContext, Cancellation, Error, LlmClient, Message, ModelRequest, ModelResponse,
    ModelsConfig, PrivacyClass,
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;
use zaturo_core::ModelReservation;

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(i64::MAX)
    }
}

#[cfg(test)]
mod tests;

pub struct Router {
    config: ModelsConfig,
    client: Arc<dyn LlmClient>,
    clock: Arc<dyn Clock>,
    requests: Semaphore,
}
impl Router {
    pub fn new(config: ModelsConfig, client: Arc<dyn LlmClient>) -> Result<Self, Error> {
        Self::with_clock(config, client, Arc::new(SystemClock))
    }
    pub fn with_clock(
        config: ModelsConfig,
        client: Arc<dyn LlmClient>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, Error> {
        config.validate()?;
        Ok(Self {
            requests: Semaphore::new(config.concurrent_requests),
            config,
            client,
            clock,
        })
    }
    pub fn config(&self) -> &ModelsConfig {
        &self.config
    }

    pub async fn complete(
        &self,
        context: &CallContext,
        messages: Vec<Message>,
        budget: &dyn BudgetPort,
        cancellation: &Cancellation,
    ) -> Result<ModelResponse, Error> {
        let input_tokens = self.validate(context, &messages, cancellation)?;
        let _permit = tokio::select! {
            _ = cancellation.cancelled() => return Err(Error::Cancelled),
            result = tokio::time::timeout(self.remaining(context)?, self.requests.acquire()) => {
                result.map_err(|_| Error::Deadline)?.map_err(|_| Error::Disabled)?
            }
        };
        for attempt in 1..=self.config.max_attempts {
            self.check_active(context, cancellation)?;
            let request_id = format!("{}:{}:{attempt}", context.operation_id, context.step);
            let reservation = ModelReservation {
                request_id: request_id.clone(),
                operation_id: context.operation_id.clone(),
                task_id: context.task_id.clone(),
                estimated_input_tokens: input_tokens,
                max_output_tokens: u64::from(self.config.max_output_tokens),
            };
            budget.reserve(&reservation).await?;
            let request = ModelRequest {
                model: self.config.default_model.clone(),
                context: context.clone(),
                request_id: request_id.clone(),
                messages: messages.clone(),
                max_output_tokens: self.config.max_output_tokens,
            };
            let outcome = self.send(context, request, cancellation).await;
            match outcome {
                Ok(mut response) => {
                    budget.settle(&request_id, &response.usage).await?;
                    self.accept_response(&response)?;
                    response.model_calls = attempt;
                    response.usage_complete = attempt == 1;
                    return Ok(response);
                }
                Err(error) => {
                    budget.uncertain(&request_id).await?;
                    if attempt == self.config.max_attempts
                        || !matches!(error, Error::RateLimited { .. } | Error::Server(_))
                    {
                        return Err(error);
                    }
                    self.backoff(context, attempt, &error, cancellation).await?;
                }
            }
        }
        Err(Error::InvalidResponse)
    }

    fn validate(
        &self,
        context: &CallContext,
        messages: &[Message],
        cancellation: &Cancellation,
    ) -> Result<u64, Error> {
        if !self.config.enabled {
            return Err(Error::Disabled);
        }
        if context.privacy != PrivacyClass::PublicCoding
            || !self.config.allowed_privacy.contains(&context.privacy)
        {
            return Err(Error::Denied(
                "explicit public-coding disclosure is required",
            ));
        }
        if !self.config.allow_global_processing {
            return Err(Error::Denied(
                "the selected model requires explicit Global processing authorization",
            ));
        }
        if zaturo_core::validate_identifier(&context.operation_id).is_err()
            || zaturo_core::validate_identifier(&context.task_id).is_err()
            || !(1..=2).contains(&context.step)
            || messages.is_empty()
            || messages.len() > 8
        {
            return Err(Error::InvalidRequest);
        }
        let bytes = messages
            .iter()
            .try_fold(0usize, |sum, message| {
                sum.checked_add(message.content.len())
            })
            .ok_or(Error::InvalidRequest)?;
        if bytes == 0 || bytes > self.config.max_context_bytes {
            return Err(Error::InvalidRequest);
        }
        self.check_active(context, cancellation)?;
        // A deliberately conservative reservation, not a tokenizer or a provider quota.
        // Actual overrun must be persisted as debt by BudgetPort before later calls fail.
        Ok(bytes as u64 + 1024)
    }

    fn check_active(
        &self,
        context: &CallContext,
        cancellation: &Cancellation,
    ) -> Result<(), Error> {
        if cancellation.is_cancelled() {
            return Err(Error::Cancelled);
        }
        self.remaining(context).map(|_| ())
    }
    fn remaining(&self, context: &CallContext) -> Result<Duration, Error> {
        let milliseconds = context
            .deadline_at
            .checked_sub(self.clock.now_ms())
            .filter(|n| *n > 0)
            .ok_or(Error::Deadline)?;
        Ok(Duration::from_millis(milliseconds as u64))
    }
    async fn send(
        &self,
        context: &CallContext,
        request: ModelRequest,
        cancellation: &Cancellation,
    ) -> Result<ModelResponse, Error> {
        self.check_active(context, cancellation)?;
        let timeout = self
            .remaining(context)?
            .min(Duration::from_secs(self.config.timeout_seconds));
        tokio::select! {
            _ = cancellation.cancelled() => Err(Error::Cancelled),
            result = tokio::time::timeout(timeout, self.client.complete(request)) => result.map_err(|_| Error::Timeout)?,
        }
    }
    fn accept_response(&self, response: &ModelResponse) -> Result<(), Error> {
        if response.model != self.config.default_model
            || response.finish_reason != "stop"
            || response.content.trim().is_empty()
            || response.content.len() > self.config.max_response_bytes
            || !response.usage.validates()
        {
            return Err(Error::InvalidResponse);
        }
        Ok(())
    }
    async fn backoff(
        &self,
        context: &CallContext,
        attempt: u32,
        error: &Error,
        cancellation: &Cancellation,
    ) -> Result<(), Error> {
        let exponential = self
            .config
            .retry_base_millis
            .saturating_mul(1 << (attempt - 1));
        let retry_after = match error {
            Error::RateLimited {
                retry_after_seconds: Some(seconds),
            } => seconds.saturating_mul(1000),
            _ => 0,
        };
        if retry_after > 5000 {
            return Err(error.clone());
        }
        let delay = Duration::from_millis(exponential.max(retry_after).min(5000));
        if delay >= self.remaining(context)? {
            return Err(Error::Deadline);
        }
        tokio::select! { _ = cancellation.cancelled() => Err(Error::Cancelled), _ = tokio::time::sleep(delay) => Ok(()) }
    }
}
