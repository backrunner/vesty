use super::*;

#[test]
fn factory_reports_processor_and_controller_classes() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        assert_eq!(factory.countClasses(), 2);

        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
        let mut processor = MaybeUninit::<PClassInfo>::zeroed();
        assert_eq!(factory.getClassInfo(0, processor.as_mut_ptr()), kResultOk);
        let processor = processor.assume_init();
        assert_eq!(processor.cid, tuid(metadata.processor_class_id));

        let mut controller = MaybeUninit::<PClassInfo>::zeroed();
        assert_eq!(factory.getClassInfo(1, controller.as_mut_ptr()), kResultOk);
        let controller = controller.assume_init();
        assert_eq!(controller.cid, tuid(metadata.controller_class_id));
    }
}

#[test]
fn factory_rejects_null_pointers_and_clears_failed_instance_output() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host pointers.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
        let processor_cid = tuid(metadata.processor_class_id);

        assert_eq!(factory.getFactoryInfo(ptr::null_mut()), kInvalidArgument);
        assert_eq!(factory.getClassInfo(0, ptr::null_mut()), kInvalidArgument);

        let stale = ptr::dangling_mut::<c_void>();
        let mut instance = stale;
        assert_eq!(
            factory.createInstance(ptr::null(), IComponent_iid.as_ptr(), &mut instance,),
            kInvalidArgument
        );
        assert!(instance.is_null());

        instance = stale;
        assert_eq!(
            factory.createInstance(processor_cid.as_ptr(), ptr::null(), &mut instance),
            kInvalidArgument
        );
        assert!(instance.is_null());

        assert_eq!(
            factory.createInstance(
                processor_cid.as_ptr(),
                IComponent_iid.as_ptr(),
                ptr::null_mut(),
            ),
            kInvalidArgument
        );

        let unknown_cid = [42 as c_char; 16];
        instance = stale;
        assert_eq!(
            factory.createInstance(unknown_cid.as_ptr(), IComponent_iid.as_ptr(), &mut instance,),
            kInvalidArgument
        );
        assert!(instance.is_null());
    }
}

#[test]
fn factory_creates_component_and_controller_instances() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();

        let processor_cid = tuid(metadata.processor_class_id);
        let mut component: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                processor_cid.as_ptr(),
                IComponent_iid.as_ptr(),
                &mut component,
            ),
            kResultOk
        );
        let component =
            ComPtr::<IComponent>::from_raw(component as *mut IComponent).expect("component");
        assert_eq!(
            component.getBusCount(
                MediaTypes_::kAudio as MediaType,
                BusDirections_::kInput as BusDirection,
            ),
            1
        );

        let controller_cid = tuid(metadata.controller_class_id);
        let mut controller: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IEditController_iid.as_ptr(),
                &mut controller,
            ),
            kResultOk
        );
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
            .expect("controller");
        assert_eq!(controller.getParameterCount(), 2);

        let mut midi_mapping: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IMidiMapping_iid.as_ptr(),
                &mut midi_mapping,
            ),
            kResultOk
        );
        let _midi_mapping = ComPtr::<IMidiMapping>::from_raw(midi_mapping as *mut IMidiMapping)
            .expect("midi mapping");

        let mut info = MaybeUninit::<ParameterInfo>::zeroed();
        assert_eq!(controller.getParameterInfo(0, info.as_mut_ptr()), kResultOk);
        let info = info.assume_init();
        assert_eq!(info.id, test_param_id("gain"));
        assert_eq!(
            info.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
            ParameterInfo_::ParameterFlags_::kCanAutomate
        );
    }
}

