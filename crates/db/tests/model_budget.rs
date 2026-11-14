//! Synthetic accounting evidence; no model requests, real credentials, or network access.
use std::sync::Arc;
use zaturo_core::{
    AssistedResult, DiagnosticResult, ModelBudgetLimits, ModelCallState, ModelReservation,
    ModelUsage, OrchestrationResult, ReservationStatus,
};
use zaturo_db::{Error, WorkerJournal};

fn reserve(operation: &str, request: &str, input: u64, output: u64) -> ModelReservation {
    ModelReservation {
        request_id: request.into(),
        operation_id: operation.into(),
        task_id: format!("task-{operation}"),
        estimated_input_tokens: input,
        max_output_tokens: output,
    }
}
fn operation(journal: &WorkerJournal, name: &str, now: i64) {
    journal
        .begin_assisted(name, &format!("task-{name}"), now)
        .unwrap();
}
fn limits() -> ModelBudgetLimits {
    ModelBudgetLimits {
        per_request_tokens: 100,
        per_task_tokens: 200,
        per_day_tokens: 400,
        per_month_tokens: 800,
        max_task_model_calls: 4,
    }
}
fn usage(input: u64, output: u64) -> ModelUsage {
    ModelUsage {
        input_tokens: input,
        output_tokens: output,
        cost_microusd: None,
    }
}
fn assisted(operation: &str) -> AssistedResult {
    AssistedResult {
        diagnostic: DiagnosticResult {
            operation_id: operation.into(),
            tool: "diagnostic.v1".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            success: true,
            summary: "Typed diagnostic passed".into(),
        },
        orchestration: OrchestrationResult {
            model: "synthetic-model".into(),
            steps: 2,
            summary: "Diagnostic evidence accepted".into(),
            usage: usage(10, 5),
            model_calls: 1,
            usage_complete: true,
        },
    }
}

#[test]
fn request_task_and_call_limits_are_independent() {
    let journal = WorkerJournal::open(":memory:").unwrap();
    operation(&journal, "operation", 0);
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "too-large", 90, 11), &limits(), 0),
        Err(Error::BudgetExceeded)
    ));
    assert_eq!(
        journal
            .reserve_model_call(&reserve("operation", "one", 60, 40), &limits(), 0)
            .unwrap(),
        ReservationStatus::Granted
    );
    assert_eq!(
        journal
            .reserve_model_call(&reserve("operation", "two", 60, 40), &limits(), 1)
            .unwrap(),
        ReservationStatus::Granted
    );
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "three", 1, 1), &limits(), 2),
        Err(Error::BudgetExceeded)
    ));
    journal.settle_model_call("one", &usage(1, 1), 3).unwrap();
    journal.settle_model_call("two", &usage(1, 1), 4).unwrap();
    let single = ModelBudgetLimits {
        max_task_model_calls: 2,
        ..limits()
    };
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "after-settlement", 1, 1), &single, 5),
        Err(Error::BudgetExceeded)
    ));
}

#[test]
fn uncertain_and_pending_reservations_survive_restart_without_new_authorization() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let journal = WorkerJournal::open(&path).unwrap();
    operation(&journal, "operation", 0);
    let request = reserve("operation", "stable-step-1", 60, 40);
    assert_eq!(
        journal.reserve_model_call(&request, &limits(), 1).unwrap(),
        ReservationStatus::Granted
    );
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal.reserve_model_call(&request, &limits(), 2).unwrap(),
        ReservationStatus::Existing(ModelCallState::Reserved)
    );
    journal
        .mark_model_call_uncertain(&request.request_id, 3)
        .unwrap();
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal.reserve_model_call(&request, &limits(), 4).unwrap(),
        ReservationStatus::Existing(ModelCallState::Uncertain)
    );
    let bounded = ModelBudgetLimits {
        per_task_tokens: 100,
        ..limits()
    };
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "new-attempt", 1, 1), &bounded, 5),
        Err(Error::BudgetExceeded)
    ));
    let mut mismatch = request.clone();
    mismatch.max_output_tokens = 41;
    assert!(matches!(
        journal.reserve_model_call(&mismatch, &limits(), 6),
        Err(Error::OperationConflict)
    ));
}

