use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::header::{Header, FLAG_EOS};

#[derive(Debug)]
pub struct Datagram {
    pub header: Header,
    pub payload: Bytes,
}

#[derive(Debug)]
struct Pending {
    datagram: Datagram,
    arrived: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertOutcome {
    Inserted,
    Late,
    Duplicate,
}

#[derive(Debug)]
pub struct Delivery {
    pub payload: Bytes,
    pub end_of_stream: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LossGap {
    pub first_sequence: u64,
    pub lost_slices: u64,
    pub lost_bytes: u64,
    pub total_lost_slices: u64,
    pub total_lost_bytes: u64,
}

#[derive(Debug, Default)]
pub struct Reorder {
    next_sequence: u64,
    next_offset: u64,
    pending: BTreeMap<u64, Pending>,
    pending_bytes: usize,
    total_lost_slices: u64,
    total_lost_bytes: u64,
}

impl Reorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, datagram: Datagram) -> InsertOutcome {
        self.insert_at(datagram, Instant::now())
    }

    /// Insert with an explicit arrival time; tests use it to control the reorder clock.
    fn insert_at(&mut self, datagram: Datagram, arrived: Instant) -> InsertOutcome {
        let sequence = datagram.header.sequence;
        if sequence < self.next_sequence {
            return InsertOutcome::Late;
        }
        if self.pending.contains_key(&sequence) {
            return InsertOutcome::Duplicate;
        }
        self.pending_bytes += datagram.payload.len();
        self.pending.insert(sequence, Pending { datagram, arrived });
        InsertOutcome::Inserted
    }

    pub fn pop_ready(&mut self) -> Option<Delivery> {
        let pending = self.pending.remove(&self.next_sequence)?;
        self.pending_bytes -= pending.datagram.payload.len();
        self.next_sequence += 1;
        self.next_offset =
            pending.datagram.header.stream_offset + pending.datagram.payload.len() as u64;
        Some(Delivery {
            end_of_stream: pending.datagram.header.flags & FLAG_EOS != 0,
            payload: pending.datagram.payload,
        })
    }

    /// OD-5 open: provisional rule, not a requirement — the head-of-line gap is
    /// overdue when the first pending datagram after it arrived at least `timeout`
    /// ago (RX calls this only after draining every socket), or when the buffered
    /// out-of-order data exceeds `memory_cap`. Each pending datagram carries its own
    /// arrival time, so after one gap is declared the next gap is overdue as soon as
    /// its own following datagram is old enough: hundreds of gaps from a dead leg
    /// are declared in one pass instead of one per timeout.
    pub fn head_gap_overdue(&self, now: Instant, timeout: Duration, memory_cap: usize) -> bool {
        let Some((&sequence, pending)) = self.pending.iter().next() else {
            return false;
        };
        if sequence <= self.next_sequence {
            return false;
        }
        self.pending_bytes > memory_cap || now.saturating_duration_since(pending.arrived) >= timeout
    }

