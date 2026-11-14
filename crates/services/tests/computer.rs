//! Synthetic lease/approval/parent-child acceptance; no models, desktops or accounts are contacted.
use serde_json::{Value, json};
use zaturo_core::{
    Assignment, ChatAction, ChatRequest, ComputerAction, ComputerBackend, ComputerObservation,
    ComputerStep, ComputerStepRequest, ControlAction, FeedKind, FeedQuery, Limits, ServiceRequest,
    ServiceResult, TaskState,
};
use zaturo_services::Engine;
const PLANNER: &str = "home-planner";
const EXECUTOR: &str = "work-pc";
fn engine() -> Engine {
    let engine = Engine::open(":memory:", Limits::default()).unwrap();
    engine
        .register(
            PLANNER,
            &["diagnostic.v1".into(), "computer.agent.v1".into()],
            1000,
        )
        .unwrap();
    engine
        .register(
            EXECUTOR,
            &[
                "diagnostic.v1".into(),
                "computer.action.v1".into(),
                "price.search.v1".into(),
            ],
            1000,
        )
        .unwrap();
    engine
}
fn chat(text: &str, key: &str) -> ChatRequest {
    ChatRequest {
        message: text.into(),
        idempotency_key: key.into(),
        target_worker: Some(EXECUTOR.into()),
        action: Some(ChatAction::Assistant),
    }
}
fn confirm(text: &str, key: &str) -> ChatRequest {
    ChatRequest {
        action: Some(ChatAction::ComputerConfirm),
        ..chat(text, key)
    }
}
fn start_parent(e: &Engine, key: &str, at: i64) -> Assignment {
    let reply = e
        .chat_as(
            chat(
                "/computador Inspect the synthetic public browser fixture.",
                key,
            ),
            PLANNER,
            "app:owner",
            at,
        )
        .unwrap();
    let task = reply.task.unwrap();
    assert_eq!(task.target_worker, PLANNER);
    assert!(
        matches!(task.service,Some(ServiceRequest::ComputerTask{target_worker,..}) if target_worker==EXECUTOR)
    );
    let mut assignment = e.claim(PLANNER, at + 1).unwrap().unwrap();
    assignment.task = e
        .start(
            &assignment.task.id,
            PLANNER,
            &assignment.lease_token,
            at + 2,
        )
        .unwrap();
    assignment
}
fn parent(e: &Engine) -> Assignment {
    start_parent(e, "computer-parent", 1000)
}
fn read() -> ComputerAction {
    ComputerAction {
        backend: ComputerBackend::Browser,
        tool: "browser_snapshot".into(),
        arguments: json!({}),
        target: json!({"page_id":"synthetic-page"}),
        expected_snapshot_sha256: None,
    }
}
fn input() -> ComputerAction {
    ComputerAction {
        backend: ComputerBackend::Desktop,
        tool: "click".into(),
        arguments: json!({"element_index":2}),
        target: json!({"window_id":1,"pid":1234,"app_id":"synthetic-editor"}),
        expected_snapshot_sha256: Some("a".repeat(64)),
    }
}
fn desktop_read() -> ComputerAction {
    let mut action = input();
    action.tool = "get_app_state".into();
    action.arguments = json!({});
    action.expected_snapshot_sha256 = None;
    action
}
fn step(
    e: &Engine,
    parent: &Assignment,
    sequence: u32,
    action: ComputerAction,
    at: i64,
) -> ComputerStep {
    e.computer_step_create(
        &parent.task.id,
        PLANNER,
        &parent.lease_token,
        ComputerStepRequest { sequence, action },
        at,
    )
    .unwrap()
}
fn result(e: &Engine, child: &Assignment) -> ServiceResult {
    let task = e.get(&child.task.id).unwrap();
    let action = match &task.service {
        Some(ServiceRequest::ComputerStep { action, .. }) => action,
        _ => panic!("expected computer child"),
    };
    let data = if action.backend == ComputerBackend::Desktop {
        json!({"target":action.target,"nodes":[{"index":2,"role":"button","name":"Observed synthetic confirmation","text":"","actions":["click"],"states":[],"editable":false,"protected":false}]})
    } else {
        json!({"target":action.target,"text":"- button \"Observed synthetic link\" [ref=e1]"})
    };
    ServiceResult {
        operation_id: task.operation_id,
        action: "computer_step".into(),
        outcome: "completed".into(),
        summary: "Synthetic observed result.".into(),
        data: json!({"observation":ComputerObservation{outcome:"completed".into(),text:"Synthetic state was observed.".into(),data:json!({"schema":1,"status":"completed","snapshot_sha256":"a".repeat(64),"data":data})}}),
    }
}
fn complete_with(
    e: &Engine,
    child_id: &str,
    at: i64,
    raw: Option<Value>,
    outcome: &str,
) -> Assignment {
    let child = e.claim(EXECUTOR, at).unwrap().unwrap();
    assert_eq!(child.task.id, child_id);
    e.start(child_id, EXECUTOR, &child.lease_token, at + 1)
        .unwrap();
    let mut observation = result(e, &child);
    if let Some(raw) = raw {
        observation.data["observation"]["data"] = raw;
    }
    observation.data["observation"]["outcome"] = json!(outcome);
    e.complete_service(child_id, EXECUTOR, &child.lease_token, observation, at + 2)
        .unwrap();
    child
}
fn complete_read(e: &Engine, child_id: &str, at: i64) {
    complete_with(e, child_id, at, None, "completed");
}
fn seed_input(e: &Engine, p: &Assignment) {
    let observed = step(e, p, 1, desktop_read(), 1003);
    complete_read(e, observed.child_id.as_deref().unwrap(), 1004);
}
fn task_count(e: &Engine) -> usize {
    e.list(100, 0).unwrap().len()
}
fn task_feed(e: &Engine) -> Vec<zaturo_core::FeedItem> {
    e.feed(FeedQuery {
        day: "1970-01-01".into(),
        offset_minutes: 0,
        kind: Some(FeedKind::Task),
        q: None,
        limit: 100,
        offset: 0,
    })
    .unwrap()
    .items
}
#[test]
fn owner_chat_routes_to_planner_and_read_step_is_durable_once_with_current_lease() {
    let e = engine();
    let p = parent(&e);
    let first = step(&e, &p, 1, read(), 1003);
    assert_eq!(first.state, "queued");
    assert_eq!(task_count(&e), 2);
    let duplicate = step(&e, &p, 1, read(), 1004);
    assert_eq!(first.id, duplicate.id);
    assert_eq!(first.child_id, duplicate.child_id);
    assert_eq!(task_count(&e), 2);
    let request = ComputerStepRequest {
        sequence: 1,
        action: read(),
    };
    assert!(
        e.computer_step_create(&p.task.id, EXECUTOR, &p.lease_token, request.clone(), 1005)
            .is_err()
    );
    assert!(
        e.computer_step_create(&p.task.id, PLANNER, "forged-lease", request.clone(), 1005)
            .is_err()
    );
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 2,
                action: read()
            },
            1005
        )
        .is_err()
    );
    let mut changed = request;
    changed.action.target = json!({"page_id":"another-page"});
    assert!(
        e.computer_step_create(&p.task.id, PLANNER, &p.lease_token, changed, 1005)
            .is_err()
    );
    complete_read(&e, first.child_id.as_deref().unwrap(), 1006);
    let status = e
        .computer_step_status(&p.task.id, &first.id, PLANNER, &p.lease_token, 1010)
        .unwrap();
    assert_eq!(status.observation.unwrap().outcome, "completed");
    assert!(
        e.computer_step_status(&p.task.id, &first.id, EXECUTOR, &p.lease_token, 1010)
            .is_err()
    );
    let next = step(&e, &p, 2, read(), 1011);
    assert_ne!(next.id, first.id);
    assert_eq!(task_count(&e), 3);
}
#[test]
fn input_waits_for_observed_exact_owner_confirmation_and_replay_never_duplicates_child() {
    let e = engine();
    let p = parent(&e);
    seed_input(&e, &p);
    let action = input();
    let waiting = step(&e, &p, 2, action.clone(), 1010);
    assert_eq!(waiting.state, "waiting_approval");
    assert!(waiting.child_id.is_none());
    assert!(e.claim(EXECUTOR, 1011).unwrap().is_none());
    assert!(
        task_feed(&e)
            .iter()
            .any(|row| row.text.contains(&action.review())
                && row.text.contains("Observed synthetic confirmation")
                && row.text.contains(&format!("/aprovar {}", waiting.id)))
    );
    let command = format!("/aprovar {}", waiting.id);
    let mut inert = chat(&command, "legacy-inert");
    inert.action = None;
    e.chat_as(inert, PLANNER, "app:owner", 1012).unwrap();
    assert_eq!(task_count(&e), 2);
    let assistant = e
        .chat_as(
            chat(&command, "mcp-shaped-inert"),
            PLANNER,
            "app:owner",
            1012,
        )
        .unwrap();
    assert!(assistant.task.is_none());
    assert_eq!(task_count(&e), 2);
    assert!(e.claim(EXECUTOR, 1012).unwrap().is_none());
    assert!(
        e.computer_step_status(&p.task.id, &waiting.id, PLANNER, &p.lease_token, 1012)
            .unwrap()
            .step
            .child_id
            .is_none()
    );
    let approve = confirm(&command, "owner-approval");
    let reply = e
        .chat_as(approve.clone(), PLANNER, "app:owner", 1013)
        .unwrap();
    assert!(reply.task.is_none());
    assert_eq!(task_count(&e), 3);
    assert_eq!(
        e.chat_as(approve, PLANNER, "app:owner", 1014)
            .unwrap()
            .message
            .id,
        reply.message.id
    );
    e.chat_as(
        confirm(&command, "owner-approval-again"),
        PLANNER,
        "app:owner",
        1015,
    )
    .unwrap();
    assert_eq!(task_count(&e), 3);
    let approved = e
        .computer_step_status(&p.task.id, &waiting.id, PLANNER, &p.lease_token, 1016)
        .unwrap();
    assert_eq!(approved.step.action, action);
    let child = e.claim(EXECUTOR, 1017).unwrap().unwrap();
    assert_eq!(
        approved.step.child_id.as_deref(),
        Some(child.task.id.as_str())
    );
    assert!(
        matches!(child.task.service,Some(ServiceRequest::ComputerStep{owner_approved:true,ref action,..}) if *action==approved.step.action)
    );
    e.start(&child.task.id, EXECUTOR, &child.lease_token, 1018)
        .unwrap();
    assert!(
        e.complete_service(
            &child.task.id,
            PLANNER,
            &child.lease_token,
            result(&e, &child),
            1019
        )
        .is_err()
    );
    assert!(
        e.complete_service(
            &child.task.id,
            EXECUTOR,
            "wrong-executor-lease",
            result(&e, &child),
            1019
        )
        .is_err()
    );
    e.complete_service(
        &child.task.id,
        EXECUTOR,
        &child.lease_token,
        result(&e, &child),
        1019,
    )
    .unwrap();
    assert_eq!(
        e.computer_step_status(&p.task.id, &waiting.id, PLANNER, &p.lease_token, 1020)
            .unwrap()
            .observation
            .unwrap()
            .outcome,
        "completed"
    );
}
#[test]
fn missing_mismatched_and_protected_observed_elements_cannot_be_approved() {
    let e = engine();
    let p = parent(&e);
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 1,
                action: input()
            },
            1003
        )
        .is_err()
    );
    seed_input(&e, &p);
    for change in [
        "missing_hash",
        "wrong_hash",
        "unobserved_index",
        "wrong_target",
    ] {
        let mut action = input();
        match change {
            "missing_hash" => action.expected_snapshot_sha256 = None,
            "wrong_hash" => action.expected_snapshot_sha256 = Some("b".repeat(64)),
            "unobserved_index" => action.arguments = json!({"element_index":999}),
            _ => action.target["pid"] = json!(9999),
        };
        assert!(
            e.computer_step_create(
                &p.task.id,
                PLANNER,
                &p.lease_token,
                ComputerStepRequest {
                    sequence: 2,
                    action
                },
                1010
            )
            .is_err(),
            "accepted {change}"
        );
    }
    assert_eq!(task_count(&e), 2);
    let observed = step(&e, &p, 2, desktop_read(), 1011);
    let raw = json!({"schema":1,"status":"completed","snapshot_sha256":"b".repeat(64),"data":{"target":input().target,"nodes":[{"index":2,"role":"password","name":"[protected]","protected":true}]}});
    complete_with(
        &e,
        observed.child_id.as_deref().unwrap(),
        1012,
        Some(raw),
        "completed",
    );
    let mut action = input();
    action.expected_snapshot_sha256 = Some("b".repeat(64));
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 3,
                action
            },
            1015
        )
        .is_err()
    );
}
#[test]
fn browser_input_requires_an_observed_reference_in_the_same_parent_snapshot() {
    let e = engine();
    let p = parent(&e);
    let first = step(&e, &p, 1, read(), 1003);
    complete_read(&e, first.child_id.as_deref().unwrap(), 1004);
    let mut action = ComputerAction {
        backend: ComputerBackend::Browser,
        tool: "browser_click".into(),
        arguments: json!({"ref":"e2","element":"Invented label"}),
        target: read().target,
        expected_snapshot_sha256: Some("a".repeat(64)),
    };
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 2,
                action: action.clone()
            },
            1010
        )
        .is_err()
    );
    action.arguments = json!({"ref":"e1","element":"Model-supplied label is not authority"});
    let waiting = step(&e, &p, 2, action, 1011);
    assert_eq!(waiting.state, "waiting_approval");
    assert!(
        task_feed(&e)
            .iter()
            .any(|r| r.text.contains("Observed synthetic link") && r.text.contains("[ref=e1]"))
    );
}
#[test]
fn parent_pause_and_cancel_revoke_running_children_and_pending_approvals() {
    for control in [ControlAction::Pause, ControlAction::Cancel] {
        let e = engine();
        let p = parent(&e);
        let first = step(&e, &p, 1, read(), 1003);
        let child_id = first.child_id.unwrap();
        let child = e.claim(EXECUTOR, 1004).unwrap().unwrap();
        e.start(&child_id, EXECUTOR, &child.lease_token, 1005)
            .unwrap();
        e.control(&p.task.id, control, 1006).unwrap();
        let revoked = e.get(&child_id).unwrap();
        assert_eq!(revoked.state, TaskState::Cancelled);
        assert!(revoked.lease_token.is_none() && revoked.assigned_worker.is_none());
        assert!(
            e.complete_service(
                &child_id,
                EXECUTOR,
                &child.lease_token,
                result(&e, &child),
                1007
            )
            .is_err()
        );
        assert!(
            e.computer_step_create(
                &p.task.id,
                PLANNER,
                &p.lease_token,
                ComputerStepRequest {
                    sequence: 2,
                    action: input()
                },
                1007
            )
            .is_err()
        );
        let e = engine();
        let p = parent(&e);
        seed_input(&e, &p);
        let waiting = step(&e, &p, 2, input(), 1010);
        e.control(&p.task.id, control, 1011).unwrap();
        assert!(
            e.chat_as(
                confirm(&format!("/aprovar {}", waiting.id), "after-parent-stop"),
                PLANNER,
                "app:owner",
                1012
            )
            .is_err()
        );
        assert_eq!(task_count(&e), 2);
        assert!(e.claim(EXECUTOR, 1012).unwrap().is_none());
    }
}
#[test]
fn expired_planner_lease_cannot_authorize_input_before_recovery_runs() {
    let e = engine();
    let p = parent(&e);
    seed_input(&e, &p);
    let waiting = step(&e, &p, 2, input(), 1010);
    assert!(
        e.chat_as(
            confirm(&format!("/aprovar {}", waiting.id), "expired-parent-lease"),
            PLANNER,
            "app:owner",
            31_002
        )
        .is_err()
    );
    assert_eq!(task_count(&e), 2);
    assert!(e.claim(EXECUTOR, 31_003).unwrap().is_none());
}
#[test]
fn approval_expiry_and_parent_lease_recovery_never_leave_executable_children() {
    let e = engine();
    let p = parent(&e);
    seed_input(&e, &p);
    let waiting = step(&e, &p, 2, input(), 1010);
    for at in (20_000..=180_000).step_by(20_000) {
        e.heartbeat(PLANNER, Some(&p.lease_token), at).unwrap();
    }
    let status = e
        .computer_step_status(
            &p.task.id,
            &waiting.id,
            PLANNER,
            &p.lease_token,
            waiting.expires_at,
        )
        .unwrap();
    assert_eq!(status.step.state, "expired");
    assert_eq!(status.observation.unwrap().outcome, "blocked");
    assert!(
        e.chat_as(
            confirm(
                &format!("/aprovar {}", waiting.id),
                "expired-owner-confirmation"
            ),
            PLANNER,
            "app:owner",
            waiting.expires_at
        )
        .is_err()
    );
    assert_eq!(task_count(&e), 2);
    let e = engine();
    let p = parent(&e);
    let child_step = step(&e, &p, 1, read(), 30_000);
    let child_id = child_step.child_id.unwrap();
    let child = e.claim(EXECUTOR, 30_001).unwrap().unwrap();
    e.start(&child_id, EXECUTOR, &child.lease_token, 30_002)
        .unwrap();
    e.recover(31_002).unwrap();
    assert_eq!(e.get(&p.task.id).unwrap().state, TaskState::Failed);
    let revoked = e.get(&child_id).unwrap();
    assert_eq!(revoked.state, TaskState::Cancelled);
    assert!(revoked.lease_token.is_none());
    assert!(
        e.complete_service(
            &child_id,
            EXECUTOR,
            &child.lease_token,
            result(&e, &child),
            31_003
        )
        .is_err()
    );
    assert!(e.claim(EXECUTOR, 31_003).unwrap().is_none());
}
#[test]
fn executor_grant_is_bound_to_running_child_and_never_outlives_planner_lease() {
    let e = engine();
    let p = parent(&e);
    let queued = step(&e, &p, 1, read(), 1003);
    let child = e.claim(EXECUTOR, 1004).unwrap().unwrap();
    assert_eq!(queued.child_id.as_deref(), Some(child.task.id.as_str()));
    assert!(
        e.computer_action_grant(&child.task.id, EXECUTOR, &child.lease_token, 1005)
            .is_err()
    );
    e.start(&child.task.id, EXECUTOR, &child.lease_token, 1006)
        .unwrap();
    let grant = e
        .computer_action_grant(&child.task.id, EXECUTOR, &child.lease_token, 1007)
        .unwrap();
    assert_eq!(grant.operation_id, child.task.operation_id);
    assert_eq!(grant.lease_token, child.lease_token);
    assert_eq!(grant.lease_expires_at, p.lease_expires_at);
    assert!(!grant.owner_approved);
    assert!(grant.deadline_at <= child.task.deadline_at && grant.deadline_at <= p.task.deadline_at);
    assert!(
        e.computer_action_grant(&child.task.id, PLANNER, &child.lease_token, 1007)
            .is_err()
    );
    assert!(
        e.computer_action_grant(&child.task.id, EXECUTOR, "forged-lease", 1007)
            .is_err()
    );
    assert!(
        e.computer_action_grant(
            &child.task.id,
            EXECUTOR,
            &child.lease_token,
            p.lease_expires_at
        )
        .is_err()
    );
    e.control(&p.task.id, ControlAction::Cancel, 1008).unwrap();
    assert!(
        e.computer_action_grant(&child.task.id, EXECUTOR, &child.lease_token, 1009)
            .is_err()
    );
}
#[test]
fn blocked_and_uncertain_observations_stop_the_parent_from_queueing_another_step() {
    for outcome in ["blocked", "uncertain"] {
        let e = engine();
        let p = parent(&e);
        let first = step(&e, &p, 1, read(), 1003);
        complete_with(&e, first.child_id.as_deref().unwrap(), 1004, None, outcome);
        assert!(
            e.computer_step_create(
                &p.task.id,
                PLANNER,
                &p.lease_token,
                ComputerStepRequest {
                    sequence: 2,
                    action: read()
                },
                1010
            )
            .is_err()
        );
        assert_eq!(task_count(&e), 2);
    }
}
#[test]
fn social_execute_binds_to_same_parent_prepared_operation_hash_and_observed_preview() {
    let e = engine();
    let p = parent(&e);
    let operation = "00000000-0000-4000-8000-000000000010";
    let hash = "a".repeat(64);
    let execute = ComputerAction {
        backend: ComputerBackend::Social,
        tool: "execute".into(),
        arguments: json!({"operation_id":operation,"approved_draft_hash":hash}),
        target: json!({}),
        expected_snapshot_sha256: None,
    };
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 1,
                action: execute.clone()
            },
            1003
        )
        .is_err()
    );
    let prepare = ComputerAction {
        backend: ComputerBackend::Social,
        tool: "prepare".into(),
        arguments: json!({"action":"publish","integration_id":"synthetic-account","content":"Original content for the exact account."}),
        target: json!({}),
        expected_snapshot_sha256: None,
    };
    let prepared = step(&e, &p, 1, prepare, 1004);
    let raw = json!({"schema":1,"status":"prepared","operation_id":operation,"draft_hash":hash,"preview_pt":"Conta autorizada: Synthetic account\nTexto exato: Original content for the exact account.","payload":{"content":"Original content for the exact account."}});
    complete_with(
        &e,
        prepared.child_id.as_deref().unwrap(),
        1005,
        Some(raw),
        "completed",
    );
    for field in ["operation_id", "approved_draft_hash"] {
        let mut altered = execute.clone();
        altered.arguments[field] = if field == "operation_id" {
            json!("00000000-0000-4000-8000-000000000011")
        } else {
            json!("b".repeat(64))
        };
        assert!(
            e.computer_step_create(
                &p.task.id,
                PLANNER,
                &p.lease_token,
                ComputerStepRequest {
                    sequence: 2,
                    action: altered
                },
                1010
            )
            .is_err()
        );
    }
    let waiting = step(&e, &p, 2, execute.clone(), 1011);
    assert!(waiting.child_id.is_none());
    assert!(
        task_feed(&e)
            .iter()
            .any(|r| r.text.contains("Rascunho exato preparado")
                && r.text.contains("Original content for the exact account.")
                && r.text.contains("Synthetic account"))
    );
    e.chat_as(
        confirm(&format!("/aprovar {}", waiting.id), "approve-social"),
        PLANNER,
        "app:owner",
        1012,
    )
    .unwrap();
    let child = e.claim(EXECUTOR, 1013).unwrap().unwrap();
    assert!(
        matches!(child.task.service,Some(ServiceRequest::ComputerStep{owner_approved:true,ref action,..})if *action==execute)
    );
}

