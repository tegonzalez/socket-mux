use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use log::{debug, warn};

use crate::header::{Header, FLAG_EOS, SEQUENCE_BITS, SEQUENCE_MASK};
use crate::queue::FreeList;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertOutcome {
    Inserted,
    Late,
    Duplicate,
    OutputDropped,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    sequence: u64,
    stream_offset: u64,
    payload_len: usize,
    buffer: Option<usize>,
    eos: bool,
    arrived: Instant,
}

/// A datagram delivered in sequence order and waiting for its TCP write.
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub sequence: u64,
    pub buffer: usize,
    pub payload_len: usize,
    pub eos: bool,
}

/// The RX memory pool: `W` datagram buffers that bound every held datagram,
/// plus `spares` receive buffers (one per lane thread, or one reserved buffer
/// in single-threaded use) that let a receiver read and drop the newest
/// datagram under the output limit without overwriting a held one. Allocated
/// once; buffers are addressed by index and never reallocated.
///
/// Ownership protocol: each index is owned by exactly one party at a time
/// (the free list, a lane thread, a hand-off queue, the reorder wheel or the
/// writer), and moves between parties only through the lock-free queues,
/// whose release/acquire ordering publishes the buffer contents.
pub struct Pool {
    base: NonNull<u8>,
    len: usize,
    datagram_size: usize,
}

// SAFETY: the pool is plain bytes; the ownership protocol above guarantees a
// buffer is never accessed by two threads at once.
unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}

impl Pool {
    pub fn new(datagram_size: usize, buffers: usize) -> Result<Self> {
        let len = datagram_size
            .checked_mul(buffers)
            .ok_or_else(|| anyhow!("RX memory pool size overflow"))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(len)?;
        bytes.resize(len, 0);
        let bytes = bytes.into_boxed_slice();
        let base = NonNull::new(Box::into_raw(bytes) as *mut u8).expect("box is non-null");
        Ok(Self {
            base,
            len,
            datagram_size,
        })
    }

    pub fn buffers(&self) -> usize {
        self.len / self.datagram_size
    }

    /// Buffer `index`, mutable.
    ///
    /// # Safety
    /// The caller must own `index` (see the type's ownership protocol).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slice_mut(&self, index: usize) -> &mut [u8] {
        assert!(index < self.buffers());
        std::slice::from_raw_parts_mut(
            self.base.as_ptr().add(index * self.datagram_size),
            self.datagram_size,
        )
    }

    /// Buffer `index`, shared.
    ///
    /// # Safety
    /// The caller must own `index`, or hold it while nobody writes it.
    pub unsafe fn slice(&self, index: usize) -> &[u8] {
        assert!(index < self.buffers());
        std::slice::from_raw_parts(
            self.base.as_ptr().add(index * self.datagram_size),
            self.datagram_size,
        )
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        // SAFETY: base/len came from Box::into_raw of a boxed slice of len bytes.
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                self.base.as_ptr(),
                self.len,
            )));
        }
    }
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool")
            .field("buffers", &self.buffers())
            .field("datagram_size", &self.datagram_size)
            .finish()
    }
}

#[derive(Debug)]
pub struct Reorder {
    capacity: usize,
    pool: Arc<Pool>,
    free: Arc<FreeList>,
    wheel: Vec<Option<Slot>>,
    /// Delivered datagrams not yet written; the first `handed` are with the
    /// writer thread. Kept until written so a duplicate of one is recognised.
    output: VecDeque<Output>,
    handed: usize,
    next: u64,
    initialized: bool,
    start_pending: bool,
    first_arrival: Option<Instant>,
    predecessor_arrival: Option<Instant>,
    predecessor_offset: u64,
    stream_ended: bool,
    total_lost_slices: u64,
    total_lost_bytes: u64,
    /// Occupied wheel slots; lets the nearest-slot search stop early.
    occupied: usize,
    gap_log: GapLog,
    /// Lost slices by cause: memory, output, timeout.
    lost_by_cause: [u64; 3],
}

impl std::fmt::Debug for FreeList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FreeList").field("len", &self.len()).finish()
    }
}

