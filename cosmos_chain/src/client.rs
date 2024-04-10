use std::{thread, time::Duration};

use ibc_proto::{
    cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient,
    ibc::core::client::v1::query_client::QueryClient as IbcClientQueryClient,
};
use log::info;
use tendermint::{block::Header, node::Id as TendermintNodeId};
use tendermint_rpc::HttpClient;
use tonic::transport::Channel;
use tracing::warn;
use types::{
    ibc_core::{
        ics02_client::{
            create_client::MsgCreateClient, height::Height, update_client::MsgUpdateClient,
        },
        ics23_commitment::specs::ProofSpecs,
        ics24_host::identifier::{chain_version, ChainId, ClientId},
    },
    light_clients::ics07_tendermint::{
        client_state::{AllowUpdate, ClientState},
        consensus_state::ConsensusState,
        trust_level::TrustLevel,
    },
    signer::Signer,
};

use crate::{
    account::Secp256k1Account,
    chain::CosmosChain,
    common::{parse_protobuf_duration, query_latest_height, query_trusted_height, QueryHeight},
    config::{CosmosChainConfig, TrustThreshold},
    error::Error,
    light_client::verify_block_header_and_fetch_light_block,
    query::{grpc, trpc},
    validate::validate_client_state,
};

pub async fn build_update_client_request(
    src_trpc_client: &mut HttpClient,
    dst_trpc_client: &mut HttpClient,
    dst_grpc_client: &mut IbcClientQueryClient<Channel>,
    client_id: ClientId,
    target_query_height: QueryHeight,
    src_chain_config: &CosmosChainConfig,
    dst_chain_config: &CosmosChainConfig,
) -> Result<MsgUpdateClient, Error> {
    // query target height
    let target_height = match target_query_height {
        QueryHeight::Latest => query_latest_height(src_trpc_client).await?,
        QueryHeight::Specific(height) => height,
    };

    // Wait for the source network to produce block(s) & reach `target_height`.
    while query_latest_height(src_trpc_client).await? < target_height {
        thread::sleep(Duration::from_millis(100));
    }

    let rt = tokio::runtime::Runtime::new().expect("runtime new error!");
    // query and verify the latest client_state of src_chain on the dst_chain
    let (client_state, _) = rt.block_on(trpc::abci::abci_query_client_state(
        dst_trpc_client,
        client_id.clone(),
        QueryHeight::Latest,
        true,
    ))?;

    let client_state_validate =
        validate_client_state(src_trpc_client, client_id.clone(), &client_state).await;

    if let Some(e) = client_state_validate {
        return Err(e);
    }

    let trusted_height =
        query_trusted_height(dst_grpc_client, client_id, &client_state, target_height).await?;

    // if trusted_height >= target_height {
    //     warn!(
    //         "skipping update: trusted height ({}) >= chain target height ({})",
    //         trusted_height, target_height
    //     );

    //     return Ok();
    // }

    todo!()
}

// pub fn build_header()

pub async fn build_create_client_request(
    trpc_client: &mut HttpClient,
    grpc_staking_client: &mut StakingQueryClient<Channel>,
    create_client_options: &CreateClientOptions,
    src_chain_config: &CosmosChainConfig,
    dst_chain_config: &CosmosChainConfig,
) -> Result<MsgCreateClient, Error> {
    // client state
    let client_state = build_client_state(
        trpc_client,
        grpc_staking_client,
        create_client_options,
        src_chain_config,
        dst_chain_config,
    ).await?;

    println!("access build consensus state");

    // consensus state
    let consensus_state = build_consensus_state(
        trpc_client,
        src_chain_config,
        &client_state,
    ).await?;

    // signer
    let account = Secp256k1Account::new(
        &src_chain_config.chain_key_path,
        &src_chain_config.hd_path,
    )?;
    let signer: Signer = account
        .address()
        .parse()
        .map_err(|e| Error::signer("address parse".to_string(), e))?;

    Ok(MsgCreateClient::new(
        client_state.into(),
        consensus_state.into(),
        signer,
    ))
}

