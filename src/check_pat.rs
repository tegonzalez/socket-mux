// PROPOSAL for OD-20: check-pat validates RX's output against the SM-D18 pattern.
// It is a fourth subcommand and not in the spec grammar yet (the spec lists only
// tx, tx-pat and rx). It opens no socket: it reads the stream from standard input.

use std::io::{self, Read};

use anyhow::{Context, Result};

use crate::cli::CheckPatConfig;
use crate::pattern::{self, BLOCK_LEN, SUMMED_LEN};

const READ_CHUNK: usize = 4 << 20; // read standard input in large chunks, not per byte

pub fn run(config: &CheckPatConfig) -> Result<u8> {
    let data = read_stdin()?;
    let stats = analyze(&data, config.seed);
    println!("{}", stats.line());
    Ok(stats.exit_code(data.is_empty()))
}

fn read_stdin() -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut chunk = vec![0_u8; READ_CHUNK];
    let mut stdin = io::stdin().lock();
    loop {
        let count = stdin
            .read(&mut chunk)
            .context("could not read standard input")?;
        if count == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..count]);
    }
    Ok(data)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    pub blocks_ok: u64,
    pub blocks_bad: u64,
    pub resyncs: u64,
    pub lost_bytes: u64,
    pub skipped_bytes: u64,
    pub trailing_bytes: u64,
    pub first_index: Option<u64>,
    pub last_index: Option<u64>,
}

impl Stats {
    pub fn line(&self) -> String {
        let show = |value: Option<u64>| {
            value
                .map(|number| number.to_string())
                .unwrap_or_else(|| "none".to_owned())
        };
        format!(
            "check-pat: blocks_ok={} blocks_bad={} resyncs={} lost_bytes={} \
             skipped_bytes={} trailing_bytes={} first_index={} last_index={}",
            self.blocks_ok,
            self.blocks_bad,
            self.resyncs,
            self.lost_bytes,
            self.skipped_bytes,
            self.trailing_bytes,
            show(self.first_index),
            show(self.last_index),
        )
    }

    // PROPOSAL for OD-20: exit 0 when no block was bad and at least one block was
    // ok or the input was empty; exit 1 otherwise.
    pub fn exit_code(&self, input_empty: bool) -> u8 {
        if self.blocks_bad == 0 && (self.blocks_ok > 0 || input_empty) {
            0
        } else {
            1
        }
    }
}

pub fn analyze(data: &[u8], seed: Option<u64>) -> Stats {
    let mut stats = Stats::default();
    let mut pos = 0_usize; // current position; the anchor block ends here (p)
    let mut anchor: i128 = -1; // a: index of the previous valid block, -1 at the start
    while data.len() - pos >= BLOCK_LEN {
        if let Some(index) = valid_window(data, pos, seed) {
            if index as i128 == anchor + 1 {
                accept(&mut stats, &mut anchor, index);
                pos += BLOCK_LEN;
                continue;
            }
        }
        let Some((q, b)) = find_next_valid(data, pos, seed) else {
            break;
        };
        // Resync from previous valid index a ending at byte p to the next valid
        // index b starting at byte q: s skipped bytes, E the missing blocks' length.
        let s = (q - pos) as i128;
        let e = (b as i128 - anchor - 1) * BLOCK_LEN as i128;
        stats.skipped_bytes += s as u64;
        if b as i128 <= anchor {
            // A valid block whose index does not move forward: corrupt. Keep the
            // anchor index (the stream is expected to continue after it) and move
            // past this block's bytes.
            stats.blocks_bad += 1;
            pos = q + BLOCK_LEN;
        } else if s < e {
            stats.lost_bytes += (e - s) as u64;
            stats.resyncs += 1;
            accept(&mut stats, &mut anchor, b);
            pos = q + BLOCK_LEN;
        } else if s == e {
            stats.blocks_bad += (b as i128 - anchor - 1) as u64;
            accept(&mut stats, &mut anchor, b);
            pos = q + BLOCK_LEN;
        } else {
            stats.blocks_bad += ((s - e) as u64 / BLOCK_LEN as u64).max(1);
            accept(&mut stats, &mut anchor, b);
            pos = q + BLOCK_LEN;
        }
    }
    // After the last valid block the remaining bytes are explained by a final gap
    // plus truncation, so they are trailing bytes, never bad blocks: after injected
    // loss the tail holds partially delivered blocks and can be many blocks long.
    stats.trailing_bytes = (data.len() - pos) as u64;
    stats
}

fn accept(stats: &mut Stats, anchor: &mut i128, index: u64) {
    stats.blocks_ok += 1;
    stats.first_index.get_or_insert(index);
    stats.last_index = Some(index);
    *anchor = index as i128;
}

