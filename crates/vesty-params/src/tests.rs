use super::*;

struct ManualParams {
    gain: FloatParam,
}

impl ParamCollection for ManualParams {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![self.gain.spec()]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        (id == self.gain.id()).then(|| self.gain.normalized())
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == self.gain.id() {
            self.gain.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        (handle.index() == 0).then(|| self.gain.normalized())
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        if handle.index() == 0 {
            self.gain.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(format!("handle:{}", handle.index())))
        }
    }
}

#[test]
fn converts_float_values() {
    let spec = ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0);
    assert!((plain_to_normalized(&spec, 0.0) - 0.833_333_333).abs() < 0.0001);
    assert!((normalized_to_plain(&spec, 1.0) - 12.0).abs() < 0.0001);
}

#[test]
fn derives_stable_vst3_param_ids_from_string_ids() {
    assert_eq!(
        VST3_PARAM_ID_ALGORITHM,
        "vesty.vst3.param.fnv1a31-positive.v2"
    );
    assert_eq!(stable_vst3_param_id("gain"), 1_983_572_582);
    assert_eq!(stable_vst3_param_id("mix"), 2_093_438_089);
    assert_eq!(stable_vst3_param_id("level"), 1_889_771_317);
    assert!(stable_vst3_param_id("mix") <= i32::MAX as u32);
    assert!(stable_vst3_param_id("level") <= i32::MAX as u32);
    assert_ne!(stable_vst3_param_id("gain"), stable_vst3_param_id("mode"));
    assert_eq!(stable_vst3_param_id("gain"), stable_vst3_param_id("gain"));
}

#[test]
fn param_handles_resolve_and_access_values() {
    let params = ManualParams {
        gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
    };
    let handle = params.resolve("gain").unwrap();
    assert_eq!(handle.index(), 0);
    assert_eq!(params.get_normalized_by_handle(handle), Some(0.5));

    params.set_normalized_by_handle(handle, 0.25).unwrap();
    assert_eq!(params.get_normalized("gain"), Some(0.25));
    assert_eq!(
        params.get_normalized_by_handle(ParamHandle::from_index(1)),
        None
    );
    assert!(matches!(
        params.set_normalized_by_handle(ParamHandle::from_index(1), 0.0),
        Err(ParamError::Unknown(id)) if id == "handle:1"
    ));
}

#[test]
fn invalid_param_handle_is_safe_fallback() {
    let params = ManualParams {
        gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
    };

    let handle = params.resolve_or_invalid("missing");

    assert!(handle.is_invalid());
    assert_eq!(handle.index(), ParamHandle::INVALID_INDEX);
    assert_eq!(params.get_normalized_by_handle(handle), None);
    let unknown_handle = format!("handle:{}", ParamHandle::INVALID_INDEX);
    assert!(matches!(
        params.set_normalized_by_handle(handle, 0.0),
        Err(ParamError::Unknown(id)) if id == unknown_handle
    ));
}

#[test]
fn converts_choice_values() {
    let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive", "Fuzz"], 1);
    assert_eq!(spec.step_count, Some(2));
    assert!((spec.default_normalized - 0.5).abs() < f64::EPSILON);
    assert_eq!(normalized_to_plain(&spec, 0.0), 0.0);
    assert_eq!(normalized_to_plain(&spec, 0.49), 1.0);
    assert_eq!(normalized_to_plain(&spec, 1.0), 2.0);
    assert_eq!(plain_to_normalized(&spec, 2.0), 1.0);
}

#[test]
fn param_spec_wire_schema_is_camel_case_with_legacy_aliases() {
    let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 1);
    let value = serde_json::to_value(&spec).unwrap();
    assert_eq!(value["kind"]["choice"]["values"][1], "Drive");
    assert_eq!(value["defaultNormalized"], 1.0);
    assert_eq!(value["stepCount"], 1);
    assert_eq!(value["flags"]["readOnly"], false);
    assert_eq!(value["flags"]["programChange"], false);
    assert_eq!(value["midiMappings"], serde_json::json!([]));
    assert!(value.get("default_normalized").is_none());
    assert!(value.get("step_count").is_none());
    assert!(value.get("midi_mappings").is_none());
    assert!(value["flags"].get("read_only").is_none());
    assert!(value["flags"].get("program_change").is_none());

    let legacy = serde_json::json!({
        "id": "mode",
        "name": "Mode",
        "kind": { "Choice": { "values": ["Clean", "Drive"] } },
        "default_normalized": 1.0,
        "unit": null,
        "step_count": 1,
        "flags": {
            "automatable": true,
            "bypass": false,
            "read_only": false
        }
    });
    let decoded: ParamSpec = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded, spec);

    let program_change: ParamSpec = serde_json::from_value(serde_json::json!({
        "id": "program",
        "name": "Program",
        "kind": { "choice": { "values": ["Init", "Lead"] } },
        "defaultNormalized": 0.0,
        "unit": null,
        "stepCount": 1,
        "flags": {
            "automatable": true,
            "bypass": false,
            "readOnly": false,
            "programChange": true
        },
        "midiMappings": []
    }))
    .unwrap();
    assert!(program_change.flags.program_change);

    let legacy_bool = serde_json::json!("Bool");
    assert_eq!(
        serde_json::from_value::<ParamKind>(legacy_bool).unwrap(),
        ParamKind::Bool
    );
}

