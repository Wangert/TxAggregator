use futures::TryFutureExt;
use http::Uri;
use ibc_proto::{
    cosmos::{
        auth::v1beta1::{
            query_client::QueryClient as AuthQueryClient, BaseAccount, EthAccount,
            QueryAccountRequest,
        },
        staking::v1beta1::{
            query_client::QueryClient as StakingQueryClient, Params as StakingParams,
        },
        tx::v1beta1::service_client::ServiceClient as TxServiceClient,
    },
    google::protobuf::Any,
    ibc::core::{
        channel::v1::query_client::QueryClient,
        client::v1::{
            query_client::QueryClient as IbcClientQueryClient,
            MsgCreateClient as IbcMsgCreateClient,
        },
        connection::v1::query_client::QueryClient as ConnectionQueryClient,
    },
    Protobuf,
};
use log::{error, info, trace};
use prost::Message;
use std::{sync::Arc, thread, time::Duration};
use tendermint::{
    abci::response::Info,
    block::{Header as TendermintHeader, Height as TendermintHeight},
};
use tendermint_light_client::types::LightBlock;
use tendermint_rpc::{Client, HttpClient};
use tokio::runtime::Runtime;
use tonic::transport::Channel;
use tracing::{debug, info as tracing_info, info_span};
use types::{
    ibc_core::{
        ics02_client::{
            create_client::{MsgCreateClient, CREATE_CLIENT_TYPE_URL},
            header::AnyHeader,
            height::Height,
            update_client::MsgUpdateClient,
        },
        ics03_connection::{
            connection::{ConnectionEnd, State},
            version::Version,
        },
        ics04_channel::{
            channel::ChannelEnd,
            events::WriteAcknowledgement,
            packet::{MsgAcknowledgement, Packet, RecvPacket, Sequence},
        },
        ics23_commitment::{
            commitment::{CommitmentPrefix, CommitmentProofBytes},
            merkle_tree::MerkleProof,
            specs::ProofSpecs,
        },
        ics24_host::{
            identifier::{chain_version, ChainId, ChannelId, ClientId, ConnectionId, PortId},
            path::{ClientConsensusStatePath, IBC_QUERY_PATH},
        },
    },
    ibc_events::{IbcEvent, IbcEventWithHeight},
    light_clients::ics07_tendermint::{
        client_state::{AllowUpdate, ClientState},
        consensus_state::ConsensusState,
        header::Header,
    },
    message::Msg,
    proofs::{ConsensusProof, Proofs},
    signer::Signer,
};
use utils::encode::protobuf;

use crate::{
    account::{self, Secp256k1Account},
    client::{
        build_consensus_state, build_create_client_request, default_trusting_period,
        ClientSettings, CreateClientOptions,
    },
    common::{parse_protobuf_duration, QueryHeight},
    config::{default::max_grpc_decoding_size, load_cosmos_chain_config, CosmosChainConfig},
    connection::ConnectionMsgType,
    error::Error,
    light_client::{
        build_light_client_io, fetch_light_block, verify_block_header_and_fetch_light_block,
        Verified,
    },
    query::{
        grpc::{self, account::query_detail_account, consensus::query_all_consensus_state_heights},
        trpc,
        types::{Block, BlockResults, TendermintStatus},
    },
    tx::{batch::batch_messages, send::send_tx, types::Memo},
    validate::validate_client_state,
};

#[derive(Debug, Clone)]
pub struct CosmosChain {
    pub id: ChainId,
    pub config: CosmosChainConfig,
    pub account: Secp256k1Account,
    // pub rt: Arc<Runtime>,
}

impl CosmosChain {
    pub fn new(path: &str) -> Self {
        let config = load_cosmos_chain_config(path);
        let config = match config {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };

        let account = match Secp256k1Account::new(&config.chain_key_path, &config.hd_path) {
            Ok(a) => a,
            Err(e) => panic!("New Secp256k1 Account Error: {}", e),
        };

        CosmosChain {
            id: ChainId::from_string(&config.chain_id),
            config: config,
            account,
            // rt: Arc::new(Runtime::new().expect("Cosmos chain runtime new error!")),
        }
    }

    pub fn id(&self) -> ChainId {
        self.id.clone()
    }

    pub fn account(&self) -> &Secp256k1Account {
        &self.account
    }

