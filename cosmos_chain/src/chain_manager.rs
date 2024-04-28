use std::{sync::Arc, thread, time::Duration};

use anyhow::Chain;
use tendermint_rpc::SubscriptionClient;
use tokio::{runtime::Runtime, sync::RwLock, time};
use types::ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId};
//wjt
// use crate::{event_pool::EventPool, query::websocket::subscribe::EventSubscriptions};
use crate::query::websocket::subscribe::{EventPool, EventSubscriptions};
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

    pub fn subscribe_newblock_event_works(&mut self,rt: Arc<Runtime>) {
        // let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));
        println!("111111");
        // let mut es = EventSubscriptions::new(rt.clone());

        println!("88888888888888");

        // let event_pool_clone = self.event_pool.clone();

        // _ = self.event_subscriptions.init_subscriptions();
        // self.event_subscriptions
        //     .listen_events(self.event_pool)
        //     .await;
        let event_pool_arc = Arc::clone(&self.event_pool);
        // let es_clone = self.event_subscriptions.clone();
        rt.block_on(self.event_subscriptions.listen_events(event_pool_arc));
        // self.event_subscriptions.client.close().unwrap();
        // self.event_subscriptions.driver_handle.await;
        // rt.block_on(self.event_subscriptions.driver_handle);
    }
    pub async fn get_event_pool(&mut self,rt: Arc<Runtime>){
        //let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));
        let height = Height::new(0, 139780).unwrap();
        loop {
            
            let result = self.event_pool.read().await;
            let pool = &*result;
            let _ = pool.read_with_height(height);
        }
    
    }
}


#[cfg(test)]
pub mod chain_manager_tests{
    use std::{sync::Arc, time::Duration};
    use std::thread;
    use tendermint_rpc::SubscriptionClient;
    use tokio::sync::RwLock;
    use tokio::time;
    use types::ibc_core::ics02_client::height::Height;
    use types::ibc_core::ics24_host::identifier::ChainId;
    
    use crate::chain_manager::ChainManager;
    use crate::{
        query::websocket::subscribe::{EventPool,EventSubscriptions},
    };
    
    pub fn start(){
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));
        println!("111111");
        let chain_id = ChainId::default();
        let mut es = EventSubscriptions::new(rt.clone());
        let mut ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id, es, ep);
        
        _ = cm.event_subscriptions.init_subscriptions();
        let subscribe_thread = tokio::spawn(async move{
            cm.subscribe_newblock_event_works(rt.clone());
            cm.event_subscriptions.client.close().unwrap();
            cm.event_subscriptions.driver_handle.await;
        });
        let read_thread =tokio::spawn(async move{
            cm.get_event_pool(rt.clone());
        });
        thread::sleep(Duration::from_secs(20));
    }
}