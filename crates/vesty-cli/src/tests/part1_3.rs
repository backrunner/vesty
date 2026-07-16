    #[test]
    fn release_action_plan_daw_host_commands_are_accepted_by_writer() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
        let evidence = resolve_daw_evidence_paths(
            Some(evidence_root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );
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
                    "evidence": daw_evidence_dir_for_host(&evidence, profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let report =
            build_release_check_report(rows, &protocol, true, &ReleaseEvidenceOptions::default());
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        let mut accepted_hosts = BTreeSet::new();
        for action in plan
            .actions
            .iter()
            .filter(|action| action.check.starts_with("daw smoke: "))
        {
            let command = action.commands.first().expect("daw smoke command");
            let host = command_arg(command, "--host").expect("host argument");
            write_daw_smoke_report(
                &evidence,
                DawSmokeReportInput {
                    host: Some(host.to_string()),
                    platform: Some("macos arm64 / test host version".to_string()),
                    scan: Some("scan=true".to_string()),
                    load: Some("load=true".to_string()),
                    ui: Some("ui=true".to_string()),
                    ui_host_param: Some("ui_host_param=true".to_string()),
                    meter_stream: Some("meter_flush sent=1".to_string()),
                    automation: Some("automation=true".to_string()),
                    buffer_sample_rate_change: Some("buffer_sample_rate_change=true".to_string()),
                    save_restore: Some("save_restore=true".to_string()),
                    offline_render: Some("offline_render=true".to_string()),
                },
            )
            .unwrap();
            accepted_hosts.insert(host.to_string());
        }

        assert_eq!(accepted_hosts.len(), vesty_core::host_profiles().len());
        assert!(accepted_hosts.contains("cubase-nuendo"));
        assert!(accepted_hosts.contains("ableton-live"));
        assert!(accepted_hosts.contains("studio-one"));
        assert!(daw_matrix_complete(&daw_matrix_rows(&evidence)));
    }

    #[test]
    fn release_action_plan_vesty_commands_parse_with_current_cli() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
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
                    "evidence": evidence_root.join(profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let report = build_release_check_report(rows, &protocol, true, &options);
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        let mut parsed = 0;
        for action in &plan.actions {
            for command in &action.commands {
                let command = command.trim();
                if !release_action_command_starts_with_vesty(command) {
                    continue;
                }
                let argv = split_release_action_command(command).unwrap_or_else(|error| {
                    panic!(
                        "action `{}` command should split with production parser: {command}\nerror: {error}",
                        action.check
                    )
                });
                assert!(
                    Cli::try_parse_from(&argv).is_ok(),
                    "action `{}` command should parse with current CLI: {command}\nargv: {argv:?}",
                    action.check
                );
                parsed += 1;
            }
        }

        assert!(parsed >= 20, "expected broad action command coverage");
        assert!(plan.actions.iter().any(|action| {
            action.check == "notarization log"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("--notary-log <notarytool.log>"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "crate publish plan"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("publish-plan --check"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "npm package pack report"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("npm-pack --check"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "dependency latest baseline"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("dependency-baseline --latest --check"))
        }));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_incomplete_actions() {
        let mut missing_command = test_release_action_plan();
        missing_command.actions[0].commands.clear();

        let error = validate_release_action_plan_sidecar(&missing_command).unwrap_err();
        assert!(error.contains("has no suggested commands"));

        let mut empty_protocol = test_release_action_plan();
        empty_protocol.protocol_snapshot = " ".to_string();
        let error = validate_release_action_plan_sidecar(&empty_protocol).unwrap_err();
        assert!(error.contains("release action plan protocol snapshot must not be empty"));

        let mut long_value = test_release_action_plan();
        long_value.actions[0].value = "x".repeat(RELEASE_ACTION_TEXT_MAX_BYTES + 1);
        let error = validate_release_action_plan_sidecar(&long_value).unwrap_err();
        assert!(error.contains("value must be at most"));

        let mut bad_evidence_root = test_release_action_plan();
        bad_evidence_root.evidence_root = Some("target/daw-evidence\nbad".to_string());
        let error = validate_release_action_plan_sidecar(&bad_evidence_root).unwrap_err();
        assert!(error.contains("release action plan evidence root must not contain control"));

        let mut escaping_protocol = test_release_action_plan();
        escaping_protocol.protocol_snapshot = "target/vesty-protocol/..".to_string();
        let error = validate_release_action_plan_sidecar(&escaping_protocol).unwrap_err();
        assert!(error.contains("release action plan protocol snapshot"));
        assert!(error.contains("parent-directory components"));

        let mut escaping_evidence_root = test_release_action_plan();
        escaping_evidence_root.evidence_root = Some("target/daw-evidence/..".to_string());
        let error = validate_release_action_plan_sidecar(&escaping_evidence_root).unwrap_err();
        assert!(error.contains("release action plan evidence root"));
        assert!(error.contains("parent-directory components"));

        let mut bad_release_evidence_dir = test_release_action_plan();
        bad_release_evidence_dir.release_evidence_dir =
            Some("target/release-evidence\nbad".to_string());
        let error = validate_release_action_plan_sidecar(&bad_release_evidence_dir).unwrap_err();
        assert!(
            error.contains("release action plan release evidence dir must not contain control")
        );

        let mut escaping_release_evidence_dir = test_release_action_plan();
        escaping_release_evidence_dir.release_evidence_dir =
            Some("target/release-evidence/..".to_string());
        let error =
            validate_release_action_plan_sidecar(&escaping_release_evidence_dir).unwrap_err();
        assert!(error.contains("release action plan release evidence dir"));
        assert!(error.contains("parent-directory components"));

        let mut control_path = test_release_action_plan();
        control_path.actions[0].evidence_path = Some("target/daw-evidence/reaper\nbad".to_string());

        let error = validate_release_action_plan_sidecar(&control_path).unwrap_err();
        assert!(error.contains("evidence path must not contain control characters"));

        let mut escaping_action_path = test_release_action_plan();
        escaping_action_path.actions[0].evidence_path =
            Some("target/daw-evidence/reaper/../cubase".to_string());
        let error = validate_release_action_plan_sidecar(&escaping_action_path).unwrap_err();
        assert!(error.contains("release action `daw smoke: REAPER` evidence path"));
        assert!(error.contains("parent-directory components"));

        let mut wrong_daw_evidence_path = test_release_action_plan();
        wrong_daw_evidence_path.actions[0].evidence_path =
            Some("target/other-daw-evidence/reaper".to_string());
        let error = validate_release_action_plan_sidecar(&wrong_daw_evidence_path).unwrap_err();
        assert!(error.contains("evidence path"));
        assert!(error.contains("does not match expected `target/daw-evidence/reaper`"));

        let mut missing_evidence_path = test_release_action_plan();
        missing_evidence_path.actions[0].evidence_path = None;
        let error = validate_release_action_plan_sidecar(&missing_evidence_path).unwrap_err();
        assert!(error.contains("missing expected evidence path"));

        let mut wrong_release_evidence_path = test_release_action_plan();
        wrong_release_evidence_path.actions[1].evidence_path =
            Some("target/other-release-evidence/vst3-sdk".to_string());
        let error = validate_release_action_plan_sidecar(&wrong_release_evidence_path).unwrap_err();
        assert!(error.contains("does not match expected `target/release-evidence/vst3-sdk"));

        let mut protocol_action = test_release_action_plan();
        protocol_action.actions[0] = ReleaseActionItem {
            check: "protocol snapshot".to_string(),
            status: "failed".to_string(),
            priority: "required".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: Some("target/other-protocol".to_string()),
            commands: vec!["vesty export-types --out target/vesty-protocol".to_string()],
        };
        let error = validate_release_action_plan_sidecar(&protocol_action).unwrap_err();
        assert!(error.contains("does not match expected `target/vesty-protocol`"));

        let mut unsafe_unicode_hint = test_release_action_plan();
        unsafe_unicode_hint.actions[0].hint = Some("collect evidence\u{202e}hidden".to_string());
        let error = validate_release_action_plan_sidecar(&unsafe_unicode_hint).unwrap_err();
        assert!(error.contains("hint must not contain unsafe Unicode format characters"));

        let mut duplicate_action = test_release_action_plan();
        duplicate_action.actions[1].check = duplicate_action.actions[0].check.clone();
        duplicate_action.actions[1].evidence_path =
            duplicate_action.actions[0].evidence_path.clone();
        let error = validate_release_action_plan_sidecar(&duplicate_action).unwrap_err();
        assert!(error.contains("duplicate release action check"));

        let mut bad_pending_count = test_release_action_plan();
        bad_pending_count.summary.action_count += 1;
        bad_pending_count.actions.push(ReleaseActionItem {
            check: "manual follow-up".to_string(),
            status: "ok".to_string(),
            priority: "optional".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: None,
            commands: vec!["vesty release-check --strict".to_string()],
        });
        let error = validate_release_action_plan_sidecar(&bad_pending_count).unwrap_err();
        assert!(error.contains("action pending count mismatch"));

        let mut too_many_commands = test_release_action_plan();
        too_many_commands.actions[0].commands =
            vec!["vesty release-check --strict".to_string(); RELEASE_ACTION_MAX_COMMANDS + 1];
        let error = validate_release_action_plan_sidecar(&too_many_commands).unwrap_err();
        assert!(error.contains("too many suggested commands"));

        let mut absurd_summary = test_release_action_plan();
        absurd_summary.summary.ok = RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS;
        let error = validate_release_action_plan_sidecar(&absurd_summary).unwrap_err();
        assert!(error.contains("summary check count"));
        assert!(error.contains("exceeds maximum"));

        let mut missing_current_gate = test_release_action_plan();
        missing_current_gate.summary.ok -= 1;
        let error = validate_release_action_plan_sidecar(&missing_current_gate).unwrap_err();
        assert!(error.contains("summary check count must match current Vesty release gate"));

        let mut unknown_action = test_release_action_plan();
        unknown_action.actions[0].check = "manual follow-up".to_string();
        unknown_action.actions[0].evidence_path = None;
        let error = validate_release_action_plan_sidecar(&unknown_action).unwrap_err();
        assert!(error.contains("unknown release action check `manual follow-up`"));
    }

    #[test]
    fn release_action_plan_sidecar_accepts_signed_bundle_compound_evidence_path() {
        let mut plan = test_release_action_plan();
        plan.summary.ok = expected_release_check_names().len() - 1;
        plan.summary.failed = 1;
        plan.summary.skipped = 0;
        plan.summary.action_count = 1;
        plan.actions = vec![ReleaseActionItem {
            check: "signed bundle evidence".to_string(),
            status: "failed".to_string(),
            priority: "required".to_string(),
            value: "missing signed bundle evidence".to_string(),
            hint: None,
            evidence_path: Some(
                "target/release-evidence/signing-macos.log and target/release-evidence/signing-windows.log"
                    .to_string(),
            ),
            commands: vec![
                "vesty release-evidence collect-signing <signed-macos-bundle.vst3> --platform macos --dir target/release-evidence"
                    .to_string(),
            ],
        }];

        validate_release_action_plan_sidecar(&plan).unwrap();
    }

    #[test]
    fn release_action_plan_writer_rejects_invalid_plan() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let plan_path = root.join("release-action-plan.json");
        let mut plan = test_release_action_plan();
        plan.actions[0].commands.clear();

        let error = write_release_action_plan(&plan_path, &plan).unwrap_err();
        assert!(error.to_string().contains("invalid release action plan"));
        assert!(error.to_string().contains("has no suggested commands"));
        assert!(!plan_path.exists());
    }

    #[test]
    fn release_action_plan_sidecar_rejects_stale_vesty_commands() {
        let mut stale_command = test_release_action_plan();
        stale_command.actions[0].commands = vec![
            "  vesty release-evidence collect-notarization --notarytool-log notarytool.log --stapler-log stapler.log --dir target/release-evidence"
                .to_string(),
        ];

        let error = validate_release_action_plan_sidecar(&stale_command).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));
        assert!(error.contains("--notarytool-log"));

        let mut bare_vesty = test_release_action_plan();
        bare_vesty.actions[0].commands = vec!["  vesty  ".to_string()];
        let error = validate_release_action_plan_sidecar(&bare_vesty).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));

        let mut unicode_space = test_release_action_plan();
        unicode_space.actions[0].commands = vec![
            "vesty\u{00a0}release-evidence collect-notarization --notarytool-log notarytool.log"
                .to_string(),
        ];
        let error = validate_release_action_plan_sidecar(&unicode_space).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));
        assert!(error.contains("--notarytool-log"));

        let mut unterminated_quote = test_release_action_plan();
        unterminated_quote.actions[0].commands =
            vec!["vesty daw-matrix --write-report --host reaper --ui \"ui=true".to_string()];
        let error = validate_release_action_plan_sidecar(&unterminated_quote).unwrap_err();
        assert!(error.contains("unterminated double quote"));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_failed_empty_plan() {
        let mut empty = test_release_action_plan();
        empty.summary.failed = 0;
        empty.summary.skipped = 0;
        empty.summary.ok = expected_release_check_names().len();
        empty.summary.action_count = 0;
        empty.actions.clear();

        let error = validate_release_action_plan_sidecar(&empty).unwrap_err();
        assert!(error.contains("failed release action plan must contain at least one action"));

        let mut empty_ok = test_release_action_plan();
        empty_ok.status = "ok".to_string();
        empty_ok.summary.ok = 0;
        empty_ok.summary.failed = 0;
        empty_ok.summary.skipped = 0;
        empty_ok.summary.action_count = 0;
        empty_ok.actions.clear();
        let error = validate_release_action_plan_sidecar(&empty_ok).unwrap_err();
        assert!(error.contains("summary must contain at least one check"));

        let mut skipped_only = test_release_action_plan();
        skipped_only.summary.ok = expected_release_check_names().len() - 1;
        skipped_only.summary.failed = 0;
        skipped_only.summary.skipped = 1;
        skipped_only.actions = vec![ReleaseActionItem {
            check: "vst3 SDK header manifest".to_string(),
            status: "skipped".to_string(),
            priority: "optional".to_string(),
            value: "not requested".to_string(),
            hint: Some("optional generated-headers audit".to_string()),
            evidence_path: Some(
                "target/release-evidence/vst3-sdk/vst3-sdk-headers.json".to_string(),
            ),
            commands: vec!["vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK".to_string()],
        }];
        skipped_only.summary.action_count = skipped_only.actions.len();

        let error = validate_release_action_plan_sidecar(&skipped_only).unwrap_err();
        assert!(
            error.contains("failed release action plan must contain at least one failed action")
        );

        let mut no_pending_actions = test_release_action_plan();
        no_pending_actions.summary.failed = 0;
        no_pending_actions.summary.skipped = 0;
        no_pending_actions.actions.clear();
        no_pending_actions.actions.push(ReleaseActionItem {
            check: "manual follow-up".to_string(),
            status: "ok".to_string(),
            priority: "optional".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: None,
            commands: vec!["vesty release-check --strict".to_string()],
        });
        no_pending_actions.summary.action_count = no_pending_actions.actions.len();

        let error = validate_release_action_plan_sidecar(&no_pending_actions).unwrap_err();
        assert!(error.contains("action pending count mismatch"));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_unknown_json_fields() {
        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["summary"]["pending"] = serde_json::json!(1);
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `pending`"));

        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["actions"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));
    }

    #[test]
    fn release_check_requires_release_artifacts_when_requested() {
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci doctor artifacts"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci release-check artifacts"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "crate publish plan" && check.status == "failed" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "crate package readiness"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "npm package pack report" && check.status == "failed"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "dependency latest baseline" && check.status == "failed"
        }));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "signed bundle evidence" && check.status == "failed"
            })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "notarization log" && check.status == "failed" })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 validate reports" && check.status == "failed" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 static validate reports" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "protocol snapshot"
                && check.status == "failed"
                && check.value.contains("cannot skip protocol snapshot")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK header manifest" && check.status == "skipped"
        }));
    }

    #[test]
    fn publish_plan_release_check_validates_dependency_order() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("publish-plan.json");
        write_publish_plan_artifact(&valid);

        let check = publish_plan_release_check(Some(&valid), true);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 publishable crates"));
        assert!(check.value.contains("final crate: vesty"));

        let invalid = root.join("bad-publish-plan.json");
        let mut plan = test_publish_plan();
        plan.packages[1]
            .internal_dependencies
            .push("vesty".to_string());
        fs::write(&invalid, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

        let check = publish_plan_release_check(Some(&invalid), true);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("orders dependency vesty after dependent vesty-core")
        );

        let missing = publish_plan_release_check(None, false);
        assert_eq!(missing.status, "skipped");
        let required = publish_plan_release_check(None, true);
        assert_eq!(required.status, "failed");
    }

    #[test]
    fn publish_plan_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("publish-plan.json");
        write_publish_plan_artifact(&valid);

        run_publish_plan(&root, Some(&valid), true, "text").unwrap();
        run_publish_plan(&root, Some(&valid), true, "json").unwrap();

        let error = run_publish_plan(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));
    }

    #[test]
    fn publish_plan_report_rejects_unknown_json_fields() {
        let unknown_top_level = r#"{
          "packages": [],
          "skipped_private": [],
          "generated_by": "manual"
        }"#;
        let error = serde_json::from_str::<PublishPlan>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generated_by`"));

        let unknown_package_field = r#"{
          "packages": [{
            "order": 1,
            "level": 1,
            "name": "vesty-params",
            "version": "0.1.0",
            "manifest_path": "/workspace/crates/vesty-params/Cargo.toml",
            "internal_dependencies": [],
            "checksum": "hidden"
          }],
          "skipped_private": []
        }"#;
        let error = serde_json::from_str::<PublishPlan>(unknown_package_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));
    }

    #[test]
    fn crate_package_release_check_validates_packaged_and_deferred_entries() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("crate-package.json");
        let valid_publish_plan = root.join("publish-plan.json");
        write_crate_package_artifact(&valid);
        write_publish_plan_artifact(&valid_publish_plan);

        let check = crate_package_release_check(Some(&valid), Some(&valid_publish_plan), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 workspace crates"));
        assert!(check.value.contains("1 packageable now"));
        assert!(check.value.contains("2 deferred"));

        let missing = crate_package_release_check(None, None, false);
        assert_eq!(missing.status, "skipped");
        let required = crate_package_release_check(None, None, true);
        assert_eq!(required.status, "failed");

        let invalid = root.join("bad-crate-package.json");
        let mut report = test_crate_package_report();
        report.packages[0].status = "deferred".to_string();
        report.packages[0].reason = Some("not actually packageable".to_string());
        fs::write(&invalid, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = crate_package_release_check(Some(&invalid), Some(&valid_publish_plan), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("vesty-params"));
        assert!(check.value.contains("expected packaged"));
    }

    #[test]
    fn crate_package_release_check_rejects_mismatched_publish_plan_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let crate_package = root.join("crate-package.json");
        let publish_plan = root.join("publish-plan.json");
        write_crate_package_artifact(&crate_package);

        let mut mismatched_plan = test_publish_plan();
        mismatched_plan.packages[1].version = "0.2.0".to_string();
        mismatched_plan.packages[1].manifest_path =
            "/workspace/crates/renamed-core/Cargo.toml".to_string();
        write_publish_plan_artifact_with_plan(&publish_plan, &mismatched_plan);

        let check = crate_package_release_check(Some(&crate_package), Some(&publish_plan), true);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("does not match crate publish plan evidence")
        );
        assert!(check.value.contains("vesty-core"));
        assert!(check.value.contains("0.2.0"));
        assert!(check.value.contains("manifest path"));
    }

    #[test]
    fn crate_package_report_rejects_entries_out_of_sync_with_embedded_publish_plan() {
        let mut missing_entry = test_crate_package_report();
        missing_entry.packages.pop();
        let error = validate_crate_package_report(&missing_entry).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package report is out of sync with embedded publish plan")
        );
        assert!(
            error
                .to_string()
                .contains("embedded publish plan package vesty is missing")
        );

        let mut extra_entry = test_crate_package_report();
        extra_entry.packages.push(CratePackageEntry {
            name: "vesty-extra".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: "/workspace/crates/vesty-extra/Cargo.toml".to_string(),
            publish_order: 4,
            internal_dependencies: Vec::new(),
            status: "packaged".to_string(),
            reason: None,
        });
        let error = validate_crate_package_report(&extra_entry).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package entry vesty-extra is not present")
        );

        let mut drifted_entry = test_crate_package_report();
        drifted_entry.packages[1].manifest_path =
            "/workspace/crates/renamed-core/Cargo.toml".to_string();
        let error = validate_crate_package_report(&drifted_entry).unwrap_err();
        assert!(error.to_string().contains("manifest path"));
        assert!(error.to_string().contains("vesty-core"));
    }

    #[test]
    fn crate_package_report_rejects_unknown_json_fields() {
        let mut report = serde_json::to_value(test_crate_package_report()).unwrap();
        report["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<CratePackageReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut report = serde_json::to_value(test_crate_package_report()).unwrap();
        report["packages"][0]["checksum"] = serde_json::json!("hidden");
        let error = serde_json::from_value::<CratePackageReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));
    }

    #[test]
    fn crate_package_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("crate-package.json");
        write_crate_package_artifact(&valid);

        run_crate_package(&root, Some(&valid), true, "text").unwrap();
        run_crate_package(&root, Some(&valid), true, "json").unwrap();

        let error = run_crate_package(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));
    }

    #[test]
    fn npm_pack_release_check_validates_workspace_package_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("npm-pack.json");
        write_npm_pack_artifact(&valid);

        let check = npm_pack_release_check(Some(&valid), true);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("1 package"));
        assert!(check.value.contains("vesty-plugin-ui"));

        let missing = npm_pack_release_check(None, false);
        assert_eq!(missing.status, "skipped");
        let required = npm_pack_release_check(None, true);
        assert_eq!(required.status, "failed");

        let mut report = test_npm_pack_report();
        report[0].files.push(NpmPackFile {
            path: "src/index.ts".to_string(),
        });
        let invalid = root.join("bad-npm-pack.json");
        fs::write(&invalid, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = npm_pack_release_check(Some(&invalid), true);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("non-release file"));
        assert!(check.value.contains("src/index.ts"));
    }

    #[test]
    fn npm_pack_report_rejects_unknown_json_fields() {
        let unknown_entry = r#"[
          {
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [{ "path": "package.json" }, { "path": "dist/index.js" }],
            "scripts": { "prepack": "echo hidden" }
          }
        ]"#;
        let error = parse_npm_pack_report_text(unknown_entry).unwrap_err();
        assert!(error.to_string().contains("unknown field `scripts`"));

        let unknown_file = r#"[
          {
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [{ "path": "package.json", "mode": 420 }]
          }
        ]"#;
        let error = parse_npm_pack_report_text(unknown_file).unwrap_err();
        assert!(error.to_string().contains("unknown field `mode`"));
    }

    #[test]
    fn npm_pack_command_output_normalizes_external_metadata() {
        let command_output = r#"[
          {
            "id": "vesty-plugin-ui@0.1.0",
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "size": 1024,
            "unpackedSize": 4096,
            "shasum": "abc",
            "integrity": "sha512-abc",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [
              { "path": "package.json", "size": 512, "mode": 420 },
              { "path": "dist/index.js", "size": 512, "mode": 420 }
            ],
            "entryCount": 2,
            "bundled": []
          }
        ]"#;

        let entries = parse_npm_pack_command_output(command_output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "vesty-plugin-ui");
        assert_eq!(entries[0].files.len(), 2);
        assert_eq!(entries[0].files[1].path, "dist/index.js");

        let normalized = serde_json::to_value(entries).unwrap();
        assert_eq!(normalized[0]["name"], "vesty-plugin-ui");
        assert_eq!(normalized[0]["files"][0]["path"], "package.json");
        assert!(normalized[0].get("id").is_none());
        assert!(normalized[0]["files"][0].get("mode").is_none());
    }

    #[test]
    fn publish_crate_and_npm_reports_reject_malformed_shape_fields() {
        let mut unsafe_skipped = test_publish_plan();
        unsafe_skipped
            .skipped_private
            .push("hidden\u{202e}package".to_string());
        let error = validate_publish_plan(&unsafe_skipped).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut duplicate_publish_dependency = test_publish_plan();
        duplicate_publish_dependency.packages[1]
            .internal_dependencies
            .push("vesty-params".to_string());
        let error = validate_publish_plan(&duplicate_publish_dependency).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate internal dependency `vesty-params`")
        );

        let mut too_many_publish_packages = test_publish_plan();
        while too_many_publish_packages.packages.len() <= PUBLISH_PLAN_MAX_PACKAGES {
            let index = too_many_publish_packages.packages.len() + 1;
            too_many_publish_packages.packages.push(PublishPlanPackage {
                order: index,
                level: index,
                name: format!("vesty-extra-{index}"),
                version: "0.1.0".to_string(),
                manifest_path: format!("/workspace/crates/vesty-extra-{index}/Cargo.toml"),
                internal_dependencies: Vec::new(),
            });
        }
        let error = validate_publish_plan(&too_many_publish_packages).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("publish plan has too many packages")
        );

        let mut unsafe_crate_reason = test_crate_package_report();
        unsafe_crate_reason.packages[0].reason = Some("hidden\u{202e}reason".to_string());
        let error = validate_crate_package_report(&unsafe_crate_reason).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut duplicate_crate_dependency = test_crate_package_report();
        duplicate_crate_dependency.packages[1]
            .internal_dependencies
            .push("vesty-params".to_string());
        let error = validate_crate_package_report(&duplicate_crate_dependency).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate internal dependency `vesty-params`")
        );

        let mut too_many_crate_packages = test_crate_package_report();
        while too_many_crate_packages.packages.len() <= CRATE_PACKAGE_MAX_PACKAGES {
            let index = too_many_crate_packages.packages.len() + 1;
            too_many_crate_packages.packages.push(CratePackageEntry {
                name: format!("vesty-extra-{index}"),
                version: "0.1.0".to_string(),
                manifest_path: format!("/workspace/crates/vesty-extra-{index}/Cargo.toml"),
                publish_order: index,
                internal_dependencies: Vec::new(),
                status: "packaged".to_string(),
                reason: None,
            });
        }
        let error = validate_crate_package_report(&too_many_crate_packages).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package report has too many packages")
        );

        let sanitized = truncate_for_report("line one\nline two\u{202e}hidden", 128);
        assert_eq!(sanitized, "line one line two hidden");

        let mut control_npm_filename = test_npm_pack_report();
        control_npm_filename[0].filename = "vesty-plugin-ui\n0.1.0.tgz".to_string();
        let error = validate_npm_pack_entries(&control_npm_filename).unwrap_err();
        assert!(error.to_string().contains(
            "npm package `vesty-plugin-ui` filename must not contain control characters"
        ));

        let mut duplicate_npm_file = test_npm_pack_report();
        duplicate_npm_file[0].files.push(NpmPackFile {
            path: "package.json".to_string(),
        });
        let error = validate_npm_pack_entries(&duplicate_npm_file).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate packed path `package.json`")
        );

        let mut too_many_npm_files = test_npm_pack_report();
        while too_many_npm_files[0].files.len() <= NPM_PACK_MAX_FILES_PER_PACKAGE {
            let index = too_many_npm_files[0].files.len();
            too_many_npm_files[0].files.push(NpmPackFile {
                path: format!("dist/extra-{index}.js"),
            });
        }
        let error = validate_npm_pack_entries(&too_many_npm_files).unwrap_err();
        assert!(error.to_string().contains("has too many packed files"));
    }

    #[test]
    fn npm_pack_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("npm-pack.json");
        write_npm_pack_artifact(&valid);

        run_npm_pack(&root, Some(&valid), true, "text").unwrap();
        run_npm_pack(&root, Some(&valid), true, "json").unwrap();

        let error = run_npm_pack(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));

        let missing_workspace = root.join("missing-workspace");
        let error = run_npm_pack(
            &missing_workspace,
            Some(&root.join("out.json")),
            false,
            "text",
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("npm workspace directory does not exist")
        );
    }

    #[test]
    fn dependency_baseline_report_validates_current_workspace() {
        let report = dependency_baseline_report(&workspace_root()).unwrap();

        validate_dependency_baseline_report(&report).unwrap();
        assert_eq!(report.status, "ok");
        assert!(report.checks.iter().any(|check| {
            check.name == "cargo workspace external dependency baseline coverage"
                && check.status == "ok"
                && check
                    .actual
                    .as_deref()
                    .is_some_and(|actual| actual.contains("arc-swap") && actual.contains("tracing"))
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "cargo workspace dependency `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.55.1")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "Steinberg VST3 SDK baseline"
                && check.expected == "v3.8.0_build_66"
                && check.actual.as_deref() == Some("v3.8.0_build_66")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm lockfile `typescript` installed version"
                && check.expected == "7.0.2"
                && check.actual.as_deref() == Some("7.0.2")
        }));
    }

    #[test]
    fn dependency_baseline_coverage_rejects_unreviewed_external_workspace_dependency() {
        let manifest = toml::from_str::<toml::Value>(
            r#"
[workspace.dependencies]
arc-swap = "1.9.1"
vesty-core = { path = "crates/vesty-core", version = "0.1.0" }
unreviewed = "9.9.9"
"#,
        )
        .unwrap();

        let check = workspace_dependency_baseline_coverage_check(&manifest);

        assert_eq!(check.status, "failed");
        assert!(check.expected.contains("arc-swap"));
        assert!(!check.expected.contains("unreviewed"));
        assert!(check.actual.as_deref().unwrap().contains("unreviewed"));
        assert!(
            check
                .hint
                .as_deref()
                .unwrap()
                .contains("REQUIRED_RUST_BASELINE_DEPENDENCIES")
        );
    }

    #[derive(Default)]
    struct FakeLatestDependencyFetcher {
        crate_versions: BTreeMap<String, Result<String, String>>,
        npm_versions: BTreeMap<String, Result<String, String>>,
    }

    impl FakeLatestDependencyFetcher {
        fn current() -> Self {
            let mut fetcher = Self::default();
            for (name, version) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
                let version = rust_registry_latest_expected(name, version);
                fetcher
                    .crate_versions
                    .insert((*name).to_string(), Ok(version));
            }
            fetcher.npm_versions.insert(
                "typescript".to_string(),
                Ok(TYPESCRIPT_BASELINE_LOCK_VERSION.to_string()),
            );
            for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
                fetcher.npm_versions.insert(
                    dependency.dependency.to_string(),
                    Ok(dependency.expected_lock_version.to_string()),
                );
            }
            fetcher
        }
    }

    impl LatestDependencyFetcher for FakeLatestDependencyFetcher {
        fn latest_crate_version(&self, name: &str) -> Result<String, String> {
            self.crate_versions
                .get(name)
                .cloned()
                .unwrap_or_else(|| Err(format!("missing fake crate version for {name}")))
        }

        fn latest_npm_version(&self, name: &str) -> Result<String, String> {
            self.npm_versions
                .get(name)
                .cloned()
                .unwrap_or_else(|| Err(format!("missing fake npm version for {name}")))
        }
    }

    #[test]
    fn dependency_baseline_latest_report_validates_registry_versions() {
        let report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();

        validate_dependency_baseline_report(&report).unwrap();
        assert_eq!(report.status, "ok");
        assert!(report.checks.iter().any(|check| {
            check.name == "crates.io latest `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.55.1")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `typescript`"
                && check.expected == "7.0.2"
                && check.actual.as_deref() == Some("7.0.2")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `react`"
                && check.expected == "19.2.7"
                && check.actual.as_deref() == Some("19.2.7")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `@types/react`"
                && check.expected == "19.2.17"
                && check.actual.as_deref() == Some("19.2.17")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `vue`"
                && check.expected == "3.5.39"
                && check.actual.as_deref() == Some("3.5.39")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `svelte`"
                && check.expected == "5.56.5"
                && check.actual.as_deref() == Some("5.56.5")
        }));

        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline-latest.json"))
            .unwrap();
        write_dependency_baseline_report(&path, &report).unwrap();
        let evidence = validate_dependency_baseline_latest_report(&path).unwrap();
        assert_eq!(
            evidence.latest_checks,
            REQUIRED_RUST_BASELINE_DEPENDENCIES.len()
                + 1
                + REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES.len()
        );
        assert_eq!(
            dependency_baseline_latest_release_check(Some(&path), true).status,
            "ok"
        );
    }

    #[test]
    fn dependency_baseline_latest_report_fails_on_registry_drift_or_query_error() {
        let mut fetcher = FakeLatestDependencyFetcher::current();
        fetcher
            .crate_versions
            .insert("wry".to_string(), Ok("0.56.0".to_string()));
        fetcher.npm_versions.insert(
            "typescript".to_string(),
            Err("registry unavailable".to_string()),
        );

        let report = dependency_baseline_report_with_latest(&workspace_root(), &fetcher).unwrap();
        assert_eq!(report.status, "failed");
        assert!(validate_dependency_baseline_report(&report).is_err());
        assert!(report.checks.iter().any(|check| {
            check.name == "crates.io latest `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.56.0")
                && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `typescript`"
                && check.actual.is_none()
                && check.status == "failed"
                && check
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("registry unavailable"))
        }));
    }

    #[test]
    fn dependency_baseline_report_rejects_inconsistent_statuses() {
        let mut forged_ok = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        let wry = forged_ok
            .checks
            .iter_mut()
            .find(|check| check.name == "crates.io latest `wry`")
            .unwrap();
        wry.actual = Some("0.56.0".to_string());
        wry.status = "ok".to_string();

        let error = validate_dependency_baseline_report(&forged_ok).unwrap_err();
        assert!(error.to_string().contains("status ok is inconsistent"));

        let mut forged_failed = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        forged_failed.checks[0].status = "failed".to_string();

        let error = validate_dependency_baseline_report(&forged_failed).unwrap_err();
        assert!(error.to_string().contains("status failed is inconsistent"));

        let mut unknown_status = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        unknown_status.checks[0].status = "partial".to_string();

        let error = validate_dependency_baseline_report(&unknown_status).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported dependency baseline check status")
        );

        let mut inconsistent_top_level = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        inconsistent_top_level.status = "failed".to_string();

        let error = validate_dependency_baseline_report(&inconsistent_top_level).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("report status `failed` is inconsistent")
        );
    }

    #[test]
    fn dependency_baseline_report_rejects_malformed_shape_fields() {
        let latest_report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        let baseline_report = dependency_baseline_report(&workspace_root()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&latest_report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error =
            serde_json::from_value::<DependencyBaselineReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&latest_report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error =
            serde_json::from_value::<DependencyBaselineReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = latest_report.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        let error = validate_dependency_baseline_report(&duplicate).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate dependency baseline check")
        );

        let mut unknown = latest_report.clone();
        unknown.checks.push(DependencyBaselineCheck {
            name: "manual extra dependency check".to_string(),
            kind: "manual".to_string(),
            path: "Cargo.toml".to_string(),
            expected: "ok".to_string(),
            actual: Some("ok".to_string()),
            status: "ok".to_string(),
            hint: None,
        });
        let error = validate_dependency_baseline_report(&unknown).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown dependency baseline check(s)")
        );
        assert!(
            error
                .to_string()
                .contains("manual:manual extra dependency check")
        );

        let mut missing = baseline_report.clone();
        missing
            .checks
            .retain(|check| check.name != "cargo workspace dependency `wry`");
        let error = validate_dependency_baseline_report(&missing).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("dependency baseline report missing required check(s)")
        );
        assert!(
            error
                .to_string()
                .contains("cargo:cargo workspace dependency `wry`")
        );

        let mut control_path = latest_report.clone();
        control_path.checks[0].path = "Cargo.toml\nforged".to_string();
        let error = validate_dependency_baseline_report(&control_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("path must not contain control characters")
        );

        let mut multiline_hint = latest_report.clone();
        multiline_hint.checks[0].hint = Some("line one\nline two\tok".to_string());
        validate_dependency_baseline_report(&multiline_hint).unwrap();

        let mut hint_nul = multiline_hint.clone();
        hint_nul.checks[0].hint = Some("line one\0line two".to_string());
        let error = validate_dependency_baseline_report(&hint_nul).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("hint must not contain control characters")
        );

        let mut hint_unsafe = multiline_hint.clone();
        hint_unsafe.checks[0].hint = Some("reviewed\u{202e}hidden".to_string());
        let error = validate_dependency_baseline_report(&hint_unsafe).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut hint_too_large = multiline_hint.clone();
        hint_too_large.checks[0].hint = Some("x".repeat(DEPENDENCY_BASELINE_HINT_MAX_BYTES + 1));
        let error = validate_dependency_baseline_report(&hint_too_large).unwrap_err();
        assert!(error.to_string().contains("hint must be at most"));

        let mut too_many = latest_report;
        while too_many.checks.len() <= DEPENDENCY_BASELINE_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(DependencyBaselineCheck {
                name: format!("extra dependency baseline check {index}"),
                kind: "extra-baseline".to_string(),
                path: format!("extra/{index}.toml"),
                expected: "0.1.0".to_string(),
                actual: Some("0.1.0".to_string()),
                status: "ok".to_string(),
                hint: None,
            });
        }
        let error = validate_dependency_baseline_report(&too_many).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("dependency baseline report has too many checks")
        );
    }

    #[test]
    fn dependency_latest_release_check_rejects_offline_baseline_report() {
        let temp = tempfile::tempdir().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline.json")).unwrap();
        let report = dependency_baseline_report(&workspace_root()).unwrap();
        write_dependency_baseline_report(&path, &report).unwrap();

        let check = dependency_baseline_latest_release_check(Some(&path), true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing latest registry checks"));
    }

    #[test]
    fn dependency_latest_release_check_requires_workspace_baseline_coverage() {
        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline-latest.json"))
            .unwrap();
        let mut report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        report
            .checks
            .retain(|check| check.name != DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME);
        fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = dependency_baseline_latest_release_check(Some(&path), true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing required check(s)"));
        assert!(
            check
                .value
                .contains(DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME)
        );
    }

    #[test]
    fn parses_exact_cargo_search_latest_version_line() {
        let output = r#"
wry = "0.55.1"    # Cross-platform WebView rendering library
wry-webkit = "0.1.0"
"#;
        assert_eq!(
            parse_cargo_search_latest_version("wry", output).as_deref(),
            Some("0.55.1")
        );
        assert!(parse_cargo_search_latest_version("missing", output).is_none());
    }

    #[test]
    fn parses_crates_io_latest_version_response() {
        let stable = r#"{"crate":{"max_stable_version":"0.55.1","max_version":"0.56.0-beta.1"}}"#;
        assert_eq!(
            parse_crates_io_latest_version(stable).as_deref(),
            Some("0.55.1")
        );

        let prerelease_only = r#"{"crate":{"max_stable_version":null,"max_version":"1.0.0-rc.1"}}"#;
        assert_eq!(
            parse_crates_io_latest_version(prerelease_only).as_deref(),
            Some("1.0.0-rc.1")
        );
        assert!(parse_crates_io_latest_version(r#"{"crate":{}}"#).is_none());
        assert!(parse_crates_io_latest_version("not json").is_none());
    }

    #[test]
    fn parses_cargo_info_version_line_for_registry_fallback() {
        let output = r#"
ts-rs #typescript #ts #bindings #ts-rs #wasm
generate typescript bindings from rust types
version: 12.0.1
license: MIT
"#;
        assert_eq!(parse_cargo_info_version(output).as_deref(), Some("12.0.1"));
        assert!(parse_cargo_info_version("license: MIT").is_none());
    }

    #[test]
    fn dependency_baseline_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("dependency-baseline.json");
        let workspace = workspace_root();

        run_dependency_baseline(&workspace, Some(&report_path), false, false, "text").unwrap();
        run_dependency_baseline(&workspace, Some(&report_path), true, false, "json").unwrap();

        let error = run_dependency_baseline(&workspace, None, true, false, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));

        let mut report = read_dependency_baseline_report(&report_path).unwrap();
        report.checks.push(DependencyBaselineCheck {
            name: "extra stale check".to_string(),
            kind: "test".to_string(),
            path: "Cargo.toml".to_string(),
            expected: "ok".to_string(),
            actual: Some("ok".to_string()),
            status: "ok".to_string(),
            hint: None,
        });
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        let error = run_dependency_baseline(&workspace, Some(&report_path), true, false, "text")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown dependency baseline check")
        );
    }

    #[test]
    fn ci_release_check_artifacts_validate_local_invariants_across_os() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_ignore_action_plan_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let plan = ReleaseActionPlan {
            version: 1,
            status: "failed".to_string(),
            summary: ReleaseActionPlanSummary {
                ok: 1,
                failed: 1,
                skipped: 0,
                action_count: 1,
            },
            protocol_snapshot: "target/vesty-protocol".to_string(),
            evidence_root: None,
            release_evidence_dir: None,
            actions: vec![ReleaseActionItem {
                check: "daw smoke matrix".to_string(),
                status: "failed".to_string(),
                priority: "required".to_string(),
                value: "missing".to_string(),
                hint: None,
                evidence_path: None,
                commands: vec!["vesty daw-matrix --strict".to_string()],
            }],
        };
        fs::write(
            root.join("release-action-plan-Linux.json"),
            serde_json::to_string_pretty(&plan).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
    }

    #[test]
    fn ci_release_check_artifacts_accept_case_insensitive_report_filenames() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Release-Check-Linux.JSON"));
        write_ci_release_check_artifact(&root.join("RELEASE-CHECK-macOS.Json"));
        write_ci_release_check_artifact(&root.join("release-check-WINDOWS.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_infer_os_from_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Linux/release-check.json"));
        write_ci_release_check_artifact(&root.join("macOS/release-check.json"));
        write_ci_release_check_artifact(&root.join("Windows/release-check.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_infer_os_from_path_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Linux/release-check.json"));
        write_ci_release_check_artifact(&root.join("macOS/release-check.json"));
        write_ci_release_check_artifact(&root.join("swing-state/release-check.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: Windows"));
        assert!(
            check
                .value
                .contains("swing-state/release-check.json: could not infer OS from artifact path")
        );
        assert!(!check.value.contains("duplicate OS reports: Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_os_label_mismatch_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let linux = root.join("release-check-Linux.json");
        let mut report: ReleaseCheckReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.os = Some("Windows".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("path indicates Linux"));
        assert!(check.value.contains("report os is Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_allow_legacy_reports_without_os_label() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        for os in ["Linux", "macOS", "Windows"] {
            let path = root.join(format!("release-check-{os}.json"));
            write_ci_release_check_artifact(&path);
            let mut report: ReleaseCheckReport =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            report.os = None;
            fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        }

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_release_check_artifacts_preserve_crate_package_readiness_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == "crate package readiness")
            .expect("crate package readiness check");
        check.status = "failed".to_string();
        check.value = "crate package readiness failed: vesty-core cargo package failed".to_string();
        check.hint = Some("inspect vesty crate-package output".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_preserve_platform_smoke_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == "platform smoke artifacts")
            .expect("platform smoke artifacts check");
        check.status = "failed".to_string();
        check.value = "required evidence missing".to_string();
        check.hint = Some("collect macOS, Windows x64 and Linux X11 smoke reports".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_preserve_vst3_sdk_audit_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        for name in [
            "vst3 SDK header manifest",
            "vst3 SDK generated bindings plan",
            "vst3 SDK generated bindings surface",
            "vst3 SDK generated bindings scaffold",
            "vst3 SDK generated bindings ABI seed",
            "vst3 SDK generated bindings ABI layout",
            "vst3 SDK generated bindings interface skeleton",
        ] {
            let check = report
                .checks
                .iter_mut()
                .find(|check| check.name == name)
                .expect("VST3 SDK audit check");
            check.status = "failed".to_string();
            check.value = format!("{name}: optional audit artifact is invalid");
            check.hint = Some("regenerate optional VST3 SDK audit artifact".to_string());
        }
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_missing_os_and_local_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "vst3 binding baseline")
            .unwrap()
            .status = "failed".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("local invariant check"));
        assert!(check.value.contains("vst3 binding baseline"));

        fs::remove_file(root.join("release-check-macOS.json")).unwrap();
        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_inconsistent_or_unknown_statuses() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let mut ok_with_failure = test_ci_release_check_report();
        ok_with_failure.status = "ok".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&ok_with_failure).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("report status `ok` is inconsistent with failed checks")
        );

        let mut unknown_check_status = test_ci_release_check_report();
        unknown_check_status.checks[0].status = "pending".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&unknown_check_status).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unexpected check status"));
        assert!(check.value.contains("host profiles=pending"));

        let mut control_value = test_ci_release_check_report();
        control_value.checks[0].value = "5 release host profile(s) covered\nbad".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&control_value).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("value must not contain control characters")
        );

        let mut long_hint = test_ci_release_check_report();
        long_hint.checks[0].hint = Some("x".repeat(RELEASE_ACTION_TEXT_MAX_BYTES + 1));
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&long_hint).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("hint must be at most"));

        let mut invalid_run_url = test_ci_release_check_report();
        invalid_run_url.ci_run_url =
            Some("https://github.com/vesty-rs/vesty/actions/runs/not-a-number".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&invalid_run_url).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("invalid ci_run_url"));

        let mut control_run_url = test_ci_release_check_report();
        control_run_url.ci_run_url =
            Some("https://github.com/vesty-rs/vesty/actions/runs/123\nbad".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&control_run_url).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("ci run url must not contain control characters")
        );

        let mut empty_checks = test_ci_release_check_report();
        empty_checks.status = "ok".to_string();
        empty_checks.checks.clear();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&empty_checks).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("must contain at least one check"));

        let mut too_many_checks = test_ci_release_check_report();
        while too_many_checks.checks.len() <= RELEASE_CHECK_MAX_CHECKS {
            let index = too_many_checks.checks.len();
            too_many_checks.checks.push(ReleaseCheckItem {
                name: format!("extra skipped check {index}"),
                status: "skipped".to_string(),
                value: "not requested".to_string(),
                hint: None,
            });
        }
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&too_many_checks).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("too many checks"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_duplicate_or_forged_invariant_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let mut duplicate = test_ci_release_check_report();
        duplicate.checks.push(duplicate.checks[0].clone());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&duplicate).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate check name"));
        assert!(check.value.contains("host profiles"));

        let mut forged_host_profiles = test_ci_release_check_report();
        forged_host_profiles
            .checks
            .iter_mut()
            .find(|check| check.name == "host profiles")
            .unwrap()
            .value = "1 release host profile covered".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_host_profiles).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("host profiles"));
        assert!(check.value.contains("inconsistent with daw_matrix"));

        let mut forged_binding_baseline = test_ci_release_check_report();
        forged_binding_baseline
            .checks
            .iter_mut()
            .find(|check| check.name == "vst3 binding baseline")
            .unwrap()
            .value = "Steinberg SDK v3.8.0_build_66".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_binding_baseline).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("vst3 binding baseline"));
        assert!(check.value.contains("does not match current baseline"));

        let mut forged_protocol = test_ci_release_check_report();
        forged_protocol
            .checks
            .iter_mut()
            .find(|check| check.name == "protocol snapshot")
            .unwrap()
            .value = "skipped in CI".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_protocol).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("protocol snapshot"));
        assert!(check.value.contains("unexpected value"));
    }