    pub fn query_compatible_versions(&self) -> Vec<Version> {
        vec![Version::default()]
    }

    pub fn query_commitment_prefix(&self) -> Result<CommitmentPrefix, Error> {
        CommitmentPrefix::try_from(self.config.store_prefix.as_bytes().to_vec())
            .map_err(Error::commitment_error)
    }

    pub async fn send_messages_and_wait_commit(
        &self,
        msgs: Vec<Any>,
    ) -> Result<Vec<IbcEventWithHeight>, Error> {
        if msgs.is_empty() {
            return Ok(vec![]);
        }

        let mut grpc_query_client = self.grpc_auth_client().await;

        let chain_config = self.config.clone();
        let key_account = self.account();

        let account_detail =
            query_detail_account(&mut grpc_query_client, key_account.address().as_str()).await?;

        let memo = Memo::new(self.config.memo_prefix.clone()).map_err(Error::memo)?;
        let msg_batches =
            batch_messages(&chain_config, &key_account, &account_detail, &memo, msgs)?;

        let mut ibc_events_with_height = vec![];
        let mut trpc_client = self.tendermint_rpc_client();
        let mut grpc_service_client = self.grpc_tx_sevice_client().await;

        // println!("msg_batch_number: {}", msg_batches.len());
        for msg_batch in msg_batches {
            let tx_results = send_tx(
                &self.config,
                &mut trpc_client,
                &mut grpc_query_client,
                &mut grpc_service_client,
                &key_account,
                &memo,
                &msg_batch,
            )
            .await?;

            ibc_events_with_height.extend(tx_results.events);
        }

        Ok(ibc_events_with_height)
    }

    pub async fn build_client_state(
        &self,
        client_settings: &ClientSettings,
    ) -> Result<ClientState, Error> {
        // query latest height
        let latest_block = self.query_latest_block().await?;
        let latest_height = Height::new(
            chain_version(latest_block.header.chain_id.as_str()),
            u64::from(latest_block.header.height),
        )
        .map_err(|e| Error::block_height("new height failed".to_string(), e))?;

        // chain id
        let chain_id = ChainId::from(latest_block.header.chain_id);

        // Get unbonding_period in the parameter list of the staking module
        let unbonding_period = self
            .query_staking_params()
            .await?
            .unbonding_time
            .ok_or_else(|| {
                Error::cosmos_params("empty unbonding time in staking params".to_string())
            })?;
        let unbonding_period = parse_protobuf_duration(unbonding_period);

        // create default trusting period
        let trusting_period = default_trusting_period(unbonding_period);

        // Deprecated, but still required by CreateClient
        let allow_update = AllowUpdate {
            after_expiry: true,
            after_misbehaviour: true,
        };

        // set standards for cross-chain proof
        let proof_specs = ProofSpecs::default();
        // set the client upgrade path
        let upgrade_path = vec!["upgrade".to_string(), "upgradedIBCState".to_string()];

        // new a client state
        let client_state = ClientState::new(
            chain_id,
            client_settings.trust_level,
            trusting_period,
            unbonding_period,
            client_settings.max_clock_drift,
            latest_height,
            proof_specs,
            upgrade_path,
            allow_update,
        )
        .map_err(|e| Error::client_state("new client state failed".to_string(), e))?;

        Ok(client_state)
    }

