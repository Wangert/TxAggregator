use std::{borrow::BorrowMut, sync::Arc, time::Duration};

use anyhow::Chain;
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
    channel::Channel, channel_pool::ChannelPool, error::Error, event_pool::EventPool,
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

    pub async fn events_handler(&mut self, channels: Arc<RwLock<ChannelPool>>) {
        loop {
            let event = self.event_pool.read().await.read_latest_event();

            if let Some(event_with_height) = event {
                match event_with_height.event {
                    IbcEvent::SendPacket(send_packet) => {
                        let channel_result =
                            search_channel(channels.clone(), &send_packet.packet).await;
                        match channel_result {
                            Ok(chan) => {
                                send_packet_handler_task(chan, send_packet.packet, event_with_height.height);
                            }
                            Err(e) => {
                                eprintln!("channel read error: {:?}", e);
                                continue;
                            }
                        }
                    }
                    IbcEvent::WriteAcknowledgement(write_ack) => {
                        let channel_result =
                            search_channel(channels.clone(), &write_ack.packet).await;
                        match channel_result {
                            Ok(chan) => {
                                write_acknowlegment_handler_task(chan, write_ack, event_with_height.height);
                            }
                            Err(e) => {
                                eprintln!("channel read error: {:?}", e);
                                continue;
                            }
                        }
                    }
                    IbcEvent::AcknowledgePacket(ack_packet) => {
                        println!("Ack Packet: {:?}", ack_packet);
                    }
                    _ => {
                        continue;
                    }
                };
            } else {
                println!("no event");
            };
        }
    }

    // pub async fn read_send_packet(&self, channel_pool: Arc<RwLock<ChannelPool>>) {
    //     loop {
    //         let event = self.event_pool.read().await.read_latest_event();
    //         let channel_pool_clone = channel_pool.clone();
    //         if let Some(event_with_height) = event {
    //             match event_with_height.event {
    //                 IbcEvent::SendPacket(sendpacket) => {
    //                     return (sendpacket.packet, event_with_height.height);
    //                     // let sp = sendpacket.packet.clone();
    //                     // tokio::spawn(async move {

    //                     // });
    //                 }
    //                 _ => {
    //                     println!("other event");
    //                 }
    //             };
    //         } else {
    //             println!("no event");
    //         };
    //         time::sleep(Duration::from_secs(2)).await;
    //     }
    // }
    // pub async fn read_ack_packet(&self) -> (WriteAcknowledgement, Height) {
    //     loop {
    //         let event = self.event_pool.read().await.read_latest_event();
    //         if let Some(event_with_height) = event {
    //             match event_with_height.event {
    //                 IbcEvent::WriteAcknowledgement(writeAcknowledgement) => {
    //                     return (writeAcknowledgement, event_with_height.height);
    //                 }
    //                 _ => {
    //                     println!("other event");
    //                 }
    //             };
    //         } else {
    //             println!("no event");
    //         };
    //         time::sleep(Duration::from_secs(2)).await;
    //     }
    // }
}

fn send_packet_handler_task(channel: Channel, packet: Packet, height: Height) {
    tokio::spawn(async move {
        let ibc_events =
            send_packet_handler(&channel, &packet, height).await;
        match ibc_events {
            Ok(events) => {
                println!("[Events_Handler] Events: {:?}", events);
            }
            Err(e) => {
                eprintln!("send packet handler error: {:?}", e);
            }
        }
    });
}

async fn send_packet_handler(
    channel: &Channel,
    packet: &Packet,
    height: Height,
) -> Result<Vec<IbcEventWithHeight>, Error> {
    let target_signer = channel.target_chain().account().get_signer()?;

    let msgs = channel
        .source_chain()
        .build_recv_packet(&packet, target_signer, height)
        .await?;

    let events = channel
        .target_chain()
        .send_messages_and_wait_commit(msgs)
        .await?;

    Ok(events)
}

fn write_acknowlegment_handler_task(
    channel: Channel,
    write_ack: WriteAcknowledgement,
    height: Height,
) {
    tokio::spawn(async move {
        let ibc_events =
            write_acknowlegmenet_handler(&channel, &write_ack, height).await;
        match ibc_events {
            Ok(events) => {
                println!("[Events_Handler] Events: {:?}", events);
            }
            Err(e) => {
                eprintln!("send packet handler error: {:?}", e);
            }
        }
    });
}

async fn write_acknowlegmenet_handler(
    channel: &Channel,
    write_ack: &WriteAcknowledgement,
    height: Height,
) -> Result<Vec<IbcEventWithHeight>, Error> {
    let target_signer = channel.target_chain().account().get_signer()?;

    let msgs = channel
        .source_chain()
        .build_ack_packet(write_ack, &height, target_signer)
        .await?;

    let events = channel
        .target_chain()
        .send_messages_and_wait_commit(msgs)
        .await?;

    Ok(events)
}

