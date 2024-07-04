use tendermint_rpc::HttpClient;
use types::{
    ibc_core::ics24_host::identifier::ClientId,
    light_clients::{client_type::ClientStateType, ics07_tendermint::client_state::ClientState},
};

use crate::{common::QueryHeight, error::Error, query::trpc};

pub async fn validate_client_state(
    src_trpc: &mut HttpClient,
    client_id: ClientId,
    client_state: ClientStateType,
) -> Option<Error> {
    match client_state {
        ClientStateType::Tendermint(cs) => {
            if cs.is_frozen() {
                return Some(Error::invalid_client_state("client is frozen".to_string()));
            }

            let consensus_state_result = trpc::abci::abci_query_consensus_state(
                src_trpc,
                client_id,
                cs.latest_height,
                QueryHeight::Latest,
                true,
            ).await;
        
            let (consensus_state, _) = match consensus_state_result {
                Ok(cs) => cs,
                Err(e) => return Some(e),
            };
        
            let consensus_state_time = consensus_state.timestamp;
        
            let latest_block = trpc::block::latest_block(src_trpc).await;
        
            let latest_block = match latest_block {
                Ok(lb) => lb,
                Err(e) => return Some(e),
            };
        
            let src_latest_block_time = latest_block.header.time;
        
            let elapsed = src_latest_block_time.duration_since(consensus_state_time).expect("compute duration error!");
        
            if cs.expired(elapsed) {
                return Some(Error::expired_client_state());
            }
        },
        ClientStateType::Aggrelite(cs) => {
            if cs.is_frozen() {
                return Some(Error::invalid_client_state("client is frozen".to_string()));
            }

            let consensus_state_result = trpc::abci::abci_query_consensus_state(
                src_trpc,
                client_id,
                cs.latest_height,
                QueryHeight::Latest,
                true,
            ).await;
        
            let (consensus_state, _) = match consensus_state_result {
                Ok(cs) => cs,
                Err(e) => return Some(e),
            };
        
            let consensus_state_time = consensus_state.timestamp;
        
            let latest_block = trpc::block::latest_block(src_trpc).await;
        
            let latest_block = match latest_block {
                Ok(lb) => lb,
                Err(e) => return Some(e),
            };
        
            let src_latest_block_time = latest_block.header.time;
        
            let elapsed = src_latest_block_time.duration_since(consensus_state_time).expect("compute duration error!");
        
            if cs.expired(elapsed) {
                return Some(Error::expired_client_state());
            }
        }
    };
    // if client_state.is_frozen() {
    //     return Some(Error::invalid_client_state("client is frozen".to_string()));
    // }

    // let consensus_state_result = trpc::abci::abci_query_consensus_state(
    //     src_trpc,
    //     client_id,
    //     client_latest_height,
    //     QueryHeight::Latest,
    //     true,
    // ).await;

    // let (consensus_state, _) = match consensus_state_result {
    //     Ok(cs) => cs,
    //     Err(e) => return Some(e),
    // };

    // let consensus_state_time = consensus_state.timestamp;

    // let latest_block = trpc::block::latest_block(src_trpc).await;

    // let latest_block = match latest_block {
    //     Ok(lb) => lb,
    //     Err(e) => return Some(e),
    // };

    // let src_latest_block_time = latest_block.header.time;

    // let elapsed = src_latest_block_time.duration_since(consensus_state_time).expect("compute duration error!");

    // if client_state.expired(elapsed) {
    //     return Some(Error::expired_client_state());
    // }

    None
}
