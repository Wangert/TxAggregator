use ibc_proto::ibc::core::client::v1::{
    query_client::QueryClient as IbcClientQueryClient, ConsensusStateWithHeight,
    QueryConsensusStateHeightsRequest, QueryConsensusStatesRequest,
};
use log::warn;
use tonic::transport::Channel;
use types::ibc_core::{ics02_client::height::Height, ics24_host::identifier::ClientId};

use crate::{common::PageRequest, error::Error};

pub async fn query_all_consensus_state_heights(
    grpc_client: &mut IbcClientQueryClient<Channel>,
    client_id: ClientId,
) -> Result<Vec<Height>, Error> {
    let page_request = PageRequest {
        limit: u32::MAX as u64,
        ..Default::default()
    };

    println!("page_request: {:?}", page_request);

    let request = tonic::Request::new(QueryConsensusStateHeightsRequest {
        client_id: client_id.to_string(),
        pagination: Some(page_request.into()),
    });

    let response = grpc_client.consensus_state_heights(request).await
        .map_err(|e| Error::grpc_status(e, "query consensus state heights".into()))?
        .into_inner();

    let mut heights: Vec<_> = response
        .consensus_state_heights
        .into_iter()
        .filter_map(|h| {
            Height::try_from(h.clone())
                .map_err(|e| {
                    warn!(
                        "failed to parse consensus state height {:?}. Error: {}",
                        h, e
                    )
                })
                .ok()
        })
        .collect();

    heights.sort_unstable();

    Ok(heights)
}

pub fn query_all_consensus_states(
    grpc_client: &mut IbcClientQueryClient<Channel>,
    client_id: ClientId,
) -> Result<Vec<ConsensusStateWithHeight>, Error> {
    let page_request = PageRequest {
        limit: u32::MAX as u64,
        ..Default::default()
    };

    let request = tonic::Request::new(QueryConsensusStatesRequest {
        client_id: client_id.to_string(),
        pagination: Some(page_request.into()),
    });

    let rt = tokio::runtime::Runtime::new().expect("runtime create error");
    let response = rt
        .block_on(grpc_client.consensus_states(request))
        .map_err(|e| Error::grpc_status(e, "query consensus states".into()))?
        .into_inner();

    Ok(response.consensus_states)
}

#[cfg(test)]
pub mod grpc_consensus_tests {
    use types::ibc_core::ics24_host::identifier::ClientId;

    use crate::{
        chain::CosmosChain,
        query::grpc::{self, connect::grpc_ibcclient_client},
    };

    #[test]
    pub fn query_all_consensus_state_heights_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/mosaic_1.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut grpc_client = rt.block_on(grpc_ibcclient_client(&cosmos_chain.config.grpc_addr));

        let client_id = ClientId::new("05-aggrelite", 0).expect("client id new error!");

        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let heights = rt.block_on(
            grpc::consensus::query_all_consensus_state_heights(&mut grpc_client, client_id));

        match heights {
            Ok(h) => println!("heights: {:?}", h),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn query_all_consensus_states_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut grpc_client = rt.block_on(grpc_ibcclient_client(&cosmos_chain.config.grpc_addr));

        let client_id = ClientId::new("07-tendermint", 16).expect("client id new error!");

        let consensus_states =
            grpc::consensus::query_all_consensus_states(&mut grpc_client, client_id);

        match consensus_states {
            Ok(c) => println!("consensus states: {:?}", c),
            Err(e) => panic!("{}", e),
        }
    }
}
