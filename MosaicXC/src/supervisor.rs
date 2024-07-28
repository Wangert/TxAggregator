use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use clap::{ArgMatches, Command};
use cli::cmd::rootcmd::CMD;
use cosmos_chain::{
    chain::CosmosChain,
    chain_manager::{ChainManager, GroupingType},
    channel::{Channel, ChannelSide},
    channel_pool::ChannelPool,
    connection::{Connection, ConnectionSide},
    error::Error,
    registered_chains::RegisteredChains,
};
use tokio::{runtime::Runtime, sync::RwLock};
use types::{
    ibc_core::{
        ics04_channel::{channel::Ordering, packet::Packet, version::Version},
        ics24_host::identifier::{ChainId, ClientId, ConnectionId, PortId},
    },
    ibc_events::{IbcEventWithHeight, TxEventsWithHeightAndGasUsed},
    light_clients::client_type::ClientType,
};

use crate::cmd_matches::before_cmd_match;

pub struct Supervisor {
    registered_chains: RegisteredChains,
    channel_pool: Arc<RwLock<ChannelPool>>,
    chain_managers: HashMap<ChainId, ChainManager>,
    // rt: Arc<Runtime>,
    completed_txs: Arc<RwLock<Vec<TxEventsWithHeightAndGasUsed>>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            registered_chains: RegisteredChains::new(),
            channel_pool: Arc::new(RwLock::new(ChannelPool::new())),
            chain_managers: HashMap::new(),
            completed_txs: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn search_chain_by_id(&self, chain_id: &str) -> Option<&CosmosChain> {
        self.registered_chains
            .get_chain_by_id(&ChainId::from_string(chain_id))
    }

    pub fn query_all_chain_ids(&self) -> Vec<ChainId> {
        self.registered_chains.get_all_chain_ids()
    }

    pub async fn query_completed_txs_counts_and_total_gas(&self) -> (usize, usize) {
        let tx_counts = self.completed_txs.read().await.len();
        let tgas = self
            .completed_txs
            .read()
            .await
            .iter()
            .map(|tx| tx.gas_used as usize)
            .collect::<Vec<usize>>()
            .iter()
            .sum();

        (tx_counts, tgas)
    }

    // pub fn register_chain(&mut self, chain: &CosmosChain) {
    //     self.registered_chains.add_chain(chain)
    // }

    pub async fn search_channel_by_key(&self, key: &str) -> Option<Channel> {
        self.channel_pool.read().await.query_channel_by_key(key)
    }

    pub async fn search_channel_by_packet(
        &self,
        packet: &Packet,
    ) -> Result<Option<Channel>, Error> {
        self.channel_pool
            .read()
            .await
            .query_channel_by_packet(packet)
    }

    pub async fn add_channel(&mut self, channel: Channel) {
        let r = self.channel_pool.write().await.add_channel(channel);
        if let Err(e) = r {
            eprintln!("[add_channel]: {:?}", e);
        }
    }

    pub fn add_chain_manager(&mut self, cm: ChainManager) {
        self.chain_managers.insert(cm.chain_id(), cm);
    }

    pub async fn cmd_matches(&mut self, args: Vec<String>) -> Result<(), Error> {
        match Command::try_get_matches_from(CMD.to_owned(), args.clone()) {
            Ok(matches) => {
                self.cmd_match(&matches).await?;
            }
            Err(err) => {
                err.print().expect("Error writing Error");
            }
        };

        Ok(())
    }

    fn register_chain(&mut self, config_path: &str) -> Result<ChainId, Error> {
        let chain = CosmosChain::new(config_path);
        self.registered_chains.add_chain(&chain);

        let cm = ChainManager::new(chain.id());

        self.add_chain_manager(cm);

        Ok(chain.id())
    }

