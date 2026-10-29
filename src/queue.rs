//! Bounded lock-free hand-off queues and the buffer free list.
//!
//! Both wrap `crossbeam_queue::ArrayQueue` (a fixed-capacity lock-free ring,
//! allocated once). Waiting on an empty or full queue follows the shared wait
//! policy ([`crate::wait`]); producers and consumers notify the matching
//! [`Signal`] so a parked peer wakes at once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_queue::ArrayQueue;

use crate::wait::{wait_until, Signal};

/// A bounded multi-producer multi-consumer queue with wake-ups.
///
/// `not_empty` is notified after every push and `not_full` after every pop.
/// Several queues may share one signal, so one thread can wait for any of
/// them (the RX reorder thread waits for arrivals *or* writer progress).
pub struct Bounded<T> {
    queue: ArrayQueue<T>,
    not_empty: Arc<Signal>,
    not_full: Arc<Signal>,
    closed: AtomicBool,
}

impl<T> Bounded<T> {
    pub fn new(capacity: usize) -> Self {
        Self::with_signals(capacity, Arc::new(Signal::new()), Arc::new(Signal::new()))
    }

    pub fn with_signals(capacity: usize, not_empty: Arc<Signal>, not_full: Arc<Signal>) -> Self {
        Self {
            queue: ArrayQueue::new(capacity.max(1)),
            not_empty,
            not_full,
            closed: AtomicBool::new(false),
        }
    }

    /// Push without waiting; gives the value back when the queue is full.
    pub fn try_push(&self, value: T) -> Result<(), T> {
        self.queue.push(value)?;
        self.not_empty.notify();
        Ok(())
    }

    /// Push, waiting (wait policy) while the queue is full. Gives the value
    /// back if `stop` is set or the queue is closed before room appears.
    pub fn push(&self, mut value: T, stop: &AtomicBool) -> Result<(), T> {
        loop {
            if stop.load(Ordering::Relaxed) || self.is_closed() {
                return Err(value);
            }
            match self.try_push(value) {
                Ok(()) => return Ok(()),
                Err(back) => value = back,
            }
            wait_until(&self.not_full, None, || {
                !self.queue.is_full() || stop.load(Ordering::Relaxed) || self.is_closed()
            });
        }
    }

    /// Pop without waiting.
    pub fn try_pop(&self) -> Option<T> {
        let value = self.queue.pop()?;
        self.not_full.notify();
        Some(value)
    }

    /// Pop, waiting (wait policy) while empty. None once the queue is closed
    /// and drained, or as soon as `stop` is set (queued items are abandoned).
    pub fn pop(&self, stop: &AtomicBool) -> Option<T> {
        loop {
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            if let Some(value) = self.try_pop() {
                return Some(value);
            }
            if self.is_closed() && self.queue.is_empty() {
                return None;
            }
            wait_until(&self.not_empty, None, || {
                !self.queue.is_empty() || stop.load(Ordering::Relaxed) || self.is_closed()
            });
        }
    }

    /// No more pushes will come; consumers drain what is queued and stop.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.not_empty.notify();
        self.not_full.notify();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.queue.is_full()
    }

    pub fn not_empty(&self) -> &Arc<Signal> {
        &self.not_empty
    }

    pub fn not_full(&self) -> &Arc<Signal> {
        &self.not_full
    }
}

/// Indices of free pool buffers. Any thread may take or return a buffer; an
/// index is owned by exactly one party between a `take` and a `give`. Nobody
/// waits on it: RX lanes that find it empty drop the newest datagram instead
/// (the output limit).
pub struct FreeList {
    indices: ArrayQueue<u32>,
}

impl FreeList {
    /// A list able to hold `capacity` indices, initially holding `initial`.
    pub fn new(capacity: usize, initial: impl IntoIterator<Item = u32>) -> Self {
        let indices = ArrayQueue::new(capacity.max(1));
        for index in initial {
            indices
                .push(index)
                .expect("initial free buffers fit the free list");
        }
        Self { indices }
    }

