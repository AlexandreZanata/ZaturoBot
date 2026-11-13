//! Publication guard over Git-visible files and reachable history, never ignored secrets.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

const MAX_LIST_BYTES: usize = 8 * 1024 * 1024;
const MAX_OBJECT_BYTES: usize = 8 * 1024 * 1024;
const MAX_HISTORY_OBJECTS: usize = 100_000;

pub fn check(root: &Path) -> Result<()> {
    let files = git_output(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    for path in files
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let relative = std::str::from_utf8(path).context("Git-visible filenames must be UTF-8")?;
        validate_path(relative)?;
        let path = root.join(relative);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_symlink() {
            validate_path(&fs::read_link(path)?.to_string_lossy())?;
            continue;
        }
        ensure!(
            metadata.len() <= MAX_OBJECT_BYTES as u64,
            "publication check: oversized file {relative}"
        );
        let mut body = Vec::new();
        fs::File::open(path)?
            .take((MAX_OBJECT_BYTES + 1) as u64)
            .read_to_end(&mut body)?;
        ensure!(
            body.len() <= MAX_OBJECT_BYTES,
            "publication check: oversized file {relative}"
        );
        scan(relative, &body)?;
    }

    let mut objects = BTreeSet::new();
    let index = git_output(root, &["ls-files", "--stage", "-z"])?;
    for entry in index
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let entry = std::str::from_utf8(entry).context("invalid Git index record")?;
        let (metadata, path) = entry.split_once('\t').context("invalid Git index record")?;
        validate_path(path)?;
        let oid = metadata
            .split_whitespace()
            .nth(1)
            .context("missing Git object ID")?;
        objects.insert((oid.to_owned(), path.to_owned()));
    }
    let history = git_output(root, &["rev-list", "--objects", "--all"])?;
    for line in std::str::from_utf8(&history)?.lines() {
        let (oid, path) = line.split_once(' ').unwrap_or((line, "<history>"));
        validate_path(path)?;
        objects.insert((oid.to_owned(), path.to_owned()));
        ensure!(
            objects.len() <= MAX_HISTORY_OBJECTS,
            "publication check: history exceeds bounded object limit"
        );
    }
    scan_git_objects(root, &objects)
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .context("missing Git output")?
        .take((MAX_LIST_BYTES + 1) as u64)
        .read_to_end(&mut output)?;
    if output.len() > MAX_LIST_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("publication check: Git inventory exceeds bounded size limit");
    }
    ensure!(
        child.wait()?.success(),
        "publication check: Git inventory failed"
    );
    Ok(output)
}

fn scan_git_objects(root: &Path, objects: &BTreeSet<(String, String)>) -> Result<()> {
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut input = child.stdin.take().context("missing Git object input")?;
    let mut output = BufReader::new(child.stdout.take().context("missing Git object output")?);
    let result = (|| -> Result<()> {
        for (oid, path) in objects {
            ensure!(
                oid.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "invalid Git object ID"
            );
            writeln!(input, "{oid}")?;
            input.flush()?;
            let mut header = String::new();
            output.read_line(&mut header)?;
            let fields: Vec<_> = header.split_whitespace().collect();
            ensure!(fields.len() == 3, "publication check: missing Git object");
            let size: usize = fields[2].parse()?;
            ensure!(
                size <= MAX_OBJECT_BYTES,
                "publication check: oversized historical object at {path}"
            );
            let mut body = vec![0; size];
            output.read_exact(&mut body)?;
            let mut newline = [0];
            output.read_exact(&mut newline)?;
            ensure!(newline == *b"\n", "invalid Git object boundary");
            if matches!(fields[1], "blob" | "commit" | "tag") {
                scan(path, &body)?;
            }
        }
        Ok(())
    })();
    drop(input);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    result?;
    ensure!(
        status.success(),
        "publication check: Git object inspection failed"
    );
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    let path = path.replace('\\', "/");
    let components: Vec<_> = path.split('/').collect();
    let name = components.last().copied().unwrap_or("");
    let private_directory = components.iter().any(|part| {
        matches!(
            *part,
            ".secrets" | ".ssh" | "target" | "artifacts" | "workspaces"
        )
    });
    let private_file = name == ".env"
        || name.ends_with(".env")
        || (name.starts_with(".env.") && name != ".env.example")
        || name.ends_with(".local.toml")
        || name.ends_with(".key")
        || name.ends_with(".key.pub")
        || name.ends_with(".pem")
        || name.ends_with(".pfx")
        || name.ends_with(".p12")
        || name.ends_with(".age")
        || name.ends_with(".gpg")
        || name == "id_rsa"
        || name == "id_ed25519"
        || name.ends_with(".db")
        || name.ends_with(".db-wal")
        || name.ends_with(".db-shm");
    ensure!(
        !private_directory && !private_file,
        "publication check: private/runtime path must not be tracked: {path}"
    );
    Ok(())
}