    pub fn declare_loss(&mut self) -> Option<LossGap> {
        let (first_sequence, first_offset) = {
            let (&sequence, pending) = self.pending.iter().next()?;
            if sequence <= self.next_sequence {
                return None;
            }
            (sequence, pending.datagram.header.stream_offset)
        };
        let gap = LossGap {
            first_sequence: self.next_sequence,
            lost_slices: first_sequence - self.next_sequence,
            lost_bytes: first_offset.saturating_sub(self.next_offset),
            total_lost_slices: self.total_lost_slices + (first_sequence - self.next_sequence),
            total_lost_bytes: self.total_lost_bytes + first_offset.saturating_sub(self.next_offset),
        };
        self.total_lost_slices = gap.total_lost_slices;
        self.total_lost_bytes = gap.total_lost_bytes;
        self.next_sequence = first_sequence;
        self.next_offset = first_offset;
        Some(gap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn datagram(sequence: u64, offset: u64, payload: &'static [u8]) -> Datagram {
        Datagram {
            header: Header::new(payload.len() as u16, sequence, offset, false),
            payload: Bytes::from_static(payload),
        }
    }

    fn deliver(reorder: &mut Reorder) -> Vec<Bytes> {
        let mut delivered = Vec::new();
        while let Some(delivery) = reorder.pop_ready() {
            delivered.push(delivery.payload);
        }
        delivered
    }

    #[test]
    fn delivers_shuffled_input_in_order() {
        let mut reorder = Reorder::new();
        assert_eq!(
            reorder.insert(datagram(2, 4, b"ef")),
            InsertOutcome::Inserted
        );
        assert!(deliver(&mut reorder).is_empty());
        reorder.insert(datagram(0, 0, b"ab"));
        assert_eq!(deliver(&mut reorder), vec![Bytes::from_static(b"ab")]);
        reorder.insert(datagram(1, 2, b"cd"));
        assert_eq!(
            deliver(&mut reorder),
            vec![Bytes::from_static(b"cd"), Bytes::from_static(b"ef")]
        );
    }

    #[test]
    fn a_gap_is_overdue_by_the_arrival_of_the_datagram_after_it() {
        let mut reorder = Reorder::new();
        let start = Instant::now();
        reorder.insert_at(datagram(0, 0, b"ab"), start);
        reorder.insert_at(datagram(2, 5, b"fg"), start);
        assert_eq!(deliver(&mut reorder).len(), 1);
        let timeout = Duration::from_millis(250);
        assert!(!reorder.head_gap_overdue(start, timeout, 1024));
        assert!(reorder.head_gap_overdue(start + timeout, timeout, 1024));
        assert_eq!(
            reorder.declare_loss(),
            Some(LossGap {
                first_sequence: 1,
                lost_slices: 1,
                lost_bytes: 3,
                total_lost_slices: 1,
                total_lost_bytes: 3,
            })
        );
        assert_eq!(deliver(&mut reorder), vec![Bytes::from_static(b"fg")]);
    }

    #[test]
    fn the_memory_cap_makes_a_gap_overdue_at_once() {
        let mut reorder = Reorder::new();
        let now = Instant::now();
        reorder.insert_at(datagram(3, 5, b"xxxxx"), now);
        assert!(reorder.head_gap_overdue(now, Duration::from_secs(3600), 4));
        assert!(!reorder.head_gap_overdue(now, Duration::from_secs(3600), 5));
    }

    #[test]
    fn accounts_for_a_consecutive_burst() {
        let mut reorder = Reorder::new();
        reorder.insert(datagram(3, 5, b"x"));
        let gap = reorder.declare_loss().unwrap();
        assert_eq!(gap.first_sequence, 0);
        assert_eq!(gap.lost_slices, 3);
        assert_eq!(gap.lost_bytes, 5);
        assert_eq!(gap.total_lost_slices, 3);
        assert_eq!(gap.total_lost_bytes, 5);
    }

    #[test]
    fn round_robin_drops_on_one_stream_are_separate_gaps() {
        let mut reorder = Reorder::new();
        reorder.insert(datagram(0, 0, b"a"));
        reorder.insert(datagram(2, 3, b"c"));
        reorder.insert(datagram(3, 4, b"d"));
        reorder.insert(datagram(5, 7, b"f"));
        assert_eq!(deliver(&mut reorder).len(), 1);

        let first = reorder.declare_loss().unwrap();
        assert_eq!(
            (first.first_sequence, first.lost_slices, first.lost_bytes),
            (1, 1, 2)
        );
        assert_eq!(deliver(&mut reorder).len(), 2);

        let second = reorder.declare_loss().unwrap();
        assert_eq!(
            (second.first_sequence, second.lost_slices, second.lost_bytes),
            (4, 1, 2)
        );
        assert_eq!(second.total_lost_slices, 2);
        assert_eq!(second.total_lost_bytes, 4);
        assert_eq!(deliver(&mut reorder), vec![Bytes::from_static(b"f")]);
    }

    #[test]
    fn gaps_whose_following_datagrams_are_old_are_all_overdue_in_one_pass() {
        // The M09/M10 shape: a dead leg leaves hundreds of gaps whose following
        // datagrams all arrived long ago; one RX pass must declare every gap.
        let mut reorder = Reorder::new();
        let long_ago = Instant::now() - Duration::from_secs(10);
        let timeout = Duration::from_millis(250);
        reorder.insert_at(datagram(0, 0, b"ab"), long_ago);
        for sequence in [2_u64, 3, 5, 6, 7, 9] {
            let offset = sequence * 2;
            reorder.insert_at(datagram(sequence, offset, b"xy"), long_ago);
        }
        assert_eq!(deliver(&mut reorder).len(), 1);

        // Emulate the RX pass: declare and deliver while the head gap is overdue.
        let now = Instant::now();
        let mut gaps = Vec::new();
        while reorder.head_gap_overdue(now, timeout, usize::MAX) {
            gaps.push(reorder.declare_loss().unwrap());
            deliver(&mut reorder);
        }
        assert_eq!(
            gaps.iter()
                .map(|gap| (gap.first_sequence, gap.lost_slices))
                .collect::<Vec<_>>(),
            vec![(1, 1), (4, 1), (8, 1)]
        );
        let last = gaps.last().unwrap();
        assert_eq!(last.total_lost_slices, 3);
        assert_eq!(last.total_lost_bytes, 6);
    }

    #[test]
    fn a_fresh_gap_is_not_overdue_until_its_own_datagram_ages() {
        // After one gap is declared, the next gap waits for its own following
        // datagram's arrival time, not for the previous gap's.
        let mut reorder = Reorder::new();
        let old = Instant::now() - Duration::from_secs(10);
        let fresh = Instant::now();
        let timeout = Duration::from_millis(250);
        reorder.insert_at(datagram(1, 2, b"cd"), old);
        reorder.insert_at(datagram(3, 6, b"gh"), fresh);
        let now = Instant::now();
        assert!(reorder.head_gap_overdue(now, timeout, usize::MAX));
        reorder.declare_loss();
        deliver(&mut reorder);
        assert!(
            !reorder.head_gap_overdue(now, timeout, usize::MAX),
            "the second gap's datagram arrived just now"
        );
        assert!(reorder.head_gap_overdue(fresh + timeout, timeout, usize::MAX));
    }

    #[test]
    fn discards_late_and_duplicate_datagrams() {
        let mut reorder = Reorder::new();
        assert_eq!(
            reorder.insert(datagram(0, 0, b"a")),
            InsertOutcome::Inserted
        );
        assert_eq!(
            reorder.insert(datagram(0, 0, b"a")),
            InsertOutcome::Duplicate
        );
        assert_eq!(deliver(&mut reorder).len(), 1);
        assert_eq!(reorder.insert(datagram(0, 0, b"a")), InsertOutcome::Late);
    }
}
