// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {

//     Ok(())
// }

use MosaicXC::supervisor::Supervisor;
use std::error::Error;

use cli::{client::Client, cmd::rootcmd::CMD};

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // env_logger::init();

    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::INFO)
    //     .init();

    let cmd = CMD.clone();

    let cmd_matches = cmd.get_matches();

    let mut client = Client::new(cmd_matches);

    let (args_sender, mut args_receiver) = mpsc::channel::<Vec<String>>(10);
    client.run(args_sender).await;

    let mut supervisor = Supervisor::new();

    loop {
        tokio::select! {
            Some(args) = args_receiver.recv() => {

                // println!("TASK2: {:?}", &args);
                supervisor.cmd_matches(args).await?;

            }
        }
    }

    Ok(())
}
