//! Only already-received, non-sensitive PNGs inside a private transfer inbox are previewed.
use anyhow::{Result, ensure};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
pub fn received_png(inbox: &Path, id: &str) -> Result<PathBuf> {
    let uuid = uuid::Uuid::parse_str(id)?;
    ensure!(uuid.to_string() == id, "canonical artifact UUID required");
    let root = fs::canonicalize(inbox)?;
    let directory = root.join(id);
    let meta = fs::symlink_metadata(&directory)?;
    ensure!(
        meta.is_dir() && !meta.file_type().is_symlink() && meta.permissions().mode() & 0o077 == 0,
        "private received artifact directory required"
    );
    let paths = fs::read_dir(&directory)?
        .take(3)
        .map(|r| r.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(paths.len() == 1, "artifact must contain one received file");
    let path = &paths[0];
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    ensure!(
        meta.is_file() && meta.len() <= 8 * 1024 * 1024,
        "PNG preview exceeds limit"
    );
    let mut header = [0; 24];
    file.read_exact(&mut header)?;
    ensure!(
        &header[..8] == b"\x89PNG\r\n\x1a\n" && &header[12..16] == b"IHDR",
        "only PNG previews are supported"
    );
    let w = u32::from_be_bytes(header[16..20].try_into()?);
    let h = u32::from_be_bytes(header[20..24].try_into()?);
    ensure!(
        (1..=4096).contains(&w) && (1..=4096).contains(&h),
        "PNG dimensions exceed preview limit"
    );
    Ok(path.clone())
}
