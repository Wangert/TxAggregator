use crate::cmd::cosmos_query_cmd::init_cmd;
use crate::cmd::cosmos_query_cmd::query_cosmos_account;
use crate::commons::CommandCompleter;
use crate::commons::SubCmd;

use clap::arg;
use clap::Arg;
use clap::Command;
use lazy_static::lazy_static;

use std::borrow::Borrow;
use std::path::PathBuf;

use sysinfo::{PidExt, System, SystemExt};

lazy_static! {
    pub static ref CMD: clap::Command<'static> = Command::new("CTXA")
        .about("Cross chain aggregator CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("chain")
                .about("chain operations")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("register")
                        .arg(arg!(-c --config <CONFIG> "chain's configure file path"))
                ),
        )
        .subcommand(
            Command::new("client")
                .about("client operations")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
                        .arg(arg!(-t --target <TARGET_CHAIN_ID>))
                )
        )
        .subcommand(
            Command::new("connection")
                .about("connection operations")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
                        .arg(arg!(-t --target <TARGET_CHAIN_ID>))
                        .arg(arg!(--sourceclient <SOURCE_CLIENT_ID>))
                        .arg(arg!(--targetclient <TARGET_CLIENT_ID>))
                )
        )
        .subcommand(
            Command::new("channel")
                .about("channel operations")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
                        .arg(arg!(-t --target <TARGET_CHAIN_ID>))
                        .arg(arg!(--sourceconn <SOURCE_CONNECTION_ID>))
                        .arg(arg!(--targetconn <TARGET_CONNECTION_ID>))
                )
        )
        .subcommand(
            Command::new("start")
                .about("start aggregator")
                .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
                .arg(arg!(-t --target <TARGET_CHAIN_ID>))
                .arg_required_else_help(true),
        );
    static ref CMD_SUBCMDS: Vec<SubCmd> = subcommands();
}

// lazy_static! {
//     pub static ref CMD: clap::Command<'static> = clap::Command::new("TxAggregator")
//         .version("1.0")
//         .author("Wangert")
//         .about("TxAggregator")
//         // .arg(
//         //     Arg::new("consensus")
//         //         .short('n')
//         //         .long("consensus")
//         //         .help("-n")
//         //         .takes_value(true)
//         //         .multiple_values(true)
//         // )
//         // .arg(
//         //     Arg::new("controller")
//         //         .short('c')
//         //         .long("controller")
//         //         .help("-c")
//         // )
//         .arg(Arg::new("CosmosQuery").short('c').long("cosmosquery").help("-a"))
//         .help_expected(true)
//         .subcommand(init_cmd())
//         .subcommand(query_cosmos_account());
//     static ref CMD_SUBCMDS: Vec<SubCmd> = subcommands();
// }


// 获取全部子命令，用于构建commandcompleter
pub fn all_subcommand(app: &Command, beginlevel: usize, input: &mut Vec<SubCmd>) {
    let nextlevel = beginlevel + 1;
    let mut subcmds = vec![];
    for iterm in app.get_subcommands() {
        subcmds.push(iterm.get_name().to_string());
        if iterm.has_subcommands() {
            all_subcommand(iterm, nextlevel, input);
        } else {
            if beginlevel == 0 {
                all_subcommand(iterm, nextlevel, input);
            }
        }
    }
    let subcommand = SubCmd {
        level: beginlevel,
        command_name: app.get_name().to_string(),
        subcommands: subcmds,
    };
    input.push(subcommand);
}

pub fn get_command_completer() -> CommandCompleter {
    CommandCompleter::new(CMD_SUBCMDS.to_vec())
}

fn subcommands() -> Vec<SubCmd> {
    let mut subcmds = vec![];
    all_subcommand(CMD.clone().borrow(), 0, &mut subcmds);
    subcmds
}

pub fn process_exists(pid: &u32) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    for (syspid, _) in sys.processes() {
        if syspid.as_u32().eq(pid) {
            return true;
        }
    }
    return false;
}

// fn cli() -> Command<'static> {
//     Command::new("CTXA")
//         .about("Cross chain aggregator CLI")
//         .subcommand_required(true)
//         .arg_required_else_help(true)
//         .allow_external_subcommands(true)
//         .subcommand(
//             Command::new("chain")
//                 .about("chain operations")
//                 .arg_required_else_help(true)
//                 .subcommand(
//                     Command::new("register")
//                         .arg(arg!(-c --config <CONFIG> "chain's configure file path")),
//                 ),
//         )
//         .subcommand(
//             Command::new("client")
//                 .about("client operations")
//                 .arg_required_else_help(true)
//                 .subcommand(
//                     Command::new("create")
//                         .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
//                         .arg(arg!(-t --target <TARGET_CHAIN_ID>)),
//                 ),
//         )
//         .subcommand(
//             Command::new("connection")
//                 .about("connection operations")
//                 .arg_required_else_help(true)
//                 .subcommand(
//                     Command::new("create")
//                         .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
//                         .arg(arg!(-t --target <TARGET_CHAIN_ID>))
//                         .arg(arg!(--source-client <SOURCE_CLIENT_ID>))
//                         .arg(arg!(--target-client <TARGET_CLIENT_ID>)),
//                 ),
//         )
//         .subcommand(
//             Command::new("channel")
//                 .about("channel operations")
//                 .arg_required_else_help(true)
//                 .subcommand(
//                     Command::new("create")
//                         .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
//                         .arg(arg!(-t --target <TARGET_CHAIN_ID>))
//                         .arg(arg!(--source-conn <SOURCE_CONNECTION_ID>))
//                         .arg(arg!(--target-conn <TARGET_CONNECTION_ID>)),
//                 ),
//         )
//         .subcommand(
//             Command::new("start")
//                 .about("start aggregator")
//                 .arg(arg!(-s --source <SOURCE_CHAIN_ID>))
//                 .arg(arg!(-t --target <TARGET_CHAIN_ID>))
//                 .arg_required_else_help(true),
//         )
// }