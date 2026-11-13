use crate::{Error, PrivacyClass};
use serde::Deserialize;
use zaturo_core::ModelBudgetLimits;

pub const DEEPSEEK_MODEL: &str = "deepseek-v4.1-flash";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelsConfig {
    pub enabled: bool,
    pub provider: String,
    pub default_model: String,
    pub allowed_models: Vec<String>,
    pub allowed_privacy: Vec<PrivacyClass>,
    pub allow_training_models: bool,
    pub allow_global_processing: bool,
    pub timeout_seconds: u64,
    pub max_context_bytes: usize,
    pub max_response_bytes: usize,
    pub max_output_tokens: u32,
    pub max_attempts: u32,
    pub retry_base_millis: u64,
    pub concurrent_requests: usize,
    pub budget: ModelBudgetLimits,
}
impl ModelsConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if self.provider != "opencode-go"
            || self.default_model != DEEPSEEK_MODEL
            || self.allowed_models != [DEEPSEEK_MODEL]
        {
            return Err(Error::InvalidConfig(
                "only the owner-selected DeepSeek model is allowed",
            ));
        }
        if self.allowed_privacy != [PrivacyClass::PublicCoding] || self.allow_training_models {
            return Err(Error::InvalidConfig(
                "only public coding data and non-training routes are supported",
            ));
        }
        if !(1..=60).contains(&self.timeout_seconds)
            || !(1024..=32768).contains(&self.max_context_bytes)
            || !(1024..=65536).contains(&self.max_response_bytes)
            || !(64..=2048).contains(&self.max_output_tokens)
            || !(1..=3).contains(&self.max_attempts)
            || !(1..=5000).contains(&self.retry_base_millis)
            || !(1..=2).contains(&self.concurrent_requests)
        {
            return Err(Error::InvalidConfig(
                "request, retry, context, or concurrency limit is outside bounds",
            ));
        }
        if !self.budget.validates() {
            return Err(Error::InvalidConfig("invalid durable budget limits"));
        }
        let maximum = self.max_context_bytes as u64 + 1024 + u64::from(self.max_output_tokens);
        if maximum > self.budget.per_request_tokens {
            return Err(Error::InvalidConfig(
                "per-request token reservation cannot cover the context bound",
            ));
        }
        Ok(())
    }
}
