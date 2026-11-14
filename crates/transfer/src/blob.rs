//! Bounded opaque object storage. Caller serializes append/checkpoint transactions.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

pub fn private_directory(path: &Path) -> Result<PathBuf> {
    let mut missing = Vec::new();
    let mut cursor = path;
    while !cursor.exists() {
        missing.push(cursor.to_owned());
        cursor = cursor
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
    }
    if !path.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
    }
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && metadata.permissions().mode() & 0o077 == 0,
        "a real private directory with mode 0700 is required"
    );
    for directory in missing.iter().rev() {
        File::open(directory)?.sync_all()?;
        File::open(
            directory
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?
        .sync_all()?;
    }
    Ok(fs::canonicalize(path)?)
}
pub fn read_file(path: &Path) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    ensure!(f.metadata()?.is_file(), "only regular files are supported");
    Ok(f)
}
pub fn new_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?)
}
pub fn digest(path: &Path) -> Result<(u64, String)> {
    digest_limited(path, 4 * 1024 * 1024 * 1024)
}
pub fn digest_limited(path: &Path, limit: u64) -> Result<(u64, String)> {
    let mut file = read_file(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut size = 0;
    ensure!(file.metadata()?.len() <= limit, "file exceeds size limit");
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        ensure!(size <= limit, "file grew beyond size limit");
        hash.update(&buf[..n]);
    }
    Ok((size, format!("{:x}", hash.finalize())))
}
/// Process-scoped advisory lock; persistent lock inode avoids stale-lock recovery races.
pub fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    ensure!(file.metadata()?.is_file(), "invalid cache lock");
    file.try_lock()
        .map_err(|_| anyhow::anyhow!("transfer is already active in another process"))?;
    Ok(file)
}
pub fn object_name(id: &str) -> Result<String> {
    let uuid = uuid::Uuid::parse_str(id)?;
    ensure!(uuid.to_string() == id, "noncanonical transfer identifier");
    Ok(format!("{uuid}.age"))
}

#[derive(Clone)]
pub struct BlobStore {
    root: PathBuf,
}
impl BlobStore {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            root: private_directory(path)?,
        })
    }
    fn path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.root.join(object_name(id)?))
    }
    /// Extra bytes left by a crash before the database checkpoint are discarded.
    /// Exact replay of an acknowledged chunk does not append or change the object.
    pub fn append(&self, id: &str, committed: u64, offset: u64, bytes: &[u8]) -> Result<bool> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= zaturo_core::MAX_CHUNK_BYTES,
            "invalid chunk size"
        );
        let path = self.path(id)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        ensure!(
            file.metadata()?.is_file() && file.metadata()?.len() >= committed,
            "spool checkpoint is inconsistent"
        );
        if offset < committed {
            ensure!(
                offset
                    .checked_add(bytes.len() as u64)
                    .is_some_and(|n| n <= committed),
                "overlapping upload checkpoint"
            );
            let mut previous = vec![0; bytes.len()];
            file.seek(SeekFrom::Start(offset))?;
            file.read_exact(&mut previous)?;
            ensure!(previous == bytes, "replayed chunk differs");
            return Ok(false);
        }
        ensure!(offset == committed, "noncontiguous upload checkpoint");
        if committed == 0 {
            ensure!(
                bytes.starts_with(b"age-encryption.org/v1\n"),
                "only age-encrypted payloads are accepted"
            );
        }
        file.set_len(committed)?;
        file.seek(SeekFrom::Start(committed))?;
        file.write_all(bytes)?;
        file.sync_data()?;
        if committed == 0 {
            File::open(&self.root)?.sync_all()?;
        }
        Ok(true)
    }
    pub fn digest(&self, id: &str, expected: u64) -> Result<String> {
        let path = self.path(id)?;
        ensure!(
            read_file(&path)?.metadata()?.len() == expected,
            "incomplete payload"
        );
        Ok(digest(&path)?.1)
    }
    pub fn read(&self, id: &str, offset: u64, length: usize, expected: u64) -> Result<Vec<u8>> {
        ensure!(
            length > 0 && length <= zaturo_core::MAX_CHUNK_BYTES && offset <= expected,
            "invalid download range"
        );
        let mut file = read_file(&self.path(id)?)?;
        ensure!(
            file.metadata()?.len() == expected,
            "payload is inconsistent"
        );
        file.seek(SeekFrom::Start(offset))?;
        let mut data = vec![0; length.min((expected - offset) as usize)];
        file.read_exact(&mut data)?;
        Ok(data)
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        match fs::remove_file(self.path(id)?) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        };
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}
