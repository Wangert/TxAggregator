use std::collections::HashMap;

use crossbeam_channel::{select, Receiver, Sender};
use serde::Serialize;
use types::ibc_core::{
    ics04_channel::packet::Packet,
    ics24_host::identifier::{ChannelId, PortId},
};
use utils::encode::base64::encode_to_base64_string;

use crate::{channel::Channel, error::Error};

#[derive(Debug, Clone)]
pub struct ChannelPool {
    channels: HashMap<String, Channel>,
    // channel_store_recv: Receiver<Channel>,  
}

impl ChannelPool {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            // channel_store_recv
        }
    }

    pub fn add_channel(&mut self, channel: Channel) -> Result<(), Error> {
        let channel_key = ChannelKey {
            source_channel_id: channel.source_chain_channel_id().cloned(),
            source_port_id: Some(channel.source_chain_port_id().clone()),
            destination_channel_id: channel.target_chain_channel_id().cloned(),
            destination_port_id: Some(channel.target_chain_port_id().clone()),
        };

        let k = encode_to_base64_string(&channel_key)
            .map_err(|e| Error::utils_encode_error("channel key".to_string(), e))?;
        self.channels.insert(k, channel);

        Ok(())
    }

    pub fn query_channel_by_key(&self, key: &str) -> Option<&Channel> {
        self.channels.get(key)
    }

    pub fn query_channel_by_packet(&self, packet: &Packet) -> Result<Option<&Channel>, Error> {
        let channel_key = ChannelKey {
            source_channel_id: Some(packet.source_channel.clone()),
            source_port_id: Some(packet.source_port.clone()),
            destination_channel_id: Some(packet.destination_channel.clone()),
            destination_port_id: Some(packet.destination_port.clone()),
        };

        let k = encode_to_base64_string(&channel_key)
            .map_err(|e| Error::utils_encode_error("channel key".to_string(), e))?;

        let v = self.channels.get(&k);

        Ok(v)
    }

    // pub async fn tasks_handler_start(&mut self) {
    //     loop {
    //         select! {
    //             recv(self.channel_store_recv) -> channel => {
    //                 match channel {
    //                     Ok(c) => {
    //                         println!("+++++++++++++++++");
    //                         println!("{:?}", c);
    //                         _ = self.add_channel(c)
    //                     },
    //                     Err(e) => println!("channel receive error: {}", e)
    //                 }
    //             }
    //         };
    //     }
    // }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelKey {
    source_channel_id: Option<ChannelId>,
    source_port_id: Option<PortId>,
    destination_channel_id: Option<ChannelId>,
    destination_port_id: Option<PortId>,
}

#[cfg(test)]
pub mod channel_pool_tests {
    use std::{borrow::BorrowMut, str::FromStr, sync::Arc, time::Duration};

