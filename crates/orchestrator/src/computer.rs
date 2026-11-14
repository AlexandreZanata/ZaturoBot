//! Model-independent, bounded semantic computer planning over a remotely authorized port.
use crate::{Error, FutureResult};
use serde::Deserialize;
use zaturo_core::{ComputerAction, ComputerObservation};
use zaturo_llm::{BudgetPort, CallContext, Cancellation, Message, PrivacyClass, Router};
pub trait ComputerExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        sequence: u32,
        action: ComputerAction,
    ) -> FutureResult<'a, ComputerObservation>;
}
#[derive(Debug)]
pub struct ComputerOutcome {
    pub outcome: String,
    pub summary: String,
    pub actions: u32,
    pub model_calls: u32,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Proposal {
    Action { action: ComputerAction },
    Finish { summary: String },
}
fn compact(value: &serde_json::Value, depth: u32) -> serde_json::Value {
    if depth > 5 {
        return serde_json::Value::Null;
    }
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(if s.starts_with("https://") {
            if s.len() <= 1024 {
                s.clone()
            } else {
                "[URL omitted; request the exact observed link]".into()
            }
        } else {
            s.chars().take(400).collect()
        }),
        serde_json::Value::Array(v) => {
            serde_json::Value::Array(v.iter().take(12).map(|v| compact(v, depth + 1)).collect())
        }
        serde_json::Value::Object(v) => serde_json::Value::Object(
            v.iter()
                .filter(|(key, _)| {
                    ![
                        "image",
                        "screenshot",
                        "base64",
                        "cookie",
                        "password",
                        "authorization",
                        "identity",
                    ]
                    .iter()
                    .any(|s| key.to_lowercase().contains(s))
                })
                .take(20)
                .map(|(k, v)| (k.clone(), compact(v, depth + 1)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
pub async fn computer_run(
    router: &Router,
    context: &CallContext,
    goal: &str,
    budget: &dyn BudgetPort,
    executor: &dyn ComputerExecutor,
    cancel: &Cancellation,
) -> Result<ComputerOutcome, Error> {
    if !router.config().is_personal_route()
        || context.privacy != PrivacyClass::Personal
        || router.config().max_attempts != 1
        || !zaturo_core::ordinary_text(goal, 2048)
    {
        return Err(Error::PolicyDenied);
    }
    let mut pending_verification: Vec<ComputerAction> = Vec::new();
    let mut prepared_social = std::collections::BTreeMap::<String, String>::new();
    let mut actions = 0;
    let mut receipts = Vec::new();
    let mut observation = serde_json::json!({"state":"not_observed","instructions":"Discover granted windows/browser state; no action has run yet."});
    for call in 1..=8 {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let step_context = CallContext {
            step: call,
            ..context.clone()
        };
        let messages=vec![Message::system(include_str!("../../../prompts/computer-planner.md")),Message::user(serde_json::json!({"owner_goal":goal,"verified_receipts":receipts,"latest_untrusted_observation":observation,"remaining_model_calls":9-call}).to_string())];
        let response = router
            .complete(&step_context, messages, budget, cancel)
            .await?;
        let proposal: Proposal =
            serde_json::from_str(&response.content).map_err(|_| Error::InvalidProposal)?;
        match proposal {
            Proposal::Finish { summary } => {
                if actions == 0 || !zaturo_core::ordinary_text(&summary, 1500) {
                    return Err(Error::InvalidProposal);
                }
                if !pending_verification.is_empty() {
                    return Ok(ComputerOutcome {
                        outcome: "partial".into(),
                        summary: format!(
                            "Etapas observadas: {actions}. Um comando foi enviado, mas seu efeito ainda não foi verificado por uma nova leitura. Não afirmo publicação ou conclusão."
                        ),
                        actions,
                        model_calls: call,
                    });
                }
                return Ok(ComputerOutcome {
                    outcome: "completed".into(),
                    summary: format!("Etapas verificadas: {actions}. Relato da IA: {summary}"),
                    actions,
                    model_calls: call,
                });
            }
            Proposal::Action { action } => {
                if !action.validates() {
                    return Err(Error::PolicyDenied);
                }
                let receipt = executor.execute(actions + 1, action.clone()).await?;
                if !receipt.validates() {
                    return Err(Error::ToolFailed);
                }
                actions += 1;
                if action.backend == zaturo_core::ComputerBackend::Social
                    && action.tool == "prepare"
                    && receipt.outcome == "completed"
                {
                    if let (Some(op), Some(kind)) = (
                        receipt
                            .data
                            .get("operation_id")
                            .and_then(serde_json::Value::as_str),
                        receipt
                            .data
                            .get("payload")
                            .and_then(|p| p.get("type"))
                            .and_then(serde_json::Value::as_str),
                    ) {
                        prepared_social.insert(
                            op.into(),
                            match kind {
                                "now" => "published",
                                "schedule" => "scheduled",
                                "draft" => "drafted",
                                _ => "unknown",
                            }
                            .into(),
                        );
                    }
                    let mut prepared = action.clone();
                    if let Some(op) = receipt
                        .data
                        .get("operation_id")
                        .and_then(serde_json::Value::as_str)
                    {
                        prepared.arguments["operation_id"] = serde_json::json!(op);
                    }
                    pending_verification.push(prepared);
                } else if action.requires_approval() {
                    if action.backend == zaturo_core::ComputerBackend::Social
                        && action.tool == "execute"
                    {
                        pending_verification.retain(|previous| {
                            !(previous.backend == action.backend
                                && previous.tool == "prepare"
                                && previous.arguments.get("operation_id")
                                    == action.arguments.get("operation_id"))
                        });
                    }
                    pending_verification.push(action.clone());
                } else if receipt.outcome == "completed"
                    || (action.backend == zaturo_core::ComputerBackend::Social
                        && action.tool == "status"
                        && receipt.outcome == "partial"
                        && receipt
                            .data
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            == Some("scheduled"))
                {
                    pending_verification.retain(|previous| {
                        if previous.backend != action.backend {
                            return true;
                        }
                        let verified = match action.backend {
                            zaturo_core::ComputerBackend::Desktop => {
                                action.tool == "get_app_state" && action.target == previous.target
                            }
                            zaturo_core::ComputerBackend::Browser => {
                                action.tool == "browser_snapshot"
                                    && action.target == previous.target
                            }
                            zaturo_core::ComputerBackend::Social => {
                                action.tool == "status"
                                    && previous.tool == "execute"
                                    && action.arguments.get("operation_id")
                                        == previous.arguments.get("operation_id")
                                    && receipt.data.get("operation_id")
                                        == action.arguments.get("operation_id")
                                    && action
                                        .arguments
                                        .get("operation_id")
                                        .and_then(serde_json::Value::as_str)
                                        .and_then(|op| prepared_social.get(op))
                                        .is_some_and(|expected| {
                                            receipt
                                                .data
                                                .get("status")
                                                .and_then(serde_json::Value::as_str)
                                                == Some(expected.as_str())
                                        })
                            }
                            _ => false,
                        };
                        !verified
                    });
                }
                receipts.push(serde_json::json!({"step":actions,"backend":action.backend,"tool":action.tool,"outcome":receipt.outcome}));
                if matches!(receipt.outcome.as_str(), "blocked" | "uncertain") {
                    return Ok(ComputerOutcome {
                        outcome: "blocked".into(),
                        summary: receipt
                            .text
                            .char_indices()
                            .take_while(|(i, c)| i + c.len_utf8() <= 1500)
                            .map(|(_, c)| c)
                            .collect(),
                        actions,
                        model_calls: call,
                    });
                }
                // Only the latest capped semantic observation; no screenshot, long history or reasoning.
                let data = compact(&receipt.data, 0);
                let data = if serde_json::to_vec(&data).is_ok_and(|b| b.len() <= 3500) {
                    data
                } else {
                    serde_json::json!({"clipped":true,"instruction":"Request a smaller semantic tree page; omitted data is not evidence."})
                };
                observation = serde_json::json!({"outcome":receipt.outcome,"text":receipt.text.chars().take(800).collect::<String>(),"data":data});
            }
        }
    }
    Ok(ComputerOutcome {
        outcome: "partial".into(),
        summary: format!(
            "Foram verificadas {actions} etapas. O limite desta tarefa foi atingido; não afirmo que o objetivo inteiro foi concluído."
        ),
        actions,
        model_calls: 8,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use zaturo_llm::{
        ModelResponse, ModelsConfig,
        fake::{FakeBudget, FakeClient, FakeReply},
    };
    struct Executor(AtomicUsize);
    impl ComputerExecutor for Executor {
        fn execute<'a>(
            &'a self,
            _: u32,
            _: ComputerAction,
        ) -> FutureResult<'a, ComputerObservation> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(ComputerObservation {
                    outcome: "completed".into(),
                    text: "Uma janela de teste foi observada; nenhuma publicação ocorreu.".into(),
                    data: serde_json::json!({"window_id":3}),
                })
            })
        }
    }
    fn response(v: serde_json::Value) -> FakeReply {
        FakeReply::Answer(ModelResponse {
            billing: Some(zaturo_llm::ModelBilling {
                source: zaturo_llm::BillingSource::Go,
                allowance_microusd: Some(0),
            }),
            model: zaturo_llm::DEEPSEEK_MODEL.into(),
            content: v.to_string(),
            usage: zaturo_core::ModelUsage {
                input_tokens: 20,
                output_tokens: 20,
                cost_microusd: Some(0),
            },
            finish_reason: "stop".into(),
            duration_ms: 1,
            model_calls: 1,
            usage_complete: true,
        })
    }
    fn setup(replies: Vec<FakeReply>) -> (Router, Arc<FakeClient>, FakeBudget, CallContext) {
        let mut c: ModelsConfig =
            toml::from_str(include_str!("../../../config/computer-models.example.toml")).unwrap();
        c.enabled = true;
        c.allow_global_processing = true;
        let f = Arc::new(FakeClient::new(replies));
        let r = Router::new(c, f.clone()).unwrap();
        (
            r,
            f,
            FakeBudget::default(),
            CallContext {
                task_id: "computer-test".into(),
                operation_id: "op-test".into(),
                step: 1,
                deadline_at: i64::MAX,
                privacy: PrivacyClass::Personal,
            },
        )
    }
    #[tokio::test]
    async fn cannot_finish_before_real_observation_or_grant_shell() {
        for action in [
            serde_json::json!({"type":"finish","summary":"I published it"}),
            serde_json::json!({"type":"action","action":{"backend":"desktop","tool":"run_shell","arguments":{},"target":{}}}),
        ] {
            let (r, f, b, c) = setup(vec![response(action)]);
            let e = Executor(AtomicUsize::new(0));
            assert!(
                computer_run(&r, &c, "Synthetic owner goal", &b, &e, &Cancellation::new())
                    .await
                    .is_err()
            );
            assert_eq!(f.calls(), 1);
            assert_eq!(e.0.load(Ordering::SeqCst), 0);
        }
    }
    #[tokio::test]
    async fn actual_receipt_precedes_bounded_finish() {
        let (r, f, b, c) = setup(vec![
            response(
                serde_json::json!({"type":"action","action":{"backend":"desktop","tool":"list_windows","arguments":{},"target":{}}}),
            ),
            response(serde_json::json!({"type":"finish","summary":"A janela foi listada."})),
        ]);
        let e = Executor(AtomicUsize::new(0));
        let out = computer_run(
            &r,
            &c,
            "List the granted window",
            &b,
            &e,
            &Cancellation::new(),
        )
        .await
        .unwrap();
        assert_eq!(out.actions, 1);
        assert_eq!(f.calls(), 2);
    }
    mod verification {
        use super::*;
        use serde_json::{Value, json};
        use std::{collections::VecDeque, sync::Mutex};
        use zaturo_core::ComputerBackend;

        struct ObservedSequence {
            remaining: Mutex<VecDeque<(ComputerAction, ComputerObservation)>>,
            calls: AtomicUsize,
        }
        impl ComputerExecutor for ObservedSequence {
            fn execute<'a>(
                &'a self,
                sequence: u32,
                action: ComputerAction,
            ) -> FutureResult<'a, ComputerObservation> {
                Box::pin(async move {
                    assert_eq!(
                        sequence as usize,
                        self.calls.fetch_add(1, Ordering::SeqCst) + 1
                    );
                    let (expected, receipt) = self.remaining.lock().unwrap().pop_front().unwrap();
                    assert_eq!(
                        action, expected,
                        "planner changed the scripted synthetic action"
                    );
                    Ok(receipt)
                })
            }
        }
        fn action(
            backend: ComputerBackend,
            tool: &str,
            arguments: Value,
            target: Value,
        ) -> ComputerAction {
            let mut result = ComputerAction {
                backend,
                tool: tool.into(),
                arguments,
                target,
                expected_snapshot_sha256: None,
            };
            if result.requires_approval() && backend != ComputerBackend::Social {
                result.expected_snapshot_sha256 = Some("a".repeat(64));
            }
            assert!(result.validates());
            result
        }
        fn receipt(outcome: &str, data: Value) -> ComputerObservation {
            ComputerObservation {
                outcome: outcome.into(),
                text: "Synthetic adapter observation; no live account or desktop was contacted."
                    .into(),
                data,
            }
        }
        fn desktop(tool: &str, id: u64) -> ComputerAction {
            action(
                ComputerBackend::Desktop,
                tool,
                if tool == "click" {
                    json!({"element_index":1})
                } else {
                    json!({})
                },
                json!({"window_id":id,"pid":700+id,"app_id":"synthetic-editor"}),
            )
        }
        fn browser(tool: &str) -> ComputerAction {
            action(
                ComputerBackend::Browser,
                tool,
                if tool == "browser_click" {
                    json!({"element":"Synthetic button","ref":"e1"})
                } else {
                    json!({})
                },
                json!({"page_id":"synthetic-page"}),
            )
        }
        async fn run_sequence(
            steps: Vec<(ComputerAction, ComputerObservation)>,
            final_claim: &str,
        ) -> ComputerOutcome {
            let expected = steps.len();
            let mut replies = steps
                .iter()
                .map(|(action, _)| response(json!({"type":"action","action":action})))
                .collect::<Vec<_>>();
            replies.push(response(json!({"type":"finish","summary":final_claim})));
            let (router, fake, budget, context) = setup(replies);
            let executor = ObservedSequence {
                remaining: Mutex::new(steps.into()),
                calls: AtomicUsize::new(0),
            };
            let out = computer_run(
                &router,
                &context,
                "Perform only the scripted synthetic task and report its actual evidence.",
                &budget,
                &executor,
                &Cancellation::new(),
            )
            .await
            .unwrap();
            assert_eq!(out.actions as usize, expected);
            assert_eq!(out.model_calls as usize, expected + 1);
            assert_eq!(fake.calls(), expected + 1);
            assert_eq!(executor.calls.load(Ordering::SeqCst), expected);
            assert!(executor.remaining.lock().unwrap().is_empty());
            out
        }

