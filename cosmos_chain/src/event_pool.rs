use std::{path::Display, sync::Arc};

use tokio::sync::RwLock;
use types::ibc_events::IbcEventWithHeight;

#[derive(Debug, Clone)]
pub struct EventPool {
    ibc_events: Arc<RwLock<Vec<IbcEventWithHeight>>>,
}

impl EventPool {
    fn new() -> Self {
        EventPool {
            ibc_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn push_events(&self, mut ibc_events: Vec<IbcEventWithHeight>) {
        self.ibc_events.write().await.append(&mut ibc_events);
    }

    async fn clear_pool(&self) -> Vec<IbcEventWithHeight> {
        let ibc_events = self.ibc_events.read().await.clone();
        self.ibc_events.write().await.clear();

        ibc_events
    }
}
