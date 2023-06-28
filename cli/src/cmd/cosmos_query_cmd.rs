use clap::Command;

// init
pub fn init_cmd() -> Command<'static> {
    clap::Command::new("init")
        .about("init")
}

// query cosmos account
pub fn query_cosmos_account() -> Command<'static> {
    clap::Command::new("queryCosmosAccount").about("queryCosmosAccount")
}