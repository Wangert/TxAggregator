use std::time::Duration;

use itertools::Itertools;
use log::info;
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
use tracing::trace;
use types::{
    ibc_core::ics02_client::height::Height,
    light_clients::ics07_tendermint::{client_state::ClientState, header::Header},
};

use crate::{config::CosmosChainConfig, error::Error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified<H> {
    /// Verified target
    pub target: H,
    /// Supporting headers needed to verify `target`
    pub supporting: Vec<H>,
}

#[derive(Debug, Clone)]
pub struct LightClient {
    is_trusted_node: bool,
}

pub fn verify_block_header_and_fetch_light_block(
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    client_state: &ClientState,
    target_height: Height,
    node_id: &TendermintNodeId,
    sync_latest_block_time: Time,
) -> Result<Verified<LightBlock>, Error> {
    if !chain_config.trusted_node {
        println!("trusted node is false");
        let trpc_io = build_light_client_io(trpc, chain_config, node_id);
        let light_block = fetch_light_block(&trpc_io, target_height)?;

        println!(
            "[verify_block_header_and_fetch_light_block] Light Block: {:?}",
            light_block
        );
        return Ok(Verified {
            target: light_block,
            supporting: vec![],
        });
    }

    let temporary_light_client = create_temporary_light_client(
        trpc,
        chain_config,
        client_state,
        node_id,
        sync_latest_block_time,
    );
    let mut temporary_light_client_state =
        create_temporary_light_client_state(trpc, target_height, chain_config, node_id)?;

    // verify height
    let light_block = temporary_light_client
        .verify_to_target(target_height.into(), &mut temporary_light_client_state)
        .map_err(|e| Error::light_client_verify_block(e))?;

    // Collect the verification trace for the target block
    let target_trace = temporary_light_client_state.get_trace(light_block.height());

    // Compute the supporting set, sorted by ascending height, omitting the target header
    let supporting_blocks = target_trace
        .into_iter()
        .unique_by(LightBlock::height)
        .sorted_by_key(LightBlock::height)
        .filter(|lb| lb.height() != light_block.height())
        .collect_vec();

    Ok(Verified {
        target: light_block,
        supporting: supporting_blocks,
    })
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
    let light_block = fetch_light_block(&trpc_io, height)?;

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

pub fn fetch_light_block(trpc_io: &ProdIo, height: Height) -> Result<LightBlock, Error> {
    println!("access fetch light block");
    let light_block = trpc_io
        .fetch_light_block(AtHeight::At(height.into()))
        .map_err(|e| Error::fetch_light_block(e))?;

    Ok(light_block)
}

fn adjust_headers(
    trpc: &mut HttpClient,
    chain_config: &CosmosChainConfig,
    node_id: &TendermintNodeId,
    trusted_height: Height,
    target: LightBlock,
    supporting: Vec<LightBlock>,
) -> Result<(Header, Vec<Header>), Error> {
    trace!(
        trusted = %trusted_height, target = %target.height(),
        "adjusting headers with {} supporting headers", supporting.len()
    );

    let prodio = build_light_client_io(trpc, chain_config, node_id);

    // Get the light block at trusted_height + 1 from chain.
    let trusted_validator_set = fetch_light_block(&prodio, trusted_height.increment())?.validators;

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
    let (latest_trusted_height, latest_trusted_validator_set) = match supporting_headers.last() {
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

#[cfg(test)]
pub mod light_client_tests {
    use futures::TryFutureExt;
    use types::ibc_core::{ics02_client::height::Height, ics24_host::identifier::chain_version};

    use crate::{
        chain::CosmosChain,
        query::{grpc::connect::grpc_auth_client, trpc},
    };

    use super::{build_light_client_io, fetch_light_block};

    #[test]
    pub fn fetch_light_block_works() {
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut trpc_client =
            trpc::connect::tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);

        let rt = tokio::runtime::Runtime::new().unwrap();

        let latest_block = rt
            .block_on(trpc::block::latest_block(&mut trpc_client))
            .expect("latest block error!");

        let latest_height = Height::new(
            chain_version(latest_block.header.chain_id.as_str()),
            u64::from(latest_block.header.height),
        )
        .expect("latest height error!");

        let status = rt
            .block_on(cosmos_chain.query_tendermint_status())
            .expect("query tendermint status error");
        let prod_io =
            build_light_client_io(&mut trpc_client, &cosmos_chain.config, &status.node_info.id);

        let light_block = fetch_light_block(&prod_io, latest_height);

        match light_block {
            Ok(light_block) => println!("Light_block: {:?}", light_block),
            Err(e) => panic!("{:?}", e),
        }
    }

    #[actix_rt::test]
    pub async fn asy_fecth_light_block_works() {
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut trpc_client =
            trpc::connect::tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);

        let rt = tokio::runtime::Runtime::new().unwrap();

        let latest_block = rt
            .block_on(trpc::block::latest_block(&mut trpc_client))
            .expect("latest block error");

        let latest_height = Height::new(
            chain_version(latest_block.header.chain_id.as_str()),
            u64::from(latest_block.header.height),
        )
        .expect("latest height error!");

        let status = rt
            .block_on(cosmos_chain.query_tendermint_status())
            .expect("query tendermint status error");
        let prod_io =
            build_light_client_io(&mut trpc_client, &cosmos_chain.config, &status.node_info.id);

        let light_block = fetch_light_block(&prod_io, latest_height);

        match light_block {
            Ok(light_block) => println!("Light_block: {:?}", light_block),
            Err(e) => panic!("{:?}", e),
        }
    }
}
