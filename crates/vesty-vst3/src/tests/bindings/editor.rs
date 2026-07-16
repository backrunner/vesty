use super::*;

#[test]
fn controller_notifies_host_when_latency_affecting_param_changes() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
        let mode_id = controller.param_id_for_test(1).expect("mode ParamID");
        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );

        assert_eq!(controller.setParamNormalized(gain_id, 0.25), kResultOk);
        assert!(handler.calls().is_empty());

        assert_eq!(controller.setParamNormalized(mode_id, 1.0), kResultOk);
        assert_eq!(
            handler.calls(),
            vec![HandlerCall::Restart(RestartFlags_::kLatencyChanged)]
        );

        assert_eq!(controller.perform_param_edit(mode_id, 0.5), kResultOk);
        assert_eq!(
            handler.calls(),
            vec![
                HandlerCall::Restart(RestartFlags_::kLatencyChanged),
                HandlerCall::Perform(mode_id, 0.5),
                HandlerCall::Restart(RestartFlags_::kLatencyChanged),
            ]
        );
    }
}

#[test]
fn controller_creates_editor_view() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
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

        let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
        let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

        let mut rect = MaybeUninit::<ViewRect>::zeroed();
        assert_eq!(view.getSize(rect.as_mut_ptr()), kResultOk);
        let rect = rect.assume_init();
        assert_eq!(rect.right, 777);
        assert_eq!(rect.bottom, 333);

        let mut too_small = ViewRect {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
        };
        assert_eq!(view.checkSizeConstraint(&mut too_small), kResultOk);
        assert_eq!(too_small.right, 320);
        assert_eq!(too_small.bottom, 200);

        assert_eq!(view.canResize(), kResultTrue);
        assert_eq!(view.getSize(ptr::null_mut()), kInvalidArgument);
        assert_eq!(view.checkSizeConstraint(ptr::null_mut()), kInvalidArgument);
        assert_eq!(view.onSize(ptr::null_mut()), kInvalidArgument);

        let mut resized = ViewRect {
            left: 4,
            top: 8,
            right: 120,
            bottom: 90,
        };
        assert_eq!(view.onSize(&mut resized), kResultOk);

        let mut current = MaybeUninit::<ViewRect>::zeroed();
        assert_eq!(view.getSize(current.as_mut_ptr()), kResultOk);
        let current = current.assume_init();
        assert_eq!(current.left, 4);
        assert_eq!(current.top, 8);
        assert_eq!(current.right, 324);
        assert_eq!(current.bottom, 208);

        let mut large = ViewRect {
            left: 1,
            top: 2,
            right: 901,
            bottom: 602,
        };
        assert_eq!(view.onSize(&mut large), kResultOk);
        let mut current = MaybeUninit::<ViewRect>::zeroed();
        assert_eq!(view.getSize(current.as_mut_ptr()), kResultOk);
        let current = current.assume_init();
        assert_eq!(current.left, 1);
        assert_eq!(current.top, 2);
        assert_eq!(current.right, 901);
        assert_eq!(current.bottom, 602);

        assert_eq!(view.removed(), kResultOk);
        assert_eq!(view.removed(), kResultOk);
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#[test]
fn editor_view_rejects_null_platform_and_parent_handles() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host handles.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
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
        let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
        let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

        assert_eq!(view.isPlatformTypeSupported(ptr::null()), kResultFalse);
        assert_eq!(
            view.attached(ptr::null_mut(), supported_platform_type_for_current_os()),
            kResultFalse
        );
    }
}

#[test]
fn editor_open_close_resize_fake_host_stress() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        const CYCLES: usize = 128;
        const WIDTHS: [i32; 6] = [1, 120, 319, 320, 777, 1200];
        const HEIGHTS: [i32; 6] = [1, 80, 199, 200, 333, 900];

        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
            .expect("factory");
        let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
        let controller_cid = tuid(metadata.controller_class_id);

        for cycle in 0..CYCLES {
            let mut controller: *mut c_void = ptr::null_mut();
            assert_eq!(
                factory.createInstance(
                    controller_cid.as_ptr(),
                    IEditController_iid.as_ptr(),
                    &mut controller,
                ),
                kResultOk,
                "cycle {cycle}: create controller"
            );
            let controller =
                ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                    .expect("controller");

            assert!(
                controller
                    .createView(c"not-editor".as_ptr() as *const c_char)
                    .is_null(),
                "cycle {cycle}: unknown view name is rejected"
            );

            let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
            let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

            let mut initial = MaybeUninit::<ViewRect>::zeroed();
            assert_eq!(
                view.getSize(initial.as_mut_ptr()),
                kResultOk,
                "cycle {cycle}: get initial size"
            );
            let initial = initial.assume_init();
            assert_eq!(initial.left, 0, "cycle {cycle}: initial left");
            assert_eq!(initial.top, 0, "cycle {cycle}: initial top");
            assert_eq!(initial.right, 777, "cycle {cycle}: initial width");
            assert_eq!(initial.bottom, 333, "cycle {cycle}: initial height");
            assert_eq!(view.canResize(), kResultTrue, "cycle {cycle}: resizable");

            let left = (cycle % 11) as i32;
            let top = (cycle % 7) as i32;
            let requested_width = WIDTHS[cycle % WIDTHS.len()];
            let requested_height = HEIGHTS[(cycle * 5) % HEIGHTS.len()];
            let expected_width = requested_width.max(320);
            let expected_height = requested_height.max(200);

            let mut constrained = ViewRect {
                left,
                top,
                right: left + requested_width,
                bottom: top + requested_height,
            };
            assert_eq!(
                view.checkSizeConstraint(&mut constrained),
                kResultOk,
                "cycle {cycle}: check size constraint"
            );
            assert_eq!(
                constrained.right - constrained.left,
                expected_width,
                "cycle {cycle}: constrained width"
            );
            assert_eq!(
                constrained.bottom - constrained.top,
                expected_height,
                "cycle {cycle}: constrained height"
            );

            let mut resized = ViewRect {
                left,
                top,
                right: left + requested_width,
                bottom: top + requested_height,
            };
            assert_eq!(
                view.onSize(&mut resized),
                kResultOk,
                "cycle {cycle}: resize"
            );

            let mut current = MaybeUninit::<ViewRect>::zeroed();
            assert_eq!(
                view.getSize(current.as_mut_ptr()),
                kResultOk,
                "cycle {cycle}: get current size"
            );
            let current = current.assume_init();
            assert_eq!(current.left, left, "cycle {cycle}: current left");
            assert_eq!(current.top, top, "cycle {cycle}: current top");
            assert_eq!(
                current.right - current.left,
                expected_width,
                "cycle {cycle}: current width"
            );
            assert_eq!(
                current.bottom - current.top,
                expected_height,
                "cycle {cycle}: current height"
            );

            assert_eq!(view.removed(), kResultOk, "cycle {cycle}: first remove");
            assert_eq!(view.removed(), kResultOk, "cycle {cycle}: second remove");
        }
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn editor_attach_failure_is_traced() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let temp = tempfile::tempdir().unwrap();
        let trace = temp.path().join("bridge-trace.log");
        std::env::set_var("VESTY_BRIDGE_TRACE", &trace);

        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
        let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

        assert_eq!(
            view.attached(
                ptr::null_mut(),
                c"unsupported-platform".as_ptr() as FIDString
            ),
            kResultFalse
        );

        std::env::remove_var("VESTY_BRIDGE_TRACE");
        let text = std::fs::read_to_string(trace).unwrap();
        assert!(text.contains("editor_attach_unsupported_platform"));
    }
}
