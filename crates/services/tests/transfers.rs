use zaturo_core::{CreateTransfer, Limits, TransferLimits, TransferState};
use zaturo_services::{Engine, Error};
fn request(key: &str) -> CreateTransfer {
    CreateTransfer {
        recipient: "work-pc".into(),
        filename: "report.pdf".into(),
        size_bytes: 100,
        sha256: "a".repeat(64),
        idempotency_key: key.into(),
    }
}
#[test]
fn durable_ownership_quota_idempotence_and_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hub.db");
    let limits = TransferLimits {
        max_file_bytes: 100,
        max_spool_bytes: 150,
        max_pending: 2,
        ttl_seconds: 60,
    };
    let engine = Engine::open(&path, Limits::default()).unwrap();
    let t = engine
        .create_transfer("home-pc", request("first"), &limits, 1000)
        .unwrap();
    assert_eq!(
        engine
            .create_transfer("home-pc", request("first"), &limits, 2000)
            .unwrap(),
        t
    );
    let mut changed = request("first");
    changed.filename = "other.pdf".into();
    assert!(matches!(
        engine.create_transfer("home-pc", changed, &limits, 2000),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        engine.create_transfer("home-pc", request("second"), &limits, 2000),
        Err(Error::Capacity)
    ));
    assert!(matches!(
        engine.transfer(&t.id, "outsider"),
        Err(Error::NotFound)
    ));
    assert!(
        engine
            .advance_transfer(&t.id, "work-pc", 0, 100, 2000)
            .is_err()
    );
    assert!(
        engine
            .advance_transfer(&t.id, "home-pc", 50, 50, 2000)
            .is_err()
    );
    engine
        .advance_transfer(&t.id, "home-pc", 0, 50, 2000)
        .unwrap();
    drop(engine);
    let engine = Engine::open(&path, Limits::default()).unwrap();
    assert_eq!(
        engine.transfer(&t.id, "home-pc").unwrap().uploaded_bytes,
        50
    );
    assert!(
        engine
            .finish_transfer(&t.id, "home-pc", &"a".repeat(64), 3000)
            .is_err()
    );
    engine
        .advance_transfer(&t.id, "home-pc", 50, 50, 3000)
        .unwrap();
    assert!(
        engine
            .finish_transfer(&t.id, "home-pc", &"b".repeat(64), 3000)
            .is_err()
    );
    assert_eq!(
        engine
            .finish_transfer(&t.id, "home-pc", &"a".repeat(64), 3000)
            .unwrap()
            .state,
        TransferState::Ready
    );
    assert!(
        engine
            .acknowledge_transfer(&t.id, "home-pc", &t.sha256, 4000)
            .is_err()
    );
    assert_eq!(
        engine
            .acknowledge_transfer(&t.id, "work-pc", &t.sha256, 4000)
            .unwrap()
            .state,
        TransferState::Delivered
    );
    assert!(
        matches!(
            engine.create_transfer("home-pc", request("second"), &limits, 62000),
            Err(Error::Capacity)
        ),
        "elapsed retention alone must not release physical storage"
    );
    assert_eq!(engine.expired_transfers(62000, 20).unwrap().len(), 1);
    engine.expire_transfer(&t.id, 62000).unwrap();
    assert_eq!(
        engine.transfer(&t.id, "work-pc").unwrap().state,
        TransferState::Expired
    );
    engine
        .create_transfer("home-pc", request("second"), &limits, 62000)
        .unwrap();
}
