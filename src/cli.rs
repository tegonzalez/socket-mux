use std::ffi::{OsStr, OsString};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use lexopt::{Arg, Parser};
use log::LevelFilter;

const DEFAULT_STREAMS: u64 = 4; // PROPOSAL SM-D12: default stream count.
const DEFAULT_BASE_PORT: u64 = 31000; // PROPOSAL SM-D12: default UDP base port.
const DEFAULT_LISTEN: &str = "127.0.0.1:31100"; // PROPOSAL SM-D12: default TX listen address.
const DEFAULT_REMOTE: &str = "127.0.0.1"; // PROPOSAL SM-D12: default TX remote address.
const DEFAULT_BIND: &str = "127.0.0.1"; // PROPOSAL SM-D12: default RX bind address.
const DEFAULT_CONNECT: &str = "127.0.0.1:31200"; // PROPOSAL SM-D12: default RX connect address.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10); // SM-D19: default 10 s.

pub const SHORT_USAGE: &str = "Usage: socket-mux [GLOBAL OPTIONS] \
     <tx|tx-pat|check-pat|rx> [GLOBAL OPTIONS | COMMAND OPTIONS]";

const LONG_HELP: &str = "\
socket-mux carries one TCP byte stream over N lossy UDP streams.

Usage: socket-mux [GLOBAL OPTIONS] <tx|tx-pat|check-pat|rx> [GLOBAL OPTIONS | COMMAND OPTIONS]
       socket-mux --help | --version

Commands:
  tx                         Read one input and transmit it over UDP
  tx-pat                     Transmit the generated pattern (never ends on its own)
  check-pat                  Validate a pattern stream read from standard input
  rx                         Receive UDP streams and deliver them over TCP

Global options (before or after the command):
  -n, --streams N            Number of UDP streams (minimum 3, default 4)
  -p, --base-port PORT       First UDP port (default 31000)
      --udp-buffer BYTES     UDP send (TX) or receive (RX) buffer size
      --timeout SECONDS      RX ends the stream after this silence (default 10)
      --log-level LEVEL      error, warn, info, debug, or trace (default info)
  -v, --verbose              Raise the log level; repeatable
  -q, --quiet                Lower the log level; repeatable
  -h, --help                 Print this help
  -V, --version              Print the version

TX options (after tx):
  -l, --listen ADDR:PORT     Loopback TCP listen address (default 127.0.0.1:31100)
  -r, --remote HOST          Remote host name or IP (default 127.0.0.1)
  INPUT                      '-' reads standard input; any other value names a file
                             (absent: accept one TCP connection at --listen)

TX-PAT options (after tx-pat):
  -r, --remote HOST          Remote host name or IP (default 127.0.0.1)
  -s, --seed U64             Fixed pattern seed (default: random, logged at info)

CHECK-PAT options (after check-pat):
  -s, --seed U64             Also regenerate and compare every block body