#[test]
fn controller_exposes_opt_in_midi_mapping() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);
        let mut mapping: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IMidiMapping_iid.as_ptr(),
                &mut mapping,
            ),
            kResultOk
        );
        let mapping =
            ComPtr::<IMidiMapping>::from_raw(mapping as *mut IMidiMapping).expect("midi mapping");

        let mut id = ParamID::MAX;
        assert_eq!(
            mapping.getMidiControllerAssignment(0, 9, 7, &mut id),
            kResultTrue
        );
        assert_eq!(id, test_param_id("gain"));

        id = ParamID::MAX;
        assert_eq!(
            mapping.getMidiControllerAssignment(0, 1, 74, &mut id),
            kResultFalse
        );
        assert_eq!(id, ParamID::MAX);

        assert_eq!(
            mapping.getMidiControllerAssignment(0, 2, 74, &mut id),
            kResultTrue
        );
        assert_eq!(id, test_param_id("cutoff"));

        assert_eq!(
            mapping.getMidiControllerAssignment(
                0,
                1,
                vesty_params::midi::PITCH_BEND as CtrlNumber,
                &mut id,
            ),
            kResultTrue
        );
        assert_eq!(id, test_param_id("pitch"));

        assert_eq!(
            mapping.getMidiControllerAssignment(0, 0, 10, &mut id),
            kResultFalse
        );
        assert_eq!(
            mapping.getMidiControllerAssignment(1, 9, 7, &mut id),
            kResultFalse
        );
        assert_eq!(
            mapping.getMidiControllerAssignment(0, -1, 7, &mut id),
            kResultFalse
        );
        assert_eq!(
            mapping.getMidiControllerAssignment(0, 0, -1, &mut id),
            kResultFalse
        );
        assert_eq!(
            mapping.getMidiControllerAssignment(0, 0, 7, ptr::null_mut()),
            kInvalidArgument
        );
    }
}

