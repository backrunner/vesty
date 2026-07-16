use vesty_ipc::{
    RtLogKind as IpcRtLogKind, RtLogLevel as IpcRtLogLevel, RtLogQueue as IpcRtLogQueue,
    RtLogRecord,
};
use vesty_rt::{QueueId, RtLogEvent, RtLogLevel};

pub(crate) fn rt_log_record(sequence: u64, event: RtLogEvent) -> RtLogRecord {
    let level = match event.level() {
        RtLogLevel::Debug => IpcRtLogLevel::Debug,
        RtLogLevel::Info => IpcRtLogLevel::Info,
        RtLogLevel::Warn => IpcRtLogLevel::Warn,
        RtLogLevel::Error => IpcRtLogLevel::Error,
    };
    let mut record = RtLogRecord {
        sequence,
        level,
        kind: IpcRtLogKind::Custom,
        queue: None,
        other_queue_id: None,
        dropped: None,
        code: None,
        value: None,
        value_a: None,
        value_b: None,
    };

    match event {
        RtLogEvent::QueueOverflow { queue, dropped } => {
            let (queue, other_queue_id) = rt_log_queue(queue);
            record.kind = IpcRtLogKind::QueueOverflow;
            record.queue = Some(queue);
            record.other_queue_id = other_queue_id;
            record.dropped = Some(dropped);
        }
        RtLogEvent::Faulted { code } => {
            record.kind = IpcRtLogKind::Faulted;
            record.code = Some(code);
        }
        RtLogEvent::HostWarning { code, value } => {
            record.kind = IpcRtLogKind::HostWarning;
            record.code = Some(code);
            record.value = Some(value);
        }
        RtLogEvent::Custom {
            code,
            value_a,
            value_b,
            ..
        } => {
            record.kind = IpcRtLogKind::Custom;
            record.code = Some(code);
            record.value_a = Some(value_a);
            record.value_b = Some(value_b);
        }
    }
    record
}

pub(crate) fn rt_log_queue(queue: QueueId) -> (IpcRtLogQueue, Option<u16>) {
    match queue {
        QueueId::Events => (IpcRtLogQueue::Events, None),
        QueueId::Params => (IpcRtLogQueue::Params, None),
        QueueId::Meter => (IpcRtLogQueue::Meter, None),
        QueueId::Log => (IpcRtLogQueue::Log, None),
        QueueId::Bridge => (IpcRtLogQueue::Bridge, None),
        QueueId::Other(id) => (IpcRtLogQueue::Other, Some(id)),
    }
}