RX options (after rx):
  -b, --bind IP              UDP bind IP (default 127.0.0.1)
  -c, --connect ADDR:PORT    TCP output address (default 127.0.0.1:31200)
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Verbosity {
    Verbose,
    Quiet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Global {
    pub streams: usize,
    pub base_port: u16,
    pub udp_buffer: Option<usize>,
    pub timeout: Duration,
    pub log_level: LevelFilter,
}

// PROPOSAL SM-D16: the tx INPUT operand; "-" is standard input, any other value a file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Input {
    Stdin,
    File(OsString),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxConfig {
    pub global: Global,
    pub listen: SocketAddr,
    pub remote: String,
    pub input: Option<Input>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxPatConfig {
    pub global: Global,
    pub remote: String,
    pub seed: Option<u64>,
}

// PROPOSAL for OD-20: check-pat is a fourth subcommand; the spec grammar lists only
// tx, tx-pat and rx, so check-pat is not in the spec grammar yet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckPatConfig {
    pub global: Global,
    pub seed: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RxConfig {
    pub global: Global,
    pub bind: IpAddr,
    pub connect: SocketAddr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Tx(TxConfig),
    TxPat(TxPatConfig),
    CheckPat(CheckPatConfig),
    Rx(RxConfig),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Help,
    Version,
    Run(Command),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageError {
    message: String,
}

impl UsageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl From<lexopt::Error> for UsageError {
    fn from(error: lexopt::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(Clone, Debug)]
struct RawGlobal {
    streams: u64,
    base_port: u64,
    udp_buffer: Option<usize>,
    timeout: Duration,
    log_level: LevelFilter,
    verbosity: Vec<Verbosity>,
}

impl Default for RawGlobal {
    fn default() -> Self {
        Self {
            streams: DEFAULT_STREAMS,
            base_port: DEFAULT_BASE_PORT,
            udp_buffer: None,
            timeout: DEFAULT_TIMEOUT,
            log_level: LevelFilter::Info, // PROPOSAL SM-D15: default log level.
            verbosity: Vec::new(),
        }
    }
}

impl RawGlobal {
    fn finish(self) -> Result<Global, UsageError> {
        if self.streams < 3 {
            return Err(UsageError::new("--streams must be at least 3"));
        }
        if !(1..=65535).contains(&self.base_port) {
            return Err(UsageError::new("--base-port must be in 1..65535"));
        }
        if self.streams - 1 > 65535 - self.base_port {
            return Err(UsageError::new(
                "base port plus stream count exceeds port 65535",
            ));
        }
        Ok(Global {
            streams: self.streams as usize,
            base_port: self.base_port as u16,
            udp_buffer: self.udp_buffer,
            timeout: self.timeout,
            log_level: apply_verbosity(self.log_level, &self.verbosity),
        })
    }
}

pub fn parse() -> Result<Action, UsageError> {
    parse_from(std::env::args_os().skip(1))
}

pub fn parse_from<I, T>(args: I) -> Result<Action, UsageError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if args.iter().any(|arg| arg == OsStr::new("--help")) || has_short_option(&args, 'h') {
        return Ok(Action::Help);
    }
    if args.iter().any(|arg| arg == OsStr::new("--version")) || has_short_option(&args, 'V') {
        return Ok(Action::Version);
    }

    let mut parser = Parser::from_iter(std::iter::once(OsString::from("socket-mux")).chain(args));
    let mut global = RawGlobal::default();
    let command = loop {
        let next = parser.next()?;
        match next {
            Some(Arg::Value(value)) => match value.to_str() {
                Some("tx") => break CommandName::Tx,
                Some("tx-pat") => break CommandName::TxPat,
                Some("check-pat") => break CommandName::CheckPat,
                Some("rx") => break CommandName::Rx,
                _ => {
                    return Err(UsageError::new(format!(
                        "unknown subcommand: {}",
                        value.to_string_lossy()
                    )))
                }
            },
            Some(arg) => {
                let option = global_option(&arg);
                if let Some(option) = option {
                    parse_global(option, &mut parser, &mut global)?;
                    continue;
                }
                match arg {
                    Arg::Short('l' | 'r' | 'b' | 'c' | 's')
                    | Arg::Long("listen" | "remote" | "bind" | "connect" | "seed") => {
                        return Err(UsageError::new(
                            "command options must appear after the subcommand",
                        ));
                    }
                    _ => return Err(UsageError::new(format!("unexpected option: {arg:?}"))),
                }
            }
            None => return Err(UsageError::new("missing subcommand")),
        }
    };

    let command = match command {
        CommandName::Tx => Command::Tx(parse_tx(&mut parser, global)?),
        CommandName::TxPat => Command::TxPat(parse_tx_pat(&mut parser, global)?),
        CommandName::CheckPat => Command::CheckPat(parse_check_pat(&mut parser, global)?),
        CommandName::Rx => Command::Rx(parse_rx(&mut parser, global)?),
    };
    Ok(Action::Run(command))
}

fn has_short_option(args: &[OsString], wanted: char) -> bool {
    args.iter().any(|arg| {
        let Some(arg) = arg.to_str() else {
            return false;
        };
        let Some(cluster) = arg.strip_prefix('-') else {
            return false;
        };
        if cluster.starts_with('-') {
            return false;
        }
        for option in cluster.chars() {
            if option == wanted {
                return true;
            }
            // These options consume the rest of their cluster as an attached value.
            if matches!(option, 'n' | 'p' | 'l' | 'r' | 'b' | 'c' | 's') {
                break;
            }
        }
        false
    })
}

#[derive(Clone, Copy)]
enum CommandName {
    Tx,
    TxPat,
    CheckPat,
    Rx,
}

fn parse_tx(parser: &mut Parser, mut global: RawGlobal) -> Result<TxConfig, UsageError> {
    let mut listen = parse_socket_addr(OsString::from(DEFAULT_LISTEN), "--listen")?;
    let mut listen_given = false;
    let mut remote = DEFAULT_REMOTE.to_owned();
    let mut input = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Arg::Short('l') | Arg::Long("listen") => {
                listen = parse_socket_addr(parser.value()?, "--listen")?;
                listen_given = true;
            }
            Arg::Short('r') | Arg::Long("remote") => {
                remote = nonempty_value(parser.value()?, "--remote")?;
            }
            Arg::Short('s') | Arg::Long("seed") => {
                return Err(UsageError::new("--seed cannot be used with tx"));
            }
            Arg::Short('b' | 'c') | Arg::Long("bind" | "connect") => {
                return Err(UsageError::new("RX options cannot be used with tx"));
            }
            Arg::Value(value) => {
                if input.is_some() {
                    return Err(UsageError::new("tx takes at most one INPUT operand"));
                }
                input = Some(if value == OsStr::new("-") {
                    Input::Stdin
                } else {
                    Input::File(value)
                });
            }
            _ => {
                let option = global_option(&arg);
                if let Some(option) = option {
                    parse_global(option, parser, &mut global)?;
                } else {
                    return Err(unexpected_after_subcommand(arg));
                }
            }
        }
    }
    if listen_given && input.is_some() {
        return Err(UsageError::new(
            "--listen cannot be used together with an INPUT operand",
        ));
    }
    if !listen.ip().is_loopback() {
        return Err(UsageError::new("--listen must be a loopback address"));
    }
    if listen.port() == 0 {
        return Err(UsageError::new("--listen port must be in 1..65535"));
    }
    Ok(TxConfig {
        global: global.finish()?,
        listen,
        remote,
        input,
    })
}

fn parse_tx_pat(parser: &mut Parser, mut global: RawGlobal) -> Result<TxPatConfig, UsageError> {
    let mut remote = DEFAULT_REMOTE.to_owned();
    let mut seed = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Arg::Short('r') | Arg::Long("remote") => {
                remote = nonempty_value(parser.value()?, "--remote")?;
            }
            Arg::Short('s') | Arg::Long("seed") => {
                seed = Some(parse_seed(parser.value()?)?);
            }
            Arg::Short('l') | Arg::Long("listen") => {
                return Err(UsageError::new("--listen cannot be used with tx-pat"));
            }
            Arg::Short('b' | 'c') | Arg::Long("bind" | "connect") => {
                return Err(UsageError::new("RX options cannot be used with tx-pat"));
            }
            Arg::Value(value) => {
                return Err(UsageError::new(format!(
                    "tx-pat takes no INPUT operand: {}",
                    value.to_string_lossy()
                )));
            }
            _ => {
                let option = global_option(&arg);
                if let Some(option) = option {
                    parse_global(option, parser, &mut global)?;
                } else {
                    return Err(unexpected_after_subcommand(arg));
                }
            }
        }
    }
    Ok(TxPatConfig {
        global: global.finish()?,
        remote,
        seed,
    })
}

