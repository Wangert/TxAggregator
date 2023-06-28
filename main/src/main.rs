// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
    
//     Ok(())
// }

use std::error::Error;
use http::Uri;

use cli::{client::Client, cmd::rootcmd::CMD};
use cosmos_chain::query::account::query_detail_account;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let cmd_matches = CMD.clone().get_matches();
    let mut client = Client::new(cmd_matches);

    
    let (args_sender, _args_recevier) = mpsc::channel::<Vec<String>>(10);
    client.run(args_sender);

    // let grpc_addr = "http://0.0.0.0:9090".parse::<Uri>().unwrap();
    // let account_addr = "cosmos1vpj4s5hsngjprp1ft5hvuqj5v7dvnxjnsn5n4z";
    // let base_account = query_detail_account(&grpc_addr, account_addr).await?;

    // println!("{:?}", base_account);

    Ok(())
}
