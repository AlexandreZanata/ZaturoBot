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