#[test]
fn controller_exposes_program_list_metadata() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);
        let mut unit_info: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IUnitInfo_iid.as_ptr(),
                &mut unit_info,
            ),
            kResultOk
        );
        let unit_info =
            ComPtr::<IUnitInfo>::from_raw(unit_info as *mut IUnitInfo).expect("unit info");
        let controller = unit_info
            .cast::<IEditController>()
            .expect("same controller edit interface");
        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );
        let gain_id = test_param_id("gain");
        let cutoff_id = test_param_id("cutoff");
        let pitch_id = test_param_id("pitch");

        assert_eq!(unit_info.getUnitCount(), 1);
        let mut root = MaybeUninit::<UnitInfo>::zeroed();
        assert_eq!(unit_info.getUnitInfo(0, root.as_mut_ptr()), kResultOk);
        let root = root.assume_init();
        assert_eq!(root.id, kRootUnitId);
        assert_eq!(root.parentUnitId, kNoParentUnitId);
        assert_eq!(root.programListId, 77);
        assert_eq!(string128_to_string(&root.name), "Midi Mapped");
        assert_eq!(unit_info.getSelectedUnit(), kRootUnitId);
        assert_eq!(unit_info.selectUnit(kRootUnitId), kResultOk);
        assert_eq!(unit_info.selectUnit(99), kInvalidArgument);

        assert_eq!(unit_info.getProgramListCount(), 1);
        let mut list = MaybeUninit::<ProgramListInfo>::zeroed();
        assert_eq!(
            unit_info.getProgramListInfo(0, list.as_mut_ptr()),
            kResultOk
        );
        let list = list.assume_init();
        assert_eq!(list.id, 77);
        assert_eq!(list.programCount, 3);
        assert_eq!(string128_to_string(&list.name), "Factory Programs");

        let mut name = [0; 128];
        assert_eq!(unit_info.getProgramName(77, 1, &mut name), kResultOk);
        assert_eq!(string128_to_string(&name), "Bright Lead");
        assert_eq!(unit_info.getProgramName(77, 3, &mut name), kInvalidArgument);
        assert_eq!(
            unit_info.getProgramName(999, 0, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 1, c"category".as_ptr() as FIDString, &mut name),
            kResultOk
        );
        assert_eq!(string128_to_string(&name), "Lead");
        assert_eq!(
            unit_info.getProgramInfo(77, 1, c"mood".as_ptr() as FIDString, &mut name),
            kResultOk
        );
        assert_eq!(string128_to_string(&name), "Bright");
        assert_eq!(
            unit_info.getProgramInfo(77, 1, c"unknown".as_ptr() as FIDString, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 1, c"invalid_value".as_ptr() as FIDString, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 0, c"category".as_ptr() as FIDString, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 3, c"category".as_ptr() as FIDString, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramInfo(999, 1, c"category".as_ptr() as FIDString, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 1, ptr::null(), &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramInfo(77, 1, c"category".as_ptr() as FIDString, ptr::null_mut()),
            kInvalidArgument
        );

        assert_eq!(unit_info.hasProgramPitchNames(77, 0), kResultFalse);
        assert_eq!(unit_info.hasProgramPitchNames(77, 1), kResultTrue);
        assert_eq!(unit_info.hasProgramPitchNames(77, 3), kInvalidArgument);
        assert_eq!(unit_info.hasProgramPitchNames(999, 1), kInvalidArgument);
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 60, &mut name),
            kResultOk
        );
        assert_eq!(string128_to_string(&name), "C4 Lead");
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 64, &mut name),
            kResultOk
        );
        assert_eq!(string128_to_string(&name), "E4 Lead");
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 63, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 62, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 0, 60, &mut name),
            kResultFalse
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 3, 60, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramPitchName(999, 1, 60, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, -1, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 128, &mut name),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.getProgramPitchName(77, 1, 60, ptr::null_mut()),
            kInvalidArgument
        );

        let mut unit_id = -99;
        assert_eq!(
            unit_info.getUnitByBus(
                MediaTypes_::kEvent as MediaType,
                BusDirections_::kInput as BusDirection,
                0,
                -1,
                &mut unit_id,
            ),
            kResultOk
        );
        assert_eq!(unit_id, kRootUnitId);
        assert_eq!(
            unit_info.getUnitByBus(
                MediaTypes_::kAudio as MediaType,
                BusDirections_::kInput as BusDirection,
                0,
                -1,
                &mut unit_id,
            ),
            kInvalidArgument
        );

        assert_eq!(
            unit_info.setUnitProgramData(77, 1, ptr::null_mut()),
            kResultOk
        );
        assert!((controller.getParamNormalized(gain_id) - 0.8).abs() < 0.000_001);
        assert!((controller.getParamNormalized(cutoff_id) - 0.9).abs() < 0.000_001);
        assert!((controller.getParamNormalized(pitch_id) - 0.65).abs() < 0.000_001);
        assert_eq!(
            handler.calls(),
            vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
        );

        assert_eq!(
            unit_info.setUnitProgramData(kRootUnitId, 2, ptr::null_mut()),
            kResultOk
        );
        assert!((controller.getParamNormalized(gain_id) - 0.3).abs() < 0.000_001);
        assert!((controller.getParamNormalized(cutoff_id) - 0.35).abs() < 0.000_001);
        assert!((controller.getParamNormalized(pitch_id) - 0.4).abs() < 0.000_001);
        assert_eq!(
            unit_info.setUnitProgramData(999, 0, ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            unit_info.setUnitProgramData(77, 99, ptr::null_mut()),
            kInvalidArgument
        );
    }
}