async fn search_channel(
    channels: Arc<RwLock<ChannelPool>>,
    packet: &Packet,
) -> Result<Channel, Error> {
    let channel_read = channels.read().await;
    let channel = channel_read.query_channel_by_packet(packet)?;

    match channel {
        Some(chan) => Ok(chan.clone()),
        None => Err(Error::empty_channel()),
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

    // #[tokio::test]
    // pub async fn send_packet_works() {
    //     let a_file_path =
    //         "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
    //     let b_file_path =
    //         "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

    //     let cosmos_chain_a = CosmosChain::new(a_file_path);
    //     let cosmos_chain_b = CosmosChain::new(b_file_path);

    //     let channel_side_a = ChannelSide::new(
    //         cosmos_chain_a,
    //         ClientId::from_str("07-tendermint-15").unwrap(),
    //         ConnectionId::from_str("connection-8").unwrap(),
    //         PortId::from_str("blog").unwrap(),
    //         None,
    //         Some(Version("blog-1".to_string())),
    //     );

    //     let channel_side_b = ChannelSide::new(
    //         cosmos_chain_b,
    //         ClientId::from_str("07-tendermint-9").unwrap(),
    //         ConnectionId::from_str("connection-6").unwrap(),
    //         PortId::from_str("blog").unwrap(),
    //         None,
    //         Some(Version("blog-1".to_string())),
    //     );

    //     let mut channel = Channel {
    //         ordering: Ordering::Unordered,
    //         side_a: channel_side_a,
    //         side_b: channel_side_b,
    //         connection_delay: Duration::from_secs(100),
    //     };

    //     let result = channel.handshake().await;
    //     println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
    //     match result {
    //         Ok(events) => println!("Event: {:?}", events),
    //         Err(e) => println!("{:?}", e),
    //     }
    //     let rt = tokio::runtime::Runtime::new().unwrap();
    //     let signer = channel.target_chain().account().get_signer().unwrap();
    //     let chain_id = channel.source_chain().id();
    //     let es = EventSubscriptions::new();
    //     let ep = EventPool::new();
    //     let mut cm = ChainManager::new(chain_id, es, ep);

    //     _ = cm
    //         .event_subscriptions
    //         .init_subscriptions("ws://10.176.35.58:26656/websocket")
    //         .await;
    //     cm.listen_events_start();
    //     let (packet, height) = cm.read_send_packet().await;

    //     let msgs = channel
    //         .source_chain()
    //         .build_recv_packet(&packet, signer, height)
    //         .await
    //         .expect("build create client msg error!");

    //     let query_height = channel.source_chain().query_latest_height().await.unwrap();

    //     // Build message(s) to update client on target chain
    //     let target_update_client_msgs = channel
    //         .build_update_client_on_target_chain(query_height + 1)
    //         .await
    //         .unwrap();

    //     let update_event = channel
    //         .target_chain()
    //         .send_messages_and_wait_commit(target_update_client_msgs)
    //         .await
    //         .unwrap();

    //     let result = channel
    //         .target_chain()
    //         .send_messages_and_wait_commit(msgs)
    //         .await;
    //     match result {
    //         Ok(events) => println!("Event: {:?}", events),
    //         Err(e) => panic!("{}", e),
    //     }
    //     loop {
    //         time::sleep(Duration::from_secs(2)).await;
    //     }
    // }

    // #[tokio::test]
    // pub async fn ack_packet_works() {
    //     let a_file_path =
    //         "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
    //     let b_file_path =
    //         "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

    //     let cosmos_chain_a = CosmosChain::new(a_file_path);
    //     let cosmos_chain_b = CosmosChain::new(b_file_path);

    //     let channel_side_a = ChannelSide::new(
    //         cosmos_chain_a,
    //         ClientId::from_str("07-tendermint-15").unwrap(),
    //         ConnectionId::from_str("connection-8").unwrap(),
    //         PortId::from_str("blog").unwrap(),
    //         None,
    //         Some(Version("blog-1".to_string())),
    //     );

    //     let channel_side_b = ChannelSide::new(
    //         cosmos_chain_b,
    //         ClientId::from_str("07-tendermint-9").unwrap(),
    //         ConnectionId::from_str("connection-6").unwrap(),
    //         PortId::from_str("blog").unwrap(),
    //         None,
    //         Some(Version("blog-1".to_string())),
    //     );

    //     let mut channel = Channel {
    //         ordering: Ordering::Unordered,
    //         side_a: channel_side_b,
    //         side_b: channel_side_a,
    //         connection_delay: Duration::from_secs(100),
    //     };

    //     let result = channel.handshake().await;
    //     println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
    //     match result {
    //         Ok(events) => println!("Event: {:?}", events),
    //         Err(e) => println!("{:?}", e),
    //     }

    //     let signer = channel.target_chain().account().get_signer().unwrap();
    //     let chain_id = channel.source_chain().id();
    //     let es = EventSubscriptions::new();
    //     let ep = EventPool::new();
    //     let mut cm = ChainManager::new(chain_id, es, ep);

    //     _ = cm
    //         .event_subscriptions
    //         .init_subscriptions("ws://10.176.35.58:26659/websocket")
    //         .await;
    //     cm.listen_events_start();
    //     let (packet, height) = cm.read_ack_packet().await;

    //     let msgs = channel
    //         .source_chain()
    //         .build_ack_packet(&packet, &height, signer)
    //         .await
    //         .expect("build create client msg error!");

    //     let query_height = channel.source_chain().query_latest_height().await.unwrap();

    //     // Build message(s) to update client on target chain
    //     let target_update_client_msgs = channel
    //         .build_update_client_on_target_chain(query_height + 1)
    //         .await
    //         .unwrap();

    //     let update_event = channel
    //         .target_chain()
    //         .send_messages_and_wait_commit(target_update_client_msgs)
    //         .await
    //         .unwrap();

    //     let result = channel
    //         .target_chain()
    //         .send_messages_and_wait_commit(msgs)
    //         .await;
    //     match result {
    //         Ok(events) => println!("Event: {:?}", events),
    //         Err(e) => panic!("{}", e),
    //     }
    //     loop {
    //         time::sleep(Duration::from_secs(2)).await;
    //     }
    // }
}
