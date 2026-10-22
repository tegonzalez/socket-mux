// PROPOSAL SM-D18: the tx-pat pattern is blocks of 4096 bytes; block i holds i as an
// 8-byte big-endian index, 4084 bytes of SplitMix64 keyed by (seed, i), and the
// Adler-32 of the first 4092 bytes. The generator choice is OD-19 open: provisional
// implementation value, not a requirement. scripts/check-pat.py of the verification
// skill reimplements this format independently; the two must stay byte-identical.

use std::time::{SystemTime, UNIX_EPOCH};

pub const BLOCK_LEN: usize = 4096; // PROPOSAL SM-D18: pattern block size.
pub const SUMMED_LEN: usize = BLOCK_LEN - 4; // PROPOSAL SM-D18: bytes the Adler-32 covers.

const GOLDEN: u64 = 0x9E3779B97F4A7C15; // PROPOSAL SM-D18: SplitMix64 increment.
const ADLER_MOD: u64 = 65521; // PROPOSAL SM-D18: Adler-32 modulus.

fn splitmix64(state: &mut u64) -> u64 {
    // PROPOSAL SM-D18: one SplitMix64 step (OD-19 open: provisional).
    *state = state.wrapping_add(GOLDEN);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

pub fn adler32(data: &[u8]) -> u32 {
    // PROPOSAL SM-D18: Adler-32, a starts at 1 and b at 0; the value is (b << 16) | a.
    let mut a: u64 = 1;
    let mut b: u64 = 0;
    // Mod at chunk ends: within 4096 bytes a stays below 1.1M and b below 4.3G.
    for chunk in data.chunks(4096) {
        for &byte in chunk {
            a += u64::from(byte);
            b += a;
        }
        a %= ADLER_MOD;
        b %= ADLER_MOD;
    }
    ((b << 16) | a) as u32
}

pub fn generate_block(seed: u64, index: u64, out: &mut [u8; BLOCK_LEN]) {
    out[..8].copy_from_slice(&index.to_be_bytes());
    // PROPOSAL SM-D18: 511 SplitMix64 steps of 8 big-endian bytes; the last step
    // contributes only its first 4 bytes, for 4084 body bytes in total.
    let mut state = seed ^ index.wrapping_mul(GOLDEN);
    let mut at = 8;
    for _ in 0..511 {
        let bytes = splitmix64(&mut state).to_be_bytes();
        let take = (SUMMED_LEN - at).min(8);
        out[at..at + take].copy_from_slice(&bytes[..take]);
        at += take;
    }
    debug_assert_eq!(at, SUMMED_LEN);
    let sum = adler32(&out[..SUMMED_LEN]);
    out[SUMMED_LEN..].copy_from_slice(&sum.to_be_bytes());
}

pub fn block_body_matches(seed: u64, window: &[u8]) -> bool {
    debug_assert_eq!(window.len(), BLOCK_LEN);
    let index = u64::from_be_bytes(window[..8].try_into().unwrap());
    let mut block = [0_u8; BLOCK_LEN];
    generate_block(seed, index, &mut block);
    window[..SUMMED_LEN] == block[..SUMMED_LEN]
}

/// The pattern as an endless byte stream: block 0, block 1, ... with no pacing.
pub struct PatternStream {
    seed: u64,
    index: u64,
    block: [u8; BLOCK_LEN],
    at: usize,
}

impl PatternStream {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            index: 0,
            block: [0_u8; BLOCK_LEN],
            at: BLOCK_LEN,
        }
    }

    /// Fill `buf` with pattern bytes; always returns `buf.len()` (never end of stream).
    pub fn fill(&mut self, buf: &mut [u8]) -> usize {
        let mut written = 0;
        while written < buf.len() {
            if self.at == BLOCK_LEN {
                generate_block(self.seed, self.index, &mut self.block);
                self.index += 1;
                self.at = 0;
            }
            let take = (BLOCK_LEN - self.at).min(buf.len() - written);
            buf[written..written + take].copy_from_slice(&self.block[self.at..self.at + take]);
            self.at += take;
            written += take;
        }
        written
    }
}

pub fn random_seed() -> u64 {
    // OD-18 open: provisional implementation value, not a requirement —
    // the system time in nanoseconds mixed with the process id through SplitMix64.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut state = nanos ^ u64::from(std::process::id()).wrapping_mul(GOLDEN);
    splitmix64(&mut state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_layout_has_index_and_adler_trailer() {
        let mut block = [0_u8; BLOCK_LEN];
        generate_block(42, 7, &mut block);
        assert_eq!(
            u64::from_be_bytes(block[..8].try_into().unwrap()),
            7,
            "first 8 bytes hold the block index, big-endian"
        );
        assert_eq!(
            u32::from_be_bytes(block[SUMMED_LEN..].try_into().unwrap()),
            adler32(&block[..SUMMED_LEN]),
            "last 4 bytes are the Adler-32 of the first 4092, big-endian"
        );
        assert!(block_body_matches(42, &block));
        assert!(!block_body_matches(43, &block));
    }

    #[test]
    fn blocks_are_deterministic_per_seed_and_differ_across_seeds() {
        let mut a = [0_u8; BLOCK_LEN];
        let mut b = [0_u8; BLOCK_LEN];
        generate_block(42, 0, &mut a);
        generate_block(42, 0, &mut b);
        assert_eq!(a, b, "same seed and index give the same block");
        generate_block(43, 0, &mut b);
        assert_ne!(a, b, "different seeds give different blocks");
        generate_block(42, 1, &mut b);
        assert_ne!(a, b, "different indices give different blocks");
    }

    #[test]
    fn adler32_matches_a_reference_sum() {
        // 4092 zero bytes: a = 1, b = 4092.
        assert_eq!(adler32(&[0_u8; 4092]), (4092 << 16) | 1);
        assert_eq!(adler32(&[]), 1);
        // "Wikipedia" from the Adler-32 specification example: 300286872 = 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn stream_is_the_concatenation_of_blocks() {
        let mut stream = PatternStream::new(9);
        let mut bytes = vec![0_u8; BLOCK_LEN * 2 + 100];
        assert_eq!(stream.fill(&mut bytes), bytes.len());
        for index in 0..2_u64 {
            let mut block = [0_u8; BLOCK_LEN];
            generate_block(9, index, &mut block);
            let at = index as usize * BLOCK_LEN;
            assert_eq!(bytes[at..at + BLOCK_LEN], block);
        }
        let mut block2 = [0_u8; BLOCK_LEN];
        generate_block(9, 2, &mut block2);
        assert_eq!(bytes[BLOCK_LEN * 2..], block2[..100]);
        assert_eq!(
            stream.fill(&mut [0_u8; 7]),
            7,
            "never returns end of stream"
        );
    }

    #[test]
    fn random_seeds_differ() {
        assert_ne!(random_seed(), random_seed());
    }
}
