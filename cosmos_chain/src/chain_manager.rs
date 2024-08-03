use std::{borrow::BorrowMut, collections::HashMap, sync::Arc, time::Duration};

use anyhow::Chain;
use ics23::InnerOp;
use rand::seq::SliceRandom;
use rand::thread_rng;
use tendermint::serializers::bytes::vec_base64string;
use tendermint_rpc::{event, SubscriptionClient};
// use tendermint_rpc::SubscriptionClient;
use tokio::{
    sync::RwLock,
    time::{self, Instant},
};
use types::{
    ibc_core::{
        ics02_client::height::Height,
        ics04_channel::{
            aggregate_packet::AggregatePacket,
            events::{self as ChannelEvents, SendPacket, WriteAcknowledgement},
            packet::Packet,
        },
        ics24_host::identifier::ChainId,
    },
    ibc_events::{IbcEvent, IbcEventWithHeight, TxEventsWithHeightAndGasUsed},
    message::Msg,
};

use types::proto::aggregate_packet::AggregatePacket as RawAggregatePacket;

//wjt
use crate::{
    chain::CosmosChain,
    channel::Channel,
    channel_pool::ChannelPool,
    error::Error,
    event_pool::{CTXGroup, EventPool, SEND_PACKET_EVENT, WRITE_ACK_EVENT},
    group::Cluster,
    group::{self, make_groups},
    query::websocket::subscribe::EventSubscriptions,
};

pub enum GroupingType {
    NonGrouping,
    Random,
    ClusterGrouping,
    None,
}
// #[derive(Clone)]
pub struct ChainManager {
    chain_id: ChainId,
    event_subscriptions: EventSubscriptions,
    event_pool: Arc<RwLock<EventPool>>,
}

impl ChainManager {
    pub fn new(
        chain_id: ChainId,
        // event_subscriptions: EventSubscriptions,
        // event_pool: EventPool,
    ) -> Self {
        Self {
            chain_id,
            event_subscriptions: EventSubscriptions::new(),
            event_pool: Arc::new(RwLock::new(EventPool::new())),
        }
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }

    pub async fn init(&mut self, url: &str) {
        self.event_subscriptions
            .init_subscriptions(url)
            .await
            .unwrap();
    }

    pub fn listen_events_start(&mut self, g_type: GroupingType,channels: Arc<RwLock<ChannelPool>>) {
        let event_pool_clone = self.event_pool.clone();

        self.event_subscriptions
            .listen_events(self.chain_id(), event_pool_clone);

        self.grouping_start(g_type,channels);
    }