/// Loss-gap log rate limit: at most GAP_LINES_PER_WINDOW lines per window; the
/// rest are summed and reported in one summary line, so a loss burst cannot
/// make RX log-bound. Totals are always exact.
const GAP_LINES_PER_WINDOW: u32 = 20;
const GAP_LOG_WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
struct GapLog {
    window_start: Option<Instant>,
    lines: u32,
    suppressed_gaps: u64,
    suppressed_slices: u64,
    suppressed_bytes: u64,
}

impl Reorder {
    /// A reorder with W = `capacity` buffers and one reserved receive buffer
    /// (index `capacity`), for single-threaded use and tests.
    #[cfg(test)]
    pub fn new(datagram_size: usize, capacity: usize) -> Result<Self> {
        Self::with_spares(datagram_size, capacity, 1)
    }

    /// A reorder whose pool has W = `capacity` buffers on the free list plus
    /// `spares` receive buffers (indices W..W+spares) owned by receivers.
    pub fn with_spares(datagram_size: usize, capacity: usize, spares: usize) -> Result<Self> {
        let buffers = capacity
            .checked_add(spares)
            .ok_or_else(|| anyhow!("RX memory pool size overflow"))?;
        if buffers > u32::MAX as usize {
            return Err(anyhow!("RX memory pool has too many buffers"));
        }
        let pool = Arc::new(Pool::new(datagram_size, buffers)?);
        let free = Arc::new(FreeList::new(buffers, 0..capacity as u32));
        let mut wheel = Vec::new();
        wheel.try_reserve_exact(capacity)?;
        wheel.resize(capacity, None);
        let mut output = VecDeque::new();
        output.try_reserve(capacity)?;
        Ok(Self {
            capacity,
            pool,
            free,
            wheel,
            output,
            handed: 0,
            next: 0,
            initialized: false,
            start_pending: false,
            first_arrival: None,
            predecessor_arrival: None,
            predecessor_offset: 0,
            stream_ended: false,
            total_lost_slices: 0,
            total_lost_bytes: 0,
            occupied: 0,
            gap_log: GapLog::default(),
            lost_by_cause: [0; 3],
        })
    }

    pub fn pool(&self) -> &Arc<Pool> {
        &self.pool
    }

