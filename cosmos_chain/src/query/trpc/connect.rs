use log::{trace, info};
use tendermint_rpc::HttpClient;

use crate::error::Error;

pub fn tendermint_rpc_client(rpc_addr: &str) -> HttpClient {
    trace!("tendermint rpc connect");

    let client = match HttpClient::new(rpc_addr) {
        Ok(client) => client,
        Err(e) => panic!("tendermint rpc connect error: {:?}", e),
    };

    info!("tendermint rpc connect success");

    client
}