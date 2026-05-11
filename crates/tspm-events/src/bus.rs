use tokio::sync::broadcast;
use tspm_core::TspmEvent;

/// Event bus for TSPM using tokio broadcast channel
pub struct EventBus {
    sender: broadcast::Sender<TspmEvent>,
}

impl EventBus {
    /// Create a new EventBus with the given buffer capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: TspmEvent) {
        let _ = self.sender.send(event);
    }

    /// Subscribe to all events
    pub fn subscribe(&self) -> broadcast::Receiver<TspmEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tspm_core::StopReason;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(TspmEvent::ProcessStop {
            name: "test".into(),
            instance_id: 0,
            pid: Some(1234),
            reason: StopReason::Manual,
        });

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, TspmEvent::ProcessStop { .. }));
    }
}