#[test]
fn a_prepared_draft_from_another_parent_cannot_authorize_publication() {
    let e = engine();
    let p = parent(&e);
    let operation = "00000000-0000-4000-8000-000000000020";
    let prepare = ComputerAction {
        backend: ComputerBackend::Social,
        tool: "prepare".into(),
        arguments: json!({"action":"draft","integration_id":"synthetic-account","content":"Synthetic first-session content"}),
        target: json!({}),
        expected_snapshot_sha256: None,
    };
    let prepared = step(&e, &p, 1, prepare, 1003);
    complete_with(
        &e,
        prepared.child_id.as_deref().unwrap(),
        1004,
        Some(
            json!({"schema":1,"status":"prepared","operation_id":operation,"draft_hash":"a".repeat(64),"preview_pt":"Synthetic exact first-session preview"}),
        ),
        "completed",
    );
    e.complete_service(
        &p.task.id,
        PLANNER,
        &p.lease_token,
        ServiceResult {
            operation_id: p.task.operation_id,
            action: "computer_task".into(),
            outcome: "completed".into(),
            summary: "Prepared a synthetic draft only; nothing published.".into(),
            data: json!({"steps_used":1}),
        },
        1010,
    )
    .unwrap();
    let another = start_parent(&e, "second-computer-parent", 2000);
    let execute = ComputerAction {
        backend: ComputerBackend::Social,
        tool: "execute".into(),
        arguments: json!({"operation_id":operation,"approved_draft_hash":"a".repeat(64)}),
        target: json!({}),
        expected_snapshot_sha256: None,
    };
    assert!(
        e.computer_step_create(
            &another.task.id,
            PLANNER,
            &another.lease_token,
            ComputerStepRequest {
                sequence: 1,
                action: execute
            },
            2003
        )
        .is_err()
    );
    assert_eq!(task_count(&e), 3);
}

