use std::{borrow::BorrowMut, sync::Arc, time::Duration};

use anyhow::Chain;
use tendermint_rpc::SubscriptionClient;
// use tendermint_rpc::SubscriptionClient;
use tokio::{sync::RwLock, time};
use types::ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId};
//wjt
use crate::{event_pool::EventPool, query::websocket::subscribe::EventSubscriptions};
// use crate::query::websocket::subscribe::{EventPool, EventSubscriptions};
// #[derive(Clone)]
pub struct ChainManager {
    chain_id: ChainId,
    event_subscriptions: EventSubscriptions,
    event_pool: Arc<RwLock<EventPool>>,
}

impl ChainManager {
    pub fn new(
        chain_id: ChainId,
        event_subscriptions: EventSubscriptions,
        event_pool: EventPool,
    ) -> Self {
        Self {
            chain_id,
            event_subscriptions,
            event_pool: Arc::new(RwLock::new(event_pool)),
        }
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }

    pub fn listen_events_start(&mut self) {
        let event_pool_clone = self.event_pool.clone();

        self.event_subscriptions
            .listen_events(self.chain_id(), event_pool_clone);
    }

    pub async fn read(&self) {
        loop {
            let event = self.event_pool.read().await.read_latest_event();
            println!("Latest event: {:?}", event);
            time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
pub mod chain_manager_tests {
    use std::thread;
    use std::{sync::Arc, time::Duration};

    use tendermint_rpc::{event, SubscriptionClient};
    use tokio::sync::RwLock;
    use tokio::time;
    use types::ibc_core::ics02_client::height::Height;
    use types::ibc_core::ics24_host::identifier::ChainId;

    use crate::chain_manager::ChainManager;
    use crate::event_pool::EventPool;
    use crate::query::websocket::subscribe::{EventSubscriptions};

    #[tokio::test]
    pub async fn subscribe_works() {

        let chain_id = ChainId::default();
        let es = EventSubscriptions::new();
        let ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id, es, ep);

        _ = cm.event_subscriptions.init_subscriptions().await;
        cm.listen_events_start();

        cm.read().await;
    }
}
