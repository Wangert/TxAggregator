use http::Uri;
use ibc_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient, BaseAccount, EthAccount, QueryAccountRequest,
};
use log::{error, info, trace};
use prost::Message;
use tendermint::abci::response::Info;
use tendermint_rpc::{Client, HttpClient};
use tonic::transport::Channel;
use tracing::{info as tracing_info, info_span};

use crate::{
    config::{default::max_grpc_decoding_size, load_cosmos_chain_config, CosmosChainConfig},
    error::Error,
    query::{grpc::{self, account::query_detail_account}, types::{Block, BlockResults}},
    query::trpc,
};

pub struct CosmosChain {
    pub config: CosmosChainConfig,
    grpc_client: Option<QueryClient<Channel>>,
    tendermint_rpc: Option<HttpClient>,
}

impl CosmosChain {
    pub fn new(path: &str) -> Self {
        let config = load_cosmos_chain_config(path);
        let config = match config {
            Ok(c) => c,
            Err(e) => panic!("{}", e),
        };

        CosmosChain {
            config: config,
            grpc_client: None,
            tendermint_rpc: None,
        }
    }

    pub fn tendermint_rpc_client(&mut self) -> Option<&mut HttpClient> {
        self.tendermint_rpc.as_mut()
    }

    pub fn grpc_client(&mut self) -> Option<&mut QueryClient<Channel>> {
        self.grpc_client.as_mut()
    }

    pub fn tendermint_rpc_connect(&mut self) {
        trace!("tendermint rpc connect");
        tracing_info!("tendermint rpc connect access");

        let client = match HttpClient::new(self.config.tendermint_rpc_addr.as_str()) {
            Ok(client) => client,
            Err(e) => panic!("tendermint rpc connect error: {:?}", e),
        };

        self.tendermint_rpc = Some(client);

        info!("tendermint rpc connect success");
    }

    pub async fn query_abci_info(&mut self) -> Result<Info, Error> {
        let trpc = self
            .tendermint_rpc_client()
            .ok_or_else(Error::empty_tendermint_rpc_client)?;
        trpc::abci::abci_info(trpc).await
    }

    pub async fn grpc_connect(&mut self) {
        trace!("grpc connect");
        tracing_info!("grpc_connect access");
        let grpc_addr = self
            .config
            .grpc_addr
            .as_str()
            .parse::<Uri>()
            .expect("grpc address parse error!");
        let mut client = match QueryClient::connect(grpc_addr).await {
            Ok(client) => client,
            Err(e) => panic!("grpc connect error: {:?}", e),
        };

        client = client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize);
        self.grpc_client = Some(client);

        info!("grpc connect success");
    }

    pub async fn query_detail_account_by_address(
        &mut self,
        account_addr: &str,
    ) -> Result<BaseAccount, Error> {
        let grpc_client = self.grpc_client().ok_or_else(Error::empty_grpc_client)?;
        trace!("query detail account by address");

        grpc::account::query_detail_account(grpc_client, account_addr).await
    }

    pub async fn query_all_accounts(&mut self) -> Result<Vec<BaseAccount>, Error> {
        // let span = info_span!("query_all_accounts");
        // let _span = span.enter();

        let grpc_client = self.grpc_client().ok_or_else(Error::empty_grpc_client)?;
        trace!("query all accounts");
        tracing_info!("query all accounts access");

        grpc::account::query_all_account(grpc_client).await
    }

    pub async fn query_latest_block(&mut self) -> Result<Block, Error> {
        let trpc = self.tendermint_rpc_client().ok_or_else(Error::empty_tendermint_rpc_client)?;
        trace!("query latest block");

        trpc::block::latest_block(trpc).await
    }

    pub async fn query_latest_block_results(&mut self) -> Result<BlockResults, Error> {
        let trpc = self.tendermint_rpc_client().ok_or_else(Error::empty_tendermint_rpc_client)?;
        trace!("query latest block results");

        trpc::block::latest_block_results(trpc).await
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

        cosmos_chain.grpc_connect().await;
    }
}