#[test]
fn param_flag_builders_mark_bypass_read_only_and_non_automatable() {
    let read_only_spec = ParamSpec::float("meter", "Meter", 0.0, 1.0, 0.0).as_read_only();
    assert!(read_only_spec.flags.read_only);
    assert!(!read_only_spec.flags.automatable);

    let non_automatable_spec =
        ParamSpec::choice("quality", "Quality", ["Eco", "High"], 1).with_automatable(false);
    assert!(!non_automatable_spec.flags.automatable);
    assert!(!non_automatable_spec.flags.read_only);

    let float = FloatParam::new("meter", "Meter", 0.0, 1.0, 0.0)
        .with_unit("dB")
        .as_read_only();
    let float_spec = float.spec();
    assert_eq!(float_spec.unit.as_deref(), Some("dB"));
    assert!(float_spec.flags.read_only);
    assert!(!float_spec.flags.automatable);

    let bypass = BoolParam::new("bypass", "Bypass", false).as_bypass();
    assert!(bypass.spec().flags.bypass);
    assert!(
        BoolParam::bypass("hard_bypass", "Hard Bypass", false)
            .spec()
            .flags
            .bypass
    );

    let choice = ChoiceParam::new("mode", "Mode", ["Clean", "Drive"], 0).with_automatable(false);
    assert!(!choice.spec().flags.automatable);

    let program = ChoiceParam::new("program", "Program", ["Init", "Lead"], 0)
        .with_automatable(false)
        .as_program_change();
    assert!(program.spec().flags.program_change);
    assert!(program.spec().flags.automatable);
}

#[test]
fn param_midi_mapping_builders_and_validation() {
    let spec = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5)
        .with_midi_cc(7)
        .with_channel_midi_cc(midi::PITCH_BEND, 2);
    assert_eq!(
        spec.midi_mappings,
        vec![
            ParamMidiMapping {
                controller: 7,
                channel: None
            },
            ParamMidiMapping {
                controller: midi::PITCH_BEND,
                channel: Some(2)
            }
        ]
    );
    validate_param_specs(&[spec]).unwrap();

    let float = FloatParam::new("cutoff", "Cutoff", 20.0, 20_000.0, 1_000.0).with_midi_cc(74);
    assert_eq!(float.spec().midi_mappings[0].controller, 74);

    let bool_param = BoolParam::new("hold", "Hold", false).with_channel_midi_cc(64, 1);
    assert_eq!(bool_param.spec().midi_mappings[0].channel, Some(1));

    let choice_param =
        ChoiceParam::new("program", "Program", ["A", "B"], 0).with_midi_cc(midi::PROGRAM_CHANGE);
    assert_eq!(
        choice_param.spec().midi_mappings[0].controller,
        midi::PROGRAM_CHANGE
    );
}

