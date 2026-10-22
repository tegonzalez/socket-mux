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
    first_arrival: Option<Instant>,
    previous_arrival: Option<Instant>,
    total_lost_slices: u64,
    total_lost_bytes: u64,
}

impl Reorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert timed by the current monotonic clock; unit tests use this when the
    /// arrival instant does not matter. RX uses `insert_at` with the arrival.
    #[cfg(test)]
    pub fn insert(&mut self, datagram: Datagram) -> InsertOutcome {
        self.insert_at(datagram, Instant::now())
    }

    /// Insert with an explicit monotonic arrival time; tests use injected instants.
    pub fn insert_at(&mut self, datagram: Datagram, arrived: Instant) -> InsertOutcome {
        self.first_arrival.get_or_insert(arrived);
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
        self.previous_arrival = Some(pending.arrived);
        self.next_offset =
            pending.datagram.header.stream_offset + pending.datagram.payload.len() as u64;
        Some(Delivery {
            end_of_stream: pending.datagram.header.flags & FLAG_EOS != 0,
            payload: pending.datagram.payload,
        })
    }

    /// SM-D19 rule A: a head-of-line gap is overdue when the datagram before it
    /// arrived at least `timeout` ago. PROPOSAL SM-D21: a start-of-stream gap is
    /// measured from the first arrival instead. OD-22 open: the provisional memory
    /// cap can declare the gap sooner.
    pub fn head_gap_overdue(&self, now: Instant, timeout: Duration, memory_cap: usize) -> bool {
        let Some(arrived) = self.head_gap_base() else {
            return false;
        };
        self.pending_bytes > memory_cap || now.saturating_duration_since(arrived) >= timeout
    }

    /// Arrival from which the current head-of-line gap's inter-packet delay is
    /// measured: the delivered predecessor, or the first arrival for sequence 0.
    fn head_gap_base(&self) -> Option<Instant> {
        let (&sequence, _) = self.pending.iter().next()?;
        if sequence <= self.next_sequence {
            return None;
        }
        self.previous_arrival.or(self.first_arrival)
    }

    pub fn total_lost_slices(&self) -> u64 {
        self.total_lost_slices
    }

    pub fn total_lost_bytes(&self) -> u64 {
        self.total_lost_bytes
    }

    /// The instant at which the head-of-line gap becomes overdue under rule A;
    /// `None` when there is no gap. The OD-22 memory cap makes it due at once.
    pub fn head_gap_deadline(&self, timeout: Duration, memory_cap: usize) -> Option<Instant> {
        let arrived = self.head_gap_base()?;
        if self.pending_bytes > memory_cap {
            return Some(arrived); // already overdue: a past instant wakes at once
        }
        Some(arrived + timeout)
    }

    /// PROPOSAL SM-D21: the one receive-loop deadline is the earlier of rule A's
    /// head-gap expiry and rule B's expiry after the last arrival on any leg.
    pub fn next_deadline(
        &self,
        timeout: Duration,
        memory_cap: usize,
        last_datagram: Option<Instant>,
    ) -> Option<Instant> {
        let gap = self.head_gap_deadline(timeout, memory_cap);
        let idle = last_datagram.map(|last| last + timeout);
        match (gap, idle) {
            (Some(gap), Some(idle)) => Some(gap.min(idle)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        }
    }

    /// SM-D19: end-of-stream flush — declare every outstanding gap, then deliver
    /// everything still held, in order. Used when the global timeout ends the
    /// stream; returns the gaps (for logging) and the deliveries.
    pub fn flush(&mut self) -> (Vec<LossGap>, Vec<Delivery>) {
        let mut gaps = Vec::new();
        let mut deliveries = Vec::new();
        // Alternate: `declare_loss` sees only the gap before the first pending
        // datagram, so deliver what each declaration unblocks before the next.
        loop {
            while let Some(delivery) = self.pop_ready() {
                let end_of_stream = delivery.end_of_stream;
                deliveries.push(delivery);
                if end_of_stream {
                    return (gaps, deliveries);
                }
            }
            match self.declare_loss() {
                Some(gap) => gaps.push(gap),
                None => break,
            }
        }
        (gaps, deliveries)
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
    fn worked_example_expires_one_three_slice_gap_after_its_predecessor() {
        let mut reorder = Reorder::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(10);
        for sequence in [0, 1, 2, 6, 7] {
            reorder.insert_at(datagram(sequence, sequence * 2, b"xy"), start);
        }
        assert_eq!(deliver(&mut reorder).len(), 3);

        let tick = Duration::from_nanos(1);
        assert!(!reorder.head_gap_overdue(start + timeout - tick, timeout, 1024));
        assert!(reorder.head_gap_overdue(start + timeout + tick, timeout, 1024));
        assert_eq!(
            reorder.declare_loss(),
            Some(LossGap {
                first_sequence: 3,
                lost_slices: 3,
                lost_bytes: 6,
                total_lost_slices: 3,
                total_lost_bytes: 6,
            })
        );
        assert_eq!(deliver(&mut reorder).len(), 2, "delivery resumes at 6");
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
    fn start_of_stream_gap_is_timed_from_the_first_arrival() {
        let mut reorder = Reorder::new();
        let first = Instant::now();
        let later = first + Duration::from_millis(400);
        let timeout = Duration::from_secs(1);
        reorder.insert_at(datagram(2, 4, b"ef"), first);
        reorder.insert_at(datagram(1, 2, b"cd"), later);

        assert_eq!(
            reorder.head_gap_deadline(timeout, usize::MAX),
            Some(first + timeout)
        );
        assert!(!reorder.head_gap_overdue(
            first + timeout - Duration::from_nanos(1),
            timeout,
            usize::MAX
        ));
        assert!(reorder.head_gap_overdue(
            first + timeout + Duration::from_nanos(1),
            timeout,
            usize::MAX
        ));
        assert_eq!(reorder.declare_loss().unwrap().first_sequence, 0);
        assert_eq!(deliver(&mut reorder).len(), 2);
    }

    #[test]
    fn one_deadline_is_the_earlier_of_rule_a_and_rule_b() {
        let mut reorder = Reorder::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(10);
        assert_eq!(reorder.next_deadline(timeout, usize::MAX, None), None);
        reorder.insert_at(datagram(0, 0, b"ab"), start);
        reorder.insert_at(datagram(2, 4, b"ef"), start);
        assert_eq!(deliver(&mut reorder).len(), 1);

        let rule_a = start + timeout;
        let later_rule_b = start + timeout * 2;
        assert_eq!(
            reorder.next_deadline(timeout, usize::MAX, Some(start + timeout)),
            Some(rule_a)
        );
        // Inject the two clock inputs independently to exercise the other order.
        let earlier_rule_b = start;
        assert_eq!(
            reorder.next_deadline(timeout, usize::MAX, Some(start - timeout)),
            Some(earlier_rule_b)
        );
        assert_eq!(later_rule_b, start + timeout * 2);
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
    fn gaps_whose_predecessors_are_old_are_all_overdue_in_one_pass() {
        // The M09/M10 shape: a dead leg leaves hundreds of gaps whose delivered
        // predecessors all arrived long ago; declare and deliver each in one pass.
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
    fn next_gap_is_timed_from_the_datagram_before_that_gap() {
        let mut reorder = Reorder::new();
        let first = Instant::now();
        let second = first + Duration::from_millis(600);
        let timeout = Duration::from_secs(1);
        reorder.insert_at(datagram(0, 0, b"ab"), first);
        reorder.insert_at(datagram(2, 4, b"ef"), second);
        reorder.insert_at(datagram(4, 8, b"ij"), second);
        assert_eq!(deliver(&mut reorder).len(), 1);

        assert!(reorder.head_gap_overdue(first + timeout, timeout, usize::MAX));
        assert_eq!(reorder.declare_loss().unwrap().first_sequence, 1);
        assert_eq!(deliver(&mut reorder).len(), 1, "sequence 2 is delivered");
        assert!(
            !reorder.head_gap_overdue(first + timeout, timeout, usize::MAX),
            "gap 3 gets sequence 2's full inter-packet delay"
        );
        assert_eq!(
            reorder.head_gap_deadline(timeout, usize::MAX),
            Some(second + timeout)
        );
        assert!(reorder.head_gap_overdue(second + timeout, timeout, usize::MAX));
        assert_eq!(reorder.declare_loss().unwrap().first_sequence, 3);
    }

    #[test]
    fn flush_declares_every_gap_and_delivers_everything_held_in_order() {
        // SM-D19: the global timeout injects end of stream; every outstanding gap
        // is declared and every held datagram is delivered, in order, through EOS.
        let mut reorder = Reorder::new();
        reorder.insert(datagram(0, 0, b"ab"));
        reorder.insert(datagram(2, 4, b"ef")); // gap: sequence 1
        reorder.insert(datagram(3, 6, b"gh"));
        reorder.insert(datagram(6, 12, b"mn")); // gap: sequences 4 to 5
        reorder.insert(Datagram {
            header: Header::new(0, 7, 14, true),
            payload: Bytes::new(),
        });
        assert_eq!(deliver(&mut reorder), vec![Bytes::from_static(b"ab")]);

        let (gaps, deliveries) = reorder.flush();
        assert_eq!(gaps.len(), 2);
        assert_eq!(
            (
                gaps[0].first_sequence,
                gaps[0].lost_slices,
                gaps[0].lost_bytes
            ),
            (1, 1, 2)
        );
        assert_eq!(
            (
                gaps[1].first_sequence,
                gaps[1].lost_slices,
                gaps[1].lost_bytes
            ),
            (4, 2, 4)
        );
        assert_eq!(gaps[1].total_lost_slices, 3);
        assert_eq!(gaps[1].total_lost_bytes, 6);
        assert_eq!(
            deliveries
                .iter()
                .map(|delivery| &delivery.payload[..])
                .collect::<Vec<_>>(),
            vec![&b"ef"[..], b"gh", b"mn", b""]
        );
        assert!(deliveries.last().unwrap().end_of_stream);
        assert!(reorder.pop_ready().is_none(), "flush leaves nothing held");
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
