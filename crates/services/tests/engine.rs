use std::sync::Arc;
use zaturo_core::{ControlAction, DiagnosticResult, Limits, NewTask, TaskState};
use zaturo_db::{HubStore, WorkerJournal};
use zaturo_services::{Engine, Error};

const WORKER: &str = "local-worker";

fn engine() -> Engine {
    Engine::open(":memory:", Limits::default()).unwrap()
}

fn register(engine: &Engine) {
    engine
        .register(WORKER, &["diagnostic.v1".into()], 1000)
        .unwrap();
}

fn report(operation_id: &str) -> DiagnosticResult {
    DiagnosticResult {
        operation_id: operation_id.into(),
        tool: "diagnostic.v1".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        success: true,
        summary: "All diagnostic acceptance checks passed".into(),
    }
}

#[test]
fn authenticated_caller_lifecycle_is_transactional_and_idempotent() {
    let engine = engine();
    register(&engine);
    let request = NewTask::diagnostic(WORKER, "request-1");
    let task = engine.create(request.clone(), 1000).unwrap();
    assert_eq!(task.state, TaskState::Queued);
    assert_eq!(engine.create(request, 2000).unwrap().id, task.id);
    let assignment = engine.claim(WORKER, 2000).unwrap().unwrap();
    assert_eq!(assignment.task.attempts, 1);
    assert_eq!(engine.claim(WORKER, 2001).unwrap().unwrap(), assignment);
    assert_eq!(
        engine
            .start(&task.id, WORKER, &assignment.lease_token, 2002)
            .unwrap()
            .state,
        TaskState::Running
    );
    assert_eq!(
        engine
            .start(&task.id, WORKER, &assignment.lease_token, 2003)
            .unwrap()
            .state,
        TaskState::Running
    );
    let result = report(&task.operation_id);
    let completed = engine
        .complete(
            &task.id,
            WORKER,
            &assignment.lease_token,
            result.clone(),
            2004,
        )
        .unwrap();
    assert_eq!(completed.state, TaskState::Completed);
    assert_eq!(
        engine
            .complete(&task.id, WORKER, &assignment.lease_token, result, 999_999)
            .unwrap(),
        completed
    );
    let events = engine.events(&task.id, 100, 0).unwrap();
    assert_eq!(
        events.iter().map(|event| event.state).collect::<Vec<_>>(),
        vec![
            TaskState::Created,
            TaskState::Queued,
            TaskState::Assigned,
            TaskState::Running,
            TaskState::Completed
        ]
    );
    assert!(engine.claim(WORKER, 2005).unwrap().is_none());
}

#[test]
fn idempotency_collision_cannot_mutate_existing_request() {
    let engine = engine();
    let request = NewTask::diagnostic(WORKER, "same-key");
    engine.create(request.clone(), 1000).unwrap();
    let mut conflicting = request;
    conflicting.request = "different owner request".into();
    assert!(matches!(
        engine.create(conflicting, 2000),
        Err(Error::Conflict(_))
    ));
    assert_eq!(engine.list(100, 0).unwrap().len(), 1);
}

