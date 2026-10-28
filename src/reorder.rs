use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use log::{debug, warn};

use crate::header::{Header, FLAG_EOS, SEQUENCE_BITS, SEQUENCE_MASK};

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

#[derive(Clone, Copy, Debug)]
struct Output {
    sequence: u64,
    buffer: usize,
    payload_len: usize,
    written: usize,
    eos: bool,
}

#[derive(Debug)]
pub struct Reorder {
    datagram_size: usize,
    capacity: usize,
    buffers: Vec<u8>,
    free: Vec<usize>,
    wheel: Vec<Option<Slot>>,
    output: VecDeque<Output>,
    next: u64,
    initialized: bool,
    start_pending: bool,
    first_arrival: Option<Instant>,
    predecessor_arrival: Option<Instant>,
    predecessor_offset: u64,
    stream_ended: bool,
    total_lost_slices: u64,
    total_lost_bytes: u64,
}

impl Reorder {
    pub fn new(datagram_size: usize, capacity: usize) -> Result<Self> {
        let pool_size = datagram_size
            .checked_mul(capacity)
            .ok_or_else(|| anyhow!("RX memory pool size overflow"))?;
        let mut buffers = Vec::new();
        buffers.try_reserve_exact(pool_size)?;
        buffers.resize(pool_size, 0);
        let mut free = Vec::new();
        free.try_reserve_exact(capacity)?;
        let first_buffer = usize::from(capacity > 1);
        free.extend((first_buffer..capacity).rev());
        let mut wheel = Vec::new();
        wheel.try_reserve_exact(capacity)?;
        wheel.resize(capacity, None);
        let mut output = VecDeque::new();
        output.try_reserve(capacity)?;
        Ok(Self {
            datagram_size,
            capacity,
            buffers,
            free,
            wheel,
            output,
            next: 0,
            initialized: false,
            start_pending: false,
            first_arrival: None,
            predecessor_arrival: None,
            predecessor_offset: 0,
            stream_ended: false,
            total_lost_slices: 0,
            total_lost_bytes: 0,
        })
    }

    pub fn take_receive_buffer(&mut self) -> Option<usize> {
        self.free.pop()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn reserved_receive_buffer(&self) -> usize {
        0
    }

    pub fn release_receive_buffer(&mut self, buffer: usize) {
        self.free.push(buffer);
    }

    pub fn buffer_mut(&mut self, buffer: usize) -> &mut [u8] {
        let start = buffer * self.datagram_size;
        &mut self.buffers[start..start + self.datagram_size]
    }

    pub fn buffer(&self, buffer: usize) -> &[u8] {
        let start = buffer * self.datagram_size;
        &self.buffers[start..start + self.datagram_size]
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
            self.start_pending = sequence != self.next && self.capacity > 1;
        }

        if self.start_pending {
            let initial_distance = self.forward_distance(self.next, sequence);
            let half = 1_u64 << (SEQUENCE_BITS - 1);
            if sequence == 0 || (initial_distance > half && initial_distance != half) {
                self.next = sequence;
                self.start_pending = false;
            }
        }
        if self.output.iter().any(|entry| entry.sequence == sequence) {
            if let Some(buffer) = buffer {
                self.release_receive_buffer(buffer);
            }
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
        let mut nearest: Option<(u64, Slot)> = None;
        for slot in self.wheel.iter().flatten().copied() {
            let distance = self.forward_distance(self.next, slot.sequence);
            if distance == 0 {
                continue;
            }
            if distance < self.capacity as u64 && nearest.is_none_or(|(best, _)| distance < best) {
                nearest = Some((distance, slot));
            }
        }
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

    fn log_gap(&mut self, cause: &str, first: u64, slices: u64, bytes: u64) {
        if slices == 0 {
            return;
        }
        self.total_lost_slices = self.total_lost_slices.wrapping_add(slices);
        self.total_lost_bytes = self.total_lost_bytes.wrapping_add(bytes);
        warn!(target: "socket_mux::rx",
            "loss gap: cause={cause} first_seq={first} lost_slices={slices} lost_bytes={bytes} total_lost_slices={} total_lost_bytes={}",
            self.total_lost_slices, self.total_lost_bytes);
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
            let output = Output {
                sequence: slot.sequence,
                buffer: slot.buffer.expect("data slot has a buffer"),
                payload_len: slot.payload_len,
                written: 0,
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
        let mut nearest = None;
        for slot in self.wheel.iter().flatten().copied() {
            let distance = self.forward_distance(self.next, slot.sequence);
            if distance > 0
                && distance < (1_u64 << (SEQUENCE_BITS - 1))
                && nearest.is_none_or(|(best, _)| distance < best)
            {
                nearest = Some((distance, slot));
            }
        }
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
        let mut nearest: Option<(u64, Slot)> = None;
        for slot in self.wheel.iter().flatten().copied() {
            let distance = self.forward_distance(self.next, slot.sequence);
            if distance > 0
                && distance < (1_u64 << (SEQUENCE_BITS - 1))
                && nearest.is_none_or(|(best, _)| distance < best)
            {
                nearest = Some((distance, slot));
            }
        }
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
            let mut nearest: Option<(u64, Slot)> = None;
            for slot in self.wheel.iter().flatten().copied() {
                let distance = self.forward_distance(self.next, slot.sequence);
                if distance < (1_u64 << (SEQUENCE_BITS - 1))
                    && nearest.is_none_or(|(best, _)| distance < best)
                {
                    nearest = Some((distance, slot));
                }
            }
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

    pub fn output_slice(&self) -> Option<&[u8]> {
        let output = self.output.front()?;
        let start = output.buffer * self.datagram_size + crate::header::HEADER_LEN + output.written;
        Some(&self.buffers[start..start + output.payload_len - output.written])
    }

    pub fn complete_output(&mut self, written: usize) -> Option<bool> {
        let front = self.output.front_mut()?;
        front.written += written;
        if front.written != front.payload_len {
            return Some(false);
        }
        let output = self.output.pop_front().expect("front was checked");
        self.free.push(output.buffer);
        Some(output.eos)
    }

    pub fn finish_empty_output(&mut self) -> Option<bool> {
        if self.output.front()?.payload_len != 0 {
            return None;
        }
        let output = self.output.pop_front().expect("front was checked");
        self.free.push(output.buffer);
        Some(output.eos)
    }

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
        assert_eq!(reorder.free.len(), 3);
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
}