/// The index of a valid 4096-byte block window at `pos`, if the window there is one.
fn valid_window(data: &[u8], pos: usize, seed: Option<u64>) -> Option<u64> {
    let window = data.get(pos..pos + BLOCK_LEN)?;
    let stored = u32::from_be_bytes(window[SUMMED_LEN..BLOCK_LEN].try_into().unwrap());
    if stored != pattern::adler32(&window[..SUMMED_LEN]) {
        return None;
    }
    let index = u64::from_be_bytes(window[..8].try_into().unwrap());
    match seed {
        Some(seed) if !pattern::block_body_matches(seed, window) => None,
        _ => Some(index),
    }
}

/// The next valid block window at or after `from`, sliding one byte at a time.
fn find_next_valid(data: &[u8], from: usize, seed: Option<u64>) -> Option<(usize, u64)> {
    if data.len() - from < BLOCK_LEN {
        return None;
    }
    let mut rolling = RollingAdler::new(&data[from..from + SUMMED_LEN]);
    let mut q = from;
    loop {
        let window = &data[q..q + BLOCK_LEN];
        let stored = u32::from_be_bytes(window[SUMMED_LEN..BLOCK_LEN].try_into().unwrap());
        if rolling.value() == stored {
            let index = u64::from_be_bytes(window[..8].try_into().unwrap());
            let body_ok = match seed {
                Some(seed) => pattern::block_body_matches(seed, window),
                None => true,
            };
            if body_ok {
                return Some((q, index));
            }
        }
        if q + BLOCK_LEN >= data.len() {
            return None;
        }
        rolling.roll(window[0], window[SUMMED_LEN]);
        q += 1;
    }
}

/// Rolling Adler-32 over a fixed 4092-byte window; one slide step is O(1).
struct RollingAdler {
    a: u64,
    b: u64,
}

impl RollingAdler {
    fn new(window: &[u8]) -> Self {
        debug_assert_eq!(window.len(), SUMMED_LEN);
        let mut a = 1_u64;
        let mut b = 0_u64;
        for &byte in window {
            a += u64::from(byte);
            b += a;
        }
        Self {
            a: a % 65521,
            b: b % 65521,
        }
    }

    fn value(&self) -> u32 {
        ((self.b << 16) | self.a) as u32
    }

