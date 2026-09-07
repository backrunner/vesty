use super::*;

#[derive(Clone, Copy)]
pub(super) enum ProcessEvents<'a> {
    Cached(&'a [VestyEvent]),
    Host(&'a ProcessData),
}

pub(super) type EventKey = (u32, u64);
pub(super) type EventOrder = (EventKey, usize);

/// Selects the next chronological batch with bounded storage, even for unsorted host lists.
/// Only the indices are heap-sorted; an inline SysEx payload moves once when its slot is replaced.
pub(super) struct EventBatch<'a> {
    events: &'a mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
    order: &'a mut [EventOrder],
    after: Option<EventKey>,
    ordinal: u64,
    len: usize,
    next: Option<EventKey>,
}

impl<'a> EventBatch<'a> {
    pub(super) fn new(
        events: &'a mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
        order: &'a mut [EventOrder],
        after: Option<EventKey>,
    ) -> Self {
        events.clear();
        Self {
            events,
            order,
            after,
            ordinal: 0,
            len: 0,
            next: None,
        }
    }

    pub(super) fn push(&mut self, event: VestyEvent) {
        let key = (event.sample_offset(), self.ordinal);
        self.ordinal += 1;
        if self.after.is_some_and(|after| key <= after) {
            return;
        }
        if self.len < MAX_BLOCK_EVENTS {
            self.events
                .push(event)
                .expect("batch has reserved capacity");
            self.order[self.len] = (key, self.len);
            let mut index = self.len;
            self.len += 1;
            while index > 0 {
                let parent = (index - 1) / 2;
                if self.order[parent].0 >= key {
                    break;
                }
                self.order.swap(parent, index);
                index = parent;
            }
        } else if key < self.order[0].0 {
            self.defer(self.order[0].0);
            self.events.as_mut_slice()[self.order[0].1] = event;
            self.order[0].0 = key;
            let mut index = 0;
            loop {
                let left = index * 2 + 1;
                if left >= self.len {
                    break;
                }
                let right = left + 1;
                let child = if right < self.len && self.order[right].0 > self.order[left].0 {
                    right
                } else {
                    left
                };
                if self.order[index].0 >= self.order[child].0 {
                    break;
                }
                self.order.swap(index, child);
                index = child;
            }
        } else {
            self.defer(key);
        }
    }

    fn defer(&mut self, key: EventKey) {
        self.next = Some(self.next.map_or(key, |next| next.min(key)));
    }

    pub(super) fn finish(self) -> Option<(EventKey, Option<u32>)> {
        if self.len == 0 {
            return None;
        }
        let order = &mut self.order[..self.len];
        order.sort_unstable_by_key(|entry| entry.0);
        let last = order[self.len - 1].0;
        let items = self.events.as_mut_slice();
        for start in 0..items.len() {
            if order[start].1 == start {
                continue;
            }
            let saved = items[start];
            let mut destination = start;
            loop {
                let source = order[destination].1;
                order[destination].1 = destination;
                if source == start {
                    items[destination] = saved;
                    break;
                }
                items[destination] = items[source];
                destination = source;
            }
        }
        Some((last, self.next.map(|key| key.0)))
    }
}

pub(super) fn rebase_events(events: &mut [VestyEvent], start: u32) {
    for event in events {
        let offset = match event {
            VestyEvent::NoteOn { sample_offset, .. }
            | VestyEvent::NoteOff { sample_offset, .. }
            | VestyEvent::PolyPressure { sample_offset, .. }
            | VestyEvent::MidiCc { sample_offset, .. }
            | VestyEvent::PitchBend { sample_offset, .. }
            | VestyEvent::ChannelPressure { sample_offset, .. }
            | VestyEvent::SysEx { sample_offset, .. }
            | VestyEvent::NoteExpressionValue { sample_offset, .. }
            | VestyEvent::NoteExpressionInt { sample_offset, .. }
            | VestyEvent::NoteExpressionText { sample_offset, .. }
            | VestyEvent::Param { sample_offset, .. } => sample_offset,
        };
        *offset -= start;
    }
}

pub(super) struct OffsetMeterSink {
    pub(super) producer: RtMeterProducer,
    pub(super) offset: u32,
}

impl vesty_core::MeterSink for OffsetMeterSink {
    fn push_meter(&mut self, mut frame: vesty_core::MeterFrame) -> bool {
        frame.sample_offset = frame.sample_offset.saturating_add(self.offset);
        self.producer.push_meter(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_preserve_every_event_and_stable_order() {
        for len in [511, 512, 513, 1024, 2049] {
            for shape in 0..3 {
                let source: Vec<_> = (0..len)
                    .map(|index| VestyEvent::NoteOn {
                        sample_offset: match shape {
                            0 => 0,
                            1 => (len - index) as u32,
                            _ => (index * 17 % 31) as u32,
                        },
                        channel: 0,
                        key: 60,
                        velocity: 1.0,
                        note_id: index,
                    })
                    .collect();
                let mut expected = source.clone();
                expected.sort_by_key(VestyEvent::sample_offset);
                let mut events = FixedEventList::new();
                let mut order = vec![((0, 0), 0); MAX_BLOCK_EVENTS];
                let mut after = None;
                let mut actual = Vec::new();
                loop {
                    let mut batch = EventBatch::new(&mut events, &mut order, after);
                    for event in &source {
                        batch.push(*event);
                    }
                    let Some((last, next)) = batch.finish() else {
                        break;
                    };
                    assert!(after.is_none_or(|previous| last > previous));
                    actual.extend_from_slice(events.as_slice());
                    after = Some(last);
                    if next.is_none() {
                        break;
                    }
                }
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn meters_keep_host_block_offsets() {
        let (producer, mut consumer) = meter_spsc(1);
        let mut sink = OffsetMeterSink {
            producer,
            offset: 23,
        };
        assert!(vesty_core::MeterSink::push_meter(
            &mut sink,
            vesty_core::MeterFrame::new(7, 3)
        ));
        assert_eq!(consumer.try_pop().unwrap().sample_offset, 26);
    }
}