fn parse_check_pat(
    parser: &mut Parser,
    mut global: RawGlobal,
) -> Result<CheckPatConfig, UsageError> {
    let mut seed = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Arg::Short('s') | Arg::Long("seed") => {
                seed = Some(parse_seed(parser.value()?)?);
            }
            Arg::Short('l' | 'r' | 'b' | 'c')
            | Arg::Long("listen" | "remote" | "bind" | "connect") => {
                return Err(UsageError::new(
                    "socket options cannot be used with check-pat",
                ));
            }
            Arg::Value(value) => {
                return Err(UsageError::new(format!(
                    "check-pat takes no operand: {}",
                    value.to_string_lossy()
                )));
            }
            _ => {
                let option = global_option(&arg);
                if let Some(option) = option {
                    parse_global(option, parser, &mut global)?;
                } else {
                    return Err(unexpected_after_subcommand(arg));
                }
            }
        }
    }
    Ok(CheckPatConfig {
        global: global.finish()?,
        seed,
    })
}

fn parse_seed(value: OsString) -> Result<u64, UsageError> {
    // PROPOSAL SM-D17: the seed is an unsigned 64-bit decimal integer.
    value
        .into_string()
        .map_err(|_| UsageError::new("--seed requires an unsigned 64-bit decimal integer"))?
        .parse::<u64>()
        .map_err(|_| UsageError::new("--seed requires an unsigned 64-bit decimal integer"))
}

