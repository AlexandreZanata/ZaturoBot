//! Durable service fences and fair polling with synthetic operations only.
use zaturo_core::{ServiceRequest, ServiceResult};
use zaturo_db::{Error, ServiceIntent, WorkerJournal};

fn submit() -> ServiceRequest {
    ServiceRequest::YoutubeSubmit {
        channel: "synthetic-channel".into(),
        account: "synthetic-account".into(),
        count: 1,
        approved_schedule: true,
    }
}
fn result(operation: &str, request: &ServiceRequest, outcome: &str) -> ServiceResult {
    ServiceResult {
        operation_id: operation.into(),
        action: request.name().into(),
        outcome: outcome.into(),
        summary: "Synthetic durable service evidence".into(),
        data: serde_json::json!({"state":outcome}),
    }
}
fn accepted(journal: &WorkerJournal, operation: &str, now: i64) {
    let request = submit();
    assert_eq!(
        journal
            .begin_service(operation, &format!("task-{operation}"), &request, now)
            .unwrap(),
        ServiceIntent::Execute
    );
    journal
        .record_service(operation, &result(operation, &request, "accepted"), now)
        .unwrap();
}

#[test]
fn side_effect_intent_survives_reopen_and_never_becomes_new_execution_authority() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let request = submit();
    {
        let journal = WorkerJournal::open(&path).unwrap();
        assert_eq!(
            journal.begin_service("op", "task", &request, 1).unwrap(),
            ServiceIntent::Execute
        );
    }
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal.begin_service("op", "task", &request, 2).unwrap(),
        ServiceIntent::Reconcile
    );
    assert!(journal.pending_service_jobs(20).unwrap().is_empty());
    assert!(matches!(
        journal.begin_service("op", "other-task", &request, 3),
        Err(Error::OperationConflict)
    ));
    let mut changed = request.clone();
    if let ServiceRequest::YoutubeSubmit { count, .. } = &mut changed {
        *count = 2;
    }
    assert!(matches!(
        journal.begin_service("op", "task", &changed, 3),
        Err(Error::OperationConflict)
    ));
    assert_eq!(
        journal.begin_service("op", "task", &request, 4).unwrap(),
        ServiceIntent::Reconcile
    );
}

#[test]
fn cached_results_survive_restart_and_conflicting_evidence_cannot_replace_them() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let request = ServiceRequest::YoutubeProfiles;
    let evidence = result("profiles", &request, "completed");
    {
        let journal = WorkerJournal::open(&path).unwrap();
        journal
            .begin_service("profiles", "task", &request, 1)
            .unwrap();
        // A read can be retried when no external side effect was admitted.
        assert_eq!(
            journal
                .begin_service("profiles", "task", &request, 2)
                .unwrap(),
            ServiceIntent::Execute
        );
        journal.record_service("profiles", &evidence, 3).unwrap();
    }
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal
            .begin_service("profiles", "task", &request, 4)
            .unwrap(),
        ServiceIntent::Cached(evidence.clone())
    );
    journal.record_service("profiles", &evidence, 5).unwrap();
    let mut changed = evidence.clone();
    changed.summary = "Conflicting synthetic replacement".into();
    assert!(matches!(
        journal.record_service("profiles", &changed, 6),
        Err(Error::OperationConflict)
    ));
    assert!(journal.record_service("unknown", &evidence, 6).is_err());
    assert_eq!(
        journal
            .begin_service("profiles", "task", &request, 7)
            .unwrap(),
        ServiceIntent::Cached(evidence)
    );
}

#[test]
fn polling_selects_only_durably_accepted_youtube_submissions() {
    let journal = WorkerJournal::open(":memory:").unwrap();
    for (operation, request, outcome) in [
        ("accepted-submit", submit(), "accepted"),
        ("completed-submit", submit(), "completed"),
        (
            "accepted-read-is-not-a-job",
            ServiceRequest::YoutubeProfiles,
            "accepted",
        ),
    ] {
        journal
            .begin_service(operation, "task", &request, 1)
            .unwrap();
        journal
            .record_service(operation, &result(operation, &request, outcome), 2)
            .unwrap();
    }
    journal
        .begin_service("unknown-submit", "task", &submit(), 3)
        .unwrap();
    let pending = journal.pending_service_jobs(100).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].operation_id, "accepted-submit");
    assert!(journal.pending_service_jobs(0).unwrap().is_empty());
}

#[test]
fn twenty_unreachable_or_unchanged_jobs_cannot_starve_the_twenty_first_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    {
        let journal = WorkerJournal::open(&path).unwrap();
        for index in 0..21 {
            accepted(&journal, &format!("operation-{index:02}"), 1);
        }
        let first = journal.pending_service_jobs(20).unwrap();
        assert_eq!(first.len(), 20);
        assert!(!first.iter().any(|j| j.operation_id == "operation-20"));
        for (index, job) in first.iter().enumerate() {
            // Attempt is persisted before status work; failed status calls need no notice ACK.
            journal.service_polled(&job.operation_id, 100).unwrap();
            if index % 2 == 0 {
                journal
                    .service_notified(&job.operation_id, "running", 101)
                    .unwrap();
            }
        }
    }
    let journal = WorkerJournal::open(&path).unwrap();
    let second = journal.pending_service_jobs(20).unwrap();
    assert_eq!(second[0].operation_id, "operation-20");
    // A backward wall clock cannot erase the prior attempt's rotation position.
    journal.service_polled("operation-00", 1).unwrap();
    assert_eq!(
        journal.pending_service_jobs(1).unwrap()[0].operation_id,
        "operation-20"
    );
    assert!(journal.service_polled("operation-00", -1).is_err());
    assert!(journal.service_polled("missing", 100).is_err());
}

#[test]
fn terminal_notice_ack_is_durable_and_cannot_regress_or_reopen_polling() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    {
        let journal = WorkerJournal::open(&path).unwrap();
        for (index, terminal) in ["completed", "failed", "uncertain"].iter().enumerate() {
            let operation = format!("operation-{index}");
            accepted(&journal, &operation, 1);
            journal.service_polled(&operation, 2).unwrap();
            journal.service_notified(&operation, terminal, 3).unwrap();
            journal.service_notified(&operation, terminal, 4).unwrap();
            assert!(journal.service_notified(&operation, "running", 5).is_err());
            assert!(journal.service_polled(&operation, 5).is_err());
        }
        assert!(journal.service_notified("missing", "running", 6).is_err());
        assert!(
            journal
                .service_notified("operation-0", "invalid", 6)
                .is_err()
        );
    }
    let journal = WorkerJournal::open(&path).unwrap();
    assert!(journal.pending_service_jobs(20).unwrap().is_empty());
    assert!(matches!(
        journal
            .begin_service("operation-0", "task-operation-0", &submit(), 7)
            .unwrap(),
        ServiceIntent::Cached(_)
    ));
}
