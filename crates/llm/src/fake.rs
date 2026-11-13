//! Synthetic model fixtures. Composition roots must never select this in production.
use crate::{BudgetPort, Error, FutureResult, LlmClient, ModelRequest, ModelResponse};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use zaturo_core::{ModelReservation, ModelUsage};

pub enum FakeReply {
    Answer(ModelResponse),
    Error(Error),
    Delay(Duration, ModelResponse),
}
pub struct FakeClient {
    replies: Mutex<VecDeque<FakeReply>>,
    calls: AtomicUsize,
}
impl FakeClient {
    pub fn new(replies: impl IntoIterator<Item = FakeReply>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().collect()),
            calls: AtomicUsize::new(0),
        }
    }
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl LlmClient for FakeClient {
    fn complete(&self, _request: ModelRequest) -> FutureResult<'_, ModelResponse> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let reply = self
                .replies
                .lock()
                .map_err(|_| Error::InvalidResponse)?
                .pop_front()
                .ok_or(Error::InvalidResponse)?;
            match reply {
                FakeReply::Answer(response) => Ok(response),
                FakeReply::Error(error) => Err(error),
                FakeReply::Delay(delay, response) => {
                    tokio::time::sleep(delay).await;
                    Ok(response)
                }
            }
        })
    }
}

/// Behavioral fixture only. Production must use the SQLite-backed budget adapter.
#[derive(Default)]
pub struct FakeBudget {
    records: Mutex<BTreeMap<String, &'static str>>,
    deny: AtomicBool,
}
impl FakeBudget {
    pub fn deny(&self) {
        self.deny.store(true, Ordering::SeqCst);
    }
    pub fn states(&self) -> BTreeMap<String, &'static str> {
        self.records
            .lock()
            .expect("fake budget lock poisoned")
            .clone()
    }
}
impl BudgetPort for FakeBudget {
    fn reserve<'a>(&'a self, reservation: &'a ModelReservation) -> FutureResult<'a, ()> {
        Box::pin(async move {
            if self.deny.load(Ordering::SeqCst) {
                return Err(Error::Budget);
            }
            let mut records = self.records.lock().map_err(|_| Error::Budget)?;
            if records.contains_key(&reservation.request_id) {
                return Err(Error::Reconciliation);
            }
            records.insert(reservation.request_id.clone(), "reserved");
            Ok(())
        })
    }
    fn settle<'a>(&'a self, id: &'a str, _: &'a ModelUsage) -> FutureResult<'a, ()> {
        Box::pin(async move {
            self.records
                .lock()
                .map_err(|_| Error::Budget)?
                .insert(id.into(), "settled");
            Ok(())
        })
    }
    fn uncertain<'a>(&'a self, id: &'a str) -> FutureResult<'a, ()> {
        Box::pin(async move {
            self.records
                .lock()
                .map_err(|_| Error::Budget)?
                .insert(id.into(), "uncertain");
            Ok(())
        })
    }
}
