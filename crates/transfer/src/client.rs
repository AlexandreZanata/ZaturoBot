//! Device-scoped bounded HTTP client and durable encrypted upload/download caches.
use crate::{Peer, blob, crypto};
use anyhow::{Result, ensure};
use reqwest::{Client, Method, Response, header};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use zaturo_core::{
    CreateTransfer, DEFAULT_CHUNK_BYTES, FileTransfer, TransferLimits, TransferState,
};

#[derive(Clone)]
pub struct DeviceClient {
    http: Client,
    origin: String,
    pub device_id: String,
}
async fn bytes(mut response: Response, limit: usize) -> Result<Vec<u8>> {
    ensure!(
        response.status().is_success(),
        "hub request failed (HTTP {})",
        response.status().as_u16()
    );
    let mut out = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            out.len().saturating_add(chunk.len()) <= limit,
            "hub response exceeds limit"
        );
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}
impl DeviceClient {
    pub fn new(origin: &str, token: &str, device_id: &str) -> Result<Self> {
        zaturo_security::validate_endpoint(origin)?;
        zaturo_security::Credential::new(token)?;
        zaturo_core::validate_identifier(device_id).map_err(anyhow::Error::msg)?;
        let mut auth = header::HeaderValue::from_str(&format!("Bearer {token}"))?;
        auth.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, auth);
        let mut builder = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120));
        // Literal-loopback HTTP must never escape the private SSH tunnel through an ambient proxy.
        if reqwest::Url::parse(origin)?.scheme() == "http" {
            builder = builder.no_proxy();
        }
        let http = builder.build()?;
        Ok(Self {
            http,
            origin: origin.trim_end_matches('/').into(),
            device_id: device_id.into(),
        })
    }
    async fn execute(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        for attempt in 0..4 {
            let next = request
                .try_clone()
                .ok_or_else(|| anyhow::anyhow!("request cannot be safely replayed"))?;
            match next.send().await {
                Ok(response)
                    if !matches!(response.status().as_u16(), 408 | 429 | 502 | 503 | 504)
                        || attempt == 3 =>
                {
                    return Ok(response);
                }
                Err(_) if attempt == 3 => anyhow::bail!(
                    "hub connection failed; resume the durable request after reconnecting"
                ),
                _ => tokio::time::sleep(Duration::from_millis(250 << attempt)).await,
            }
        }
        unreachable!("bounded retry loop always returns")
    }
    async fn json<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        Ok(serde_json::from_slice(&bytes(response, 128 * 1024).await?)?)
    }
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.json(
            self.execute(self.http.get(format!("{}{path}", self.origin)))
                .await?,
        )
        .await
    }
    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        self.json(
            self.execute(self.http.post(format!("{}{path}", self.origin)).json(body))
                .await?,
        )
        .await
    }
    pub async fn verify_identity(&self) -> Result<Peer> {
        let me: Peer = self.get("/v1/device/me").await?;
        ensure!(
            me.id == self.device_id,
            "device token does not match local configuration"
        );
        Ok(me)
    }
    pub async fn private_frame(&self, id: &str, ciphertext: Vec<u8>) -> Result<()> {
        blob::object_name(id)?;
        ensure!(
            ciphertext.len() <= 8 * 1024 * 1024 + 65536,
            "private frame exceeds limit"
        );
        let response = self
            .execute(
                self.http
                    .post(format!(
                        "{}/v1/device/private-views/{id}/content",
                        self.origin
                    ))
                    .body(ciphertext),
            )
            .await?;
        bytes(response, 1024).await?;
        Ok(())
    }
    pub async fn peers(&self) -> Result<Vec<Peer>> {
        self.get("/v1/device/peers").await
    }
    pub async fn limits(&self) -> Result<TransferLimits> {
        let limits: TransferLimits = self.get("/v1/device/limits").await?;
        ensure!(limits.validates(), "invalid hub transfer limits");
        Ok(limits)
    }
    pub async fn status(&self, id: &str) -> Result<FileTransfer> {
        blob::object_name(id)?;
        let transfer: FileTransfer = self.get(&format!("/v1/device/transfers/{id}")).await?;
        ensure!(
            transfer.id == id
                && transfer.size_bytes <= 4 * 1024 * 1024 * 1024
                && zaturo_core::safe_filename(&transfer.filename)
                && zaturo_core::valid_sha256(&transfer.sha256),
            "invalid transfer metadata"
        );
        Ok(transfer)
    }
    pub async fn list(&self, incoming: bool, pending: bool) -> Result<Vec<FileTransfer>> {
        self.list_page(incoming, pending, 0).await
    }
    async fn list_page(
        &self,
        incoming: bool,
        pending: bool,
        offset: usize,
    ) -> Result<Vec<FileTransfer>> {
        self.get(&format!(
            "/v1/device/transfers?incoming={incoming}&pending={pending}&limit=20&offset={offset}"
        ))
        .await
    }
    /// Persists encrypted bytes and the idempotent admission request before contacting the hub.
    /// The returned key is accepted by upload after any interruption, including lost admission replies.
    pub async fn prepare(
        &self,
        source: &Path,
        recipient: &str,
        identity: &Path,
        cache: &Path,
    ) -> Result<String> {
        let me = self.verify_identity().await?;
        ensure!(
            crypto::public_identity(identity)? == me.encryption_recipient,
            "local encryption identity differs from registered public key"
        );
        let peer = self
            .peers()
            .await?
            .into_iter()
            .find(|p| p.id == recipient && p.id != me.id)
            .ok_or_else(|| anyhow::anyhow!("recipient is not registered"))?;
        let limits = self.limits().await?;
        let cache = blob::private_directory(cache)?;
        let key = uuid::Uuid::new_v4().to_string();
        let cipher = cache.join(format!("{key}.send.age"));
        let src = source.to_owned();
        let output = cipher.clone();
        let encrypted = tokio::task::spawn_blocking(move || {
            crypto::encrypt(
                &src,
                &output,
                &[peer.encryption_recipient, me.encryption_recipient],
                limits.max_file_bytes,
            )
        })
        .await?;
        let (size_bytes, sha256, filename) = match encrypted {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(cipher);
                return Err(error);
            }
        };
        let request = CreateTransfer {
            recipient: recipient.into(),
            filename,
            size_bytes,
            sha256,
            idempotency_key: key.clone(),
        };
        let mut file = blob::new_file(&cache.join(format!("{key}.request.json")))?;
        serde_json::to_writer(&mut file, &request)?;
        file.sync_all()?;
        fs::File::open(&cache)?.sync_all()?;
        Ok(key)
    }
    pub async fn upload(&self, key: &str, cache: &Path) -> Result<FileTransfer> {
        blob::object_name(key)?;
        let cache = blob::private_directory(cache)?;
        let _lock = blob::lock(&cache.join(format!("{key}.send.lock")))?;
        let request: CreateTransfer = serde_json::from_reader(
            blob::read_file(&cache.join(format!("{key}.request.json")))?.take(4096),
        )?;
        ensure!(request.idempotency_key == key, "upload cache key mismatch");
        let mut transfer: FileTransfer = self.post("/v1/device/transfers", &request).await?;
        if matches!(
            transfer.state,
            TransferState::Ready | TransferState::Delivered
        ) {
            let _ = fs::remove_file(cache.join(format!("{key}.send.age")));
            return Ok(transfer);
        }
        ensure!(
            transfer.state == TransferState::Uploading,
            "transfer expired"
        );
        let cipher = cache.join(format!("{key}.send.age"));
        let path = cipher.clone();
        let expected = (request.size_bytes, request.sha256);
        ensure!(
            tokio::task::spawn_blocking(move || blob::digest(&path)).await?? == expected,
            "cached encrypted upload is inconsistent"
        );
        let mut file = blob::read_file(&cipher)?;
        while transfer.uploaded_bytes < transfer.size_bytes {
            let offset = transfer.uploaded_bytes;
            file.seek(SeekFrom::Start(offset))?;
            let mut chunk =
                vec![0; DEFAULT_CHUNK_BYTES.min((transfer.size_bytes - offset) as usize)];
            file.read_exact(&mut chunk)?;
            // A lost response is reconciled by the next invocation's durable hub checkpoint.
            transfer = self
                .json(
                    self.execute(
                        self.http
                            .request(
                                Method::PUT,
                                format!(
                                    "{}/v1/device/transfers/{}/chunk?offset={offset}",
                                    self.origin, transfer.id
                                ),
                            )
                            .body(chunk),
                    )
                    .await?,
                )
                .await?;
        }
        let transfer: FileTransfer = self
            .post(
                &format!("/v1/device/transfers/{}/complete", transfer.id),
                &serde_json::json!({}),
            )
            .await?;
        fs::remove_file(cipher)?;
        Ok(transfer)
    }
    pub async fn receive(
        &self,
        id: &str,
        identity: &Path,
        cache: &Path,
        inbox: &Path,
    ) -> Result<PathBuf> {
        let transfer = self.status(id).await?;
        ensure!(
            transfer.recipient == self.device_id
                && matches!(
                    transfer.state,
                    TransferState::Ready | TransferState::Delivered
                ),
            "transfer is not ready for this recipient"
        );
        ensure!(
            zaturo_core::safe_filename(&transfer.filename),
            "invalid remote filename"
        );
        let cache = blob::private_directory(cache)?;
        let inbox = blob::private_directory(inbox)?;
        // Each transfer has its own private directory: identical basenames never collide.
        let _lock = blob::lock(&cache.join(format!("{id}.receive.lock")))?;
        let target_dir = blob::private_directory(&inbox.join(id))?;
        let destination = target_dir.join(&transfer.filename);
        let path = cache.join(blob::object_name(id)?);
        let mut file = match blob::new_file(&path) {
            Ok(f) => f,
            Err(e) if path.symlink_metadata().is_ok() => {
                let _ = e;
                std::fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&path)?
            }
            Err(e) => return Err(e),
        };
        ensure!(
            file.metadata()?.is_file() && file.metadata()?.len() <= transfer.size_bytes,
            "invalid download cache"
        );
        let mut offset = file.metadata()?.len();
        file.seek(SeekFrom::Start(offset))?;
        while offset < transfer.size_bytes {
            let response=self.execute(self.http.get(format!("{}/v1/device/transfers/{id}/chunk?offset={offset}&length={DEFAULT_CHUNK_BYTES}",self.origin))).await?;
            let chunk = bytes(response, DEFAULT_CHUNK_BYTES).await?;
            ensure!(
                !chunk.is_empty() && offset + chunk.len() as u64 <= transfer.size_bytes,
                "invalid download chunk"
            );
            file.write_all(&chunk)?;
            file.sync_data()?;
            offset += chunk.len() as u64;
        }
        drop(file);
        let cipher = path.clone();
        let t = transfer.clone();
        let key = identity.to_owned();
        let dest = destination.clone();
        let scratch =
            blob::private_directory(&inbox.join(".staging"))?.join(format!("{id}.partial"));
        tokio::task::spawn_blocking(move || -> Result<()> {
            ensure!(
                blob::digest(&cipher)? == (t.size_bytes, t.sha256),
                "encrypted payload checksum mismatch"
            );
            crypto::decrypt(
                &cipher,
                &key,
                &scratch,
                &dest,
                &t.filename,
                4 * 1024 * 1024 * 1024,
            )?;
            fs::File::open(dest.parent().expect("private inbox parent"))?.sync_all()?;
            Ok(())
        })
        .await??;
        let _: FileTransfer = self
            .post(
                &format!("/v1/device/transfers/{id}/ack"),
                &serde_json::json!({"sha256":transfer.sha256}),
            )
            .await?;
        fs::remove_file(path)?;
        Ok(destination)
    }
    pub async fn sync(&self, identity: &Path, cache: &Path, inbox: &Path) -> Result<SyncOutcome> {
        let mut pending = Vec::new();
        loop {
            let page = self.list_page(true, true, pending.len()).await?;
            let done = page.len() < 20;
            ensure!(
                pending.len() + page.len() <= 512,
                "pending inbox exceeds hard transfer limit"
            );
            pending.extend(page);
            if done {
                break;
            }
        }
        let mut outcome = SyncOutcome::default();
        for transfer in pending {
            match self.receive(&transfer.id, identity, cache, inbox).await {
                Ok(path) => outcome.received.push(path),
                Err(_) => outcome.failed += 1,
            }
        }
        Ok(outcome)
    }
}
/// One corrupt file cannot starve later transfers. Failed items remain pending for explicit retry.
#[derive(Default)]
pub struct SyncOutcome {
    pub received: Vec<PathBuf>,
    pub failed: usize,
}
use std::os::unix::fs::OpenOptionsExt;
