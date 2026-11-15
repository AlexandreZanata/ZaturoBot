//! One owner-confirmed capture of the fixed local window, encrypted in memory only.
use crate::{config::WorkerConfig, now_ms};
use zaturo_core::{
    Assignment, ComputerLease, FailureCode, PrivateViewGrant, ServiceRequest, ServiceResult,
};
use zaturo_llm::Cancellation;
use zaturo_protocol::ClientMessage;
pub async fn execute(
    work: Assignment,
    config: WorkerConfig,
    token: String,
    cancel: Cancellation,
) -> ClientMessage {
    let result = tokio::select! {biased;_=cancel.cancelled()=>Err(FailureCode::ReconciliationRequired),r=run(&work,&config,&token)=>r};
    match result {
        Ok(result) => ClientMessage::CompletedService {
            task_id: work.task.id,
            lease_token: work.lease_token,
            result,
        },
        Err(code) => ClientMessage::Failed {
            task_id: work.task.id,
            lease_token: work.lease_token,
            code,
        },
    }
}
async fn run(
    work: &Assignment,
    config: &WorkerConfig,
    token: &str,
) -> Result<ServiceResult, FailureCode> {
    let Some(ServiceRequest::DesktopPrivateView {
        session_id,
        recipient,
        owner_confirmed: true,
    }) = &work.task.service
    else {
        return Err(FailureCode::PolicyDenied);
    };
    let computer = config.computer.as_ref().ok_or(FailureCode::PolicyDenied)?;

    let client = crate::files::client(config, token).map_err(|_| FailureCode::ExecutionFailed)?;
    let grant: PrivateViewGrant = client
        .get(&format!("/v1/device/private-views/{session_id}/grant"))
        .await
        .map_err(|_| FailureCode::PolicyDenied)?;
    if grant.id != *session_id
        || grant.worker_id != config.worker_id
        || grant.device_id != "desktop"
        || grant.recipient != *recipient
        || grant.watch_seconds != 0
        || grant.expires_at <= now_ms()
    {
        return Err(FailureCode::PolicyDenied);
    }
    let lease = ComputerLease {
        operation_id: work.task.operation_id.clone(),
        lease_token: work.lease_token.clone(),
        lease_expires_at: work.lease_expires_at,
        deadline_at: work.task.deadline_at,
        owner_approved: true,
    };
    let pixels = if computer.wayland_portal_capture {
        zaturo_computer::capture_wayland(computer, &lease).await
    } else {
        let target = computer
            .capture_target
            .as_ref()
            .ok_or(FailureCode::PolicyDenied)?;
        zaturo_computer::capture(computer, &lease, target.clone()).await
    }
    .map_err(|_| FailureCode::ExecutionFailed)?;
    let cipher = zaturo_transfer::crypto::encrypt_private_image(&pixels, recipient)
        .map_err(|_| FailureCode::ExecutionFailed)?;
    drop(pixels);
    client
        .private_frame(session_id, cipher)
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?;
    Ok(ServiceResult {
        operation_id: work.task.operation_id.clone(),
        action: "desktop_private_view".into(),
        outcome: "completed".into(),
        summary: zaturo_core::PRIVATE_VIEW_READY_SUMMARY.into(),
        data: serde_json::json!({"session_id":session_id,"delivery":"ephemeral_encrypted_memory","max_view_seconds":60}),
    })
}
