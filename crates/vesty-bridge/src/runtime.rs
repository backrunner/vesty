use serde_json::{Value, json};
use std::collections::BTreeMap;
use vesty_core::MeterFrame;
use vesty_ipc::{
    BridgeDiagnosticsSnapshot, BridgeErrorCode, BridgeErrorPayload, BridgeHelloPayload, BridgeKind,
    BridgeLane, BridgePacket, BridgeReadyPayload, IpcError, ParamChangeSource, ParamChangedEvent,
    ParamValueSnapshot, PluginFaultReport, PluginSnapshot, advance_bridge_packet_seq,
    max_message_bytes_for_lane, parse_packet, sanitize_bridge_json_value,
    validate_bridge_packet_id, validate_bridge_packet_seq, validate_bridge_session,
    validate_packet_type,
};
use vesty_params::{
    ParamSpec, format_normalized_value, normalized_to_plain, parse_normalized_value,
    validate_param_specs,
};
use vesty_rt::RtLogEvent;

use crate::{
    BridgeRuntimeCreateError, BridgeRuntimeError, BridgeStateStore, BridgeTransport,
    MAX_CONFIG_ENTRIES, MAX_PENDING_PARAM_GESTURES, MAX_SUBSCRIPTIONS, ParamGesture,
    ParamGesturePhase, SubscriptionTable, config_write_from_payload, normalized_value_from_payload,
    param_id_from_payload, param_text_from_payload, payload_gesture_id, ready_ack_protocol_version,
    rt_log_record, subscription_topic_from_payload, ui_state_write_from_payload,
    validate_empty_payload, validate_inbound_request_shape, validate_payload_allowed_fields,
};

pub struct BridgeRuntime<T: BridgeTransport> {
    pub(crate) session: String,
    pub(crate) seq: u64,
    pub(crate) transport: T,
    pub(crate) ready: BridgeReadyPayload,
    pub(crate) params: BTreeMap<String, ParamSpec>,
    pub(crate) state: BridgeStateStore,
    pub(crate) subscriptions: SubscriptionTable,
    pub(crate) coalesced_params: BTreeMap<String, usize>,
    pub(crate) latest_meters: BTreeMap<String, Value>,
    pub(crate) pending_param_gestures: Vec<ParamGesture>,
    pub(crate) dropped_param_gestures: u64,
    pub(crate) hello_acknowledged: bool,
    pub(crate) ready_acknowledged: bool,
    pub(crate) fault_report: Option<PluginFaultReport>,
}

impl<T: BridgeTransport> BridgeRuntime<T> {
    pub fn try_new(
        session: impl Into<String>,
        ready: BridgeReadyPayload,
        transport: T,
    ) -> Result<Self, BridgeRuntimeCreateError> {
        Self::new(session, ready, transport)
    }

    pub fn new(
        session: impl Into<String>,
        ready: BridgeReadyPayload,
        transport: T,
    ) -> Result<Self, BridgeRuntimeCreateError> {
        let session = session.into();
        validate_bridge_session(&session).map_err(BridgeRuntimeCreateError::Session)?;
        validate_bridge_session(&ready.editor_session_id)
            .map_err(BridgeRuntimeCreateError::Session)?;
        validate_param_specs(&ready.params)?;
        let state = BridgeStateStore::new(ready.snapshot.clone());
        let params = ready
            .params
            .iter()
            .cloned()
            .map(|spec| (spec.id.clone(), spec))
            .collect();
        Ok(Self {
            session,
            seq: 1,
            transport,
            ready,
            params,
            state,
            subscriptions: SubscriptionTable::default(),
            coalesced_params: BTreeMap::new(),
            latest_meters: BTreeMap::new(),
            pending_param_gestures: Vec::new(),
            dropped_param_gestures: 0,
            hello_acknowledged: false,
            ready_acknowledged: false,
            fault_report: None,
        })
    }

