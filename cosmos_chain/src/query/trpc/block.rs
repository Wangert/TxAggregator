use tendermint::block::Height;
use tendermint_rpc::{Client, HttpClient};

use crate::{
    error::Error,
    query::types::{Block, BlockResults},
};

pub async fn latest_block(trpc: &mut HttpClient) -> Result<Block, Error> {
    let block_resp = trpc
        .latest_block()
        .await
        .map_err(|e| Error::latest_block(e))?;

    Ok(Block::from(block_resp))
}

pub async fn latest_block_results(trpc: &mut HttpClient) -> Result<BlockResults, Error> {
    let block_results_resp = trpc
        .latest_block_results()
        .await
        .map_err(|e| Error::latest_block_results(e))?;

    Ok(BlockResults::from(block_results_resp))
}

pub async fn block_results(trpc: &mut HttpClient, height: u32) -> Result<BlockResults, Error> {
    let height = Height::from(height);
    let block_results_resp = trpc
        .block_results(height)
        .await
        .map_err(|e| Error::trpc("block results".to_string(), e))?;

    Ok(BlockResults::from(block_results_resp))
}

#[cfg(test)]
pub mod trpc_block_tests {
    use crate::chain::CosmosChain;

    use super::block_results;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[actix_rt::test]
    pub async fn trpc_block_results_works() {
        init();
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);

        cosmos_chain.tendermint_rpc_connect();
        let trpc_client = cosmos_chain.tendermint_rpc_client().expect("rpc client is empty");
        let block_results = block_results(trpc_client, 50 as u32).await;

        match block_results {
            Ok(block_results) => println!("BlockResults: {:?}", block_results),
            Err(e) => println!("{}", e),
        }
    }
}
