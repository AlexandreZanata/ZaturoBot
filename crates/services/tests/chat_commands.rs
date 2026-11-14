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
fn explicit_video_is_single_durable_generation_without_paid_chat_or_publication() {
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::open(temp.path().join("hub.db"), Limits::default()).unwrap();
    engine
        .register(
            "work-pc",
            &["diagnostic.v1".into(), "youtube.generate.v1".into()],
            1000,
        )
        .unwrap();
    let request = message("faça 4 videos do fallem price", "same");
    let first = engine.chat(request.clone(), "home", 1001).unwrap();
    let task = first.task.unwrap();
    assert_eq!(task.target_worker, "work-pc");
    assert_eq!(
        task.service,
        Some(ServiceRequest::YoutubeGenerate {
            channel: "fallem-prince".into(),
            count: 4
        })
    );
    assert_eq!(task.max_attempts, 1);
    let retry = engine.chat(request, "home", 1002).unwrap();
    assert_eq!(retry.task.unwrap().id, task.id);
    let not_a_command = engine
        .chat(
            message("não faça 4 videos do fallem price", "negated"),
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
            &["diagnostic.v1".into(), "assistant.chat.v1".into()],
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
    let Some(ServiceRequest::AssistantChat { context, .. }) = task.service else {
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
