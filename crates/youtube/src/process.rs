//! Fixed module invocation with bounded output and process-group ownership.
use crate::{Error, YoutubeClient};
use rustix::process::{Pid, Signal, kill_process_group};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};
use zeroize::Zeroizing;
struct Group(Option<Pid>);
impl Group {
    fn kill(&mut self) {
        if let Some(pid) = self.0.take() {
            let _ = kill_process_group(pid, Signal::KILL);
        }
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        self.kill();
    }
}
async fn read<R: AsyncRead + Unpin>(reader: R, limit: usize) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut bytes = Zeroizing::new(Vec::new());
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error::Rejected)?;
    if bytes.len() > limit {
        Err(Error::OutputLimit)
    } else {
        Ok(bytes)
    }
}
impl YoutubeClient {
    pub(crate) async fn invoke(&self, args: &[String]) -> Result<serde_json::Value, Error> {
        let mut command = Command::new(&self.config.python);
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("PYTHONPATH", &self.project_root)
            .env("LANG", "C.UTF-8")
            .env("PYTHONUNBUFFERED", "1")
            .env(
                "PATH",
                std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
            );
        // Browser/video subprocesses may need the owner's local display/session, never bot credentials.
        for name in [
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
            "XAUTHORITY",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .current_dir(&self.project_root)
            .args(["-m", "tools.ytbot.service"])
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .process_group(0);
        let mut child = command.spawn().map_err(|_| Error::Unavailable)?;
        let pid = child
            .id()
            .and_then(|p| i32::try_from(p).ok())
            .filter(|p| *p > 1)
            .and_then(Pid::from_raw)
            .ok_or(Error::Unavailable)?;
        let mut group = Group(Some(pid));
        let stdout = child.stdout.take().ok_or(Error::Unavailable)?;
        let stderr = child.stderr.take().ok_or(Error::Unavailable)?;
        let outcome = tokio::time::timeout(self.timeout, async {
            tokio::try_join!(read(stdout, 65536), read(stderr, 8192), async {
                child.wait().await.map_err(|_| Error::Rejected)
            })
        })
        .await;
        match outcome {
            Ok(Ok((stdout, _stderr, status))) => {
                group.0 = None;
                if !status.success() {
                    return Err(Error::Rejected);
                }
                serde_json::from_slice(&stdout).map_err(|_| Error::InvalidOutput)
            }
            other => {
                group.kill();
                let _ = child.start_kill();
                let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
                match other {
                    Err(_) => Err(Error::Timeout),
                    Ok(Err(e)) => Err(e),
                    Ok(Ok(_)) => unreachable!(),
                }
            }
        }
    }
}