    async fn create_client(
        &self,
        source_chain_id: &str,
        target_chain_id: &str,
        client_type: &str,
    ) -> Result<Vec<TxEventsWithHeightAndGasUsed>, Error> {
        let ctype = if "tendermint".eq_ignore_ascii_case(client_type) {
            ClientType::Tendermint
        } else if "aggrelite".eq_ignore_ascii_case(client_type) {
            ClientType::Aggrelite
        } else {
            return Err(Error::client_type_not_exist());
        };

        let cosmos_chain_a = self
            .search_chain_by_id(source_chain_id)
            .ok_or_else(Error::empty_chain_id)?;
        let cosmos_chain_b = self
            .search_chain_by_id(target_chain_id)
            .ok_or_else(Error::empty_chain_id)?;

        let client_settings = cosmos_chain_a.client_settings(&cosmos_chain_b.config);
        let client_state = cosmos_chain_b
            .build_client_state(&client_settings, ctype)
            .await?;
        let consensus_state = cosmos_chain_b
            .build_consensus_state(client_state.clone())
            .await?;

        let msgs = cosmos_chain_a
            .build_create_client_msg(client_state, consensus_state)
            .await?;

        cosmos_chain_a.send_messages_and_wait_commit(msgs).await
    }

    async fn create_connection(
        &self,
        source_chain_id: &str,
        target_chain_id: &str,
        source_client: &str,
        target_client: &str,
    ) -> Result<Connection, Error> {
        let cosmos_chain_a = self
            .search_chain_by_id(source_chain_id)
            .ok_or_else(Error::empty_chain_id)?;
        let cosmos_chain_b = self
            .search_chain_by_id(target_chain_id)
            .ok_or_else(Error::empty_chain_id)?;

        let mut connection_side_a = ConnectionSide::new(
            cosmos_chain_a.clone(),
            ClientId::from_str(source_client).map_err(Error::identifier_error)?,
        );
        let mut connection_side_b = ConnectionSide::new(
            cosmos_chain_b.clone(),
            ClientId::from_str(target_client).map_err(Error::identifier_error)?,
        );

        connection_side_a.connection_id = None;
        connection_side_b.connection_id = None;
        let mut connection =
            Connection::new(connection_side_a, connection_side_b, Duration::from_secs(0));

        connection.handshake().await?;

        Ok(connection)
    }

    async fn create_channel(
        &mut self,
        source: CreateChannelParams,
        target: CreateChannelParams,
    ) -> Result<Channel, Error> {
        let cosmos_chain_a = self
            .search_chain_by_id(source.chain_id.as_str())
            .ok_or_else(Error::empty_chain_id)?;
        let cosmos_chain_b = self
            .search_chain_by_id(target.chain_id.as_str())
            .ok_or_else(Error::empty_chain_id)?;

        let channel_side_a = ChannelSide::new(
            cosmos_chain_a.clone(),
            source.client_id,
            source.conn_id,
            source.port_id,
            None,
            Some(source.version),
        );

        let channel_side_b = ChannelSide::new(
            cosmos_chain_b.clone(),
            target.client_id,
            target.conn_id,
            target.port_id,
            None,
            Some(target.version),
        );

        let mut channel = Channel {
            ordering: Ordering::Unordered,
            side_a: channel_side_a,
            side_b: channel_side_b,
            connection_delay: Duration::from_secs(100),
        };

        // let result = rt.block_on(channel.channel_handshake());
        channel.handshake().await?;

        Ok(channel)
    }

    async fn start(&mut self, mode: &str, gtype: &str) {
        let chains = self.registered_chains.clone();
        for (_, cm) in &mut self.chain_managers {
            let chain = chains.get_chain_by_id(&cm.chain_id());

            let mut url = String::new();
            if let Some(chain) = chain {
                let u = format!("ws://{}/websocket", chain.config.tendermint_rpc_addr);
                url = u.clone();
            }

            let g_type = if "0".eq_ignore_ascii_case(gtype) {
                GroupingType::NonGrouping
            } else if "1".eq_ignore_ascii_case(gtype) {
                GroupingType::Random
            } else if "2".eq_ignore_ascii_case(gtype) {
                GroupingType::ClusterGrouping
            } else {
                GroupingType::None
            };

            if "mosaicxc".eq_ignore_ascii_case(mode) {
                let channels = self.channel_pool.clone();
                cm.init(url.as_str()).await;
                cm.listen_events_start(g_type,channels);
                // let channels = self.channel_pool.clone();
                // cm.init(url.as_str()).await;
                // cm.listen_events_start(channels);
                
                cm.events_aggregate_send_packet_handler(self.channel_pool.clone(), self.completed_txs.clone());
            } else if "cosmosibc".eq_ignore_ascii_case(mode) {
                let channels = self.channel_pool.clone();
                cm.init(url.as_str()).await;
                cm.listen_events_start(g_type,channels);
                // let channels = self.channel_pool.clone();
                // cm.init(url.as_str()).await;
                // cm.listen_events_start(channels);
                
                cm.events_handler(self.channel_pool.clone(), self.completed_txs.clone());
            } else {
                println!("!!!!mode is not exist!!!!");
            }
        }
        }
    

