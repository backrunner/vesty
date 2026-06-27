use rtrb::{Consumer, PopError, Producer, PushError, RingBuffer};
use std::cell::Cell;
use thiserror::Error;
use vesty_core::{MeterFrame, MeterSink};

pub fn spsc<T>(capacity: usize) -> (RtProducer<T>, RtConsumer<T>) {
    let (producer, consumer) = RingBuffer::new(capacity);
    (
        RtProducer { inner: producer },
        RtConsumer { inner: consumer },
    )
}

pub struct RtProducer<T> {
    inner: Producer<T>,
}

impl<T> RtProducer<T> {
    pub fn try_push(&mut self, value: T) -> Result<(), PushError<T>> {
        self.inner.push(value)
    }
}

pub struct RtConsumer<T> {
    inner: Consumer<T>,
}

impl<T> RtConsumer<T> {
    pub fn try_pop(&mut self) -> Result<T, PopError> {
        self.inner.pop()
    }
}

pub type RtMeterProducer = RtProducer<MeterFrame>;
pub type RtMeterConsumer = RtConsumer<MeterFrame>;

pub fn meter_spsc(capacity: usize) -> (RtMeterProducer, RtMeterConsumer) {
    spsc(capacity)
}

impl MeterSink for RtMeterProducer {
    fn push_meter(&mut self, frame: MeterFrame) -> bool {
        self.try_push(frame).is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueId {
    Events,
    Params,
    Meter,
    Log,
    Bridge,
    Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtLogEvent {
    QueueOverflow {
        queue: QueueId,
        dropped: u32,
    },
    Faulted {
        code: u32,
    },
    HostWarning {
        code: u32,
        value: i64,
    },
    Custom {
        level: RtLogLevel,
        code: u32,
        value_a: i64,
        value_b: i64,
    },
}

impl RtLogEvent {
    pub fn level(&self) -> RtLogLevel {
        match self {
            Self::QueueOverflow { .. } | Self::HostWarning { .. } => RtLogLevel::Warn,
            Self::Faulted { .. } => RtLogLevel::Error,
            Self::Custom { level, .. } => *level,
        }
    }
}

pub type RtLogProducer = RtProducer<RtLogEvent>;
pub type RtLogConsumer = RtConsumer<RtLogEvent>;

pub fn log_spsc(capacity: usize) -> (RtLogProducer, RtLogConsumer) {
    spsc(capacity)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FixedListError {
    #[error("fixed list capacity exceeded")]
    Full,
}

#[derive(Clone, Debug)]
pub struct FixedEventList<T, const N: usize> {
    items: Vec<T>,
}

impl<T, const N: usize> FixedEventList<T, N> {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(N),
        }
    }

    pub fn push(&mut self, item: T) -> Result<(), FixedListError> {
        if self.items.len() >= N {
            return Err(FixedListError::Full);
        }
        self.items.push(item);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<T, const N: usize> Default for FixedEventList<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static NO_ALLOC_REGION: Cell<bool> = const { Cell::new(false) };
}

pub struct NoAllocGuard {
    previous: bool,
}

impl NoAllocGuard {
    pub fn enter() -> Self {
        let previous = NO_ALLOC_REGION.with(|flag| {
            let previous = flag.get();
            flag.set(true);
            previous
        });
        Self { previous }
    }

    pub fn is_active() -> bool {
        NO_ALLOC_REGION.with(Cell::get)
    }
}

impl Drop for NoAllocGuard {
    fn drop(&mut self) {
        NO_ALLOC_REGION.with(|flag| flag.set(self.previous));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_roundtrips() {
        let (mut producer, mut consumer) = spsc(2);
        producer.try_push(7).unwrap();
        assert_eq!(consumer.try_pop().unwrap(), 7);
    }

    #[test]
    fn meter_queue_implements_meter_sink() {
        let (mut producer, mut consumer) = meter_spsc(1);
        let mut frame = MeterFrame::new(42, 0);
        frame.set_channel(0, 0.75, 0.5);

        assert!(producer.push_meter(frame));
        assert!(!producer.push_meter(MeterFrame::new(43, 0)));
        assert_eq!(consumer.try_pop().unwrap(), frame);
        assert!(consumer.try_pop().is_err());
    }

    #[test]
    fn log_queue_uses_fixed_events_and_drops_on_overflow() {
        let (mut producer, mut consumer) = log_spsc(1);
        let event = RtLogEvent::QueueOverflow {
            queue: QueueId::Meter,
            dropped: 1,
        };
        assert_eq!(event.level(), RtLogLevel::Warn);
        assert!(producer.try_push(event).is_ok());
        assert!(producer.try_push(RtLogEvent::Faulted { code: 7 }).is_err());
        assert_eq!(consumer.try_pop().unwrap(), event);
        assert!(consumer.try_pop().is_err());
    }

    #[test]
    fn fixed_list_enforces_capacity() {
        let mut list = FixedEventList::<u32, 1>::new();
        assert_eq!(list.push(1), Ok(()));
        assert_eq!(list.push(2), Err(FixedListError::Full));
    }

    #[test]
    fn no_alloc_guard_tracks_region() {
        assert!(!NoAllocGuard::is_active());
        {
            let _guard = NoAllocGuard::enter();
            assert!(NoAllocGuard::is_active());
        }
        assert!(!NoAllocGuard::is_active());
    }
}
