use tendermint::block::{Header, Height};
use tendermint_rpc::{Client, HttpClient};

use crate::{
    error::Error,
    query::types::{Block, BlockResults, HeaderResult},
};

pub fn latest_block(trpc: &mut HttpClient) -> Result<Block, Error> {
    let rt = tokio::runtime::Runtime::new().expect("runtime create error");
    let block_resp = rt.block_on(trpc
        .latest_block())
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

pub async fn detail_block_header(trpc: &mut HttpClient, height: tendermint::block::Height) -> Result<Header, Error> {
    let detail_block_header = trpc
        .header(height)
        .await
        .map_err(|e| Error::trpc("block header".to_string(), e))?;
    Ok(detail_block_header.header)
}

pub async fn test_detail_block_header(trpc: &mut HttpClient, height: tendermint::block::Height) -> Result<HeaderResult, Error> {
    let detail_block_header = trpc
        .header(height)
        .await
        .map_err(|e| Error::trpc("block header".to_string(), e))?;
    Ok(HeaderResult::from(detail_block_header))
}

#[cfg(test)]
pub mod trpc_block_tests {
    use tendermint::block::Height;

    use crate::chain::CosmosChain;

    use super::{block_results, test_detail_block_header};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn trpc_block_results_works() {
        init();
        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut trpc_client = cosmos_chain
            .tendermint_rpc_client();
        let block_results = rt.block_on(block_results(&mut trpc_client, 50 as u32));

        match block_results {
            Ok(block_results) => println!("BlockResults: {:?}", block_results),
            Err(e) => println!("{}", e),
        }
    }

    #[actix_rt::test]
    pub async fn trpc_header_works() {
        init();
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let mut trpc_client = cosmos_chain
            .tendermint_rpc_client();

        let height = Height::from(50 as u32);
        let header_results = test_detail_block_header(&mut trpc_client, height).await;

        match header_results {
            Ok(header_results) => println!("HeaderResults: {:?}", header_results),
            Err(e) => println!("{}", e),
        }
    }
}
