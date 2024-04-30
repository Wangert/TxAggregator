use std::{sync::Arc, time::Duration};

use anyhow::Chain;
use tendermint_rpc::SubscriptionClient;
// use tendermint_rpc::SubscriptionClient;
use tokio::{sync::RwLock, time};
use types::ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId};
//wjt
// use crate::{event_pool::EventPool, query::websocket::subscribe::EventSubscriptions};
use crate::query::websocket::subscribe::{EventPool, EventSubscriptions};
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

    // pub async fn subscribe_newblock_event_works(&mut self) {
    //     // let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));

    //     println!("88888888888888");
    //     // self.event_subscriptions.init_subscriptions();
    //     let chain_id = self.chain_id.clone();
    //     let ep = self.event_pool.clone();
    //     self.event_subscriptions.listen_events(chain_id, ep).await;
    //     self.event_subscriptions.client.close().unwrap();
    //     self.event_subscriptions.driver_handle.await;
    //     // time::sleep(Duration::from_secs(2)).await;
    // }

    // pub async fn read_event_pool(&mut self) {
    //     println!("2222222222222");
    //     let height = Height::new(0, 139780).unwrap();
    //     // let result = rt.block_on(es.event_pool.read());
    //     loop {
    //         let _ = self.event_pool.read().await.read_with_height(height);
    //         time::sleep(Duration::from_secs(2)).await;
    //     }
    // }
}

#[cfg(test)]
pub mod chain_manager_tests {
    use std::thread;
    use std::{sync::Arc, time::Duration};

    use tendermint_rpc::{event, SubscriptionClient};
    use tokio::time;
    use types::ibc_core::ics02_client::height::Height;
    use types::ibc_core::ics24_host::identifier::ChainId;

    use crate::chain_manager::ChainManager;
    use crate::query::websocket::subscribe::{EventPool, EventSubscriptions};
    #[test]
    pub fn start() {
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));
        println!("111111");
        let chain_id = ChainId::default();
        let mut es = EventSubscriptions::new(rt.clone());
        let mut ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id, es, ep);
        rt.block_on(cm.event_subscriptions.init_subscriptions());
        let height = Height::new(0, 193825).unwrap();
        let event_pool_clone = cm.event_pool.clone();

        rt.spawn(async move {
            
            let chain_id = cm.chain_id.clone();
            cm.event_subscriptions
                .listen_events(chain_id, cm.event_pool)
                .await;
            cm.event_subscriptions.client.close().unwrap();
            cm.event_subscriptions.driver_handle.await;
        });
        
        rt.spawn(async move {
            loop {
                event_pool_clone.read().await.read_with_height(height);
                time::sleep(Duration::from_secs(2)).await;
            }
        });
        thread::sleep(Duration::from_secs(20));
    }
}