    pub fn receive_json(&mut self, text: &str) -> Result<(), BridgeRuntimeError> {
        let packet = match parse_packet(text) {
            Ok(packet) => packet,
            Err(error) => {
                let recoverable = matches!(&error, IpcError::Parse(_));
                if recoverable && let Some(packet) = self.recoverable_parse_error_packet(text) {
                    self.transport.send(&packet)?;
                    return Ok(());
                }
                return Err(error.into());
            }
        };
        let max_bytes = max_message_bytes_for_lane(&packet.lane);
        if text.len() > max_bytes {
            return self.send_error(
                &packet,
                BridgeErrorCode::Backpressure,
                "bridge message too large",
                true,
            );
        }
        self.handle_packet(packet)
    }

    fn recoverable_parse_error_packet(&mut self, text: &str) -> Option<BridgePacket> {
        let value: Value = serde_json::from_str(text).ok()?;
        let object = value.as_object()?;
        if object.get("v")?.as_u64()? != 1 {
            return None;
        }
        if object.get("kind")?.as_str()? != "request" {
            return None;
        }
        if object.get("session")?.as_str()? != self.session {
            return None;
        }
        validate_bridge_packet_seq(object.get("seq")?.as_u64()?).ok()?;
        let reply_to = object.get("id")?.as_str()?;
        if validate_bridge_packet_id(reply_to).is_err() {
            return None;
        }
        let lane = object
            .get("lane")
            .cloned()
            .and_then(|lane| serde_json::from_value::<BridgeLane>(lane).ok())
            .unwrap_or(BridgeLane::Command);

        Some(BridgePacket {
            v: 1,
            session: self.session.clone(),
            seq: self.next_seq(),
            lane,
            kind: BridgeKind::Error,
            packet_type: "bridge.parseError.error".to_string(),
            id: None,
            reply_to: Some(reply_to.to_string()),
            payload: None,
            error: Some(BridgeErrorPayload::new(
                BridgeErrorCode::ParseError,
                "failed to parse bridge packet",
                false,
            )),
            flags: Vec::new(),
        })
    }

