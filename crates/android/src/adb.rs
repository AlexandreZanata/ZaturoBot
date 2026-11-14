//! Fixed ADB invocations against the loopback server, with owned process-group cleanup.
use crate::{Error, Result, SessionLease};
use rustix::process::{Pid, Signal, kill_process_group};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};
use zaturo_core::{
    AndroidAuthorization, AndroidCapability, AndroidDeviceObservation, AndroidDeviceState,
    AndroidDeviceStatus, AndroidTransport, AndroidWatchRequest, MAX_ANDROID_SCREENSHOT_BYTES,
    RegisteredAndroidDevice,
};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct AdbLimits {
    pub command_timeout: Duration,
    pub max_text_bytes: usize,
    pub max_screenshot_bytes: usize,
}
impl Default for AdbLimits {
    fn default() -> Self {
        Self {
            command_timeout: Duration::from_secs(5),
            max_text_bytes: 16 * 1024,
            max_screenshot_bytes: MAX_ANDROID_SCREENSHOT_BYTES,
        }
    }
}
impl AdbLimits {
    fn valid(&self) -> bool {
        !self.command_timeout.is_zero()
            && self.command_timeout <= Duration::from_secs(15)
            && (32..=256 * 1024).contains(&self.max_text_bytes)
            && (128..=MAX_ANDROID_SCREENSHOT_BYTES).contains(&self.max_screenshot_bytes)
    }
}

/// Private image bytes. Deliberately has no Debug, Serialize, file writer or model adapter.
pub struct SensitiveFrame {
    png: Zeroizing<Vec<u8>>,
    captured_at: i64,
    width: u32,
    height: u32,
}
impl SensitiveFrame {
    pub fn bytes(&self) -> &[u8] {
        self.png.as_slice()
    }
    pub fn captured_at(&self) -> i64 {
        self.captured_at
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn from_png(png: Zeroizing<Vec<u8>>, captured_at: i64) -> Result<Self> {
        if png.len() < 57
            || !png.starts_with(b"\x89PNG\r\n\x1a\n")
            || png[8..12] != 13u32.to_be_bytes()
            || &png[12..16] != b"IHDR"
            || &png[png.len() - 8..png.len() - 4] != b"IEND"
        {
            return Err(Error::InvalidOutput);
        }
        let width = u32::from_be_bytes(png[16..20].try_into().map_err(|_| Error::InvalidOutput)?);
        let height = u32::from_be_bytes(png[20..24].try_into().map_err(|_| Error::InvalidOutput)?);
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > 16_777_216
        {
            return Err(Error::InvalidOutput);
        }
        Ok(Self {
            png,
            captured_at,
            width,
            height,
        })
    }
}
#[derive(Clone)]
pub struct AdbClient {
    executable: PathBuf,
    home: PathBuf,
    limits: AdbLimits,
}
struct ProcessGroup(Option<Pid>);
impl ProcessGroup {
    fn terminate(&mut self) {
        if let Some(pid) = self.0.take() {
            let _ = kill_process_group(pid, Signal::KILL);
        }
    }
    fn disarm(&mut self) {
        self.0 = None;
    }
}
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.terminate();
    }
}

