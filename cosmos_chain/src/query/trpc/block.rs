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
