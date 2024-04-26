use std::sync::Arc;

use anyhow::Chain;
use tokio::sync::RwLock;
use types::ibc_core::ics24_host::identifier::ChainId;

use crate::{event_pool::EventPool, query::websocket::subscribe::EventSubscriptions};

pub struct ChainManager {
    chain_id: ChainId,
    event_subscriptions: EventSubscriptions,
    event_pool: Arc<RwLock<EventPool>>,
}

impl ChainManager {
    pub fn new(chain_id: ChainId, event_subscriptions: EventSubscriptions, event_pool: EventPool) -> Self {
        Self {
            chain_id,
            event_subscriptions,
            event_pool: Arc::new(RwLock::new(event_pool)),
        }
    }
}
