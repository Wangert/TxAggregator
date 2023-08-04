use tendermint_rpc::{HttpClient, Client};

use crate::{error::Error, query::types::TendermintStatus};

pub fn tendermint_status(trpc: &mut HttpClient) -> Result<TendermintStatus, Error> {
    let rt = tokio::runtime::Runtime::new().expect("runtime create error");
    let status_resp = rt.block_on(trpc.status()).map_err(|e| Error::trpc("query tendermint status error".to_string(), e))?;
    Ok(TendermintStatus::from(status_resp))
}