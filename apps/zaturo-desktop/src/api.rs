use crate::config::DesktopConfig;
use anyhow::{Result, ensure};
use reqwest::{Client, Response, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;

#[derive(Clone, Deserialize)]
pub struct FeedItem {
    pub id: String,
    pub created_at: i64,
    pub kind: String,
    pub role: String,
    pub text: String,
    pub task_id: Option<String>,
    pub artifact_id: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
}
#[derive(Deserialize)]
pub struct FeedPage {
    pub items: Vec<FeedItem>,
    pub has_more: bool,
}
#[derive(Deserialize)]
pub struct Worker {
    pub id: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub last_seen_at: i64,
}
#[derive(Serialize)]
pub struct ChatRequest {
    pub message: String,
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_worker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}
#[derive(Clone)]
pub struct HubClient {
    client: Client,
    origin: String,
}
impl HubClient {
    pub fn new(config: &DesktopConfig, token: &str) -> Result<Self> {
        config.validate()?;
        zaturo_security::Credential::new(token)?;
        let mut headers = header::HeaderMap::new();
        let mut auth = header::HeaderValue::from_str(&format!("Bearer {token}"))?;
        auth.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth);
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .default_headers(headers)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(12))
                .build()?,
            origin: config.hub_url.trim_end_matches('/').into(),
        })
    }
    async fn read<T: DeserializeOwned>(mut response: Response) -> Result<T> {
        ensure!(
            response.status().is_success(),
            "hub request failed (HTTP {})",
            response.status().as_u16()
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 1024 * 1024,
                "hub response exceeded client limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid hub response"))
    }
    pub async fn feed(
        &self,
        day: &str,
        offset_minutes: i32,
        kind: &str,
        query: &str,
        offset: u32,
    ) -> Result<FeedPage> {
        ensure!(
            chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok(),
            "use a YYYY-MM-DD date"
        );
        ensure!(
            query.len() <= 256 && offset <= 1000000,
            "invalid feed filter"
        );
        let mut params = vec![
            ("day", day.to_owned()),
            ("offset_minutes", offset_minutes.to_string()),
            ("limit", "20".into()),
            ("offset", offset.to_string()),
        ];
        if !kind.is_empty() {
            params.push(("kind", kind.into()));
        }
        if !query.trim().is_empty() {
            params.push(("q", query.trim().into()));
        }
        let mut page: FeedPage = Self::read(
            self.client
                .get(format!("{}/v1/feed", self.origin))
                .query(&params)
                .send()
                .await?,
        )
        .await?;
        ensure!(page.items.len() <= 20, "hub page exceeded limit");
        page.items.retain(|item| !item.sensitive);
        Ok(page)
    }
    pub async fn workers(&self) -> Result<Vec<Worker>> {
        let workers: Vec<Worker> = Self::read(
            self.client
                .get(format!("{}/v1/workers?limit=100&offset=0", self.origin))
                .send()
                .await?,
        )
        .await?;
        ensure!(workers.len() <= 100, "worker page exceeded limit");
        Ok(workers)
    }
    pub async fn services(&self) -> Result<serde_json::Value> {
        Self::read(
            self.client
                .get(format!("{}/v1/services", self.origin))
                .send()
                .await?,
        )
        .await
    }
    pub async fn send(&self, request: &ChatRequest) -> Result<()> {
        ensure!(
            !request.message.trim().is_empty() && request.message.len() <= 4096,
            "message must contain 1..4096 bytes"
        );
        let _: serde_json::Value = Self::read(
            self.client
                .post(format!("{}/v1/chat", self.origin))
                .json(request)
                .send()
                .await?,
        )
        .await?;
        Ok(())
    }
}

#[derive(Clone, Deserialize)]
pub struct PrivateView {
    pub id: String,
    pub state: String,
    pub expires_at: i64,
    pub task_id: Option<String>,
}
impl HubClient {
    pub async fn create_private(
        &self,
        worker: &str,
        device: &str,
        watch_seconds: u32,
    ) -> Result<PrivateView> {
        zaturo_core::validate_identifier(worker).map_err(anyhow::Error::msg)?;
        zaturo_core::validate_identifier(device).map_err(anyhow::Error::msg)?;
        ensure!(
            watch_seconds <= 120,
            "private watch must be at most 120 seconds"
        );
        Self::read(self.client.post(format!("{}/v1/private-views",self.origin)).json(&serde_json::json!({"worker_id":worker,"device_id":device,"watch_seconds":watch_seconds,"explicit_owner_confirmation":true})).send().await?).await
    }
    pub async fn private_status(&self, id: &str) -> Result<PrivateView> {
        uuid::Uuid::parse_str(id)?;
        Self::read(
            self.client
                .get(format!("{}/v1/private-views/{id}", self.origin))
                .send()
                .await?,
        )
        .await
    }
    pub async fn private_content(&self, id: &str) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        uuid::Uuid::parse_str(id)?;
        let mut response = self
            .client
            .get(format!("{}/v1/private-views/{id}/content", self.origin))
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "private view is not available"
        );
        let mut data = zeroize::Zeroizing::new(Vec::new());
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                data.len() + chunk.len() <= 9 * 1024 * 1024,
                "encrypted private view exceeds limit"
            );
            data.extend_from_slice(&chunk);
        }
        Ok(data)
    }
    pub async fn cancel_private(&self, id: &str) -> Result<()> {
        uuid::Uuid::parse_str(id)?;
        let response = self
            .client
            .delete(format!("{}/v1/private-views/{id}", self.origin))
            .send()
            .await?;
        ensure!(
            response.status().is_success() || response.status().as_u16() == 404,
            "private view revocation failed"
        );
        Ok(())
    }
}