#[test]
fn validates_param_specs_for_host_and_bridge_schema() {
    let specs = vec![
        ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
        ParamSpec::bool("bypass", "Bypass", false).as_bypass(),
        ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 0),
    ];
    validate_param_specs(&specs).unwrap();
    assert!(ParamRegistry::try_new(specs).is_ok());

    let duplicate = vec![
        ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5),
        ParamSpec::bool("gain", "Gain Toggle", false),
    ];
    assert!(matches!(
        validate_param_specs(&duplicate),
        Err(ParamSpecError::DuplicateId { id }) if id == "gain"
    ));

    let empty_id = vec![ParamSpec::float(" ", "Gain", 0.0, 1.0, 0.5)];
    assert!(matches!(
        validate_param_specs(&empty_id),
        Err(ParamSpecError::EmptyId { index: 0 })
    ));

    let mut control = ParamSpec::float("gain", "Gain\nBad", 0.0, 1.0, 0.5);
    assert!(matches!(
        validate_param_specs(&[control.clone()]),
        Err(ParamSpecError::ControlCharacter { field: "name", .. })
    ));
    control.name = "Gain".to_string();
    control.unit = Some("dB\nBad".to_string());
    assert!(matches!(
        validate_param_specs(&[control]),
        Err(ParamSpecError::ControlCharacter { field: "unit", .. })
    ));

    let invalid_default = ParamSpec {
        default_normalized: 1.1,
        ..ParamSpec::bool("toggle", "Toggle", false)
    };
    assert!(matches!(
        validate_param_specs(&[invalid_default]),
        Err(ParamSpecError::InvalidDefaultNormalized { .. })
    ));

    let invalid_range = ParamSpec::float("flat", "Flat", 1.0, 1.0, 1.0);
    assert!(matches!(
        validate_param_specs(&[invalid_range]),
        Err(ParamSpecError::InvalidFloatRange { .. })
    ));

    let empty_choice = ParamSpec::choice("mode", "Mode", ["Clean", ""], 0);
    assert!(matches!(
        validate_param_specs(&[empty_choice]),
        Err(ParamSpecError::EmptyChoiceLabel { index: 1, .. })
    ));

    let read_only_automatable = ParamSpec {
        flags: ParamFlags {
            automatable: true,
            bypass: false,
            read_only: true,
            program_change: false,
        },
        ..ParamSpec::float("meter", "Meter", 0.0, 1.0, 0.0)
    };
    assert!(matches!(
        validate_param_specs(&[read_only_automatable]),
        Err(ParamSpecError::ReadOnlyAutomatable { id }) if id == "meter"
    ));

    let invalid_midi_controller =
        ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0).with_midi_cc(999);
    assert!(matches!(
        validate_param_specs(&[invalid_midi_controller]),
        Err(ParamSpecError::InvalidMidiController {
            controller: 999,
            ..
        })
    ));

    let invalid_midi_channel =
        ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0).with_channel_midi_cc(7, 16);
    assert!(matches!(
        validate_param_specs(&[invalid_midi_channel]),
        Err(ParamSpecError::InvalidMidiChannel { channel: 16, .. })
    ));

    let duplicate_midi_mapping = ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0)
        .with_midi_cc(7)
        .with_midi_cc(7);
    assert!(matches!(
        validate_param_specs(&[duplicate_midi_mapping]),
        Err(ParamSpecError::DuplicateMidiMapping { controller: 7, .. })
    ));
}

#[test]
fn formats_and_parses_choice_labels() {
    let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0);
    assert_eq!(format_normalized_value(&spec, 0.0), "Clean");
    assert_eq!(format_normalized_value(&spec, 0.51), "Drive");
    assert_eq!(format_normalized_value(&spec, 1.0), "Fuzz");
    assert_eq!(parse_normalized_value(&spec, "Drive"), Some(0.5));
    assert_eq!(parse_normalized_value(&spec, "fuzz"), Some(1.0));
    assert_eq!(parse_normalized_value(&spec, "0"), Some(0.0));
    assert_eq!(parse_normalized_value(&spec, "missing"), None);
}

#[test]
fn choice_param_snaps_to_nearest_index() {
    let param = ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 99);
    assert_eq!(param.index(), 2);
    assert_eq!(param.value(), Some("Fuzz"));
    assert_eq!(param.normalized(), 1.0);

    param.set_normalized(0.1);
    assert_eq!(param.index(), 0);
    assert_eq!(param.value(), Some("Clean"));
    assert_eq!(param.normalized(), 0.0);

    param.set_normalized(0.74);
    assert_eq!(param.index(), 1);
    assert_eq!(param.value(), Some("Drive"));
    assert_eq!(param.normalized(), 0.5);

    param.set_normalized(0.75);
    assert_eq!(param.index(), 2);
    assert_eq!(param.value(), Some("Fuzz"));
    assert_eq!(param.normalized(), 1.0);
}

#[test]
fn empty_choice_param_is_safe() {
    let param = ChoiceParam::new("empty", "Empty", std::iter::empty::<&str>(), 4);
    assert_eq!(param.index(), 0);
    assert_eq!(param.value(), None);
    assert_eq!(param.normalized(), 0.0);
    assert_eq!(param.spec().step_count, Some(0));
}

#[test]
fn smoother_reaches_target() {
    let mut smoother = LinearSmoother::new(0.0);
    smoother.set_target(1.0, 4);
    let values: Vec<f32> = (0..4).map(|_| smoother.next_value()).collect();
    assert_eq!(values, vec![0.25, 0.5, 0.75, 1.0]);
}