        #[tokio::test]
        async fn input_dispatch_then_finish_cannot_promote_an_invented_success() {
            for adapter_outcome in ["partial", "completed"] {
                let out = run_sequence(
                    vec![(
                        desktop("click", 1),
                        receipt(
                            adapter_outcome,
                            json!({"dispatched":true,"verified_external_effect":false}),
                        ),
                    )],
                    "Synthetic model claims the whole task was completed.",
                )
                .await;
                assert_eq!(out.outcome, "partial");
                assert!(out.summary.contains("ainda não foi verificado"));
                assert!(!out.summary.contains("Synthetic model claims"));
            }
        }

        #[tokio::test]
        async fn every_pending_target_and_backend_requires_its_own_fresh_observation() {
            let input = || {
                receipt(
                    "partial",
                    json!({"dispatched":true,"verified_external_effect":false}),
                )
            };
            let observed = || {
                receipt(
                    "completed",
                    json!({"schema":1,"status":"completed","synthetic_state":"observed"}),
                )
            };
            let scenarios = vec![
                (
                    "second window does not erase the first",
                    vec![
                        (desktop("click", 1), input()),
                        (desktop("click", 2), input()),
                        (desktop("get_app_state", 2), observed()),
                    ],
                    "partial",
                ),
                (
                    "browser observation does not verify a desktop input",
                    vec![
                        (desktop("click", 1), input()),
                        (browser("browser_click"), input()),
                        (browser("browser_snapshot"), observed()),
                    ],
                    "partial",
                ),
                (
                    "another window cannot verify the target",
                    vec![
                        (desktop("click", 1), input()),
                        (desktop("get_app_state", 2), observed()),
                    ],
                    "partial",
                ),
                (
                    "doctor metadata cannot verify input",
                    vec![
                        (desktop("click", 1), input()),
                        (desktop("doctor", 1), observed()),
                    ],
                    "partial",
                ),
                (
                    "both backends are independently observed",
                    vec![
                        (desktop("click", 1), input()),
                        (browser("browser_click"), input()),
                        (browser("browser_snapshot"), observed()),
                        (desktop("get_app_state", 1), observed()),
                    ],
                    "completed",
                ),
            ];
            for (name, steps, expected) in scenarios {
                let out = run_sequence(steps, "The requested synthetic state was observed.").await;
                assert_eq!(out.outcome, expected, "{name}");
            }
        }

