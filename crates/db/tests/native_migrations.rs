//! Old native-client schemas migrate transactionally without losing owner records/fences.
use zaturo_db::{HubStore, WorkerJournal};
#[test]
fn hub_five_upgrades_to_current_and_failed_upgrade_rolls_back() {
    for collision in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("hub.db");
        let old = rusqlite::Connection::open(&path).unwrap();
        old.execute_batch(concat!(
            include_str!("../migrations/0001_hub.sql"),
            "\n",
            include_str!("../migrations/0004_hub_transfers.sql"),
            "\n",
            include_str!("../migrations/0005_hub_activity.sql")
        ))
        .unwrap();
        old.execute(
            "INSERT INTO chat_requests VALUES('preserved','{}','{}')",
            [],
        )
        .unwrap();
        if collision {
            old.execute_batch("CREATE TABLE shared_tasks(id TEXT);")
                .unwrap();
        }
        drop(old);
        let migrated = HubStore::open(&path);
        assert_eq!(migrated.is_err(), collision);
        drop(migrated);
        let connection = rusqlite::Connection::open(path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, if collision { 5 } else { 11 });
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM chat_requests WHERE idempotency_key='preserved'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        if !collision {
            assert_eq!(
                connection
                    .query_row("SELECT count(*) FROM app_viewers", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
}
#[test]
fn worker_six_upgrades_to_eight_preserving_uncertain_service_intent() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("worker.db");
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch(concat!(
        include_str!("../migrations/0002_worker.sql"),
        "\n",
        include_str!("../migrations/0003_worker_model_budget.sql"),
        "\n",
        include_str!("../migrations/0006_worker_services.sql")
    ))
    .unwrap();
    let request = zaturo_core::ServiceRequest::YoutubeSubmit {
        channel: "channel".into(),
        account: "account".into(),
        count: 1,
        approved_schedule: true,
    };
    old.execute("INSERT INTO service_operations(operation_id,task_id,request_json,started_at,updated_at) VALUES('preserved','task',?1,1,1)",[serde_json::to_string(&request).unwrap()]).unwrap();
    drop(old);
    let journal = WorkerJournal::open(&path).unwrap();
    assert!(matches!(
        journal
            .begin_service("preserved", "task", &request, 2)
            .unwrap(),
        zaturo_db::ServiceIntent::Reconcile
    ));
    drop(journal);
    let reopened = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        reopened
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        8
    );
}

#[test]
fn hub_seven_preserves_shared_records_and_rolls_back_a_computer_table_collision() {
    for collision in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hub.db");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(concat!(
            include_str!("../migrations/0001_hub.sql"),
            "\n",
            include_str!("../migrations/0004_hub_transfers.sql"),
            "\n",
            include_str!("../migrations/0005_hub_activity.sql"),
            "\n",
            include_str!("../migrations/0007_hub_shared.sql")
        ))
        .unwrap();
        c.execute("INSERT INTO app_viewers(app_id,recipient,updated_at) VALUES('preserved','age1synthetic',1)",[]).unwrap();
        if collision {
            c.execute_batch("CREATE TABLE computer_steps(id TEXT);")
                .unwrap();
        }
        drop(c);
        assert_eq!(HubStore::open(&path).is_err(), collision);
        let c = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            if collision { 7 } else { 11 }
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM app_viewers WHERE app_id='preserved'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}

#[test]
fn hub_nine_upgrades_to_current_and_rolls_back_on_ml_table_collision() {
    for collision in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hub.db");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(concat!(
            include_str!("../migrations/0001_hub.sql"),
            "\n",
            include_str!("../migrations/0004_hub_transfers.sql"),
            "\n",
            include_str!("../migrations/0005_hub_activity.sql"),
            "\n",
            include_str!("../migrations/0007_hub_shared.sql"),
            "\n",
            include_str!("../migrations/0009_hub_computer.sql")
        ))
        .unwrap();
        c.execute(
            "INSERT INTO workers(id,body) VALUES('preserved','{\"id\":\"preserved\",\"capabilities\":[],\"last_seen_at\":1}')",
            [],
        )
        .unwrap();
        if collision {
            c.execute_batch("CREATE TABLE ml_products(product_id TEXT);")
                .unwrap();
        }
        drop(c);
        assert_eq!(HubStore::open(&path).is_err(), collision);
        let c = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            if collision { 9 } else { 11 }
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM workers WHERE id='preserved'",
                [],
                |r| { r.get::<_, i64>(0) }
            )
            .unwrap(),
            1
        );
    }
}

#[test]
fn hub_ten_returns_unreviewed_telegram_queue_to_owner_approval() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hub.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(concat!(
        include_str!("../migrations/0001_hub.sql"),
        "\n",
        include_str!("../migrations/0004_hub_transfers.sql"),
        "\n",
        include_str!("../migrations/0005_hub_activity.sql"),
        "\n",
        include_str!("../migrations/0007_hub_shared.sql"),
        "\n",
        include_str!("../migrations/0009_hub_computer.sql"),
        "\n",
        include_str!("../migrations/0010_hub_mercadolivre_commerce.sql")
    ))
    .unwrap();
    let sha = "a".repeat(64);
    for batch in ["b1", "b2"] {
        c.execute(
            "INSERT INTO ml_ingest_batches VALUES(?1,?1,'w','q','@procurali',1,1,1,1,?2,'{}')",
            rusqlite::params![batch, sha],
        )
        .unwrap();
    }
    c.execute(
        "INSERT INTO ml_products VALUES('MLB1','slug','t','b',NULL,'v','c','https://x','s',NULL,NULL,NULL,NULL,'d','d','r','f','[]','[]','[]','[]','approved','b2',1,1)",
        [],
    )
    .unwrap();
    for (offer, batch, current) in [("old", "b1", 0), ("new", "b2", 1)] {
        c.execute(
            "INSERT INTO ml_offers VALUES(?1,'MLB1',?2,'https://meli.la/x','id','txt',NULL,NULL,0,100,100,NULL,'BRL',NULL,NULL,'p','s','f','InStock',NULL,1,30,?3,?4,1)",
            rusqlite::params![offer, batch, sha, current],
        )
        .unwrap();
    }
    for (delivery, offer, status, channel) in [
        ("d-old", "old", "queued", "@procurali"),
        ("d-new", "new", "queued", "@procurali"),
        ("d-done", "old", "published", "@other"),
    ] {
        c.execute(
            "INSERT INTO ml_telegram_deliveries VALUES(?1,'MLB1',?2,?4,'m',NULL,NULL,?3,NULL,NULL,1,1,NULL)",
            rusqlite::params![delivery, offer, status, channel],
        )
        .unwrap();
    }
    drop(c);
    drop(HubStore::open(&path).unwrap());
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        11
    );
    let status = |id: &str| -> (String, Option<String>) {
        c.query_row(
            "SELECT status, error_code FROM ml_telegram_deliveries WHERE delivery_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    };
    assert_eq!(status("d-new"), ("awaiting_approval".into(), None));
    assert_eq!(
        status("d-old"),
        ("skipped".into(), Some("superseded".into()))
    );
    assert_eq!(status("d-done"), ("published".into(), None));
}
