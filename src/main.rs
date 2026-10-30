mod check_pat;
mod cli;
mod header;
mod pattern;
mod queue;
mod reorder;
mod report;
mod rx;
mod stop;
mod tx;
mod tx_pat;
mod udp;
mod wait;

use std::process::ExitCode;

use anyhow::Result;
use cli::{Action, Command};
use log::error;

fn main() -> ExitCode {
    match cli::parse() {
        Ok(Action::Help) => {
            print!("{}", cli::help());
            ExitCode::SUCCESS
        }
        Ok(Action::Version) => {
            println!("socket-mux {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Action::Run(command)) => run(command),
        Err(error) => {
            eprintln!("socket-mux: {error}\n{}", cli::SHORT_USAGE);
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> ExitCode {
    let log_level = match &command {
        Command::Tx(config) => config.global.log_level,
        Command::TxPat(config) => config.global.log_level,
        Command::CheckPat(config) => config.global.log_level,
        Command::Rx(config) => config.global.log_level,
    };
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_millis()
        .init();

    // check-pat opens no socket.
    if let Command::CheckPat(config) = command {
        return match check_pat::run(&config) {
            Ok(code) => ExitCode::from(code),
            Err(error) => {
                error!(target: "socket_mux", "{error:#}");
                ExitCode::FAILURE
            }
        };
    }

    // TX and RX run on plain OS threads with blocking sockets; no async runtime.
    let stop = stop::Stop::new();
    let result: Result<()> = match command {
        Command::Tx(config) => tx::run(config),
        Command::TxPat(config) => tx_pat::run(config),
        Command::Rx(config) => rx::run(config, stop),
        Command::CheckPat(_) => unreachable!("check-pat returned above"),
    };
    exit_code(result)
}

fn exit_code(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(target: "socket_mux", "{error:#}");
            ExitCode::FAILURE
        }
    }
}