fn parse_rx(parser: &mut Parser, mut global: RawGlobal) -> Result<RxConfig, UsageError> {
    let mut bind = DEFAULT_BIND
        .parse::<IpAddr>()
        .map_err(|_| UsageError::new("invalid default --bind address"))?;
    let mut connect = parse_socket_addr(OsString::from(DEFAULT_CONNECT), "--connect")?;
    while let Some(arg) = parser.next()? {
        match arg {
            Arg::Short('b') | Arg::Long("bind") => {
                bind = parser
                    .value()?
                    .into_string()
                    .map_err(|_| UsageError::new("--bind must be valid UTF-8"))?
                    .parse()
                    .map_err(|_| UsageError::new("--bind must be an IP address"))?;
            }
            Arg::Short('c') | Arg::Long("connect") => {
                connect = parse_socket_addr(parser.value()?, "--connect")?;
            }
            Arg::Short('s') | Arg::Long("seed") => {
                return Err(UsageError::new("--seed cannot be used with rx"));
            }
            Arg::Short('l' | 'r') | Arg::Long("listen" | "remote") => {
                return Err(UsageError::new("TX options cannot be used with rx"));
            }
            _ => {
                let option = global_option(&arg);
                if let Some(option) = option {
                    parse_global(option, parser, &mut global)?;
                } else {
                    return Err(unexpected_after_subcommand(arg));
                }
            }
        }
    }
    if connect.port() == 0 {
        return Err(UsageError::new("--connect port must be in 1..65535"));
    }
    Ok(RxConfig {
        global: global.finish()?,
        bind,
        connect,
    })
}

#[derive(Clone, Copy)]
enum GlobalOption {
    Streams,
    BasePort,
    UdpBuffer,
    Timeout,
    LogLevel,
    Verbose,
    Quiet,
}

fn global_option(arg: &Arg) -> Option<GlobalOption> {
    match arg {
        Arg::Short('n') | Arg::Long("streams") => Some(GlobalOption::Streams),
        Arg::Short('p') | Arg::Long("base-port") => Some(GlobalOption::BasePort),
        Arg::Long("udp-buffer") => Some(GlobalOption::UdpBuffer),
        Arg::Long("timeout") => Some(GlobalOption::Timeout),
        Arg::Long("log-level") => Some(GlobalOption::LogLevel),
        Arg::Short('v') | Arg::Long("verbose") => Some(GlobalOption::Verbose),
        Arg::Short('q') | Arg::Long("quiet") => Some(GlobalOption::Quiet),
        _ => None,
    }
}

fn parse_global(
    option: GlobalOption,
    parser: &mut Parser,
    global: &mut RawGlobal,
) -> Result<(), UsageError> {
    match option {
        GlobalOption::Streams => {
            global.streams = parse_integer(parser.value()?, "--streams")?;
        }
        GlobalOption::BasePort => {
            global.base_port = parse_integer(parser.value()?, "--base-port")?;
        }
        GlobalOption::UdpBuffer => {
            let size = parse_integer(parser.value()?, "--udp-buffer")?;
            if size == 0 || size > usize::MAX as u64 {
                return Err(UsageError::new("--udp-buffer must be at least 1"));
            }
            global.udp_buffer = Some(size as usize);
        }
        GlobalOption::Timeout => {
            global.timeout = parse_timeout(parser.value()?)?;
        }
        GlobalOption::LogLevel => {
            global.log_level = parse_log_level(&parser.value()?)?;
        }
        GlobalOption::Verbose => global.verbosity.push(Verbosity::Verbose),
        GlobalOption::Quiet => global.verbosity.push(Verbosity::Quiet),
    }
    Ok(())
}

