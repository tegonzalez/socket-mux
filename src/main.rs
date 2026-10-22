mod check_pat;
mod cli;
mod header;
mod pattern;
mod reorder;
mod rx;
mod tx;
mod tx_pat;
mod udp;

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

    // check-pat opens no socket and needs no async runtime.
    if let Command::CheckPat(config) = command {
        return match check_pat::run(&config) {
            Ok(code) => ExitCode::from(code),
            Err(error) => {
                error!(target: "socket_mux", "{error:#}");
                ExitCode::FAILURE
            }
        };
    }

    // PROPOSAL SM-D8: run all socket I/O on Tokio's current-thread runtime.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(target: "socket_mux", "could not start Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result: Result<()> = runtime.block_on(async {
        match command {
            Command::Tx(config) => tx::run(config).await,
            Command::TxPat(config) => tx_pat::run(config).await,
            Command::Rx(config) => rx::run(config).await,
            Command::CheckPat(_) => unreachable!("check-pat runs before the runtime"),
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(target: "socket_mux", "{error:#}");
            ExitCode::FAILURE
        }
    }
}
