use zaturo_core::{ChatAction, ChatRequest, Limits, ServiceRequest};
use zaturo_services::Engine;
fn message(text: &str, key: &str) -> ChatRequest {
    ChatRequest {
        message: text.into(),
        idempotency_key: key.into(),
        target_worker: Some("work-pc".into()),
        action: Some(ChatAction::Assistant),
    }
}
#[test]
fn exact_owner_video_request_queues_only_one_installed_executor_task() {
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    engine
        .register(
            "work-pc",
            &["diagnostic.v1".into(), "youtube.opencode.v1".into()],
            1000,
        )
        .unwrap();
    let request = message("faça 2 videos para o falllem price", "same");
    let first = engine.chat(request.clone(), "home", 1001).unwrap();
    let task = first.task.unwrap();
    assert_eq!(task.target_worker, "work-pc");
    assert_eq!(
        task.service,
        Some(ServiceRequest::YoutubeOpenCodeGenerate {
            channel: "fallem-prince".into(),
            count: 2
        })
    );
    assert_eq!(task.max_attempts, 1);
    let retry = engine.chat(request, "home", 1002).unwrap();
    assert_eq!(retry.task.unwrap().id, task.id);
    let not_a_command = engine
        .chat(
            message("não faça 2 videos para o falllem price", "negated"),
            "home",
            1003,
        )
        .unwrap();
    assert!(not_a_command.task.is_none());
    assert!(not_a_command.message.text.contains("não foi executada"));
}
#[test]
fn chat_uses_only_the_configured_assistant_worker_and_a_small_context() {
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    engine
        .register(
            "home",
            &["diagnostic.v1".into(), "assistant.route.v1".into()],
            1000,
        )
        .unwrap();
    for n in 0..6 {
        let mut request = message(
            &format!("Synthetic previous message {n} {}", "x".repeat(600)),
            &format!("history-{n}"),
        );
        request.action = None;
        engine.chat(request, "home", 1001 + n).unwrap();
    }
    let reply = engine
        .chat(
            message("Explique o que você pode fazer.", "conversation"),
            "home",
            2000,
        )
        .unwrap();
    let task = reply.task.unwrap();
    assert_eq!(task.target_worker, "home");
    let Some(ServiceRequest::AssistantRoute { context, .. }) = task.service else {
        panic!("wrong route")
    };
    assert!(context.len() <= 4);
    assert!(context.iter().map(|t| t.text.len()).sum::<usize>() <= 1024);
    assert!(
        context
            .iter()
            .all(|t| t.text.len() <= 512 && t.role == "owner")
    );
}
#[test]
fn shared_chat_is_an_inert_atomic_idempotent_handoff_with_real_author() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hub.db");
    let engine = Engine::open(&path, Limits::default()).unwrap();
    let request = message(
        "/compartilhar Inspect the latest public fixture after the owner requests it.",
        "handoff",
    );
    let reply = engine
        .chat_as(request.clone(), "home", "app:mobile", 1000)
        .unwrap();
    assert!(reply.task.is_none());
    assert!(
        reply
            .message
            .text
            .contains("não será executada automaticamente")
    );
    engine.chat_as(request, "home", "app:mobile", 1001).unwrap();
    let entries = engine.shared_list(Some("work-pc"), None, 20, 0).unwrap();
    assert_eq!(entries.len(), 1);
    let detail = engine.shared_read(&entries[0].id, None).unwrap();
    assert_eq!(detail.task.author, "app:mobile");
    assert!(detail.task.instructions.starts_with("Inspect"));
    drop(engine);
    let engine = Engine::open(&path, Limits::default()).unwrap();
    assert_eq!(
        engine
            .shared_list(Some("work-pc"), None, 20, 0)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn interpreted_action_creates_one_validated_child_and_rejects_changed_scope() {
    use zaturo_core::{IntentDecision, RegisteredAction, ServiceResult};
    let temp = tempfile::tempdir().unwrap();
    let e = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    e.register(
        "work-pc",
        &["assistant.route.v1".into(), "youtube.opencode.v1".into()],
        1000,
    )
    .unwrap();
    e.register("home", &["computer.agent.v1".into()], 1000)
        .unwrap();
    let reply = e
        .chat(
            message(
                "por favor, você pode gerar dois videos para o fallem prince?",
                "interpret-once",
            ),
            "home",
            1001,
        )
        .unwrap();
    let parent = reply.task.unwrap();
    assert_eq!(parent.target_worker, "work-pc");
    let assignment = e.claim("work-pc", 1002).unwrap().unwrap();
    e.start(&parent.id, "work-pc", &assignment.lease_token, 1003)
        .unwrap();
    let decision = IntentDecision::Action {
        action: RegisteredAction::YoutubeGenerate {
            channel: "zaflas".into(),
            count: 2,
        },
    };
    let mut result = ServiceResult {
        operation_id: parent.operation_id.clone(),
        action: "assistant_route".into(),
        outcome: "completed".into(),
        summary: "Pedido interpretado.".into(),
        data: serde_json::json!({"decision":decision,"model":"deepseek-v4.1-flash","model_calls":1,"usage":{"input_tokens":100,"output_tokens":50,"cost_microusd":0},"billing":{"source":"go","allowance_microusd":1}}),
    };
    assert!(
        e.complete_service(
            &parent.id,
            "work-pc",
            &assignment.lease_token,
            result.clone(),
            1004
        )
        .is_err()
    );
    result.data["decision"] = serde_json::to_value(IntentDecision::Action {
        action: RegisteredAction::YoutubeGenerate {
            channel: "fallem-prince".into(),
            count: 2,
        },
    })
    .unwrap();
    e.complete_service(
        &parent.id,
        "work-pc",
        &assignment.lease_token,
        result.clone(),
        1004,
    )
    .unwrap();
    e.complete_service(&parent.id, "work-pc", &assignment.lease_token, result, 1005)
        .unwrap();
    let tasks = e.list(20, 0).unwrap();
    assert_eq!(tasks.len(), 2);
    let child = tasks.iter().find(|t| t.id != parent.id).unwrap();
    assert_ne!(child.operation_id, parent.operation_id);
    assert_eq!(
        child.service,
        Some(ServiceRequest::YoutubeOpenCodeGenerate {
            channel: "fallem-prince".into(),
            count: 2
        })
    );
}

#[test]
fn an_existing_text_only_home_worker_keeps_ordinary_chat_compatibility() {
    let temp = tempfile::tempdir().unwrap();
    let e = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    e.register("home", &["assistant.chat.v1".into()], 1000)
        .unwrap();
    let reply = e
        .chat(
            message("Olá, explique suas funções.", "old-home-chat"),
            "home",
            1001,
        )
        .unwrap();
    assert!(matches!(
        reply.task.unwrap().service,
        Some(ServiceRequest::AssistantChat { .. })
    ));
}

#[test]
fn registered_public_research_cannot_expand_into_desktop_or_browser_input() {
    use zaturo_core::{
        ComputerAction, ComputerBackend, ComputerStepRequest, IntentDecision, RegisteredAction,
        ServiceResult,
    };
    let temp = tempfile::tempdir().unwrap();
    let e = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    e.register(
        "work-pc",
        &["assistant.route.v1".into(), "computer.action.v1".into()],
        1000,
    )
    .unwrap();
    e.register("home", &["computer.agent.v1".into()], 1000)
        .unwrap();
    let text = "Pesquise os preços do POCO X7 e X7 Pro.";
    let route = e
        .chat(message(text, "bounded-research"), "home", 1001)
        .unwrap()
        .task
        .unwrap();
    let work = e.claim("work-pc", 1002).unwrap().unwrap();
    e.start(&route.id, "work-pc", &work.lease_token, 1003)
        .unwrap();
    let result = ServiceResult {
        operation_id: route.operation_id.clone(),
        action: "assistant_route".into(),
        outcome: "completed".into(),
        summary: "Pedido interpretado.".into(),
        data: serde_json::json!({"decision":IntentDecision::Action{action:RegisteredAction::BrowserResearch{request:text.into()}},"model":"deepseek-v4.1-flash","model_calls":1,"usage":{"input_tokens":100,"output_tokens":50,"cost_microusd":0},"billing":{"source":"go"}}),
    };
    e.complete_service(&route.id, "work-pc", &work.lease_token, result, 1004)
        .unwrap();
    let parent = e.claim("home", 1005).unwrap().unwrap();
    e.start(&parent.task.id, "home", &parent.lease_token, 1006)
        .unwrap();
    let action = |backend, tool: &str| ComputerAction {
        backend,
        tool: tool.into(),
        arguments: serde_json::json!({}),
        target: serde_json::json!({}),
        expected_snapshot_sha256: None,
    };
    for proposed in [
        action(ComputerBackend::Desktop, "list_windows"),
        action(ComputerBackend::Browser, "browser_click"),
    ] {
        assert!(
            e.computer_step_create(
                &parent.task.id,
                "home",
                &parent.lease_token,
                ComputerStepRequest {
                    sequence: 1,
                    action: proposed
                },
                1007
            )
            .is_err()
        );
    }
    assert!(
        e.computer_step_create(
            &parent.task.id,
            "home",
            &parent.lease_token,
            ComputerStepRequest {
                sequence: 1,
                action: action(ComputerBackend::Browser, "doctor")
            },
            1007
        )
        .is_ok()
    );
}