fn unexpected_after_subcommand(arg: Arg) -> UsageError {
    match arg {
        Arg::Value(value) => UsageError::new(format!(
            "unexpected positional argument: {}",
            value.to_string_lossy()
        )),
        _ => UsageError::new(format!("unexpected option: {arg:?}")),
    }
}

fn parse_integer(value: OsString, option: &str) -> Result<u64, UsageError> {
    value
        .into_string()
        .map_err(|_| UsageError::new(format!("{option} requires a non-negative integer")))?
        .parse::<u64>()
        .map_err(|_| UsageError::new(format!("{option} requires a non-negative integer")))
}

fn parse_socket_addr(value: OsString, option: &str) -> Result<SocketAddr, UsageError> {
    value
        .into_string()
        .map_err(|_| UsageError::new(format!("{option} requires an addr:port value")))?
        .parse()
        .map_err(|_| UsageError::new(format!("{option} requires a numeric addr:port value")))
}

fn nonempty_value(value: OsString, option: &str) -> Result<String, UsageError> {
    let value = value
        .into_string()
        .map_err(|_| UsageError::new(format!("{option} requires valid UTF-8")))?;
    if value.is_empty() {
        Err(UsageError::new(format!(
            "{option} requires a non-empty value"
        )))
    } else {
        Ok(value)
    }
}

// PROPOSAL SM-D21: a decimal number of seconds above 0 (`10`, `0.5`); zero,
// negatives, non-numbers and non-finite values are usage errors. A missing
// value is lexopt's missing-argument error.
fn parse_timeout(value: OsString) -> Result<Duration, UsageError> {
    let error = || UsageError::new("--timeout requires a number of seconds above 0");
    let seconds = value
        .into_string()
        .map_err(|_| error())?
        .parse::<f64>()
        .map_err(|_| error())?;
    let duration = Duration::try_from_secs_f64(seconds).map_err(|_| error())?;
    if duration.is_zero() {
        return Err(error());
    }
    Ok(duration)
}

fn parse_log_level(value: &OsStr) -> Result<LevelFilter, UsageError> {
    match value.to_str() {
        Some("error") => Ok(LevelFilter::Error),
        Some("warn") => Ok(LevelFilter::Warn),
        Some("info") => Ok(LevelFilter::Info),
        Some("debug") => Ok(LevelFilter::Debug),
        Some("trace") => Ok(LevelFilter::Trace),
        _ => Err(UsageError::new(
            "--log-level must be error, warn, info, debug, or trace",
        )),
    }
}

fn apply_verbosity(mut level: LevelFilter, adjustments: &[Verbosity]) -> LevelFilter {
    for adjustment in adjustments {
        let rank: u8 = match level {
            LevelFilter::Error => 0,
            LevelFilter::Warn => 1,
            LevelFilter::Info => 2,
            LevelFilter::Debug => 3,
            LevelFilter::Trace => 4,
            LevelFilter::Off => 0,
        };
        let rank = match adjustment {
            Verbosity::Verbose => (rank + 1).min(4),
            Verbosity::Quiet => rank.saturating_sub(1),
        };
        level = match rank {
            0 => LevelFilter::Error,
            1 => LevelFilter::Warn,
            2 => LevelFilter::Info,
            3 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        };
    }
    level
}

