    #[test]
    fn strict_validate_matches_binary_exports_after_path_normalization() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("static-validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");
        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binaries = vec!["./Gain.vst3/Contents/MacOS/Gain".to_string()];
        report.static_check.binary_exports[0].binary =
            "Gain.vst3\\Contents\\MacOS\\Gain".to_string();

        validate_static_validate_report(&report).unwrap();
        assert!(strict_static_bundle_check_error(&report.static_check).is_none());
    }

    #[test]
    fn validate_report_rejects_mismatched_binary_export_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].binary = "Other.vst3/Contents/MacOS/Gain".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("binary export check path does not belong to Gain.vst3")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].binary =
            "Gain.vst3/Contents/MacOS/Unlisted".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("binary not listed in static_check.binaries")
        );
    }

    #[test]
    fn validate_report_rejects_manifest_paths_from_other_bundle_or_bad_asset_count() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.parameter_manifest = Some(format!(
            "Other.vst3/Contents/Resources/{PARAMETER_MANIFEST_FILE}"
        ));
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("static_check.parameter_manifest does not belong to Gain.vst3")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.asset_manifest = Some(format!(
            "Gain.vst3/Contents/Resources/{ASSET_MANIFEST_FILE}"
        ));
        report.static_check.asset_count = 0;
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("asset_manifest is present but asset_count is 0")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.asset_count = 2;
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("asset_count is 2 but asset_manifest is missing")
        );
    }

    #[test]
    fn validate_report_rejects_failed_binary_export_checks() {
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/x86_64-linux/Plugin.so".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/x86_64-linux/Plugin.so".to_string(),
                    platform: "linux-x64".to_string(),
                    status: "failed".to_string(),
                    tool: Some("nm -D --defined-only".to_string()),
                    required_symbols: vec![
                        "GetPluginFactory".to_string(),
                        "ModuleEntry".to_string(),
                        "ModuleExit".to_string(),
                    ],
                    found_symbols: vec!["GetPluginFactory".to_string()],
                    missing_symbols: vec!["ModuleEntry".to_string(), "ModuleExit".to_string()],
                    error: None,
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck {
                status: "passed".to_string(),
                path: Some("/tools/validator".to_string()),
                exit_code: Some(0),
                tests_passed: Some(47),
                tests_failed: Some(0),
                stdout: None,
                stderr: None,
                reason: None,
                error: None,
            },
        };

        let static_error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            static_error
                .to_string()
                .contains("binary export check failed")
        );

        let release_error = validate_release_validate_report(&report).unwrap_err();
        assert!(release_error.to_string().contains("ModuleEntry"));
    }

    #[test]
    fn validate_report_binary_export_expectations_use_vst3_sys_plan() {
        for platform in ["macos", "windows-x64", "linux-x64"] {
            assert_eq!(
                expected_binary_export_symbols(platform),
                vesty_vst3_sys::required_binary_export_tool_symbols(platform)
            );
        }
        assert_eq!(expected_binary_export_symbols("linux-x11"), None);
    }

    #[test]
    fn validate_report_rejects_incomplete_ok_binary_export_checks() {
        let mut report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/MacOS/Plugin".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/MacOS/Plugin".to_string(),
                    platform: "macos".to_string(),
                    status: "ok".to_string(),
                    tool: Some("nm -gU".to_string()),
                    required_symbols: vec!["_GetPluginFactory".to_string()],
                    found_symbols: vec!["_GetPluginFactory".to_string()],
                    missing_symbols: Vec::new(),
                    error: None,
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("incomplete required symbol list")
        );
        assert!(error.to_string().contains("_bundleEntry"));

        report.static_check.binary_exports[0].required_symbols =
            expected_binary_export_symbols("macos")
                .unwrap()
                .iter()
                .map(|symbol| (*symbol).to_string())
                .collect();
        report.static_check.binary_exports[0].found_symbols = vec!["_GetPluginFactory".to_string()];

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(error.to_string().contains("did not record found symbols"));
        assert!(error.to_string().contains("_bundleEntry"));
    }

    #[test]
    fn validate_report_accepts_skipped_binary_export_checks_with_reason() {
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/x86_64-win/Plugin.vst3".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/x86_64-win/Plugin.vst3".to_string(),
                    platform: "windows-x64".to_string(),
                    status: "skipped".to_string(),
                    tool: None,
                    required_symbols: expected_binary_export_symbols("windows-x64")
                        .unwrap()
                        .iter()
                        .map(|symbol| (*symbol).to_string())
                        .collect(),
                    found_symbols: Vec::new(),
                    missing_symbols: Vec::new(),
                    error: Some("dumpbin unavailable on this host".to_string()),
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        validate_static_validate_report(&report).unwrap();
        let check = static_validate_reports_release_check(&[], false);
        assert_eq!(check.status, "skipped");
    }

    #[test]
    fn strict_validate_requires_ok_binary_export_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("static-validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");
        let mut report = read_validate_report(&report_path).unwrap();

        assert!(strict_static_bundle_check_error(&report.static_check).is_none());

        report.static_check.binary_exports.clear();
        let error = strict_static_bundle_check_error(&report.static_check).unwrap();
        assert!(error.contains("strict validation requires binary export evidence"));
        assert!(error.contains("Gain.vst3/Contents/MacOS/Gain"));

        report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].status = "skipped".to_string();
        report.static_check.binary_exports[0].tool = None;
        report.static_check.binary_exports[0].found_symbols.clear();
        report.static_check.binary_exports[0].error =
            Some("nm unavailable on this runner".to_string());
        validate_static_validate_report(&report).unwrap();

        let error = strict_static_bundle_check_error(&report.static_check).unwrap();
        assert!(error.contains("strict validation requires ok binary export evidence"));
        assert!(error.contains("nm unavailable on this runner"));
    }

    #[test]
    fn validate_command_accepts_strict_flag() {
        let cli = Cli::try_parse_from([
            "vesty",
            "validate",
            "Plugin.vst3",
            "--static-only",
            "--strict",
            "--format",
            "json",
        ])
        .unwrap();

        match cli.command {
            Commands::Validate {
                static_only,
                strict,
                format,
                ..
            } => {
                assert!(static_only);
                assert!(strict);
                assert_eq!(format, "json");
            }
            _ => panic!("expected validate command"),
        }
    }

    #[test]
    fn daw_commands_share_release_action_default_evidence_layout() {
        for command in ["daw-matrix", "release-check"] {
            let cli = Cli::try_parse_from(["vesty", command]).unwrap();
            let (
                reaper_evidence,
                cubase_evidence,
                bitwig_evidence,
                ableton_evidence,
                studio_one_evidence,
            ) = match cli.command {
                Commands::DawMatrix {
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                    ..
                }
                | Commands::ReleaseCheck {
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                    ..
                } => (
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                ),
                _ => panic!("expected DAW evidence command"),
            };

            assert_eq!(
                reaper_evidence,
                Utf8PathBuf::from("target/daw-evidence/reaper")
            );
            assert_eq!(
                cubase_evidence,
                Utf8PathBuf::from("target/daw-evidence/cubase")
            );
            assert_eq!(
                bitwig_evidence,
                Utf8PathBuf::from("target/daw-evidence/bitwig")
            );
            assert_eq!(
                ableton_evidence,
                Utf8PathBuf::from("target/daw-evidence/ableton")
            );
            assert_eq!(
                studio_one_evidence,
                Utf8PathBuf::from("target/daw-evidence/studio-one")
            );
        }
    }

    #[test]
    fn param_manifest_command_writes_and_checks_stable_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let specs = root.join("params.json");
        let out = root.join("vesty-parameters.json");
        fs::write(
            &specs,
            r#"{
  "version": 1,
  "parameters": [
    {
      "id": "gain",
      "name": "Gain",
      "kind": { "float": { "min": -60.0, "max": 12.0 } },
      "defaultNormalized": 0.8333333333333334,
      "unit": "dB",
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    },
    {
      "id": "bypass",
      "name": "Bypass",
      "kind": "bool",
      "defaultNormalized": 0.0,
      "unit": null,
      "stepCount": 1,
      "flags": {
        "automatable": true,
        "bypass": true,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
"#,
        )
        .unwrap();

        run_param_manifest(specs.clone(), Some(out.clone()), false, "json").unwrap();
        let manifest = read_parameter_manifest(&out).unwrap();
        assert_eq!(manifest.parameters.len(), 2);
        assert_eq!(
            manifest.id_algorithm,
            "vesty.vst3.param.fnv1a31-positive.v2"
        );
        assert_eq!(manifest.parameters[0].id, "gain");
        assert_eq!(manifest.parameters[0].vst3_param_id, 1_983_572_582);
        assert!(!manifest.parameters[0].spec.flags.program_change);
        assert!(manifest.parameters[1].spec.flags.bypass);
        assert!(!manifest.parameters[1].spec.flags.program_change);
        let manifest_text = fs::read_to_string(&out).unwrap();
        assert!(manifest_text.contains(r#""programChange": false"#));

        run_param_manifest(specs.clone(), Some(out.clone()), true, "text").unwrap();

        let mut tampered = serde_json::to_value(&manifest).unwrap();
        tampered["parameters"][0]["vst3ParamId"] =
            serde_json::json!(manifest.parameters[0].vst3_param_id.wrapping_add(1));
        fs::write(&out, serde_json::to_string_pretty(&tampered).unwrap()).unwrap();

        let error = run_param_manifest(specs, Some(out), true, "text").unwrap_err();
        assert!(
            error.to_string().contains("vst3ParamId") || error.to_string().contains("out of date")
        );
    }

    #[test]
    fn param_manifest_report_rejects_unknown_json_fields() {
        let text = r#"{
          "status": "ok",
          "specs": "params.specs.json",
          "manifest": "vesty-parameters.json",
          "parameters": 1,
          "id_algorithm": "vesty.vst3.param.fnv1a31-positive.v2",
          "check": true,
          "generatedBy": "manual"
        }"#;

        let error = serde_json::from_str::<ParamManifestReport>(text).unwrap_err();

        assert!(error.to_string().contains("unknown field `generatedBy`"));
    }

    #[test]
    fn param_manifest_check_requires_output_path() {
        let temp = tempfile::tempdir().unwrap();
        let specs = Utf8PathBuf::from_path_buf(temp.path().join("params.json")).unwrap();
        fs::write(&specs, "[]").unwrap();

        let error = run_param_manifest(specs, None, true, "text").unwrap_err();

        assert!(error.to_string().contains("--check requires --out"));
    }

    #[cfg(unix)]
    #[test]
    fn param_manifest_rejects_symlinked_specs_input() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external_specs = root.join("external-params.json");
        let specs = root.join("params.json");
        fs::write(&external_specs, "[]").unwrap();
        std::os::unix::fs::symlink(&external_specs, &specs).unwrap();

        let error = run_param_manifest(
            specs,
            Some(root.join("vesty-parameters.json")),
            false,
            "text",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("parameter specs must not be a symlink")
        );
    }

    #[test]
    fn smoke_host_report_validates_local_examples_and_optional_bridge_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        fs::write(
            &meter_log,
            r#"{"lane":"meter","type":"meter.main","payload":{"peaks":[0.25]}}"#,
        )
        .unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "ok");
        assert!(smoke_host_report_all_ok(&report));
        assert!(
            report
                .external_evidence_note
                .contains("does not replace real DAW")
        );
        for name in [
            "gain config",
            "gain parameter sidecar",
            "midi-synth config",
            "midi-synth parameter sidecar",
            "web-ui-param-demo config",
            "web-ui-param-demo parameter sidecar",
            "web-ui-param-demo UI assets",
            "JSBridge trace",
            "meter stream",
        ] {
            assert!(
                report
                    .checks
                    .iter()
                    .any(|check| check.name == name && check.status == "ok"),
                "missing ok smoke-host check {name}: {report:#?}"
            );
        }
    }

    #[test]
    fn smoke_host_report_rejects_malformed_shape_fields() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let report = build_smoke_host_report(&workspace, None, None);

        let mut unknown_top_level = serde_json::to_value(&report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<SmokeHostReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<SmokeHostReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = report.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        let error = validate_smoke_host_report(&duplicate).unwrap_err();
        assert!(error.to_string().contains("duplicate smoke-host check"));
        let error = print_smoke_host_report(&duplicate, OutputFormat::Json, None).unwrap_err();
        assert!(error.to_string().contains("duplicate smoke-host check"));

        let mut unknown = report.clone();
        unknown.checks.push(SmokeHostCheck {
            name: "extra check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        let error = validate_smoke_host_report(&unknown).unwrap_err();
        assert!(error.to_string().contains("unknown smoke-host check(s)"));
        assert!(error.to_string().contains("extra_check"));

        let mut missing = report.clone();
        missing
            .checks
            .retain(|check| check.name != "JSBridge trace");
        let error = validate_smoke_host_report(&missing).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report missing required check(s)")
        );
        assert!(error.to_string().contains("jsbridge_trace"));

        let mut control_workspace = report.clone();
        control_workspace.workspace = "workspace\nforged".to_string();
        let error = validate_smoke_host_report(&control_workspace).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host workspace must not contain control characters")
        );

        let mut unsafe_hint = report.clone();
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        let error = validate_smoke_host_report(&unsafe_hint).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut too_many = report.clone();
        while too_many.checks.len() <= SMOKE_HOST_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(SmokeHostCheck {
                name: format!("extra check {index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        let error = validate_smoke_host_report(&too_many).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report has too many checks")
        );

        let mut generated = report.clone();
        generated.status = "failed".to_string();
        let check = generated
            .checks
            .iter_mut()
            .find(|check| check.name == "JSBridge trace")
            .unwrap();
        *check = smoke_host_failed(
            "JSBridge trace",
            "line one\nline two\u{202e}",
            "hint\twith newline\nnext",
        );
        validate_smoke_host_report(&generated).unwrap();
        let check = generated
            .checks
            .iter()
            .find(|check| check.name == "JSBridge trace")
            .unwrap();
        assert!(!check.value.contains('\n'));
        assert!(!check.value.contains('\u{202e}'));

        let diagnostic = smoke_host_failed(
            "multiline diagnostic",
            "line one\nline two\u{202e}",
            "hint\twith newline\nnext",
        );
        assert!(!diagnostic.name.contains('\n'));
        assert!(!diagnostic.value.contains('\n'));
        assert!(!diagnostic.value.contains('\u{202e}'));
    }

    #[test]
    fn smoke_host_check_mode_rejects_drift_and_strict_rejects_skipped_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        fs::write(&meter_log, "meter_flush sent=1\n").unwrap();
        let out = workspace.join("smoke-host.json");

        run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: Some(bridge_trace.clone()),
            meter_log: Some(meter_log.clone()),
            out: Some(out.clone()),
            check: false,
            strict: true,
            format: "json".to_string(),
        })
        .unwrap();
        run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: Some(bridge_trace),
            meter_log: Some(meter_log),
            out: Some(out.clone()),
            check: true,
            strict: true,
            format: "text".to_string(),
        })
        .unwrap();

        fs::write(
            workspace.join("examples/gain/vesty.toml"),
            smoke_host_vesty_toml("Wrong Gain", "Fx", false),
        )
        .unwrap();
        let error = run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: None,
            meter_log: None,
            out: Some(out),
            check: true,
            strict: false,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report is out of date")
        );

        let strict_error = run_smoke_host(SmokeHostOptions {
            workspace,
            bridge_trace: None,
            meter_log: None,
            out: None,
            check: false,
            strict: true,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            strict_error
                .to_string()
                .contains("smoke-host checks are incomplete")
        );
    }

    #[test]
    fn smoke_host_rejects_contradictory_bridge_and_meter_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