#[test]
fn controller_supports_program_data_streams() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);
        let mut program_list_data: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IProgramListData_iid.as_ptr(),
                &mut program_list_data,
            ),
            kResultOk
        );
        let program_list_data =
            ComPtr::<IProgramListData>::from_raw(program_list_data as *mut IProgramListData)
                .expect("program list data");
        let controller = program_list_data
            .cast::<IEditController>()
            .expect("same controller edit interface");
        let unit_info = program_list_data
            .cast::<IUnitInfo>()
            .expect("same controller unit info interface");
        let gain_id = test_param_id("gain");
        let cutoff_id = test_param_id("cutoff");
        let pitch_id = test_param_id("pitch");
        let assert_param = |id: ParamID, expected: f64| {
            assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
        };

        assert_eq!(program_list_data.programDataSupported(77), kResultTrue);
        assert_eq!(
            program_list_data.programDataSupported(999),
            kInvalidArgument
        );

        assert_eq!(controller.setParamNormalized(gain_id, 0.42), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.84), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.21), kResultOk);

        let saved = ComWrapper::new(MemoryStream::default());
        let saved_ptr = saved.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.getProgramData(77, 1, saved_ptr.as_ptr()),
            kResultOk
        );
        let saved_bytes = saved.bytes();
        let saved_json = program_data_json(&saved_bytes);
        assert_eq!(saved_json["version"], 1);
        assert_eq!(saved_json["listId"], 77);
        assert_eq!(saved_json["programIndex"], 1);
        assert!((saved_json["data"]["gain"].as_f64().unwrap() - 0.42).abs() < 0.000_001);
        assert!((saved_json["data"]["cutoff"].as_f64().unwrap() - 0.84).abs() < 0.000_001);
        assert!((saved_json["data"]["pitch"].as_f64().unwrap() - 0.21).abs() < 0.000_001);

        assert_eq!(controller.setParamNormalized(gain_id, 0.1), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.2), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.3), kResultOk);
        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );
        let input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
        let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, input_ptr.as_ptr()),
            kResultOk
        );
        assert_param(gain_id, 0.42);
        assert_param(cutoff_id, 0.84);
        assert_param(pitch_id, 0.21);
        assert_eq!(
            handler.calls(),
            vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
        );

        assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);
        let unit_input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
        let unit_input_ptr = unit_input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            unit_info.setUnitProgramData(77, 1, unit_input_ptr.as_ptr()),
            kResultOk
        );
        assert_param(gain_id, 0.42);
        assert_param(cutoff_id, 0.84);
        assert_param(pitch_id, 0.21);

        assert_eq!(controller.setParamNormalized(gain_id, 0.14), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.24), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.34), kResultOk);
        let root_input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
        let root_input_ptr = root_input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            unit_info.setUnitProgramData(kRootUnitId, 1, root_input_ptr.as_ptr()),
            kResultOk
        );
        assert_param(gain_id, 0.42);
        assert_param(cutoff_id, 0.84);
        assert_param(pitch_id, 0.21);

        let out_of_range = ComWrapper::new(MemoryStream::default());
        let out_of_range_ptr = out_of_range.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.getProgramData(77, 99, out_of_range_ptr.as_ptr()),
            kInvalidArgument
        );
        let unknown_list = ComWrapper::new(MemoryStream::default());
        let unknown_list_ptr = unknown_list.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.getProgramData(999, 0, unknown_list_ptr.as_ptr()),
            kInvalidArgument
        );
        assert_eq!(
            program_list_data.getProgramData(77, 1, ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            program_list_data.setProgramData(77, 1, ptr::null_mut()),
            kInvalidArgument
        );

        let bad_magic = ComWrapper::new(MemoryStream::with_bytes(b"bad".to_vec()));
        let bad_magic_ptr = bad_magic.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, bad_magic_ptr.as_ptr()),
            kInvalidArgument
        );

        let future_version = ComWrapper::new(MemoryStream::with_bytes(
            raw_program_data_bytes_with_version(
                2,
                77,
                1,
                serde_json::json!({
                    "gain": 0.42,
                    "cutoff": 0.84,
                    "pitch": 0.21,
                }),
            ),
        ));
        let future_version_ptr = future_version.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, future_version_ptr.as_ptr()),
            kInvalidArgument
        );

        let list_mismatch = ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
            78,
            1,
            serde_json::json!({
                "gain": 0.42,
                "cutoff": 0.84,
                "pitch": 0.21,
            }),
        )));
        let list_mismatch_ptr = list_mismatch.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, list_mismatch_ptr.as_ptr()),
            kInvalidArgument
        );

        let program_mismatch = ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
            77,
            2,
            serde_json::json!({
                "gain": 0.42,
                "cutoff": 0.84,
                "pitch": 0.21,
            }),
        )));
        let program_mismatch_ptr = program_mismatch.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, program_mismatch_ptr.as_ptr()),
            kInvalidArgument
        );

        let missing_data = ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
            77,
            1,
            serde_json::json!({ "gain": 0.42 }),
        )));
        let missing_data_ptr = missing_data.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            program_list_data.setProgramData(77, 1, missing_data_ptr.as_ptr()),
            kResultFalse
        );
    }
}

