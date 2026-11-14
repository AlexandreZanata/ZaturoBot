//! Atomic owner chat, indexed daily history and typed completion notices.
use crate::{Engine, Error, Result};
use uuid::Uuid;
use zaturo_core::{
    ChatAction, ChatReply, ChatRequest, FeedItem, FeedKind, FeedPage, FeedQuery, NewTask,
    ServiceRequest, Task, TaskKind, TaskState,
};
use zaturo_db::Repository;

pub(crate) fn notice(task: &Task) -> FeedItem {
    let detail = task
        .service_result
        .as_ref()
        .map(|s| s.summary.as_str())
        .or_else(|| task.result.as_ref().map(|r| r.summary.as_str()))
        .unwrap_or("Consulte o histórico da tarefa para os detalhes.");
    let label = if task
        .service_result
        .as_ref()
        .is_some_and(|r| r.outcome == "accepted")
    {
        "Serviço iniciado"
    } else {
        match task.state {
            TaskState::Completed => "Tarefa concluída",
            TaskState::Failed => "Tarefa não concluída",
            TaskState::Cancelled => "Tarefa cancelada",
            _ => "Tarefa enviada",
        }
    };
    FeedItem {
        id: Uuid::new_v4().to_string(),
        created_at: task.updated_at,
        kind: if task.kind == TaskKind::Service {
            FeedKind::Service
        } else {
            FeedKind::Task
        },
        role: "assistant".into(),
        text: format!("{label} · {}: {detail}", task.target_worker),
        task_id: Some(task.id.clone()),
        artifact_id: None,
        sensitive: false,
    }
}
pub(crate) fn task_notice(repository: &Repository<'_>, task: &Task) -> Result<()> {
    let item = notice(task);
    repository.insert_feed(&item, &format!("task:{}:{}", task.id, task.state.as_str()))?;
    Ok(())
}
impl Engine {
    pub fn feed(&self, query: FeedQuery) -> Result<FeedPage> {
        self.page(query.limit, query.offset)?;
        let (start, end) = query.range().map_err(Error::Invalid)?;
        let text = query.q.unwrap_or_default().to_lowercase();
        self.store.read(|r| {
            let mut items = r.feed_page(
                start,
                end,
                query.kind.map(|k| k.as_str()),
                &text,
                query.limit + 1,
                query.offset,
            )?;
            let has_more = items.len() > query.limit as usize;
            items.truncate(query.limit as usize);
            Ok(FeedPage { items, has_more })
        })
    }
    pub fn chat(&self, request: ChatRequest, default_worker: &str, now: i64) -> Result<ChatReply> {
        if !request.validates() {
            return Err(Error::Invalid(
                "invalid chat request; do not paste credentials".into(),
            ));
        }
        let target = request
            .target_worker
            .as_deref()
            .unwrap_or(default_worker)
            .to_owned();
        zaturo_core::validate_identifier(&target).map_err(Error::Invalid)?;
        self.store.transaction(|r| {
   if let Some((previous,reply))=r.chat_request(&request.idempotency_key)? {return if previous==request {Ok(reply)} else {Err(Error::Conflict("chat idempotency collision".into()))};}
   let task=if let Some(action)=request.action {
    let mut task=NewTask::diagnostic(target,format!("chat-{}",request.idempotency_key));task.request=request.message.clone();
    match action {ChatAction::Diagnostic=>{},ChatAction::YoutubeProfiles=>{task.kind=TaskKind::Service;task.service=Some(ServiceRequest::YoutubeProfiles);},ChatAction::AndroidDevices=>{task.kind=TaskKind::Service;task.service=Some(ServiceRequest::AndroidDevices);}}
    Some(crate::create_record(r,task,now,&self.limits)?)
   }else{None};
   let owner=FeedItem{id:Uuid::new_v4().to_string(),created_at:now,kind:FeedKind::Chat,role:"owner".into(),text:request.message.clone(),task_id:task.as_ref().map(|t|t.id.clone()),artifact_id:None,sensitive:false};
   r.insert_feed(&owner,&format!("chat:{}:owner",request.idempotency_key))?;
   let message=FeedItem{id:Uuid::new_v4().to_string(),created_at:now.saturating_add(1),kind:FeedKind::Chat,role:"assistant".into(),text:if task.is_some(){"Solicitação enviada ao worker. O resultado aparecerá neste histórico.".into()}else{"Mensagem salva. Use Diagnóstico, YouTube ou Android para executar uma ação disponível; outras solicitações ficam registradas sem execução automática.".into()},task_id:task.as_ref().map(|t|t.id.clone()),artifact_id:None,sensitive:false};
   r.insert_feed(&message,&format!("chat:{}:reply",request.idempotency_key))?;
   let reply=ChatReply{message,task};r.save_chat(&request.idempotency_key,&request,&reply)?;Ok(reply)
  })
    }
    pub fn seed_activity(&self) -> Result<()> {
        let mut offset = 0;
        loop {
            let count = self.store.transaction(|r| {
                let tasks = r.tasks(100, offset)?;
                for task in &tasks {
                    if task.state.is_terminal() {
                        task_notice(r, task)?;
                    }
                }
                Ok::<_, Error>(tasks.len())
            })?;
            if count < 100 {
                break;
            }
            offset += 100;
        }
        Ok(())
    }
    pub fn publish_activity(&self, item: FeedItem, key: &str) -> Result<FeedItem> {
        if !item.validates() || key.len() > 128 {
            return Err(Error::Invalid("invalid activity record".into()));
        }
        self.store.transaction(|r| {
            r.insert_feed(&item, key)?;
            Ok(r.feed_by_key(key)?.unwrap_or(item))
        })
    }
}
impl Engine {
    pub fn service_activity(
        &self,
        worker: &str,
        update: zaturo_core::ServiceActivity,
        now: i64,
    ) -> Result<FeedItem> {
        if !update.validates() {
            return Err(Error::Invalid("invalid service update".into()));
        }
        self.store.transaction(|r| {
            let task = r
                .task_by_operation(&update.operation_id)?
                .ok_or(Error::NotFound)?;
            if task.target_worker != worker
                || task.assigned_worker.as_deref() != Some(worker)
                || !matches!(task.service, Some(ServiceRequest::YoutubeSubmit { .. }))
                || task
                    .service_result
                    .as_ref()
                    .is_none_or(|s| s.outcome != "accepted")
            {
                return Err(Error::NotFound);
            }
            let key = format!("service:{}:{}", update.operation_id, update.state);
            if let Some(item) = r.feed_by_key(&key)? {
                return Ok(item);
            }
            if r.service_state(&update.operation_id)?
                .is_some_and(|s| matches!(s.as_str(), "completed" | "failed" | "uncertain"))
            {
                return Err(Error::Conflict(
                    "service job already has a terminal observation".into(),
                ));
            }
            let label = match update.state.as_str() {
                "completed" => "Vídeo: processamento concluído",
                "failed" => "Vídeo: processamento falhou",
                "uncertain" => "Vídeo: verificação manual necessária",
                _ => "Vídeo em processamento",
            };
            let item = FeedItem {
                id: Uuid::new_v4().to_string(),
                created_at: now,
                kind: FeedKind::Service,
                role: "assistant".into(),
                text: format!("{label} · {}", update.summary),
                task_id: Some(task.id),
                artifact_id: None,
                sensitive: false,
            };
            r.save_service_state(&update.operation_id, &update.state, now)?;
            r.insert_feed(&item, &key)?;
            Ok(item)
        })
    }
}
