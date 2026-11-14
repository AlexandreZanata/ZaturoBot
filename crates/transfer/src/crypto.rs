//! Standard streaming age encryption; no decryption identity is sent to the hub.
use crate::blob::{digest, digest_limited, new_file, read_file};
use age::secrecy::ExposeSecret;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    filename: String,
    size_bytes: u64,
    sha256: String,
}

pub fn generate_identity(path: &Path) -> Result<String> {
    let key = age::x25519::Identity::generate();
    let public = key.to_public().to_string();
    let mut file = new_file(path)?;
    file.write_all(key.to_string().expose_secret().as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(public)
}
pub fn public_identity(path: &Path) -> Result<String> {
    Ok(load_identity(path)?.to_public().to_string())
}
fn load_identity(path: &Path) -> Result<age::x25519::Identity> {
    let mut key = String::new();
    read_file(path)?.take(512).read_to_string(&mut key)?;
    key.trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid transfer decryption identity; value is not logged"))
}
pub fn valid_recipient(value: &str) -> bool {
    value.parse::<age::x25519::Recipient>().is_ok()
}

pub fn encrypt(
    source: &Path,
    output: &Path,
    recipients: &[String],
    limit: u64,
) -> Result<(u64, String, String)> {
    let filename = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("UTF-8 filename required"))?
        .to_owned();
    ensure!(zaturo_core::safe_filename(&filename), "invalid filename");
    let (size_bytes, sha256) = digest_limited(source, limit)?;
    let keys = recipients
        .iter()
        .map(|s| {
            s.parse::<age::x25519::Recipient>()
                .map_err(|_| anyhow::anyhow!("invalid recipient encryption key"))
        })
        .collect::<Result<Vec<_>>>()?;
    let encryptor = age::Encryptor::with_recipients(keys.iter().map(|k| k as &dyn age::Recipient))?;
    let mut out = new_file(output)?;
    {
        let mut writer = encryptor.wrap_output(&mut out)?;
        serde_json::to_writer(
            &mut writer,
            &Envelope {
                filename: filename.clone(),
                size_bytes,
                sha256,
            },
        )?;
        writer.write_all(b"\n")?;
        let count = io::copy(
            &mut read_file(source)?.take(size_bytes.saturating_add(1)),
            &mut writer,
        )?;
        ensure!(count == size_bytes, "source file changed during encryption");
        writer.finish()?;
    }
    out.sync_all()?;
    let (encrypted_size, encrypted_hash) = digest(output)?;
    ensure!(
        encrypted_size <= limit,
        "encrypted file exceeds transfer limit"
    );
    Ok((encrypted_size, encrypted_hash, filename))
}

/// Decrypt into a private scratch file, validate the plaintext, then publish without overwrite.
pub fn decrypt(
    ciphertext: &Path,
    identity: &Path,
    scratch: &Path,
    destination: &Path,
    expected_filename: &str,
    limit: u64,
) -> Result<()> {
    ensure!(
        scratch != destination,
        "plaintext staging must be separate from destination"
    );
    let key = load_identity(identity)?;
    let decryptor = age::Decryptor::new(BufReader::new(read_file(ciphertext)?))?;
    let mut reader =
        BufReader::new(decryptor.decrypt(std::iter::once(&key as &dyn age::Identity))?);
    let mut header = Vec::new();
    Read::by_ref(&mut reader)
        .take(4097)
        .read_until(b'\n', &mut header)?;
    ensure!(
        header.len() <= 4096 && header.last() == Some(&b'\n'),
        "invalid encrypted metadata"
    );
    let meta: Envelope = serde_json::from_slice(&header)
        .map_err(|_| anyhow::anyhow!("invalid encrypted metadata"))?;
    ensure!(
        meta.filename == expected_filename
            && zaturo_core::safe_filename(&meta.filename)
            && meta.size_bytes <= limit
            && zaturo_core::valid_sha256(&meta.sha256),
        "encrypted metadata does not match transfer"
    );
    if scratch.exists() {
        ensure!(
            fs::symlink_metadata(scratch)?.is_file()
                && !fs::symlink_metadata(scratch)?.file_type().is_symlink(),
            "invalid scratch file"
        );
        fs::remove_file(scratch)?;
    }
    let mut out = new_file(scratch)?;
    let copied = io::copy(
        &mut reader.take(meta.size_bytes.saturating_add(1)),
        &mut out,
    )?;
    ensure!(copied == meta.size_bytes, "invalid plaintext length");
    out.sync_all()?;
    ensure!(
        digest(scratch)? == (meta.size_bytes, meta.sha256.clone()),
        "plaintext checksum mismatch"
    );
    if destination.symlink_metadata().is_ok() {
        ensure!(
            read_file(destination)?.metadata()?.len() == meta.size_bytes
                && digest(destination)? == (meta.size_bytes, meta.sha256),
            "destination already contains a different file"
        );
    } else {
        fs::hard_link(scratch, destination)?;
    }
    fs::remove_file(scratch)?;
    Ok(())
}

/// Sensitive PNG transport: standard age in memory, deliberately no file envelope or disk path.
pub fn encrypt_private_image(png: &[u8], recipient: &str) -> Result<Vec<u8>> {
    ensure!(
        png.len() <= 8 * 1024 * 1024 && png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "invalid private PNG"
    );
    let key = recipient
        .parse::<age::x25519::Recipient>()
        .map_err(|_| anyhow::anyhow!("invalid private recipient"))?;
    let encryptor = age::Encryptor::with_recipients(std::iter::once(&key as &dyn age::Recipient))?;
    let mut out = Vec::new();
    {
        let mut writer = encryptor.wrap_output(&mut out)?;
        writer.write_all(png)?;
        writer.finish()?;
    }
    Ok(out)
}
pub fn decrypt_private_image(
    ciphertext: &[u8],
    identity: &Path,
) -> Result<zeroize::Zeroizing<Vec<u8>>> {
    ensure!(
        ciphertext.len() <= 8 * 1024 * 1024 + 65536,
        "private image exceeds limit"
    );
    let key = load_identity(identity)?;
    let decryptor = age::Decryptor::new(ciphertext)?;
    let mut reader = decryptor.decrypt(std::iter::once(&key as &dyn age::Identity))?;
    let mut plain = zeroize::Zeroizing::new(Vec::new());
    Read::by_ref(&mut reader)
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut plain)?;
    ensure!(
        plain.len() <= 8 * 1024 * 1024 && plain.starts_with(b"\x89PNG\r\n\x1a\n"),
        "invalid private PNG"
    );
    Ok(plain)
}
