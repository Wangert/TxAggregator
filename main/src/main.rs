// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {

//     Ok(())
// }

use http::Uri;
use std::{error::Error, str::FromStr, time::Duration};
use types::ibc_core::ics24_host::identifier::ClientId;

use cli::{client::Client, cmd::rootcmd::CMD, cmd_matches::before_cmd_match};
use cosmos_chain::{
    account::Secp256k1Account,
    chain::CosmosChain,
    connection::{Connection, ConnectionSide},
    query::grpc::account::query_detail_account,
};
use log::info;
use tokio::{sync::mpsc, time::sleep};
use tracing::{info_span, metadata::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let grpc_addr = "http://0.0.0.0:9090".parse::<Uri>().unwrap();
    // let account_addr = "cosmos1vpj4s5hsngjprp1ft5hvuqj5v7dvnxjnsn5n4z";
    // let base_account = query_detail_account(&grpc_addr, account_addr).await?;

    // println!("{:?}", base_account);

    // env_logger::init();

    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::INFO)
    //     .init();

    // println!("&&&&&&&&&&&&&");
    let cmd = CMD.clone();
    // println!("&&&&&&&&&&&&&");

    let cmd_matches = cmd.get_matches();
    // println!("&&&&&&&&&&&&&");

    let mut client = Client::new(cmd_matches);

    // println!("!!!!!!!!!!!!!!!");
    let (args_sender, mut args_receiver) = mpsc::channel::<Vec<String>>(10);
    client.run(args_sender).await;

    loop {
        tokio::select! {
            Some(args) = args_receiver.recv() => {

                println!("TASK2: {:?}", &args);
                before_cmd_match(args);

            }
        }
        // let args_option = args_receiver.recv().await;
        // println!("TASK2: {:?}", args_option);

        // if let Some(args) = args_option {
        //     before_cmd_match(args);
        // }
    }

    // sleep(Duration::from_secs(20)).await;
    // let a_file_path =
    //     "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
    // let b_file_path =
    //     "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

    // let cosmos_chain_a = CosmosChain::new(a_file_path);
    // let cosmos_chain_b = CosmosChain::new(b_file_path);

    // let connection = Connection::new(
    //     ConnectionSide::new(
    //         cosmos_chain_a,
    //         ClientId::from_str("07-tendermint-7").unwrap(),
    //     ),
    //     ConnectionSide::new(
    //         cosmos_chain_b,
    //         ClientId::from_str("07-tendermint-1").unwrap(),
    //     ),
    //     Duration::from_secs(100),
    // );

    // let result = connection.build_connection_open_init_and_send();
    // match result {
    //     Ok(events) => println!("Event: {:?}", events),
    //     Err(e) => panic!("{}", e),
    // }

    Ok(())
}
