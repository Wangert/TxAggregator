use clap::{ArgMatches, Command};
use cli::cmd::rootcmd::CMD;

pub fn before_cmd_match(args: Vec<String>) {
    match Command::try_get_matches_from(CMD.to_owned(), args.clone()) {
        Ok(matches) => {
            cmd_match(&matches);
        }
        Err(err) => {
            err.print().expect("Error writing Error");
        }
    };
}

pub fn cmd_match(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("chain", sub_matches)) => {
            let chain_command = sub_matches.subcommand().unwrap();
            match chain_command {
                ("register", sub_matches) => {
                    let config = sub_matches.get_one::<String>("CONFIG");
                    println!("Chain register: {:?}", config);
                }

                _ => unreachable!(),
            }
        }

        _ => unreachable!(),
    }
}