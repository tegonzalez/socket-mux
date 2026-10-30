mod check_pat;
mod cli;
mod header;
mod mtu;
mod pattern;
mod queue;
mod reorder;
mod report;
mod rx;
mod signals;
mod stop;
mod tx;
mod tx_pat;
mod udp;
mod wait;

use std::process::ExitCode;

use anyhow::Result;
use cli::{Action, Command};
use log::{debug, error, warn};

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

    // TX and RX pick the datagram size: --datagram-size, or the adapter MTU
    // less 48. It is logged once; a size that gives an invalid W is a usage
    // error.
    let mut command = command;
    let resolved = match &mut command {
        Command::Tx(config) => {
            mtu::resolve(&mut config.global, mtu::Path::Destination(&config.remote))
        }
        Command::TxPat(config) => {
            mtu::resolve(&mut config.global, mtu::Path::Destination(&config.remote))
        }
        Command::Rx(config) => mtu::resolve(&mut config.global, mtu::Path::Bind(config.bind)),
        Command::CheckPat(_) => unreachable!("check-pat returned above"),
    };
    if let Err(error) = resolved {
        eprintln!("socket-mux: {error}\n{}", cli::SHORT_USAGE);
        return ExitCode::from(2);
    }

    // TX and RX run on plain OS threads with blocking sockets; no async runtime.
    // SIGINT/SIGTERM (Ctrl-C on Windows) set `stop`; the run winds down, the
    // reporter prints its summary, and the exit status is 128 + signal.
    let stop = stop::Stop::new();
    if let Err(error) = signals::install(stop.clone()) {
        warn!(target: "socket_mux", "{error:#}; signals use the default disposition");
    }
    let result: Result<()> = match command {
        Command::Tx(config) => tx::run(config, stop),
        Command::TxPat(config) => tx_pat::run(config, stop),
        Command::Rx(config) => rx::run(config, stop),
        Command::CheckPat(_) => unreachable!("check-pat returned above"),
    };
    if let Some(status) = signals::exit_status() {
        // Errors caused by the stop itself (a shut socket) are not failures.
        if let Err(error) = result {
            debug!(target: "socket_mux", "after stop: {error:#}");
        }
        return ExitCode::from(status);
    }
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
