use crate::cmd::cosmos_query_cmd::init_cmd;
use crate::cmd::cosmos_query_cmd::query_cosmos_account;
use crate::commons::CommandCompleter;
use crate::commons::SubCmd;

use clap::Arg;
use clap::Command as clap_Command;
use lazy_static::lazy_static;

use std::borrow::Borrow;

use sysinfo::{PidExt, System, SystemExt};

lazy_static! {
    pub static ref CMD: clap::Command<'static> = clap::Command::new("TxAggregator")
        .version("1.0")
        .author("Wangert")
        .about("TxAggregator")
        // .arg(
        //     Arg::new("consensus")
        //         .short('n')
        //         .long("consensus")
        //         .help("-n")
        //         .takes_value(true)
        //         .multiple_values(true)
        // )
        // .arg(
        //     Arg::new("controller")
        //         .short('c')
        //         .long("controller")
        //         .help("-c")
        // )
        .arg(Arg::new("CosmosQuery").short('c').long("cosmosquery").help("-a"))
        .help_expected(true)
        .subcommand(init_cmd())
        .subcommand(query_cosmos_account());
    static ref CMD_SUBCMDS: Vec<SubCmd> = subcommands();
}

// 获取全部子命令，用于构建commandcompleter
pub fn all_subcommand(app: &clap_Command, beginlevel: usize, input: &mut Vec<SubCmd>) {
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