    /// Slide the window one byte forward: remove `out`, append `into`.
    fn roll(&mut self, out: u8, into: u8) {
        let (a, b) = (self.a as i64, self.b as i64);
        let (out, into) = (i64::from(out), i64::from(into));
        let n = SUMMED_LEN as i64;
        // a' = a - out + into; b' = b - (n + 1) * out + a - 1 + into (mod 65521).
        self.a = (a - out + into).rem_euclid(65521) as u64;
        self.b = (b - (n + 1) * out + a - 1 + into).rem_euclid(65521) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{generate_block, BLOCK_LEN as B};

    const SEED: u64 = 42;

    fn stream(seed: u64, blocks: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(blocks as usize * B);
        let mut block = [0_u8; B];
        for index in 0..blocks {
            generate_block(seed, index, &mut block);
            data.extend_from_slice(&block);
        }
        data
    }

    fn stats(data: &[u8]) -> Stats {
        analyze(data, Some(SEED))
    }

    #[test]
    fn clean_stream_is_all_ok() {
        let data = stream(SEED, 10);
        let stats = stats(&data);
        assert_eq!(stats.blocks_ok, 10);
        assert_eq!(stats.blocks_bad, 0);
        assert_eq!(stats.resyncs, 0);
        assert_eq!(stats.lost_bytes, 0);
        assert_eq!(stats.skipped_bytes, 0);
        assert_eq!(stats.trailing_bytes, 0);
        assert_eq!(stats.first_index, Some(0));
        assert_eq!(stats.last_index, Some(9));
        assert_eq!(stats.exit_code(false), 0);
    }

    #[test]
    fn empty_input_exits_zero() {
        let stats = stats(&[]);
        assert_eq!(stats, Stats::default());
        assert_eq!(stats.exit_code(true), 0);
        assert!(stats.line().contains("first_index=none last_index=none"));
    }

    #[test]
    fn removed_byte_ranges_are_loss_not_corruption() {
        // SplitMix64 picks the ranges, so the positions are arbitrary but repeatable.
        let mut rng = 7_u64;
        let mut next = || {
            rng = rng.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = rng;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        let blocks = 20_u64;
        // Three distinct blocks, arbitrary offset and length inside each block.
        let mut ranges: Vec<(usize, usize)> = [3_u64, 11, 17]
            .into_iter()
            .map(|block| {
                let offset = (next() % 3000) as usize;
                let len = 1 + (next() % 1000) as usize;
                (block as usize * B + offset, len)
            })
            .collect();
        ranges.sort();
        let mut data = stream(SEED, blocks);
        let mut removed = 0_u64;
        for &(start, len) in ranges.iter().rev() {
            removed += len as u64;
            data.drain(start..start + len);
        }
        let stats = stats(&data);
        assert_eq!(stats.lost_bytes, removed, "one loss gap per removed range");
        assert_eq!(stats.resyncs, 3);
        assert_eq!(stats.blocks_bad, 0);
        // Each cut leaves its block undeliverable; every other block is found.
        assert_eq!(stats.blocks_ok, blocks - 3);
        assert_eq!(stats.exit_code(false), 0);
    }

    #[test]
    fn one_flipped_byte_is_one_bad_block() {
        let mut data = stream(SEED, 10);
        data[2 * B + 100] ^= 1;
        let stats = stats(&data);
        assert_eq!(stats.blocks_ok, 9);
        assert_eq!(stats.blocks_bad, 1);
        assert_eq!(stats.lost_bytes, 0);
        assert_eq!(stats.exit_code(false), 1);
    }

    #[test]
    fn block_aligned_whole_block_removal_is_a_loss_gap() {
        // impl-2 section 5 says "reported as bad", but the section 3 rule (s = 0 <
        // E = 4096) and the verification skill's cut-block case both make it a loss.
        let mut data = stream(SEED, 10);
        data.drain(5 * B..6 * B);
        let stats = stats(&data);
        assert_eq!(stats.blocks_ok, 9);
        assert_eq!(stats.blocks_bad, 0);
        assert_eq!(stats.resyncs, 1);
        assert_eq!(stats.lost_bytes, B as u64);
        assert_eq!(stats.exit_code(false), 0);
    }

    #[test]
    fn truncated_tail_is_trailing_not_bad() {
        let mut data = stream(SEED, 10);
        data.truncate(10 * B - 1000);
        let stats = stats(&data);
        assert_eq!(stats.blocks_ok, 9);
        assert_eq!(stats.blocks_bad, 0);
        assert_eq!(stats.trailing_bytes, 3096);
        assert_eq!(stats.exit_code(false), 0);
    }

    #[test]
    fn input_starting_mid_block_resyncs_to_the_next_block() {
        let data = stream(SEED, 10)[3 * B + 500..].to_vec();
        let stats = stats(&data);
        assert_eq!(stats.blocks_ok, 6, "blocks 4 to 9");
        assert_eq!(stats.resyncs, 1);
        assert_eq!(stats.lost_bytes, (3 * B + 500) as u64);
        assert_eq!(stats.blocks_bad, 0);
        assert_eq!(stats.first_index, Some(4));
        assert_eq!(stats.last_index, Some(9));
        assert_eq!(stats.exit_code(false), 0);
    }

    #[test]
    fn wrong_seed_finds_no_valid_block() {
        let data = stream(SEED, 10);
        let stats = analyze(&data, Some(SEED + 1));
        assert_eq!(stats.blocks_ok, 0);
        assert_eq!(stats.exit_code(false), 1);
    }

    #[test]
    fn without_a_seed_only_the_adler_and_index_order_are_checked() {
        let data = stream(SEED, 5);
        let stats = analyze(&data, None);
        assert_eq!(stats.blocks_ok, 5);
        assert_eq!(stats.exit_code(false), 0);
        let mut corrupted = stream(SEED, 5);
        corrupted[B + 42] ^= 0x80;
        let stats = analyze(&corrupted, None);
        assert_eq!(stats.blocks_bad, 1);
        assert_eq!(stats.exit_code(false), 1);
    }

    #[test]
    fn rolling_adler_matches_fresh_adler_at_every_position() {
        let mut data = vec![0_u8; SUMMED_LEN * 3];
        let mut state = 0x1234_5678_9ABC_DEF0_u64;
        for byte in &mut data {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            *byte = (z ^ (z >> 31)) as u8;
        }
        let mut rolling = RollingAdler::new(&data[..SUMMED_LEN]);
        for q in 0..=SUMMED_LEN {
            assert_eq!(
                rolling.value(),
                pattern::adler32(&data[q..q + SUMMED_LEN]),
                "rolling value diverged at position {q}"
            );
            if q + SUMMED_LEN < data.len() {
                rolling.roll(data[q], data[q + SUMMED_LEN]);
            }
        }
    }
}
