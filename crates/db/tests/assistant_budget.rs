//! Synthetic durable spend/upgrade evidence; no provider network or production databases.
use zaturo_core::{
    ModelBudgetLimits, ModelReservation, ModelUsage, ReservationStatus, ServiceRequest,
};
use zaturo_db::{ServiceIntent, WorkerJournal};
fn limits() -> ModelBudgetLimits {
    ModelBudgetLimits {
        per_request_tokens: 100,
        per_task_tokens: 100,
        per_day_tokens: 160,
        per_month_tokens: 1000,
        max_task_model_calls: 1,
    }
}
fn reservation(op: &str) -> ModelReservation {
    ModelReservation {
        request_id: format!("{op}:1:1"),
        operation_id: op.into(),
        task_id: format!("task-{op}"),
        estimated_input_tokens: 64,
        max_output_tokens: 16,
    }
}
fn assistant(journal: &WorkerJournal, op: &str) {
    let req = ServiceRequest::AssistantChat {
        message: "Synthetic owner message".into(),
        context: vec![],
    };
    assert!(matches!(
        journal
            .begin_service(op, &format!("task-{op}"), &req, 1000)
            .unwrap(),
        ServiceIntent::Execute
    ));
}
#[test]
fn assistant_uncertainty_survives_restart_without_another_admission() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.db");
    let journal = WorkerJournal::open(&path).unwrap();
    assistant(&journal, "one");
    let request = reservation("one");
    assert_eq!(
        journal
            .reserve_assistant_call(&request, &limits(), 1000)
            .unwrap(),
        ReservationStatus::Granted
    );
    journal.interrupt_model_operation("one", 1001).unwrap();
    drop(journal);
    let journal = WorkerJournal::open(&path).unwrap();
    assert!(matches!(
        journal
            .reserve_assistant_call(&request, &limits(), 1002)
            .unwrap(),
        ReservationStatus::Existing(_)
    ));
    assert!(matches!(
        journal
            .begin_service(
                "one",
                "task-one",
                &ServiceRequest::AssistantChat {
                    message: "Synthetic owner message".into(),
                    context: vec![]
                },
                1002
            )
            .unwrap(),
        ServiceIntent::Reconcile
    ));
    assistant(&journal, "two");
    assert_eq!(
        journal
            .reserve_assistant_call(&reservation("two"), &limits(), 1002)
            .unwrap(),
        ReservationStatus::Granted
    );
    assistant(&journal, "three");
    assert!(matches!(
        journal.reserve_assistant_call(&reservation("three"), &limits(), 1002),
        Err(zaturo_db::Error::BudgetExceeded)
    ));
}
#[test]
fn personal_and_coding_reservations_share_day_and_month_charges() {
    for personal_first in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let journal = WorkerJournal::open(directory.path().join("worker.db")).unwrap();
        assistant(&journal, "personal");
        journal
            .begin_assisted("coding", "task-coding", 1000)
            .unwrap();
        let mut cap = limits();
        cap.per_day_tokens = 100;
        if personal_first {
            journal
                .reserve_assistant_call(&reservation("personal"), &cap, 1000)
                .unwrap();
            assert!(matches!(
                journal.reserve_model_call(&reservation("coding"), &cap, 1001),
                Err(zaturo_db::Error::BudgetExceeded)
            ));
        } else {
            journal
                .reserve_model_call(&reservation("coding"), &cap, 1000)
                .unwrap();
            assert!(matches!(
                journal.reserve_assistant_call(&reservation("personal"), &cap, 1001),
                Err(zaturo_db::Error::BudgetExceeded)
            ));
        }
    }
}
#[test]
fn settlement_debt_and_clock_rollback_cannot_restore_allowance() {
    let directory = tempfile::tempdir().unwrap();
    let journal = WorkerJournal::open(directory.path().join("worker.db")).unwrap();
    assistant(&journal, "one");
    let request = reservation("one");
    journal
        .reserve_assistant_call(&request, &limits(), 1000)
        .unwrap();
    let usage = ModelUsage {
        input_tokens: 120,
        output_tokens: 0,
        cost_microusd: None,
    };
    assert!(matches!(
        journal.settle_assistant_call(&request.request_id, &usage, 1001),
        Err(zaturo_db::Error::BudgetExceeded)
    ));
    assert!(matches!(
        journal.settle_assistant_call(&request.request_id, &usage, 1002),
        Err(zaturo_db::Error::BudgetExceeded)
    ));
    assistant(&journal, "two");
    assert!(matches!(
        journal.reserve_assistant_call(&reservation("two"), &limits(), 999),
        Err(zaturo_db::Error::OperationConflict)
    ));
    assert!(matches!(
        journal.reserve_assistant_call(&reservation("two"), &limits(), 1002),
        Err(zaturo_db::Error::BudgetExceeded)
    ));
}
#[test]
fn only_the_matching_assistant_service_intent_can_reserve_personal_calls() {
    let directory = tempfile::tempdir().unwrap();
    let journal = WorkerJournal::open(directory.path().join("worker.db")).unwrap();
    journal
        .begin_service("bad", "task-bad", &ServiceRequest::YoutubeProfiles, 1)
        .unwrap();
    assert!(
        journal
            .reserve_assistant_call(&reservation("bad"), &limits(), 1000)
            .is_err()
    );
    assert!(
        journal
            .reserve_assistant_call(&reservation("missing"), &limits(), 1000)
            .is_err()
    );
}
