use clap::{ArgMatches, Command};

use crate::cmd::rootcmd::CMD;

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
                    let config = sub_matches.get_one::<String>("config");
                    println!();
                    println!("Chain Register:");
                    println!(
                        "Chain_Configure_File_Path({:?})",
                        config
                    );
                }

                _ => unreachable!(),
            }
        }
        Some(("client", sub_matches)) => {
            let client_command = sub_matches.subcommand().unwrap();
            match client_command {
                ("create", sub_matches) => {
                    let source_chain = sub_matches.get_one::<String>("source");
                    let target_chain = sub_matches.get_one::<String>("target");
                    println!();
                    println!("Client Create:");
                    println!(
                        "Source_Chain({:?}) -- Target_Chain({:?})",
                        source_chain, target_chain
                    );
                }

                _ => unreachable!(),
            }
        }
        Some(("connection", sub_matches)) => {
            let connection_command = sub_matches.subcommand().unwrap();
            match connection_command {
                ("create", sub_matches) => {
                    let source_chain = sub_matches.get_one::<String>("source");
                    let target_chain = sub_matches.get_one::<String>("target");
                    let source_client = sub_matches.get_one::<String>("sourceclient");
                    let target_client = sub_matches.get_one::<String>("targetclient");
                    println!();
                    println!("Connection Create:");
                    println!(
                        "Source_Chain({:?}) -- Target_Chain({:?})",
                        source_chain, target_chain
                    );
                    println!(
                        "Source_Client({:?}) -- Target_Client({:?})",
                        source_client, target_client
                    );
                }

                _ => unreachable!(),
            }
        }
        Some(("channel", sub_matches)) => {
            let channel_command = sub_matches.subcommand().unwrap();
            match channel_command {
                ("create", sub_matches) => {
                    let source_chain = sub_matches.get_one::<String>("source");
                    let target_chain = sub_matches.get_one::<String>("target");
                    let source_conn = sub_matches.get_one::<String>("sourceconn");
                    let target_conn = sub_matches.get_one::<String>("targetconn");
                    println!();
                    println!("Connection Create:");
                    println!(
                        "Source_Chain({:?}) -- Target_Chain({:?})",
                        source_chain, target_chain
                    );
                    println!(
                        "Source_Connection({:?}) -- Target_Connection({:?})",
                        source_conn, target_conn
                    );
                }

                _ => unreachable!(),
            }
        }
        Some(("start", sub_matches)) => {
            let source_chain = sub_matches.get_one::<String>("source");
            let target_chain = sub_matches.get_one::<String>("target");
            println!();
            println!("Channel Create:");
            println!(
                "Source_Chain({:?}) -- Target_Chain({:?})",
                source_chain, target_chain
            );
        }
        _ => unreachable!(),
    }
}