pub async fn build_client_state(
    trpc_client: &mut HttpClient,
    grpc_staking_client: &mut StakingQueryClient<Channel>,
    create_client_options: &CreateClientOptions,
    src_chain_config: &CosmosChainConfig,
    dst_chain_config: &CosmosChainConfig,
) -> Result<ClientState, Error> {
    // query latest height
    let latest_block = trpc::block::latest_block(trpc_client).await?;
    // let abci_info = trpc::abci::abci_info(trpc_client).await?;
    // let last_block_header_info =
    //     trpc::block::detail_block_header(trpc_client, abci_info.last_block_height).await?;
    let latest_height = Height::new(
        chain_version(latest_block.header.chain_id.as_str()),
        u64::from(latest_block.header.height),
    )
    .map_err(|e| Error::block_height("new height failed".to_string(), e))?;

    // chain id
    let chain_id = ChainId::from(latest_block.header.chain_id);

    // max_clock_drift, trusting_period and trust_level setting
    let client_settings =
        ClientSettings::new(create_client_options, src_chain_config, dst_chain_config);

    // Get unbonding_period in the parameter list of the staking module
    let unbonding_period = grpc::staking::query_staking_params(grpc_staking_client).await?
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
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    client_state: &ClientState,
) -> Result<ConsensusState, Error> {
    let status = trpc::consensus::tendermint_status(trpc).await?;

    println!("status.node_info.id: {:?}", status.node_info.id);
    let verified_block = verify_block_header_and_fetch_light_block(
        trpc,
        chain_config,
        client_state,
        client_state.latest_height,
        &status.node_info.id,
        status.sync_info.latest_block_time,
    )?;

    Ok(ConsensusState::from(verified_block.target.signed_header.header))
}

#[derive(Debug, Default)]
pub struct CreateClientOptions {
    pub max_clock_drift: Option<Duration>,
    pub trusting_period: Option<Duration>,
    pub trust_level: Option<TrustLevel>,
}

/// Cosmos-specific client parameters for the `build_client_state` operation.
#[derive(Clone, Debug, Default)]
pub struct ClientSettings {
    pub max_clock_drift: Duration,
    pub trusting_period: Option<Duration>,
    pub trust_level: TrustLevel,
}

impl ClientSettings {
    pub fn new(
        options: &CreateClientOptions,
        src_chain_config: &CosmosChainConfig,
        dst_chain_config: &CosmosChainConfig,
    ) -> Self {
        let max_clock_drift = match options.max_clock_drift {
            None => calculate_client_state_drift(src_chain_config, dst_chain_config),
            Some(user_value) => {
                if user_value > Duration::from_secs(dst_chain_config.max_block_time) {
                    warn!(
                        "user specified max_clock_drift ({:?}) exceeds max_block_time \
                        of the destination chain {:?}",
                        user_value, dst_chain_config,
                    );
                }
                user_value
            }
        };

        let trust_level = options.trust_level.unwrap_or_else(|| {
            TrustLevel::new(
                src_chain_config.trust_threshold.numerator,
                src_chain_config.trust_threshold.denominator,
            )
            .unwrap()
        });

        ClientSettings {
            max_clock_drift,
            trusting_period: options.trusting_period,
            trust_level,
        }
    }
}

/// The client state clock drift must account for destination
/// chain block frequency and clock drift on source and dest.
/// https://github.com/informalsystems/hermes/issues/1445
pub fn calculate_client_state_drift(
    src_chain_config: &CosmosChainConfig,
    dst_chain_config: &CosmosChainConfig,
) -> Duration {
    Duration::from_secs(
        src_chain_config.clock_drift
            + dst_chain_config.clock_drift
            + dst_chain_config.max_block_time,
    )
}

/// Fetches the trusting period as a `Duration` from the chain config.
/// If no trusting period exists in the config, the trusting period is calculated
/// as two-thirds of the `unbonding_period`.
pub fn default_trusting_period(unbonding_period: Duration) -> Duration {
    2 * unbonding_period / 3
}