    pub fn free_list(&self) -> &Arc<FreeList> {
        &self.free
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[cfg(test)]
    pub fn take_receive_buffer(&mut self) -> Option<usize> {
        self.free.take().map(|index| index as usize)
    }

    #[cfg(test)]
    pub fn reserved_receive_buffer(&self) -> usize {
        self.capacity
    }

    pub fn release_receive_buffer(&mut self, buffer: usize) {
        self.free.give(buffer as u32);
    }

    #[cfg(test)]
    pub fn buffer_mut(&mut self, buffer: usize) -> &mut [u8] {
        // SAFETY: single-threaded use; &mut self owns every buffer it handles.
        unsafe { self.pool.slice_mut(buffer) }
    }

    pub fn accept(
        &mut self,
        header: Header,
        buffer: Option<usize>,
        arrived: Instant,
    ) -> Result<InsertOutcome> {
        let sequence = header.sequence & SEQUENCE_MASK;
        if self.stream_ended {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            return Ok(InsertOutcome::Late);
        }
        if !self.initialized {
            self.initialized = true;
            self.first_arrival = Some(arrived);
            let half = 1_u64 << (SEQUENCE_BITS - 1);
            self.next = if sequence > half { sequence } else { 0 };
            self.start_pending = sequence != 0 && self.capacity > 1;
        }

        if self.start_pending {
            let initial_distance = self.forward_distance(self.next, sequence);
            let half = 1_u64 << (SEQUENCE_BITS - 1);
            if initial_distance > half {
                self.next = sequence;
            }
            if sequence == 0 && (initial_distance == 0 || initial_distance > half) {
                self.start_pending = false;
            }
        }
        if self.in_output(sequence) {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            debug!(target: "socket_mux::rx", "discarding duplicate datagram: seq={sequence}");
            return Ok(InsertOutcome::Duplicate);
        }
        let mut distance = self.forward_distance(self.next, sequence);
        let half = 1_u64 << (SEQUENCE_BITS - 1);
        if distance == half {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            return Err(anyhow!(
                "ambiguous sequence gap: next={} received={sequence}",
                self.next
            ));
        }
        if distance > half {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            debug!(target: "socket_mux::rx", "discarding late datagram: seq={sequence}");
            return Ok(InsertOutcome::Late);
        }

        let mut slot_index = sequence as usize % self.capacity;
        if self.wheel[slot_index].is_some_and(|slot| slot.sequence == sequence) {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            debug!(target: "socket_mux::rx", "discarding duplicate datagram: seq={sequence}");
            return Ok(InsertOutcome::Duplicate);
        }

        while distance >= self.capacity as u64 {
            self.make_memory_room(sequence, header.stream_offset, arrived)?;
            distance = self.forward_distance(self.next, sequence);
            if distance >= half {
                if let Some(buffer) = buffer {
                    self.release_receive_buffer(buffer);
                }
                return Err(anyhow!(
                    "ambiguous sequence gap: next={} received={sequence}",
                    self.next
                ));
            }
        }
        slot_index = sequence as usize % self.capacity;
        if self.wheel[slot_index].is_some() {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
            return Err(anyhow!("RX sequence wheel slot collision at {sequence}"));
        }

        let output_dropped = buffer.is_none();
        self.occupied += 1;
        self.wheel[slot_index] = Some(Slot {
            sequence,
            stream_offset: header.stream_offset,
            payload_len: usize::from(header.payload_len),
            buffer,
            eos: header.flags & FLAG_EOS != 0,
            arrived,
        });
        self.progress();
        Ok(if output_dropped {
            InsertOutcome::OutputDropped
        } else {
            InsertOutcome::Inserted
        })
    }

    fn forward_distance(&self, from: u64, to: u64) -> u64 {
        to.wrapping_sub(from) & SEQUENCE_MASK
    }

    fn make_memory_room(
        &mut self,
        incoming: u64,
        incoming_offset: u64,
        arrived: Instant,
    ) -> Result<()> {
        if self.start_pending {
            self.start_pending = false;
            self.progress();
        }
        let nearest = self.nearest_slot(1);
        if let Some((distance, slot)) = nearest {
            self.log_gap(
                "memory",
                self.next,
                distance,
                slot.stream_offset.wrapping_sub(self.predecessor_offset),
            );
            self.next = slot.sequence;
            self.predecessor_offset = slot.stream_offset;
        } else {
            let count = self.forward_distance(self.next, incoming);
            self.log_gap(
                "memory",
                self.next,
                count,
                incoming_offset.wrapping_sub(self.predecessor_offset),
            );
            self.next = incoming;
            self.predecessor_offset = incoming_offset;
            self.predecessor_arrival = Some(arrived);
        }
        self.progress();
        Ok(())
    }

    /// Nearest occupied slot at forward distance >= `min` from next, within
    /// half the sequence space. Walks forward from next and stops at the first
    /// hit, so the common case (a gap just behind waiting data) is O(gap), not
    /// O(W). During start-up, held sequences may lie behind next, so it scans.
    fn nearest_slot(&self, min: u64) -> Option<(u64, Slot)> {
        if self.occupied == 0 {
            return None;
        }
        let half = 1_u64 << (SEQUENCE_BITS - 1);
        if self.start_pending {
            let mut nearest: Option<(u64, Slot)> = None;
            for slot in self.wheel.iter().flatten().copied() {
                let distance = self.forward_distance(self.next, slot.sequence);
                if distance >= min && distance < half && nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, slot));
                }
            }
            return nearest;
        }
        // Outside start-up every held sequence s satisfies 0 <= s - next < W,
        // so slot (next + d) mod W, if occupied, holds sequence next + d.
        for distance in min..self.capacity as u64 {
            let sequence = self.next.wrapping_add(distance) & SEQUENCE_MASK;
            if let Some(slot) = self.wheel[sequence as usize % self.capacity] {
                if slot.sequence == sequence {
                    return Some((distance, slot));
                }
            }
        }
        None
    }

    /// Whether `sequence` is queued for TCP output. Output holds increasing
    /// sequences behind next, so a binary search on forward distance from the
    /// front finds it in O(log n) rather than scanning the queue per datagram.
    fn in_output(&self, sequence: u64) -> bool {
        let Some(front) = self.output.front() else {
            return false;
        };
        let target = self.forward_distance(front.sequence, sequence);
        let back = self.output.back().expect("front exists");
        if target > self.forward_distance(front.sequence, back.sequence) {
            return false;
        }
        self.output
            .binary_search_by_key(&target, |entry| self.forward_distance(front.sequence, entry.sequence))
            .is_ok()
    }

    fn log_gap(&mut self, cause: &str, first: u64, slices: u64, bytes: u64) {
        if slices == 0 {
            return;
        }
        self.total_lost_slices = self.total_lost_slices.wrapping_add(slices);
        self.total_lost_bytes = self.total_lost_bytes.wrapping_add(bytes);
        let cause_index = match cause {
            "memory" => 0,
            "output" => 1,
            _ => 2,
        };
        self.lost_by_cause[cause_index] += slices;
        let now = Instant::now();
        let log = &mut self.gap_log;
        if log
            .window_start
            .is_none_or(|start| now.saturating_duration_since(start) >= GAP_LOG_WINDOW)
        {
            log.window_start = Some(now);
            log.lines = 0;
            self.report_suppressed_gaps();
        }
        let log = &mut self.gap_log;
        if log.lines < GAP_LINES_PER_WINDOW {
            log.lines += 1;
            warn!(target: "socket_mux::rx",
                "loss gap: cause={cause} first_seq={first} lost_slices={slices} lost_bytes={bytes} total_lost_slices={} total_lost_bytes={}",
                self.total_lost_slices, self.total_lost_bytes);
        } else {
            log.suppressed_gaps += 1;
            log.suppressed_slices += slices;
            log.suppressed_bytes += bytes;
        }
    }

    /// Emit one summary line for gaps the rate limit held back, if any.
    pub fn report_suppressed_gaps(&mut self) {
        let log = &mut self.gap_log;
        if log.suppressed_gaps == 0 {
            return;
        }
        warn!(target: "socket_mux::rx",
            "loss gaps not logged individually: gaps={} lost_slices={} lost_bytes={} total_lost_slices={} total_lost_bytes={}",
            log.suppressed_gaps, log.suppressed_slices, log.suppressed_bytes,
            self.total_lost_slices, self.total_lost_bytes);
        log.suppressed_gaps = 0;
        log.suppressed_slices = 0;
        log.suppressed_bytes = 0;
    }

    fn progress(&mut self) {
        if self.start_pending {
            return;
        }
        loop {
            let index = self.next as usize % self.capacity;
            let Some(slot) = self.wheel[index] else {
                return;
            };
            if slot.sequence != self.next {
                return;
            }
            if slot.buffer.is_none() {
                let first = self.next;
                let mut slices = 0;
                let mut bytes = 0;
                loop {
                    let index = self.next as usize % self.capacity;
                    let Some(marker) = self.wheel[index] else {
                        break;
                    };
                    if marker.sequence != self.next || marker.buffer.is_some() {
                        break;
                    }
                    self.wheel[index] = None;
                    self.occupied -= 1;
                    self.next = self.next.wrapping_add(1) & SEQUENCE_MASK;
                    self.predecessor_offset =
                        marker.stream_offset.wrapping_add(marker.payload_len as u64);
                    self.predecessor_arrival = Some(marker.arrived);
                    slices += 1;
                    bytes += marker.payload_len as u64;
                }
                self.log_gap("output", first, slices, bytes);
                continue;
            }

            let slot = self.wheel[index].take().expect("slot was checked");
            self.occupied -= 1;
            let output = Output {
                sequence: slot.sequence,
                buffer: slot.buffer.expect("data slot has a buffer"),
                payload_len: slot.payload_len,
                eos: slot.eos,
            };
            self.output.push_back(output);
            self.next = self.next.wrapping_add(1) & SEQUENCE_MASK;
            self.predecessor_offset = slot.stream_offset.wrapping_add(slot.payload_len as u64);
            self.predecessor_arrival = Some(slot.arrived);
            if slot.eos {
                self.stream_ended = true;
                return;
            }
        }
    }

    pub fn next_deadline(
        &self,
        timeout: Duration,
        last_datagram: Option<Instant>,
    ) -> Option<Instant> {
        let nearest = self.nearest_slot(1);
        let gap = if self.start_pending {
            self.first_arrival
                .and_then(|base| base.checked_add(timeout))
        } else {
            nearest.and_then(|_| {
                self.predecessor_arrival
                    .or(self.first_arrival)?
                    .checked_add(timeout)
            })
        };
        let idle = last_datagram.and_then(|last| last.checked_add(timeout));
        match (gap, idle) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        }
    }

    pub fn expire_gap(&mut self, now: Instant, timeout: Duration) -> bool {
        if self.start_pending {
            let Some(first) = self.first_arrival else {
                return false;
            };
            if now.saturating_duration_since(first) < timeout {
                return false;
            }
            let mut initial: Option<(u64, Slot)> = None;
            for slot in self.wheel.iter().flatten().copied() {
                let distance = self.forward_distance(self.next, slot.sequence);
                if distance < (1_u64 << (SEQUENCE_BITS - 1))
                    && initial.is_none_or(|(best, _)| distance < best)
                {
                    initial = Some((distance, slot));
                }
            }
            if let Some((distance, slot)) = initial {
                if distance > 0 {
                    self.log_gap(
                        "timeout",
                        self.next,
                        distance,
                        slot.stream_offset.wrapping_sub(self.predecessor_offset),
                    );
                    self.next = slot.sequence;
                    self.predecessor_offset = slot.stream_offset;
                }
            }
            self.start_pending = false;
            self.progress();
            return true;
        }
        let nearest = self.nearest_slot(1);
        let Some((distance, slot)) = nearest else {
            return false;
        };
        let Some(base) = (if self.start_pending {
            self.first_arrival
        } else {
            self.predecessor_arrival.or(self.first_arrival)
        }) else {
            return false;
        };
        if now.saturating_duration_since(base) < timeout {
            return false;
        }
        self.log_gap(
            "timeout",
            self.next,
            distance,
            slot.stream_offset.wrapping_sub(self.predecessor_offset),
        );
        self.next = slot.sequence;
        self.predecessor_offset = slot.stream_offset;
        self.start_pending = false;
        self.progress();
        true
    }

    pub fn flush(&mut self) {
        self.start_pending = false;
        loop {
            self.progress();
            if self.stream_ended {
                return;
            }
            let nearest = self.nearest_slot(0);
            let Some((distance, slot)) = nearest else {
                return;
            };
            if distance > 0 {
                self.log_gap(
                    "timeout",
                    self.next,
                    distance,
                    slot.stream_offset.wrapping_sub(self.predecessor_offset),
                );
                self.next = slot.sequence;
                self.predecessor_offset = slot.stream_offset;
                self.start_pending = false;
            }
            self.progress();
            if self.stream_ended
                || (slot.eos && slot.sequence == (self.next.wrapping_sub(1) & SEQUENCE_MASK))
            {
                return;
            }
        }
    }

    /// The unwritten payload of the oldest output (single-threaded use).
    #[cfg(test)]
    pub fn output_slice(&self) -> Option<&[u8]> {
        let output = self.output.front()?;
        // SAFETY: the reorder owns buffers in its output queue.
        let buffer = unsafe { self.pool.slice(output.buffer) };
        Some(&buffer[crate::header::HEADER_LEN..crate::header::HEADER_LEN + output.payload_len])
    }

    /// The oldest output was written completely (single-threaded use);
    /// returns whether it was EOS.
    #[cfg(test)]
    pub fn complete_output(&mut self, written: usize) -> Option<bool> {
        assert_eq!(self.output.front()?.payload_len, written, "whole payloads only");
        let output = self.output.pop_front().expect("front was checked");
        self.free.give(output.buffer as u32);
        Some(output.eos)
    }

    /// The oldest delivered datagram not yet handed to the writer thread.
    pub fn unhanded(&self) -> Option<Output> {
        self.output.get(self.handed).copied()
    }

    /// The datagram from [`Reorder::unhanded`] is now with the writer.
    pub fn mark_handed(&mut self) {
        debug_assert!(self.handed < self.output.len());
        self.handed += 1;
    }

    /// The writer completed the oldest `count` handed datagrams and returned
    /// their buffers to the free list; forget them.
    pub fn retire(&mut self, count: usize) {
        debug_assert!(count <= self.handed);
        self.output.drain(..count);
        self.handed -= count;
    }

    /// Every delivered datagram is with the writer (or written).
    pub fn all_handed(&self) -> bool {
        self.handed == self.output.len()
    }

    #[cfg(test)]
    pub fn has_output(&self) -> bool {
        !self.output.is_empty()
    }

    pub fn stream_ended(&self) -> bool {
        self.stream_ended
    }

    pub fn total_lost_slices(&self) -> u64 {
        self.total_lost_slices
    }

    pub fn total_lost_bytes(&self) -> u64 {
        self.total_lost_bytes
    }
}