#[test]
fn controller_program_change_param_selects_visible_program() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
        let gain_id = test_param_id("gain");
        let cutoff_id = test_param_id("cutoff");
        let pitch_id = test_param_id("pitch");
        let program_id = test_param_id("program");
        let assert_param = |id: ParamID, expected: f64| {
            assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
        };

        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );

        assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);

        assert_eq!(controller.setParamNormalized(program_id, 0.5), kResultOk);
        assert_param(gain_id, 0.8);
        assert_param(cutoff_id, 0.9);
        assert_param(pitch_id, 0.65);
        assert_param(program_id, 0.5);
        assert_eq!(
            handler.calls(),
            vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
        );
    }
}

#[test]
fn controller_program_change_param_gesture_selects_visible_program() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
        let gain_id = test_param_id("gain");
        let cutoff_id = test_param_id("cutoff");
        let pitch_id = test_param_id("pitch");
        let program_id = test_param_id("program");
        let assert_param = |id: ParamID, expected: f64| {
            assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
        };

        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );

        assert_eq!(controller.perform_param_edit(program_id, 1.0), kResultOk);
        assert_param(gain_id, 0.3);
        assert_param(cutoff_id, 0.35);
        assert_param(pitch_id, 0.4);
        assert_param(program_id, 1.0);
        assert_eq!(
            handler.calls(),
            vec![
                HandlerCall::Perform(program_id, 1.0),
                HandlerCall::Restart(RestartFlags_::kParamValuesChanged),
            ]
        );
    }
}