    pub fn take(&self) -> Option<u32> {
        self.indices.pop()
    }

    pub fn give(&self, index: u32) {
        // Capacity covers every buffer of the pool, so this cannot fail unless
        // an index is returned twice, which is a logic error.
        self.indices
            .push(index)
            .expect("pool buffer returned to the free list twice");
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn bounded_queue_delivers_every_item_once_across_producers() {
        let queue = Arc::new(Bounded::<u64>::new(64));
        let stop = Arc::new(AtomicBool::new(false));
        let producers = 4_u64;
        let per = 50_000_u64;
        let handles: Vec<_> = (0..producers)
            .map(|p| {
                let (queue, stop) = (queue.clone(), stop.clone());
                thread::spawn(move || {
                    for i in 0..per {
                        queue.push(p * per + i, &stop).unwrap();
                    }
                })
            })
            .collect();
        let consumer = {
            let (queue, stop) = (queue.clone(), stop.clone());
            thread::spawn(move || {
                let mut seen = vec![false; (producers * per) as usize];
                let mut last = vec![None::<u64>; producers as usize];
                while let Some(v) = queue.pop(&stop) {
                    assert!(!seen[v as usize], "duplicate {v}");
                    seen[v as usize] = true;
                    // FIFO per producer.
                    let p = (v / per) as usize;
                    assert!(last[p].is_none_or(|l| l < v), "out of order {v}");
                    last[p] = Some(v);
                }
                seen.iter().all(|s| *s)
            })
        };
        for handle in handles {
            handle.join().unwrap();
        }
        queue.close();
        assert!(consumer.join().unwrap(), "every item delivered");
    }

    #[test]
    fn stop_releases_a_blocked_pusher_and_popper() {
        let queue = Arc::new(Bounded::<u8>::new(1));
        let stop = Arc::new(AtomicBool::new(false));
        queue.try_push(1).unwrap();
        let pusher = {
            let (queue, stop) = (queue.clone(), stop.clone());
            thread::spawn(move || queue.push(2, &stop))
        };
        let empty = Arc::new(Bounded::<u8>::new(1));
        let popper = {
            let (empty, stop) = (empty.clone(), stop.clone());
            thread::spawn(move || empty.pop(&stop))
        };
        thread::sleep(std::time::Duration::from_millis(30));
        stop.store(true, Ordering::Relaxed);
        queue.not_full().notify();
        empty.not_empty().notify();
        assert_eq!(pusher.join().unwrap(), Err(2));
        assert_eq!(popper.join().unwrap(), None);
    }

    #[test]
    fn free_list_neither_loses_nor_duplicates_buffers_under_concurrency() {
        let buffers = 256_u32;
        let free = Arc::new(FreeList::new(buffers as usize, 0..buffers));
        // Each thread repeatedly takes a buffer, marks it owned, and returns it.
        let owner: Arc<Vec<AtomicBool>> =
            Arc::new((0..buffers).map(|_| AtomicBool::new(false)).collect());
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (free, owner) = (free.clone(), owner.clone());
                thread::spawn(move || {
                    let mut held = Vec::with_capacity(8);
                    for round in 0..40_000 {
                        if let Some(index) = free.take() {
                            let was = owner[index as usize].swap(true, Ordering::AcqRel);
                            assert!(!was, "buffer {index} handed out twice");
                            held.push(index);
                        }
                        if held.len() == 8 || round % 3 == 0 {
                            if let Some(index) = held.pop() {
                                owner[index as usize].store(false, Ordering::Release);
                                free.give(index);
                            }
                        }
                    }
                    for index in held {
                        owner[index as usize].store(false, Ordering::Release);
                        free.give(index);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(free.len(), buffers as usize, "no buffer lost");
        let mut all: Vec<u32> = std::iter::from_fn(|| free.take()).collect();
        all.sort_unstable();
        assert_eq!(all, (0..buffers).collect::<Vec<_>>(), "each buffer once");
    }
}
