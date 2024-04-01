use std::sync::Arc;
use http::Uri;
use ibc_proto::{
    cosmos::{
        auth::v1beta1::{
            query_client::QueryClient as AuthQueryClient, BaseAccount, EthAccount,
            QueryAccountRequest,
        },
        staking::v1beta1::{
            query_client::QueryClient as StakingQueryClient, Params as StakingParams,
        }, tx::v1beta1::service_client::ServiceClient as TxServiceClient,
    },
    google::protobuf::Any,
};
use log::{error, info, trace};
use prost::Message;
use tendermint::abci::response::Info;
use tendermint_rpc::{Client, HttpClient};
use tokio::runtime::Runtime;
use tonic::transport::Channel;
use tracing::{info as tracing_info, info_span};
use types::{
    ibc_core::{ics03_connection::version::Version, ics24_host::identifier::ChainId},
    ibc_events::{IbcEvent, IbcEventWithHeight},
};

use crate::{
    account::{self, Secp256k1Account},
    config::{default::max_grpc_decoding_size, load_cosmos_chain_config, CosmosChainConfig},
    error::Error,
    query::{
        grpc::{self, account::query_detail_account},
        trpc,
        types::{Block, BlockResults},
    },
    tx::{batch::batch_messages, send::send_tx, types::Memo},
};

#[derive(Debug, Clone)]
pub struct CosmosChain {
    pub id: ChainId,
    pub config: CosmosChainConfig,
    // grpc_auth_client: Option<AuthQueryClient<Channel>>,
    // grpc_staking_client: Option<StakingQueryClient<Channel>>,
    // tendermint_rpc: Option<HttpClient>,
    pub account: Secp256k1Account,
    pub rt: Arc<Runtime>,
}

impl CosmosChain {
    pub fn new(path: &str) -> Self {
        let config = load_cosmos_chain_config(path);
        let config = match config {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };

        let account = match Secp256k1Account::new(&config.chain_a_key_path, &config.hd_path) {
            Ok(a) => a,
            Err(e) => panic!("New Secp256k1 Account Error: {}", e),
        };

        CosmosChain {
            id: ChainId::from_string(&config.chain_id),
            config: config,
            // grpc_auth_client: None,
            // grpc_staking_client: None,
            // tendermint_rpc: None,
            account,
            rt: Arc::new(Runtime::new().expect("Cosmos chain runtime new error!")),
        }
    }

    pub fn id(&self) -> ChainId {
        self.id.clone()
    }

    pub fn account(&self) -> Secp256k1Account {
        self.account.clone()
    }

    pub fn query_compatible_versions(&self) -> Vec<Version> {
        vec![Version::default()]
    }

    pub fn send_messages_and_wait_commit(
        &self,
        msgs: Vec<Any>,
    ) -> Result<Vec<IbcEventWithHeight>, Error> {
        let rt = self.rt.clone();
        if msgs.is_empty() {
            return Ok(vec![]);
        }

        let mut grpc_query_client = self.grpc_auth_client().clone();

        let chain_config = self.config.clone();
        let key_account = self.account();

        let account_detail = rt.block_on(query_detail_account(
            &mut grpc_query_client,
            key_account.address().as_str(),
        ))?;

        let memo = Memo::new(self.config.memo_prefix.clone()).map_err(Error::memo)?;
        let msg_batches =
            batch_messages(&chain_config, &key_account, &account_detail, &memo, msgs)?;

        let mut ibc_events_with_height = vec![];
        let mut trpc_client = self.tendermint_rpc_client().clone();
        let mut grpc_service_client = self.grpc_tx_sevice_client().clone();
    
        for msg_batch in msg_batches {
            let tx_results = rt.block_on(send_tx(
                &self.config,
                &mut trpc_client,
                &mut grpc_query_client,
                &mut grpc_service_client,
                &key_account,
                &memo,
                &msg_batch,
            ))?;

            ibc_events_with_height.extend(tx_results.events);
        }
        
        Ok(ibc_events_with_height)
    }
    // pub fn tendermint_rpc_connect(&mut self) {
    //     trace!("tendermint rpc connect");
    //     tracing_info!("tendermint rpc connect access");

