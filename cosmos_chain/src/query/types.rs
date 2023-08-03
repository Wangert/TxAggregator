use tendermint::{
    abci::{response::DeliverTx, Event},
    block::{Commit, Header, Height, Id},
    consensus::Params,
    evidence::List,
    node::Info as NodeInfo,
    validator::{Info as ValidatorInfo, Update},
};
use tendermint_rpc::endpoint::{
    block as trpc_block, block_results, status as trpc_status, status::SyncInfo, header as trpc_header,
};

#[derive(Debug, Clone)]
pub struct Block {
    pub id: Id,
    pub header: Header,
    pub data: Vec<Vec<u8>>,
    pub evidence: List,
    pub last_commit: Option<Commit>,
}

impl From<trpc_block::Response> for Block {
    fn from(value: trpc_block::Response) -> Self {
        Self {
            id: value.block_id,
            header: value.block.header,
            data: value.block.data,
            evidence: value.block.evidence,
            last_commit: value.block.last_commit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockResults {
    pub height: Height,
    pub txs_results: Option<Vec<DeliverTx>>,
    pub begin_block_events: Option<Vec<Event>>,
    pub end_block_events: Option<Vec<Event>>,
    pub validator_update: Vec<Update>,
    pub consensus_param_updates: Option<Params>,
}

impl From<block_results::Response> for BlockResults {
    fn from(value: block_results::Response) -> Self {
        Self {
            height: value.height,
            txs_results: value.txs_results,
            begin_block_events: value.begin_block_events,
            end_block_events: value.end_block_events,
            validator_update: value.validator_updates,
            consensus_param_updates: value.consensus_param_updates,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TendermintStatus {
    pub node_info: NodeInfo,
    pub sync_info: SyncInfo,
    pub validator_info: ValidatorInfo,
}

impl From<trpc_status::Response> for TendermintStatus {
    fn from(value: trpc_status::Response) -> Self {
        Self {
            node_info: value.node_info,
            sync_info: value.sync_info,
            validator_info: value.validator_info,
        }
    }
}


#[derive(Debug, Clone)]
pub struct HeaderResult {
    pub header: Header,
}

impl From<trpc_header::Response> for HeaderResult {
    fn from(value: trpc_header::Response) -> Self {
        Self { header: value.header }
    }
}
