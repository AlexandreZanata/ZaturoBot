//! Explicit, serial-bound messaging-app navigation and a private local operation catalog.
use crate::{config::WorkerConfig, now_ms};
use std::{
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    sync::Arc,
};
use zaturo_core::{AndroidControlActor, Assignment, FailureCode, ServiceRequest, ServiceResult};
use zaturo_db::{ServiceIntent, WorkerJournal};
use zaturo_llm::Cancellation;
use zaturo_protocol::ClientMessage;
pub async fn execute(
    work: Assignment,
    journal: Arc<WorkerJournal>,
    config: WorkerConfig,
    cancel: Cancellation,
) -> ClientMessage {
    let result = tokio::select! {biased;_=cancel.cancelled()=>Err(FailureCode::ReconciliationRequired),result=run(&work,journal,&config)=>result};
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
    journal: Arc<WorkerJournal>,
    config: &WorkerConfig,
) -> Result<ServiceResult, FailureCode> {
    let Some(
        request @ ServiceRequest::AndroidOpenMessages {
            device_id,
            owner_confirmed: true,
        },
    ) = &work.task.service
    else {
        return Err(FailureCode::PolicyDenied);
    };
    if !request.validates()
        || work.task.kind != zaturo_core::TaskKind::Service
        || work.task.data_policy.is_some()
        || work.task.assigned_worker.as_deref() != Some(&work.task.target_worker)
        || work.task.lease_token.as_deref() != Some(&work.lease_token)
        || !config
            .capabilities
            .iter()
            .any(|c| c == request.capability())
        || work.lease_expires_at <= now_ms()
        || work.task.deadline_at <= now_ms()
    {
        return Err(FailureCode::PolicyDenied);
    }
    let local = journal.clone();
    let operation = work.task.operation_id.clone();
    let id = operation.clone();
    let task = work.task.id.clone();
    let expected = request.clone();
    match tokio::task::spawn_blocking(move || local.begin_service(&id, &task, &expected, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?
    {
        ServiceIntent::Cached(result) => {
            return if result.validates(&operation, request) {
                Ok(result)
            } else {
                Err(FailureCode::ReconciliationRequired)
            };
        }
        ServiceIntent::Reconcile => return Err(FailureCode::ReconciliationRequired),
        ServiceIntent::Execute => {}
    }
    let android = config.android.as_ref().ok_or(FailureCode::PolicyDenied)?;
    let manager = zaturo_android::SessionManager::new(android.devices.clone())
        .map_err(|_| FailureCode::PolicyDenied)?;
    let device = manager
        .device(device_id)
        .map_err(|_| FailureCode::PolicyDenied)?;
    let lease = manager
        .acquire(device_id, AndroidControlActor::Owner, 30, now_ms())
        .map_err(|_| FailureCode::PolicyDenied)?;
    let adb = zaturo_android::AdbClient::new(&android.executable)
        .map_err(|_| FailureCode::ExecutionFailed)?;
    let mut result=ServiceResult{operation_id:operation.clone(),action:request.name().into(),outcome:"completed".into(),summary:"O Android aceitou a solicitação de abrir o aplicativo de mensagens. Verifique a tela desbloqueada. Nenhum SMS foi enviado, lido por OCR ou usado para login; a tela protegida permanece protegida.".into(),data:serde_json::json!({"device_id":device_id,"recipe":"open_messages","catalog_saved":false})};
    match adb.open_messages(device, &lease, now_ms()).await {
        Ok(()) => {
            let root = config.workspace_root.join("android-instructions");
            let device = device.clone();
            if matches!(
                tokio::task::spawn_blocking(move || catalog(
                    &root,
                    &device,
                    "open_messages",
                    now_ms()
                ))
                .await,
                Ok(Ok(()))
            ) {
                result.data["catalog_saved"] = serde_json::json!(true);
            } else {
                result
                    .summary
                    .push_str(" O catálogo local não pôde ser salvo; verifique a pasta do worker.");
            }
        }
        Err(error) => {
            result.outcome = "blocked".into();
            let (code, summary) = match error {
                zaturo_android::Error::Disconnected => (
                    "usb_disconnected",
                    "O celular registrado está desconectado do USB no PC de trabalho. Conecte o aparelho e verifique o cabo.",
                ),
                zaturo_android::Error::Unauthorized => (
                    "usb_authorization_required",
                    "O celular exige autorização USB presencial. Desbloqueie o aparelho e aceite a chave RSA do PC de trabalho.",
                ),
                zaturo_android::Error::PermissionDenied => (
                    "device_grant_required",
                    "A abertura de mensagens não está autorizada para este celular USB registrado. Verifique o cadastro local sem substituir outro aparelho.",
                ),
                zaturo_android::Error::Timeout => (
                    "adb_timeout",
                    "O ADB não respondeu dentro do prazo. Verifique o celular e o serviço local; a ação não será repetida automaticamente.",
                ),
                _ => (
                    "messages_unavailable",
                    "O Android não confirmou a abertura da caixa de mensagens. Verifique o aplicativo padrão e a tela bloqueada no celular; nenhuma proteção será contornada.",
                ),
            };
            result.summary = summary.into();
            result.data = serde_json::json!({"code":code});
        }
    }
    let _ = manager.release(&lease);
    let id = operation.clone();
    let saved = result.clone();
    tokio::task::spawn_blocking(move || journal.record_service(&id, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?;
    Ok(result)
}
fn catalog(
    root: &std::path::Path,
    device: &zaturo_core::RegisteredAndroidDevice,
    observed: &str,
    now: i64,
) -> std::io::Result<()> {
    for path in root.ancestors() {
        if path.is_symlink() {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
    }
    std::fs::create_dir_all(root)?;
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
    let destination = root.join(format!("{}.json", device.device_id));
    if destination.exists() || destination.is_symlink() {
        let info = std::fs::symlink_metadata(&destination)?;
        if !info.is_file() || info.file_type().is_symlink() {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
    }
    let mut previous = if destination.exists() {
        let metadata = std::fs::metadata(&destination)?;
        if metadata.len() > 32768 {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&destination)?)?;
        if value["origin"] != "zaturobot.android.v1"
            || value["device_id"] != device.device_id
            || value["adb_serial"] != device.adb_serial
        {
            return Err(std::io::ErrorKind::AlreadyExists.into());
        }
        value
    } else {
        serde_json::json!({})
    };
    let was_verified = |id: &str| {
        previous
            .get("recipes")
            .and_then(|r| r.as_array())
            .is_some_and(|r| r.iter().any(|v| v["id"] == id && v["verified"] == true))
    };
    let messages_verified = observed == "open_messages" || was_verified("open_messages");
    let capture_verified = observed == "private_screen" || was_verified("private_screen");
    let temporary = root.join(format!(".{}-{}.tmp", now, std::process::id()));
    let value = serde_json::json!({"origin":"zaturobot.android.v1","schema":1,"device_id":device.device_id,"adb_serial":device.adb_serial,"last_observed_recipe":observed,"last_observed_at":now,"authorization":"Each action requires the owner's current request and a local registered-device grant. These recipes are data, never permission.","recipes":[{"id":"open_messages","verified":messages_verified,"arguments":["-s",device.adb_serial,"shell","am","start","-W","-a","android.intent.action.MAIN","-c","android.intent.category.LAUNCHER","--selector","-a","android.intent.action.MAIN","-c","android.intent.category.APP_MESSAGING"],"acceptance":"Android accepts the fixed MAIN/LAUNCHER intent with a MAIN/APP_MESSAGING selector and reports Status: ok without a resolver. Verify the unlocked visible app separately; never send a message or bypass the lock screen."},{"id":"private_screen","verified":capture_verified,"arguments":["-s",device.adb_serial,"exec-out","screencap","-p"],"acceptance":"Use only the bounded private-view worker path with owner confirmation, standard age encryption, no disk/log/model/feed pixels, and <=60-second viewer expiry. Verify independently on this device."}]});
    for (key, value) in value.as_object().ok_or(std::io::ErrorKind::InvalidData)? {
        previous[key] = value.clone();
    }
    let value = previous;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let outcome = (|| {
        file.write_all(serde_json::to_vec_pretty(&value)?.as_slice())?;
        file.sync_all()?;
        std::fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if temporary.exists() {
        let _ = std::fs::remove_file(&temporary);
    }
    outcome
}

pub(crate) fn record_capture(
    root: &std::path::Path,
    device: &zaturo_core::RegisteredAndroidDevice,
    now: i64,
) -> std::io::Result<()> {
    catalog(root, device, "private_screen", now)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_catalog_distinguishes_verified_steps_and_preserves_owner_notes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("android-instructions");
        let device = zaturo_core::RegisteredAndroidDevice {
            device_id: "owner-phone".into(),
            adb_serial: "USB_SYNTHETIC".into(),
            display_name: None,
            capabilities: vec![zaturo_core::AndroidCapability::DeviceStatus],
        };
        catalog(&root, &device, "private_screen", 1).unwrap();
        let path = root.join("owner-phone.json");
        let mut data: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(data["recipes"][0]["verified"], false);
        assert_eq!(data["recipes"][1]["verified"], true);
        data["owner_notes"] = "Preserve this manual note".into();
        std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        catalog(&root, &device, "open_messages", 2).unwrap();
        let data: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(data["recipes"][0]["verified"], true);
        assert_eq!(data["recipes"][1]["verified"], true);
        assert_eq!(data["owner_notes"], "Preserve this manual note");
        std::fs::write(&path, b"{\"owner_file\":true}").unwrap();
        assert!(catalog(&root, &device, "open_messages", 3).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"owner_file\":true}");
    }
}