readyAck reply
result=0
bridge timeout
"#,
        )
        .unwrap();
        fs::write(&meter_log, "meter_flush sent=1\nmeter stream failed\n").unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "JSBridge trace"
                && check.status == "failed"
                && check
                    .value
                    .contains("no accepted bridge roundtrip or param gesture markers")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "meter stream"
                && check.status == "failed"
                && check
                    .value
                    .contains("no accepted nonzero meter stream markers")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn smoke_host_rejects_symlinked_report_bridge_trace_meter_log_and_parameter_specs() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());

        let external_report = workspace.join("external-smoke-host.json");
        let report_path = workspace.join("smoke-host.json");
        fs::write(&external_report, "{}").unwrap();
        unix_fs::symlink(&external_report, &report_path).unwrap();
        let error = run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: None,
            meter_log: None,
            out: Some(report_path),
            check: true,
            strict: false,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report must not be a symlink")
        );

        let external_bridge_trace = workspace.join("external-bridge-trace.log");
        let bridge_trace = workspace.join("bridge-trace.log");
        fs::write(
            &external_bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        unix_fs::symlink(&external_bridge_trace, &bridge_trace).unwrap();

        let external_meter_log = workspace.join("external-meter.log");
        let meter_log = workspace.join("meter.log");
        fs::write(&external_meter_log, "meter_flush sent=1\n").unwrap();
        unix_fs::symlink(&external_meter_log, &meter_log).unwrap();

        let specs_path = workspace.join("examples/gain/params.specs.json");
        let external_specs = workspace.join("external-params.specs.json");
        fs::copy(&specs_path, &external_specs).unwrap();
        fs::remove_file(&specs_path).unwrap();
        unix_fs::symlink(&external_specs, &specs_path).unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "gain parameter sidecar"
                && check.status == "failed"
                && check
                    .value
                    .contains("parameter specs must not be a symlink")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "JSBridge trace"
                && check.status == "failed"
                && check.value.contains("bridge trace must not be a symlink")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "meter stream"
                && check.status == "failed"
                && check.value.contains("meter log must not be a symlink")
        }));
    }

    #[test]
    fn smoke_host_report_flags_missing_ui_assets_without_claiming_release_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        fs::remove_dir_all(workspace.join("examples/web-ui-param-demo/ui/dist")).unwrap();

        let report = build_smoke_host_report(&workspace, None, None);

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "web-ui-param-demo UI assets"
                && check.status == "failed"
                && check.value.contains("ui/dist")
                && check.value.contains("missing required file")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "JSBridge trace" && check.status == "skipped" })
        );
        assert!(
            report
                .external_evidence_note
                .contains("does not load plugin binaries")
        );
    }

    #[test]
    fn validator_summary_extracts_passed_and_failed_counts() {
        let text = r#"
-------------------------------------------------------------
Result: 47 tests passed, 0 tests failed
-------------------------------------------------------------
"#;
        assert_eq!(validator_test_summary(text), Some((47, 0)));
        assert_eq!(
            validator_test_summary(
                "2026-06-08T10:00:00Z RESULT: 47 TESTS PASSED, 0 TESTS FAILED\n"
            ),
            Some((47, 0))
        );
        assert_eq!(
            validator_test_summary("Tests passed: 47\nTests failed: 0\n"),
            Some((47, 0))
        );
        assert_eq!(
            validator_test_summary("1 test passed\n2 tests failed\n"),
            Some((1, 2))
        );
        assert_eq!(validator_test_summary("Result: unavailable"), None);
        assert_eq!(validator_test_summary("Result: 47 tests passed"), None);
    }

    #[test]
    fn protocol_export_check_detects_snapshot_drift() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();

        check_protocol_export(&out).unwrap();

        fs::write(
            out.join("typescript/protocol/BridgePacket.ts"),
            "export type BridgePacket = never;\n",
        )
        .unwrap();
        let error = check_protocol_export(&out).unwrap_err().to_string();
        assert!(error.contains("protocol export drift detected"));
        assert!(error.contains("changed"));
        assert!(error.contains("typescript/protocol/BridgePacket.ts"));
    }

    #[test]
    fn protocol_release_check_reports_drift_paths_and_snapshot_command() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();
        fs::write(
            out.join("typescript/protocol/BridgePacket.ts"),
            "export type BridgePacket = never;\n",
        )
        .unwrap();

        let check = protocol_release_check(&out, false, false);

        assert_eq!(check.name, "protocol snapshot");
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("changed"));
        assert!(check.value.contains("typescript/protocol/BridgePacket.ts"));
        let expected_hint =
            format!("run `vesty export-types --out {out}` and commit/update the snapshot");
        assert_eq!(check.hint.as_deref(), Some(expected_hint.as_str()));
    }

    #[test]
    fn protocol_release_check_rejects_skip_when_release_artifacts_are_required() {
        let protocol_snapshot = Utf8Path::new("target/vesty-protocol");
        let check = protocol_release_check(protocol_snapshot, true, true);

        assert_eq!(check.name, "protocol snapshot");
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("cannot skip protocol snapshot when --require-release-artifacts is set")
        );
        assert_eq!(
            check.hint.as_deref(),
            Some(
                "run `vesty export-types --out target/vesty-protocol --check` in final release evidence"
            )
        );
    }

    #[test]
    fn plugin_ui_protocol_sources_match_generated_export() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();

        let generated = collect_relative_files(&out.join("typescript")).unwrap();
        let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
        let package_src = workspace_root.join("packages/plugin-ui/src");
        let package = collect_relative_files(&package_src).unwrap();

        for (relative, expected_bytes) in &generated {
            let actual_bytes = package
                .get(relative)
                .unwrap_or_else(|| panic!("missing generated protocol source: {relative}"));
            let expected = String::from_utf8_lossy(expected_bytes).replace("\r\n", "\n");
            let actual = String::from_utf8_lossy(actual_bytes).replace("\r\n", "\n");
            if actual != expected {
                assert_eq!(
                    actual, expected,
                    "vesty-plugin-ui protocol source drifted: {relative}"
                );
            }
        }

        let extra = package
            .keys()
            .filter(|relative| {
                (relative.starts_with("protocol/") || relative.starts_with("serde_json/"))
                    && relative.as_str() != "protocol/index.ts"
                    && !generated.contains_key(*relative)
            })
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            extra.is_empty(),
            "vesty-plugin-ui has stale generated protocol sources: {extra:?}"
        );
    }

    #[test]
    fn release_report_paths_are_portable_across_windows_and_unix_separators() {
        let windows = r"C:\artifacts\release-evidence\package\VestyGain.static-validate.json";
        let portable = "C:/artifacts/release-evidence/package/VestyGain.static-validate.json";

        assert!(release_report_paths_equal(windows, portable));
        assert!(release_report_path_ends_with(
            windows,
            "package/VestyGain.static-validate.json"
        ));
        assert_eq!(portable_report_path(Utf8Path::new(windows)), portable);
        assert_eq!(
            recognized_json_artifact_name_from_path(Utf8Path::new(windows)).map(|(name, _)| name),
            Some("vst3 static validate report")
        );
        assert!(validate_report_path_prefers_static(Utf8Path::new(windows)));
    }

    #[test]
    fn workspace_packages_have_release_metadata() {
        let root = workspace_root();
        let root_manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(!root_manifest.contains("example.com"));
        for expected in [
            "authors = [\"Vesty Contributors\"]",
            "categories = [",
            "homepage = \"https://github.com/backrunner/vesty\"",
            "keywords = [",
            "license = \"Apache-2.0\"",
            "repository = \"https://github.com/backrunner/vesty\"",
        ] {
            assert!(
                root_manifest.contains(expected),
                "workspace Cargo.toml missing release metadata: {expected}"
            );
        }
        assert!(root.join("README.md").is_file());
        assert!(root.join("LICENSE-APACHE").is_file());
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        for expected in [
            "FloatParam::new(\"gain\", \"Gain\", -60.0, 12.0, 0.0).with_unit(\"dB\")",
            "const INFO: PluginInfo = PluginInfo",
            "fn params(&self) -> &Self::Params",
            "fn create_kernel(&self, _init: KernelInit) -> Self::Kernel",
            "fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult",
            "audio.copy_input_to_output(channel, gain)",
        ] {
            assert!(
                readme.contains(expected),
                "README minimal plugin example is missing current API shape: {expected}"
            );
        }
        for stale in [
            "#[param(id",
            "fn create_kernel(params:",
            "copy_input_to_output(linear)",
        ] {
            assert!(
                !readme.contains(stale),
                "README minimal plugin example contains stale API shape: {stale}"
            );
        }

        for entry in fs::read_dir(root.join("crates")).unwrap() {
            let entry = entry.unwrap();
            let manifest_path = Utf8PathBuf::from_path_buf(entry.path())
                .unwrap()
                .join("Cargo.toml");
            let manifest = fs::read_to_string(&manifest_path).unwrap();
            for expected in [
                "description = ",
                "authors.workspace = true",
                "categories.workspace = true",
                "homepage.workspace = true",
                "keywords.workspace = true",
                "readme = \"../../README.md\"",
                "repository.workspace = true",
            ] {
                assert!(
                    manifest.contains(expected),
                    "{manifest_path} missing release metadata: {expected}"
                );
            }
            assert!(!manifest.contains("example.com"), "{manifest_path}");
        }

        for entry in fs::read_dir(root.join("examples")).unwrap() {
            let entry = entry.unwrap();
            let example_dir = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
            let manifest_path = example_dir.join("Cargo.toml");
            let manifest = fs::read_to_string(&manifest_path).unwrap();
            assert!(
                manifest.contains("publish = false"),
                "{manifest_path} should not be publishable"
            );
            let source = fs::read_to_string(example_dir.join("src/lib.rs")).unwrap();
            assert!(
                source.contains("url: \"https://github.com/backrunner/vesty\""),
                "{example_dir}/src/lib.rs should use the project URL"
            );
            assert!(
                !source.contains("example.com") && !source.contains("dev@example.com"),
                "{example_dir}/src/lib.rs should not contain placeholder contact metadata"
            );
        }

        for package in ["plugin-ui"] {
            let package_json =
                fs::read_to_string(root.join("packages").join(package).join("package.json"))
                    .unwrap();
            for expected in [
                "\"description\":",
                "\"license\": \"Apache-2.0\"",
                "\"repository\":",
                "\"homepage\": \"https://github.com/backrunner/vesty#readme\"",
                "\"keywords\": [",
                "\"exports\":",
                "\"files\": [",
            ] {
                assert!(
                    package_json.contains(expected),
                    "packages/{package}/package.json missing release metadata: {expected}"
                );
            }
            assert!(!package_json.contains("example.com"), "packages/{package}");
        }
    }

    #[test]
    fn ci_package_static_validate_uses_strict_binary_export_gate() {
        let workflow = fs::read_to_string(workspace_root().join(".github/workflows/ci.yml"))
            .expect("ci workflow should be readable");
        let static_validate_step = workflow
            .split("- name: Static validate packaged bundles")
            .nth(1)
            .and_then(|tail| tail.split("- uses: actions/upload-artifact").next())
            .expect("package static validate step should exist");

        assert_eq!(
            static_validate_step
                .matches("cargo run -p vesty-cli -- validate")
                .count(),
            3,
            "package smoke should static-validate all three examples"
        );
        assert_eq!(
            static_validate_step.matches("--static-only").count(),
            3,
            "package smoke validate commands should stay static-only"
        );
        assert_eq!(
            static_validate_step.matches("--strict").count(),
            3,
            "package smoke must fail early on skipped binary export evidence"
        );
        for bundle in ["VestyGain", "VestyWebUIDemo", "VestyMIDISynth"] {
            assert!(
                static_validate_step.contains(&format!("target/ci-package/{bundle}.vst3")),
                "package smoke should validate {bundle}.vst3"
            );
            assert!(
                static_validate_step.contains(&format!(
                    "--report target/ci-package/{bundle}.validate.json"
                )),
                "package smoke should upload {bundle} static validate report"
            );
        }
    }

    #[test]
    fn doctor_report_includes_toolchain_webview_and_validator_checks() {
        let report = doctor_report();
        validate_doctor_report(&report).unwrap();
        assert_eq!(report.os.as_deref(), Some(doctor_os_label()));
        let names = report
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"rustc"));
        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"node"));
        assert!(names.contains(&"npm"));
        assert!(names.contains(&"vst3 binding baseline"));
        assert!(names.contains(&"vst3 SDK headers"));
        assert!(names.contains(&"vst3 validator"));
        assert!(names.contains(&"system webview"));
        let baseline = report
            .checks
            .iter()
            .find(|check| check.name == "vst3 binding baseline")
            .expect("binding baseline check");
        assert_eq!(baseline.status, "ok");
        assert!(baseline.value.contains("v3.8.0_build_66"));
        assert!(baseline.value.contains("upstream vst3 crate 0.3.0"));
        let sdk_headers = report
            .checks
            .iter()
            .find(|check| check.name == "vst3 SDK headers")
            .expect("vst3 SDK headers check");
        assert!(matches!(
            sdk_headers.status.as_str(),
            "ok" | "missing" | "skipped"
        ));
        if cfg!(target_os = "macos") {
            assert!(names.contains(&"signing: codesign"));
            assert!(names.contains(&"signing: notarytool"));
        } else if cfg!(target_os = "windows") {
            assert!(names.contains(&"signing: signtool"));
        } else if cfg!(target_os = "linux") {
            assert!(names.contains(&"signing: linux release policy"));
        } else {
            assert!(names.contains(&"signing: release signing"));
        }
        assert!(names.contains(&"daw install: REAPER"));
        assert!(names.contains(&"daw install: Cubase/Nuendo"));
        assert!(names.contains(&"daw install: Bitwig Studio"));
        assert!(names.contains(&"daw install: Ableton Live"));
        assert!(names.contains(&"daw install: Studio One"));
    }

    #[test]
    fn command_presence_check_uses_candidate_paths_and_missing_hint() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let tool = root.join("tool");
        fs::write(&tool, "test").unwrap();

        let found = command_presence_check(
            "signing: test",
            "definitely-not-a-vesty-test-tool",
            std::slice::from_ref(&tool),
            "install it",
        );
        assert_eq!(found.status, "ok");
        assert_eq!(found.value, tool.to_string());
        assert_eq!(found.hint, None);

        let missing = command_presence_check(
            "signing: missing",
            "definitely-not-a-vesty-test-tool",
            &[],
            "install it",
        );
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.hint.as_deref(), Some("install it"));
    }

    #[test]
    fn bundle_signing_command_maps_platforms() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let binary = Utf8Path::new("/tmp/VestyGain.vst3/Contents/x86_64-win/VestyGain.vst3");

        let mac = bundle_signing_command(
            BundlePlatform::Macos,
            bundle,
            binary,
            "Developer ID Application: Example",
        )
        .unwrap();
        assert!(mac.program.ends_with("codesign"));
        assert_eq!(mac.args[0], "--force");
        assert!(mac.args.contains(&"--deep".to_string()));
        assert!(mac.args.contains(&"runtime".to_string()));
        assert!(
            mac.args
                .contains(&"Developer ID Application: Example".to_string())
        );
        assert_eq!(mac.args.last().map(String::as_str), Some(bundle.as_str()));

        let windows = bundle_signing_command(
            BundlePlatform::WindowsX64,
            bundle,
            binary,
            "Example Code Signing",
        )
        .unwrap();
        assert!(windows.program.ends_with("signtool.exe"));
        assert_eq!(windows.args[0], "sign");
        assert!(windows.args.contains(&"/fd".to_string()));
        assert!(windows.args.contains(&"SHA256".to_string()));
        assert!(windows.args.contains(&"/tr".to_string()));
        assert!(windows.args.contains(&"Example Code Signing".to_string()));
        assert_eq!(
            windows.args.last().map(String::as_str),
            Some(binary.as_str())
        );

        let linux = bundle_signing_command(BundlePlatform::LinuxX64, bundle, binary, "Example")
            .unwrap_err()
            .to_string();
        assert!(linux.contains("release-channel specific"));

        let empty = bundle_signing_command(BundlePlatform::Macos, bundle, binary, "  ")
            .unwrap_err()
            .to_string();
        assert!(empty.contains("cannot be empty"));
    }

    #[test]
    fn dev_install_mode_parses_copy_and_symlink() {
        assert_eq!(
            parse_dev_install_mode("copy").unwrap(),
            DevInstallMode::Copy
        );
        assert_eq!(
            parse_dev_install_mode("symlink").unwrap(),
            DevInstallMode::Symlink
        );
        assert_eq!(
            parse_dev_install_mode("link").unwrap(),
            DevInstallMode::Symlink
        );
        assert!(parse_dev_install_mode("move").is_err());
    }

    #[test]
    fn dev_binary_autodiscovery_uses_root_cdylib_target() {
        let metadata = serde_json::json!({
            "root_package": "path+file:///plugin#demo-plugin@0.1.0",
            "packages": [
                {
                    "id": "path+file:///plugin#demo-plugin@0.1.0",
                    "targets": [
                        {
                            "name": "demo-plugin",
                            "kind": ["cdylib", "rlib"]
                        }
                    ]
                }
            ],
            "target_directory": "/tmp/plugin/target"
        });

        assert_eq!(
            cdylib_target_name_from_metadata(&metadata, None).unwrap(),
            "demo-plugin"
        );
        let filename = cdylib_filename("demo-plugin");
        if cfg!(target_os = "windows") {
            assert_eq!(filename, "demo_plugin.dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(filename, "libdemo_plugin.dylib");
        } else {
            assert_eq!(filename, "libdemo_plugin.so");
        }
    }

    #[test]
    fn dev_binary_autodiscovery_uses_current_manifest_in_workspace_metadata() {
        let metadata = serde_json::json!({
            "packages": [
                {
                    "id": "path+file:///workspace/crates/vesty#0.1.0",
                    "manifest_path": "/workspace/crates/vesty/Cargo.toml",
                    "targets": [
                        {
                            "name": "vesty",
                            "kind": ["lib"]
                        }
                    ]
                },
                {
                    "id": "path+file:///workspace/examples/gain#vesty-example-gain@0.1.0",
                    "manifest_path": "/workspace/examples/gain/Cargo.toml",
                    "targets": [
                        {
                            "name": "vesty_example_gain",
                            "kind": ["rlib", "cdylib"]
                        }
                    ]
                }
            ],
            "workspace_default_members": [
                "path+file:///workspace/crates/vesty#0.1.0"
            ],
            "target_directory": "/workspace/target"
        });

        assert_eq!(
            cdylib_target_name_from_metadata(
                &metadata,
                Some(Utf8Path::new("/workspace/examples/gain/Cargo.toml")),
            )
            .unwrap(),
            "vesty_example_gain"
        );
    }

    #[test]
    fn install_dev_bundle_copies_and_replaces_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();
        let install_dir = root.join("user-vst3");

        let installed = install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed, install_dir.join("VestyGain.vst3"));
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=1"
        );

        fs::write(&resource, "version=2").unwrap();
        let installed_again =
            install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed_again, installed);
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=2"
        );
    }

    #[test]
    fn install_dev_bundle_rejects_non_vst3_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("NotAPlugin");
        fs::create_dir(&source).unwrap();

        let error = install_dev_bundle(&source, &root.join("user-vst3"), DevInstallMode::Copy)
            .unwrap_err()
            .to_string();

        assert!(error.contains(".vst3"));
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_rejects_symlinked_source_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();
        let bundle_link = root.join("build/VestyGainLink.vst3");
        unix_fs::symlink(&bundle, &bundle_link).unwrap();

        let error = install_dev_bundle(&bundle_link, &root.join("user-vst3"), DevInstallMode::Copy)
            .expect_err("dev install should reject symlinked source bundle")
            .to_string();

        assert!(error.contains("dev install source must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_rejects_symlinked_install_dir_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();

        let external = root.join("external-vst3");
        let parent_link = root.join("linked-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let error = install_dev_bundle(
            &bundle,
            &parent_link.join("user-vst3"),
            DevInstallMode::Copy,
        )
        .expect_err("dev install should reject symlinked install dir parents")
        .to_string();

        assert!(error.contains("dev install directory parent must not be a symlink"));
        assert!(!external.join("user-vst3").exists());
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_unlinks_existing_destination_symlink_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();

        let install_dir = root.join("user-vst3");
        let external = root.join("external-installed");
        let destination = install_dir.join("VestyGain.vst3");
        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir(&external).unwrap();
        fs::write(external.join("keep.txt"), "do not remove\n").unwrap();
        unix_fs::symlink(&external, &destination).unwrap();

        let installed = install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed, destination);
        assert_eq!(
            fs::read_to_string(external.join("keep.txt")).unwrap(),
            "do not remove\n"
        );
        assert!(
            !fs::symlink_metadata(&installed)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=1"
        );
    }

    #[test]
    fn notarization_credentials_reject_missing_or_mixed_modes() {
        assert_eq!(
            notarization_credentials(Some("Profile"), None, None, None).unwrap(),
            NotarizationCredentials::KeychainProfile("Profile".to_string())
        );
        assert_eq!(
            notarization_credentials(
                None,
                Some("dev@example.com"),
                Some("TEAMID"),
                Some("app-pass")
            )
            .unwrap(),
            NotarizationCredentials::AppleId {
                apple_id: "dev@example.com".to_string(),
                team_id: "TEAMID".to_string(),
                password: "app-pass".to_string(),
            }
        );
        assert!(
            notarization_credentials(None, Some("dev@example.com"), None, Some("app-pass"))
                .unwrap_err()
                .to_string()
                .contains("provide --keychain-profile")
        );
        assert!(
            notarization_credentials(Some("Profile"), Some("dev@example.com"), None, None)
                .unwrap_err()
                .to_string()
                .contains("not both")
        );
    }

    #[test]
    fn notarization_plan_builds_archive_submit_and_staple_commands() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let credentials = NotarizationCredentials::KeychainProfile("VestyNotary".to_string());
        let plan = notarization_plan(bundle, None, &credentials, true, true).unwrap();

        assert_eq!(plan.archive.as_str(), "/tmp/VestyGain.vst3.zip");
        assert_eq!(plan.commands.len(), 3);
        assert!(plan.commands[0].program.ends_with("ditto"));
        assert_eq!(plan.commands[0].args[0], "-c");
        assert!(plan.commands[0].args.contains(&"--keepParent".to_string()));
        assert_eq!(
            plan.commands[0].args.last().map(String::as_str),
            Some("/tmp/VestyGain.vst3.zip")
        );
        assert!(plan.commands[1].program.ends_with("xcrun"));
        assert_eq!(plan.commands[1].args[0], "notarytool");
        assert!(plan.commands[1].args.contains(&"--wait".to_string()));
        assert!(
            plan.commands[1]
                .args
                .contains(&"--keychain-profile".to_string())
        );
        assert!(plan.commands[1].args.contains(&"VestyNotary".to_string()));
        assert_eq!(
            plan.commands[2].args,
            ["stapler", "staple", bundle.as_str()]
        );
    }

    #[test]
    fn notarization_plan_supports_apple_id_without_wait_or_staple() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let archive = Utf8Path::new("/tmp/VestyGain-notary.zip");
        let credentials = NotarizationCredentials::AppleId {
            apple_id: "dev@example.com".to_string(),
            team_id: "TEAMID".to_string(),
            password: "app-pass".to_string(),
        };
        let plan = notarization_plan(bundle, Some(archive), &credentials, false, false).unwrap();

        assert_eq!(plan.archive, archive);
        assert_eq!(plan.commands.len(), 2);
        assert!(!plan.commands[1].args.contains(&"--wait".to_string()));
        assert!(plan.commands[1].args.contains(&"--apple-id".to_string()));
        assert!(
            plan.commands[1]
                .args
                .contains(&"dev@example.com".to_string())
        );
        assert!(plan.commands[1].args.contains(&"--team-id".to_string()));
        assert!(plan.commands[1].args.contains(&"TEAMID".to_string()));
        assert!(plan.commands[1].args.contains(&"--password".to_string()));
        assert!(plan.commands[1].args.contains(&"app-pass".to_string()));

        let error = notarization_plan(bundle, Some(archive), &credentials, false, true)
            .unwrap_err()
            .to_string();
        assert!(error.contains("--no-wait"));
    }

    #[test]
    fn validate_report_and_validator_log_can_be_written_to_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("reports/validate.json");
        let validator_log_path = root.join("logs/validator.log");
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("moduleinfo.json".to_string()),
                binaries: vec!["Plugin".to_string()],
                binary_exports: Vec::new(),
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        write_validate_report(Some(&report_path), &report).unwrap();
        write_validator_log(
            Some(&validator_log_path),
            Utf8Path::new("/tools/validator"),
            Utf8Path::new("Plugin.vst3"),
            "Result: 1 tests passed, 0 tests failed\n",
            "",
        )
        .unwrap();

        let report_text = fs::read_to_string(report_path).unwrap();
        assert!(report_text.contains(r#""bundle": "Plugin.vst3""#));
        let log_text = fs::read_to_string(validator_log_path).unwrap();
        assert!(log_text.contains("validator=/tools/validator"));
        assert!(log_text.contains("[stdout]"));
    }

    #[cfg(unix)]
    #[test]
    fn report_writers_reject_symlink_output_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external.json");
        let report_path = root.join("reports/validate.json");
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, &report_path).unwrap();
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("moduleinfo.json".to_string()),
                binaries: vec!["Plugin".to_string()],
                binary_exports: Vec::new(),
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        let error = write_validate_report(Some(&report_path), &report)
            .expect_err("report writer should reject symlink output")
            .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn validator_log_writer_rejects_symlink_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external.log");
        let validator_log_path = root.join("logs/validator.log");
        fs::create_dir_all(validator_log_path.parent().unwrap()).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, &validator_log_path).unwrap();

        let error = write_validator_log(
            Some(&validator_log_path),
            Utf8Path::new("/tools/validator"),
            Utf8Path::new("Plugin.vst3"),
            "Result: 1 tests passed, 0 tests failed\n",
            "",
        )
        .expect_err("validator log writer should reject symlink output")
        .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[test]
    fn daw_install_check_reports_detected_path_without_smoke_claim() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let app = root.join("REAPER.app");
        fs::create_dir(&app).unwrap();

        let check = daw_install_check("REAPER", vec![app.clone()]);
        assert_eq!(check.status, "ok");
        assert_eq!(check.value, app.to_string());
        assert!(
            check
                .hint
                .as_deref()
                .unwrap()
                .contains("smoke evidence is still required")
        );

        let missing = daw_install_check("REAPER", vec![root.join("Missing.app")]);
        assert_eq!(missing.status, "missing");
    }

    #[test]
    fn host_quirks_can_filter_by_alias() {
        let profiles = selected_host_profiles(Some("Live")).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "ableton-live");
        assert_eq!(
            profiles[0].required_smoke_checks,
            vesty_core::RELEASE_SMOKE_CHECKS
        );

        let all = selected_host_profiles(None).unwrap();
        assert_eq!(all.len(), 5);
        assert!(selected_host_profiles(Some("missing host")).is_err());
    }

    #[test]
    fn host_quirk_profiles_serialize_for_json_output() {
        let profiles = selected_host_profiles(Some("Cubase")).unwrap();
        let value = serde_json::to_value(&profiles).unwrap();
        assert_eq!(value[0]["name"], "Cubase/Nuendo");
        assert_eq!(value[0]["required_smoke_checks"][0], "scan");
        assert!(value[0]["quirks"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn host_evidence_readme_includes_profile_checks_and_quirks() {
        let readme = host_evidence_readme("Bitwig Studio");

        assert!(readme.contains("# Bitwig Studio Vesty Smoke Evidence"));
        assert!(readme.contains("Host id: `bitwig`"));
        assert!(readme.contains("Platforms: macos, windows, linux-x11"));
        assert!(readme.contains(
            "Required smoke checks: scan, load, ui, ui_host_param, meter_stream, automation, buffer_sample_rate_change, save_restore, offline_render"
        ));
        assert!(readme.contains("Wayland support is experimental"));
        assert!(readme.contains("Meter/analyzer streams are latest-wins"));
        assert!(readme.contains("Templates, pending values and `vesty doctor` install detection do not count as pass evidence."));
        assert!(readme.contains("Accepted Pass Markers"));
        assert!(readme.contains("scan=true"));
        assert!(readme.contains("ui_host_param=true"));
        assert!(readme.contains("meter_flush sent=1"));
        assert!(readme.contains("buffer_sample_rate_change=true"));
        assert!(readme.contains("render_file=/absolute/path/to/rendered.wav"));
        assert!(readme.contains("vesty daw-matrix --evidence-root target/daw-evidence --strict"));
    }

    #[test]
    fn release_check_passes_with_complete_matrix_and_protocol_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let protocol = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();

        let report =
            build_release_check_report(rows, &protocol, false, &ReleaseEvidenceOptions::default());

        assert!(
            release_check_complete(&report),
            "expected complete release check, failed checks: {}",
            report
                .checks
                .iter()
                .filter(|check| check.status == "failed")
                .map(|check| format!("{} = {}", check.name, check.value))
                .collect::<Vec<_>>()
                .join("; ")
        );
        assert_eq!(report.status, "ok");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "protocol snapshot" && check.status == "ok")
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 binding baseline"
                && check.status == "ok"
                && check.value.contains("v3.8.0_build_66")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "ci doctor artifacts" && check.status == "skipped" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK header manifest" && check.status == "skipped"
        }));
    }

    #[test]
    fn release_check_reports_missing_host_evidence() {
        let rows = vec![
            complete_release_row("REAPER"),
            serde_json::json!({
                "host": "Cubase/Nuendo",
                "scan": true,
                "load": false,
                "ui": false,
                "ui_host_param": false,
                "meter_stream": false,
                "automation": false,
                "buffer_sample_rate_change": false,
                "save_restore": false,
                "offline_render": false,
                "evidence": "target/cubase-smoke",
            }),
        ];

        let report = build_release_check_report(
            rows,
            Utf8Path::new("target/missing-protocol"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        assert!(!release_check_complete(&report));
        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "daw smoke: Cubase/Nuendo"
                && check.status == "failed"
                && check.value.contains("load")
        }));
    }

    #[test]
    fn release_action_plan_lists_required_and_optional_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                if profile.name == "Cubase/Nuendo" {
                    serde_json::json!({
                        "host": "Cubase/Nuendo",
                        "platform": "macOS test platform",
                        "platform_supported": true,
                        "scan": true,
                        "load": false,
                        "ui": false,
                        "ui_host_param": false,
                        "meter_stream": false,
                        "automation": false,
                        "buffer_sample_rate_change": false,
                        "save_restore": false,
                        "offline_render": false,
                        "evidence": evidence_root.join("cubase").to_string(),
                    })
                } else {
                    complete_release_row(profile.name)
                }
            })
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let report = build_release_check_report(rows, &protocol, true, &options);

        assert!(!release_check_complete(&report));
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        assert_eq!(plan.version, 1);
        assert_eq!(plan.status, "failed");
        assert_eq!(
            plan.summary.action_count,
            report
                .checks
                .iter()
                .filter(|check| check.status != "ok")
                .count()
        );
        let daw_matrix = plan
            .actions
            .iter()
            .find(|action| action.check == "daw matrix")
            .expect("daw matrix action");
        assert_eq!(
            daw_matrix.evidence_path.as_deref(),
            Some(portable_report_path(&evidence_root).as_str())
        );

        let cubase = plan
            .actions
            .iter()
            .find(|action| action.check == "daw smoke: Cubase/Nuendo")
            .expect("cubase action");
        assert_eq!(cubase.priority, "required");
        assert_eq!(
            cubase.evidence_path.as_deref(),
            Some(portable_report_path(&evidence_root.join("cubase")).as_str())
        );
        assert!(cubase.commands.iter().any(|command| {
            command.contains("vesty daw-matrix --write-report --host cubase-nuendo")
        }));

        let platform = plan
            .actions
            .iter()
            .find(|action| action.check == "platform smoke artifacts")
            .expect("platform action");
        assert_eq!(platform.priority, "required");
        assert_eq!(
            platform.evidence_path.as_deref(),
            Some(portable_report_path(&release_evidence_dir.join("platform-smoke")).as_str())
        );
        assert!(
            platform
                .commands
                .iter()
                .any(|command| command.contains("vesty platform-smoke --write-report"))
        );
        let platform_commands = platform.commands.join("\n");
        assert!(platform_commands.contains("--platform macos"));
        assert!(platform_commands.contains("WebKit.framework loaded"));
        assert!(platform_commands.contains("--platform windows-x64"));
        assert!(platform_commands.contains("WebView2 runtime loaded"));
        assert!(platform_commands.contains("--platform linux-x11"));
        assert!(platform_commands.contains("WebKitGTK loaded; X11 display active"));
        assert!(!platform_commands.contains("system_webview=true"));
        assert!(!platform_commands.contains("vst3_validator=true"));

        let crate_package = plan
            .actions
            .iter()
            .find(|action| action.check == "crate package readiness")
            .expect("crate package readiness action");
        assert_eq!(crate_package.status, "failed");
        assert_eq!(crate_package.priority, "required");
        assert_eq!(
            crate_package.evidence_path.as_deref(),
            Some(
                portable_report_path(
                    &release_evidence_dir.join("crate-package/crate-package.json")
                )
                .as_str()
            )
        );
        assert!(
            crate_package
                .commands
                .iter()
                .any(|command| command.contains("vesty crate-package --out"))
        );
        assert!(
            crate_package
                .commands
                .iter()
                .any(|command| command.contains("vesty crate-package --check --out"))
        );

        let sdk = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 SDK header manifest")
            .expect("sdk action");
        assert_eq!(sdk.status, "skipped");
        assert_eq!(sdk.priority, "optional");
        assert!(
            sdk.commands
                .iter()
                .any(|command| command.contains("vesty vst3-sdk manifest"))
        );
        assert!(
            sdk.commands
                .iter()
                .any(|command| command.contains("vesty vst3-sdk manifest")
                    && command.contains("--check"))
        );

        let plan_path = root.join("release-action-plan.json");
        write_release_action_plan(&plan_path, &plan).unwrap();
        let decoded: ReleaseActionPlan =
            serde_json::from_str(&fs::read_to_string(plan_path).unwrap()).unwrap();
        assert_eq!(decoded, plan);
    }

    #[test]
    fn release_action_plan_uses_default_release_evidence_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                serde_json::json!({
                    "host": profile.name,
                    "scan": false,
                    "load": false,
                    "ui": false,
                    "ui_host_param": false,
                    "meter_stream": false,
                    "automation": false,
                    "buffer_sample_rate_change": false,
                    "save_restore": false,
                    "offline_render": false,
                    "evidence": Utf8PathBuf::from("target/daw-evidence").join(profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let report = build_release_check_report(rows, &protocol, true, &options);
        let plan = build_release_action_plan(&report, &protocol, None, None);

        let daw_matrix = plan
            .actions
            .iter()
            .find(|action| action.check == "daw matrix")
            .expect("daw matrix action");
        assert_eq!(
            daw_matrix.evidence_path.as_deref(),
            Some("target/daw-evidence")
        );

        let protocol_action = plan
            .actions
            .iter()
            .find(|action| action.check == "protocol snapshot")
            .expect("protocol snapshot action");
        assert_eq!(
            protocol_action.evidence_path.as_deref(),
            Some(portable_report_path(&protocol).as_str())
        );

        let ci_run_url = plan
            .actions
            .iter()
            .find(|action| action.check == "ci run url")
            .expect("ci run url action");
        assert_eq!(
            ci_run_url.evidence_path.as_deref(),
            Some("target/release-evidence/ci-run-url.txt")
        );
        assert!(ci_run_url.commands.iter().any(|command| command
            == "vesty release-check --write-evidence-template target/release-evidence"));

        let crate_package = plan
            .actions
            .iter()
            .find(|action| action.check == "crate package readiness")
            .expect("crate package readiness action");
        assert_eq!(
            crate_package.evidence_path.as_deref(),
            Some("target/release-evidence/crate-package/crate-package.json")
        );

        let generic_validator = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 validate reports")
            .expect("vst3 validate reports action");
        assert_eq!(
            generic_validator.evidence_path.as_deref(),
            Some("target/release-evidence/validator")
        );
        assert!(
            generic_validator
                .commands
                .iter()
                .any(|command| command.contains("vesty validate <bundle.vst3> --strict"))
        );
        assert_example_validator_matrix_commands(generic_validator);

        let validator = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 example validator coverage")
            .expect("vst3 example validator coverage action");
        assert_eq!(
            validator.evidence_path.as_deref(),
            Some("target/release-evidence/validator")
        );
        assert!(
            validator
                .commands
                .iter()
                .any(|command| command.contains("target/release-evidence/validator/"))
        );
        assert!(
            validator
                .commands
                .iter()
                .any(|command| { command.contains("vesty validate <bundle.vst3> --strict") })
        );
        assert_example_validator_matrix_commands(validator);

        let generic_static_validate = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 static validate reports")
            .expect("vst3 static validate reports action");
        assert_eq!(
            generic_static_validate.evidence_path.as_deref(),
            Some("target/release-evidence/package")
        );
        assert!(generic_static_validate.commands.iter().any(|command| {
            command.contains("vesty validate <bundle.vst3> --static-only --strict")
        }));
        assert_example_static_matrix_commands(generic_static_validate);

        let static_validate = plan
            .actions
            .iter()
            .find(|action| action.check == "ci example static validate coverage")
            .expect("ci example static validate coverage action");
        assert_eq!(
            static_validate.evidence_path.as_deref(),
            Some("target/release-evidence/package")
        );
        assert!(
            static_validate
                .commands
                .iter()
                .any(|command| command.contains("target/release-evidence/package/"))
        );
        assert!(static_validate.commands.iter().any(|command| {
            command.contains("vesty validate <bundle.vst3> --static-only --strict")
        }));
        assert_example_static_matrix_commands(static_validate);

        let signing = plan
            .actions
            .iter()
            .find(|action| action.check == "signed bundle evidence")
            .expect("signed bundle evidence action");
        assert_eq!(
            signing.evidence_path.as_deref(),
            Some(
                "target/release-evidence/signing-macos.log and target/release-evidence/signing-windows.log"
            )
        );

        let notarization = plan
            .actions
            .iter()
            .find(|action| action.check == "notarization log")
            .expect("notarization log action");
        assert_eq!(
            notarization.evidence_path.as_deref(),
            Some("target/release-evidence/notary.log")
        );
    }

    fn assert_example_validator_matrix_commands(action: &ReleaseActionItem) {
        let matrix_commands = action
            .commands
            .iter()
            .filter(|command| command.contains("<path-to-"))
            .collect::<Vec<_>>();
        assert_eq!(
            matrix_commands.len(),
            REQUIRED_EXAMPLE_BUNDLES.len() * REQUIRED_EXAMPLE_VALIDATE_PLATFORMS.len(),
            "{} should include one concrete validator command per example/platform",
            action.check
        );

        for platform in REQUIRED_EXAMPLE_VALIDATE_PLATFORMS {
            for bundle in REQUIRED_EXAMPLE_BUNDLES {
                let report =
                    format!("target/release-evidence/validator/{bundle}.{platform}.validate.json");
                let log =
                    format!("target/release-evidence/validator/{bundle}.{platform}.validator.log");
                let expected = matrix_commands
                    .iter()
                    .filter(|command| {
                        command.contains(&format!("vesty validate <path-to-{bundle}> --strict"))
                            && command.contains(&report)
                            && command.contains(&log)
                            && !command.contains("--static-only")
                    })
                    .count();
                assert_eq!(
                    expected, 1,
                    "{} should include exactly one strict validator command for {bundle} on {platform}",
                    action.check
                );
            }
        }
    }

    fn assert_example_static_matrix_commands(action: &ReleaseActionItem) {
        let matrix_commands = action
            .commands
            .iter()
            .filter(|command| command.contains("<path-to-"))
            .collect::<Vec<_>>();
        assert_eq!(
            matrix_commands.len(),
            REQUIRED_EXAMPLE_BUNDLES.len() * REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS.len(),
            "{} should include one concrete static validate command per example/platform",
            action.check
        );

        for platform in REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS {
            for bundle in REQUIRED_EXAMPLE_BUNDLES {
                let report = format!(
                    "target/release-evidence/package/{bundle}.{platform}.static-validate.json"
                );
                let expected = matrix_commands
                    .iter()
                    .filter(|command| {
                        command.contains(&format!(
                            "vesty validate <path-to-{bundle}> --static-only --strict"
                        )) && command.contains(&report)
                            && !command.contains("--validator-log")
                    })
                    .count();
                assert_eq!(
                    expected, 1,
                    "{} should include exactly one strict static validate command for {bundle} on {platform}",
                    action.check
                );
            }
        }
    }

