// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
    
//     Ok(())
// }

use std::error::Error;
use http::Uri;

use cli::{client::Client, cmd::rootcmd::CMD};
use cosmos_chain::{query::grpc::account::query_detail_account, chain::CosmosChain};
use log::info;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // let cmd_matches = CMD.clone().get_matches();
    // let mut client = Client::new(cmd_matches);

    
    // let (args_sender, _args_recevier) = mpsc::channel::<Vec<String>>(10);
    // client.run(args_sender);

    // let grpc_addr = "http://0.0.0.0:9090".parse::<Uri>().unwrap();
    // let account_addr = "cosmos1vpj4s5hsngjprp1ft5hvuqj5v7dvnxjnsn5n4z";
    // let base_account = query_detail_account(&grpc_addr, account_addr).await?;

    // println!("{:?}", base_account);

    env_logger::init();
    
    let file_path = "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
    let mut cosmos_chain = CosmosChain::new(file_path);

    cosmos_chain.grpc_connect().await;
    cosmos_chain.tendermint_rpc_connect();

    // let account_adrr = "cosmos1w4e4v6rk8mmj49yzadwslvg6fs968uz4qvssfq";
    // let base_account = cosmos_chain.query_detail_account_by_address(account_adrr).await?;

    // info!("Query detail account info: {:?}", base_account);

    let accounts = cosmos_chain.query_all_accounts().await?;
    info!("Query all accounts: {:?}", accounts);

    let abci_info = cosmos_chain.query_abci_info().await?;

    info!("Query abci info: {:?}", abci_info);

    Ok(())
}
