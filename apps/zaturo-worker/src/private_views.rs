//! Sensitive owner capture/watch: memory-only standard age transport, never task result bytes.
use crate::{config::WorkerConfig, now_ms};
use zaturo_core::{
    AndroidControlActor, AndroidWatchRequest, Assignment, FailureCode, PrivateViewGrant,
    ServiceRequest, ServiceResult,
};
use zaturo_llm::Cancellation;
use zaturo_protocol::ClientMessage;
pub async fn execute(
    work: Assignment,
    config: WorkerConfig,
    token: String,
    cancellation: Cancellation,
) -> ClientMessage {
    let result = tokio::select! {biased;_=cancellation.cancelled()=>Err(FailureCode::ExecutionFailed),result=run(&work,&config,&token)=>result};
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
    let Some(ServiceRequest::AndroidPrivateView {
        device_id,
        session_id,
        recipient,
        watch_seconds,
        owner_confirmed,
    }) = &work.task.service
    else {
        return Err(FailureCode::PolicyDenied);
    };
    if !owner_confirmed || !zaturo_transfer::crypto::valid_recipient(recipient) {
        return Err(FailureCode::PolicyDenied);
    }
    let android = config
        .android
        .as_ref()
        .filter(|a| a.allow_sensitive_views)
        .ok_or(FailureCode::PolicyDenied)?;
    let client = crate::files::client(config, token).map_err(|_| FailureCode::ExecutionFailed)?;
    let grant: PrivateViewGrant = client
        .get(&format!("/v1/device/private-views/{session_id}/grant"))
        .await
        .map_err(|_| FailureCode::PolicyDenied)?;
    if grant.id != *session_id
        || grant.worker_id != config.worker_id
        || grant.device_id != *device_id
        || grant.recipient != *recipient
        || grant.watch_seconds != *watch_seconds
        || grant.expires_at <= now_ms()
    {
        return Err(FailureCode::PolicyDenied);
    }
    let manager = zaturo_android::SessionManager::new(android.devices.clone())
        .map_err(|_| FailureCode::PolicyDenied)?;
    let device = manager
        .device(device_id)
        .map_err(|_| FailureCode::PolicyDenied)?;
    let seconds = (*watch_seconds + 30).min(180);
    let lease = manager
        .acquire(device_id, AndroidControlActor::Owner, seconds, now_ms())
        .map_err(|_| FailureCode::PolicyDenied)?;
    let adb = zaturo_android::AdbClient::new(&android.executable)
        .map_err(|_| FailureCode::ExecutionFailed)?;
    let frame = if *watch_seconds == 0 {
        Some(
            adb.capture(device, &lease, now_ms())
                .await
                .map_err(|_| FailureCode::ExecutionFailed)?,
        )
    } else {
        adb.watch(
            device,
            &lease,
            &AndroidWatchRequest {
                duration_seconds: *watch_seconds,
                poll_seconds: 3,
            },
            now_ms(),
        )
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
    };
    let Some(frame) = frame else {
        let _: serde_json::Value = client
            .post(
                &format!("/v1/device/private-views/{session_id}/complete"),
                &serde_json::json!({"state":"unchanged"}),
            )
            .await
            .map_err(|_| FailureCode::ExecutionFailed)?;
        return Ok(ServiceResult {
            operation_id: work.task.operation_id.clone(),
            action: "android_private_view".into(),
            outcome: "completed".into(),
            summary: zaturo_core::PRIVATE_VIEW_UNCHANGED_SUMMARY.into(),
            data: serde_json::json!({"session_id":session_id,"state":"unchanged"}),
        });
    };
    let cipher = zaturo_transfer::crypto::encrypt_private_image(frame.bytes(), recipient)
        .map_err(|_| FailureCode::ExecutionFailed)?;
    drop(frame);
    let _ = manager.release(&lease);
    client
        .private_frame(session_id, cipher)
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?;
    Ok(ServiceResult {
        operation_id: work.task.operation_id.clone(),
        action: "android_private_view".into(),
        outcome: "completed".into(),
        summary: zaturo_core::PRIVATE_VIEW_READY_SUMMARY.into(),
        data: serde_json::json!({"session_id":session_id,"delivery":"ephemeral_encrypted_memory","max_view_seconds":60}),
    })
}
