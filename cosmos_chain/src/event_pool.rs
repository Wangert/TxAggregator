use std::{path::Display, sync::Arc};

use tokio::sync::RwLock;
use types::{ibc_core::ics02_client::height::Height, ibc_events::IbcEventWithHeight};

#[derive(Debug, Clone)]
pub struct EventPool {
    ibc_events: Vec<IbcEventWithHeight>,
}

impl EventPool {
    pub fn new() -> Self {
        EventPool {
            ibc_events: Vec::new(),
        }
    }

    pub fn push_events(&mut self, mut ibc_events: Vec<IbcEventWithHeight>) {
        self.ibc_events.append(&mut ibc_events);
    }

    pub fn clear_pool(&mut self) -> Vec<IbcEventWithHeight> {
        let ibc_events = self.ibc_events.clone();
        self.ibc_events.clear();

        ibc_events
    }

    pub fn read_latest_event(&mut self) -> Option<IbcEventWithHeight> {
        let event = self.ibc_events.last().cloned();
        self.ibc_events.pop();

        event
    }
}
