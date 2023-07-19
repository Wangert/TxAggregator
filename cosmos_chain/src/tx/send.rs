use tendermint_rpc::{
    endpoint::broadcast::tx_async::Response as TxAsyncResponse,
    endpoint::broadcast::tx_sync::Response as TxSyncResponse, Client, HttpClient,
};

use crate::error::Error;

pub async fn broadcast_tx_sync(
    trpc_client: &HttpClient,
    tx_bytes: Vec<u8>,
) -> Result<TxSyncResponse, Error> {
    let response = trpc_client
        .broadcast_tx_sync(tx_bytes)
        .await
        .map_err(|e| Error::trpc("broadcast tx sync".to_string(), e))?;
    Ok(response)
}

pub async fn broadcast_tx_async(
    trpc_client: &HttpClient,
    tx_bytes: Vec<u8>,
) -> Result<TxAsyncResponse, Error> {
    let response = trpc_client
        .broadcast_tx_async(tx_bytes)
        .await
        .map_err(|e| Error::trpc("broadcast tx async".to_string(), e))?;
    Ok(response)
}
