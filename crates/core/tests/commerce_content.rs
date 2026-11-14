use zaturo_core::commerce::{ContentState, OwnerContentResolution, OwnerResolutionOutcome};

#[test]
fn content_state_machine_rejects_illegal_transitions() {
    use ContentState::*;
    let states = [
        Draft, Approved, Scheduled, Publishing, Published, Rejected, Failed, Uncertain, Withdrawn,
    ];
    let allowed = [
        (Draft, Approved),
        (Draft, Rejected),
        (Draft, Withdrawn),
        (Approved, Scheduled),
        (Approved, Withdrawn),
        (Scheduled, Publishing),
        (Scheduled, Failed),
        (Scheduled, Withdrawn),
        (Publishing, Published),
        (Publishing, Failed),
        (Publishing, Uncertain),
    ];
    for from in states {
        for to in states {
            assert_eq!(
                from.can_transition_to(to),
                allowed.contains(&(from, to)),
                "{from:?} -> {to:?}"
            );
            assert_eq!(
                from.transition_to(to).is_ok(),
                allowed.contains(&(from, to))
            );
        }
    }
}

#[test]
fn uncertain_requires_owner_resolution() {
    let resolution = OwnerContentResolution {
        outcome: OwnerResolutionOutcome::ConfirmedPublished,
        evidence: "Owner verified external post fixture-17".into(),
    };
    assert_eq!(
        ContentState::Uncertain
            .resolve_uncertain(&resolution)
            .unwrap(),
        ContentState::Published
    );
    assert!(
        ContentState::Publishing
            .resolve_uncertain(&resolution)
            .is_err()
    );
    let unpublished = OwnerContentResolution {
        outcome: OwnerResolutionOutcome::ConfirmedNotPublished,
        evidence: "Owner verified the provider history".into(),
    };
    assert_eq!(
        ContentState::Uncertain
            .resolve_uncertain(&unpublished)
            .unwrap(),
        ContentState::Failed
    );
    for evidence in [
        String::new(),
        " \n".into(),
        "x".repeat(513),
        "bad\0proof".into(),
    ] {
        assert!(
            ContentState::Uncertain
                .resolve_uncertain(&OwnerContentResolution {
                    evidence,
                    ..resolution.clone()
                })
                .is_err()
        );
    }
}

#[test]
fn content_states_use_exact_snake_case_contract() {
    for (state, expected) in [
        (ContentState::Draft, "draft"),
        (ContentState::Approved, "approved"),
        (ContentState::Scheduled, "scheduled"),
        (ContentState::Publishing, "publishing"),
        (ContentState::Published, "published"),
        (ContentState::Rejected, "rejected"),
        (ContentState::Failed, "failed"),
        (ContentState::Uncertain, "uncertain"),
        (ContentState::Withdrawn, "withdrawn"),
    ] {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        assert_eq!(serde_json::from_str::<ContentState>(&json).unwrap(), state);
    }
    assert!(serde_json::from_str::<ContentState>("\"sent\"").is_err());
}