#[test]
fn usage_settlement_is_idempotent_and_cannot_be_overwritten() {
    let journal = WorkerJournal::open(":memory:").unwrap();
    operation(&journal, "operation", 0);
    let request = reserve("operation", "one", 60, 40);
    journal.reserve_model_call(&request, &limits(), 1).unwrap();
    let actual = ModelUsage {
        input_tokens: 30,
        output_tokens: 20,
        cost_microusd: Some(123),
    };
    journal.settle_model_call("one", &actual, 2).unwrap();
    journal.settle_model_call("one", &actual, 3).unwrap();
    assert_eq!(journal.model_call_usage("one").unwrap(), Some(actual));
    assert_eq!(
        journal.reserve_model_call(&request, &limits(), 4).unwrap(),
        ReservationStatus::Existing(ModelCallState::Settled)
    );
    assert!(matches!(
        journal.settle_model_call("one", &usage(20, 20), 5),
        Err(Error::OperationConflict)
    ));
    assert!(journal.mark_model_call_uncertain("one", 6).is_err());
    assert!(
        journal
            .settle_model_call("missing", &usage(1, 1), 7)
            .is_err()
    );
}

#[test]
fn overrun_debt_commits_even_when_settlement_denies_and_blocks_the_operation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let journal = WorkerJournal::open(&path).unwrap();
    operation(&journal, "operation", 0);
    let request = reserve("operation", "one", 60, 40);
    journal.reserve_model_call(&request, &limits(), 1).unwrap();
    assert!(matches!(
        journal.settle_model_call("one", &usage(100, 50), 2),
        Err(Error::BudgetExceeded)
    ));
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal.model_call_usage("one").unwrap(),
        Some(usage(100, 50))
    );
    assert_eq!(
        journal.reserve_model_call(&request, &limits(), 3).unwrap(),
        ReservationStatus::Existing(ModelCallState::Overrun)
    );
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "two", 1, 1), &limits(), 4),
        Err(Error::BudgetExceeded)
    ));
}

#[test]
fn utc_day_and_month_caps_do_not_reset_task_spending() {
    const NOVEMBER: i64 = 1_793_491_200_000;
    const DAY: i64 = 86_400_000;
    const DECEMBER: i64 = 1_796_083_200_000;
    let journal = WorkerJournal::open(":memory:").unwrap();
    let cap = ModelBudgetLimits {
        per_request_tokens: 40,
        per_task_tokens: 40,
        per_day_tokens: 60,
        per_month_tokens: 100,
        max_task_model_calls: 4,
    };
    for name in ["one", "two", "three"] {
        operation(&journal, name, NOVEMBER);
    }
    journal
        .reserve_model_call(&reserve("one", "one-1", 20, 20), &cap, NOVEMBER)
        .unwrap();
    assert!(matches!(
        journal.reserve_model_call(&reserve("two", "two-1", 20, 20), &cap, NOVEMBER + 1),
        Err(Error::BudgetExceeded)
    ));
    journal
        .reserve_model_call(&reserve("two", "two-1", 20, 20), &cap, NOVEMBER + DAY)
        .unwrap();
    assert!(matches!(
        journal.reserve_model_call(
            &reserve("three", "three-1", 20, 20),
            &cap,
            NOVEMBER + 2 * DAY
        ),
        Err(Error::BudgetExceeded)
    ));
    journal
        .reserve_model_call(&reserve("three", "three-1", 20, 20), &cap, DECEMBER)
        .unwrap();
    assert!(matches!(
        journal.reserve_model_call(&reserve("one", "one-2", 1, 1), &cap, DECEMBER + DAY),
        Err(Error::BudgetExceeded)
    ));
}

