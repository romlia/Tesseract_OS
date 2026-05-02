use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;

#[derive(Debug)]
pub struct QueueFull;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackpressurePolicy {
    DropOldest,
    DropNewest,
    Block,
}

pub trait EventBus<T>: Send + Sync {
    fn push(&self, event: T) -> Result<(), QueueFull>;
    fn pop(&self) -> Option<T>;
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
}

pub struct LockFreeEventBus {
    buffer: [UnsafeCell<Option<crate::context::SensoryEvent>>; 256],
    head: AtomicUsize,
    tail: AtomicUsize,
    policy: BackpressurePolicy,
}

unsafe impl Sync for LockFreeEventBus {}
unsafe impl Send for LockFreeEventBus {}

impl Default for LockFreeEventBus {
    fn default() -> Self {
        Self::new(BackpressurePolicy::Block)
    }
}

impl LockFreeEventBus {
    pub fn new(policy: BackpressurePolicy) -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: UnsafeCell<Option<crate::context::SensoryEvent>> = UnsafeCell::new(None);
        Self {
            buffer: [INIT; 256],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            policy,
        }
    }
}

pub struct QueueDepthMonitor<'a, T> {
    bus: &'a dyn EventBus<T>,
    threshold_percent: usize,
}

impl<'a, T> QueueDepthMonitor<'a, T> {
    pub fn new(bus: &'a dyn EventBus<T>, threshold_percent: usize) -> Self {
        Self { bus, threshold_percent }
    }
    
    pub fn is_overloaded(&self) -> bool {
        let cap = self.bus.capacity();
        if cap == 0 { return false; }
        (self.bus.len() * 100) / cap > self.threshold_percent
    }
}

impl EventBus<crate::context::SensoryEvent> for LockFreeEventBus {
    fn capacity(&self) -> usize { 256 }

    fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        if head >= tail {
            head - tail
        } else {
            256 - tail + head
        }
    }

    fn push(&self, event: crate::context::SensoryEvent) -> Result<(), QueueFull> {
        let backoff = crossbeam::utils::Backoff::new();
        loop {
            if crate::SHUTDOWN.load(Ordering::Relaxed) { return Ok(()); }
            let tail = self.tail.load(Ordering::Acquire);
            let next_tail = (tail + 1) % 256;
            
            if next_tail == self.head.load(Ordering::Acquire) {
                match self.policy {
                    BackpressurePolicy::DropOldest => {
                        let _ = self.pop();
                        continue;
                    }
                    BackpressurePolicy::DropNewest => {
                        return Err(QueueFull);
                    }
                    BackpressurePolicy::Block => {
                        if backoff.is_completed() {
                            tracing::warn!("EventBus blocked too long, dropping event.");
                            return Err(QueueFull);
                        }
                        backoff.spin();
                        continue;
                    }
                }
            }

            unsafe {
                *self.buffer[tail].get() = Some(event);
            }
            self.tail.store(next_tail, Ordering::Release);
            return Ok(());
        }
    }

    fn pop(&self) -> Option<crate::context::SensoryEvent> {
        let head = self.head.load(Ordering::Relaxed);
        if head == self.tail.load(Ordering::Acquire) {
            return None; // Empty
        }

        let event = unsafe {
            (*self.buffer[head].get()).take()
        };
        self.head.store((head + 1) % 256, Ordering::Release);
        event
    }
}

pub struct CrossbeamBus<T> {
    queue: crossbeam::queue::ArrayQueue<T>,
}

impl<T> CrossbeamBus<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: crossbeam::queue::ArrayQueue::new(capacity),
        }
    }
}

impl<T: Send + Sync> EventBus<T> for CrossbeamBus<T> {
    fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn push(&self, event: T) -> Result<(), QueueFull> {
        self.queue.push(event).map_err(|_| QueueFull)
    }

    fn pop(&self) -> Option<T> {
        self.queue.pop()
    }
}
