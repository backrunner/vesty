    #[test]
    fn daw_matrix_complete_requires_every_smoke_check_to_pass() {
        let complete = serde_json::json!({
            "host": "Test Host",
            "platform_supported": true,
            "scan": true,
            "load": true,
            "ui": true,
            "ui_host_param": true,
            "meter_stream": true,
            "automation": true,
            "buffer_sample_rate_change": true,
            "save_restore": true,
            "offline_render": true,
        });
        assert!(daw_matrix_complete(&[complete]));

        let unsupported_platform = serde_json::json!({
            "host": "Test Host",
            "platform_supported": false,
            "scan": true,
            "load": true,
            "ui": true,
            "ui_host_param": true,
            "meter_stream": true,
            "automation": true,
            "buffer_sample_rate_change": true,
            "save_restore": true,
            "offline_render": true,
        });
        assert!(!daw_matrix_complete(&[unsupported_platform]));

        let incomplete = serde_json::json!({
            "host": "Test Host",
            "platform_supported": true,
            "scan": true,
            "load": true,
            "ui": true,
            "ui_host_param": true,
            "meter_stream": false,
            "automation": true,
            "buffer_sample_rate_change": true,
            "save_restore": true,
            "offline_render": true,
        });
        assert!(!daw_matrix_complete(&[incomplete]));

        let missing_buffer_sample_rate = serde_json::json!({
            "host": "Test Host",
            "platform_supported": true,
            "scan": true,
            "load": true,
            "ui": true,
            "ui_host_param": true,
            "meter_stream": true,
            "automation": true,
            "buffer_sample_rate_change": false,
            "save_restore": true,
            "offline_render": true,
        });
        assert!(!daw_matrix_complete(&[missing_buffer_sample_rate]));
        assert!(!daw_matrix_complete(&[]));
    }

    #[test]
    fn daw_matrix_platform_text_marks_unsupported_or_unknown_platforms() {
        let supported = serde_json::json!({
            "platform": "Linux X11",
            "platform_supported": true,
        });
        assert_eq!(daw_platform_text(&supported), "Linux X11");

        let unsupported = serde_json::json!({
            "platform": "Linux Wayland",
            "platform_supported": false,
        });
        assert_eq!(
            daw_platform_text(&unsupported),
            "Linux Wayland (unsupported)"
        );

        let missing = serde_json::json!({
            "platform_supported": false,
        });
        assert_eq!(daw_platform_text(&missing), "missing (unsupported)");

        let unknown = serde_json::json!({
            "platform": "manual evidence",
        });
        assert_eq!(daw_platform_text(&unknown), "manual evidence (unknown)");
    }

    fn complete_release_row(host: &str) -> serde_json::Value {
        serde_json::json!({
            "host": host,
            "platform": "macOS test platform",
            "platform_supported": true,
            "scan": true,
        "load": true,
        "ui": true,
            "ui_host_param": true,
            "meter_stream": true,
            "automation": true,
            "buffer_sample_rate_change": true,
            "save_restore": true,
            "offline_render": true,
            "evidence": "test evidence",
        })
    }

    fn complete_release_rows() -> Vec<serde_json::Value> {
        vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect()
    }

    fn command_arg<'a>(command: &'a str, flag: &str) -> Option<&'a str> {
        let mut parts = command.split_whitespace();
        while let Some(part) = parts.next() {
            if part == flag {
                return parts.next();
            }
        }
        None
    }

    #[test]
    fn reaper_param_watch_accepts_host_side_param_motion() {
        let log = r#"
target_fx=VST3: Vesty Web UI Demo (Vesty)|ok=true
poll=1|param0=0.5
poll=2|param0=0.53899997472763
poll=3|param0=0.88899999856949
"#;
        assert!(reaper_param_watch_moved(log));
    }

    #[test]
    fn reaper_param_watch_rejects_static_param() {
        let log = r#"
target_fx=VST3: Vesty Web UI Demo (Vesty)|ok=true
poll=1|param0=0.5
poll=2|param0=0.5
"#;
        assert!(!reaper_param_watch_moved(log));
    }

    #[test]
    fn bridge_trace_accepts_packet_and_legacy_param_gesture_formats() {
        let packet_trace = r#"
ipc: {"type":"param.begin"}
ipc: {"type":"param.perform"}
ipc: {"type":"param.end"}
relay: ok result=0
"#;
        assert!(bridge_trace_relayed_param_gesture(packet_trace));

        let legacy_trace = r#"
relay: ParamGesture { phase: Begin } result=0
relay: ParamGesture { phase: Perform } result=0
relay: ParamGesture { phase: End } result=0
"#;
        assert!(bridge_trace_relayed_param_gesture(legacy_trace));
    }

    #[test]
    fn meter_stream_accepts_bridge_packets_flushes_and_meter_logs() {
        let packet_trace = r#"
packet: {"lane":"meter","type":"meter.main","payload":{"peaks":[0.75],"rms":[0.5]}}
"#;
        assert!(meter_stream_delivered(packet_trace));

        let flush_trace = "meter_flush sent=1";
        assert!(meter_stream_delivered(flush_trace));

        let meter_log = "topic=meter.main|peak=0.125|rms=0.0625";
        assert!(meter_stream_delivered(meter_log));
    }

    #[test]
    fn meter_stream_rejects_missing_or_zero_meter_evidence() {
        assert!(!meter_stream_delivered("meter_flush sent=0"));
        assert!(!meter_stream_delivered("topic=meter.main|peak=0.0"));
        assert!(!meter_stream_delivered(
            r#"{"lane":"meter","type":"meter.main"}"#
        ));
        assert!(!meter_stream_delivered("peak=0.5"));
    }

    #[test]
    fn generic_daw_evidence_defaults_to_missing_when_dir_absent() {
        let dir = Utf8PathBuf::from("target/does-not-exist-for-daw-matrix-test");
        let row = collect_generic_daw_evidence("Bitwig Studio", &dir);
        assert_eq!(row["host"], "Bitwig Studio");
        assert_eq!(row["evidence"], dir.to_string());
        assert_eq!(row["scan"], false);
        assert_eq!(row["meter_stream"], false);
    }

    #[test]
    fn daw_evidence_root_maps_standard_host_directories() {
        let root = Utf8PathBuf::from("target/daw-evidence-root");
        let paths = resolve_daw_evidence_paths(
            Some(root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );

        assert_eq!(paths.reaper, root.join("reaper"));
        assert_eq!(paths.cubase, root.join("cubase"));
        assert_eq!(paths.bitwig, root.join("bitwig"));
        assert_eq!(paths.ableton, root.join("ableton"));
        assert_eq!(paths.studio_one, root.join("studio-one"));
    }

    #[test]
    fn daw_evidence_root_templates_create_standard_host_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = resolve_daw_evidence_paths(
            Some(root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );

        assert_eq!(write_daw_evidence_templates(&paths).unwrap(), 55);
        assert_eq!(write_daw_evidence_templates(&paths).unwrap(), 0);

        for (host_dir, host_name) in [
            ("reaper", "REAPER"),
            ("cubase", "Cubase/Nuendo"),
            ("bitwig", "Bitwig Studio"),
            ("ableton", "Ableton Live"),
            ("studio-one", "Studio One"),
        ] {
            let dir = root.join(host_dir);
            assert!(dir.is_dir(), "{host_dir}");
            for file in [
                "README.md",
                "platform.txt",
                "scan-smoke.log",
                "load-smoke.log",
                "ui-smoke.log",
                "ui-host-smoke.log",
                "meter-stream.log",
                "automation-smoke.log",
                "buffer-sample-rate.log",
                "restore-smoke.log",
                "offline-render.log",
            ] {
                assert!(dir.join(file).is_file(), "{host_dir}/{file}");
            }
            let readme = fs::read_to_string(dir.join("README.md")).unwrap();
            assert!(readme.contains(host_name));
            assert!(readme.contains("Required smoke checks"));
        }

        let cubase = collect_generic_daw_evidence("Cubase/Nuendo", &root.join("cubase"));
        assert_eq!(cubase["evidence"], root.join("cubase").to_string());
        for key in [
            "scan",
            "load",
            "ui",
            "ui_host_param",
            "meter_stream",
            "automation",
            "buffer_sample_rate_change",
            "save_restore",
            "offline_render",
        ] {
            assert_eq!(cubase[key], false, "{key}");
        }
    }

    #[test]
    fn daw_evidence_templates_do_not_count_as_pass_or_overwrite_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let paths = DawEvidencePaths {
            reaper: root.join("reaper"),
            cubase: root.join("cubase"),
            bitwig: root.join("bitwig"),
            ableton: root.join("ableton"),
            studio_one: root.join("studio-one"),
        };

        fs::create_dir_all(&paths.bitwig).unwrap();
        fs::write(paths.bitwig.join("scan-smoke.log"), "scan=true\n").unwrap();
        fs::write(paths.bitwig.join("README.md"), "existing manual notes\n").unwrap();

        let created = write_daw_evidence_templates(&paths).unwrap();
        assert!(created > 0);
        assert_eq!(
            fs::read_to_string(paths.bitwig.join("scan-smoke.log")).unwrap(),
            "scan=true\n"
        );
        assert_eq!(
            fs::read_to_string(paths.bitwig.join("README.md")).unwrap(),
            "existing manual notes\n"
        );

        let cubase = collect_generic_daw_evidence("Cubase/Nuendo", &paths.cubase);
        for key in [
            "scan",
            "load",
            "ui",
            "ui_host_param",
            "meter_stream",
            "automation",
            "buffer_sample_rate_change",
            "save_restore",
            "offline_render",
        ] {
            assert_eq!(cubase[key], false, "{key}");
        }
        assert!(paths.cubase.join("README.md").is_file());
        assert!(paths.cubase.join("platform.txt").is_file());
        let readme = fs::read_to_string(paths.cubase.join("README.md")).unwrap();
        assert!(readme.contains("Treat Steinberg hosts as the strict reference path"));
        assert!(readme.contains("Templates, pending values"));
    }

    #[cfg(unix)]
    #[test]
    fn daw_evidence_templates_reject_symlink_slots() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = resolve_daw_evidence_paths(
            Some(root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform.txt")).unwrap();
        fs::create_dir_all(&paths.reaper).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, paths.reaper.join("platform.txt")).unwrap();

        let error = write_daw_evidence_templates(&paths)
            .expect_err("DAW evidence template should reject symlink slots")
            .to_string();

        assert!(error.contains("template output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn daw_evidence_templates_reject_symlink_host_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = resolve_daw_evidence_paths(
            Some(root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-reaper")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &paths.reaper).unwrap();

        let error = write_daw_evidence_templates(&paths)
            .expect_err("DAW evidence templates should reject symlink host dirs")
            .to_string();

        assert!(error.contains("template output directory must not be a symlink"));
        assert!(!external.join("README.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn daw_evidence_templates_reject_symlink_evidence_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-root")).unwrap();
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &root).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let error = write_daw_evidence_templates(&paths)
            .expect_err("DAW evidence templates should reject symlink evidence roots")
            .to_string();

        assert!(error.contains("template output directory parent must not be a symlink"));
        assert!(!external.join("README.md").exists());
        assert!(!external.join("reaper").exists());
    }

    #[cfg(unix)]
    #[test]
    fn evidence_template_dirs_reject_nested_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-evidence");
        let link = root.join("linked-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &link).unwrap();

        let release_evidence = link.join("missing/release-evidence");
        let error = write_release_evidence_templates(&release_evidence)
            .expect_err("release evidence templates should reject nested symlink parents")
            .to_string();

        assert!(error.contains("template output directory parent must not be a symlink"));
        assert!(!external.join("missing/release-evidence").exists());
    }

    #[test]
    fn release_evidence_templates_do_not_count_as_pass_or_overwrite_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("signing-macos.log"), "manual signing notes\n").unwrap();

        let created = write_release_evidence_templates(&root).unwrap();
        assert_eq!(created, 14);
        assert!(root.join("validator").is_dir());
        assert!(root.join("package").is_dir());
        assert_eq!(
            fs::read_to_string(root.join("signing-macos.log")).unwrap(),
            "manual signing notes\n"
        );
        assert_eq!(write_release_evidence_templates(&root).unwrap(), 0);

        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("Vesty Release Artifact Evidence"));
        assert!(readme.contains("Templates and pending values do not count"));
        assert!(readme.contains("vesty release-evidence collect-local"));
        assert!(readme.contains("--vst3-sdk-dir /path/to/VST_SDK"));
        assert!(readme.contains("--vst3-sdk-bindings-module"));
        assert!(readme.contains("vesty release-evidence import-ci"));
        assert!(readme.contains("import-ci-report.json"));
        assert!(readme.contains("preserves existing files by default"));
        assert!(readme.contains("vesty release-evidence collect-signing"));
        assert!(readme.contains("vesty release-evidence collect-notarization"));
        assert!(readme.contains("writes the captured log only after the evidence parser accepts"));
        assert!(readme.contains("Linux signing remains release-channel specific"));
        assert!(
            readme.contains(
                "does not inspect plugin binaries and does not create DAW, platform smoke"
            )
        );
        assert!(readme.contains("Optional CI static packaging smoke"));
        assert!(readme.contains("vesty export-types --out target/vesty-protocol --check"));
        assert!(readme.contains("--protocol-snapshot target/vesty-protocol"));
        assert!(readme.contains("--plan target/release-evidence/release-action-plan.json"));
        assert!(readme.contains("not pass evidence"));
        assert!(readme.contains("Do not pass `--skip-protocol` to the final"));
        assert!(readme.contains("--release-evidence-dir target/release-evidence"));
        assert!(readme.contains("publish-plan/publish-plan.json"));
        assert!(readme.contains("crate-package/crate-package.json"));
        assert!(readme.contains("--crate-package"));
        assert!(readme.contains("npm-pack/npm-pack.json"));
        assert!(readme.contains("dependency-baseline/dependency-baseline-latest.json"));
        assert!(readme.contains("`dependency-baseline.json` without registry latest checks"));
        assert!(readme.contains("validator/<bundle>.<platform>.validate.json"));
        assert!(readme.contains("vesty validate target/vesty/MyPlugin.vst3 --strict"));
        assert!(readme.contains("vesty validate --strict --report"));
        assert!(readme.contains("package/<bundle>.<platform>.static-validate.json"));
        assert!(readme.contains("vst3-sdk/vst3-sdk-headers.json"));
        assert!(readme.contains("vst3-sdk/generated-bindings-plan.json"));
        assert!(readme.contains("vst3-sdk/generated-bindings-surface.json"));
        assert!(readme.contains("vst3-sdk/generated.rs"));
        assert!(readme.contains("vst3-sdk/generated-abi-seed.rs"));
        assert!(readme.contains("vst3-sdk/generated-abi.rs"));
        assert!(readme.contains("vst3-sdk/generated-interface-skeleton.rs"));
        assert!(
            readme.contains("release-check validates these SDK audit files when they are present")
        );
        assert!(readme.contains(
            "rather than proof that full SDK 3.8 bindings or final release readiness exist"
        ));
        assert!(readme.contains(
            "readiness/surface/drift/interface/ABI/COM identity/dispatch/exposure-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan artifacts, not proof"
        ));
        assert!(readme.contains("layout size/alignment and field-offset fingerprints"));
        assert!(readme.contains(
            "interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata"
        ));
        assert!(readme.contains("IID/queryInterface entries"));
        assert!(readme.contains("COM object interface exposure records"));
        assert!(readme.contains("COM_OBJECT_INTERFACES"));
        assert!(readme.contains("COM_OBJECT_IDENTITY_PLANS"));
        assert!(readme.contains("COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES"));
        assert!(readme.contains("per-object queryInterface dispatch records"));
        assert!(readme.contains("pure lookup helpers"));
        assert!(readme.contains("interface_id_for_iid()"));
        assert!(readme.contains("query_interface_entry_by_interface()"));
        assert!(readme.contains("query_interface_entry_for_iid()"));
        assert!(readme.contains("com_object_query_interface_dispatch_by_interface()"));
        assert!(readme.contains("com_object_query_interface_dispatch_for_iid()"));
        assert!(readme.contains("factory export plan records"));
        assert!(readme.contains("processor/controller factory class plan records"));
        assert!(readme.contains("platform module export plan records"));
        assert!(readme.contains("binary export symbol plan records"));
        assert!(readme.contains("binary export inspection tool plan records"));
        assert!(readme.contains("FACTORY_EXPORT_PLAN"));
        assert!(readme.contains("FACTORY_CLASS_PLANS"));
        assert!(readme.contains("MODULE_EXPORT_PLANS"));
        assert!(readme.contains("BINARY_EXPORT_SYMBOL_PLANS"));
        assert!(readme.contains("BINARY_EXPORT_INSPECTION_TOOL_PLANS"));
        assert!(readme.contains("does not emit callable `queryInterface` glue"));
        assert!(readme.contains("generated factory exports"));
        assert!(readme.contains("generated module exports"));
        assert!(readme.contains("pure required-symbol checks"));
        assert!(readme.contains("inspection-tool helpers"));
        assert!(readme.contains("binary inspection tooling"));
        assert!(readme.contains("binary-export-symbol-plan"));
        assert!(readme.contains("binary-export-inspection-tool-plan"));
        assert!(readme.contains("bindingsGenerated"));
        assert!(readme.contains("vesty vst3-sdk emit-abi-seed"));
        assert!(readme.contains("vesty vst3-sdk emit-abi"));
        assert!(readme.contains("vesty vst3-sdk emit-interface-skeleton"));
        assert!(readme.contains("ABI_LAYOUT_GENERATED = true"));
        assert!(readme.contains("INTERFACE_SKELETON_GENERATED = true"));
        assert!(readme.contains("FULL_COM_BINDINGS_GENERATED = false"));
        assert!(readme.contains("ABI seed"));
        assert!(readme.contains("foundational ABI layout module"));
        assert!(readme.contains("interface/vtable skeleton"));
        assert!(
            readme.contains("no C++ AST parsing, ABI verification or Rust bindings generation")
        );
        assert!(readme.contains("reserved generated-headers backend"));
        let crate_package_readme =
            fs::read_to_string(root.join("crate-package/README.md")).unwrap();
        assert!(crate_package_readme.contains("vesty crate-package --out"));
        assert!(crate_package_readme.contains("vesty crate-package --check --out"));
        assert!(
            crate_package_readme.contains("cargo package -p <crate> --allow-dirty --no-verify")
        );
        assert!(
            crate_package_readme.contains("does not count as crate package readiness evidence")
        );
        let dependency_readme =
            fs::read_to_string(root.join("dependency-baseline/README.md")).unwrap();
        assert!(dependency_readme.contains("dependency-baseline-latest.json"));
        assert!(dependency_readme.contains("--latest"));
        assert!(dependency_readme.contains("does not count"));
        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &root).unwrap();
        assert!(options.dependency_baseline_report.is_none());
        assert!(readme.contains("ci-release-checks/"));
        assert!(readme.contains("release-action-plan-<OS>.json"));
        assert!(readme.contains("release-check pass evidence"));
        assert!(readme.contains("vesty-plugin-ui"));
        assert!(readme.contains("Steinberg validator-passed reports for `VestyGain.vst3`"));
        assert!(readme.contains("`linux-x64`, `macos` and `windows-x64`"));
        assert!(readme.contains("full 3x3 validator matrix"));
        assert!(readme.contains(
            "Each example/platform report must include `static_check.parameter_manifest`"
        ));
        assert!(readme.contains("static_check.binary_exports"));
        assert!(
            readme.contains("`VestyWebUIDemo.vst3` must also include UI asset manifest evidence")
        );
        assert!(readme.contains("The template README alone does not count"));
        assert!(readme.contains("Accepted release artifact markers"));
        assert!(readme.contains("exact `key=value` or `key: value` pairs"));
        assert!(readme.contains("Positive signing/notarization markers do not override"));
        assert!(readme.contains("VST3 binding baseline"));
        assert!(readme.contains("codesign=pass"));
        assert!(readme.contains("signtool=pass"));
        assert!(readme.contains("Generic `signed=true` / `signature=ok` markers are rejected"));
        assert!(readme.contains("Number of errors: 0"));
        assert!(readme.contains("requires both macOS codesign and Windows signtool coverage"));
        assert!(readme.contains("requires both accepted notarytool output and stapler success"));
        assert!(readme.contains("Generic `notarization=pass` / `notary=ok` markers are rejected"));
        assert!(readme.contains("status: Accepted"));
        assert!(readme.contains("The staple and validate action worked!"));
        let ci_doctor_readme = fs::read_to_string(root.join("ci-doctor/README.md")).unwrap();
        assert!(ci_doctor_readme.contains("doctor-Linux.json"));
        assert!(ci_doctor_readme.contains("vst3 binding baseline"));
        assert!(ci_doctor_readme.contains("does not count as CI doctor evidence"));
        let ci_release_check_readme =
            fs::read_to_string(root.join("ci-release-checks/README.md")).unwrap();
        assert!(ci_release_check_readme.contains("release-check-Linux.json"));
        assert!(ci_release_check_readme.contains("release-action-plan-Linux.json"));
        assert!(ci_release_check_readme.contains("Action plan sidecars"));
        assert!(ci_release_check_readme.contains("only reads `release-check*.json`"));
        assert!(ci_release_check_readme.contains("never treats them as pass evidence"));
        assert!(ci_release_check_readme.contains("local invariants"));
        assert!(ci_release_check_readme.contains("only for these per-OS CI snapshots"));
        assert!(ci_release_check_readme.contains("will fail if `--skip-protocol` is used"));
        assert!(ci_release_check_readme.contains("does not count as CI release-check evidence"));
        let platform_smoke_readme =
            fs::read_to_string(root.join("platform-smoke/README.md")).unwrap();
        assert!(platform_smoke_readme.contains("Platform Smoke Evidence"));
        assert!(platform_smoke_readme.contains("vesty platform-smoke --write-report"));
        assert!(platform_smoke_readme.contains("--system-webview"));
        assert!(platform_smoke_readme.contains("--jsbridge-roundtrip"));
        assert!(platform_smoke_readme.contains("--meter-stream"));
        assert!(platform_smoke_readme.contains("Linux Wayland is experimental"));
        assert!(platform_smoke_readme.contains("do not count as platform smoke evidence"));
        let publish_plan_readme = fs::read_to_string(root.join("publish-plan/README.md")).unwrap();
        assert!(publish_plan_readme.contains("vesty publish-plan --out"));
        assert!(publish_plan_readme.contains("vesty publish-plan --check --out"));
        assert!(publish_plan_readme.contains("does not count as publish-plan evidence"));
        let npm_pack_readme = fs::read_to_string(root.join("npm-pack/README.md")).unwrap();
        assert!(npm_pack_readme.contains("vesty npm-pack --out"));
        assert!(npm_pack_readme.contains("npm pack --workspaces --dry-run --json"));
        assert!(npm_pack_readme.contains("does not count as npm pack evidence"));
        let sdk_readme = fs::read_to_string(root.join("vst3-sdk/README.md")).unwrap();
        assert!(sdk_readme.contains("VST3 SDK Generated Bindings Evidence"));
        assert!(sdk_readme.contains("vesty vst3-sdk manifest"));
        assert!(sdk_readme.contains("vesty vst3-sdk binding-plan"));
        assert!(sdk_readme.contains("vesty vst3-sdk binding-surface"));
        assert!(sdk_readme.contains("vesty vst3-sdk emit-scaffold"));
        assert!(sdk_readme.contains("vesty vst3-sdk emit-abi-seed"));
        assert!(sdk_readme.contains("vesty vst3-sdk emit-abi"));
        assert!(sdk_readme.contains("vesty vst3-sdk emit-interface-skeleton"));
        assert!(sdk_readme.contains("generated-bindings-plan.json"));
        assert!(sdk_readme.contains("generated-bindings-surface.json"));
        assert!(sdk_readme.contains("generated-abi-seed.rs"));
        assert!(sdk_readme.contains("generated-abi.rs"));
        assert!(sdk_readme.contains("generated-interface-skeleton.rs"));
        assert!(sdk_readme.contains("metadata-only Rust module"));
        assert!(sdk_readme.contains("deterministic ABI seed module"));
        assert!(sdk_readme.contains("foundational ABI layout module"));
        assert!(sdk_readme.contains("ABI_LAYOUT_RECORDS"));
        assert!(sdk_readme.contains("ABI_FIELD_OFFSETS"));
        assert!(sdk_readme.contains("interface/vtable skeleton module"));
        assert!(sdk_readme.contains("com-object-interface-exposure-plan"));
        assert!(sdk_readme.contains("COM_OBJECT_INTERFACES"));
        assert!(sdk_readme.contains("com-object-identity-plan"));
        assert!(sdk_readme.contains("per-object-query-interface-dispatch-plan"));
        assert!(sdk_readme.contains("factory-export-plan"));
        assert!(sdk_readme.contains("factory-class-plan"));
        assert!(sdk_readme.contains("module-export-plan"));
        assert!(sdk_readme.contains("binary-export-symbol-plan"));
        assert!(sdk_readme.contains("COM_OBJECT_IDENTITY_PLANS"));
        assert!(sdk_readme.contains("COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES"));
        assert!(sdk_readme.contains("QUERY_INTERFACE_IID_LOOKUP_SCOPE"));
        assert!(sdk_readme.contains("interface_id_for_iid()"));
        assert!(sdk_readme.contains("query_interface_entry_for_iid()"));
        assert!(sdk_readme.contains("com_object_query_interface_dispatch_for_iid()"));
        assert!(sdk_readme.contains("FACTORY_EXPORT_PLAN"));
        assert!(sdk_readme.contains("FACTORY_CLASS_PLANS"));
        assert!(sdk_readme.contains("MODULE_EXPORT_PLANS"));
        assert!(sdk_readme.contains("BINARY_EXPORT_SYMBOL_PLANS"));
        assert!(sdk_readme.contains("BINARY_EXPORT_INSPECTION_TOOL_PLANS"));
        assert!(sdk_readme.contains("ABI_LAYOUT_GENERATED"));
        assert!(sdk_readme.contains("INTERFACE_SKELETON_GENERATED"));
        assert!(sdk_readme.contains("vst3-sdk/generated.rs"));
        assert!(sdk_readme.contains("vst3-sdk/generated-abi-seed.rs"));
        assert!(sdk_readme.contains("vst3-sdk/generated-abi.rs"));
        assert!(sdk_readme.contains("vst3-sdk/generated-interface-skeleton.rs"));
        assert!(sdk_readme.contains("validating its scaffold markers"));
        assert!(sdk_readme.contains("FULL_COM_BINDINGS_GENERATED = false"));
        assert!(sdk_readme.contains("bindingsGenerated: false"));
        assert!(
            sdk_readme.contains("does not parse C++ ASTs, verify ABI, or generate Rust bindings")
        );
        assert!(sdk_readme.contains(
            "does not count as VST3 SDK header manifest, generated bindings plan, generated bindings surface, generated scaffold, ABI seed, ABI layout, or interface skeleton evidence"
        ));

        let quiet_root =
            Utf8PathBuf::from_path_buf(temp.path().join("release-evidence-pending")).unwrap();
        write_release_evidence_templates(&quiet_root).unwrap();
        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &quiet_root).unwrap();
        assert!(options.ci_run_url.is_none());
        assert!(options.ci_doctor_dir.is_none());
        assert!(options.ci_release_check_dir.is_none());
        assert!(options.platform_smoke_dir.is_none());
        assert!(options.validate_reports.is_empty());
        assert!(options.static_validate_reports.is_empty());
        assert!(options.publish_plan_report.is_none());
        assert!(options.npm_pack_report.is_none());
        assert!(options.vst3_sdk_manifest.is_none());
        assert!(options.vst3_sdk_binding_plan.is_none());
        assert!(options.vst3_sdk_binding_surface.is_none());
        assert!(options.signed_bundle_evidence.is_empty());
        assert!(options.notarization_log.is_none());

        assert_eq!(
            write_platform_smoke_templates(&quiet_root.join("platform-smoke")).unwrap(),
            3
        );
        let mut options_with_pending_platform_smoke = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options_with_pending_platform_smoke, &quiet_root).unwrap();
        assert!(
            options_with_pending_platform_smoke
                .platform_smoke_dir
                .is_none()
        );

        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);
        assert!(release_check_complete(&report));

        let validate_check =
            validate_reports_release_check(&[root.join("validate-report.json")], true);
        assert_eq!(validate_check.status, "failed");
        assert!(
            validate_check
                .value
                .contains("static bundle check status is failed")
        );

        let static_validate_check = static_validate_reports_release_check(
            &[root.join("static-validate-report.json")],
            false,
        );
        assert_eq!(static_validate_check.status, "failed");
        assert!(
            static_validate_check
                .value
                .contains("static bundle check status is failed")
        );

        let signing_check =
            signed_bundle_evidence_release_check(&[root.join("signing-windows.log")], true);
        assert_eq!(signing_check.status, "failed");
        assert!(
            signing_check
                .value
                .contains("no positive signing marker found")
                || signing_check
                    .value
                    .contains("negative signing evidence found"),
            "{}",
            signing_check.value
        );

        let notary_check = notarization_log_release_check(Some(&root.join("notary.log")), true);
        assert_eq!(notary_check.status, "failed");
        assert!(
            notary_check
                .value
                .contains("no accepted notarization marker found")
                || notary_check
                    .value
                    .contains("negative notarization evidence found"),
            "{}",
            notary_check.value
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_templates_reject_symlink_slots() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external_notary =
            Utf8PathBuf::from_path_buf(temp.path().join("external-notary.log")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(&external_notary, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external_notary, root.join("notary.log")).unwrap();

        let error = write_release_evidence_templates(&root)
            .expect_err("release evidence templates should reject symlink slots")
            .to_string();

        assert!(error.contains("template output file must not be a symlink"));
        assert_eq!(
            fs::read_to_string(&external_notary).unwrap(),
            "do not overwrite\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_templates_reject_symlink_standard_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external_publish =
            Utf8PathBuf::from_path_buf(temp.path().join("external-publish-plan")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::create_dir(&external_publish).unwrap();
        unix_fs::symlink(&external_publish, root.join("publish-plan")).unwrap();

        let error = write_release_evidence_templates(&root)
            .expect_err("release evidence templates should reject symlink standard dirs")
            .to_string();

        assert!(error.contains("template output directory must not be a symlink"));
        assert!(!external_publish.join("README.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_templates_reject_validator_and_package_symlink_dirs() {
        for dir_name in ["validator", "package"] {
            let temp = tempfile::tempdir().unwrap();
            let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
            let external =
                Utf8PathBuf::from_path_buf(temp.path().join(format!("external-{dir_name}")))
                    .unwrap();
            fs::create_dir_all(&root).unwrap();
            fs::create_dir(&external).unwrap();
            unix_fs::symlink(&external, root.join(dir_name)).unwrap();

            let error = write_release_evidence_templates(&root)
                .expect_err("release evidence template should reject symlinked matrix dirs")
                .to_string();

            assert!(error.contains("template output directory must not be a symlink"));
            assert!(error.contains(dir_name));
            assert!(!external.join("README.md").exists());
        }
    }

    #[test]
    fn signing_verification_command_maps_platforms_and_evidence_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let macos_binary = bundle.join("Contents/MacOS/VestyGain");
        let windows_binary = bundle.join("Contents/x86_64-win/VestyGain.vst3");
        fs::create_dir_all(macos_binary.parent().unwrap()).unwrap();
        fs::create_dir_all(windows_binary.parent().unwrap()).unwrap();
        fs::write(&macos_binary, "mach-o").unwrap();
        fs::write(&windows_binary, "pe").unwrap();

        let ambiguous = infer_signing_bundle_platform(&bundle)
            .expect_err("multi-platform bundle should require explicit platform")
            .to_string();
        assert!(ambiguous.contains("--platform"));

        let mac = signing_verification_command(
            BundlePlatform::Macos,
            &bundle,
            None,
            Some(Utf8Path::new("/usr/bin/codesign")),
        )
        .unwrap();
        assert_eq!(mac.program, "/usr/bin/codesign");
        assert_eq!(mac.args[0], "--verify");
        assert!(mac.args.contains(&"--deep".to_string()));
        assert!(mac.args.contains(&"--strict".to_string()));
        assert_eq!(
            mac.args.last().map(String::as_str),
            Some(portable_report_path(&bundle).as_str())
        );
        assert_eq!(
            default_signing_evidence_path(&root, BundlePlatform::Macos),
            root.join("signing-macos.log")
        );

        let windows = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            None,
            Some(Utf8Path::new("signtool.exe")),
        )
        .unwrap();
        assert_eq!(windows.program, "signtool.exe");
        assert_eq!(windows.args[..3], ["verify", "/pa", "/v"]);
        assert_eq!(
            windows.args.last().map(String::as_str),
            Some(portable_report_path(&windows_binary).as_str())
        );
        assert_eq!(
            default_signing_evidence_path(&root, BundlePlatform::WindowsX64),
            root.join("signing-windows.log")
        );

        let linux = signing_platform_for_bundle_platform(BundlePlatform::LinuxX64)
            .unwrap_err()
            .to_string();
        assert!(linux.contains("release-channel specific"));
    }

    #[test]
    fn signing_verification_rejects_macos_binary_argument() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let macos_binary = bundle.join("Contents/MacOS/VestyGain");
        fs::create_dir_all(macos_binary.parent().unwrap()).unwrap();
        fs::write(&macos_binary, "mach-o").unwrap();

        let error = signing_verification_command(
            BundlePlatform::Macos,
            &bundle,
            Some(&macos_binary),
            Some(Utf8Path::new("/usr/bin/codesign")),
        )
        .expect_err("macOS signing verification should reject --binary")
        .to_string();
        assert!(error.contains("--binary is only supported for windows-x64"));
    }

    #[test]
    fn signing_verification_linux_remains_release_channel_specific_with_binary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let linux_binary = bundle.join("Contents/x86_64-linux/VestyGain.so");
        fs::create_dir_all(linux_binary.parent().unwrap()).unwrap();
        fs::write(&linux_binary, "elf").unwrap();

        let error = signing_verification_command(
            BundlePlatform::LinuxX64,
            &bundle,
            Some(&linux_binary),
            Some(Utf8Path::new("sign")),
        )
        .expect_err("Linux signing verification should stay release-channel specific")
        .to_string();
        assert!(error.contains("release-channel specific"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_symlinked_explicit_tool() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let real_tool = root.join("real-codesign");
        let tool_link = root.join("codesign-link");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::write(&real_tool, "#!/bin/sh\n").unwrap();
        unix_fs::symlink(&real_tool, &tool_link).unwrap();

        let error =
            signing_verification_command(BundlePlatform::Macos, &bundle, None, Some(&tool_link))
                .expect_err("signing verification should reject symlinked explicit tools")
                .to_string();
        assert!(error.contains("signing verification tool must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_symlinked_explicit_tool_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let external_tools = root.join("external-tools");
        let linked_tools = root.join("linked-tools");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::create_dir(&external_tools).unwrap();
        unix_fs::symlink(&external_tools, &linked_tools).unwrap();

        let error = signing_verification_command(
            BundlePlatform::Macos,
            &bundle,
            None,
            Some(&linked_tools.join("codesign")),
        )
        .expect_err("signing verification should reject symlinked explicit tool parents")
        .to_string();
        assert!(error.contains("signing verification tool parent must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_symlinked_bundle_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let real_bundle = root.join("RealGain.vst3");
        let link_bundle = root.join("LinkedGain.vst3");
        fs::create_dir_all(real_bundle.join("Contents/MacOS")).unwrap();
        unix_fs::symlink(&real_bundle, &link_bundle).unwrap();

        let infer_error = infer_signing_bundle_platform(&link_bundle)
            .expect_err("signing platform inference should reject symlinked bundles")
            .to_string();
        assert!(infer_error.contains("signing evidence source must not be a symlink"));

        let command_error = signing_verification_command(
            BundlePlatform::Macos,
            &link_bundle,
            None,
            Some(Utf8Path::new("/usr/bin/codesign")),
        )
        .expect_err("signing verification should reject symlinked bundles")
        .to_string();
        assert!(command_error.contains("signing verification target must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_symlinked_windows_payload_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let contents = bundle.join("Contents");
        let external_payload = root.join("external-win-payload");
        fs::create_dir_all(&contents).unwrap();
        fs::create_dir(&external_payload).unwrap();
        fs::write(external_payload.join("VestyGain.vst3"), "pe").unwrap();
        unix_fs::symlink(&external_payload, contents.join("x86_64-win")).unwrap();

        let infer_error = infer_signing_bundle_platform(&bundle)
            .expect_err("signing platform inference should reject symlinked payload dirs")
            .to_string();
        assert!(infer_error.contains("Windows signing payload directory must not be a symlink"));

        let command_error = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            None,
            Some(Utf8Path::new("signtool.exe")),
        )
        .expect_err("Windows signing verification should reject symlinked payload dirs")
        .to_string();
        assert!(command_error.contains("Windows signing payload directory must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_symlinked_explicit_windows_binary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let external_binary = root.join("external.vst3");
        let binary_link = root.join("linked.vst3");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(&external_binary, "pe").unwrap();
        unix_fs::symlink(&external_binary, &binary_link).unwrap();

        let error = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            Some(&binary_link),
            Some(Utf8Path::new("signtool.exe")),
        )
        .expect_err("Windows signing verification should reject symlinked explicit binaries")
        .to_string();
        assert!(error.contains("Windows signing verification binary must not be a symlink"));
    }

    #[test]
    fn signing_verification_rejects_explicit_windows_binary_outside_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let platform_dir = bundle.join("Contents/x86_64-win");
        let external_binary = root.join("external.vst3");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::write(&external_binary, "pe").unwrap();

        let error = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            Some(&external_binary),
            Some(Utf8Path::new("signtool.exe")),
        )
        .expect_err("Windows signing verification should reject binaries outside the bundle")
        .to_string();
        assert!(error.contains(
            "Windows signing verification binary must be inside bundle Contents/x86_64-win"
        ));
    }

    #[test]
    fn signing_verification_rejects_explicit_windows_binary_wrong_extension() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let platform_dir = bundle.join("Contents/x86_64-win");
        let binary = platform_dir.join("VestyGain.dll");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::write(&binary, "pe").unwrap();

        let error = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            Some(&binary),
            Some(Utf8Path::new("signtool.exe")),
        )
        .expect_err("Windows signing verification should reject non-.vst3 binaries")
        .to_string();
        assert!(error.contains("Windows signing verification binary must be a .vst3 file"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_verification_rejects_explicit_windows_binary_through_payload_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let platform_dir = bundle.join("Contents/x86_64-win");
        let external_dir = root.join("external-payload");
        let binary = platform_dir.join("linked/VestyGain.vst3");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::create_dir(&external_dir).unwrap();
        fs::write(external_dir.join("VestyGain.vst3"), "pe").unwrap();
        unix_fs::symlink(&external_dir, platform_dir.join("linked")).unwrap();

        let error = signing_verification_command(
            BundlePlatform::WindowsX64,
            &bundle,
            Some(&binary),
            Some(Utf8Path::new("signtool.exe")),
        )
        .expect_err("Windows signing verification should reject binaries that escape the bundle")
        .to_string();
        assert!(error.contains(
            "Windows signing verification binary must be inside bundle Contents/x86_64-win"
        ));
    }

    #[test]
    fn signing_evidence_parses_captured_command_output_text() {
        let macos = captured_command_log(
            &CommandSpec {
                program: "codesign".to_string(),
                args: vec!["--verify".to_string(), "VestyGain.vst3".to_string()],
            },
            &successful_test_output(
                "",
                "VestyGain.vst3: valid on disk\nVestyGain.vst3: satisfies its Designated Requirement\n",
            ),
        );
        let macos_platforms = signing_evidence_platforms_from_text(&macos).unwrap();
        assert!(macos_platforms.contains(&SigningEvidencePlatform::Macos));

        let windows = captured_command_log(
            &CommandSpec {
                program: "signtool.exe".to_string(),
                args: vec!["verify".to_string(), "VestyGain.vst3".to_string()],
            },
            &successful_test_output(
                "Successfully verified: VestyGain.vst3\nNumber of errors: 0\n",
                "",
            ),
        );
        let windows_platforms = signing_evidence_platforms_from_text(&windows).unwrap();
        assert!(windows_platforms.contains(&SigningEvidencePlatform::Windows));
    }

    #[test]
    fn collect_notarization_release_evidence_requires_acceptance_and_staple() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let notary_log = root.join("notarytool.json");
        let stapler_log = root.join("stapler.log");
        let evidence_dir = root.join("release-evidence");
        fs::write(&notary_log, r#"{ "status": "Accepted" }"#).unwrap();

        let missing_stapler = collect_notarization_release_evidence(CollectNotarizationOptions {
            notary_log: notary_log.clone(),
            stapler_log: None,
            dir: evidence_dir.clone(),
            out: None,
            format: "json".to_string(),
        })
        .unwrap_err()
        .to_string();
        assert!(missing_stapler.contains("stapler success"));
        assert!(!evidence_dir.join("notary.log").exists());

        fs::write(&stapler_log, "The staple and validate action worked!\n").unwrap();
        collect_notarization_release_evidence(CollectNotarizationOptions {
            notary_log,
            stapler_log: Some(stapler_log),
            dir: evidence_dir.clone(),
            out: None,
            format: "json".to_string(),
        })
        .unwrap();

        let collected = evidence_dir.join("notary.log");
        assert!(validate_notarization_evidence(&collected).is_ok());
        let check = notarization_log_release_check(Some(&collected), true);
        assert_eq!(check.status, "ok");
    }

    #[cfg(unix)]
    #[test]
    fn collect_notarization_release_evidence_rejects_symlink_input_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external_notary = root.join("external-notarytool.json");
        let notary_log = root.join("notarytool.json");
        let stapler_log = root.join("stapler.log");
        fs::write(&external_notary, r#"{ "status": "Accepted" }"#).unwrap();
        fs::write(&stapler_log, "The staple and validate action worked!\n").unwrap();
        unix_fs::symlink(&external_notary, &notary_log).unwrap();

        let error = collect_notarization_release_evidence(CollectNotarizationOptions {
            notary_log,
            stapler_log: Some(stapler_log),
            dir: root.join("release-evidence"),
            out: None,
            format: "json".to_string(),
        })
        .expect_err("notarization collection should reject symlinked input logs")
        .to_string();

        assert!(error.contains("notarytool log must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn collect_release_evidence_commands_reject_symlink_output_parents() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-release-evidence");
        let parent_link = root.join("release-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let collect_local_error = collect_local_release_evidence(
            &workspace_root(),
            &parent_link.join("local"),
            &root.join("vesty-protocol"),
            CollectLocalOptions {
                template: false,
                protocol: false,
                publish_plan: false,
                crate_package: false,
                npm_pack: false,
                dependency_baseline_latest: false,
                vst3_sdk_dir: None,
                vst3_sdk_bindings_module: Utf8PathBuf::from("target/vst3-sdk/generated.rs"),
            },
            "json",
        )
        .expect_err("collect-local should reject symlinked output parents")
        .to_string();
        assert!(collect_local_error.contains("release evidence dir parent must not be a symlink"));
        assert!(!external.join("local").exists());

        let signing_error = collect_signing_release_evidence(CollectSigningOptions {
            bundle: root.join("VestyGain.vst3"),
            platform: Some("macos".to_string()),
            binary: None,
            dir: parent_link.join("signing"),
            out: None,
            tool: Some(Utf8PathBuf::from("codesign")),
            format: "json".to_string(),
        })
        .expect_err("collect-signing should reject symlinked output parents")
        .to_string();
        assert!(signing_error.contains("release evidence dir parent must not be a symlink"));
        assert!(!external.join("signing").exists());

        let notary_log = root.join("notarytool.json");
        let stapler_log = root.join("stapler.log");
        fs::write(&notary_log, r#"{ "status": "Accepted" }"#).unwrap();
        fs::write(&stapler_log, "The staple and validate action worked!\n").unwrap();
        let notarization_error =
            collect_notarization_release_evidence(CollectNotarizationOptions {
                notary_log,
                stapler_log: Some(stapler_log),
                dir: parent_link.join("notarization"),
                out: None,
                format: "json".to_string(),
            })
            .expect_err("collect-notarization should reject symlinked output parents")
            .to_string();
        assert!(notarization_error.contains("release evidence dir parent must not be a symlink"));
        assert!(!external.join("notarization").exists());
    }

    #[test]
    fn collect_local_release_evidence_writes_only_local_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let protocol = Utf8PathBuf::from_path_buf(temp.path().join("vesty-protocol")).unwrap();

        collect_local_release_evidence(
            &workspace_root(),
            &root,
            &protocol,
            CollectLocalOptions {
                template: true,
                protocol: true,
                publish_plan: true,
                crate_package: false,
                npm_pack: false,
                dependency_baseline_latest: false,
                vst3_sdk_dir: None,
                vst3_sdk_bindings_module: Utf8PathBuf::from("target/vst3-sdk/generated.rs"),
            },
            "json",
        )
        .unwrap();

        assert!(root.join("README.md").is_file());
        assert!(protocol.join("typescript").is_dir());
        assert!(protocol.join("json-schema").is_dir());
        assert!(root.join("publish-plan/publish-plan.json").is_file());
        assert!(!root.join("npm-pack/npm-pack.json").exists());

        let report: LocalReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(root.join("local-collect-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report.evidence_dir, portable_report_path(&root));
        assert_eq!(report.items.len(), 3);
        assert!(
            report
                .external_evidence_note
                .contains("must come from real external runs")
        );
        assert!(report.items.iter().any(|item| {
            item.name == "protocol snapshot"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, protocol.as_str()))
        }));
        assert!(
            report
                .items
                .iter()
                .any(|item| item.name == "crate publish plan" && item.status == "ok")
        );

        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &root).unwrap();
        assert_eq!(
            options.publish_plan_report,
            Some(root.join("publish-plan/publish-plan.json"))
        );
        assert!(options.npm_pack_report.is_none());
        assert!(options.ci_doctor_dir.is_none());
        assert!(options.platform_smoke_dir.is_none());
        assert!(options.signed_bundle_evidence.is_empty());
        assert!(options.notarization_log.is_none());

        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let release_report = build_release_check_report(rows, &protocol, false, &options);
        let publish_check = release_report
            .checks
            .iter()
            .find(|check| check.name == "crate publish plan")
            .unwrap();
        assert_eq!(publish_check.status, "ok");
        let npm_check = release_report
            .checks
            .iter()
            .find(|check| check.name == "npm package pack report")
            .unwrap();
        assert_eq!(npm_check.status, "skipped");
    }

    #[test]
    fn collect_local_release_evidence_can_include_vst3_sdk_audit_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let protocol = Utf8PathBuf::from_path_buf(temp.path().join("unused-protocol")).unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let bindings_module = Utf8PathBuf::from("target/vst3-sdk/generated.rs");

        collect_local_release_evidence(
            &workspace_root(),
            &root,
            &protocol,
            CollectLocalOptions {
                template: true,
                protocol: false,
                publish_plan: false,
                crate_package: false,
                npm_pack: false,
                dependency_baseline_latest: false,
                vst3_sdk_dir: Some(sdk),
                vst3_sdk_bindings_module: bindings_module.clone(),
            },
            "json",
        )
        .unwrap();

        let manifest = root.join("vst3-sdk/vst3-sdk-headers.json");
        let binding_plan = root.join("vst3-sdk/generated-bindings-plan.json");
        let binding_surface = root.join("vst3-sdk/generated-bindings-surface.json");
        let scaffold = root.join("vst3-sdk/generated.rs");
        let abi_seed = root.join("vst3-sdk/generated-abi-seed.rs");
        let abi = root.join("vst3-sdk/generated-abi.rs");
        let interface_skeleton = root.join("vst3-sdk/generated-interface-skeleton.rs");
        assert!(manifest.is_file());
        assert!(binding_plan.is_file());
        assert!(binding_surface.is_file());
        assert!(scaffold.is_file());
        assert!(abi_seed.is_file());
        assert!(abi.is_file());
        assert!(interface_skeleton.is_file());
        assert_eq!(
            vst3_sdk_manifest_release_check(Some(&manifest)).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_binding_plan_release_check(Some(&binding_plan)).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_binding_surface_release_check(Some(&binding_surface)).status,
            "ok"
        );
        validate_vst3_sdk_generated_bindings_scaffold_text(&fs::read_to_string(&scaffold).unwrap())
            .unwrap();
        validate_vst3_sdk_generated_bindings_abi_seed_text(&fs::read_to_string(&abi_seed).unwrap())
            .unwrap();
        validate_vst3_sdk_generated_bindings_abi_text(&fs::read_to_string(&abi).unwrap()).unwrap();
        validate_vst3_sdk_generated_bindings_interface_skeleton_text(
            &fs::read_to_string(&interface_skeleton).unwrap(),
        )
        .unwrap();

        let report: LocalReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(root.join("local-collect-report.json")).unwrap(),
        )
        .unwrap();
        assert!(
            report
                .external_evidence_note
                .contains("explicitly requested VST3 SDK audit evidence")
        );
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK header manifest"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, manifest.as_str()))
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings plan"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, binding_plan.as_str()))
                && item.value.contains(bindings_module.as_str())
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings surface"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, binding_surface.as_str()))
                && item.value.contains("bindings generated false")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings scaffold"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, scaffold.as_str()))
                && item.value.contains("bindings generated false")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI seed"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, abi_seed.as_str()))
                && item.value.contains("full COM bindings generated false")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI layout"
                && item.status == "ok"
                && item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, abi.as_str()))
                && item.value.contains("layout fingerprints present")
                && item.value.contains("full COM bindings generated false")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings interface skeleton"
                && item.status == "ok"
                && item.path.as_deref().is_some_and(|path| {
                    release_report_paths_equal(path, interface_skeleton.as_str())
                })
                && item.value.contains("full COM bindings generated false")
        }));

        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &root).unwrap();
        assert_eq!(options.vst3_sdk_manifest, Some(manifest));
        assert_eq!(options.vst3_sdk_binding_plan, Some(binding_plan));
        assert_eq!(options.vst3_sdk_binding_surface, Some(binding_surface));
    }

    #[test]
    fn release_evidence_audit_reports_reject_malformed_shape_fields() {
        let item = LocalReleaseEvidenceItem {
            name: "protocol snapshot".to_string(),
            status: "ok".to_string(),
            path: Some("target/vesty-protocol".to_string()),
            value: "2 TypeScript file(s), 1 JSON schema file(s)".to_string(),
        };
        let local = LocalReleaseEvidenceReport {
            evidence_dir: "target/release-evidence".to_string(),
            workspace: ".".to_string(),
            protocol_snapshot: Some("target/vesty-protocol".to_string()),
            items: vec![item.clone()],
            external_evidence_note: "external runs still required".to_string(),
        };
        validate_local_release_evidence_report_shape(&local).unwrap();

        let mut local_unknown_top_level = serde_json::to_value(&local).unwrap();
        local_unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<LocalReleaseEvidenceReport>(local_unknown_top_level)
            .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut local_unknown_item = serde_json::to_value(&local).unwrap();
        local_unknown_item["items"][0]["owner"] = serde_json::json!("release");
        let error =
            serde_json::from_value::<LocalReleaseEvidenceReport>(local_unknown_item).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut bad_note = local.clone();
        bad_note.external_evidence_note = "external runs\u{202E}".to_string();
        let error = validate_local_release_evidence_report_shape(&bad_note)
            .expect_err("unsafe Unicode in local report note should be rejected")
            .to_string();
        assert!(error.contains("unsafe Unicode"));

        let mut too_many = local.clone();
        too_many.items = vec![item; RELEASE_EVIDENCE_REPORT_MAX_ITEMS + 1];
        let error = validate_local_release_evidence_report_shape(&too_many)
            .expect_err("oversized local report should be rejected")
            .to_string();
        assert!(error.contains("too many items"));

        let mut ok_without_path = local.clone();
        ok_without_path.items[0].path = None;
        let error = validate_local_release_evidence_report_shape(&ok_without_path)
            .expect_err("ok local evidence item should carry an evidence path")
            .to_string();
        assert!(error.contains("status ok must include an evidence path"));

        let mut local_template_path_drift = local.clone();
        local_template_path_drift
            .items
            .push(LocalReleaseEvidenceItem {
                name: "release evidence template".to_string(),
                status: "ok".to_string(),
                path: Some("target/release-evidence/templates".to_string()),
                value: "template files created".to_string(),
            });
        let error = validate_local_release_evidence_report_shape(&local_template_path_drift)
            .expect_err("local template item path should point at evidence root")
            .to_string();
        assert!(error.contains("must match release evidence dir"));

        let mut local_template_bad_status = local.clone();
        local_template_bad_status
            .items
            .push(LocalReleaseEvidenceItem {
                name: "release evidence template".to_string(),
                status: "skipped".to_string(),
                path: None,
                value: "template skipped".to_string(),
            });
        let error = validate_local_release_evidence_report_shape(&local_template_bad_status)
            .expect_err("local template item status should be ok")
            .to_string();
        assert!(error.contains("release evidence template"));
        assert!(error.contains("status must be ok"));

        let mut local_non_protocol_outside_evidence = local.clone();
        local_non_protocol_outside_evidence
            .items
            .push(LocalReleaseEvidenceItem {
                name: "crate publish plan".to_string(),
                status: "ok".to_string(),
                path: Some("target/other-evidence/publish-plan.json".to_string()),
                value: "1 publishable crate".to_string(),
            });
        let error =
            validate_local_release_evidence_report_shape(&local_non_protocol_outside_evidence)
                .expect_err("local non-protocol item outside evidence dir should be rejected")
                .to_string();
        assert!(error.contains("local release evidence item"));
        assert!(error.contains("must be under local release evidence dir"));

        let local_fixed_slot_cases = [
            (
                "crate publish plan",
                "target/release-evidence/publish-plan-copy.json",
                "publish-plan/publish-plan.json",
            ),
            (
                "npm package pack report",
                "target/release-evidence/npm-pack/npm-pack-copy.json",
                "npm-pack/npm-pack.json",
            ),
            (
                "dependency latest baseline",
                "target/release-evidence/dependency-baseline/latest-copy.json",
                "dependency-baseline/dependency-baseline-latest.json",
            ),
            (
                "vst3 SDK generated bindings ABI layout",
                "target/release-evidence/vst3-sdk/generated-abi-copy.rs",
                "vst3-sdk/generated-abi.rs",
            ),
        ];
        for (name, path, expected) in local_fixed_slot_cases {
            let mut report = local.clone();
            report.items.push(LocalReleaseEvidenceItem {
                name: name.to_string(),
                status: "ok".to_string(),
                path: Some(path.to_string()),
                value: "local evidence generated".to_string(),
            });
            let error = validate_local_release_evidence_report_shape(&report)
                .expect_err("local evidence item should use the standard slot")
                .to_string();
            assert!(error.contains(name), "{error}");
            assert!(error.contains(expected), "{error}");
        }

        let mut local_root_escape = local.clone();
        local_root_escape.evidence_dir = "target/release-evidence/..".to_string();
        let error = validate_local_release_evidence_report_shape(&local_root_escape)
            .expect_err("local evidence root with parent-directory component should be rejected")
            .to_string();
        assert!(error.contains("root path must not contain parent-directory components"));

        let mut protocol_item_drift = local.clone();
        protocol_item_drift.items[0].path = Some("target/other-protocol".to_string());
        let error = validate_local_release_evidence_report_shape(&protocol_item_drift)
            .expect_err("local protocol snapshot top-level and item path should match")
            .to_string();
        assert!(error.contains("does not match protocol snapshot item status/path"));

        let mut protocol_root_escape = local.clone();
        protocol_root_escape.protocol_snapshot = Some("target/vesty-protocol/..".to_string());
        protocol_root_escape.items[0].path = Some("target/vesty-protocol/..".to_string());
        let error = validate_local_release_evidence_report_shape(&protocol_root_escape)
            .expect_err("local protocol snapshot path escape should be rejected")
            .to_string();
        assert!(error.contains("local release protocol snapshot"));
        assert!(error.contains("parent-directory components"));

        let mut protocol_item_without_top_level = local.clone();
        protocol_item_without_top_level.protocol_snapshot = None;
        let error = validate_local_release_evidence_report_shape(&protocol_item_without_top_level)
            .expect_err("protocol item without top-level protocol snapshot should be rejected")
            .to_string();
        assert!(error.contains("top-level protocol_snapshot is missing"));

        let mut duplicate_protocol_item = local.clone();
        duplicate_protocol_item
            .items
            .push(LocalReleaseEvidenceItem {
                name: "protocol snapshot".to_string(),
                status: "ok".to_string(),
                path: Some("target/vesty-protocol".to_string()),
                value: "duplicate protocol snapshot".to_string(),
            });
        let error = validate_local_release_evidence_report_shape(&duplicate_protocol_item)
            .expect_err("duplicate protocol snapshot items should be rejected")
            .to_string();
        assert!(error.contains("must match exactly one protocol snapshot item"));

        let import_item = import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(Utf8Path::new("downloaded/doctor.json")),
            None,
            "first line\nsecond line\u{202E}\0third line",
        );
        assert_eq!(import_item.value, "first line second line third line");
        validate_import_ci_release_evidence_item_shape(&import_item).unwrap();

        let empty_import = ImportCiReleaseEvidenceReport {
            evidence_dir: "target/release-evidence".to_string(),
            source: "target/downloaded-artifacts".to_string(),
            items: Vec::new(),
            external_evidence_note: "no recognized artifacts were imported".to_string(),
        };
        validate_import_ci_release_evidence_report_shape(&empty_import).unwrap();

        let mut import_unknown_top_level = serde_json::to_value(&empty_import).unwrap();
        import_unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error =
            serde_json::from_value::<ImportCiReleaseEvidenceReport>(import_unknown_top_level)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut bad_import = empty_import.clone();
        bad_import.items = vec![ImportCiReleaseEvidenceItem {
            name: "artifact".to_string(),
            status: "copied".to_string(),
            source: None,
            path: None,
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&bad_import)
            .expect_err("unsupported import status should be rejected")
            .to_string();
        assert!(error.contains("unsupported status"));

        let mut imported_without_output = empty_import.clone();
        imported_without_output.items = vec![ImportCiReleaseEvidenceItem {
            name: "ci doctor artifact".to_string(),
            status: "imported".to_string(),
            source: Some("downloaded/doctor-Linux.json".to_string()),
            path: None,
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&imported_without_output)
            .expect_err("imported item should include destination")
            .to_string();
        assert!(error.contains("status imported must include an output path"));

        let mut ok_without_output = empty_import.clone();
        ok_without_output.items = vec![ImportCiReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "ok".to_string(),
            source: None,
            path: None,
            value: "template files created".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&ok_without_output)
            .expect_err("ok import item should include output path")
            .to_string();
        assert!(error.contains("status ok must include an output path"));

        let mut import_template_path_drift = empty_import.clone();
        import_template_path_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "ok".to_string(),
            source: None,
            path: Some("target/release-evidence/templates".to_string()),
            value: "template files created".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&import_template_path_drift)
            .expect_err("import-ci template item path should point at evidence root")
            .to_string();
        assert!(error.contains("must match release evidence dir"));

        let mut import_template_with_source = empty_import.clone();
        import_template_with_source.items = vec![ImportCiReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "ok".to_string(),
            source: Some("target/downloaded-artifacts/template.txt".to_string()),
            path: Some("target/release-evidence".to_string()),
            value: "template files created".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&import_template_with_source)
            .expect_err("import-ci template item should not claim artifact source")
            .to_string();
        assert!(error.contains("must not include a source"));

        let mut import_template_bad_status = empty_import.clone();
        import_template_bad_status.items = vec![ImportCiReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "imported".to_string(),
            source: None,
            path: Some("target/release-evidence".to_string()),
            value: "template files created".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&import_template_bad_status)
            .expect_err("import-ci template item status should be ok")
            .to_string();
        assert!(error.contains("release evidence template"));
        assert!(error.contains("status must be ok"));

        let mut failed_with_output = empty_import.clone();
        failed_with_output.items = vec![ImportCiReleaseEvidenceItem {
            name: "ci doctor artifact".to_string(),
            status: "failed".to_string(),
            source: Some("downloaded/doctor-Linux.json".to_string()),
            path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
            value: "missing checks".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&failed_with_output)
            .expect_err("failed import item should not claim output path")
            .to_string();
        assert!(error.contains("status failed must not include an output path"));

        let mut skipped_with_unrelated_output = empty_import.clone();
        skipped_with_unrelated_output.items = vec![ImportCiReleaseEvidenceItem {
            name: "json artifact".to_string(),
            status: "skipped".to_string(),
            source: Some("downloaded/unknown.json".to_string()),
            path: Some("target/release-evidence/unknown.json".to_string()),
            value: "unrecognized JSON artifact".to_string(),
        }];
        let error =
            validate_import_ci_release_evidence_report_shape(&skipped_with_unrelated_output)
                .expect_err(
                    "skipped import item should not claim output except destination-exists case",
                )
                .to_string();
        assert!(error.contains("status skipped may include an output path only"));

        let skipped_existing = ImportCiReleaseEvidenceReport {
            items: vec![ImportCiReleaseEvidenceItem {
                name: "ci doctor artifact".to_string(),
                status: "skipped".to_string(),
                source: Some("target/downloaded-artifacts/doctor-Linux.json".to_string()),
                path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
                value: ImportWriteOutcome::SkippedExisting.value().to_string(),
            }],
            ..empty_import.clone()
        };
        validate_import_ci_release_evidence_report_shape(&skipped_existing).unwrap();

        let mut import_unknown_item = serde_json::to_value(&skipped_existing).unwrap();
        import_unknown_item["items"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<ImportCiReleaseEvidenceReport>(import_unknown_item)
            .unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let duplicate_success_output_path = ImportCiReleaseEvidenceReport {
            items: vec![
                ImportCiReleaseEvidenceItem {
                    name: "ci doctor artifact".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/doctor-Linux.json".to_string()),
                    path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
                    value: "copied into release evidence".to_string(),
                },
                ImportCiReleaseEvidenceItem {
                    name: "ci doctor artifact".to_string(),
                    status: "imported".to_string(),
                    source: Some(
                        "target/downloaded-artifacts/duplicate-doctor-Linux.json".to_string(),
                    ),
                    path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
                    value: "copied into release evidence".to_string(),
                },
            ],
            ..empty_import.clone()
        };
        let error =
            validate_import_ci_release_evidence_report_shape(&duplicate_success_output_path)
                .expect_err("successful import-ci items should not share output paths")
                .to_string();
        assert!(error.contains("duplicates successful item"));
        assert!(error.contains("target/release-evidence/ci-doctor/doctor-Linux.json"));

        let skipped_existing_duplicate_output_path = ImportCiReleaseEvidenceReport {
            items: vec![
                ImportCiReleaseEvidenceItem {
                    name: "ci doctor artifact".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/doctor-Linux.json".to_string()),
                    path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
                    value: "copied into release evidence".to_string(),
                },
                ImportCiReleaseEvidenceItem {
                    name: "ci doctor artifact".to_string(),
                    status: "skipped".to_string(),
                    source: Some(
                        "target/downloaded-artifacts/duplicate-doctor-Linux.json".to_string(),
                    ),
                    path: Some("target/release-evidence/ci-doctor/doctor-Linux.json".to_string()),
                    value: ImportWriteOutcome::SkippedExisting.value().to_string(),
                },
            ],
            ..empty_import.clone()
        };
        validate_import_ci_release_evidence_report_shape(&skipped_existing_duplicate_output_path)
            .unwrap();

        let mut source_outside_root = skipped_existing.clone();
        source_outside_root.items[0].source =
            Some("target/other-artifacts/doctor-Linux.json".to_string());
        let error = validate_import_ci_release_evidence_report_shape(&source_outside_root)
            .expect_err("import item source outside declared source root should be rejected")
            .to_string();
        assert!(error.contains("source"));
        assert!(error.contains("must be under import-ci source"));

        let mut output_outside_root = skipped_existing.clone();
        output_outside_root.items[0].path =
            Some("target/other-evidence/ci-doctor/doctor-Linux.json".to_string());
        let error = validate_import_ci_release_evidence_report_shape(&output_outside_root)
            .expect_err("import item output outside declared evidence root should be rejected")
            .to_string();
        assert!(error.contains("path"));
        assert!(error.contains("must be under import-ci evidence dir"));

        let dynamic_output_cases = [
            (
                "ci doctor artifact",
                "target/downloaded-artifacts/doctor-Linux.json",
                "target/release-evidence/ci-doctor/linux-doctor.json",
                "`ci-doctor/doctor-<OS>.json`",
            ),
            (
                "ci release-check artifact",
                "target/downloaded-artifacts/release-check-Linux.json",
                "target/release-evidence/ci-release-checks/linux-release-check.json",
                "`ci-release-checks/release-check-<OS>.json`",
            ),
            (
                "release action plan sidecar",
                "target/downloaded-artifacts/release-action-plan-Linux.json",
                "target/release-evidence/ci-release-checks/action-plan-Linux.json",
                "`ci-release-checks/release-action-plan-<OS>.json`",
            ),
            (
                "platform smoke artifact",
                "target/downloaded-artifacts/platform-smoke/linux-x11.json",
                "target/release-evidence/platform-smoke/linux.json",
                "`platform-smoke/<platform>.json`",
            ),
            (
                "vst3 validate report",
                "target/downloaded-artifacts/validator/VestyGain.macos.validate.json",
                "target/release-evidence/validator/VestyGain.macos.validator.json",
                "`validator/<safe-bundle>.<platform>.validate.json`",
            ),
            (
                "vst3 static validate report",
                "target/downloaded-artifacts/package/VestyGain.linux-x64.static-validate.json",
                "target/release-evidence/package/VestyGain.linux-x64.validate.json",
                "`package/<safe-bundle>.<platform>.static-validate.json`",
            ),
        ];
        for (name, source, path, expected_shape) in dynamic_output_cases {
            let mut report = empty_import.clone();
            report.items = vec![ImportCiReleaseEvidenceItem {
                name: name.to_string(),
                status: "imported".to_string(),
                source: Some(source.to_string()),
                path: Some(path.to_string()),
                value: "copied into release evidence".to_string(),
            }];
            let error = validate_import_ci_release_evidence_report_shape(&report)
                .expect_err("dynamic import-ci item output should match production shape")
                .to_string();
            assert!(error.contains(name), "{error}");
            assert!(error.contains(expected_shape), "{error}");
        }

        let mut publish_plan_output_drift = empty_import.clone();
        publish_plan_output_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "crate publish plan".to_string(),
            status: "imported".to_string(),
            source: Some("target/downloaded-artifacts/publish-plan.json".to_string()),
            path: Some("target/release-evidence/publish-plan-copy.json".to_string()),
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&publish_plan_output_drift)
            .expect_err("publish plan import output should use the standard slot")
            .to_string();
        assert!(error.contains("crate publish plan"));
        assert!(error.contains("publish-plan/publish-plan.json"));

        let mut sdk_scaffold_output_drift = empty_import.clone();
        sdk_scaffold_output_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "vst3 SDK generated bindings scaffold".to_string(),
            status: "imported".to_string(),
            source: Some("target/downloaded-artifacts/vst3-sdk/generated.rs".to_string()),
            path: Some("target/release-evidence/vst3-sdk/generated-copy.rs".to_string()),
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&sdk_scaffold_output_drift)
            .expect_err("VST3 SDK scaffold import output should use the standard slot")
            .to_string();
        assert!(error.contains("vst3 SDK generated bindings scaffold"));
        assert!(error.contains("vst3-sdk/generated.rs"));

        let mut notarization_output_drift = empty_import.clone();
        notarization_output_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "notarization log".to_string(),
            status: "imported".to_string(),
            source: Some("target/downloaded-artifacts/notarytool.log".to_string()),
            path: Some("target/release-evidence/notary-copy.log".to_string()),
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&notarization_output_drift)
            .expect_err("notarization log import output should use the standard slot")
            .to_string();
        assert!(error.contains("notarization log"));
        assert!(error.contains("target/release-evidence/notary.log"));

        let valid_signing_dynamic_paths = ImportCiReleaseEvidenceReport {
            items: vec![
                ImportCiReleaseEvidenceItem {
                    name: "signed bundle evidence".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/macos-codesign.log".to_string()),
                    path: Some("target/release-evidence/signing-macos.log".to_string()),
                    value: "copied into release evidence".to_string(),
                },
                ImportCiReleaseEvidenceItem {
                    name: "signed bundle evidence".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/windows-signtool.log".to_string()),
                    path: Some("target/release-evidence/signing-windows.log".to_string()),
                    value: "copied into release evidence".to_string(),
                },
                ImportCiReleaseEvidenceItem {
                    name: "signed bundle evidence".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/signing-unknown.log".to_string()),
                    path: Some("target/release-evidence/signing/signing-unknown.log".to_string()),
                    value: "copied into release evidence".to_string(),
                },
                ImportCiReleaseEvidenceItem {
                    name: "signed bundle evidence".to_string(),
                    status: "imported".to_string(),
                    source: Some("target/downloaded-artifacts/macos/VestyGain.vst3".to_string()),
                    path: Some("target/release-evidence/signed-bundles/VestyGain.vst3".to_string()),
                    value: "copied signed macOS .vst3 bundle".to_string(),
                },
            ],
            ..empty_import.clone()
        };
        validate_import_ci_release_evidence_report_shape(&valid_signing_dynamic_paths).unwrap();

        let mut signing_root_log_drift = empty_import.clone();
        signing_root_log_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "signed bundle evidence".to_string(),
            status: "imported".to_string(),
            source: Some("target/downloaded-artifacts/macos-codesign.log".to_string()),
            path: Some("target/release-evidence/signing-copy.log".to_string()),
            value: "copied into release evidence".to_string(),
        }];
        let error = validate_import_ci_release_evidence_report_shape(&signing_root_log_drift)
            .expect_err("signing evidence root-level output should use a standard slot")
            .to_string();
        assert!(error.contains("signed bundle evidence"));
        assert!(error.contains("signing-macos.log"));
        assert!(error.contains("signed-bundles"));

        let mut signing_bundle_extension_drift = empty_import.clone();
        signing_bundle_extension_drift.items = vec![ImportCiReleaseEvidenceItem {
            name: "signed bundle evidence".to_string(),
            status: "imported".to_string(),
            source: Some("target/downloaded-artifacts/macos/VestyGain.vst3".to_string()),
            path: Some("target/release-evidence/signed-bundles/VestyGain.txt".to_string()),
            value: "copied signed macOS .vst3 bundle".to_string(),
        }];
        let error =
            validate_import_ci_release_evidence_report_shape(&signing_bundle_extension_drift)
                .expect_err("signed bundle evidence output should keep .vst3 bundle extension")
                .to_string();
        assert!(error.contains("signed-bundles"));
        assert!(error.contains("<safe-bundle>.vst3"));

        let root_escape = ImportCiReleaseEvidenceReport {
            source: "target/downloaded-artifacts/..".to_string(),
            ..skipped_existing.clone()
        };
        let error = validate_import_ci_release_evidence_report_shape(&root_escape)
            .expect_err("import source root with parent-directory component should be rejected")
            .to_string();
        assert!(error.contains("root path must not contain parent-directory components"));

        let mut child_escape = skipped_existing.clone();
        child_escape.items[0].source =
            Some("target/downloaded-artifacts/../other/doctor-Linux.json".to_string());
        let error = validate_import_ci_release_evidence_report_shape(&child_escape)
            .expect_err("import source path escaping source root should be rejected")
            .to_string();
        assert!(error.contains("must be under import-ci source"));

        let explicit_ci_run_url_file = ImportCiReleaseEvidenceReport {
            items: vec![ImportCiReleaseEvidenceItem {
                name: "ci run url".to_string(),
                status: "imported".to_string(),
                source: Some("target/manual-ci-run-url.txt".to_string()),
                path: Some("target/release-evidence/ci-run-url.txt".to_string()),
                value: "https://github.com/vesty-rs/vesty/actions/runs/1234567890".to_string(),
            }],
            ..empty_import.clone()
        };
        validate_import_ci_release_evidence_report_shape(&explicit_ci_run_url_file).unwrap();

        let mut explicit_ci_run_url_bad_value = explicit_ci_run_url_file.clone();
        explicit_ci_run_url_bad_value.items[0].value = "copied into release evidence".to_string();
        let error =
            validate_import_ci_release_evidence_report_shape(&explicit_ci_run_url_bad_value)
                .expect_err("imported ci run url value should be the actual GitHub Actions run URL")
                .to_string();
        assert!(error.contains("GitHub Actions run URL value"));

        let mut explicit_ci_run_url_path_drift = explicit_ci_run_url_file.clone();
        explicit_ci_run_url_path_drift.items[0].path =
            Some("target/release-evidence/ci-run-url-copy.txt".to_string());
        let error =
            validate_import_ci_release_evidence_report_shape(&explicit_ci_run_url_path_drift)
                .expect_err("imported ci run url output path should use the standard slot")
                .to_string();
        assert!(error.contains("does not match expected"));
        assert!(error.contains("target/release-evidence/ci-run-url.txt"));

        let mut explicit_ci_run_url_source_escape = explicit_ci_run_url_file.clone();
        explicit_ci_run_url_source_escape.items[0].source =
            Some("target/manual/../ci-run-url.txt".to_string());
        let error =
            validate_import_ci_release_evidence_report_shape(&explicit_ci_run_url_source_escape)
                .expect_err("external ci run url source path escape should be rejected")
                .to_string();
        assert!(error.contains("import-ci `ci run url` source"));
        assert!(error.contains("parent-directory components"));

        let collected = CollectedReleaseEvidenceReport {
            evidence_dir: "target/release-evidence".to_string(),
            kind: "signing".to_string(),
            output: "target/release-evidence/signing-macos.log".to_string(),
            items: vec![LocalReleaseEvidenceItem {
                name: "macOS signing verification".to_string(),
                status: "ok".to_string(),
                path: Some("target/release-evidence/signing-macos.log".to_string()),
                value: "codesign --verify --deep --strict".to_string(),
            }],
        };
        validate_collected_release_evidence_report_shape(&collected).unwrap();

        let mut collected_unknown_top_level = serde_json::to_value(&collected).unwrap();
        collected_unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error =
            serde_json::from_value::<CollectedReleaseEvidenceReport>(collected_unknown_top_level)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut collected_unknown_item = serde_json::to_value(&collected).unwrap();
        collected_unknown_item["items"][0]["owner"] = serde_json::json!("release");
        let error =
            serde_json::from_value::<CollectedReleaseEvidenceReport>(collected_unknown_item)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut bad_kind = collected.clone();
        bad_kind.kind = "release\u{202E}".to_string();
        let error = validate_collected_release_evidence_report_shape(&bad_kind)
            .expect_err("unsafe collected report kind should be rejected")
            .to_string();
        assert!(error.contains("unsafe Unicode"));

        let mut bad_status = collected.clone();
        bad_status.items[0].status = "imported".to_string();
        let error = validate_collected_release_evidence_report_shape(&bad_status)
            .expect_err("collected report item status should be strict")
            .to_string();
        assert!(error.contains("unsupported status"));

        let mut output_outside_evidence = collected.clone();
        output_outside_evidence.output = "target/other-evidence/signing-macos.log".to_string();
        let error = validate_collected_release_evidence_report_shape(&output_outside_evidence)
            .expect_err("collected report output outside evidence dir should be rejected")
            .to_string();
        assert!(error.contains("collected release evidence output"));
        assert!(error.contains("must be under collected release evidence dir"));

        let mut item_path_outside_evidence = collected.clone();
        item_path_outside_evidence.items[0].path =
            Some("target/other-evidence/signing-macos.log".to_string());
        let error = validate_collected_release_evidence_report_shape(&item_path_outside_evidence)
            .expect_err("collected report item path outside evidence dir should be rejected")
            .to_string();
        assert!(error.contains("collected release evidence item"));
        assert!(error.contains("must be under collected release evidence dir"));

        let mut item_path_drift = collected.clone();
        item_path_drift.items[0].path =
            Some("target/release-evidence/signing-windows.log".to_string());
        let error = validate_collected_release_evidence_report_shape(&item_path_drift)
            .expect_err("collected report item path should match top-level output")
            .to_string();
        assert!(error.contains("must match report output"));

        let mut signing_output_drift = collected.clone();
        signing_output_drift.output = "target/release-evidence/signing/codesign.log".to_string();
        signing_output_drift.items[0].path =
            Some("target/release-evidence/signing/codesign.log".to_string());
        let error = validate_collected_release_evidence_report_shape(&signing_output_drift)
            .expect_err("collected signing output should use the standard slot")
            .to_string();
        assert!(error.contains("collected release evidence output"));
        assert!(error.contains("signing-macos.log"));

        let notarization_output_drift = CollectedReleaseEvidenceReport {
            evidence_dir: "target/release-evidence".to_string(),
            kind: "notarization".to_string(),
            output: "target/release-evidence/notary-copy.log".to_string(),
            items: vec![LocalReleaseEvidenceItem {
                name: "macOS notarization".to_string(),
                status: "ok".to_string(),
                path: Some("target/release-evidence/notary-copy.log".to_string()),
                value: "accepted notarytool result and stapler success".to_string(),
            }],
        };
        let error = validate_collected_release_evidence_report_shape(&notarization_output_drift)
            .expect_err("collected notarization output should use the standard slot")
            .to_string();
        assert!(error.contains("collected release evidence output"));
        assert!(error.contains("notary.log"));

        let root_escape = CollectedReleaseEvidenceReport {
            evidence_dir: "target/release-evidence/..".to_string(),
            ..collected.clone()
        };
        let error = validate_collected_release_evidence_report_shape(&root_escape)
            .expect_err(
                "collected evidence root with parent-directory component should be rejected",
            )
            .to_string();
        assert!(error.contains("root path must not contain parent-directory components"));

        let mut empty_collected = collected;
        empty_collected.items.clear();
        let error = validate_collected_release_evidence_report_shape(&empty_collected)
            .expect_err("collected reports should contain at least one item")
            .to_string();
        assert!(error.contains("has no items"));
    }