    pub async fn build_consensus_state(
        &self,
        client_state: &ClientState,
    ) -> Result<ConsensusState, Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        build_consensus_state(&mut trpc_client, &self.config, client_state).await
    }

    pub async fn build_create_client_msg(
        &self,
        client_state: ClientState,
        consensus_state: ConsensusState,
    ) -> Result<Vec<Any>, Error> {
        let msg_create_client = MsgCreateClient::new(
            client_state.into(),
            consensus_state.into(),
            self.account().get_signer()?,
        );

        let ibc_msg_create_client = IbcMsgCreateClient::from(msg_create_client);
        let protobuf_value = protobuf::encode_to_bytes(&ibc_msg_create_client)
            .map_err(|e| Error::utils_protobuf_encode("create client msg".to_string(), e))?;
        let msg = Any {
            type_url: CREATE_CLIENT_TYPE_URL.to_string(),
            value: protobuf_value,
        };

        Ok(vec![msg])
    }

    pub fn client_settings(&self, client_src_chain_config: &CosmosChainConfig) -> ClientSettings {
        let create_client_options = CreateClientOptions {
            max_clock_drift: Some(Duration::from_secs(self.config.max_block_time)),
            trusting_period: Some(Duration::from_secs(self.config.trusting_period * 86400)),
            trust_level: None,
        };

        ClientSettings::new(
            &create_client_options,
            client_src_chain_config,
            &self.config,
        )
    }

    pub fn tendermint_rpc_client(&self) -> HttpClient {
        trpc::connect::tendermint_rpc_client(&self.config.tendermint_rpc_addr)
    }

    pub async fn grpc_auth_client(&self) -> AuthQueryClient<Channel> {
        grpc::connect::grpc_auth_client(&self.config.grpc_addr).await
    }

    pub async fn grpc_ibcclient_client(&self) -> IbcClientQueryClient<Channel> {
        grpc::connect::grpc_ibcclient_client(&self.config.grpc_addr).await
    }

    pub async fn grpc_staking_client(&self) -> StakingQueryClient<Channel> {
        grpc::connect::grpc_staking_client(&self.config.grpc_addr).await
    }

    pub async fn grpc_connection_client(&self) -> ConnectionQueryClient<Channel> {
        grpc::connect::grpc_connection_client(&self.config.grpc_addr).await
    }

    pub async fn grpc_tx_sevice_client(&self) -> TxServiceClient<Channel> {
        grpc::connect::grpc_tx_service_client(&self.config.grpc_addr).await
    }

    pub async fn grpc_channel_client(&self) -> QueryClient<Channel> {
        grpc::connect::grpc_channel_client(&self.config.grpc_addr).await
    }

    pub async fn query_abci_info(&mut self) -> Result<Info, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trpc::abci::abci_info(&mut trpc).await
    }

    pub async fn query_packet_commitment(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: &Sequence,
        height_query: QueryHeight,
        prove: bool,
    ) -> Result<(Vec<u8>, Option<MerkleProof>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        trpc::packet::query_packet_commitment(
            &mut trpc_client,
            channel_id,
            port_id,
            sequence,
            height_query,
            prove,
        )
        .await
    }

    pub async fn query_packet_acknowledgement(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequence: &Sequence,
        height_query: QueryHeight,
        prove: bool,
    ) -> Result<(Vec<u8>, Option<MerkleProof>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        trpc::packet::query_packet_acknowledgement(
            &mut trpc_client,
            channel_id,
            port_id,
            sequence,
            height_query,
            prove,
        )
        .await
    }

    pub async fn query_unreceived_packets(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        sequences: Vec<Sequence>,
    ) -> Result<Vec<Sequence>, Error> {
        let mut grpc_client = self.grpc_channel_client().await;
        grpc::packet::query_unreceived_packets(
            &mut grpc_client,
            port_id.clone(),
            channel_id.clone(),
            sequences,
        )
        .await
    }

    pub async fn query_connection(
        &self,
        connection_id: &ConnectionId,
        height_query: QueryHeight,
        prove: bool,
    ) -> Result<(ConnectionEnd, Option<MerkleProof>), Error> {
        let mut grpc_client = self.grpc_connection_client().await;
        let mut trpc_client = self.tendermint_rpc_client().clone();
        grpc::connection::query_connection(
            &mut grpc_client,
            &mut trpc_client,
            connection_id,
            height_query,
            prove,
        )
        .await
    }

    pub async fn query_channel(
        &self,
        channel_id: &ChannelId,
        port_id: &PortId,
        height_query: QueryHeight,
        prove: bool,
    ) -> Result<(ChannelEnd, Option<MerkleProof>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        grpc::channel::query_channel(&mut trpc_client, channel_id, port_id, height_query, prove)
            .await
    }

    pub async fn query_detail_account_by_address(
        &mut self,
        account_addr: &str,
    ) -> Result<BaseAccount, Error> {
        let mut grpc_client = self.grpc_auth_client().await;
        trace!("query detail account by address");

        grpc::account::query_detail_account(&mut grpc_client, account_addr).await
    }

    pub async fn query_all_accounts(&self) -> Result<Vec<BaseAccount>, Error> {
        // let span = info_span!("query_all_accounts");
        // let _span = span.enter();

        let mut grpc_client = self.grpc_auth_client().await;
        trace!("query all accounts");
        tracing_info!("query all accounts access");

        grpc::account::query_all_account(&mut grpc_client).await
    }

    pub async fn query_staking_params(&self) -> Result<StakingParams, Error> {
        let mut grpc_client = self.grpc_staking_client().await;
        trace!("query staking params");

        grpc::staking::query_staking_params(&mut grpc_client).await
    }

    pub async fn query_block_header(
        &self,
        height: TendermintHeight,
    ) -> Result<TendermintHeader, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trpc::block::detail_block_header(&mut trpc, height).await
    }

    pub async fn query_latest_block(&self) -> Result<Block, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trace!("query latest block");

        trpc::block::latest_block(&mut trpc).await
    }

    pub async fn query_block(&self, height: TendermintHeight) -> Result<Block, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trace!("query block");

        trpc::block::block(&mut trpc, height).await
    }

    pub async fn query_latest_block_results(&self) -> Result<BlockResults, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trace!("query latest block results");

        trpc::block::latest_block_results(&mut trpc).await
    }

    pub async fn query_latest_height(&self) -> Result<Height, Error> {
        let latest_block_results = self.query_latest_block_results().await?;
        let block_header = self.query_block(latest_block_results.height).await?.header;
        let revision_number = ChainId::chain_version(block_header.chain_id.as_str());
        let revision_height = u64::from(self.query_latest_block_results().await?.height);

        Height::new(revision_number, revision_height).map_err(Error::type_error)
    }

    pub async fn query_client_consensus_state(
        &self,
        client_id: &ClientId,
        target_height: Height,
        query_height: QueryHeight,
        prove: bool,
    ) -> Result<(ConsensusState, Option<MerkleProof>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        let data = ClientConsensusStatePath {
            client_id: client_id.clone(),
            epoch: target_height.revision_number(),
            height: target_height.revision_height(),
        };

        let abci_query = trpc::abci::abci_query(
            &mut trpc_client,
            IBC_QUERY_PATH.to_string(),
            data.to_string(),
            query_height.into(),
            prove,
        )
        .await?;

        let consensus_state: ConsensusState = Protobuf::<Any>::decode_vec(&abci_query.value)
            .map_err(|e| Error::tendermint_protobuf_decode("consensus_state".to_string(), e))?;

        Ok((consensus_state, abci_query.merkle_proof))
    }

    pub async fn query_client_state(
        &self,
        client_id: &ClientId,
        query_height: QueryHeight,
        prove: bool,
    ) -> Result<(ClientState, Option<MerkleProof>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        trpc::abci::abci_query_client_state(
            &mut trpc_client,
            client_id.clone(),
            query_height,
            prove,
        )
        .await
    }

    pub async fn query_tendermint_status(&self) -> Result<TendermintStatus, Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        trpc::consensus::tendermint_status(&mut trpc_client).await
    }

    pub async fn query_consensus_state_heights(
        &self,
        client_id: &ClientId,
    ) -> Result<Vec<Height>, Error> {
        let mut grpc_client = self.grpc_ibcclient_client().await;
        query_all_consensus_state_heights(&mut grpc_client, client_id.clone()).await
    }

    pub async fn query_light_blocks(
        &self,
        client_state: &ClientState,
        target_height: Height,
    ) -> Result<Verified<LightBlock>, Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        let chain_config = self.config.clone();
        let chain_status = self.query_tendermint_status().await?;

        verify_block_header_and_fetch_light_block(
            &mut trpc_client,
            &chain_config,
            &client_state,
            target_height,
            &chain_status.node_info.id,
            chain_status.sync_info.latest_block_time,
        )
    }

    pub async fn query_trusted_height(
        &self,
        target_height: Height,
        client_id: &ClientId,
        client_state: &ClientState,
    ) -> Result<Height, Error> {
        let client_latest_height = client_state.latest_height;

        if client_latest_height < target_height {
            // If the latest height of the client is already lower than the
            // target height, we can simply use it.
            Ok(client_latest_height)
        } else {
            // Potential optimization: cache the list of consensus heights
            // so that subsequent fetches can be fast.
            let cs_heights = self.query_consensus_state_heights(client_id).await?;

            // Iterate through the available consesnsus heights and find one
            // that is lower than the target height.
            cs_heights
                .into_iter()
                .find(|h| h < &target_height)
                .ok_or_else(Error::missing_smaller_trusted_height)
        }
    }

    // Built from the generating end of an event
    pub async fn build_recv_packet(
        &self,
        packet: &Packet,
        target_signer: Signer,
        height: Height,
    ) -> Result<Vec<Any>, Error> {
        let (_, proof) = self
            .query_packet_commitment(
                &packet.source_port,
                &packet.source_channel,
                &packet.sequence,
                QueryHeight::Latest,
                true,
            )
            .await?;

        let packet_proof = proof.ok_or_else(|| Error::empty_response_proof())?;

        let proofs = Proofs::new(
            CommitmentProofBytes::try_from(packet_proof).map_err(Error::commitment_error)?,
            None,
            None,
            None,
            None,
            height.increment(),
        )
        .map_err(Error::proof_error)?;

        let recv_packet = RecvPacket::new(packet.clone(), proofs, target_signer);

        Ok(vec![recv_packet.to_any()])
    }

    // Built from the generating end of an event
    pub async fn build_ack_packet(
        &self,
        write_ack: &WriteAcknowledgement,
        height: &Height,
        target_signer: Signer,
    ) -> Result<Vec<Any>, Error> {
        let (_, proof) = self
            .query_packet_acknowledgement(
                write_ack.dst_port_id(),
                write_ack.dst_channel_id(),
                write_ack.sequence(),
                QueryHeight::Latest,
                true,
            )
            .await?;

        let packet_proof = proof.ok_or_else(|| Error::empty_response_proof())?;

        let proofs = Proofs::new(
            CommitmentProofBytes::try_from(packet_proof).map_err(Error::commitment_error)?,
            None,
            None,
            None,
            None,
            height.increment(),
        )
        .map_err(Error::proof_error)?;

        let ack_packet = MsgAcknowledgement::new(
            write_ack.packet.clone(),
            write_ack.ack.clone().into(),
            proofs,
            target_signer,
        );

        Ok(vec![ack_packet.to_any()])
    }

    pub async fn build_connection_proofs_and_client_state(
        &self,
        message_type: ConnectionMsgType,
        connection_id: &ConnectionId,
        client_id: &ClientId,
        height: Height,
    ) -> Result<(Option<ClientState>, Proofs), Error> {
        let (connection_end, maybe_connection_proof) = self
            .query_connection(connection_id, QueryHeight::Specific(height), true)
            .await?;

        let Some(connection_proof) = maybe_connection_proof else {
            return Err(Error::empty_response_proof());
        };

        // Check that the connection state is compatible with the message
        match message_type {
            ConnectionMsgType::OpenTry => {
                if !connection_end.state_matches(&State::Init)
                    && !connection_end.state_matches(&State::TryOpen)
                {
                    return Err(Error::bad_connection_state());
                }
            }
            ConnectionMsgType::OpenAck => {
                if !connection_end.state_matches(&State::TryOpen)
                    && !connection_end.state_matches(&State::Open)
                {
                    return Err(Error::bad_connection_state());
                }
            }
            ConnectionMsgType::OpenConfirm => {
                if !connection_end.state_matches(&State::Open) {
                    return Err(Error::bad_connection_state());
                }
            }
        }

        let mut client_state_option = None;
        let mut client_proof_option = None;
        let mut consensus_proof_option = None;

        match message_type {
            ConnectionMsgType::OpenTry | ConnectionMsgType::OpenAck => {
                let (client_state, maybe_client_state_proof) = self
                    .query_client_state(client_id, QueryHeight::Specific(height), true)
                    .await?;

                let Some(client_state_proof) = maybe_client_state_proof else {
                    return Err(Error::empty_response_proof());
                };

                client_proof_option = Some(
                    CommitmentProofBytes::try_from(client_state_proof)
                        .map_err(Error::commitment_error)?,
                );

                let consensus_state_proof = {
                    let (_, maybe_consensus_state_proof) = self
                        .query_client_consensus_state(
                            client_id,
                            client_state.latest_height,
                            QueryHeight::Specific(height),
                            true,
                        )
                        .await?;

                    let Some(consensus_state_proof) = maybe_consensus_state_proof else {
                        return Err(Error::empty_response_proof());
                    };

                    consensus_state_proof
                };

                consensus_proof_option = Option::from(
                    ConsensusProof::new(
                        CommitmentProofBytes::try_from(consensus_state_proof)
                            .map_err(Error::commitment_error)?,
                        client_state.latest_height,
                    )
                    .map_err(Error::proof_error)?,
                );

                client_state_option = Some(client_state);
            }
            _ => {}
        }

        Ok((
            client_state_option,
            Proofs::new(
                CommitmentProofBytes::try_from(connection_proof)
                    .map_err(Error::commitment_error)?,
                client_proof_option,
                consensus_proof_option,
                None, // TODO: Retrieve host consensus proof when available
                None,
                height.increment(),
            )
            .map_err(Error::proof_error)?,
        ))
    }

    pub async fn build_channel_proofs(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        height: Height,
    ) -> Result<Proofs, Error> {
        let (_, mp) = self
            .query_channel(channel_id, port_id, QueryHeight::Specific(height), true)
            .await?;

        if let Some(channel_proof) = mp {
            let channel_proof_bytes =
                CommitmentProofBytes::try_from(channel_proof).map_err(Error::commitment_error)?;

            Proofs::new(
                channel_proof_bytes,
                None,
                None,
                None,
                None,
                height.increment(),
            )
            .map_err(Error::proof_error)
        } else {
            return Err(Error::empty_response_proof());
        }
    }

    pub async fn adjust_headers(
        &self,
        trusted_height: Height,
        target: LightBlock,
        supporting: Vec<LightBlock>,
    ) -> Result<(Header, Vec<Header>), Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        let chain_config = self.config.clone();
        let chain_status = self.query_tendermint_status().await?;

        let prodio =
            build_light_client_io(&mut trpc_client, &chain_config, &chain_status.node_info.id);

        // Get the light block at trusted_height + 1 from chain.
        let trusted_validator_set =
            fetch_light_block(&prodio, trusted_height.increment())?.validators;

        let mut supporting_headers = Vec::with_capacity(supporting.len());

        let mut current_trusted_height = trusted_height;
        let mut current_trusted_validators = trusted_validator_set.clone();

        for support in supporting {
            let header = Header {
                signed_header: support.signed_header.clone(),
                validator_set: support.validators,
                trusted_height: current_trusted_height,
                trusted_validator_set: current_trusted_validators,
            };

            // This header is now considered to be the currently trusted header
            current_trusted_height = header.height();

            // Therefore we can now trust the next validator set, see NOTE above.
            current_trusted_validators =
                fetch_light_block(&prodio, header.height().increment())?.validators;

            supporting_headers.push(header);
        }

        // a) Set the trusted height of the target header to the height of the previous
        // supporting header if any, or to the initial trusting height otherwise.
        //
        // b) Set the trusted validators of the target header to the validators of the successor to
        // the last supporting header if any, or to the initial trusted validators otherwise.
        let (latest_trusted_height, latest_trusted_validator_set) = match supporting_headers.last()
        {
            Some(prev_header) => {
                let prev_succ = fetch_light_block(&prodio, prev_header.height().increment())?;
                (prev_header.height(), prev_succ.validators)
            }
            None => (trusted_height, trusted_validator_set),
        };

        let target_header = Header {
            signed_header: target.signed_header,
            validator_set: target.validators,
            trusted_height: latest_trusted_height,
            trusted_validator_set: latest_trusted_validator_set,
        };

        Ok((target_header, supporting_headers))
    }

    pub async fn validate_client_state(
        &self,
        client_id: &ClientId,
        client_state: &ClientState,
    ) -> Option<Error> {
        let mut trpc_client = self.tendermint_rpc_client();
        validate_client_state(&mut trpc_client, client_id.clone(), client_state).await
    }

    pub async fn build_update_client_own(
        &self,
        client_id: &ClientId,
        target_height: Height,
    ) -> Result<Vec<Any>, Error> {
        trace!("build_update_client_own");
        // query consensus state on source chain
        let client_consensus_state_on_source = self
            .query_client_consensus_state(&client_id, target_height, QueryHeight::Latest, false)
            .await;

        if let Ok(_) = client_consensus_state_on_source {
            debug!("consensus state already exists at height {target_height}, skipping update");
            return Ok(vec![]);
        }

        let target_chain_latest_height = || self.query_latest_height();

        while target_chain_latest_height().await? < target_height {
            thread::sleep(Duration::from_millis(100));
        }

        // validate client state
        let (client_state, _) = self
            .query_client_state(&client_id, QueryHeight::Latest, true)
            .await?;
        let client_state_validate = self.validate_client_state(&client_id, &client_state).await;

        if let Some(e) = client_state_validate {
            return Err(e);
        }

        // Obtain the required block based on the target block height and client_state
        let verified_blocks = self
            .query_light_blocks(&client_state, target_height)
            .await?;

        let trusted_height = self
            .query_trusted_height(target_height, &client_id, &client_state)
            .await?;

        let (target_header, support_headers) = self
            .adjust_headers(
                trusted_height,
                verified_blocks.target,
                verified_blocks.supporting,
            )
            .await
            .map(|(target_header, support_headers)| {
                let header = AnyHeader::from(target_header);
                let support: Vec<AnyHeader> = support_headers
                    .into_iter()
                    .map(|h| AnyHeader::from(h))
                    .collect();
                (header, support)
            })?;

        let signer = self.account().get_signer()?;

        let mut msgs = vec![];
        for header in support_headers {
            msgs.push(MsgUpdateClient {
                header: header.into(),
                client_id: client_id.clone(),
                signer: signer.clone(),
            });
        }

        msgs.push(MsgUpdateClient {
            header: target_header.into(),
            signer,
            client_id: client_id.clone(),
        });

        let encoded_messages = msgs.into_iter().map(Msg::to_any).collect::<Vec<Any>>();

        return Ok(encoded_messages);
    }
}

