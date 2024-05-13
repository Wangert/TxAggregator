use std::{borrow::BorrowMut, sync::Arc, time::Duration};

use anyhow::Chain;
use digest::block_buffer::Error;

use tendermint_rpc::{event, SubscriptionClient};
// use tendermint_rpc::SubscriptionClient;
use tokio::{sync::RwLock, time};
use types::{
    ibc_core::{
        ics02_client::height::Height,
        ics04_channel::{
            events::{self as ChannelEvents, SendPacket, WriteAcknowledgement},
            packet::Packet,
        },
        ics24_host::identifier::ChainId,
    },
    ibc_events::{IbcEvent, IbcEventWithHeight},
};

//wjt
use crate::{
    channel_pool::ChannelPool, event_pool::EventPool,
    query::websocket::subscribe::EventSubscriptions,
};
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
            // match event {
            //     Some(IbcEventWithHeight { event: IbcEvent::NewBlock(_), .. }) => {
            //         println!("1111");
            //     },
            //     _ => {},
            // };
            println!("Latest event: {:?}", event);
            time::sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn read_send_packet(&self, channel_pool: Arc<RwLock<ChannelPool>>) {
        loop {
            let event = self.event_pool.read().await.read_latest_event();
            let channel_pool_clone = channel_pool.clone();
            if let Some(event_with_height) = event {
                match event_with_height.event {
                    IbcEvent::SendPacket(sendpacket) => {
                        // return (sendpacket.packet, event_with_height.height);
                        let sp = sendpacket.packet.clone();
                        tokio::spawn(async move {
                            
                            
                            
                        });
                    }
                    _ => {
                        println!("other event");
                    }
                };
            } else {
                println!("no event");
            };
            time::sleep(Duration::from_secs(2)).await;
        }
    }
    pub async fn read_ack_packet(&self) -> (WriteAcknowledgement, Height) {
        loop {
            let event = self.event_pool.read().await.read_latest_event();
            if let Some(event_with_height) = event {
                match event_with_height.event {
                    IbcEvent::WriteAcknowledgement(writeAcknowledgement) => {
                        return (writeAcknowledgement, event_with_height.height);
                    }
                    _ => {
                        println!("other event");
                    }
                };
            } else {
                println!("no event");
            };
            time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
pub mod chain_manager_tests {
    use std::str::FromStr;
    use std::thread;
    use std::{sync::Arc, time::Duration};

    use tendermint_rpc::{event, SubscriptionClient};
    use tokio::sync::RwLock;
    use tokio::time;
    use types::ibc_core::ics02_client::height::Height;
    use types::ibc_core::ics04_channel::channel::Ordering;
    use types::ibc_core::ics04_channel::version::Version;
    use types::ibc_core::ics24_host::identifier::{ChainId, ClientId, ConnectionId, PortId};

    use crate::chain::CosmosChain;
    use crate::chain_manager::ChainManager;
    use crate::channel::{Channel, ChannelSide};
    use crate::event_pool::EventPool;
    use crate::query::websocket::subscribe::EventSubscriptions;

    #[tokio::test]
    pub async fn subscribe_works() {
        let chain_id = ChainId::default();
        let es = EventSubscriptions::new();
        let ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id, es, ep);

        _ = cm
            .event_subscriptions
            .init_subscriptions("ws://10.176.35.58:26656/websocket")
            .await;
        cm.listen_events_start();

        cm.read().await;
        // cm.read_send_packet().await;
    }

    

    
}
