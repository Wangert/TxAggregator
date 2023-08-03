use std::time::Duration;

use tendermint::{node::Id as TendermintNodeId, Time};
use tendermint_light_client::{
    components::{
        clock::FixedClock,
        io::{AtHeight, Io, ProdIo},
        scheduler,
    },
    light_client::LightClient as TendermintLightClient,
    state::State as LightClientState,
    store::{memory::MemoryStore, LightStore},
    types::{LightBlock, Status},
    verifier::ProdVerifier,
};
use tendermint_rpc::HttpClient;
use types::light_clients::ics07_tendermint::{client_state::ClientState, height::Height};

use crate::{config::CosmosChainConfig, error::Error};

#[derive(Debug, Clone)]
pub struct LightClient {
    is_trusted_node: bool,
}

pub fn verify_block_header_and_fetch_light_block(
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    client_state: &ClientState,
    height: Height,
    node_id: &TendermintNodeId,
    sync_latest_block_time: Time,
) -> Result<LightBlock, Error> {
    if !chain_config.trusted_node {
        let trpc_io = build_light_client_io(trpc, chain_config, node_id);
        let light_block = fetch_light_block(trpc_io, height)?;

        return Ok(light_block);
    }

    let temporary_light_client = create_temporary_light_client(
        trpc,
        chain_config,
        client_state,
        node_id,
        sync_latest_block_time,
    );
    let mut temporary_light_client_state =
        create_temporary_light_client_state(trpc, height, chain_config, node_id)?;

    // verify height
    let light_block = temporary_light_client
        .verify_to_target(height.into(), &mut temporary_light_client_state)
        .map_err(|e| Error::light_client_verify_block(e))?;

    Ok(light_block)
}

pub fn create_temporary_light_client(
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    client_state: &ClientState,
    node_id: &TendermintNodeId,
    sync_latest_block_time: Time,
) -> TendermintLightClient {
    let clock = FixedClock::new(sync_latest_block_time);
    let verifier = ProdVerifier::default();
    let scheduler = scheduler::basic_bisecting_schedule;

    let trpc_io = build_light_client_io(trpc, chain_config, node_id);
    TendermintLightClient::new(
        node_id.clone(),
        client_state.as_light_client_options(),
        clock,
        scheduler,
        verifier,
        trpc_io,
    )
}

pub fn create_temporary_light_client_state(
    trpc: &mut HttpClient,
    height: Height,
    chain_config: &CosmosChainConfig,
    node_id: &TendermintNodeId,
) -> Result<LightClientState, Error> {
    let trpc_io = build_light_client_io(trpc, chain_config, node_id);
    let light_block = fetch_light_block(trpc_io, height)?;

    let mut store = MemoryStore::new();
    store.insert(light_block, Status::Trusted);

    Ok(LightClientState::new(store))
}

pub fn build_light_client_io(
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    node_id: &TendermintNodeId,
) -> ProdIo {
    let rpc_timeout = chain_config.rpc_timeout;
    let rpc_timeout = Duration::from_secs(rpc_timeout);

    ProdIo::new(node_id.clone(), trpc.clone(), Some(rpc_timeout))
}

pub fn fetch_light_block(trpc_io: ProdIo, height: Height) -> Result<LightBlock, Error> {
    let light_block = trpc_io
        .fetch_light_block(AtHeight::At(height.into()))
        .map_err(|e| Error::fetch_light_block(e))?;

    Ok(light_block)
}
