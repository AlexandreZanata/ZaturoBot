use zaturo_core::{
    ChatAction, ChatRequest, FeedKind, FeedQuery, Limits, NewTask, ServiceRequest, ServiceResult,
    TaskKind, TaskState,
};
use zaturo_services::{Engine, Error};
fn query(day: &str) -> FeedQuery {
    FeedQuery {
        day: day.into(),
        offset_minutes: -240,
        kind: None,
        q: None,
        limit: 20,
        offset: 0,
    }
}
#[test]
fn daily_history_respects_local_midnight_unicode_literal_search_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hub.db");
    let engine = Engine::open(&path, Limits::default()).unwrap();
    let item = |text: &str, key: &str, _now: i64| ChatRequest {
        message: text.into(),
        idempotency_key: key.into(),
        target_worker: None,
        action: None,
    };
    engine
        .chat(item("Antes da meia-noite", "before", 0), "pc", 14_399_998)
        .unwrap();
    let req = item("DIAGNÓSTICO solicitado", "after", 0);
    let reply = engine.chat(req.clone(), "pc", 14_400_000).unwrap();
    assert_eq!(
        engine
            .chat(req.clone(), "pc", 14_500_000)
            .unwrap()
            .message
            .id,
        reply.message.id
    );
    let mut collision = req;
    collision.message = "changed".into();
    assert!(matches!(
        engine.chat(collision, "pc", 15_000_000),
        Err(Error::Conflict(_))
    ));
    assert_eq!(engine.feed(query("1969-12-31")).unwrap().items.len(), 2);
    let mut search = query("1970-01-01");
    search.q = Some("diagnóstico solicitado".into());
    let page = engine.feed(search).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].role, "owner");
    let mut literal = query("1970-01-01");
    literal.q = Some("%".into());
    assert!(engine.feed(literal).unwrap().items.is_empty());
    let mut first = query("1970-01-01");
    first.limit = 1;
    assert!(engine.feed(first).unwrap().has_more);
    assert!(engine.feed(query("2026-02-30")).is_err());
    drop(engine);
    let engine = Engine::open(&path, Limits::default()).unwrap();
    assert_eq!(engine.feed(query("1970-01-01")).unwrap().items.len(), 2);
}
#[test]
fn typed_chat_task_completion_is_atomic_and_generic_chat_never_executes() {
    let e = Engine::open(":memory:", Limits::default()).unwrap();
    e.register("pc", &["diagnostic.v1".into()], 14_400_000)
        .unwrap();
    let req = ChatRequest {
        message: "Verifique este computador".into(),
        idempotency_key: "task-one".into(),
        target_worker: Some("pc".into()),
        action: Some(ChatAction::Diagnostic),
    };
    let reply = e.chat(req, "pc", 14_400_001).unwrap();
    let task = reply.task.unwrap();
    assert_eq!(task.state, TaskState::Queued);
    let a = e.claim("pc", 14_400_002).unwrap().unwrap();
    e.start(&task.id, "pc", &a.lease_token, 14_400_003).unwrap();
    let result = zaturo_core::DiagnosticResult {
        operation_id: task.operation_id.clone(),
        tool: "diagnostic.v1".into(),
        os: "linux".into(),
        arch: "test".into(),
        success: true,
        summary: "Verified fixture".into(),
    };
    e.complete(&task.id, "pc", &a.lease_token, result.clone(), 14_400_004)
        .unwrap();
    e.complete(&task.id, "pc", &a.lease_token, result, 14_400_005)
        .unwrap();
    let mut q = query("1970-01-01");
    q.kind = Some(FeedKind::Task);
    assert_eq!(e.feed(q).unwrap().items.len(), 1);
    let req = ChatRequest {
        message: "arbitrary request remains data".into(),
        idempotency_key: "free-text".into(),
        target_worker: None,
        action: None,
    };
    assert!(e.chat(req, "pc", 14_400_006).unwrap().task.is_none());
    let item = zaturo_core::FeedItem {
        id: "private".into(),
        created_at: 14_400_000,
        kind: FeedKind::Android,
        role: "assistant".into(),
        text: "synthetic sensitive".into(),
        task_id: None,
        artifact_id: None,
        sensitive: true,
    };
    assert!(e.publish_activity(item, "no-sensitive-screens").is_err());
}
#[test]
fn service_kinds_capabilities_fence_completion_and_job_updates() {
    let e = Engine::open(":memory:", Limits::default()).unwrap();
    e.register(
        "work",
        &[
            "diagnostic.v1".into(),
            "youtube.read.v1".into(),
            "youtube.submit.v1".into(),
        ],
        14_400_000,
    )
    .unwrap();
    let mut req = NewTask::diagnostic("work", "service-one");
    req.kind = TaskKind::Service;
    req.service = Some(ServiceRequest::YoutubeSubmit {
        channel: "channel".into(),
        account: "account".into(),
        count: 1,
        approved_schedule: true,
    });
    let task = e.create(req, 14_400_001).unwrap();
    assert_eq!(task.required_capabilities, ["youtube.submit.v1"]);
    let a = e.claim("work", 14_400_002).unwrap().unwrap();
    e.start(&task.id, "work", &a.lease_token, 14_400_003)
        .unwrap();
    let result = ServiceResult {
        operation_id: task.operation_id.clone(),
        action: "youtube_submit".into(),
        outcome: "accepted".into(),
        summary: "Job accepted; processing is still running".into(),
        data: serde_json::json!({"state":"accepted"}),
    };
    e.complete_service(&task.id, "work", &a.lease_token, result, 14_400_004)
        .unwrap();
    let update = zaturo_core::ServiceActivity {
        operation_id: task.operation_id,
        state: "completed".into(),
        summary: "One scheduled video verified".into(),
    };
    assert!(
        e.service_activity("other", update.clone(), 14_400_005)
            .is_err()
    );
    let first = e
        .service_activity("work", update.clone(), 14_400_005)
        .unwrap();
    assert_eq!(
        e.service_activity("work", update, 14_400_006).unwrap().id,
        first.id
    );
}
