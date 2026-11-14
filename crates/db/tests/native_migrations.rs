//! Old native-client schemas migrate transactionally without losing owner records/fences.
use zaturo_db::{HubStore, WorkerJournal};
#[test]
fn hub_five_upgrades_to_nine_and_failed_upgrade_rolls_back() {
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
        assert_eq!(version, if collision { 5 } else { 9 });
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
            if collision { 7 } else { 9 }
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