    pub fn handle_packet(&mut self, packet: BridgePacket) -> Result<(), BridgeRuntimeError> {
        if validate_bridge_session(&packet.session).is_err() {
            return Ok(());
        }

        if packet.kind != BridgeKind::Request {
            return Ok(());
        }

        if packet.v != 1 {
            return self.send_error(
                &packet,
                BridgeErrorCode::UnsupportedVersion,
                "unsupported bridge protocol version",
                false,
            );
        }

        if let Err(message) = validate_packet_type(&packet.packet_type) {
            return self.send_error(&packet, BridgeErrorCode::ValidationError, message, false);
        }

        if let Err(message) = validate_inbound_request_shape(&packet) {
            return self.send_error(&packet, BridgeErrorCode::ValidationError, message, false);
        }

        if packet.session != self.session {
            if packet.packet_type == "bridge.hello" && packet.session == "pending" {
                self.reset_for_reload();
            } else {
                return self.send_error(
                    &packet,
                    BridgeErrorCode::PermissionDenied,
                    "session mismatch",
                    false,
                );
            }
        }

        if let Some(message) = self.disabled_request_capability_message(&packet) {
            return self.send_capability_disabled(&packet, message);
        }

        match (packet.kind.clone(), packet.packet_type.as_str()) {
            (BridgeKind::Request, "bridge.hello") => {
                let Some(hello) = packet
                    .payload
                    .clone()
                    .and_then(|payload| serde_json::from_value::<BridgeHelloPayload>(payload).ok())
                else {
                    return self.send_error(
                        &packet,
                        BridgeErrorCode::ValidationError,
                        "missing or invalid bridge hello payload",
                        false,
                    );
                };
                if !hello.has_valid_shape() {
                    return self.send_error(
                        &packet,
                        BridgeErrorCode::ValidationError,
                        "invalid bridge hello metadata",
                        false,
                    );
                }
                if !hello.supports_protocol(self.ready.protocol_version) {
                    return self.send_error(
                        &packet,
                        BridgeErrorCode::UnsupportedVersion,
                        "unsupported bridge hello protocol",
                        false,
                    );
                }
                let payload = serde_json::to_value(&self.ready).unwrap_or_else(|_| json!({}));
                let response = packet.response_to(self.next_seq(), Some(payload));
                self.transport.send(&response)?;
                self.hello_acknowledged = true;
                if self.session == "pending" {
                    self.session = self.ready.editor_session_id.clone();
                }
            }
            (BridgeKind::Request, "bridge.readyAck") => {
                if !self.hello_acknowledged {
                    return self.send_error(
                        &packet,
                        BridgeErrorCode::PermissionDenied,
                        "readyAck requires bridge.hello",
                        false,
                    );
                }
                let protocol_version = match ready_ack_protocol_version(packet.payload.as_ref()) {
                    Ok(protocol_version) => protocol_version,
                    Err(message) => {
                        return self.send_error(
                            &packet,
                            BridgeErrorCode::ValidationError,
                            message,
                            false,
                        );
                    }
                };
                if protocol_version != u64::from(self.ready.protocol_version) {
                    return self.send_error(
                        &packet,
                        BridgeErrorCode::UnsupportedVersion,
                        "unsupported bridge readyAck protocol",
                        false,
                    );
                }
                self.ready_acknowledged = true;
                let response = packet.response_to(
                    self.next_seq(),
                    Some(json!({
                        "ready": true,
                        "editorSessionId": self.ready.editor_session_id.clone(),
                    })),
                );
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "snapshot.get") => {
                if let Err(message) = validate_empty_payload(packet.payload.as_ref()) {
                    self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    return Ok(());
                }
                let payload =
                    serde_json::to_value(self.state.snapshot()).unwrap_or_else(|_| json!({}));
                let response = packet.response_to(self.next_seq(), Some(payload));
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "diagnostics.get") => {
                if let Err(message) = validate_empty_payload(packet.payload.as_ref()) {
                    self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    return Ok(());
                }
                let payload =
                    serde_json::to_value(self.diagnostics_snapshot()).unwrap_or_else(|_| json!({}));
                let response = packet.response_to(self.next_seq(), Some(payload));
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "subscription.add") => {
                let topic = match subscription_topic_from_payload(packet.payload.as_ref()) {
                    Ok(topic) => topic,
                    Err(message) => {
                        self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                        return Ok(());
                    }
                };
                if let Some(message) = self.disabled_subscription_topic_message(topic) {
                    self.send_capability_disabled(&packet, message)?;
                    return Ok(());
                }
                if !self.subscriptions.contains(topic)
                    && self.subscriptions.len() >= MAX_SUBSCRIPTIONS
                {
                    self.send_error(
                        &packet,
                        BridgeErrorCode::Backpressure,
                        "subscription table full",
                        true,
                    )?;
                    return Ok(());
                }
                self.subscriptions.subscribe(topic);
                let response = packet.response_to(self.next_seq(), Some(json!({ "topic": topic })));
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "subscription.remove") => {
                let topic = match subscription_topic_from_payload(packet.payload.as_ref()) {
                    Ok(topic) => topic,
                    Err(message) => {
                        self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                        return Ok(());
                    }
                };
                if let Some(message) = self.disabled_subscription_topic_message(topic) {
                    self.send_capability_disabled(&packet, message)?;
                    return Ok(());
                }
                self.subscriptions.unsubscribe(topic);
                self.latest_meters.remove(topic);
                let response = packet.response_to(self.next_seq(), Some(json!({ "topic": topic })));
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "state.setConfig") => {
                match config_write_from_payload(packet.payload.as_ref()) {
                    Ok((base_revision, key, value)) => {
                        if base_revision != self.state.snapshot().config_revision {
                            let mut error = BridgeErrorPayload::new(
                                BridgeErrorCode::StateConflict,
                                "config revision conflict",
                                true,
                            );
                            error.set_details(json!({
                                "snapshot": self.state.snapshot(),
                            }));
                            let response = packet.error_to(self.next_seq(), error);
                            self.transport.send(&response)?;
                            return Ok(());
                        }
                        if !self.state.has_config_key(&key)
                            && self.state.config_entry_count() >= MAX_CONFIG_ENTRIES
                        {
                            self.send_error(
                                &packet,
                                BridgeErrorCode::Backpressure,
                                "config entry table full",
                                true,
                            )?;
                            return Ok(());
                        }
                        self.state.set_config_value(key, value);
                        let payload = serde_json::to_value(self.state.snapshot())
                            .unwrap_or_else(|_| json!({}));
                        let response = packet.response_to(self.next_seq(), Some(payload.clone()));
                        self.transport.send(&response)?;
                        self.emit_event("state.changed", payload)?;
                    }
                    Err(message) => {
                        self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    }
                }
            }
            (BridgeKind::Request, "state.setUiState") => {
                match ui_state_write_from_payload(packet.payload.as_ref()) {
                    Ok((base_revision, value)) => {
                        if base_revision != self.state.snapshot().ui_revision {
                            let mut error = BridgeErrorPayload::new(
                                BridgeErrorCode::StateConflict,
                                "ui state revision conflict",
                                true,
                            );
                            error.set_details(json!({
                                "snapshot": self.state.snapshot(),
                            }));
                            let response = packet.error_to(self.next_seq(), error);
                            self.transport.send(&response)?;
                            return Ok(());
                        }
                        self.state.set_ui_state(value);
                        let payload = serde_json::to_value(self.state.snapshot())
                            .unwrap_or_else(|_| json!({}));
                        let response = packet.response_to(self.next_seq(), Some(payload.clone()));
                        self.transport.send(&response)?;
                        self.emit_event("state.changed", payload)?;
                    }
                    Err(message) => {
                        self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    }
                }
            }
            (BridgeKind::Request, "param.begin") => self.handle_param_ack(&packet, "begin")?,
            (BridgeKind::Request, "param.end") => self.handle_param_ack(&packet, "end")?,
            (BridgeKind::Request, "param.perform") => self.handle_param_perform(&packet)?,
            (BridgeKind::Request, "param.format") => self.handle_param_format(&packet)?,
            (BridgeKind::Request, "param.parse") => self.handle_param_parse(&packet)?,
            (BridgeKind::Request, "meter.flush") => {
                if let Err(message) = validate_empty_payload(packet.payload.as_ref()) {
                    self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    return Ok(());
                }
                let response = packet.response_to(
                    self.next_seq(),
                    Some(json!({ "queued": self.pending_latest_meter_count() })),
                );
                self.transport.send(&response)?;
            }
            (BridgeKind::Request, "event.flush") => {
                if let Err(message) = validate_empty_payload(packet.payload.as_ref()) {
                    self.send_error(&packet, BridgeErrorCode::ValidationError, message, false)?;
                    return Ok(());
                }
                let response = packet.response_to(
                    self.next_seq(),
                    Some(json!({
                        "pendingMeterTopics": self.pending_latest_meter_count(),
                        "pendingParamGestures": self.pending_param_gesture_count(),
                    })),
                );
                self.transport.send(&response)?;
            }
            _ => {
                self.send_error(
                    &packet,
                    BridgeErrorCode::UnsupportedType,
                    "unsupported bridge packet type",
                    false,
                )?;
            }
        }

        Ok(())
    }

