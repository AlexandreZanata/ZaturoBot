use crate::{Engine, Error, Result, check_lease, transition};
use zaturo_core::{ServiceResult, Task, TaskKind, TaskState};
impl Engine {
    pub fn complete_service(
        &self,
        task_id: &str,
        worker: &str,
        lease: &str,
        result: ServiceResult,
        now: i64,
    ) -> Result<Task> {
        self.store.transaction(|r| {
            let mut task = r.task(task_id)?.ok_or(Error::NotFound)?;
            let request = task
                .service
                .as_ref()
                .ok_or_else(|| Error::Invalid("missing service request".into()))?;
            if task.kind != TaskKind::Service || !result.validates(&task.operation_id, request) {
                return Err(Error::Invalid("invalid service evidence".into()));
            }
            if task.state == TaskState::Completed
                && task.assigned_worker.as_deref() == Some(worker)
                && task.lease_token.as_deref() == Some(lease)
                && task.service_result.as_ref() == Some(&result)
            {
                return Ok(task);
            }
            check_lease(&task, worker, lease, now)?;
            if task.state != TaskState::Running {
                return Err(Error::Conflict("service task has not started".into()));
            }
            route_child(r, &task, &result, &self.limits, now)?;
            task.service_result = Some(result);
            transition(
                r,
                &mut task,
                TaskState::Completed,
                now,
                "Typed service evidence accepted",
            )?;
            Ok(task)
        })
    }
}

fn route_child(
    repository: &zaturo_db::Repository<'_>,
    parent: &Task,
    result: &ServiceResult,
    limits: &crate::Limits,
    now: i64,
) -> Result<()> {
    use zaturo_core::{
        FeedItem, FeedKind, IntentDecision, NewTask, RegisteredAction, ServiceRequest,
    };
    let Some(ServiceRequest::AssistantRoute {
        message,
        target_worker,
        planner_worker,
        ..
    }) = &parent.service
    else {
        return Ok(());
    };
    if result.outcome != "completed" {
        return Ok(());
    }
    let decision: IntentDecision = serde_json::from_value(result.data["decision"].clone())
        .map_err(|_| Error::Invalid("invalid registered intent evidence".into()))?;
    let IntentDecision::Action { action } = decision else {
        return Ok(());
    };
    if !action.authorized_by(message) {
        return Err(Error::Invalid(
            "intent lacks current owner authority".into(),
        ));
    }
    let (target, service, timeout) = match action {
        RegisteredAction::YoutubeGenerate { channel, count } => (
            target_worker.clone(),
            ServiceRequest::YoutubeOpenCodeGenerate { channel, count },
            180,
        ),
        RegisteredAction::BrowserResearch { request } => (
            planner_worker.clone(),
            ServiceRequest::ComputerTask {
                request,
                target_worker: target_worker.clone(),
            },
            300,
        ),
        RegisteredAction::AndroidDevices => {
            (target_worker.clone(), ServiceRequest::AndroidDevices, 60)
        }
        RegisteredAction::AndroidOpenMessages { device_id } => (
            target_worker.clone(),
            ServiceRequest::AndroidOpenMessages {
                device_id,
                owner_confirmed: true,
            },
            60,
        ),
    };
    let available = repository
        .worker(&target)?
        .is_some_and(|w| w.capabilities.iter().any(|c| c == service.capability()))
        && (!matches!(service, ServiceRequest::ComputerTask { .. })
            || repository
                .worker(target_worker)?
                .is_some_and(|w| w.capabilities.iter().any(|c| c == "computer.action.v1")));
    let (text, task_id) = if available {
        let mut request =
            NewTask::diagnostic(target.clone(), format!("route-{}", parent.operation_id));
        request.kind = TaskKind::Service;
        request.request = message.clone();
        request.service = Some(service);
        request.max_attempts = 1;
        request.timeout_seconds = timeout;
        let child = crate::create_record(repository, request, now, limits)?;
        (
            format!(
                "Ação registrada enviada ao computador {target}. Operação {}. O resultado será confirmado por evidência de execução.",
                child.operation_id
            ),
            Some(child.id),
        )
    } else {
        (
            format!(
                "O computador {target} ainda não anunciou a ação registrada {}. A interpretação foi concluída, mas a ação não foi executada.",
                service.capability()
            ),
            Some(parent.id.clone()),
        )
    };
    repository.insert_feed(
        &FeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            kind: FeedKind::Task,
            role: "assistant".into(),
            text,
            task_id,
            artifact_id: None,
            sensitive: false,
            capture: None,
        },
        &format!("route:{}:dispatch", parent.operation_id),
    )?;
    Ok(())
}
