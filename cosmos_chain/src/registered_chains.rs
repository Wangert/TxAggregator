use std::{borrow::Borrow, collections::HashMap};

use anyhow::Chain;
use itertools::Itertools;
use types::ibc_core::ics24_host::identifier::ChainId;

use crate::chain::CosmosChain;

pub struct RegisteredChains {
    chains: HashMap<ChainId, CosmosChain>,
    count: u64,
}

impl RegisteredChains {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            count: 0,
        }
    }

    pub fn add_chain(&mut self, chain: &CosmosChain) {
        let result = self.chains.insert(chain.id(), chain.clone());
        if result.is_none() {
            self.count = self.count + 1;
        }
    }

    pub fn get_chain_by_id(&self, chain_id: &ChainId) -> Option<&CosmosChain> {
        self.chains.get(chain_id)
    }

    pub fn get_all_chain_ids(&self) -> Vec<ChainId> {
        self.chains
            .borrow()
            .into_iter()
            .map(|(k, _)| k.clone())
            .collect_vec()
    }
}
