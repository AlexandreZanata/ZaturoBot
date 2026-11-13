use zaturo_core::{
    DataPolicy, DiagnosticResult, FailureCode, Limits, ModelUsage, NewTask, OrchestrationResult,
    Task, TaskState,
};
use zaturo_services::Engine;

const WORKER: &str = "worker";
fn engine() -> Engine {
    let e = Engine::open(":memory:", Limits::default()).unwrap();
    e.register(
        WORKER,
        &["diagnostic.v1".into(), "agent.diagnostic.v1".into()],
        0,
    )
    .unwrap();
    e
}
fn report(operation: &str) -> DiagnosticResult {
    DiagnosticResult {
        operation_id: operation.into(),
        tool: "diagnostic.v1".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        success: true,
        summary: "Typed diagnostic passed".into(),
    }
}
fn orchestration() -> OrchestrationResult {
    OrchestrationResult {
        model: "synthetic-model".into(),
        steps: 2,
        summary: "Validated diagnostic evidence".into(),
        usage: ModelUsage {
            input_tokens: 50,
            output_tokens: 20,
            cost_microusd: None,
        },
        model_calls: 1,
        usage_complete: true,
    }
}

#[test]
fn assisted_completion_requires_explicit_policy_and_typed_orchestration_evidence() {
    let e = engine();
    let mut unauthorized = NewTask::assisted_diagnostic(WORKER, "unauthorized");
    unauthorized.data_policy = None;
    assert!(e.create(unauthorized, 1).is_err());
    let task = e
        .create(NewTask::assisted_diagnostic(WORKER, "assisted"), 1)
        .unwrap();
    assert_eq!(task.data_policy, Some(DataPolicy::PublicCoding));
    assert_eq!(task.required_capabilities, vec!["agent.diagnostic.v1"]);
    let assignment = e.claim(WORKER, 2).unwrap().unwrap();
    e.start(&task.id, WORKER, &assignment.lease_token, 3)
        .unwrap();
    assert!(
        e.complete(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            4
        )
        .is_err()
    );
    let mut invalid = orchestration();
    invalid.usage_complete = false;
    invalid.usage.cost_microusd = Some(1);
    assert!(
        e.complete_assisted(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            invalid,
            4
        )
        .is_err()
    );
    let done = e
        .complete_assisted(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            orchestration(),
            5,
        )
        .unwrap();
    assert_eq!(done.state, TaskState::Completed);
    assert_eq!(done.orchestration, Some(orchestration()));
    assert_eq!(
        e.complete_assisted(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            orchestration(),
            999_999
        )
        .unwrap(),
        done
    );
    let mut conflict = orchestration();
    conflict.summary = "Changed result".into();
    assert!(
        e.complete_assisted(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            conflict,
            6
        )
        .is_err()
    );
}

#[test]
fn unsupported_assisted_task_does_not_starve_an_eligible_diagnostic() {
    let e = engine();
    e.register(WORKER, &["diagnostic.v1".into()], 0).unwrap();
    let mut assisted = NewTask::assisted_diagnostic(WORKER, "assisted");
    assisted.priority = 100;
    e.create(assisted, 1).unwrap();
    let plain = e.create(NewTask::diagnostic(WORKER, "plain"), 2).unwrap();
    assert_eq!(e.claim(WORKER, 3).unwrap().unwrap().task.id, plain.id);
}

#[test]
fn typed_worker_failure_is_fenced_idempotent_and_terminal() {
    let e = engine();
    let task = e
        .create(NewTask::assisted_diagnostic(WORKER, "fail"), 1)
        .unwrap();
    let assignment = e.claim(WORKER, 2).unwrap().unwrap();
    assert!(
        e.fail(
            &task.id,
            WORKER,
            &assignment.lease_token,
            FailureCode::PolicyDenied,
            3
        )
        .is_err()
    );
    e.start(&task.id, WORKER, &assignment.lease_token, 3)
        .unwrap();
    assert!(
        e.fail(
            &task.id,
            "other-worker",
            &assignment.lease_token,
            FailureCode::PolicyDenied,
            4
        )
        .is_err()
    );
    let failed = e
        .fail(
            &task.id,
            WORKER,
            &assignment.lease_token,
            FailureCode::ReconciliationRequired,
            5,
        )
        .unwrap();
    assert_eq!(failed.failure.as_deref(), Some("reconciliation_required"));
    assert_eq!(failed.state, TaskState::Failed);
    assert_eq!(
        e.fail(
            &task.id,
            WORKER,
            &assignment.lease_token,
            FailureCode::ReconciliationRequired,
            999_999
        )
        .unwrap(),
        failed
    );
    assert!(
        e.fail(
            &task.id,
            WORKER,
            &assignment.lease_token,
            FailureCode::BudgetExceeded,
            6
        )
        .is_err()
    );
    assert!(
        e.complete_assisted(
            &task.id,
            WORKER,
            &assignment.lease_token,
            report(&task.operation_id),
            orchestration(),
            7
        )
        .is_err()
    );
    assert_eq!(
        e.events(&task.id, 100, 0).unwrap().last().unwrap().state,
        TaskState::Failed
    );
}

#[test]
fn legacy_hub_task_json_remains_readable_without_a_schema_rewrite() {
    let e = engine();
    let task = e.create(NewTask::diagnostic(WORKER, "legacy"), 1).unwrap();
    let mut value = serde_json::to_value(&task).unwrap();
    value.as_object_mut().unwrap().remove("data_policy");
    value.as_object_mut().unwrap().remove("orchestration");
    let decoded: Task = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, task);
}
