use anyhow::Result;
use log::info;

use crate::cli::TxPatConfig;
use crate::pattern::{self, PatternStream};
use crate::tx::{self, Reader};

/// SM-P10: tx-pat sends exactly like tx (same sockets, header, round-robin, payload
/// size, --udp-buffer), but its input is the generated pattern.
pub fn run(config: TxPatConfig) -> Result<()> {
    // SM-D17: a random seed unless --seed sets a fixed one; always logged.
    let seed = config.seed.unwrap_or_else(pattern::random_seed);
    info!(target: "socket_mux::tx", "tx-pat seed={seed}");
    let session = pattern::random_session_id();
    info!(target: "socket_mux::tx", "session={session}");
    let sockets = tx::udp_sockets(&config.global, &config.remote)?;
    // Implementation value: tx-pat never
    // ends on its own, does not pace, and sends no EOS; it runs until signalled.
    let reader = Reader::Pattern(Box::new(PatternStream::new(seed)));
    tx::send_stream(sockets, reader, session, config.global.datagram_size)
}