#[test]
fn concurrent_reservations_cannot_overspend_the_shared_daily_limit() {
    let journal = Arc::new(WorkerJournal::open(":memory:").unwrap());
    let cap = ModelBudgetLimits {
        per_request_tokens: 30,
        per_task_tokens: 30,
        per_day_tokens: 50,
        per_month_tokens: 100,
        max_task_model_calls: 4,
    };
    for n in 0..8 {
        operation(&journal, &format!("op-{n}"), 0);
    }
    let handles = (0..8)
        .map(|n| {
            let journal = journal.clone();
            let cap = cap.clone();
            std::thread::spawn(move || {
                journal
                    .reserve_model_call(
                        &reserve(&format!("op-{n}"), &format!("request-{n}"), 20, 10),
                        &cap,
                        1,
                    )
                    .is_ok()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|v| *v)
            .count(),
        1
    );
}

#[test]
fn backward_clock_invalid_input_and_wrong_operation_fail_closed() {
    let journal = WorkerJournal::open(":memory:").unwrap();
    operation(&journal, "operation", 0);
    let request = reserve("operation", "one", 20, 10);
    journal
        .reserve_model_call(&request, &limits(), 100)
        .unwrap();
    assert!(matches!(
        journal.reserve_model_call(&reserve("operation", "two", 20, 10), &limits(), 99),
        Err(Error::OperationConflict)
    ));
    assert!(
        journal
            .reserve_model_call(&reserve("missing", "missing", 20, 10), &limits(), 101)
            .is_err()
    );
    let mut invalid = request;
    invalid.request_id = "invalid/request".into();
    assert!(
        journal
            .reserve_model_call(&invalid, &limits(), 102)
            .is_err()
    );
    assert!(
        journal
            .reserve_model_call(&reserve("operation", "negative", 20, 10), &limits(), -1)
            .is_err()
    );
}

#[test]
fn version_two_database_migrates_and_cached_assisted_evidence_survives_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch(include_str!("../migrations/0002_worker.sql"))
        .unwrap();
    let legacy = assisted("legacy").diagnostic;
    old.execute("INSERT INTO operations(operation_id,task_id,started_at,updated_at,result_json) VALUES ('legacy','task-legacy',0,0,?1)",[serde_json::to_string(&legacy).unwrap()]).unwrap();
    drop(old);
    let journal = WorkerJournal::open(&path).unwrap();
    operation(&journal, "operation", 0);
    assert_eq!(
        journal.begin("legacy", "task-legacy", 1).unwrap(),
        Some(legacy)
    );
    assert!(
        journal
            .record_assisted("operation", &assisted("operation"), 1)
            .is_err()
    );
    journal
        .record_result("operation", &assisted("operation").diagnostic, 1)
        .unwrap();
    assert_eq!(
        journal.result("operation").unwrap(),
        Some(assisted("operation").diagnostic)
    );
    journal
        .record_assisted("operation", &assisted("operation"), 1)
        .unwrap();
    assert!(journal.begin("operation", "task-operation", 2).is_err());
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal
            .begin_assisted("operation", "task-operation", 3)
            .unwrap(),
        Some(assisted("operation"))
    );
    assert!(
        journal
            .begin_assisted("operation", "wrong-task", 3)
            .is_err()
    );
    let mut changed = assisted("operation");
    changed.orchestration.summary = "Changed completion".into();
    assert!(journal.record_assisted("operation", &changed, 4).is_err());
    assert!(
        journal
            .reserve_model_call(&reserve("operation", "after-final", 20, 10), &limits(), 5)
            .is_err()
    );
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        6
    );
}

#[test]
fn interruption_marks_only_pending_calls_of_that_operation_and_never_refunds() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let journal = WorkerJournal::open(&path).unwrap();
    for op in ["cancelled", "other"] {
        operation(&journal, op, 0);
    }
    let settled = reserve("cancelled", "settled", 10, 10);
    let pending = reserve("cancelled", "pending", 20, 10);
    let other = reserve("other", "unrelated", 20, 10);
    for request in [&settled, &pending, &other] {
        journal.reserve_model_call(request, &limits(), 1).unwrap();
    }
    journal
        .settle_model_call("settled", &usage(5, 5), 2)
        .unwrap();
    journal.interrupt_model_operation("cancelled", 3).unwrap();
    journal.interrupt_model_operation("cancelled", 4).unwrap();
    journal
        .interrupt_model_operation("never-started", 4)
        .unwrap();
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert_eq!(
        journal.reserve_model_call(&settled, &limits(), 5).unwrap(),
        ReservationStatus::Existing(ModelCallState::Settled)
    );
    assert_eq!(
        journal.reserve_model_call(&pending, &limits(), 5).unwrap(),
        ReservationStatus::Existing(ModelCallState::Uncertain)
    );
    assert_eq!(
        journal.reserve_model_call(&other, &limits(), 5).unwrap(),
        ReservationStatus::Existing(ModelCallState::Reserved)
    );
    let cap = ModelBudgetLimits {
        per_request_tokens: 40,
        per_task_tokens: 40,
        ..limits()
    };
    assert!(matches!(
        journal.reserve_model_call(&reserve("cancelled", "new", 1, 1), &cap, 6),
        Err(Error::BudgetExceeded)
    ));
}
