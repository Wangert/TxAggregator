use crate::chain::CosmosChain;

pub struct Connection {
    pub chain_a: CosmosChain,
    pub chain_b: CosmosChain,
}

impl Connection {
    pub fn new(a_config_path: &str, b_config_path: &str) -> Self {
        Self { chain_a: CosmosChain::new(a_config_path), chain_b: CosmosChain::new(b_config_path) }
    }
}