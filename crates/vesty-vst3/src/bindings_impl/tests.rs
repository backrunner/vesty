use super::*;
use vesty_params::{ParamHandle, ParamSpec};

#[test]
fn vst3_state_migration_accepts_v1_and_rejects_future_versions() {
    let state = Vst3State {
        version: VST3_STATE_VERSION,
        params: vec![ParamState {
            id: "gain".to_string(),
            normalized: 0.5,
        }],
        custom: None,
        bridge: None,
    };
    assert_eq!(migrate_vst3_state(state).unwrap().params[0].id, "gain");

    let future = Vst3State {
        version: VST3_STATE_VERSION + 1,
        params: Vec::new(),
        custom: None,
        bridge: None,
    };
    let error = migrate_vst3_state(future).unwrap_err().to_string();
    assert!(error.contains("unsupported VST3 state version"));
}

#[test]
fn stable_vst3_param_ids_are_derived_from_string_ids() {
    let gain = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5);
    let mode = ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 0);
    let first = Vst3ParamIds::try_from_specs(&[gain.clone(), mode.clone()]).unwrap();
    let second = Vst3ParamIds::try_from_specs(&[mode, gain]).unwrap();

    let gain_host_id = stable_vst3_param_id("gain");
    let mode_host_id = stable_vst3_param_id("mode");
    assert_ne!(gain_host_id, 0);
    assert_ne!(gain_host_id, mode_host_id);
    assert_eq!(first.host_id_for_index(0), Some(gain_host_id));
    assert_eq!(first.index_for_host_id(gain_host_id), Some(0));
    assert_eq!(second.host_id_for_index(1), Some(gain_host_id));
    assert_eq!(second.index_for_host_id(gain_host_id), Some(1));
}

#[test]
fn stable_vst3_param_id_registry_rejects_collisions() {
    let first = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5);
    let second = ParamSpec::float("gain", "Duplicate Gain", 0.0, 1.0, 0.5);
    let error = Vst3ParamIds::try_from_specs(&[first, second]).unwrap_err();
    assert_eq!(error.host_id, stable_vst3_param_id("gain"));
    assert_eq!(error.first_id, "gain");
    assert_eq!(error.second_id, "gain");
}

#[test]
fn attribute_list_roundtrips_int_float_string_and_binary() {
    let list = VestyAttributeList::default();

    let mut int_value = 0;
    let mut float_value = 0.0;
    let string_value: Vec<TChar> = "Vesty"
        .encode_utf16()
        .map(|unit| unit as TChar)
        .chain(std::iter::once(0))
        .collect();
    let binary_value = [1_u8, 3, 5, 8, 13];

    // SAFETY: The test passes valid nul-terminated attribute IDs, output pointers, and
    // UTF-16 string/binary buffers for the duration of each direct COM trait call.
    unsafe {
        assert_eq!(list.setInt(c"int".as_ptr() as IAttrID, 42), kResultOk);
        assert_eq!(
            list.getInt(c"int".as_ptr() as IAttrID, &mut int_value),
            kResultOk
        );
        assert_eq!(int_value, 42);

        assert_eq!(
            list.setFloat(c"float".as_ptr() as IAttrID, 0.625),
            kResultOk
        );
        assert_eq!(
            list.getFloat(c"float".as_ptr() as IAttrID, &mut float_value),
            kResultOk
        );
        assert_eq!(float_value, 0.625);

        assert_eq!(
            list.setString(c"string".as_ptr() as IAttrID, string_value.as_ptr()),
            kResultOk
        );
        let mut string_out = [0 as TChar; 16];
        assert_eq!(
            list.getString(
                c"string".as_ptr() as IAttrID,
                string_out.as_mut_ptr(),
                (string_out.len() * size_of::<TChar>()) as uint32,
            ),
            kResultOk
        );
        assert_eq!(&string_out[..string_value.len()], string_value.as_slice());

        let mut tiny_string_out = [99 as TChar; 3];
        assert_eq!(
            list.getString(
                c"string".as_ptr() as IAttrID,
                tiny_string_out.as_mut_ptr(),
                (tiny_string_out.len() * size_of::<TChar>()) as uint32,
            ),
            kResultFalse
        );
        assert_eq!(tiny_string_out.last().copied(), Some(0));

        assert_eq!(
            list.setBinary(
                c"binary".as_ptr() as IAttrID,
                binary_value.as_ptr() as *const c_void,
                binary_value.len() as uint32,
            ),
            kResultOk
        );
        let mut data = std::ptr::null();
        let mut size = 0;
        assert_eq!(
            list.getBinary(c"binary".as_ptr() as IAttrID, &mut data, &mut size),
            kResultOk
        );
        assert_eq!(size as usize, binary_value.len());
        assert_eq!(
            slice::from_raw_parts(data as *const u8, size as usize),
            binary_value
        );
    }
}

