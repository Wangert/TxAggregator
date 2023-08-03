use tendermint_rpc::{HttpClient, Client};

use crate::{error::Error, query::types::TendermintStatus};

pub async fn tendermint_status(trpc: &mut HttpClient) -> Result<TendermintStatus, Error> {
    let status_resp = trpc.status().await.map_err(|e| Error::trpc("query tendermint status error".to_string(), e))?;
    Ok(TendermintStatus::from(status_resp))
}