        const ISSUED_OPERATION: &str = "11111111-1111-4111-8111-111111111111";
        const OTHER_OPERATION: &str = "22222222-2222-4222-8222-222222222222";
        fn prepare(kind: &str) -> (ComputerAction, ComputerObservation) {
            // Omitted operation_id is assigned by the real worker adapter, then returned in its receipt.
            let command = action(
                ComputerBackend::Social,
                "prepare",
                json!({
                    "action":match kind {"now"=>"publish","schedule"=>"schedule",_=>"draft"},
                    "integration_id":"synthetic-account", "content":"Synthetic inert draft",
                }),
                json!({}),
            );
            (
                command,
                receipt(
                    "completed",
                    json!({
                        "schema":1,"status":"prepared","operation_id":ISSUED_OPERATION,
                        "draft_hash":"a".repeat(64),"preview_pt":"Synthetic inert draft",
                        "payload":{"type":kind},
                    }),
                ),
            )
        }
        fn accepted() -> (ComputerAction, ComputerObservation) {
            (
                action(
                    ComputerBackend::Social,
                    "execute",
                    json!({
                        "operation_id":ISSUED_OPERATION,"approved_draft_hash":"a".repeat(64),
                    }),
                    json!({}),
                ),
                receipt(
                    "partial",
                    json!({
                        "schema":1,"status":"accepted","operation_id":ISSUED_OPERATION,
                        "post_id":"synthetic-post","integration_id":"synthetic-account",
                    }),
                ),
            )
        }
        fn status(
            state: &str,
            requested_operation: &str,
            observed_operation: &str,
        ) -> (ComputerAction, ComputerObservation) {
            (
                action(
                    ComputerBackend::Social,
                    "status",
                    json!({"operation_id":requested_operation}),
                    json!({}),
                ),
                receipt(
                    if matches!(state, "accepted" | "scheduled") {
                        "partial"
                    } else {
                        "completed"
                    },
                    json!({
                        "schema":1,"status":state,"operation_id":observed_operation,
                        "post_id":"synthetic-post","integration_id":"synthetic-account","source":"postiz_public_api",
                    }),
                ),
            )
        }