async fn bounded_read<R: AsyncRead + Unpin>(reader: R, limit: usize) -> Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::new());
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error::CommandFailed)?;
    if bytes.len() > limit {
        return Err(Error::OutputLimit);
    }
    Ok(bytes)
}
impl AdbClient {
    pub fn new(executable: &Path) -> Result<Self> {
        Self::with_limits(executable, AdbLimits::default())
    }
    pub fn with_limits(executable: &Path, limits: AdbLimits) -> Result<Self> {
        if !executable.is_absolute() || !limits.valid() {
            return Err(Error::Invalid);
        }
        let executable = fs::canonicalize(executable).map_err(|_| Error::Unavailable)?;
        let metadata = fs::metadata(&executable).map_err(|_| Error::Unavailable)?;
        if !metadata.is_file()
            || metadata.permissions().mode() & 0o111 == 0
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(Error::Unavailable);
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or(Error::Invalid)?;
        Ok(Self {
            executable,
            home,
            limits,
        })
    }
    async fn command(&self, args: &[&str], limit: usize) -> Result<Zeroizing<Vec<u8>>> {
        let mut command = Command::new(&self.executable);
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("LANG", "C.UTF-8")
            .env("PATH", "/usr/bin:/bin")
            .args(["-H", "127.0.0.1", "-P", "5037"])
            .args(args)
            .env_remove("ADB_SERVER_SOCKET")
            .env_remove("ANDROID_SERIAL")
            .env_remove("ADB_SERIAL")
            .env_remove("ADB_TRACE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .process_group(0);
        let mut child = command.spawn().map_err(|error| Error::SpawnFailed {
            kind: error.kind(),
            os_code: error.raw_os_error(),
        })?;
        let raw = child
            .id()
            .and_then(|p| i32::try_from(p).ok())
            .filter(|p| *p > 1)
            .and_then(Pid::from_raw)
            .ok_or(Error::Unavailable)?;
        let mut group = ProcessGroup(Some(raw));
        let stdout = child.stdout.take().ok_or(Error::Unavailable)?;
        let stderr = child.stderr.take().ok_or(Error::Unavailable)?;
        let completed = tokio::time::timeout(self.limits.command_timeout, async {
            tokio::try_join!(
                bounded_read(stdout, limit),
                bounded_read(stderr, 4096),
                async { child.wait().await.map_err(|_| Error::CommandFailed) }
            )
        })
        .await;
        match completed {
            Ok(Ok((output, _errors, status))) => {
                group.disarm();
                if status.success() {
                    Ok(output)
                } else {
                    Err(Error::CommandFailed)
                }
            }
            failure => {
                group.terminate();
                let _ = child.start_kill();
                let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
                match failure {
                    Err(_) => Err(Error::Timeout),
                    Ok(Err(e)) => Err(e),
                    Ok(Ok(_)) => unreachable!(),
                }
            }
        }
    }
    pub async fn discover(&self, now: i64) -> Result<Vec<AndroidDeviceObservation>> {
        if now < 0 {
            return Err(Error::Invalid);
        }
        let output = self
            .command(&["devices", "-l"], self.limits.max_text_bytes)
            .await?;
        parse_devices(
            std::str::from_utf8(&output).map_err(|_| Error::InvalidOutput)?,
            now,
        )
    }
    async fn ready(
        &self,
        device: &RegisteredAndroidDevice,
        now: i64,
    ) -> Result<AndroidDeviceObservation> {
        device.validate().map_err(|_| Error::Invalid)?;
        let observed = self
            .discover(now)
            .await?
            .into_iter()
            .find(|d| d.adb_serial == device.adb_serial)
            .ok_or(Error::Disconnected)?;
        match observed.state {
            AndroidDeviceState::Unauthorized => Err(Error::Unauthorized),
            AndroidDeviceState::Error
                if observed.authorization == AndroidAuthorization::UsbPermissionDenied =>
            {
                Err(Error::Unauthorized)
            }
            AndroidDeviceState::Ready if observed.transport == AndroidTransport::Usb => {
                Ok(observed)
            }
            AndroidDeviceState::Ready => Err(Error::PermissionDenied),
            _ => Err(Error::Disconnected),
        }
    }
    pub async fn inspect(
        &self,
        device: &RegisteredAndroidDevice,
        now: i64,
    ) -> Result<AndroidDeviceStatus> {
        device.validate().map_err(|_| Error::Invalid)?;
        if !device
            .capabilities
            .contains(&AndroidCapability::DeviceStatus)
        {
            return Err(Error::PermissionDenied);
        }
        let observed = self
            .discover(now)
            .await?
            .into_iter()
            .find(|d| d.adb_serial == device.adb_serial);
        let mut status = AndroidDeviceStatus {
            device_id: device.device_id.clone(),
            adb_serial: device.adb_serial.clone(),
            display_name: device.display_name.clone(),
            manufacturer: None,
            model: observed.as_ref().and_then(|v| v.model.clone()),
            android_version: None,
            api_level: None,
            state: observed
                .as_ref()
                .map_or(AndroidDeviceState::Disconnected, |d| d.state),
            authorization: observed
                .as_ref()
                .map_or(AndroidAuthorization::Unknown, |d| d.authorization),
            last_seen_at: observed.as_ref().map(|d| d.last_seen_at),
            capabilities: device.capabilities.clone(),
        };
        if observed.as_ref().is_some_and(|d| {
            d.state == AndroidDeviceState::Ready && d.transport == AndroidTransport::Usb
        }) {
            status.manufacturer = Some(self.property(device, "ro.product.manufacturer").await?);
            status.model = Some(self.property(device, "ro.product.model").await?);
            status.android_version = Some(self.property(device, "ro.build.version.release").await?);
            let api = self.property(device, "ro.build.version.sdk").await?;
            status.api_level = Some(
                api.parse::<u32>()
                    .ok()
                    .filter(|n| (16..=100).contains(n))
                    .ok_or(Error::InvalidOutput)?,
            );
        }
        Ok(status)
    }
    async fn property(&self, device: &RegisteredAndroidDevice, name: &str) -> Result<String> {
        let bytes = self
            .command(
                &["-s", &device.adb_serial, "shell", "getprop", name],
                self.limits.max_text_bytes,
            )
            .await?;
        let value = std::str::from_utf8(&bytes)
            .map_err(|_| Error::InvalidOutput)?
            .trim();
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(Error::InvalidOutput);
        }
        Ok(value.to_owned())
    }
    /// Capture only into protected memory. Callers must not persist or relay this as an ordinary file.
    pub async fn capture(
        &self,
        device: &RegisteredAndroidDevice,
        lease: &SessionLease,
        now: i64,
    ) -> Result<SensitiveFrame> {
        lease.authorize_sensitive(device, now)?;
        let started = tokio::time::Instant::now();
        let remaining = Duration::from_millis((lease.status().expires_at - now) as u64);
        tokio::select! {biased;
            _=lease.revoked()=>Err(Error::PermissionDenied),
            _=tokio::time::sleep(remaining)=>Err(Error::PermissionDenied),
            result=async {
                self.ready(device,now).await?;
                let output=self.command(&["-s",&device.adb_serial,"exec-out","screencap","-p"],self.limits.max_screenshot_bytes).await?;
                let captured_at=now.saturating_add(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX));lease.authorize_sensitive(device,captured_at)?;
                SensitiveFrame::from_png(output,captured_at)
            }=>result,
        }
    }
    /// Observe only the first screenshot-byte change. This does not identify SMS, OCR or an OTP.
    pub async fn watch(
        &self,
        device: &RegisteredAndroidDevice,
        lease: &SessionLease,
        request: &AndroidWatchRequest,
        now: i64,
    ) -> Result<Option<SensitiveFrame>> {
        if !request.validate() {
            return Err(Error::Invalid);
        }
        lease.authorize_sensitive(device, now)?;
        if now
            .checked_add((request.duration_seconds * 1000) as i64)
            .is_none_or(|end| end > lease.status().expires_at)
        {
            return Err(Error::PermissionDenied);
        }
        let started = tokio::time::Instant::now();
        let observation = async {
            let baseline = self.capture(device, lease, now).await?;
            let previous: [u8; 32] = Sha256::digest(baseline.bytes()).into();
            drop(baseline);
            loop {
                tokio::select! {biased;_ =lease.revoked()=>return Err(Error::PermissionDenied),_ =tokio::time::sleep(Duration::from_secs(request.poll_seconds))=>{}}
                let sample_now = now.saturating_add(
                    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
                );
                let frame = self.capture(device, lease, sample_now).await?;
                let current: [u8; 32] = Sha256::digest(frame.bytes()).into();
                if current != previous {
                    return Ok(Some(frame));
                }
            }
        };
        match tokio::time::timeout(Duration::from_secs(request.duration_seconds), observation).await
        {
            Ok(result) => result,
            Err(_) => Ok(None),
        }
    }
}