fn scan(path: &str, bytes: &[u8]) -> Result<()> {
    for kind in ["", "RSA ", "OPENSSH ", "EC ", "DSA ", "ENCRYPTED "] {
        let marker = ["-----BEGIN ", kind, "PRIVATE KEY-----"].concat();
        ensure!(
            !contains(bytes, marker.as_bytes()),
            "publication check: private-key material at {path}"
        );
    }
    let putty = ["PuTTY-", "User-Key-File-"].concat();
    ensure!(
        !contains(bytes, putty.as_bytes()),
        "publication check: private-key material at {path}"
    );
    let pgp = ["-----BEGIN PGP ", "PRIVATE KEY BLOCK-----"].concat();
    ensure!(
        !contains(bytes, pgp.as_bytes()),
        "publication check: private-key material at {path}"
    );
    for (prefix, minimum) in [
        ("oc_sk_", 24),
        ("ghp_", 30),
        ("github_pat_", 32),
        ("sk-proj-", 32),
        ("sk-ant-", 32),
        ("sk-", 32),
        ("sk_live_", 24),
        ("xoxb-", 24),
        ("xoxp-", 24),
        ("AIza", 35),
        ("AKIA", 20),
        ("AGE-SECRET-KEY-", 32),
    ] {
        ensure!(
            !live_prefix(bytes, prefix.as_bytes(), minimum),
            "publication check: credential-shaped literal at {path}"
        );
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        for line in text.lines() {
            let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if !key.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                continue;
            }
            if !["_TOKEN", "_SECRET", "_PASSWORD", "_API_KEY"]
                .iter()
                .any(|suffix| key.ends_with(suffix))
            {
                continue;
            }
            let value = value.trim().trim_matches(['\'', '"']);
            let synthetic = value.is_empty()
                || [
                    "replace-",
                    "synthetic-",
                    "test-",
                    "example-",
                    "YOUR_",
                    "$",
                    "<",
                ]
                .iter()
                .any(|prefix| value.starts_with(prefix));
            ensure!(
                synthetic,
                "publication check: credential assignment at {path}"
            );
        }
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|part| part == needle)
}

fn live_prefix(bytes: &[u8], prefix: &[u8], minimum: usize) -> bool {
    bytes
        .windows(prefix.len())
        .enumerate()
        .any(|(index, candidate)| {
            candidate == prefix
                && (index == 0 || !token_character(bytes[index - 1]))
                && bytes[index..]
                    .iter()
                    .take_while(|byte| token_character(**byte))
                    .count()
                    >= minimum
        })
}

fn token_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_paths_are_rejected_without_reading_their_contents() {
        for path in [
            ".env",
            ".secrets/ssh/key",
            "nested/.env.production",
            "config/worker.env",
            "backup.age",
            "ssh-key-date.key.pub",
            "config/worker.local.toml",
            "data/worker.db",
            "../.ssh/id_ed25519",
        ] {
            assert!(validate_path(path).is_err());
        }
        for path in [
            ".env.example",
            "crates/security/src/lib.rs",
            "docs/SECURITY.md",
        ] {
            validate_path(path).unwrap();
        }
    }
    #[test]
    fn known_secret_markers_fail_without_echoing_the_value() {
        let marker = ["-----BEGIN ", "OPENSSH ", "PRIVATE KEY-----"].concat();
        let error = scan("fixture.txt", marker.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(!error.contains(&marker));
        let token = ["oc_sk_", &"x".repeat(48)].concat();
        let error = scan("fixture.txt", token.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(!error.contains(&token));
        let assignment = ["ZATURO_OWNER_TOKEN=", &"a".repeat(64)].concat();
        assert!(scan("fixture.txt", assignment.as_bytes()).is_err());
        let legacy = ["sk-", &"x".repeat(48)].concat();
        assert!(scan("fixture.txt", legacy.as_bytes()).is_err());
        scan("fixture.txt", format!("task-{}", "x".repeat(48)).as_bytes()).unwrap();
        scan(
            "fixture.txt",
            b"ZATURO_OWNER_TOKEN=replace-with-a-local-token\nOPENCODE_GO_API_KEY=\n",
        )
        .unwrap();
    }
    #[test]
    fn staged_and_historical_secrets_are_detected_even_after_worktree_cleanup() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap();
            assert!(output.status.success(), "Git test fixture command failed");
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "synthetic@example.invalid"]);
        git(&["config", "user.name", "Synthetic Test"]);
        fs::write(root.join("safe.txt"), "safe initial content").unwrap();
        git(&["add", "safe.txt"]);
        git(&["commit", "-qm", "Create synthetic fixture"]);
        check(root).unwrap();
        let token = ["oc_sk_", &"x".repeat(48)].concat();
        fs::write(root.join("safe.txt"), &token).unwrap();
        git(&["add", "safe.txt"]);
        fs::write(root.join("safe.txt"), "safe worktree").unwrap();
        assert!(check(root).is_err());
        git(&["commit", "-qm", "Introduce synthetic credential fixture"]);
        git(&["add", "safe.txt"]);
        git(&["commit", "-qm", "Remove synthetic credential fixture"]);
        assert!(check(root).is_err());
    }
    #[test]
    fn ignored_private_files_are_never_read() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap();
        fs::write(root.join(".gitignore"), ".secrets/\n.env\n").unwrap();
        fs::create_dir(root.join(".secrets")).unwrap();
        fs::write(
            root.join(".secrets/key"),
            ["-----BEGIN ", "PRIVATE KEY-----"].concat(),
        )
        .unwrap();
        fs::write(
            root.join(".env"),
            "ZATURO_OWNER_TOKEN=actual-private-fixture",
        )
        .unwrap();
        check(root).unwrap();
    }

    #[test]
    fn credentials_in_commit_messages_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "synthetic@example.invalid"],
            vec!["config", "user.name", "Synthetic Test"],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let token = ["oc_sk_", &"x".repeat(48)].concat();
        let output = Command::new("git")
            .args(["commit", "--allow-empty", "-qm", &token])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        let message = check(root).unwrap_err().to_string();
        assert!(!message.contains(&token));
    }
}
