use std::sync::Arc;
use tokio::sync::watch;

/// Cloneable cancellation signal; cancellation does not imply provider-side rollback.
#[derive(Debug, Clone)]
pub struct Cancellation {
    signal: Arc<watch::Sender<bool>>,
}
impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}
impl Cancellation {
    pub fn new() -> Self {
        let (signal, _) = watch::channel(false);
        Self {
            signal: Arc::new(signal),
        }
    }
    pub fn cancel(&self) {
        self.signal.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.signal.borrow()
    }
    pub async fn cancelled(&self) {
        let mut receiver = self.signal.subscribe();
        let _ = receiver.wait_for(|cancelled| *cancelled).await;
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn cancellation_before_subscription_is_not_lost() {
        let cancellation = super::Cancellation::new();
        cancellation.cancel();
        cancellation.cancelled().await;
        assert!(cancellation.is_cancelled());
    }
}