/// Parse only bounded `adb devices -l` metadata; never accept it as a permission grant.
pub fn parse_devices(output: &str, now: i64) -> Result<Vec<AndroidDeviceObservation>> {
    if now < 0 || output.len() > 256 * 1024 {
        return Err(Error::InvalidOutput);
    }
    let mut devices = Vec::new();
    let mut serials = BTreeSet::new();
    let mut header = false;
    for line in output.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line == "List of devices attached" {
            if header {
                return Err(Error::InvalidOutput);
            }
            header = true;
            continue;
        }
        if !header || line.chars().any(|c| c.is_control() && c != '\t') {
            return Err(Error::InvalidOutput);
        }
        let mut fields = line.split_whitespace();
        let serial = fields.next().ok_or(Error::InvalidOutput)?;
        if serial.len() > 128
            || serial.starts_with('-')
            || !serial
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || !serials.insert(serial.to_owned())
        {
            return Err(Error::InvalidOutput);
        }
        let raw_state = fields.next().ok_or(Error::InvalidOutput)?;
        let rest = fields.collect::<Vec<_>>();
        let (state, authorization) = match raw_state {
            "device" => (AndroidDeviceState::Ready, AndroidAuthorization::Authorized),
            "unauthorized" => (
                AndroidDeviceState::Unauthorized,
                AndroidAuthorization::Required,
            ),
            "offline" => (
                AndroidDeviceState::Connecting,
                AndroidAuthorization::Unknown,
            ),
            "no" if rest.first().is_some_and(|v| v.starts_with("permissions")) => (
                AndroidDeviceState::Error,
                AndroidAuthorization::UsbPermissionDenied,
            ),
            _ => (AndroidDeviceState::Error, AndroidAuthorization::Unknown),
        };
        let model = rest
            .iter()
            .find_map(|v| v.strip_prefix("model:"))
            .filter(|v| !v.is_empty() && v.len() <= 128)
            .map(|v| v.replace('_', " "));
        let transport = if serial.starts_with("emulator-") {
            AndroidTransport::Emulator
        } else if serial.contains(':') {
            AndroidTransport::Network
        } else if rest.iter().any(|v| v.starts_with("usb:")) {
            AndroidTransport::Usb
        } else {
            AndroidTransport::Unknown
        };
        devices.push(AndroidDeviceObservation {
            adb_serial: serial.into(),
            model,
            transport,
            state,
            authorization,
            last_seen_at: now,
        });
        if devices.len() > 32 {
            return Err(Error::OutputLimit);
        }
    }
    if !header {
        return Err(Error::InvalidOutput);
    }
    Ok(devices)
}