#[test]
fn controller_exposes_note_expression_metadata() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);
        let mut note_expression: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                INoteExpressionController_iid.as_ptr(),
                &mut note_expression,
            ),
            kResultOk
        );
        let mut physical_mapping: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                INoteExpressionPhysicalUIMapping_iid.as_ptr(),
                &mut physical_mapping,
            ),
            kResultOk
        );
        let note_expression = ComPtr::<INoteExpressionController>::from_raw(
            note_expression as *mut INoteExpressionController,
        )
        .expect("note expression controller");
        let physical_mapping = ComPtr::<INoteExpressionPhysicalUIMapping>::from_raw(
            physical_mapping as *mut INoteExpressionPhysicalUIMapping,
        )
        .expect("note expression physical ui mapping");

        assert_eq!(note_expression.getNoteExpressionCount(0, -1), 2);
        assert_eq!(note_expression.getNoteExpressionCount(0, 15), 2);
        assert_eq!(note_expression.getNoteExpressionCount(1, 0), 0);
        assert_eq!(note_expression.getNoteExpressionCount(0, 16), 0);

        let mut info = MaybeUninit::<NoteExpressionTypeInfo>::zeroed();
        assert_eq!(
            note_expression.getNoteExpressionInfo(0, 0, 0, info.as_mut_ptr()),
            kResultOk
        );
        let expression_info = info.assume_init();
        assert_eq!(
            expression_info.typeId,
            NoteExpressionTypeIDs_::kBrightnessTypeID
        );
        assert_eq!(string128_to_string(&expression_info.title), "Brightness");
        assert_eq!(string128_to_string(&expression_info.shortTitle), "Bright");
        assert_eq!(expression_info.valueDesc.defaultValue, 0.5);
        assert_eq!(expression_info.valueDesc.minimum, 0.0);
        assert_eq!(expression_info.valueDesc.maximum, 1.0);
        let absolute_flag = NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsAbsolute as int32;
        assert_eq!(expression_info.flags & absolute_flag, absolute_flag);
        assert_eq!(expression_info.associatedParameterId, kNoParamId);

        assert_eq!(
            note_expression.getNoteExpressionInfo(0, 0, 2, info.as_mut_ptr()),
            kInvalidArgument
        );
        assert_eq!(
            note_expression.getNoteExpressionInfo(1, 0, 0, info.as_mut_ptr()),
            kInvalidArgument
        );
        assert_eq!(
            note_expression.getNoteExpressionInfo(0, 0, 0, ptr::null_mut()),
            kInvalidArgument
        );

        let mut text = [0; 128];
        assert_eq!(
            note_expression.getNoteExpressionStringByValue(
                0,
                0,
                NoteExpressionTypeIDs_::kBrightnessTypeID,
                0.625,
                &mut text,
            ),
            kResultOk
        );
        assert_eq!(string128_to_string(&text), "0.625");

        let parse_input = wide_cstring("0.875");
        let mut parsed = 0.0;
        assert_eq!(
            note_expression.getNoteExpressionValueByString(
                0,
                0,
                NoteExpressionTypeIDs_::kBrightnessTypeID,
                parse_input.as_ptr(),
                &mut parsed,
            ),
            kResultOk
        );
        assert_eq!(parsed, 0.875);

        let mut bounded_parse_input = [b' ' as u16; 128];
        for (index, unit) in "0.5".encode_utf16().enumerate() {
            bounded_parse_input[index] = unit;
        }
        parsed = 0.0;
        assert_eq!(
            note_expression.getNoteExpressionValueByString(
                0,
                0,
                NoteExpressionTypeIDs_::kBrightnessTypeID,
                bounded_parse_input.as_ptr(),
                &mut parsed,
            ),
            kResultOk
        );
        assert_eq!(parsed, 0.5);
        assert_eq!(
            note_expression.getNoteExpressionStringByValue(
                0,
                0,
                NoteExpressionTypeIDs_::kInvalidTypeID,
                0.5,
                &mut text,
            ),
            kInvalidArgument
        );
        assert_eq!(
            note_expression.getNoteExpressionValueByString(
                0,
                0,
                NoteExpressionTypeIDs_::kInvalidTypeID,
                parse_input.as_ptr(),
                &mut parsed,
            ),
            kInvalidArgument
        );

        let mut maps = [PhysicalUIMap {
            physicalUITypeID: PhysicalUITypeIDs_::kInvalidPUITypeID as PhysicalUITypeID,
            noteExpressionTypeID: NoteExpressionTypeIDs_::kInvalidTypeID,
        }; 3];
        let mut list = PhysicalUIMapList {
            count: maps.len() as uint32,
            map: maps.as_mut_ptr(),
        };
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(0, -1, &mut list),
            kResultOk
        );
        assert_eq!(list.count, 2);
        assert_eq!(
            maps[0].physicalUITypeID,
            PhysicalUITypeIDs_::kPUIPressure as PhysicalUITypeID
        );
        assert_eq!(
            maps[0].noteExpressionTypeID,
            NoteExpressionTypeIDs_::kBrightnessTypeID
        );
        assert_eq!(
            maps[1].physicalUITypeID,
            PhysicalUITypeIDs_::kPUIYMovement as PhysicalUITypeID
        );
        assert_eq!(
            maps[1].noteExpressionTypeID,
            NoteExpressionTypeIDs_::kTuningTypeID
        );

        let mut one_map = [PhysicalUIMap {
            physicalUITypeID: PhysicalUITypeIDs_::kInvalidPUITypeID as PhysicalUITypeID,
            noteExpressionTypeID: NoteExpressionTypeIDs_::kInvalidTypeID,
        }; 1];
        let mut one_list = PhysicalUIMapList {
            count: one_map.len() as uint32,
            map: one_map.as_mut_ptr(),
        };
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(0, 0, &mut one_list),
            kResultOk
        );
        assert_eq!(one_list.count, 1);
        assert_eq!(
            one_map[0].physicalUITypeID,
            PhysicalUITypeIDs_::kPUIPressure as PhysicalUITypeID
        );
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(1, 0, &mut list),
            kInvalidArgument
        );
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(0, 16, &mut list),
            kInvalidArgument
        );
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(0, 0, ptr::null_mut()),
            kInvalidArgument
        );
        let mut invalid_list = PhysicalUIMapList {
            count: 1,
            map: ptr::null_mut(),
        };
        assert_eq!(
            physical_mapping.getPhysicalUIMapping(0, 0, &mut invalid_list),
            kInvalidArgument
        );
    }
}