    pub fn drain_param_gestures(&mut self) -> Vec<ParamGesture> {
        self.coalesced_params.clear();
        std::mem::take(&mut self.pending_param_gestures)
    }

    pub fn set_ready_param_values(&mut self, param_values: Vec<ParamValueSnapshot>) {
        self.ready.param_values = param_values;
    }

    fn reset_for_reload(&mut self) {
        self.session = "pending".to_string();
        self.hello_acknowledged = false;
        self.ready_acknowledged = false;
        self.subscriptions = SubscriptionTable::default();
        self.coalesced_params.clear();
        self.latest_meters.clear();
        self.pending_param_gestures.clear();
    }

    pub fn pending_param_gesture_count(&self) -> usize {
        self.pending_param_gestures.len()
    }

    pub fn snapshot(&self) -> &PluginSnapshot {
        self.state.snapshot()
    }

    pub fn restore_snapshot_from_host(
        &mut self,
        snapshot: PluginSnapshot,
    ) -> Result<bool, BridgeRuntimeError> {
        if self.state.snapshot() == &snapshot {
            return Ok(false);
        }

        self.ready.snapshot = snapshot.clone();
        self.state.replace_snapshot(snapshot);
        let payload = serde_json::to_value(self.state.snapshot()).unwrap_or_else(|_| json!({}));
        self.emit_event("state.changed", payload)?;
        Ok(true)
    }