pub fn help() -> &'static str {
    LONG_HELP
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Command {
        match parse_from(args.iter().copied()) {
            Ok(Action::Run(command)) => command,
            other => panic!("expected command, got {other:?}"),
        }
    }

    fn usage(args: &[&str]) {
        assert!(
            parse_from(args.iter().copied()).is_err(),
            "accepted {args:?}"
        );
    }

    #[test]
    fn accepts_spec_examples_and_long_equals_values() {
        let Command::Rx(rx) = run(&[
            "-n",
            "8",
            "rx",
            "--bind",
            "0.0.0.0",
            "--connect",
            "127.0.0.1:8080",
        ]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.streams, 8);
        assert_eq!(rx.bind, "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(rx.connect, "127.0.0.1:8080".parse().unwrap());

        let Command::Tx(tx) = run(&[
            "--streams=8",
            "--base-port=32000",
            "tx",
            "--listen=127.0.0.1:18080",
            "--remote=192.0.2.10",
        ]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.base_port, 32000);
        assert_eq!(tx.listen, "127.0.0.1:18080".parse().unwrap());
        assert_eq!(tx.remote, "192.0.2.10");

        let Command::Rx(rx) = run(&["-v", "--udp-buffer", "4194304", "rx"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.log_level, LevelFilter::Debug);
        assert_eq!(rx.global.udp_buffer, Some(4194304));
    }

    #[test]
    fn defaults_and_repeated_values_follow_the_spec() {
        let Command::Tx(tx) = run(&["--streams", "8", "-n", "3", "tx"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.streams, 3);
        assert_eq!(tx.global.base_port, 31000);
        assert_eq!(tx.listen, "127.0.0.1:31100".parse().unwrap());
        assert_eq!(tx.remote, "127.0.0.1");

        let Command::Rx(rx) = run(&["rx"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.streams, 4);
        assert_eq!(rx.bind, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(rx.connect, "127.0.0.1:31200".parse().unwrap());
    }

    #[test]
    fn help_and_version_win_over_all_errors() {
        assert_eq!(parse_from(["--help"]), Ok(Action::Help));
        assert_eq!(parse_from(["-n", "bad", "tx", "--help"]), Ok(Action::Help));
        assert_eq!(parse_from(["--version"]), Ok(Action::Version));
        assert_eq!(parse_from(["rx", "--help"]), Ok(Action::Help));
        assert_eq!(parse_from(["rx", "-vh"]), Ok(Action::Help));
        assert_eq!(parse_from(["tx-pat", "-V"]), Ok(Action::Version));
        assert_eq!(parse_from(["tx-pat", "-vV"]), Ok(Action::Version));
        assert_eq!(parse_from(["unknown", "-V"]), Ok(Action::Version));
        assert!(matches!(
            parse_from(["tx", "-rhost"]),
            Ok(Action::Run(Command::Tx(_)))
        ));
    }

    #[test]
    fn rejects_missing_and_unknown_subcommands() {
        usage(&[]);
        usage(&["-n", "3"]);
        usage(&["unknown"]);
    }

    #[test]
    fn accepts_global_options_after_subcommands() {
        let Command::TxPat(tx_pat) = run(&["tx-pat", "-p", "32000"]) else {
            panic!("expected tx-pat");
        };
        assert_eq!(tx_pat.global.base_port, 32000);

        let Command::Tx(tx) = run(&["tx", "-p", "32000", "-"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.base_port, 32000);
        assert_eq!(tx.input, Some(Input::Stdin));

        let Command::Rx(rx) = run(&["rx", "--base-port=32000"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.base_port, 32000);

        let Command::Rx(rx) = run(&["rx", "-n", "5", "--bind", "127.0.0.1"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.streams, 5);
        assert_eq!(rx.bind, "127.0.0.1".parse::<IpAddr>().unwrap());

        let Command::CheckPat(check_pat) =
            run(&["check-pat", "--log-level", "debug", "--seed", "1"])
        else {
            panic!("expected check-pat");
        };
        assert_eq!(check_pat.global.log_level, LevelFilter::Debug);
        assert_eq!(check_pat.seed, Some(1));
    }

    #[test]
    fn global_values_and_verbosity_follow_command_line_order() {
        let Command::TxPat(tx_pat) = run(&["-p", "31000", "tx-pat", "-p", "32000"]) else {
            panic!("expected tx-pat");
        };
        assert_eq!(tx_pat.global.base_port, 32000);

        let Command::Rx(rx) = run(&["-p", "32000", "rx", "-p", "31000"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.base_port, 31000);

        let Command::Tx(tx) = run(&["--log-level", "warn", "tx", "-v"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.log_level, LevelFilter::Info);
    }

    #[test]
    fn accepts_attached_and_clustered_globals_after_the_subcommand() {
        let Command::TxPat(tx_pat) = run(&["tx-pat", "-p32000", "-vv"]) else {
            panic!("expected tx-pat");
        };
        assert_eq!(tx_pat.global.base_port, 32000);
        assert_eq!(tx_pat.global.log_level, LevelFilter::Trace);

        let Command::Rx(rx) = run(&["rx", "--udp-buffer=4096", "-q"]) else {
            panic!("expected rx");
        };
        assert_eq!(rx.global.udp_buffer, Some(4096));
        assert_eq!(rx.global.log_level, LevelFilter::Warn);
    }

    #[test]
    fn rejects_command_options_in_the_wrong_position_or_mode() {
        usage(&["--listen", "127.0.0.1:1", "tx"]);
        usage(&["--bind", "127.0.0.1", "rx"]);
        usage(&["rx", "--listen", "127.0.0.1:31100"]);
        usage(&["rx", "--remote", "x"]);
        usage(&["tx", "--connect", "127.0.0.1:31200"]);
    }

    #[test]
    fn rejects_invalid_counts_ports_and_addresses() {
        usage(&["-n", "2", "tx"]);
        usage(&["-n", "0", "rx"]);
        usage(&["-p", "0", "tx"]);
        usage(&["-p", "65536", "rx"]);
        usage(&["-n", "3", "-p", "65534", "rx"]);
        usage(&["tx-pat", "-n2"]);
        usage(&["tx", "--listen", "0.0.0.0:31100"]);
        usage(&["tx", "--listen", "127.0.0.1:0"]);
        usage(&["rx", "--connect", "127.0.0.1:0"]);
    }

    fn command_timeout(command: &Command) -> Duration {
        match command {
            Command::Tx(config) => config.global.timeout,
            Command::TxPat(config) => config.global.timeout,
            Command::CheckPat(config) => config.global.timeout,
            Command::Rx(config) => config.global.timeout,
        }
    }

    #[test]
    fn timeout_defaults_to_ten_seconds() {
        assert_eq!(command_timeout(&run(&["rx"])), Duration::from_secs(10));
        assert_eq!(command_timeout(&run(&["tx"])), Duration::from_secs(10));
        assert_eq!(command_timeout(&run(&["tx-pat"])), Duration::from_secs(10));
        assert_eq!(
            command_timeout(&run(&["check-pat"])),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn timeout_accepts_fractional_seconds_before_and_after_the_subcommand() {
        let cases: &[(&[&str], Duration)] = &[
            (&["--timeout", "0.5", "rx"], Duration::from_millis(500)),
            (&["rx", "--timeout", "0.5"], Duration::from_millis(500)),
            (&["tx", "--timeout=0.5"], Duration::from_millis(500)),
            (&["--timeout=0.5", "tx-pat"], Duration::from_millis(500)),
            (&["--timeout", "10", "rx"], Duration::from_secs(10)),
        ];
        for (args, expected) in cases {
            assert_eq!(command_timeout(&run(args)), *expected, "{args:?}");
        }
    }

    #[test]
    fn the_last_timeout_wins_across_positions() {
        assert_eq!(
            command_timeout(&run(&["--timeout", "30", "rx", "--timeout", "2"])),
            Duration::from_secs(2)
        );
        assert_eq!(
            command_timeout(&run(&["rx", "--timeout", "2", "--timeout", "30"])),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn bad_timeout_values_are_usage_errors() {
        usage(&["--timeout", "0", "rx"]);
        usage(&["--timeout", "0.0", "rx"]);
        usage(&["--timeout", "-1", "rx"]);
        usage(&["--timeout", "x", "rx"]);
        usage(&["--timeout", "nan", "rx"]);
        usage(&["--timeout", "inf", "rx"]);
        usage(&["--timeout", "1e999", "rx"]);
        usage(&["rx", "--timeout"]);
        usage(&["tx-pat", "--timeout", "0"]);
    }

    #[test]
    fn rejects_missing_malformed_and_nonnumeric_values() {
        usage(&["-n"]);
        usage(&["-n", "many", "tx"]);
        usage(&["--udp-buffer", "0", "rx"]);
        usage(&["--log-level", "loud", "tx"]);
        usage(&["tx", "--listen", "not-an-address"]);
        usage(&["rx", "--bind", "localhost"]);
        usage(&["rx", "--connect", "127.0.0.1"]);
        usage(&["tx", "--remote="]);
    }

    #[test]
    fn tx_input_operand_and_its_usage_errors() {
        // PROPOSAL SM-D16: no operand keeps the TCP listen default; "-" is stdin.
        let Command::Tx(tx) = run(&["tx"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.input, None);
        let Command::Tx(tx) = run(&["tx", "-"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.input, Some(Input::Stdin));
        let Command::Tx(tx) = run(&["tx", "--remote", "192.0.2.10", "data.bin"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.input, Some(Input::File(OsString::from("data.bin"))));
        assert_eq!(tx.remote, "192.0.2.10");

        usage(&["tx", "--listen", "127.0.0.1:31100", "-"]);
        usage(&["tx", "file-a", "--listen=127.0.0.1:31100"]);
        usage(&["tx", "file-a", "file-b"]);
        usage(&["tx", "-", "-"]);
    }

    #[test]
    fn tx_pat_options_and_usage_errors() {
        let Command::TxPat(tx_pat) = run(&["tx-pat"]) else {
            panic!("expected tx-pat");
        };
        assert_eq!(tx_pat.remote, "127.0.0.1");
        assert_eq!(tx_pat.seed, None);

        let Command::TxPat(tx_pat) =
            run(&["-n", "8", "tx-pat", "--seed", "42", "-r", "192.0.2.10"])
        else {
            panic!("expected tx-pat");
        };
        assert_eq!(tx_pat.seed, Some(42));
        assert_eq!(tx_pat.remote, "192.0.2.10");
        assert_eq!(tx_pat.global.streams, 8);

        usage(&["tx-pat", "--listen", "127.0.0.1:31100"]);
        usage(&["tx-pat", "-"]);
        usage(&["tx-pat", "data.bin"]);
        usage(&["tx-pat", "--seed", "not-a-number"]);
        usage(&["tx-pat", "--seed", "-1"]);
        usage(&["tx-pat", "--seed", "18446744073709551616"]);
        usage(&["tx-pat", "--bind", "127.0.0.1"]);
        usage(&["tx-pat", "--connect", "127.0.0.1:31200"]);
    }

    #[test]
    fn check_pat_options_and_usage_errors() {
        let Command::CheckPat(check_pat) = run(&["check-pat"]) else {
            panic!("expected check-pat");
        };
        assert_eq!(check_pat.seed, None);
        let Command::CheckPat(check_pat) = run(&["check-pat", "-s", "7"]) else {
            panic!("expected check-pat");
        };
        assert_eq!(check_pat.seed, Some(7));

        usage(&["check-pat", "--remote", "127.0.0.1"]);
        usage(&["check-pat", "--bind", "127.0.0.1"]);
        usage(&["check-pat", "file"]);
        usage(&["check-pat", "--seed", "x"]);
    }

    #[test]
    fn seed_is_rejected_on_tx_and_rx_and_before_the_subcommand() {
        usage(&["tx", "--seed", "1"]);
        usage(&["rx", "--seed", "1"]);
        usage(&["--seed", "1", "tx-pat"]);
        usage(&["-s", "1", "check-pat"]);
    }

    #[test]
    fn verbosity_uses_log_level_as_its_starting_point_and_clamps() {
        let Command::Tx(tx) = run(&["-v", "--log-level", "warn", "-v", "tx"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.log_level, LevelFilter::Debug);
        let Command::Tx(tx) = run(&["-q", "-q", "-q", "-q", "tx"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.log_level, LevelFilter::Error);
        let Command::Tx(tx) = run(&["-v", "-v", "-v", "tx"]) else {
            panic!("expected tx");
        };
        assert_eq!(tx.global.log_level, LevelFilter::Trace);
    }
}