    //     let client = match HttpClient::new(self.config.tendermint_rpc_addr.as_str()) {
    //         Ok(client) => client,
    //         Err(e) => panic!("tendermint rpc connect error: {:?}", e),
    //     };

    //     self.tendermint_rpc = Some(client);

    //     info!("tendermint rpc connect success");
    // }

    pub fn tendermint_rpc_client(&self) -> HttpClient {
        trpc::connect::tendermint_rpc_client(&self.config.tendermint_rpc_addr)
    }

    pub fn grpc_auth_client(&self) -> AuthQueryClient<Channel> {
        self.rt
            .block_on(grpc::connect::grpc_auth_client(&self.config.grpc_addr))
    }

    pub fn grpc_staking_client(&self) -> StakingQueryClient<Channel> {
        self.rt
            .block_on(grpc::connect::grpc_staking_client(&self.config.grpc_addr))
    }

    pub fn grpc_tx_sevice_client(&self) -> TxServiceClient<Channel> {
        self.rt.block_on(grpc::connect::grpc_tx_service_client(&self.config.grpc_addr))
    }

    pub async fn query_abci_info(&mut self) -> Result<Info, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trpc::abci::abci_info(&mut trpc).await
    }

    // pub async fn grpc_connect(&mut self) {
    //     trace!("grpc connect");
    //     tracing_info!("grpc_connect access");
    //     let grpc_addr = self
    //         .config
    //         .grpc_addr
    //         .as_str()
    //         .parse::<Uri>()
    //         .expect("grpc address parse error!");
    //     let mut auth_client = match AuthQueryClient::connect(grpc_addr.clone()).await {
    //         Ok(client) => client,
    //         Err(e) => panic!("grpc auth client connect error: {:?}", e),
    //     };
    //     let mut staking_client = match StakingQueryClient::connect(grpc_addr).await {
    //         Ok(client) => client,
    //         Err(e) => panic!("grpc staking client connect error: {:?}", e),
    //     };

    //     auth_client =
    //         auth_client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize);
    //     staking_client =
    //         staking_client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize);
    //     self.grpc_auth_client = Some(auth_client);
    //     self.grpc_staking_client = Some(staking_client);

    //     info!("grpc connect success");
    // }

    pub fn query_detail_account_by_address(
        &mut self,
        account_addr: &str,
    ) -> Result<BaseAccount, Error> {
        let mut grpc_client = self.grpc_auth_client();
        trace!("query detail account by address");

        self.rt.block_on(grpc::account::query_detail_account(
            &mut grpc_client,
            account_addr,
        ))
    }

    pub async fn query_all_accounts(&mut self) -> Result<Vec<BaseAccount>, Error> {
        // let span = info_span!("query_all_accounts");
        // let _span = span.enter();

        let mut grpc_client = self.grpc_auth_client();
        trace!("query all accounts");
        tracing_info!("query all accounts access");

        self.rt
            .block_on(grpc::account::query_all_account(&mut grpc_client))
    }

    pub fn query_staking_params(&mut self) -> Result<StakingParams, Error> {
        let mut grpc_client = self.grpc_staking_client();
        trace!("query staking params");

        grpc::staking::query_staking_params(&mut grpc_client)
    }

    pub fn query_latest_block(&mut self) -> Result<Block, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trace!("query latest block");

        trpc::block::latest_block(&mut trpc)
    }

    pub fn query_latest_block_results(&mut self) -> Result<BlockResults, Error> {
        let mut trpc = self.tendermint_rpc_client();
        trace!("query latest block results");

        self.rt
            .block_on(trpc::block::latest_block_results(&mut trpc))
    }
}

#[cfg(test)]
pub mod chain_tests {
    use log::info;

    use super::CosmosChain;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[actix_rt::test]
    pub async fn grpc_connect_works() {
        init();
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);
    }

    #[actix_rt::test]
    pub async fn query_staking_params_works() {
        init();
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);

        let staking_params = cosmos_chain
            .query_staking_params()
            .expect("query_staking_params error!");

        println!("staking params: {:?}", staking_params);
    }
}