impl Drop for Reorder {
    fn drop(&mut self) {
        // Rate-limited gaps are never lost from the log: summarise at the end.
        self.report_suppressed_gaps();
        if self.initialized {
            let [memory, output, timeout] = self.lost_by_cause;
            log::info!(target: "socket_mux::rx",
                "rx totals: last_seq={} total_lost_slices={} total_lost_bytes={} lost_memory={memory} lost_output={output} lost_timeout={timeout}",
                self.next.wrapping_sub(1) & SEQUENCE_MASK, self.total_lost_slices, self.total_lost_bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(sequence: u64, offset: u64, length: u16, eos: bool) -> Header {
        Header::new(1, length, sequence, offset, eos)
    }

    fn accept_data(reorder: &mut Reorder, seq: u64, offset: u64, len: u16, at: Instant) {
        let buffer = reorder.take_receive_buffer().unwrap();
        assert_eq!(
            reorder
                .accept(header(seq, offset, len, false), Some(buffer), at)
                .unwrap(),
            InsertOutcome::Inserted
        );
    }

    #[test]
    fn wheel_orders_data_and_rejects_duplicate_and_late() {
        let mut reorder = Reorder::new(128, 4).unwrap();
        let now = Instant::now();
        accept_data(&mut reorder, 0, 0, 2, now);
        let buffer = reorder.take_receive_buffer().unwrap();
        assert_eq!(
            reorder
                .accept(header(2, 4, 2, false), Some(buffer), now)
                .unwrap(),
            InsertOutcome::Inserted
        );
        let buffer = reorder.take_receive_buffer().unwrap();
        assert_eq!(
            reorder
                .accept(header(2, 4, 2, false), Some(buffer), now)
                .unwrap(),
            InsertOutcome::Duplicate
        );
        assert!(reorder.output_slice().is_some());
        while reorder.has_output() {
            let len = reorder.output_slice().unwrap().len();
            reorder.complete_output(len);
        }
        let buffer = reorder.take_receive_buffer().unwrap();
        assert_eq!(
            reorder
                .accept(header(0, 0, 2, false), Some(buffer), now)
                .unwrap(),
            InsertOutcome::Late
        );
    }

    #[test]
    fn startup_waits_for_older_packets_then_declares_initial_loss() {
        let mut reorder = Reorder::new(128, 8).unwrap();
        let now = Instant::now();
        accept_data(&mut reorder, 5, 5, 1, now);
        assert!(reorder.output.is_empty());
        accept_data(&mut reorder, 4, 4, 1, now + Duration::from_millis(1));
        assert!(reorder.output.is_empty());
        assert!(reorder.expire_gap(now + Duration::from_secs(2), Duration::from_secs(2)));
        assert_eq!(
            reorder
                .output
                .iter()
                .map(|item| item.sequence)
                .collect::<Vec<_>>(),
            vec![4, 5]
        );
        assert_eq!(reorder.total_lost_slices(), 4);
        assert_eq!(reorder.total_lost_bytes(), 4);
    }

    #[test]
    fn startup_waits_for_older_packets_in_the_upper_half() {
        let mut reorder = Reorder::new(128, 8).unwrap();
        let now = Instant::now();
        let first = (1_u64 << (SEQUENCE_BITS - 1)) + 72;
        let timeout = Duration::from_secs(2);
        accept_data(&mut reorder, first, 1, 1, now);
        assert!(!reorder.has_output());
        accept_data(
            &mut reorder,
            first - 1,
            0,
            1,
            now + Duration::from_millis(1),
        );
        assert!(!reorder.has_output());
        assert_eq!(reorder.next_deadline(timeout, None), Some(now + timeout));
        assert!(!reorder.expire_gap(now + timeout - Duration::from_nanos(1), timeout));
        assert!(reorder.expire_gap(now + timeout, timeout));
        assert_eq!(
            reorder
                .output
                .iter()
                .map(|item| item.sequence)
                .collect::<Vec<_>>(),
            vec![first - 1, first]
        );
    }

    #[test]
    fn all_w_buffers_hold_output_and_only_the_newest_is_dropped() {
        for capacity in [1, 4] {
            let mut reorder = Reorder::new(128, capacity).unwrap();
            let now = Instant::now();
            for sequence in 0..capacity as u64 {
                let buffer = reorder.take_receive_buffer().unwrap();
                reorder.buffer_mut(buffer)[crate::header::HEADER_LEN] = b'A' + sequence as u8;
                assert_eq!(
                    reorder
                        .accept(header(sequence, sequence, 1, false), Some(buffer), now)
                        .unwrap(),
                    InsertOutcome::Inserted
                );
            }
            assert!(reorder.take_receive_buffer().is_none());
            let receive_target = reorder.reserved_receive_buffer();
            reorder.buffer_mut(receive_target).fill(b'!');
            assert_eq!(
                reorder
                    .accept(
                        header(capacity as u64, capacity as u64, 1, false),
                        None,
                        now
                    )
                    .unwrap(),
                InsertOutcome::OutputDropped
            );
            assert_eq!(reorder.total_lost_slices(), 1);
            assert_eq!(reorder.total_lost_bytes(), 1);
            for sequence in 0..capacity {
                assert_eq!(reorder.output_slice().unwrap(), &[b'A' + sequence as u8]);
                assert_eq!(reorder.complete_output(1), Some(false));
            }
            assert!(!reorder.has_output());
            accept_data(
                &mut reorder,
                capacity as u64 + 1,
                capacity as u64 + 1,
                1,
                now,
            );
            assert!(reorder.has_output());
        }
    }

    #[test]
    fn memory_window_declares_a_gap_and_accepts_the_deep_datagram() {
        let mut reorder = Reorder::new(128, 3).unwrap();
        let now = Instant::now();
        accept_data(&mut reorder, 0, 0, 2, now);
        let buffer = reorder.take_receive_buffer().unwrap();
        assert_eq!(
            reorder
                .accept(header(4, 8, 2, false), Some(buffer), now)
                .unwrap(),
            InsertOutcome::Inserted
        );
        assert_eq!(reorder.total_lost_slices(), 3);
        assert_eq!(reorder.total_lost_bytes(), 6);
    }

    #[test]
    fn output_loss_is_recorded_once_when_it_reaches_the_head() {
        let mut reorder = Reorder::new(128, 3).unwrap();
        let now = Instant::now();
        assert_eq!(
            reorder.accept(header(0, 0, 3, false), None, now).unwrap(),
            InsertOutcome::OutputDropped
        );
        assert_eq!(reorder.total_lost_slices(), 1);
        assert_eq!(reorder.total_lost_bytes(), 3);
    }

    #[test]
    fn serial_arithmetic_orders_across_sequence_rollover() {
        let mut reorder = Reorder::new(128, 4).unwrap();
        let now = Instant::now();
        reorder.initialized = true;
        reorder.next = SEQUENCE_MASK - 1;
        reorder.predecessor_arrival = Some(now);
        for (sequence, offset) in [(SEQUENCE_MASK - 1, 0), (SEQUENCE_MASK, 1), (0, 2)] {
            accept_data(&mut reorder, sequence, offset, 1, now);
        }
        assert_eq!(reorder.next, 1);
        assert_eq!(
            reorder
                .output
                .iter()
                .map(|item| item.sequence)
                .collect::<Vec<_>>(),
            vec![SEQUENCE_MASK - 1, SEQUENCE_MASK, 0]
        );
    }

    #[test]
    fn half_sequence_space_is_fatal_ambiguity() {
        let mut reorder = Reorder::new(128, 4).unwrap();
        reorder.initialized = true;
        reorder.next = 0;
        let now = Instant::now();
        let buffer = reorder.take_receive_buffer().unwrap();
        let error = reorder
            .accept(
                header(1_u64 << (SEQUENCE_BITS - 1), 0, 1, false),
                Some(buffer),
                now,
            )
            .unwrap_err();
        assert!(error.to_string().contains("ambiguous sequence gap"));
        assert_eq!(reorder.free.len(), 4);
    }

    #[test]
    fn timeout_deadline_uses_predecessor_arrival() {
        let mut reorder = Reorder::new(128, 4).unwrap();
        let now = Instant::now();
        accept_data(&mut reorder, 0, 0, 2, now);
        let buffer = reorder.take_receive_buffer().unwrap();
        reorder
            .accept(header(2, 4, 2, false), Some(buffer), now)
            .unwrap();
        let timeout = Duration::from_secs(2);
        assert_eq!(reorder.next_deadline(timeout, None), Some(now + timeout));
        assert!(!reorder.expire_gap(now + timeout - Duration::from_nanos(1), timeout));
        assert!(reorder.expire_gap(now + timeout, timeout));
        assert_eq!(reorder.total_lost_slices(), 1);
    }

    #[test]
    fn duplicate_of_queued_output_is_found_across_a_long_queue_and_rollover() {
        let start = SEQUENCE_MASK - 50; // the queue crosses the sequence rollover
        let mut reorder = Reorder::new(128, 256).unwrap();
        let now = Instant::now();
        // Force start-up to resolve so `start` is accepted as next.
        let mut seq = start;
        accept_data(&mut reorder, seq, 0, 1, now);
        assert!(reorder.expire_gap(now + Duration::from_secs(1), Duration::from_secs(1)));
        for i in 1..200_u64 {
            seq = start.wrapping_add(i) & SEQUENCE_MASK;
            accept_data(&mut reorder, seq, i, 1, now);
        }
        assert_eq!(reorder.output.len(), 200);
        for probe in [0_u64, 1, 50, 51, 120, 199] {
            let sequence = start.wrapping_add(probe) & SEQUENCE_MASK;
            assert!(reorder.in_output(sequence), "probe {probe}");
            let buffer = reorder.take_receive_buffer().unwrap();
            assert_eq!(
                reorder.accept(header(sequence, probe, 1, false), Some(buffer), now).unwrap(),
                InsertOutcome::Duplicate
            );
        }
        assert!(!reorder.in_output(start.wrapping_add(200) & SEQUENCE_MASK));
        assert!(!reorder.in_output(start.wrapping_sub(1) & SEQUENCE_MASK));
    }

    #[test]
    fn nearest_slot_matches_a_full_scan() {
        let capacity = 64;
        let mut reorder = Reorder::new(64, capacity).unwrap();
        let now = Instant::now();
        accept_data(&mut reorder, 0, 0, 1, now);
        // Hold a sparse, irregular set ahead of a gap at next.
        let mut held = vec![];
        let mut x = 7_u64;
        for _ in 0..20 {
            x = (x * 37 + 11) % (capacity as u64 - 2) + 2;
            if !held.contains(&x) {
                held.push(x);
                accept_data(&mut reorder, x, x, 1, now);
            }
        }
        let brute = |reorder: &Reorder, min: u64| {
            reorder
                .wheel
                .iter()
                .flatten()
                .map(|slot| reorder.forward_distance(reorder.next, slot.sequence))
                .filter(|&d| d >= min)
                .min()
        };
        for min in [0, 1, 5, 30] {
            assert_eq!(reorder.nearest_slot(min).map(|(d, _)| d), brute(&reorder, min), "min {min}");
        }
        // Drain gap by gap; the fast search must agree at every step.
        while reorder.expire_gap(now + Duration::from_secs(9), Duration::from_secs(1)) {
            assert_eq!(reorder.nearest_slot(1).map(|(d, _)| d), brute(&reorder, 1));
        }
        assert_eq!(reorder.occupied, 0);
        assert!(reorder.nearest_slot(0).is_none());
    }

    #[test]
    fn gap_log_rate_limit_keeps_totals_exact() {
        let mut reorder = Reorder::new(64, 8).unwrap();
        let gaps = u64::from(GAP_LINES_PER_WINDOW) + 500;
        for i in 0..gaps {
            reorder.log_gap("output", i * 2, 1, 10);
        }
        assert_eq!(reorder.total_lost_slices(), gaps);
        assert_eq!(reorder.total_lost_bytes(), gaps * 10);
        assert_eq!(reorder.gap_log.lines, GAP_LINES_PER_WINDOW);
        assert_eq!(reorder.gap_log.suppressed_gaps, 500);
        assert_eq!(reorder.gap_log.suppressed_slices, 500);
        assert_eq!(reorder.gap_log.suppressed_bytes, 5000);
        reorder.report_suppressed_gaps();
        assert_eq!(reorder.gap_log.suppressed_gaps, 0);
        assert_eq!(reorder.total_lost_slices(), gaps);
        assert_eq!(reorder.lost_by_cause, [0, gaps, 0]);
    }
}