    pub fn is_subscribed(&self, topic: &str) -> bool {
        self.subscriptions.contains(topic)
    }

    pub fn ready_acknowledged(&self) -> bool {
        self.ready_acknowledged
    }

    pub fn set_fault_report(&mut self, fault_report: Option<PluginFaultReport>) {
        self.fault_report = fault_report;
    }

    pub fn diagnostics_snapshot(&self) -> BridgeDiagnosticsSnapshot {
        BridgeDiagnosticsSnapshot {
            ready_acknowledged: self.ready_acknowledged,
            subscription_count: self.subscriptions.len(),
            subscriptions: self.subscriptions.topics(),
            pending_param_gestures: self.pending_param_gestures.len(),
            dropped_param_gestures: self.dropped_param_gestures,
            pending_meter_topics: self.latest_meters.len(),
            fault: self.fault_report.clone(),
        }
    }

    fn handle_param_ack(
        &mut self,
        packet: &BridgePacket,
        phase: &'static str,
    ) -> Result<(), BridgeRuntimeError> {
        if let Err(message) =
            validate_payload_allowed_fields(packet.payload.as_ref(), &["id", "gestureId"])
        {
            return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
        }
        let id = match param_id_from_payload(packet.payload.as_ref()) {
            Ok(id) => id.to_string(),
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let Some(spec) = self.params.get(&id) else {
            return self.send_error(
                packet,
                BridgeErrorCode::ValidationError,
                "unknown parameter id",
                false,
            );
        };
        if spec.flags.read_only {
            return self.send_error(
                packet,
                BridgeErrorCode::PermissionDenied,
                "parameter is read only",
                false,
            );
        }
        let gesture_id = match payload_gesture_id(packet.payload.as_ref()) {
            Ok(gesture_id) => gesture_id,
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let gesture_phase = match phase {
            "begin" => ParamGesturePhase::Begin,
            "end" => ParamGesturePhase::End,
            _ => unreachable!("unknown parameter gesture phase"),
        };
        if self.pending_param_gestures.len() >= MAX_PENDING_PARAM_GESTURES
            && gesture_phase == ParamGesturePhase::End
        {
            self.drop_pending_param_gesture_for_end();
        }
        if self.pending_param_gestures.len() >= MAX_PENDING_PARAM_GESTURES {
            return self.send_error(
                packet,
                BridgeErrorCode::Backpressure,
                "pending parameter gesture queue full",
                true,
            );
        }
        self.pending_param_gestures.push(ParamGesture {
            phase: gesture_phase,
            id: id.clone(),
            normalized: None,
            gesture_id,
            request_ids: packet.id.iter().cloned().collect(),
        });
        self.coalesced_params.remove(&id);
        let response =
            packet.response_to(self.next_seq(), Some(json!({ "id": id, "phase": phase })));
        self.transport.send(&response)?;
        Ok(())
    }

    fn drop_pending_param_gesture_for_end(&mut self) {
        let drop_index = self
            .pending_param_gestures
            .iter()
            .position(|gesture| gesture.phase == ParamGesturePhase::Perform)
            .unwrap_or(0);
        if !self.pending_param_gestures.is_empty() {
            self.pending_param_gestures.remove(drop_index);
            self.rebuild_coalesced_param_indices();
            self.dropped_param_gestures = self.dropped_param_gestures.saturating_add(1);
        }
    }

    fn handle_param_perform(&mut self, packet: &BridgePacket) -> Result<(), BridgeRuntimeError> {
        if let Err(message) = validate_payload_allowed_fields(
            packet.payload.as_ref(),
            &["id", "normalized", "gestureId"],
        ) {
            return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
        }
        let id = match param_id_from_payload(packet.payload.as_ref()) {
            Ok(id) => id.to_string(),
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let Some(spec) = self.params.get(&id) else {
            return self.send_error(
                packet,
                BridgeErrorCode::ValidationError,
                "unknown parameter id",
                false,
            );
        };
        if spec.flags.read_only {
            return self.send_error(
                packet,
                BridgeErrorCode::PermissionDenied,
                "parameter is read only",
                false,
            );
        }
        let gesture_id = match payload_gesture_id(packet.payload.as_ref()) {
            Ok(gesture_id) => gesture_id,
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let normalized = match normalized_value_from_payload(packet.payload.as_ref()) {
            Ok(normalized) => normalized.clamp(0.0, 1.0),
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        if self.try_coalesce_param_perform(
            &id,
            normalized,
            gesture_id.clone(),
            packet.id.as_deref(),
        ) {
            let response = packet.response_to(
                self.next_seq(),
                Some(json!({ "id": id, "normalized": normalized, "queued": true, "coalesced": true })),
            );
            self.transport.send(&response)?;
            return Ok(());
        }

        if self.pending_param_gestures.len() >= MAX_PENDING_PARAM_GESTURES {
            return self.send_error(
                packet,
                BridgeErrorCode::Backpressure,
                "pending parameter gesture queue full",
                true,
            );
        }

        let index = self.pending_param_gestures.len();
        self.pending_param_gestures.push(ParamGesture {
            phase: ParamGesturePhase::Perform,
            id: id.clone(),
            normalized: Some(normalized),
            gesture_id,
            request_ids: packet.id.iter().cloned().collect(),
        });
        self.coalesced_params.insert(id.clone(), index);
        let response = packet.response_to(
            self.next_seq(),
            Some(json!({ "id": id, "normalized": normalized, "queued": true })),
        );
        self.transport.send(&response)?;
        Ok(())
    }

    fn try_coalesce_param_perform(
        &mut self,
        id: &str,
        normalized: f64,
        gesture_id: Option<String>,
        request_id: Option<&str>,
    ) -> bool {
        let Some(index) = self.coalesced_params.get(id).copied() else {
            return false;
        };
        let Some(gesture) = self.pending_param_gestures.get_mut(index) else {
            self.coalesced_params.remove(id);
            return false;
        };
        if gesture.phase != ParamGesturePhase::Perform || gesture.id != id {
            self.coalesced_params.remove(id);
            return false;
        }
        gesture.normalized = Some(normalized);
        gesture.gesture_id = gesture_id;
        if let Some(request_id) = request_id {
            gesture.request_ids.push(request_id.to_string());
        }
        true
    }

    fn rebuild_coalesced_param_indices(&mut self) {
        self.coalesced_params.clear();
        for (index, gesture) in self.pending_param_gestures.iter().enumerate() {
            if gesture.phase == ParamGesturePhase::Perform {
                self.coalesced_params.insert(gesture.id.clone(), index);
            }
        }
    }

    fn handle_param_format(&mut self, packet: &BridgePacket) -> Result<(), BridgeRuntimeError> {
        let (spec, normalized) = match self.param_value_request(packet) {
            Ok(request) => request,
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let display = format_normalized_value(spec, normalized);
        let response = packet.response_to(self.next_seq(), Some(json!(display)));
        self.transport.send(&response)?;
        Ok(())
    }

    fn handle_param_parse(&mut self, packet: &BridgePacket) -> Result<(), BridgeRuntimeError> {
        if let Err(message) =
            validate_payload_allowed_fields(packet.payload.as_ref(), &["id", "text"])
        {
            return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
        }
        let id = match param_id_from_payload(packet.payload.as_ref()) {
            Ok(id) => id.to_string(),
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let Some(spec) = self.params.get(&id).cloned() else {
            return self.send_error(
                packet,
                BridgeErrorCode::ValidationError,
                "unknown parameter id",
                false,
            );
        };
        let text = match param_text_from_payload(packet.payload.as_ref()) {
            Ok(text) => text,
            Err(message) => {
                return self.send_error(packet, BridgeErrorCode::ValidationError, message, false);
            }
        };
        let Some(normalized) = parse_normalized_value(&spec, text) else {
            return self.send_error(
                packet,
                BridgeErrorCode::ValidationError,
                "invalid parameter text",
                false,
            );
        };
        let response = packet.response_to(self.next_seq(), Some(json!(normalized)));
        self.transport.send(&response)?;
        Ok(())
    }

    fn param_value_request<'a>(
        &'a self,
        packet: &BridgePacket,
    ) -> Result<(&'a ParamSpec, f64), &'static str> {
        validate_payload_allowed_fields(packet.payload.as_ref(), &["id", "normalized"])?;
        let id = param_id_from_payload(packet.payload.as_ref())?;
        let normalized = normalized_value_from_payload(packet.payload.as_ref())?.clamp(0.0, 1.0);
        let Some(spec) = self.params.get(id) else {
            return Err("unknown parameter id");
        };
        Ok((spec, normalized))
    }

    pub fn emit_event(&mut self, topic: &str, payload: Value) -> Result<(), BridgeRuntimeError> {
        if validate_packet_type(topic).is_err() {
            return Ok(());
        }
        if self.disabled_topic_message(topic).is_some() {
            return Ok(());
        }
        if !self.subscriptions.contains(topic) {
            return Ok(());
        }

        let packet = BridgePacket {
            v: 1,
            session: self.session.clone(),
            seq: self.next_seq(),
            lane: BridgeLane::Event,
            kind: BridgeKind::Event,
            packet_type: topic.to_string(),
            id: None,
            reply_to: None,
            payload: Some(sanitize_bridge_json_value(payload)),
            error: None,
            flags: Vec::new(),
        };
        self.transport.send(&packet)?;
        Ok(())
    }

    pub fn emit_param_changed(
        &mut self,
        id: &str,
        normalized: f64,
        source: ParamChangeSource,
        gesture_id: Option<String>,
    ) -> Result<bool, BridgeRuntimeError> {
        let Some(spec) = self.params.get(id).cloned() else {
            return Ok(false);
        };
        let normalized = normalized.clamp(0.0, 1.0);
        self.state.advance_params_revision();
        let event = ParamChangedEvent {
            id: id.to_string(),
            normalized,
            plain: Some(normalized_to_plain(&spec, normalized)),
            display: Some(format_normalized_value(&spec, normalized)),
            source,
            gesture_id,
            revision: self.state.snapshot().revision,
        };
        let payload = serde_json::to_value(event).unwrap_or_else(|_| json!({}));
        self.emit_event("param.changed", payload)?;
        Ok(true)
    }

    pub fn queue_latest_meter(&mut self, topic: &str, payload: Value) -> bool {
        if validate_packet_type(topic).is_err() {
            return false;
        }
        if !self.ready.capabilities.meter_stream {
            return false;
        }
        if !self.subscriptions.contains(topic) {
            return false;
        }

        self.latest_meters
            .insert(topic.to_string(), sanitize_bridge_json_value(payload));
        true
    }

    pub fn queue_latest_meter_frame(&mut self, topic: &str, frame: &MeterFrame) -> bool {
        let channels = frame.channel_count();
        let payload = json!({
            "idHash": frame.id_hash,
            "sampleOffset": frame.sample_offset,
            "channels": channels,
            "peaks": &frame.peaks[..channels],
            "rms": &frame.rms[..channels],
        });
        self.queue_latest_meter(topic, payload)
    }

    pub fn pending_latest_meter_count(&self) -> usize {
        self.latest_meters.len()
    }

    pub fn flush_latest_meters(&mut self) -> Result<usize, BridgeRuntimeError> {
        if !self.ready.capabilities.meter_stream {
            self.latest_meters.clear();
            return Ok(0);
        }
        if self.latest_meters.is_empty() {
            return Ok(0);
        }

        let latest_meters = std::mem::take(&mut self.latest_meters);
        let mut packets = Vec::with_capacity(latest_meters.len());
        for (topic, payload) in latest_meters {
            if validate_packet_type(&topic).is_err() {
                continue;
            }
            if !self.subscriptions.contains(&topic) {
                continue;
            }
            packets.push(BridgePacket {
                v: 1,
                session: self.session.clone(),
                seq: self.next_seq(),
                lane: BridgeLane::Meter,
                kind: BridgeKind::Event,
                packet_type: topic,
                id: None,
                reply_to: None,
                payload: Some(sanitize_bridge_json_value(payload)),
                error: None,
                flags: vec!["latest".to_string()],
            });
        }

        let sent = packets.len();
        self.transport.send_batch(&packets)?;
        Ok(sent)
    }

    pub fn emit_fault_report(
        &mut self,
        topic: &str,
        report: PluginFaultReport,
    ) -> Result<(), BridgeRuntimeError> {
        self.fault_report = Some(report.clone());
        let payload = serde_json::to_value(report).unwrap_or_else(|_| json!({}));
        self.emit_event(topic, payload)
    }

    pub fn emit_rt_log_event(
        &mut self,
        topic: &str,
        sequence: u64,
        event: RtLogEvent,
    ) -> Result<(), BridgeRuntimeError> {
        if validate_packet_type(topic).is_err() {
            return Ok(());
        }
        if !self.ready.capabilities.diagnostics {
            return Ok(());
        }
        if !self.subscriptions.contains(topic) {
            return Ok(());
        }

        let record = rt_log_record(sequence, event);
        let packet = BridgePacket {
            v: 1,
            session: self.session.clone(),
            seq: self.next_seq(),
            lane: BridgeLane::Log,
            kind: BridgeKind::Event,
            packet_type: topic.to_string(),
            id: None,
            reply_to: None,
            payload: Some(sanitize_bridge_json_value(
                serde_json::to_value(record).unwrap_or_else(|_| json!({})),
            )),
            error: None,
            flags: Vec::new(),
        };
        self.transport.send(&packet)?;
        Ok(())
    }

    fn send_error(
        &mut self,
        packet: &BridgePacket,
        code: BridgeErrorCode,
        message: &'static str,
        retryable: bool,
    ) -> Result<(), BridgeRuntimeError> {
        let response = packet.error_to(
            self.next_seq(),
            BridgeErrorPayload::new(code, message, retryable),
        );
        self.transport.send(&response)?;
        Ok(())
    }

    fn send_capability_disabled(
        &mut self,
        packet: &BridgePacket,
        message: &'static str,
    ) -> Result<(), BridgeRuntimeError> {
        self.send_error(packet, BridgeErrorCode::UnsupportedType, message, false)
    }

    fn disabled_request_capability_message(&self, packet: &BridgePacket) -> Option<&'static str> {
        if packet.kind != BridgeKind::Request {
            return None;
        }

        let capabilities = &self.ready.capabilities;
        match packet.packet_type.as_str() {
            "diagnostics.get" if !capabilities.diagnostics => {
                Some("diagnostics capability is disabled")
            }
            "subscription.add" | "subscription.remove" if !capabilities.subscriptions => {
                Some("subscriptions capability is disabled")
            }
            "state.setConfig" | "state.setUiState" if !capabilities.state_config => {
                Some("state config capability is disabled")
            }
            "param.begin" | "param.perform" | "param.end" if !capabilities.param_gestures => {
                Some("param gestures capability is disabled")
            }
            "param.format" | "param.parse" if !capabilities.param_format_parse => {
                Some("param format/parse capability is disabled")
            }
            "meter.flush" if !capabilities.meter_stream => {
                Some("meter stream capability is disabled")
            }
            _ => None,
        }
    }

    fn disabled_subscription_topic_message(&self, topic: &str) -> Option<&'static str> {
        self.disabled_topic_message(topic)
    }

    fn disabled_topic_message(&self, topic: &str) -> Option<&'static str> {
        let capabilities = &self.ready.capabilities;
        if topic.starts_with("meter.") {
            if capabilities.meter_stream {
                None
            } else {
                Some("meter stream capability is disabled")
            }
        } else if topic == "diagnostics.fault" || topic == "log.rt" {
            if capabilities.diagnostics {
                None
            } else {
                Some("diagnostics capability is disabled")
            }
        } else if capabilities.reliable_events {
            None
        } else {
            Some("reliable events capability is disabled")
        }
    }

    pub(crate) fn next_seq(&mut self) -> u64 {
        let seq = self.seq;
        self.seq = advance_bridge_packet_seq(self.seq);
        seq
    }
}
