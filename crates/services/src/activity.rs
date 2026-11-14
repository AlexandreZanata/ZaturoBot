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
        .is_some_and(|r| r.outcome == "blocked")
    {
        "Ação não executada"
    } else if task
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
        kind: if matches!(task.service, Some(ServiceRequest::AssistantChat { .. })) {
            FeedKind::Chat
        } else if task.kind == TaskKind::Service {
            FeedKind::Service
        } else {
            FeedKind::Task
        },
        role: "assistant".into(),
        text: if matches!(task.service, Some(ServiceRequest::AssistantChat { .. }))
            && task.service_result.is_some()
        {
            if task
                .service_result
                .as_ref()
                .is_some_and(|r| r.outcome == "completed")
            {
                format!("IA · {detail}")
            } else {
                detail.to_owned()
            }
        } else {
            format!("{label} · {}: {detail}", task.target_worker)
        },
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
        self.chat_as(request, default_worker, "owner", now)
    }
    pub fn chat_as(
        &self,
        request: ChatRequest,
        default_worker: &str,
        actor: &str,
        now: i64,
    ) -> Result<ChatReply> {
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
            if let Some((previous,reply))=r.chat_request(&request.idempotency_key)? {
                return if previous==request {Ok(reply)} else {Err(Error::Conflict("chat idempotency collision".into()))};
            }
            let mut response="Mensagem salva sem execução automática.".to_owned();
            let mut selected_target=target.clone();
            let mut service=None;
            let action=match request.action {
                Some(ChatAction::Assistant)=>match zaturo_core::chat_intent(&request.message) {
                    zaturo_core::ChatIntent::Shared{instructions}=>{
                        let entry=crate::shared::create_record(r,&zaturo_core::SharedCreate{idempotency_key:request.idempotency_key.clone(),target_worker:target.clone(),title:"Pedido do chat".into(),instructions,transfer_ids:vec![]},actor,now)?;
                        response=format!("Instrução salva na área compartilhada de {}. No outro Codex, peça para consultar a última tarefa. Ela não será executada automaticamente.",entry.target_worker);None
                    },
                    zaturo_core::ChatIntent::AndroidOpenMessages=>Some(ChatAction::AndroidOpenMessages),
                    zaturo_core::ChatIntent::Diagnostic=>Some(ChatAction::Diagnostic),
                    zaturo_core::ChatIntent::YoutubeProfiles=>Some(ChatAction::YoutubeProfiles),
                    zaturo_core::ChatIntent::AndroidDevices=>Some(ChatAction::AndroidDevices),
                    zaturo_core::ChatIntent::YoutubeGenerate{channel,count}=>{service=Some(ServiceRequest::YoutubeGenerate{channel,count});Some(ChatAction::Assistant)},
                    zaturo_core::ChatIntent::Conversation=>{
                        selected_target=default_worker.to_owned();
                        if request.message.len()<=2048 {service=Some(ServiceRequest::AssistantChat{message:request.message.clone(),context:conversation_context(r)?});}
                        else {response="Para conversar com a IA, reduza a mensagem para até 2048 bytes.".into();}
                        service.as_ref().map(|_|ChatAction::Assistant)
                    }
                },other=>other,
            };
            let task=if let Some(action)=action {
                let mut task=NewTask::diagnostic(selected_target.clone(),format!("chat-{}",request.idempotency_key));
                task.request=request.message.clone();
                match action {
                    ChatAction::Diagnostic=>{},
                    ChatAction::YoutubeProfiles=>{service=Some(ServiceRequest::YoutubeProfiles);},
                    ChatAction::AndroidOpenMessages=>{service=Some(ServiceRequest::AndroidOpenMessages{device_id:"owner-phone".into(),owner_confirmed:true});},
                    ChatAction::AndroidDevices=>{service=Some(ServiceRequest::AndroidDevices);},
                    ChatAction::Assistant=>{},
                }
                if let Some(service)=service {task.kind=TaskKind::Service;task.max_attempts=1;task.timeout_seconds=60;task.service=Some(service);}
                let capability=task.service.as_ref().map(|s|s.capability()).unwrap_or("diagnostic.v1");
                let worker=r.worker(&selected_target)?;
                // Legacy explicit service buttons retain their durable offline queue semantics.
                let unsupported=matches!(request.action,Some(ChatAction::Assistant | ChatAction::AndroidOpenMessages)) && worker.as_ref().is_none_or(|w|!w.capabilities.iter().any(|c|c==capability));
                if unsupported {
                    response=format!("O computador {selected_target} ainda não anunciou a funcionalidade {capability}. A ação não foi executada. Atualize ou conecte o worker pelo Codex desse computador.");None
                } else {
                    response=if worker.as_ref().is_some_and(|w|now.saturating_sub(w.last_seen_at)>30000){"O computador está sem contato recente. A tarefa ficou na fila persistente e será enviada quando ele reconectar.".into()}else if matches!(task.service,Some(ServiceRequest::YoutubeGenerate{..})){"Solicitação de geração enviada. O PC verificará os logins antes de criar os vídeos. Publicar ou agendar exige uma ação separada.".into()}else{"Solicitação enviada ao worker. O resultado aparecerá neste histórico.".into()};
                    Some(crate::create_record(r,task,now,&self.limits)?)
                }
            }else{None};
            let owner=FeedItem{id:Uuid::new_v4().to_string(),created_at:now,kind:FeedKind::Chat,role:"owner".into(),text:request.message.clone(),task_id:task.as_ref().map(|t|t.id.clone()),artifact_id:None,sensitive:false};
            r.insert_feed(&owner,&format!("chat:{}:owner",request.idempotency_key))?;
            let message=FeedItem{id:Uuid::new_v4().to_string(),created_at:now.saturating_add(1),kind:FeedKind::Chat,role:"assistant".into(),text:response,task_id:task.as_ref().map(|t|t.id.clone()),artifact_id:None,sensitive:false};
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
                || !matches!(
                    task.service,
                    Some(
                        ServiceRequest::YoutubeSubmit { .. }
                            | ServiceRequest::YoutubeGenerate { .. }
                    )
                )
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

fn conversation_context(r: &Repository<'_>) -> Result<Vec<zaturo_core::ConversationTurn>> {
    let mut remaining = 1024;
    let mut turns = vec![];
    for item in r.conversation_tail()? {
        if remaining == 0 {
            break;
        }
        let mut end = item.text.len().min(remaining).min(512);
        while !item.text.is_char_boundary(end) {
            end -= 1;
        }
        let text = item.text[..end].to_owned();
        remaining -= end;
        let turn = zaturo_core::ConversationTurn {
            role: item.role,
            text,
        };
        if turn.validates() {
            turns.push(turn);
        }
    }
    turns.reverse();
    Ok(turns)
}
