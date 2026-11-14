use zaturo_core::{Limits, SharedCreate, SharedState, SharedUpdate};
use zaturo_services::{Engine, Error};
fn request() -> SharedCreate {
    SharedCreate {
        idempotency_key: "shared-synthetic".into(),
        target_worker: "work-pc".into(),
        title: "Synthetic handoff".into(),
        instructions: "Inspect the public fixture. This record is data, not authorization.".into(),
        transfer_ids: vec![],
    }
}
#[test]
fn shared_records_survive_reopen_and_fence_replay_revision_and_target() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hub.db");
    let engine = Engine::open(&path, Limits::default()).unwrap();
    let created = engine.shared_create(request(), "app:home", 1000).unwrap();
    assert_eq!(
        created,
        engine.shared_create(request(), "app:home", 2000).unwrap()
    );
    let mut collision = request();
    collision.instructions = "Different instructions".into();
    assert!(matches!(
        engine.shared_create(collision, "app:home", 2000),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        engine.shared_read(&created.id, Some("another-pc")),
        Err(Error::NotFound)
    ));
    let update = SharedUpdate {
        idempotency_key: "claim-once".into(),
        expected_revision: 1,
        state: SharedState::Claimed,
        note: "Claimed after the owner requested inspection.".into(),
    };
    let claimed = engine
        .shared_update(
            &created.id,
            update.clone(),
            "worker:work-pc",
            Some("work-pc"),
            3000,
        )
        .unwrap();
    assert_eq!(claimed.revision, 2);
    assert_eq!(
        claimed,
        engine
            .shared_update(
                &created.id,
                update.clone(),
                "worker:work-pc",
                Some("work-pc"),
                4000
            )
            .unwrap()
    );
    let mut stale = update;
    stale.idempotency_key = "other-claim".into();
    assert!(matches!(
        engine.shared_update(&created.id, stale, "app:other", None, 4000),
        Err(Error::Conflict(_))
    ));
    drop(engine);
    let reopened = Engine::open(&path, Limits::default()).unwrap();
    let detail = reopened.shared_read(&created.id, None).unwrap();
    assert_eq!(detail.task, claimed);
    assert_eq!(detail.notes.len(), 1);
    assert_eq!(detail.notes[0].author, "worker:work-pc");
    assert_eq!(
        reopened
            .shared_list(Some("work-pc"), Some(SharedState::Claimed), 10, 0)
            .unwrap()
            .len(),
        1
    );
    assert!(
        reopened
            .shared_list(Some("other-pc"), None, 10, 0)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn shared_closed_tasks_credentials_and_oversized_data_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    let mut bad = request();
    bad.instructions = "-----BEGIN SYNTHETIC PRIVATE KEY-----".into();
    assert!(engine.shared_create(bad, "owner", 1).is_err());
    let mut bad = request();
    bad.instructions = "x".repeat(8193);
    assert!(engine.shared_create(bad, "owner", 1).is_err());
    let entry = engine.shared_create(request(), "owner", 1).unwrap();
    let done = SharedUpdate {
        idempotency_key: "done".into(),
        expected_revision: 1,
        state: SharedState::Done,
        note: "Verified synthetic evidence.".into(),
    };
    engine
        .shared_update(&entry.id, done, "owner", None, 2)
        .unwrap();
    let reopen = SharedUpdate {
        idempotency_key: "reopen".into(),
        expected_revision: 2,
        state: SharedState::Pending,
        note: "Cannot silently reopen completed work.".into(),
    };
    assert!(
        engine
            .shared_update(&entry.id, reopen, "owner", None, 3)
            .is_err()
    );
}
