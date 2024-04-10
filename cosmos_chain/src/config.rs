use serde::{Serialize, Deserialize};
use tendermint::block::Height as TmBlockHeight;
use utils::file::toml_file;

use crate::{error::Error, tx::types::GasPrice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosChainConfig {
    pub chain_id: String,
    pub grpc_addr: String,
    pub tendermint_rpc_addr: String,
    pub blockchain_api_addr: String,
    pub rpc_timeout: u64,

    pub chain_key_path: String,
    // pub chain_b_key_path: String,
    pub hd_path: String,
    pub trusted_node: bool,


    pub default_gas: Option<u64>,
    pub max_gas: Option<u64>,
    pub gas_multiplier: Option<f64>,
    pub fee_granter: Option<String>,
    pub gas_price: GasPrice,

    pub clock_drift: u64,
    pub max_block_time: u64,
    pub trusting_period: u64,
    pub trust_threshold: TrustThreshold,

    pub memo_prefix: String,
    pub store_prefix: String,

    pub max_msg_num: u64,
    pub max_tx_size: u64,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustThreshold {
    pub numerator: u64,
    pub denominator: u64,
}

pub fn load_cosmos_chain_config(path: &str) -> Result<CosmosChainConfig, Error> {
    let config: CosmosChainConfig = toml_file::toml_file_read(path).map_err(|e| Error::load_cosmos_chain_conifg(e))?;
    Ok(config)
}

pub mod default {
    use byte_unit::Byte;


    pub fn max_grpc_decoding_size() -> Byte {
        Byte::from_bytes(33554432)
    }
}

#[cfg(test)]
pub mod cosmos_config_test {
    use utils::file::toml_file;

    use super::CosmosChainConfig;

    #[test]
    pub fn read_cosmos_chain_config_works() {
        let file_path = "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";

        let config: CosmosChainConfig = toml_file::toml_file_read(file_path).unwrap();

        println!("{:#?}", config);
    }
}