//! Real Python module transport against a temporary synthetic project; no external effects.
use std::{fs, path::Path, time::Duration};
use zaturo_core::ServiceRequest;
use zaturo_youtube::{Error, JobState, YoutubeClient, YoutubeConfig};
const SCRIPT: &str = include_str!("fixtures/service.py");
fn project(root: &Path) -> YoutubeConfig {
    let package = root.join("tools/ytbot");
    fs::create_dir_all(&package).unwrap();
    fs::write(root.join("tools/__init__.py"), b"").unwrap();
    fs::write(package.join("__init__.py"), b"").unwrap();
    fs::write(package.join("service.py"), SCRIPT).unwrap();
    YoutubeConfig {
        python: "/usr/bin/python3".into(),
        project_root: root.to_owned(),
        allow_schedule: false,
    }
}
fn op() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn submit() -> ServiceRequest {
    ServiceRequest::YoutubeSubmit {
        channel: "channel".into(),
        account: "account".into(),
        count: 1,
        approved_schedule: true,
    }
}
#[tokio::test]
async fn fixed_profile_read_has_minimal_environment_and_curated_output() {
    let temp = tempfile::tempdir().unwrap();
    let client = YoutubeClient::new(project(temp.path())).unwrap();
    let operation = op();
    let result = client
        .execute(&operation, &ServiceRequest::YoutubeProfiles)
        .await
        .unwrap();
    assert!(result.validates(&operation, &ServiceRequest::YoutubeProfiles));
    assert_eq!(result.data["profiles"][0]["id"], "channel");
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("SYNTHETIC_EXTRA_MUST_NOT_LEAK")
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("calls.jsonl"))
            .unwrap()
            .trim(),
        "[\"profiles\"]"
    );
}
#[tokio::test]
async fn runs_are_curated_bounded_and_honestly_truncated() {
    let temp = tempfile::tempdir().unwrap();
    let client = YoutubeClient::new(project(temp.path())).unwrap();
    let operation = op();
    let request = ServiceRequest::YoutubeRuns {
        channel: "channel".into(),
        limit: 20,
    };
    let result = client.execute(&operation, &request).await.unwrap();
    assert!(result.validates(&operation, &request));
    assert_eq!(result.data["total"], 20);
    assert_eq!(result.data["truncated"], true);
    assert!(result.data["runs"].as_array().unwrap().len() < 20);
    assert!(serde_json::to_vec(&result).unwrap().len() <= 4096);
}
#[tokio::test]
async fn local_schedule_denial_prevents_even_starting_python() {
    let temp = tempfile::tempdir().unwrap();
    let client = YoutubeClient::new(project(temp.path())).unwrap();
    assert_eq!(
        client.execute(&op(), &submit()).await.unwrap_err(),
        Error::PolicyDenied
    );
    assert!(!temp.path().join("calls.jsonl").exists());
}
#[tokio::test]
async fn lost_submit_response_reconciles_status_without_second_submission() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = project(temp.path());
    config.allow_schedule = true;
    let client = YoutubeClient::new(config).unwrap();
    let operation = op();
    fs::write(temp.path().join("mode"), "lost-response").unwrap();
    assert_eq!(
        client.execute(&operation, &submit()).await.unwrap_err(),
        Error::Rejected
    );
    fs::write(temp.path().join("mode"), "normal").unwrap();
    let result = client.reconcile(&operation, &submit()).await.unwrap();
    assert_eq!(result.outcome, "accepted");
    assert!(!result.summary.contains("completed"));
    let calls = fs::read_to_string(temp.path().join("calls.jsonl")).unwrap();
    assert_eq!(
        calls.lines().filter(|line| line.contains("submit")).count(),
        1
    );
    assert_eq!(
        calls.lines().filter(|line| line.contains("status")).count(),
        1
    );
    fs::write(temp.path().join("mode"), "completed").unwrap();
    let completed = client.reconcile(&operation, &submit()).await.unwrap();
    assert_eq!(completed.outcome, "completed");
    assert_eq!(completed.data["scheduled_count"], 1);
}
#[tokio::test]
async fn terminal_failure_uncertainty_and_wrong_identity_do_not_become_success() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = project(temp.path());
    config.allow_schedule = true;
    let client = YoutubeClient::new(config).unwrap();
    let operation = op();
    client.execute(&operation, &submit()).await.unwrap();
    for (mode, error) in [
        ("failed", Error::JobFailed),
        ("uncertain", Error::ReconciliationRequired),
        ("dishonest-completion", Error::ReconciliationRequired),
    ] {
        fs::write(temp.path().join("mode"), mode).unwrap();
        assert_eq!(
            client.reconcile(&operation, &submit()).await.unwrap_err(),
            error
        );
    }
    fs::write(temp.path().join("mode"), "failed").unwrap();
    let observation = client.status(&operation).await.unwrap();
    assert_eq!(observation.state, JobState::Failed);
    fs::write(temp.path().join("mode"), "wrong-operation").unwrap();
    assert_eq!(
        client.status(&operation).await.unwrap_err(),
        Error::InvalidOutput
    );
}
#[tokio::test]
async fn timeout_overflow_and_backend_errors_are_bounded_and_opaque() {
    let temp = tempfile::tempdir().unwrap();
    let config = project(temp.path());
    let client = YoutubeClient::with_timeout(config, Duration::from_millis(100)).unwrap();
    let operation = op();
    fs::write(temp.path().join("mode"), "slow").unwrap();
    assert_eq!(
        client
            .execute(&operation, &ServiceRequest::YoutubeProfiles)
            .await
            .unwrap_err(),
        Error::Timeout
    );
    fs::write(temp.path().join("mode"), "oversize").unwrap();
    assert_eq!(
        client
            .execute(&operation, &ServiceRequest::YoutubeProfiles)
            .await
            .unwrap_err(),
        Error::OutputLimit
    );
    fs::write(temp.path().join("mode"), "rejected").unwrap();
    let error = client
        .execute(&operation, &ServiceRequest::YoutubeProfiles)
        .await
        .unwrap_err();
    assert_eq!(error, Error::Rejected);
    assert!(!format!("{error:?} {error}").contains("SYNTHETIC_PRIVATE_ERROR"));
}
#[tokio::test]
async fn configured_interpreter_symlink_keeps_venv_style_invocation() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = project(temp.path());
    let link = temp.path().join("python-venv-link");
    std::os::unix::fs::symlink("/usr/bin/python3", &link).unwrap();
    config.python = link.clone();
    let client = YoutubeClient::new(config).unwrap();
    assert_eq!(client.config().python, link);
    client
        .execute(&op(), &ServiceRequest::YoutubeProfiles)
        .await
        .unwrap();
}