#[cfg(test)]
pub mod chain_tests {
    use std::str::FromStr;

    use log::info;
    use types::ibc_core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};

    use crate::common::QueryHeight;

    use super::CosmosChain;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn create_client_works() {
        init();
        let a_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();
        // let rt_a = cosmos_chain_a.rt.clone();
        // let rt_b = cosmos_chain_b.rt.clone();
        let client_settings = cosmos_chain_a.client_settings(&cosmos_chain_b.config);
        let client_state = rt
            .block_on(cosmos_chain_b.build_client_state(&client_settings))
            .expect("build client state error!");
        let consensus_state = rt
            .block_on(cosmos_chain_b.build_consensus_state(&client_state))
            .expect("build consensus state error!");
        let msgs = rt
            .block_on(cosmos_chain_a.build_create_client_msg(client_state, consensus_state))
            .expect("build create client msg error!");

        let result = rt.block_on(cosmos_chain_a.send_messages_and_wait_commit(msgs));

        match result {
            Ok(events) => println!("Event: {:?}", events),
            Err(e) => panic!("{}", e),
        }
    }

    #[actix_rt::test]
    pub async fn grpc_connect_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);
    }

    #[actix_rt::test]
    pub async fn query_staking_params_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let staking_params = rt
            .block_on(cosmos_chain.query_staking_params())
            .expect("query_staking_params error!");

        println!("staking params: {:?}", staking_params);
    }

    #[test]
    pub fn query_client_state_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let client_id = ClientId::from_str("07-tendermint-7").expect("client id error!");
        let client_state_result =
            rt.block_on(cosmos_chain.query_client_state(&client_id, QueryHeight::Latest, true));

        match client_state_result {
            Ok((client_state, _)) => println!("client_state: {:?}", client_state),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn update_client_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let target_height = rt
            .block_on(cosmos_chain.query_latest_height())
            .expect("query latest height error!");
        let client_id = ClientId::from_str("07-tendermint-6").expect("client id error!");

        let update_client_msgs = rt
            .block_on(cosmos_chain.build_update_client_own(&client_id, target_height))
            .expect("build update client error!");

        let update_client_result =
            rt.block_on(cosmos_chain.send_messages_and_wait_commit(update_client_msgs));

        match update_client_result {
            Ok(event) => println!("Event: {:?}", event),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn query_connection_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let connection_id = ConnectionId::from_str("connection-1").expect("connection id error!");
        let connection_result =
            rt.block_on(cosmos_chain.query_connection(&connection_id, QueryHeight::Latest, true));

        match connection_result {
            Ok((connnection, _)) => println!("connection: {:?}", connnection),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn query_channel_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let channel_id = ChannelId::from_str("channel-0").expect("channel id error!");
        let port_id = PortId::from_str("transfer").unwrap();
        let channel_result = rt.block_on(cosmos_chain.query_channel(
            &channel_id,
            &port_id,
            QueryHeight::Latest,
            true,
        ));

        match channel_result {
            Ok((channel, _)) => println!("channel: {:?}", channel),
            Err(e) => panic!("{}", e),
        }
    }
}