#[test]
fn attribute_list_rejects_invalid_pointers_and_missing_values() {
    let list = VestyAttributeList::default();
    let string_value: Vec<TChar> = "Vesty"
        .encode_utf16()
        .map(|unit| unit as TChar)
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: The test intentionally passes null pointers to verify defensive COM boundary
    // handling; non-null inputs are valid for the duration of each direct trait call.
    unsafe {
        assert_eq!(list.setInt(std::ptr::null(), 42), kInvalidArgument);
        assert_eq!(
            list.getInt(c"missing".as_ptr() as IAttrID, std::ptr::null_mut()),
            kInvalidArgument
        );
        let mut int_value = 0;
        assert_eq!(
            list.getInt(c"missing".as_ptr() as IAttrID, &mut int_value),
            kInvalidArgument
        );

        assert_eq!(list.setFloat(std::ptr::null(), 1.0), kInvalidArgument);
        assert_eq!(
            list.getFloat(c"missing".as_ptr() as IAttrID, std::ptr::null_mut()),
            kInvalidArgument
        );
        let mut float_value = 0.0;
        assert_eq!(
            list.getFloat(c"missing".as_ptr() as IAttrID, &mut float_value),
            kInvalidArgument
        );

        assert_eq!(
            list.setString(c"string".as_ptr() as IAttrID, std::ptr::null()),
            kInvalidArgument
        );
        assert_eq!(
            list.setString(std::ptr::null(), string_value.as_ptr()),
            kInvalidArgument
        );
        let mut string_out = [0 as TChar; 4];
        assert_eq!(
            list.getString(
                c"missing".as_ptr() as IAttrID,
                string_out.as_mut_ptr(),
                (string_out.len() * size_of::<TChar>()) as uint32,
            ),
            kInvalidArgument
        );
        assert_eq!(
            list.getString(
                c"missing".as_ptr() as IAttrID,
                std::ptr::null_mut(),
                (string_out.len() * size_of::<TChar>()) as uint32,
            ),
            kInvalidArgument
        );
        assert_eq!(
            list.getString(c"missing".as_ptr() as IAttrID, string_out.as_mut_ptr(), 1,),
            kInvalidArgument
        );

        assert_eq!(
            list.setBinary(c"binary".as_ptr() as IAttrID, std::ptr::null(), 1),
            kInvalidArgument
        );
        assert_eq!(
            list.setBinary(c"empty".as_ptr() as IAttrID, std::ptr::null(), 0),
            kResultOk
        );
        let mut data = std::ptr::null();
        let mut size = uint32::MAX;
        assert_eq!(
            list.getBinary(c"empty".as_ptr() as IAttrID, &mut data, &mut size),
            kResultOk
        );
        assert_eq!(size, 0);
        assert!(data.is_null());
        assert_eq!(
            list.getBinary(c"missing".as_ptr() as IAttrID, &mut data, &mut size),
            kInvalidArgument
        );
        assert_eq!(
            list.getBinary(
                c"empty".as_ptr() as IAttrID,
                std::ptr::null_mut(),
                &mut size,
            ),
            kInvalidArgument
        );
        assert_eq!(
            list.getBinary(
                c"empty".as_ptr() as IAttrID,
                &mut data,
                std::ptr::null_mut(),
            ),
            kInvalidArgument
        );
    }
}

#[test]
fn events_are_sorted_by_sample_offset_stably() {
    let mut events = FixedEventList::<VestyEvent, MAX_BLOCK_EVENTS>::new();
    let gain = ParamHandle::from_index(0);
    events
        .push(VestyEvent::Param {
            sample_offset: 16,
            handle: gain,
            id_hash: 1,
            normalized: 0.25,
        })
        .unwrap();
    events
        .push(VestyEvent::NoteOn {
            sample_offset: 4,
            channel: 0,
            key: 60,
            velocity: 1.0,
            note_id: -1,
        })
        .unwrap();
    events
        .push(VestyEvent::Param {
            sample_offset: 4,
            handle: gain,
            id_hash: 1,
            normalized: 0.5,
        })
        .unwrap();
    events
        .push(VestyEvent::NoteOff {
            sample_offset: 32,
            channel: 0,
            key: 60,
            velocity: 0.0,
            note_id: -1,
        })
        .unwrap();

    sort_events_by_sample_offset(&mut events);

    assert_eq!(
        events.as_slice(),
        &[
            VestyEvent::NoteOn {
                sample_offset: 4,
                channel: 0,
                key: 60,
                velocity: 1.0,
                note_id: -1,
            },
            VestyEvent::Param {
                sample_offset: 4,
                handle: gain,
                id_hash: 1,
                normalized: 0.5,
            },
            VestyEvent::Param {
                sample_offset: 16,
                handle: gain,
                id_hash: 1,
                normalized: 0.25,
            },
            VestyEvent::NoteOff {
                sample_offset: 32,
                channel: 0,
                key: 60,
                velocity: 0.0,
                note_id: -1,
            },
        ]
    );
}

#[test]
fn sysex_copy_uses_fixed_buffer_and_reports_truncation() {
    let mut long = [0_u8; MAX_SYSEX_BYTES + 4];
    for (index, byte) in long.iter_mut().enumerate() {
        *byte = (index & 0x7f) as u8;
    }

    // SAFETY: Test data points to a stack array that remains alive for the duration of the copy.
    let (data, data_len, truncated) = unsafe { copy_sysex_data(long.as_ptr(), long.len() as u32) };
    assert_eq!(data_len as usize, MAX_SYSEX_BYTES);
    assert!(truncated);
    assert_eq!(&data[..8], &long[..8]);
    assert_eq!(
        &data[MAX_SYSEX_BYTES - 4..],
        &long[MAX_SYSEX_BYTES - 4..MAX_SYSEX_BYTES]
    );

    // SAFETY: Null pointer with a non-zero declared size is treated as an empty truncated event.
    let (data, data_len, truncated) = unsafe { copy_sysex_data(std::ptr::null(), 4) };
    assert_eq!(data_len, 0);
    assert!(truncated);
    assert!(data.iter().all(|byte| *byte == 0));
}