#[test]
fn factory_rejects_processor_and_controller_with_invalid_param_schema() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<InvalidParamSchemaPlugin>())
                .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<InvalidParamSchemaPlugin>();

        let processor_cid = tuid(metadata.processor_class_id);
        let mut component: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                processor_cid.as_ptr(),
                IComponent_iid.as_ptr(),
                &mut component,
            ),
            kResultFalse
        );
        assert!(component.is_null());

        let controller_cid = tuid(metadata.controller_class_id);
        let mut controller: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IEditController_iid.as_ptr(),
                &mut controller,
            ),
            kResultFalse
        );
        assert!(controller.is_null());
    }
}

#[test]
fn controller_maps_bypass_and_read_only_parameter_flags() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<FlagPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<FlagPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);
        let mut controller: *mut c_void = ptr::null_mut();
        assert_eq!(
            factory.createInstance(
                controller_cid.as_ptr(),
                IEditController_iid.as_ptr(),
                &mut controller,
            ),
            kResultOk
        );
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
            .expect("controller");
        assert_eq!(controller.getParameterCount(), 3);

        let mut bypass = MaybeUninit::<ParameterInfo>::zeroed();
        assert_eq!(
            controller.getParameterInfo(0, bypass.as_mut_ptr()),
            kResultOk
        );
        let bypass = bypass.assume_init();
        assert_eq!(
            bypass.flags & ParameterInfo_::ParameterFlags_::kIsBypass,
            ParameterInfo_::ParameterFlags_::kIsBypass
        );
        assert_eq!(
            bypass.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
            ParameterInfo_::ParameterFlags_::kCanAutomate
        );

        let mut meter = MaybeUninit::<ParameterInfo>::zeroed();
        assert_eq!(
            controller.getParameterInfo(1, meter.as_mut_ptr()),
            kResultOk
        );
        let meter = meter.assume_init();
        assert_eq!(meter.flags & ParameterInfo_::ParameterFlags_::kIsBypass, 0);
        assert_eq!(
            meter.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
            0
        );
        assert_eq!(
            controller.setParamNormalized(meter.id, 0.75),
            kInvalidArgument
        );
        assert_eq!(controller.getParamNormalized(meter.id), 0.0);

        let mut program = MaybeUninit::<ParameterInfo>::zeroed();
        assert_eq!(
            controller.getParameterInfo(2, program.as_mut_ptr()),
            kResultOk
        );
        let program = program.assume_init();
        assert_eq!(
            program.flags & ParameterInfo_::ParameterFlags_::kIsProgramChange,
            ParameterInfo_::ParameterFlags_::kIsProgramChange
        );
        assert_eq!(
            program.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
            ParameterInfo_::ParameterFlags_::kCanAutomate
        );
        assert_eq!(controller.setParamNormalized(program.id, 1.0), kResultOk);
        assert_eq!(controller.getParamNormalized(program.id), 1.0);
    }
}
