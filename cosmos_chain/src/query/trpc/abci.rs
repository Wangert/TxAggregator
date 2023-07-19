use tendermint::abci::response::Info;
use tendermint_rpc::{ HttpClient, Client };

use crate::error::Error;

pub async fn abci_info(trpc: &mut HttpClient) -> Result<Info, Error>{
    let abci_info = trpc.abci_info().await.map_err(|e| Error::abci_info(e))?;

    Ok(abci_info)

}