    async fn cmd_match(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        match matches.subcommand() {
            Some(("chain", sub_matches)) => {
                let chain_command = sub_matches.subcommand().unwrap();
                match chain_command {
                    ("register", sub_matches) => {
                        let config = sub_matches.get_one::<String>("config");
                        println!();
                        println!("[Chain Register]:");
                        println!("Chain_Configure_File_Path({:?})", config);

                        if let Some(path) = config {
                            let chain_id = self.register_chain(path)?;
                            println!("*********************************************");
                            println!("chain{:?} register successful!", chain_id);
                        }
                    }
                    ("queryall", sub_matches) => {
                        let all_chain_ids = self.query_all_chain_ids();
                        println!();
                        println!("[All chains]:");
                        println!("{:#?}", all_chain_ids);
                    }

                    _ => unreachable!(),
                }
            }
            Some(("client", sub_matches)) => {
                let client_command = sub_matches.subcommand().unwrap();
                match client_command {
                    ("create", sub_matches) => {
                        let source_chain = sub_matches
                            .get_one::<String>("source")
                            .ok_or_else(Error::empty_chain_id)?;
                        let target_chain = sub_matches
                            .get_one::<String>("target")
                            .ok_or_else(Error::empty_chain_id)?;
                        let client_type = sub_matches
                            .get_one::<String>("clienttype")
                            .ok_or_else(Error::empty_client_type)?;

                        println!();
                        println!("[Client Create]:");
                        println!(
                            "Source_Chain({:?}) -- Target_Chain({:?})",
                            source_chain, target_chain
                        );

                        // let rt = self.rt.clone();
                        let events = self
                            .create_client(source_chain, target_chain, client_type)
                            .await?;
                        println!("*********************************************");
                        println!("Client create successful!");
                        println!("[Events]:");
                        println!("{:#?}", events);
                    }

                    _ => unreachable!(),
                }
            }
            Some(("connection", sub_matches)) => {
                let connection_command = sub_matches.subcommand().unwrap();
                match connection_command {
                    ("create", sub_matches) => {
                        let source_chain = sub_matches
                            .get_one::<String>("source")
                            .ok_or_else(Error::empty_chain_id)?;
                        let target_chain = sub_matches
                            .get_one::<String>("target")
                            .ok_or_else(Error::empty_chain_id)?;
                        let source_client = sub_matches
                            .get_one::<String>("sourceclient")
                            .ok_or_else(Error::empty_client_id)?;
                        let target_client = sub_matches
                            .get_one::<String>("targetclient")
                            .ok_or_else(Error::empty_client_id)?;

                        println!();
                        println!("[Connection Create]:");
                        println!(
                            "Source_Chain({:?}) -- Target_Chain({:?})",
                            source_chain, target_chain
                        );
                        println!(
                            "Source_Client({:?}) -- Target_Client({:?})",
                            source_client, target_client
                        );

                        // let rt = self.rt.clone();
                        let c = self
                            .create_connection(
                                source_chain,
                                target_chain,
                                source_client,
                                target_client,
                            )
                            .await?;

                        println!("*********************************************");
                        println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
                        println!("Connection create successful!");
                        println!("[Connection]:");
                        println!("{}", c);
                    }

                    _ => unreachable!(),
                }
            }
            Some(("channel", sub_matches)) => {
                let channel_command = sub_matches.subcommand().unwrap();
                match channel_command {
                    ("create", sub_matches) => {
                        let source_chain = sub_matches
                            .get_one::<String>("source")
                            .ok_or_else(Error::empty_chain_id)?;
                        let target_chain = sub_matches
                            .get_one::<String>("target")
                            .ok_or_else(Error::empty_chain_id)?;
                        let source_client = sub_matches
                            .get_one::<String>("sourceclient")
                            .ok_or_else(Error::empty_client_id)?;
                        let target_client = sub_matches
                            .get_one::<String>("targetclient")
                            .ok_or_else(Error::empty_client_id)?;
                        let source_conn = sub_matches
                            .get_one::<String>("sourceconn")
                            .ok_or_else(Error::empty_connection_id)?;
                        let target_conn = sub_matches
                            .get_one::<String>("targetconn")
                            .ok_or_else(Error::empty_connection_id)?;
                        let source_port = sub_matches
                            .get_one::<String>("sourceport")
                            .ok_or_else(Error::empty_port_id)?;
                        let target_port = sub_matches
                            .get_one::<String>("targetport")
                            .ok_or_else(Error::empty_port_id)?;
                        let source_version = sub_matches
                            .get_one::<String>("sourceversion")
                            .ok_or_else(Error::empty_channel_version)?;
                        let target_version = sub_matches
                            .get_one::<String>("targetversion")
                            .ok_or_else(Error::empty_channel_version)?;

                        let source_params = CreateChannelParams {
                            chain_id: ChainId::from_string(source_chain),
                            client_id: ClientId::from_str(source_client)
                                .map_err(Error::identifier_error)?,
                            conn_id: ConnectionId::from_str(source_conn)
                                .map_err(Error::identifier_error)?,
                            port_id: PortId::from_str(source_port)
                                .map_err(Error::identifier_error)?,
                            version: Version(source_version.to_string()),
                        };

                        let target_params = CreateChannelParams {
                            chain_id: ChainId::from_string(target_chain),
                            client_id: ClientId::from_str(target_client)
                                .map_err(Error::identifier_error)?,
                            conn_id: ConnectionId::from_str(target_conn)
                                .map_err(Error::identifier_error)?,
                            port_id: PortId::from_str(target_port)
                                .map_err(Error::identifier_error)?,
                            version: Version(target_version.to_string()),
                        };

                        println!();
                        println!("[Channel Create]:");
                        println!(
                            "Source_Chain({:?}) -- Target_Chain({:?})",
                            source_chain, target_chain
                        );
                        println!(
                            "Source_Connection({:?}) -- Target_Connection({:?})",
                            source_conn, target_conn
                        );

                        let c = self.create_channel(source_params, target_params).await?;
                        println!("*********************************************");
                        println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
                        println!("Channel create successful!");
                        println!("[Channel]:");
                        println!("{}", c);

                        let c2 = c.flipped();
                        self.add_channel(c).await;
                        self.add_channel(c2).await;
                    }

                    _ => unreachable!(),
                }
            }
            Some(("aggregator", sub_matches)) => {
                let chain_command = sub_matches.subcommand().unwrap();
                match chain_command {
                    ("start", sub_matches) => {
                        let mode = sub_matches
                            .get_one::<String>("mode")
                            .ok_or_else(Error::mode_not_exist)?;
                        let gtype = sub_matches
                            .get_one::<String>("gtype")
                            .ok_or_else(Error::grouping_type_not_exist)?;
                        // let source_chain = sub_matches.get_one::<String>("source");
                        // let target_chain = sub_matches.get_one::<String>("target");
                        println!();
                        println!("All chain managers start!!!");
                        // println!(
                        //     "Source_Chain({:?}) -- Target_Chain({:?})",
                        //     source_chain, target_chain
                        // );

                        self.start(&mode, &gtype).await;
                    }
                    ("querytotalgas", sub_matches) => {
                        let (tx_counts, tgas) = self.query_completed_txs_counts_and_total_gas().await;
                        println!();
                        println!("[Number of ctx]: {}",tx_counts);
                        println!("[Total gas]: {}",tgas);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }
    }

pub struct CreateChannelParams {
    pub chain_id: ChainId,
    pub client_id: ClientId,
    pub conn_id: ConnectionId,
    pub port_id: PortId,
    pub version: Version,
}
