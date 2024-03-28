use ibc_proto::{google::protobuf::Any, Protobuf};
use tendermint::{abci::response::Info, block::Height as TmHeight};
use tendermint_rpc::{Client, HttpClient};
use types::{
    ibc_core::{ics02_client::height::Height, ics24_host::{
        identifier::ClientId,
        path::{ClientConsensusStatePath, ClientStatePath, IBC_QUERY_PATH},
    }},
    light_clients::ics07_tendermint::{
        client_state::ClientState, consensus_state::ConsensusState,
    },
};

use crate::{common::QueryHeight, error::Error, query::types::AbciQuery};

pub async fn abci_info(trpc: &mut HttpClient) -> Result<Info, Error> {
    let abci_info = trpc.abci_info().await.map_err(|e| Error::abci_info(e))?;

    Ok(abci_info)
}

pub async fn abci_query(
    trpc: &mut HttpClient,
    path: String,
    data: String,
    height: TmHeight,
    prove: bool,
) -> Result<AbciQuery, Error> {
    let response = trpc
        .abci_query(Some(path), data, Some(height), prove)
        .await
        .map_err(|e| Error::trpc("abci_query".to_string(), e))?;

    if !response.code.is_ok() {
        // Fail with response log.
        return Err(Error::abci_query(response, "fail response".to_string()));
    }

    if prove && response.proof.is_none() {
        // Fail due to empty proof
        return Err(Error::abci_query(response, "empty proof".to_string()));
    }

    Ok(AbciQuery::from(response))
}

pub async fn abci_query_client_state(
    trpc: &mut HttpClient,
    client_id: ClientId,
    query_height: QueryHeight,
    prove: bool,
) -> Result<ClientState, Error> {
    let client_state_path = ClientStatePath(client_id);
    let abci_query = abci_query(
        trpc,
        IBC_QUERY_PATH.into(),
        client_state_path.to_string(),
        query_height.into(),
        prove,
    )
    .await?;

    let client_state: ClientState = Protobuf::<Any>::decode_vec(&abci_query.value)
        .map_err(|e| Error::tendermint_protobuf_decode("client_state".to_string(), e))?;

    Ok(client_state)
}

pub async fn abci_query_consensus_state(
    trpc: &mut HttpClient,
    client_id: ClientId,
    consensus_height: Height,
    query_height: QueryHeight,
    prove: bool,
) -> Result<ConsensusState, Error> {
    let consensus_state_path = ClientConsensusStatePath {
        client_id,
        epoch: consensus_height.revision_number(),
        height: consensus_height.revision_height(),
    };

    let abci_query = abci_query(
        trpc,
        IBC_QUERY_PATH.into(),
        consensus_state_path.to_string(),
        query_height.into(),
        prove,
    )
    .await?;

    let consensus_state: ConsensusState = Protobuf::<Any>::decode_vec(&abci_query.value)
        .map_err(|e| Error::tendermint_protobuf_decode("consensus_state".to_string(), e))?;

    Ok(consensus_state)
}

#[cfg(test)]
pub mod abci_tests {
    use ibc_proto::{google::protobuf::Any, Protobuf};
    use tendermint::block::Height;
    use tendermint_rpc::{Client, HttpClient};
    use types::{
        ibc_core::ics24_host::{
            identifier::ClientId,
            path::{ClientStatePath, IBC_QUERY_PATH},
        },
        light_clients::ics07_tendermint::client_state::ClientState,
    };

    use crate::{
        account::Secp256k1Account,
        chain::CosmosChain,
        common::QueryHeight,
        query::trpc::{self, connect::tendermint_rpc_client},
    };

    use super::abci_query;

    #[test]
    pub fn abci_info_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut trpc_client = tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);

        let abciinfo = rt.block_on(trpc::abci::abci_info(&mut trpc_client)).expect("abci_info query error!");

        println!("abci_info: {:?}", abciinfo);
    }

    #[test]
    pub fn abci_info_o_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let trpc_client = HttpClient::new("http://127.0.0.1:26657").unwrap();

        let abciinfo = rt.block_on(trpc_client.abci_info()).unwrap();

        println!("abci_info: {:?}", abciinfo);
    }

    #[test]
    pub fn abci_status_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let trpc_client = HttpClient::new("http://127.0.0.1:26657").unwrap();

        let abciinfo = rt.block_on(trpc_client.status()).unwrap();

        println!("abci_info: {:?}", abciinfo);
    }

    #[test]
    pub fn abci_query_consensus_state_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut trpc_client = tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);

        let client_id = ClientId::new("07-tendermint", 16).expect("client id new error!");
        let query_height = QueryHeight::Latest;
        let client_state = rt.block_on(trpc::abci::abci_query_client_state(
            &mut trpc_client,
            client_id.clone(),
            query_height,
            true,
        )).expect("client_state query error!");

        println!("client_state: {:?}", client_state);

        let consensus_state = rt.block_on(trpc::abci::abci_query_consensus_state(&mut trpc_client, client_id, client_state.latest_height, query_height, true));

        match consensus_state {
            Ok(cs) => println!("consensus_state: {:?}", cs),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn abci_query_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let account = Secp256k1Account::new(
            &cosmos_chain.config.chain_a_key_path,
            &cosmos_chain.config.hd_path,
        )
        .expect("account error!");

        let mut trpc_client = tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);

        let client_id = ClientId::new("07-tendermint", 16).expect("client id new error!");
        let client_state_path = ClientStatePath(client_id);

        let r = rt.block_on(abci_query(
            &mut trpc_client,
            IBC_QUERY_PATH.into(),
            client_state_path.to_string(),
            Height::from(0_u32),
            true,
        ));

        let abci_query = match r {
            Ok(abci_query) => {
                println!("abci_query: {:?}", abci_query);
                abci_query
            }
            Err(e) => panic!("{}", e),
        };

        let client_state: ClientState =
            Protobuf::<Any>::decode_vec(&abci_query.value).expect("decode error");

        println!("ClientState: {:?}", client_state);
        // match client_state {
        //     Ok(client_state) => println!("client_state: {:?}", client_state),
        //     Err(e) => panic!("{}", e),
        // }
    }
}