#[test]
fn queue_capacity_and_pagination_are_enforced() {
    let engine = Engine::open(
        ":memory:",
        Limits {
            max_queued_tasks: 1,
            max_page_size: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let request = NewTask::diagnostic(WORKER, "one");
    engine.create(request.clone(), 1000).unwrap();
    assert!(engine.create(request, 1001).is_ok());
    assert!(matches!(
        engine.create(NewTask::diagnostic(WORKER, "two"), 1001),
        Err(Error::Capacity)
    ));
    assert!(engine.list(0, 0).is_err());
    assert!(engine.list(3, 0).is_err());
    assert!(engine.list(2, 1_000_001).is_err());
    assert!(engine.list(2, 1).unwrap().is_empty());
}

#[test]
fn capabilities_and_target_worker_gate_assignment() {
    let engine = engine();
    engine
        .register("other-worker", &["diagnostic.v1".into()], 1000)
        .unwrap();
    engine.register(WORKER, &[], 1000).unwrap();
    engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    assert!(engine.claim("other-worker", 1001).unwrap().is_none());
    assert!(engine.claim(WORKER, 1001).unwrap().is_none());
    assert!(engine.register(WORKER, &["shell".into()], 1001).is_err());
    register(&engine);
    assert!(engine.claim(WORKER, 1001).unwrap().is_some());
}

#[test]
fn worker_capacity_and_invalid_configuration_fail_closed() {
    assert!(
        Engine::open(
            ":memory:",
            Limits {
                lease_seconds: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let engine = Engine::open(
        ":memory:",
        Limits {
            max_workers: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    register(&engine);
    assert!(matches!(
        engine.register("another", &[], 1001),
        Err(Error::Capacity)
    ));
    register(&engine);
}

#[test]
fn wrong_worker_stale_token_and_unstarted_completion_are_rejected() {
    let engine = engine();
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let assignment = engine.claim(WORKER, 1001).unwrap().unwrap();
    assert!(
        engine
            .start(&task.id, "other-worker", &assignment.lease_token, 1002)
            .is_err()
    );
    assert!(engine.start(&task.id, WORKER, "wrong-token", 1002).is_err());
    assert!(
        engine
            .complete(
                &task.id,
                WORKER,
                &assignment.lease_token,
                report(&task.operation_id),
                1002
            )
            .is_err()
    );
    engine
        .start(&task.id, WORKER, &assignment.lease_token, 1002)
        .unwrap();
    assert!(
        engine
            .complete(
                &task.id,
                WORKER,
                &assignment.lease_token,
                report("wrong-operation"),
                1003
            )
            .is_err()
    );
    let mut failed = report(&task.operation_id);
    failed.success = false;
    assert!(
        engine
            .complete(&task.id, WORKER, &assignment.lease_token, failed, 1003)
            .is_err()
    );
    assert_eq!(engine.get(&task.id).unwrap().state, TaskState::Running);
    assert_eq!(engine.events(&task.id, 100, 0).unwrap().len(), 4);
}

#[test]
fn heartbeats_extend_only_current_unexpired_lease_without_event_growth() {
    let engine = engine();
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let assignment = engine.claim(WORKER, 2000).unwrap().unwrap();
    engine
        .heartbeat(WORKER, Some(&assignment.lease_token), 3000)
        .unwrap();
    assert_eq!(engine.get(&task.id).unwrap().lease_expires_at, Some(33_000));
    assert!(engine.heartbeat(WORKER, Some("wrong"), 4000).is_err());
    assert!(
        engine
            .heartbeat(WORKER, Some(&assignment.lease_token), 33_000)
            .is_err()
    );
    assert_eq!(engine.events(&task.id, 100, 0).unwrap().len(), 3);
    engine.heartbeat(WORKER, None, 40_000).unwrap();
    assert_eq!(engine.get(&task.id).unwrap().lease_expires_at, Some(33_000));
}

#[test]
fn hub_restart_recovers_task_and_rejects_previous_lease() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hub.db");
    let (task, old_assignment) = {
        let engine = Engine::open(&path, Limits::default()).unwrap();
        register(&engine);
        let task = engine
            .create(NewTask::diagnostic(WORKER, "one"), 1000)
            .unwrap();
        let assignment = engine.claim(WORKER, 2000).unwrap().unwrap();
        engine
            .start(&task.id, WORKER, &assignment.lease_token, 2001)
            .unwrap();
        (task, assignment)
    };
    let engine = Engine::open(&path, Limits::default()).unwrap();
    assert_eq!(engine.get(&task.id).unwrap().state, TaskState::Running);
    assert_eq!(
        engine.claim(WORKER, 3000).unwrap().unwrap().lease_token,
        old_assignment.lease_token
    );
    assert_eq!(engine.recover(32_000).unwrap(), 1);
    let assignment = engine.claim(WORKER, 32_001).unwrap().unwrap();
    assert_eq!(assignment.task.operation_id, task.operation_id);
    assert_eq!(assignment.task.attempts, 2);
    assert_ne!(assignment.lease_token, old_assignment.lease_token);
    assert!(
        engine
            .complete(
                &task.id,
                WORKER,
                &old_assignment.lease_token,
                report(&task.operation_id),
                32_002
            )
            .is_err()
    );
    engine
        .start(&task.id, WORKER, &assignment.lease_token, 32_002)
        .unwrap();
    assert_eq!(
        engine
            .complete(
                &task.id,
                WORKER,
                &assignment.lease_token,
                report(&task.operation_id),
                32_003
            )
            .unwrap()
            .state,
        TaskState::Completed
    );
}

#[test]
fn cancelled_task_revokes_lease_and_cannot_resume_or_complete() {
    let engine = engine();
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let assignment = engine.claim(WORKER, 1001).unwrap().unwrap();
    engine
        .start(&task.id, WORKER, &assignment.lease_token, 1002)
        .unwrap();
    assert_eq!(
        engine
            .control(&task.id, ControlAction::Cancel, 1003)
            .unwrap()
            .state,
        TaskState::Cancelled
    );
    assert!(
        engine
            .complete(
                &task.id,
                WORKER,
                &assignment.lease_token,
                report(&task.operation_id),
                1004
            )
            .is_err()
    );
    assert!(
        engine
            .control(&task.id, ControlAction::Resume, 1004)
            .is_err()
    );
    assert!(engine.claim(WORKER, 1004).unwrap().is_none());
}

#[test]
fn pause_resume_revokes_lease_and_retains_operation_identity() {
    let engine = engine();
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let assignment = engine.claim(WORKER, 1001).unwrap().unwrap();
    assert_eq!(
        engine
            .control(&task.id, ControlAction::Pause, 1002)
            .unwrap()
            .state,
        TaskState::Paused
    );
    assert!(
        engine
            .start(&task.id, WORKER, &assignment.lease_token, 1003)
            .is_err()
    );
    assert!(engine.claim(WORKER, 1003).unwrap().is_none());
    engine
        .control(&task.id, ControlAction::Resume, 1004)
        .unwrap();
    let next = engine.claim(WORKER, 1005).unwrap().unwrap();
    assert_ne!(assignment.lease_token, next.lease_token);
    assert_eq!(next.task.operation_id, task.operation_id);
}

#[test]
fn deadlines_and_retry_budget_eventually_fail_tasks() {
    let engine = engine();
    register(&engine);
    let mut request = NewTask::diagnostic(WORKER, "retry");
    request.max_attempts = 1;
    let task = engine.create(request, 1000).unwrap();
    engine.claim(WORKER, 1001).unwrap().unwrap();
    engine.recover(31_001).unwrap();
    assert_eq!(engine.get(&task.id).unwrap().state, TaskState::Failed);
    let mut request = NewTask::diagnostic(WORKER, "deadline");
    request.timeout_seconds = 1;
    let expired = engine.create(request, 40_000).unwrap();
    engine.recover(41_000).unwrap();
    assert_eq!(
        engine.get(&expired.id).unwrap().failure.as_deref(),
        Some("Task deadline expired")
    );
    assert!(engine.claim(WORKER, 41_000).unwrap().is_none());
}

#[test]
fn worker_restart_reconciles_cached_result_into_reassigned_hub_task() {
    let directory = tempfile::tempdir().unwrap();
    let hub_path = directory.path().join("hub.db");
    let journal_path = directory.path().join("worker.db");
    let engine = Engine::open(&hub_path, Limits::default()).unwrap();
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let assignment = engine.claim(WORKER, 2000).unwrap().unwrap();
    engine
        .start(&task.id, WORKER, &assignment.lease_token, 2001)
        .unwrap();
    {
        let journal = WorkerJournal::open(&journal_path).unwrap();
        assert!(
            journal
                .begin(&task.operation_id, &task.id, 2002)
                .unwrap()
                .is_none()
        );
        journal
            .record_result(&task.operation_id, &report(&task.operation_id), 2003)
            .unwrap();
    }
    engine.recover(32_000).unwrap();
    let assignment = engine.claim(WORKER, 32_001).unwrap().unwrap();
    engine
        .start(&task.id, WORKER, &assignment.lease_token, 32_002)
        .unwrap();
    let journal = WorkerJournal::open(&journal_path).unwrap();
    let cached = journal
        .begin(&task.operation_id, &task.id, 32_003)
        .unwrap()
        .unwrap();
    assert_eq!(
        engine
            .complete(&task.id, WORKER, &assignment.lease_token, cached, 32_004)
            .unwrap()
            .state,
        TaskState::Completed
    );
}

#[test]
fn concurrent_claims_have_one_lease_and_one_attempt() {
    let engine = Arc::new(engine());
    register(&engine);
    let task = engine
        .create(NewTask::diagnostic(WORKER, "one"), 1000)
        .unwrap();
    let handles = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            std::thread::spawn(move || engine.claim(WORKER, 1001).unwrap().unwrap())
        })
        .collect::<Vec<_>>();
    let assignments = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert!(assignments.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(engine.get(&task.id).unwrap().attempts, 1);
}

#[test]
fn transaction_error_rolls_back_all_writes() {
    let store = HubStore::open(":memory:").unwrap();
    let result: Result<(), zaturo_db::Error> = store.transaction(|repository| {
        repository.save_worker(&zaturo_core::Worker {
            id: WORKER.into(),
            capabilities: vec![],
            last_seen_at: 0,
        })?;
        Err(zaturo_db::Error::OperationConflict)
    });
    assert!(result.is_err());
    let worker: Option<zaturo_core::Worker> = store
        .read::<_, zaturo_db::Error>(|repository| repository.worker(WORKER))
        .unwrap();
    assert!(worker.is_none());
}

#[test]
fn hub_and_worker_database_schemas_are_not_interchangeable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("separate.db");
    drop(WorkerJournal::open(&path).unwrap());
    assert!(matches!(
        HubStore::open(&path),
        Err(zaturo_db::Error::SchemaVersion)
    ));
}

#[test]
fn bounded_recovery_page_eventually_processes_backlog() {
    let engine = Engine::open(
        ":memory:",
        Limits {
            max_page_size: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    for key in ["one", "two", "three"] {
        let mut request = NewTask::diagnostic(WORKER, key);
        request.timeout_seconds = 1;
        engine.create(request, 1000).unwrap();
    }
    assert_eq!(engine.recover(2000).unwrap(), 1);
    assert_eq!(engine.recover(2000).unwrap(), 1);
    assert_eq!(engine.recover(2000).unwrap(), 1);
    assert_eq!(engine.recover(2000).unwrap(), 0);
}

#[test]
fn pause_resume_cannot_bypass_attempt_budget() {
    let engine = engine();
    register(&engine);
    let mut request = NewTask::diagnostic(WORKER, "bounded-attempts");
    request.max_attempts = 1;
    let task = engine.create(request, 1000).unwrap();
    engine.claim(WORKER, 1001).unwrap().unwrap();
    engine
        .control(&task.id, ControlAction::Pause, 1002)
        .unwrap();
    engine
        .control(&task.id, ControlAction::Resume, 1003)
        .unwrap();
    assert!(engine.claim(WORKER, 1004).unwrap().is_none());
    assert_eq!(engine.get(&task.id).unwrap().state, TaskState::Failed);
    assert_eq!(engine.get(&task.id).unwrap().attempts, 1);
}

#[test]
fn queued_priority_is_explicit_and_fifo_breaks_ties() {
    let engine = engine();
    register(&engine);
    let first = engine
        .create(NewTask::diagnostic(WORKER, "first"), 1000)
        .unwrap();
    let mut urgent = NewTask::diagnostic(WORKER, "urgent");
    urgent.priority = 100;
    let urgent = engine.create(urgent, 1001).unwrap();
    assert_eq!(
        engine.claim(WORKER, 1002).unwrap().unwrap().task.id,
        urgent.id
    );
    engine
        .control(&urgent.id, ControlAction::Cancel, 1003)
        .unwrap();
    assert_eq!(
        engine.claim(WORKER, 1004).unwrap().unwrap().task.id,
        first.id
    );
}
