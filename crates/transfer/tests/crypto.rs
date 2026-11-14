use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    os::unix::fs::symlink,
};
use zaturo_transfer::{
    blob::{self, BlobStore},
    crypto,
};
#[test]
fn encryption_authenticates_keys_content_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let first = crypto::generate_identity(&p.join("first")).unwrap();
    let second = crypto::generate_identity(&p.join("second")).unwrap();
    fs::write(p.join("report.txt"), b"private file content").unwrap();
    crypto::encrypt(&p.join("report.txt"), &p.join("file.age"), &[first], 4096).unwrap();
    assert!(
        !fs::read(p.join("file.age"))
            .unwrap()
            .windows(20)
            .any(|b| b == b"private file content")
    );
    assert!(
        crypto::decrypt(
            &p.join("file.age"),
            &p.join("second"),
            &p.join("scratch"),
            &p.join("out"),
            "report.txt",
            4096
        )
        .is_err()
    );
    assert!(
        crypto::decrypt(
            &p.join("file.age"),
            &p.join("first"),
            &p.join("scratch"),
            &p.join("out"),
            "../report.txt",
            4096
        )
        .is_err()
    );
    crypto::decrypt(
        &p.join("file.age"),
        &p.join("first"),
        &p.join("scratch"),
        &p.join("out"),
        "report.txt",
        4096,
    )
    .unwrap();
    crypto::decrypt(
        &p.join("file.age"),
        &p.join("first"),
        &p.join("scratch"),
        &p.join("out"),
        "report.txt",
        4096,
    )
    .unwrap();
    fs::write(p.join("out"), b"keep this existing file").unwrap();
    assert!(
        crypto::decrypt(
            &p.join("file.age"),
            &p.join("first"),
            &p.join("scratch"),
            &p.join("out"),
            "report.txt",
            4096
        )
        .is_err()
    );
    assert_eq!(fs::read(p.join("out")).unwrap(), b"keep this existing file");
    let mut damaged = fs::read(p.join("file.age")).unwrap();
    let index = damaged.len() - 1;
    damaged[index] ^= 1;
    fs::write(p.join("tampered.age"), damaged).unwrap();
    assert!(
        crypto::decrypt(
            &p.join("tampered.age"),
            &p.join("first"),
            &p.join("scratch"),
            &p.join("unpublished"),
            "report.txt",
            4096
        )
        .is_err()
    );
    assert!(!p.join("unpublished").exists());
    fs::write(p.join("empty"), b"").unwrap();
    crypto::encrypt(&p.join("empty"), &p.join("empty.age"), &[second], 4096).unwrap();
    crypto::decrypt(
        &p.join("empty.age"),
        &p.join("second"),
        &p.join("scratch"),
        &p.join("empty.out"),
        "empty",
        4096,
    )
    .unwrap();
    assert_eq!(fs::metadata(p.join("empty.out")).unwrap().len(), 0);
}
#[test]
fn spool_recovers_orphan_bytes_rejects_replays_and_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::open(&dir.path().join("spool")).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let first = b"age-encryption.org/v1\nfirst bytes";
    assert!(store.append(&id, 0, 0, first).unwrap());
    assert!(!store.append(&id, first.len() as u64, 0, first).unwrap());
    assert!(
        store
            .append(&id, first.len() as u64, 0, b"wrong bytes")
            .is_err()
    );
    let path = dir
        .path()
        .join("spool")
        .join(blob::object_name(&id).unwrap());
    let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::End(0)).unwrap();
    file.write_all(b"uncommitted bytes after crash").unwrap();
    drop(file);
    assert!(
        store
            .append(&id, first.len() as u64, first.len() as u64, b"end")
            .unwrap()
    );
    assert_eq!(fs::read(path).unwrap(), [first.as_slice(), b"end"].concat());
    assert!(
        store
            .read(
                &id,
                0,
                zaturo_core::MAX_CHUNK_BYTES + 1,
                first.len() as u64 + 3
            )
            .is_err()
    );
    assert!(store.append("../secret", 0, 0, first).is_err());
    let other = uuid::Uuid::new_v4().to_string();
    let real = dir.path().join("real");
    fs::write(&real, b"must survive").unwrap();
    symlink(
        &real,
        dir.path()
            .join("spool")
            .join(blob::object_name(&other).unwrap()),
    )
    .unwrap();
    assert!(store.append(&other, 0, 0, first).is_err());
    assert_eq!(fs::read(real).unwrap(), b"must survive");
    let fifo = dir.path().join("pipe");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        blob::read_file(&fifo).is_err(),
        "a FIFO must fail immediately rather than waiting for a writer"
    );
    let lock_path = dir.path().join("transfer.lock");
    let lock = blob::lock(&lock_path).unwrap();
    assert!(blob::lock(&lock_path).is_err());
    drop(lock);
    assert!(blob::lock(&lock_path).is_ok());
}
