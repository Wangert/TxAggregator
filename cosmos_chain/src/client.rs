use std::time::Duration;

use ibc_proto::cosmos::staking::v1beta1::query_client::QueryClient as StakingQueryClient;
use tendermint_rpc::HttpClient;
use tonic::transport::Channel;
use tracing::warn;
use types::{
    ibc_core::{
        ics02_client::create_client::MsgCreateClient,
        ics23_commitment::specs::ProofSpecs,
        ics24_host::identifier::{chain_version, ChainId},
    },
    light_clients::ics07_tendermint::{
        client_state::{AllowUpdate, ClientState},
        consensus_state::ConsensusState,
        height::Height,
        trust_level::TrustLevel,
    },
};

use crate::{
    common::parse_protobuf_duration,
    config::{CosmosChainConfig, TrustThreshold},
    error::Error,
    query::{grpc, trpc},
};

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
    )
    .await?;
    // consensus state

    // signer

    todo!()
}

async fn build_client_state(
    trpc_client: &mut HttpClient,
    grpc_staking_client: &mut StakingQueryClient<Channel>,
    create_client_options: &CreateClientOptions,
    src_chain_config: &CosmosChainConfig,
    dst_chain_config: &CosmosChainConfig,
) -> Result<ClientState, Error> {
    // query latest height
    let abci_info = trpc::abci::abci_info(trpc_client).await?;
    let last_block_header_info =
        trpc::block::detail_block_header(trpc_client, abci_info.last_block_height).await?;
    let latest_height = Height::new(
        chain_version(last_block_header_info.chain_id.as_str()),
        chain_version(last_block_header_info.chain_id.as_str()),
    )
    .map_err(|e| Error::block_height("new height failed".to_string(), e))?;

    // chain id
    let chain_id = ChainId::from(last_block_header_info.chain_id);

    // max_clock_drift, trusting_period and trust_level setting
    let client_settings =
        ClientSettings::new(create_client_options, src_chain_config, dst_chain_config);

    // Get unbonding_period in the parameter list of the staking module
    let unbonding_period = grpc::staking::query_staking_params(grpc_staking_client)
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

fn build_consensus_state() -> Result<ConsensusState, Error> {
    let consensus_state = ConsensusState {
        timestamp: todo!(),
        commitment_root: todo!(),
        next_validators_hash: todo!(),
    };
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
fn calculate_client_state_drift(
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
fn default_trusting_period(unbonding_period: Duration) -> Duration {
    2 * unbonding_period / 3
}