#[test]
fn a_parent_has_at_most_eight_sequential_steps_even_after_successful_reads() {
    let e = engine();
    let p = parent(&e);
    for sequence in 1..=8 {
        let at = 1000 + i64::from(sequence) * 10;
        let current = step(&e, &p, sequence, read(), at);
        complete_read(&e, current.child_id.as_deref().unwrap(), at + 1);
    }
    assert_eq!(task_count(&e), 9);
    assert!(
        e.computer_step_create(
            &p.task.id,
            PLANNER,
            &p.lease_token,
            ComputerStepRequest {
                sequence: 9,
                action: read()
            },
            1100
        )
        .is_err()
    );
    assert_eq!(task_count(&e), 9);
}

#[test]
fn missing_executor_reports_its_actual_worker_and_capability_without_planner_calls() {
    let e = Engine::open(":memory:", Limits::default()).unwrap();
    e.register(
        PLANNER,
        &["diagnostic.v1".into(), "computer.agent.v1".into()],
        1000,
    )
    .unwrap();
    e.register(EXECUTOR, &["diagnostic.v1".into()], 1000)
        .unwrap();
    let reply = e
        .chat_as(
            chat("/computador Inspect a public page", "missing-executor"),
            PLANNER,
            "app:owner",
            1001,
        )
        .unwrap();
    assert!(reply.task.is_none());
    assert!(reply.message.text.contains(EXECUTOR));
    assert!(reply.message.text.contains("computer.action.v1"));
    assert!(!reply.message.text.contains("computer.agent.v1"));
    assert!(e.claim(PLANNER, 1002).unwrap().is_none());
}
