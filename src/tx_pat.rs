use std::sync::Arc;

use anyhow::Result;
use log::info;

use crate::cli::TxPatConfig;
use crate::pattern::{self, PatternStream};
use crate::stop::Stop;
use crate::tx::{self, Reader};

/// SM-P10: tx-pat sends exactly like tx (same sockets, header, round-robin, payload
/// size, --udp-buffer), but its input is the generated pattern.
pub fn run(config: TxPatConfig, stop: Arc<Stop>) -> Result<()> {
    // SM-D17: a random seed unless --seed sets a fixed one; always logged.
    let seed = config.seed.unwrap_or_else(pattern::random_seed);
    info!(target: "socket_mux::tx", "tx-pat seed={seed}");
    let session = pattern::random_session_id();
    info!(target: "socket_mux::tx", "session={session}");
    let sockets = tx::udp_sockets(&config.global, &config.remote)?;
    // By default tx-pat does not pace and runs until signalled, without EOS.
    // --bytes N sends exactly N pattern bytes, then EOS, and exits 0;
    // --ipd-ns paces datagrams across all lanes.
    if config.ipd_ns > 0 {
        info!(target: "socket_mux::tx", "pacing ipd_ns={}", config.ipd_ns);
    }
    let reader = Reader::Pattern(Box::new(PatternStream::new(seed)), config.bytes);
    let pacer = tx::Pacer::new(config.ipd_ns);
    tx::with_report(&config.global, |meter| {
        tx::send_stream(
            sockets,
            reader,
            session,
            config.global.datagram_size,
            pacer,
            &stop,
            meter,
        )
    })
}
