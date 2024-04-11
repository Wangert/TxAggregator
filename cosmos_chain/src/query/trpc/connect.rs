use log::{trace, info};
use tendermint_rpc::{HttpClient, Client};

pub fn tendermint_rpc_client(rpc_addr: &str) -> HttpClient {
    trace!("tendermint rpc connect");

    let client = match HttpClient::new(rpc_addr) {
        Ok(client) => client,
        Err(e) => panic!("tendermint rpc connect error: {:?}", e),
    };

    println!("tendermint rpc connect success");
    info!("tendermint rpc connect success");

    client
}