    pub fn grouping_start(&mut self, g_type: GroupingType, channels: Arc<RwLock<ChannelPool>>) {
        let ep = self.event_pool.clone();
        let cp = channels.clone();
        tokio::spawn(async move {
            loop {
                println!("Start Grouping");
                let group_size = ep.read().await.group_size as usize;
                let mut current_ctxes = ep.read().await.get_ibc_events_class();
                // ep.write().await.clear_ibc_events_class();

                for (c, events) in current_ctxes.iter_mut() {
                    let mut groups: Vec<CTXGroup> = vec![];

                    match g_type {
                        GroupingType::NonGrouping => {
                            loop {
                                let mut new_group = vec![];
                                if events.len() > group_size {
                                    new_group = events.drain(..group_size).collect::<CTXGroup>();
                                    groups.push(new_group.clone());
                                    // println!("200:{:?}", new_group);
                                } else {
                                    new_group = events.drain(..).collect::<CTXGroup>();
                                    groups.push(new_group.clone());

                                    ep.write().await.get_ibc_events_class_mut().remove(c);
                                    // println!("小于200:{:?}", new_group);
                                    break;
                                }
                            }
                        }
                        GroupingType::Random => {
                            events.shuffle(&mut thread_rng());
                            loop {
                                let mut new_group = vec![];
                                if events.len() > group_size {
                                    new_group = events.drain(..group_size).collect::<CTXGroup>();
                                    groups.push(new_group.clone());
                                    // println!("200:{:?}", new_group);
                                } else {
                                    new_group = events.drain(..).collect::<CTXGroup>();
                                    groups.push(new_group.clone());

                                    ep.write().await.get_ibc_events_class_mut().remove(c);
                                    // println!("小于200:{:?}", new_group);
                                    break;
                                }
                            }
                        }
                        GroupingType::ClusterGrouping => {
                            let (_, height, channelkey) = c;
                            let channelop = cp.read().await.query_channel_by_key(&channelkey);
                            groups =
                                make_groups(events, height, channelop, group_size)
                                    .await;
                            ep.write().await.get_ibc_events_class_mut().remove(c);
                        }
                        GroupingType::None => {}
                    }

                    // ep.write().await.get_ibc_events_class_mut().remove(c);
                    println!("我来了我来了我来了我来了我来了groups_num: {}", groups.len());

                    ep.write().await.update_ctx_pending_groups(c, groups);
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }

    pub async fn read(&self) {
        loop {
            let event = self.event_pool.write().await.read_latest_event();
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

    pub fn events_handler(
        &mut self,
        channels: Arc<RwLock<ChannelPool>>,
        completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
    ) {
        let ep = self.event_pool.clone();
        let chain_id = self.chain_id();
        tokio::spawn(async move {
            loop {
                let event = ep.write().await.read_latest_event();

                if let Some(event_with_height) = event {
                    match event_with_height.event {
                        IbcEvent::SendPacket(send_packet) => {
                            let channel_result =
                                search_channel(channels.clone(), &send_packet.packet).await;
                            match channel_result {
                                Ok(chan) => {
                                    let completed_txs_clone = completed_txs.clone();
                                    send_packet_handler_task(
                                        chain_id.clone(),
                                        chan,
                                        send_packet.packet,
                                        event_with_height.height,
                                        completed_txs_clone,
                                    )
                                    .await;
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
                                    write_acknowlegment_handler_task(
                                        chain_id.clone(),
                                        chan.flipped(),
                                        write_ack,
                                        event_with_height.height,
                                    );
                                }
                                Err(e) => {
                                    eprintln!("channel read error: {:?}", e);
                                    continue;
                                }
                            }
                        }
                        IbcEvent::AcknowledgePacket(ack_packet) => {
                            println!("[[CHAIN:{:?}]] Ack Packet: {:?}", chain_id, ack_packet);
                        }
                        _ => {
                            continue;
                        }
                    };
                } else {
                    // println!("no event");
                };

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    pub fn events_handler_test(&mut self, channels: Arc<RwLock<ChannelPool>>) {
        let ep = self.event_pool.clone();
        let chain_id = self.chain_id();
        tokio::spawn(async move {
            loop {
                let event = ep.write().await.read_latest_event();

                if let Some(event_with_height) = event {
                    match &event_with_height.event {
                        IbcEvent::SendPacket(send_packet) => {
                            let channel_result =
                                search_channel(channels.clone(), &send_packet.packet).await;
                            match channel_result {
                                Ok(chan) => {
                                    println!("+++++++++++++++++++++++++++++++++");
                                    println!("Event:{:?}", &event_with_height);
                                    let packet = &send_packet.packet.clone();
                                    let packets_proofs_map = chan
                                        .source_chain()
                                        .query_packets_merkle_proof_infos(
                                            vec![packet.clone()],
                                            &event_with_height.height,
                                        )
                                        .await
                                        .expect("query packets merkle proof error!");

                                    println!("{:?}", packets_proofs_map);
                                    println!("+++++++++++++++++++++++++++++++++");
                                }
                                Err(e) => {
                                    eprintln!("channel read error: {:?}", e);
                                    continue;
                                }
                            }
                        }
                        IbcEvent::WriteAcknowledgement(write_ack) => {
                            // let channel_result =
                            //     search_channel(channels.clone(), &write_ack.packet).await;
                            // match channel_result {
                            //     Ok(chan) => {
                            //         write_acknowlegment_handler_task(
                            //             chain_id.clone(),
                            //             chan.flipped(),
                            //             write_ack,
                            //             event_with_height.height,
                            //         );
                            //     }
                            //     Err(e) => {
                            //         eprintln!("channel read error: {:?}", e);
                            //         continue;
                            //     }
                            // }
                        }
                        IbcEvent::AcknowledgePacket(ack_packet) => {
                            println!("[[CHAIN:{:?}]] Ack Packet: {:?}", chain_id, ack_packet);
                        }
                        _ => {
                            continue;
                        }
                    };
                } else {
                    // println!("no event");
                };

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    pub fn events_aggregate_send_packet_handler(
        &mut self,
        channels: Arc<RwLock<ChannelPool>>,
        completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
    ) {
        let ep = self.event_pool.clone();
        let chain_id = self.chain_id();
        tokio::spawn(async move {
            loop {
                let channel_keys = channels.read().await.all_channel_keys();
                for k in channel_keys {
                    // let events = ep
                    //     .write()
                    //     .await
                    //     .read_next_events(SEND_PACKET_EVENT, 200, k.clone());
                    let events = ep
                        .write()
                        .await
                        .next_pending_group(SEND_PACKET_EVENT, k.clone());

                    println!("Events Handler Number: {:?}", events.len());

                    let channel_result = search_channel_by_key(channels.clone(), k.as_str()).await;

                    if events.len() == 0 {
                        continue;
                    }
                    match channel_result {
                        Ok(chan) => {
                            let completed_txs_clone = completed_txs.clone();
                            send_packet_aggregate_handler_task(
                                chain_id.clone(),
                                chan,
                                events,
                                completed_txs_clone,
                            )
                            .await;
                        }
                        Err(e) => {
                            eprintln!("channel read error: {:?}", e);
                            continue;
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    pub fn events_aggregate_write_ack_handler(
        &mut self,
        channels: Arc<RwLock<ChannelPool>>,
        completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
    ) {
        let ep = self.event_pool.clone();
        let chain_id = self.chain_id();
        tokio::spawn(async move {
            loop {
                let channel_keys = channels.read().await.all_channel_keys();
                for k in channel_keys {
                    let events = ep
                        .write()
                        .await
                        .read_next_events(WRITE_ACK_EVENT, 100, k.clone());

                    let channel_result = search_channel_by_key(channels.clone(), k.as_str()).await;

                    match channel_result {
                        Ok(chan) => {
                            send_packet_aggregate_handler_task(
                                chain_id.clone(),
                                chan.flipped(),
                                events,
                                completed_txs.clone(),
                            )
                            .await;
                        }
                        Err(e) => {
                            eprintln!("channel read error: {:?}", e);
                            continue;
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }
}

async fn send_packet_aggregate_handler_task(
    chain_id: ChainId,
    channel: Channel,
    send_packet_events: Vec<IbcEventWithHeight>,
    completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
) {
    let height = send_packet_events[0].height;
    let mut packets = vec![];
    for event in send_packet_events {
        // println!("+++++++++++++++++++++++++++++++++");
        // println!("Event:{:?}", event);
        packets.push(event.event.packet().unwrap().clone());
        // let packet = event.event.packet().unwrap();
    }

    let ibc_events = send_aggregate_packet_handler(&channel, packets, height).await;

    match ibc_events {
        Ok(mut events) => {
            println!(
                "[[CHAIN:{:?}]] Events_Handler Events: {:?}",
                chain_id, events
            );

            completed_txs.write().await.append(&mut events);
        }
        Err(e) => {
            eprintln!("send packet handler error: {:?}", e);
        }
    };
}

async fn send_aggregate_packet_handler_test(
    channel: &Channel,
    packets: Vec<Packet>,
    height: Height,
) {
    let target_signer = channel
        .target_chain()
        .account()
        .get_signer()
        .expect("get signer error");
    let a_packet = channel
        .source_chain()
        .build_aggregate_packet(packets.clone(), target_signer, height)
        .await;

    println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
    match a_packet {
        Ok((p, hash_count, old_hash_count)) => {
            // println!("AggregatePacket: {:?}", p);
            let raw_p: RawAggregatePacket = p.clone().into();
            println!("RawAggregatePacket: {:?}", raw_p);
            println!();
            println!(
                "AggregatePacket: packets count({})-subproof count({})",
                p.packets.len(),
                p.proof.len(),
            );
            println!(
                "Number of on-chain hash computations: aggre({:?})-old({:?})",
                hash_count, old_hash_count
            );
            println!("Aggregate_Packet_Size: {}", std::mem::size_of_val(&raw_p));
        }
        Err(e) => eprintln!("{}", e),
    }
}

async fn send_aggregate_packet_handler(
    channel: &Channel,
    packets: Vec<Packet>,
    height: Height,
) -> Result<Vec<TxEventsWithHeightAndGasUsed>, Error> {
    let target_signer = channel.target_chain().account().get_signer()?;

    let start_time = Instant::now();
    let (a_packet, hash_count, old_hash_count) = channel
        .source_chain()
        .build_aggregate_packet(packets, target_signer, height)
        .await?;
    let d = start_time.elapsed();

    let raw_p: RawAggregatePacket = a_packet.clone().into();
    // println!("RawAggregatePacket: {:?}", raw_p);
    println!(
        "AggregatePacket: packets count({})-subproof count({})",
        raw_p.packets.len(),
        raw_p.proof.len()
    );
    println!(
        "Number of on-chain hash computations: aggre({:?})-old({:?})",
        hash_count, old_hash_count
    );
    println!("Aggregate_Packet_Size: {}", std::mem::size_of_val(&raw_p));
    println!("Build Aggregate Packet Duration: {}", d.as_millis());

    let msgs = vec![a_packet.to_any()];
    // tokio::time::sleep(Duration::from_secs(5)).await;
    let query_height = channel.source_chain().query_latest_height().await.unwrap();
    // Build message(s) to update client on target chain
    let target_update_client_msgs = channel
        .build_update_client_on_target_chain(height + 1)
        .await?;

    let update_event = channel
        .target_chain()
        .send_messages_and_wait_commit(target_update_client_msgs)
        .await?;

    println!("Update Event: {:?}", update_event);

    let events = channel
        .target_chain()
        .send_messages_and_wait_commit(msgs)
        .await?;

    Ok(events)
}

fn write_acknowlegment_aggregate_handler_task(
    chain_id: ChainId,
    channel: Channel,
    send_packet_events: Vec<IbcEventWithHeight>,
) {
    tokio::spawn(async move {
        todo!()
        // let ibc_events = send_packet_handler(&channel, &packet, height).await;
        // match ibc_events {
        //     Ok(events) => {
        //         println!(
        //             "[[CHAIN:{:?}]] Events_Handler Events: {:?}",
        //             chain_id, events
        //         );
        //     }
        //     Err(e) => {
        //         eprintln!("send packet handler error: {:?}", e);
        //     }
        // }
    });
}

async fn send_packet_handler_task(
    chain_id: ChainId,
    channel: Channel,
    packet: Packet,
    height: Height,
    completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
) {
    let ibc_events = send_packet_handler(&channel, &packet, height).await;
    match ibc_events {
        Ok(mut events) => {
            println!(
                "[[CHAIN:{:?}]] Events_Handler Events: {:?}",
                chain_id, events
            );

            completed_txs.write().await.append(&mut events);
        }
        Err(e) => {
            eprintln!("send packet handler error: {:?}", e);
        }
    };
}

async fn send_packet_handler(
    channel: &Channel,
    packet: &Packet,
    height: Height,
) -> Result<Vec<TxEventsWithHeightAndGasUsed>, Error> {
    let target_signer = channel.target_chain().account().get_signer()?;

    let msgs = channel
        .source_chain()
        .build_recv_packet(&packet, target_signer, height)
        .await?;

    // tokio::time::sleep(Duration::from_secs(5)).await;
    let query_height = channel.source_chain().query_latest_height().await.unwrap();
    // Build message(s) to update client on target chain
    let target_update_client_msgs = channel
        .build_update_client_on_target_chain(query_height + 1)
        .await?;

    let update_event = channel
        .target_chain()
        .send_messages_and_wait_commit(target_update_client_msgs)
        .await?;

    // println!("Update Event: {:?}", update_event);

    let events = channel
        .target_chain()
        .send_messages_and_wait_commit(msgs)
        .await?;

    Ok(events)
}

fn write_acknowlegment_handler_task(
    chain_id: ChainId,
    channel: Channel,
    write_ack: WriteAcknowledgement,
    height: Height,
) {
    tokio::spawn(async move {
        let ibc_events = write_acknowlegmenet_handler(&channel, &write_ack, height).await;
        match ibc_events {
            Ok(events) => {
                println!(
                    "[[CHAIN:{:?}]] Events_Handler Events: {:?}",
                    chain_id, events
                );
            }
            Err(e) => {
                eprintln!("write acknowlegment handler error: {:?}", e);
            }
        }
    });
}

async fn write_acknowlegmenet_handler(
    channel: &Channel,
    write_ack: &WriteAcknowledgement,
    height: Height,
) -> Result<Vec<TxEventsWithHeightAndGasUsed>, Error> {
    let target_signer = channel.target_chain().account().get_signer()?;

    let msgs = channel
        .source_chain()
        .build_ack_packet(write_ack, &height, target_signer)
        .await?;

    // tokio::time::sleep(Duration::from_secs(5)).await;

    let query_height = channel.source_chain().query_latest_height().await.unwrap();
    // Build message(s) to update client on target chain
    let target_update_client_msgs = channel
        .build_update_client_on_target_chain(query_height + 1)
        .await?;

    let update_event = channel
        .target_chain()
        .send_messages_and_wait_commit(target_update_client_msgs)
        .await?;

    // println!("Update Event: {:?}", update_event);

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

async fn search_channel_by_key(
    channels: Arc<RwLock<ChannelPool>>,
    channel_key: &str,
) -> Result<Channel, Error> {
    let channel_read = channels.read().await;
    let channel = channel_read.query_channel_by_key(channel_key);

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
    use types::ibc_core::ics24_host::identifier::{
        ChainId, ChannelId, ClientId, ConnectionId, PortId,
    };

    use crate::chain::CosmosChain;
    use crate::chain_manager::{ChainManager, GroupingType};
    use crate::channel::{Channel, ChannelSide};
    use crate::channel_pool::ChannelPool;
    use crate::event_pool::EventPool;
    use crate::query::websocket::subscribe::EventSubscriptions;

    #[tokio::test]
    pub async fn subscribe_works() {
        let chain_id = ChainId::default();
        // let es = EventSubscriptions::new();
        // let ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id);

        _ = cm
            .event_subscriptions
            .init_subscriptions("ws://10.176.35.58:26659/websocket")
            .await;
        // cm.listen_events_start(GroupingType::NonGrouping);

        cm.read().await;
        // cm.read_send_packet().await;
    }

    #[tokio::test]
    pub async fn events_handler_works() {
        let a_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let channel_side_a = ChannelSide::new(
            cosmos_chain_a,
            ClientId::from_str("07-tendermint-22").unwrap(),
            ConnectionId::from_str("connection-22").unwrap(),
            PortId::from_str("blog").unwrap(),
            None,
            Some(Version("blog-1".to_string())),
        );

        let channel_side_b = ChannelSide::new(
            cosmos_chain_b,
            ClientId::from_str("07-tendermint-13").unwrap(),
            ConnectionId::from_str("connection-18").unwrap(),
            PortId::from_str("blog").unwrap(),
            None,
            Some(Version("blog-1".to_string())),
        );

        let mut channel = Channel {
            ordering: Ordering::Unordered,
            side_a: channel_side_a,
            side_b: channel_side_b,
            connection_delay: Duration::from_secs(100),
        };

        let result = channel.handshake().await;
        println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        match result {
            Ok(events) => println!("Event: {:?}", events),
            Err(e) => println!("{:?}", e),
        }

        let chain_id = channel.source_chain().id();

        let mut channel_pool = ChannelPool::new();
        channel_pool
            .add_channel(channel)
            .expect("add channel error");

        // let signer = channel.target_chain().account().get_signer().unwrap();
        // let es = EventSubscriptions::new();
        // let ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id);

        _ = cm
            .event_subscriptions
            .init_subscriptions("ws://127.0.0.1:26657/websocket")
            .await;
        // cm.listen_events_start(GroupingType::NonGrouping);

        let channels = Arc::new(RwLock::new(channel_pool));
        let completed_txs = Arc::new(RwLock::new(vec![]));
        cm.events_handler(channels, completed_txs.clone());

        loop {}
    }

    #[tokio::test]
    pub async fn events_handler_b_works() {
        let a_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/mosaic_four_vals.toml";
        let b_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/mosaic_four_vals.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let channel_side_a = ChannelSide::new(
            cosmos_chain_a,
            ClientId::from_str("05-aggrelite-0").unwrap(),
            ConnectionId::from_str("connection-1").unwrap(),
            PortId::from_str("blog").unwrap(),
            Some(ChannelId::new(0)),
            Some(Version("blog-1".to_string())),
        );

        let channel_side_b = ChannelSide::new(
            cosmos_chain_b,
            ClientId::from_str("05-aggrelite-0").unwrap(),
            ConnectionId::from_str("connection-0").unwrap(),
            PortId::from_str("blog").unwrap(),
            Some(ChannelId::new(1)),
            Some(Version("blog-1".to_string())),
        );

        let mut channel = Channel {
            ordering: Ordering::Unordered,
            side_a: channel_side_a,
            side_b: channel_side_b,
            connection_delay: Duration::from_secs(100),
        };

        let chain_id = channel.target_chain().id();

        let mut channel_pool = ChannelPool::new();
        channel_pool
            .add_channel(channel)
            .expect("add channel error");

        // let signer = channel.target_chain().account().get_signer().unwrap();
        // let es = EventSubscriptions::new();
        // let ep = EventPool::new();
        let mut cm = ChainManager::new(chain_id);

        _ = cm
            .event_subscriptions
            .init_subscriptions("ws://127.0.0.1:26657/websocket")
            .await;
        // cm.listen_events_start(GroupingType::NonGrouping);

        let channels = Arc::new(RwLock::new(channel_pool));
        // println!("000000000000");
        // cm.events_handler_test(channels);

        let completed_txs = Arc::new(RwLock::new(vec![]));
        cm.events_aggregate_send_packet_handler(channels, completed_txs.clone());

        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
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
