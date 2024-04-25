use std::{path::Display, sync::Arc};

use tokio::sync::RwLock;
use types::ibc_events::IbcEventWithHeight;

#[derive(Debug)]
pub struct EventPool {
    ibc_events: Arc<RwLock<Vec<IbcEventWithHeight>>>,
    chain_id: String
}



impl EventPool {
    fn new(chainid: String) -> Self {
        EventPool { ibc_events: Arc::new(RwLock::new(Vec::new())), chain_id: chainid }
    }

    async fn push_events(&self, mut ibc_events: Vec<IbcEventWithHeight>) {
        self.ibc_events.write().await.append(&mut ibc_events);
    }

    async fn clear_pool(&self) -> Vec<IbcEventWithHeight> {
        let ibc_events = self.ibc_events.read().await.clone();
        self.ibc_events.write().await.clear();
// 确保同一个池
        ibc_events
    }
    //
    // async fn fetch_events(&self, mut ibc_event: IbcEventWithHeight) -> Vec<IbcEventWithHeight>{
    //     let mut lock = self.ibc_events.write().await;
    //     let mut removed_events = Vec::new();
    //
    //     lock.retain(|event| {
    //         if *event == ibc_event {
    //             removed_events.push(event.clone());
    //             return false;
    //         }
    //         true
    //     });
    //
    //     removed_events
    // }

    async fn get_event_count(&self) -> usize {
        let lock = self.ibc_events.read().await;
        lock.len()
    }

}