        #[tokio::test]
        async fn accepted_and_queued_publication_cannot_be_reported_as_published() {
            for steps in [
                vec![prepare("now")],
                vec![prepare("now"), accepted()],
                vec![
                    prepare("now"),
                    accepted(),
                    status("accepted", ISSUED_OPERATION, ISSUED_OPERATION),
                ],
                vec![
                    prepare("now"),
                    accepted(),
                    status("scheduled", ISSUED_OPERATION, ISSUED_OPERATION),
                ],
            ] {
                let out =
                    run_sequence(steps, "Synthetic false claim: this post was published.").await;
                assert_eq!(out.outcome, "partial");
                assert!(!out.summary.contains("Synthetic false claim"));
            }
        }

        #[tokio::test]
        async fn social_finish_requires_actual_operation_and_the_prepared_action_status() {
            let scenarios = [
                (
                    "published confirms immediate publication",
                    "now",
                    "published",
                    ISSUED_OPERATION,
                    ISSUED_OPERATION,
                    "completed",
                ),
                (
                    "queue confirms a schedule without claiming publication",
                    "schedule",
                    "scheduled",
                    ISSUED_OPERATION,
                    ISSUED_OPERATION,
                    "completed",
                ),
                (
                    "draft cannot confirm a schedule",
                    "schedule",
                    "drafted",
                    ISSUED_OPERATION,
                    ISSUED_OPERATION,
                    "partial",
                ),
                (
                    "published is not evidence that the requested schedule remains queued",
                    "schedule",
                    "published",
                    ISSUED_OPERATION,
                    ISSUED_OPERATION,
                    "partial",
                ),
                (
                    "draft confirms the requested Postiz draft",
                    "draft",
                    "drafted",
                    ISSUED_OPERATION,
                    ISSUED_OPERATION,
                    "completed",
                ),
                (
                    "another requested operation cannot clear the publication",
                    "now",
                    "published",
                    OTHER_OPERATION,
                    OTHER_OPERATION,
                    "partial",
                ),
                (
                    "mismatched receipt operation cannot clear the publication",
                    "now",
                    "published",
                    ISSUED_OPERATION,
                    OTHER_OPERATION,
                    "partial",
                ),
            ];
            for (name, kind, state, requested, observed, expected) in scenarios {
                let out = run_sequence(
                    vec![
                        prepare(kind),
                        accepted(),
                        status(state, requested, observed),
                    ],
                    "The requested state was observed in the synthetic API receipt.",
                )
                .await;
                assert_eq!(out.outcome, expected, "{name}");
                if kind == "schedule" && expected == "completed" {
                    assert!(!out.summary.contains("publicado"));
                }
            }
        }
    }
}
