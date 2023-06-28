use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosChainConfig {
    pub grpc_addr: String,
    pub tendermint_rpc_addr: String
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