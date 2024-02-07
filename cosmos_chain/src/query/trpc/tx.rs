use tendermint::Hash;
use tendermint_rpc::{
    endpoint::{tx::Response as TxResponse, tx_search::Response as TxSearchResponse},
    query::Query,
    Client, HttpClient, Order,
};

use crate::error::Error;

pub async fn tx(trpc: &mut HttpClient, hash: Hash, prove: bool) -> Result<TxResponse, Error> {
    trpc.tx(hash, prove)
        .await
        .map_err(|e| Error::trpc("tx".to_string(), e))
}

pub async fn tx_search(
    trpc: &mut HttpClient,
    query: Query,
    prove: bool,
    page: u32,
    per_page: u8,
    order: Order,
) -> Result<TxSearchResponse, Error> {
    trpc.tx_search(query, prove, page, per_page, order)
        .await
        .map_err(|e| Error::trpc("tx_search".to_string(), e))
}

#[cfg(test)]
pub mod tx_tests {
    use tendermint::{Hash, hash::Algorithm};

    use crate::query::trpc::connect::tendermint_rpc_client;

    use super::tx;

    #[test]
    pub fn tx_works() {
        let tx_hash = Hash::from_hex_upper(Algorithm::Sha256, "F2C4A33575AC91B6F52C3B5A32F72B57627164EB72EE68B407963AA0B48A6796").expect("hash error");

        let mut trpc = tendermint_rpc_client("http://10.176.35.57:26657");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let tx_response = rt.block_on(tx(&mut trpc, tx_hash, true));

        println!("tx_response: {:#?}", tx_response);
    }
}