    use tokio::{runtime::Runtime, sync::Mutex, time::sleep};
    use types::ibc_core::{
        ics04_channel::{channel::Ordering, version::Version},
        ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId},
    };
    use utils::encode::base64::encode_to_base64_string;

    use crate::{
        chain::CosmosChain,
        channel::{Channel, ChannelSide},
        error::Error,
    };

    use super::{ChannelKey, ChannelPool};

    pub fn create_channel(t: u64) -> Channel {
        let a_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let channel_side_a = ChannelSide::new(
            cosmos_chain_a,
            ClientId::from_str("07-tendermint-13").unwrap(),
            ConnectionId::from_str("connection-9").unwrap(),
            PortId::from_str("blog").unwrap(),
            Some(ChannelId::new(t)),
            Some(Version("blog-1".to_string())),
        );
        let channel_side_b = ChannelSide::new(
            cosmos_chain_b,
            ClientId::from_str("07-tendermint-13").unwrap(),
            ConnectionId::from_str("connection-9").unwrap(),
            PortId::from_str("blog").unwrap(),
            Some(ChannelId::new(t)),
            Some(Version("blog-1".to_string())),
        );

        Channel {
            ordering: Ordering::Unordered,
            side_a: channel_side_a,
            side_b: channel_side_b,
            connection_delay: Duration::from_secs(t),
        }
    }

    #[tokio::test]
    pub async fn multi_thread_channel_pool_rw_works() {
        // let (s, r) = crossbeam_channel::unbounded();
        let channel_pool = Arc::new(Mutex::new(ChannelPool::new()));

        let channel_pool_clone = channel_pool.clone();
        let job_1 = tokio::spawn(async move {
            let mut count = 1;
            loop {
                println!("Job_1");
                let channel = create_channel(count);
                _ = channel_pool.lock().await.add_channel(channel);

                count = count + 1;

                if count == 6 {
                    count = 1;
                }

                sleep(Duration::from_secs(2)).await;
            }
        });

        let job_2 = tokio::spawn(async move {
            let mut count = 1;
            loop {
                println!("Job_2");
                let channel = create_channel(count);
                let channel_key = ChannelKey {
                    source_channel_id: channel.source_chain_channel_id().cloned(),
                    source_port_id: Some(channel.source_chain_port_id().clone()),
                    destination_channel_id: channel.target_chain_channel_id().cloned(),
                    destination_port_id: Some(channel.target_chain_port_id().clone()),
                };

                let k = encode_to_base64_string(&channel_key)
                    .map_err(|e| Error::utils_encode_error("channel key".to_string(), e));

                match k {
                    Ok(key) => {
                        let c = channel_pool_clone
                            .lock()
                            .await
                            .query_channel_by_key(&key)
                            .cloned();

                        println!("{:?}", c);
                    }
                    Err(_) => {
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }
                
                count = count + 1;

                if count == 6 {
                    count = 1;
                }

                sleep(Duration::from_secs(2)).await;
            }
        });

        // _ = tokio::join!(job_1, job_2);

        sleep(Duration::from_secs(20)).await
    }

    // #[tokio::test]
    // pub async fn multi_thread_channel_pool_2_works() {
    //     let (s, r) = crossbeam_channel::unbounded();
    //     let channel_pool = Arc::new(ChannelPool::new(r));

    //     tokio::spawn(channel_pool.tasks_handler_start());

    //     let channel_pool_clone = channel_pool.clone();
    //     let job_1 = tokio::spawn(async move {
    //         let mut count = 1;
    //         loop {
    //             println!("Job_1");
    //             let channel = create_channel(count);
    //             _ = s.send(channel);

    //             count = count + 1;

    //             if count == 6 {
    //                 count = 1;
    //             }

    //             sleep(Duration::from_secs(2)).await;
    //         }
    //     });

    //     let job_2 = tokio::spawn(async move {
    //         let mut count = 1;
    //         loop {
    //             println!("Job_2");
    //             let channel = create_channel(count);
    //             let channel_key = ChannelKey {
    //                 source_channel_id: channel.source_chain_channel_id().cloned(),
    //                 source_port_id: Some(channel.source_chain_port_id().clone()),
    //                 destination_channel_id: channel.target_chain_channel_id().cloned(),
    //                 destination_port_id: Some(channel.target_chain_port_id().clone()),
    //             };

    //             let k = encode_to_base64_string(&channel_key)
    //                 .map_err(|e| Error::utils_encode_error("channel key".to_string(), e));

    //             match k {
    //                 Ok(key) => {
    //                     let c = channel_pool_clone
    //                         .query_channel_by_key(&key)
    //                         .cloned();

    //                     println!("{:?}", c);
    //                 }
    //                 Err(_) => {
    //                     sleep(Duration::from_secs(2)).await;
    //                     continue;
    //                 }
    //             }
                
    //             count = count + 1;

    //             if count == 6 {
    //                 count = 1;
    //             }

    //             sleep(Duration::from_secs(2)).await;
    //         }
    //     });

    //     // _ = tokio::join!(job_1, job_2);

    //     sleep(Duration::from_secs(20)).await
    // }
}
