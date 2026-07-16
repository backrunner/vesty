    #[test]
    fn release_evidence_dir_discovers_validate_reports_by_content() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-package")).unwrap();
        fs::create_dir_all(&root).unwrap();
        let gain_static = root.join("VestyGain.validate.json");
        let web_static = root.join("VestyWebUIDemo.validate.json");
        let synth_release = root.join("VestyMIDISynth.validator.json");
        write_validate_artifact(&gain_static, "ok", "skipped");
        write_validate_artifact(&web_static, "ok", "skipped");
        write_validate_artifact(&synth_release, "ok", "passed");
        fs::write(
            root.join("static-validate-release-check.json"),
            r#"{"status":"failed","checks":[]}"#,
        )
        .unwrap();

        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.validate_reports, vec![synth_release]);
        assert_eq!(
            options.static_validate_reports,
            vec![gain_static, web_static]
        );

        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);
        assert!(release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 validate reports"
                && check.status == "ok"
                && check.value.contains("1 validate report")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 static validate reports"
                && check.status == "ok"
                && check.value.contains("2 static validate report")
        }));
    }

    #[test]
    fn release_evidence_dir_discovers_nested_signing_and_notary_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        write_release_evidence_templates(&root).unwrap();

        let signing_dir = root.join("downloaded-artifacts/signing");
        fs::create_dir_all(&signing_dir).unwrap();
        let codesign_log = signing_dir.join("macos-codesign-output.log");
        let signtool_log = signing_dir.join("windows-signtool-output.txt");
        fs::write(
            &codesign_log,
            "VestyGain.vst3: valid on disk\nVestyGain.vst3: satisfies its designated requirement\n",
        )
        .unwrap();
        fs::write(&signtool_log, "Successfully verified: VestyGain.vst3\n").unwrap();

        let signed_bundle = root.join("downloaded-artifacts/macos/VestyGain.vst3");
        fs::create_dir_all(signed_bundle.join("Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&signed_bundle);

        let notary_dir = root.join("downloaded-artifacts/notary");
        fs::create_dir_all(&notary_dir).unwrap();
        let notary_log = notary_dir.join("notary-output.log");
        fs::write(
            &notary_log,
            "id: test\nstatus: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();

        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert!(options.signed_bundle_evidence.contains(&codesign_log));
        assert!(options.signed_bundle_evidence.contains(&signtool_log));
        assert!(options.signed_bundle_evidence.contains(&signed_bundle));
        assert!(
            !options
                .signed_bundle_evidence
                .contains(&root.join("signing-macos.log"))
        );
        assert!(
            !options
                .signed_bundle_evidence
                .contains(&root.join("signing-windows.log"))
        );
        assert_eq!(options.notarization_log, Some(notary_log));

        let signing_check =
            signed_bundle_evidence_release_check(&options.signed_bundle_evidence, true);
        assert_eq!(signing_check.status, "ok");
        let notary_check =
            notarization_log_release_check(options.notarization_log.as_deref(), true);
        assert_eq!(notary_check.status, "ok");
    }

    #[test]
    fn release_check_rejects_static_only_validate_report_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let validate_report = root.join("validate-static-only.json");
        write_validate_artifact(&validate_report, "ok", "skipped");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            validate_reports: vec![validate_report],
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 validate reports"
                && check.status == "failed"
                && check.value.contains("validator status is skipped")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn release_check_rejects_symlink_validate_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-validator.json");
        let validate_report = root.join("validate-report.json");
        write_validate_artifact(&external, "ok", "passed");
        unix_fs::symlink(&external, &validate_report).unwrap();

        let check = validate_reports_release_check(&[validate_report], true);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("validate report must not be a symlink")
        );
    }

    #[test]
    fn release_check_rejects_validator_passed_report_without_exit_or_test_counts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let missing_exit = root.join("validator-missing-exit.json");
        let missing_counts = root.join("validator-missing-counts.json");
        write_validate_artifact(&missing_exit, "ok", "passed");
        write_validate_artifact(&missing_counts, "ok", "passed");

        let mut report = read_validate_report(&missing_exit).unwrap();
        report.validator.exit_code = None;
        fs::write(
            &missing_exit,
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let mut report = read_validate_report(&missing_counts).unwrap();
        report.validator.tests_passed = None;
        report.validator.tests_failed = None;
        fs::write(
            &missing_counts,
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = validate_reports_release_check(&[missing_exit, missing_counts], true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("validator exit code is missing"));
        assert!(
            check
                .value
                .contains("validator passed test count is missing")
        );
    }

    #[test]
    fn example_validate_coverage_accepts_partial_when_not_required() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let gain = root.join("VestyGain.validator.json");
        write_example_validate_artifact(&gain, "VestyGain.vst3");

        let check = example_validate_coverage_release_check(&[gain], false);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("1 of 3 example validator report"));
        assert!(
            check
                .hint
                .as_deref()
                .unwrap()
                .contains("VestyWebUIDemo.vst3")
        );
    }

    #[test]
    fn example_validate_coverage_requires_all_release_platforms() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let gain = root.join("VestyGain.validator.json");
        write_example_validate_artifact(&gain, "VestyGain.vst3");

        let partial = example_validate_coverage_release_check(&[gain], true);
        assert_eq!(partial.status, "failed");
        assert!(partial.value.contains("VestyWebUIDemo.vst3"));
        assert!(partial.value.contains("VestyMIDISynth.vst3"));
        assert!(partial.value.contains("VestyGain.vst3@linux-x64"));
        assert!(partial.value.contains("VestyGain.vst3@windows-x64"));
        assert!(
            partial
                .hint
                .as_deref()
                .is_some_and(|hint| { hint.contains("vesty validate --strict --report <path>") })
        );

        let full = write_example_validate_matrix(&root.join("full"));
        let complete = example_validate_coverage_release_check(&full, true);
        assert_eq!(complete.status, "ok");
        assert!(
            complete
                .value
                .contains("9 example/platform validator report")
        );
        assert!(complete.value.contains("full release coverage"));
    }

    #[test]
    fn release_check_accepts_example_validator_reports_with_sidecar_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let validate_reports = write_example_validate_reports(&root.join("validator"));
        let static_validate_reports =
            write_example_static_validate_platform(&root.join("static"), "macos");
        let options = ReleaseEvidenceOptions {
            validate_reports,
            static_validate_reports,
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage"
                && check.status == "ok"
                && check.value.contains("3 of 3 example validator report")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage"
                && check.status == "ok"
                && check.value.contains("platforms: macos")
        }));
    }

    #[test]
    fn release_check_requires_example_validator_reports_for_all_release_platforms() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let macos_only = write_example_validate_reports(&root.join("validator-macos"));
        let options = ReleaseEvidenceOptions {
            validate_reports: macos_only,
            static_validate_reports: write_example_static_validate_matrix(&root.join("static")),
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let report =
            build_release_check_report(rows.clone(), Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage"
                && check.status == "failed"
                && check.value.contains("VestyGain.vst3@linux-x64")
                && check.value.contains("VestyGain.vst3@windows-x64")
        }));

        let options = ReleaseEvidenceOptions {
            validate_reports: write_example_validate_matrix(&root.join("validator-full")),
            static_validate_reports: write_example_static_validate_matrix(
                &root.join("static-full"),
            ),
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let complete = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(complete.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage"
                && check.status == "ok"
                && check.value.contains("9 example/platform validator report")
        }));
    }

    #[test]
    fn example_validator_coverage_rejects_multi_platform_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.macos+windows-x64.validate.json");
        write_validate_artifact_with_bundle_and_binaries(
            &report_path,
            "VestyGain.vst3",
            "ok",
            "passed",
            vec![
                "VestyGain.vst3/Contents/MacOS/VestyGain",
                "VestyGain.vst3/Contents/x86_64-win/VestyGain.vst3",
            ],
        );

        let check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);

        assert_eq!(check.status, "failed");
        assert!(
            check.value.contains(
                "validator-passed example report must contain exactly one release platform"
            )
        );
        assert!(check.value.contains("macos"));
        assert!(check.value.contains("windows-x64"));
    }

    #[test]
    fn example_static_coverage_rejects_multi_platform_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyMIDISynth.mixed.static-validate.json");
        write_validate_artifact_with_bundle_and_binaries(
            &report_path,
            "VestyMIDISynth.vst3",
            "ok",
            "skipped",
            vec![
                "VestyMIDISynth.vst3/Contents/MacOS/VestyMIDISynth",
                "VestyMIDISynth.vst3/Contents/x86_64-linux/VestyMIDISynth.so",
            ],
        );

        let check = example_static_validate_coverage_release_check(
            std::slice::from_ref(&report_path),
            false,
        );

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("static example report must contain exactly one release platform")
        );
        assert!(check.value.contains("linux-x64"));
        assert!(check.value.contains("macos"));
    }

    #[test]
    fn example_coverage_rejects_report_file_name_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.windows-x64.validate.json");
        write_example_validate_artifact_for_platform(&report_path, "VestyGain.vst3", "macos");

        let check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("file name indicates windows-x64, but static binaries indicate macos")
        );
    }

    #[test]
    fn example_validator_coverage_rejects_report_file_name_bundle_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.macos.validate.json");
        write_example_validate_artifact_for_platform(&report_path, "VestyMIDISynth.vst3", "macos");

        let check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains(
            "file name indicates VestyGain.vst3, but report bundle is VestyMIDISynth.vst3"
        ));
    }

    #[test]
    fn example_static_coverage_rejects_report_file_name_bundle_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.macos.static-validate.json");
        write_example_static_validate_artifact(&report_path, "VestyMIDISynth.vst3", "macos");

        let check = example_static_validate_coverage_release_check(
            std::slice::from_ref(&report_path),
            false,
        );

        assert_eq!(check.status, "failed");
        assert!(check.value.contains(
            "file name indicates VestyGain.vst3, but report bundle is VestyMIDISynth.vst3"
        ));
    }

    #[test]
    fn example_coverage_platform_file_name_match_uses_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let fuzzy = root.join("VestyGain.mywindowsx64note.validate.json");
        write_example_validate_artifact_for_platform(&fuzzy, "VestyGain.vst3", "macos");

        let fuzzy_check =
            example_validate_coverage_release_check(std::slice::from_ref(&fuzzy), false);

        assert_eq!(fuzzy_check.status, "ok");
        assert!(fuzzy_check.value.contains("platforms: macos"));

        let labeled = root.join("VestyGain.windows_x64.validate.json");
        write_example_validate_artifact_for_platform(&labeled, "VestyGain.vst3", "macos");

        let labeled_check =
            example_validate_coverage_release_check(std::slice::from_ref(&labeled), false);

        assert_eq!(labeled_check.status, "failed");
        assert!(
            labeled_check
                .value
                .contains("file name indicates windows-x64, but static binaries indicate macos")
        );
    }

    #[test]
    fn example_coverage_bundle_file_name_match_uses_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let fuzzy = root.join("MyVestyGainNote.macos.validate.json");
        write_example_validate_artifact_for_platform(&fuzzy, "VestyMIDISynth.vst3", "macos");

        let fuzzy_check =
            example_validate_coverage_release_check(std::slice::from_ref(&fuzzy), false);

        assert_eq!(fuzzy_check.status, "ok");
        assert!(fuzzy_check.value.contains("VestyMIDISynth.vst3"));

        let labeled = root.join("VestyGain.VestyMIDISynth.macos.validate.json");
        write_example_validate_artifact_for_platform(&labeled, "VestyMIDISynth.vst3", "macos");

        let labeled_check =
            example_validate_coverage_release_check(std::slice::from_ref(&labeled), false);

        assert_eq!(labeled_check.status, "failed");
        assert!(
            labeled_check
                .value
                .contains("multiple example bundle labels")
        );

        let dashed = root.join("Vesty-Gain.macos.validate.json");
        write_example_validate_artifact_for_platform(&dashed, "VestyMIDISynth.vst3", "macos");

        let dashed_check =
            example_validate_coverage_release_check(std::slice::from_ref(&dashed), false);

        assert_eq!(dashed_check.status, "failed");
        assert!(dashed_check.value.contains(
            "file name indicates VestyGain.vst3, but report bundle is VestyMIDISynth.vst3"
        ));

        let underscored = root.join("Vesty_Gain.macos.static-validate.json");
        write_example_static_validate_artifact(&underscored, "VestyMIDISynth.vst3", "macos");

        let underscored_check = example_static_validate_coverage_release_check(
            std::slice::from_ref(&underscored),
            false,
        );

        assert_eq!(underscored_check.status, "failed");
        assert!(underscored_check.value.contains(
            "file name indicates VestyGain.vst3, but report bundle is VestyMIDISynth.vst3"
        ));
    }

    #[test]
    fn example_validator_coverage_rejects_duplicate_bundle_platform_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let first = root.join("VestyGain.macos.validate.json");
        let second = root.join("copy/VestyGain.macos.validate.json");
        write_example_validate_artifact_for_platform(&first, "VestyGain.vst3", "macos");
        write_example_validate_artifact_for_platform(&second, "VestyGain.vst3", "macos");

        let check = example_validate_coverage_release_check(&[first, second], false);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("duplicate validator-passed example report for VestyGain.vst3@macos")
        );
    }

    #[test]
    fn example_static_coverage_rejects_duplicate_bundle_platform_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let first = root.join("VestyWebUIDemo.windows-x64.static-validate.json");
        let second = root.join("copy/VestyWebUIDemo.windows-x64.static-validate.json");
        write_example_static_validate_artifact(&first, "VestyWebUIDemo.vst3", "windows-x64");
        write_example_static_validate_artifact(&second, "VestyWebUIDemo.vst3", "windows-x64");

        let check = example_static_validate_coverage_release_check(&[first, second], false);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("duplicate static example report for VestyWebUIDemo.vst3@windows-x64")
        );
    }

    #[test]
    fn release_check_requires_binary_export_evidence_for_strict_example_matrix() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let mut validate_reports = write_example_validate_matrix(&root.join("validator-full"));
        let static_validate_reports =
            write_example_static_validate_matrix(&root.join("static-full"));
        let tampered = validate_reports
            .iter()
            .find(|path| path.as_str().contains("VestyGain.vst3.linux-x64"))
            .cloned()
            .expect("fixture should include linux gain validator report");
        let mut report = read_validate_report(&tampered).unwrap();
        report.static_check.binary_exports.clear();
        fs::write(&tampered, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        validate_reports.sort();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            validate_reports,
            static_validate_reports,
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage"
                && check.status == "failed"
                && check
                    .value
                    .contains("missing binary export evidence in static_check.binary_exports")
        }));
    }

    #[test]
    fn release_check_rejects_skipped_binary_export_evidence_for_strict_example_matrix() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let validate_reports = write_example_validate_matrix(&root.join("validator-full"));
        let static_validate_reports =
            write_example_static_validate_matrix(&root.join("static-full"));
        let tampered = static_validate_reports
            .iter()
            .find(|path| path.as_str().contains("VestyWebUIDemo.vst3.windows-x64"))
            .cloned()
            .expect("fixture should include windows web-ui static report");
        let mut report = read_validate_report(&tampered).unwrap();
        for check in &mut report.static_check.binary_exports {
            if check.platform == "windows-x64" {
                check.status = "skipped".to_string();
                check.tool = None;
                check.found_symbols.clear();
                check.missing_symbols.clear();
                check.error = Some("llvm-objdump and dumpbin unavailable".to_string());
            }
        }
        validate_static_validate_report(&report).unwrap();
        fs::write(&tampered, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            validate_reports,
            static_validate_reports,
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage"
                && check.status == "failed"
                && check
                    .value
                    .contains("must include ok binary export evidence")
                && check.value.contains("status=skipped")
        }));
    }

    #[test]
    fn example_coverage_rejects_web_ui_report_without_asset_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyWebUIDemo.no-assets.json");
        write_validate_artifact_with_bundle_and_binaries_and_assets(
            &report_path,
            REQUIRED_WEB_UI_EXAMPLE_BUNDLE,
            "ok",
            "passed",
            vec!["VestyWebUIDemo.vst3/Contents/MacOS/VestyWebUIDemo"],
            None,
            0,
        );

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(validator_check.value.contains("missing UI asset manifest"));
        assert!(
            validator_check
                .value
                .contains("static_check.asset_manifest: <missing>")
        );
        assert!(validator_check.value.contains("asset_count: 0"));

        let static_report_path = root.join("VestyWebUIDemo.no-assets.static-validate.json");
        write_validate_artifact_with_bundle_and_binaries_and_assets(
            &static_report_path,
            REQUIRED_WEB_UI_EXAMPLE_BUNDLE,
            "ok",
            "skipped",
            vec!["VestyWebUIDemo.vst3/Contents/MacOS/VestyWebUIDemo"],
            None,
            0,
        );
        let static_check = example_static_validate_coverage_release_check(
            std::slice::from_ref(&static_report_path),
            false,
        );
        assert_eq!(static_check.status, "failed");
        assert!(static_check.value.contains("missing UI asset manifest"));
    }

    #[test]
    fn example_coverage_rejects_web_ui_report_with_non_bundle_asset_manifest_path() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyWebUIDemo.bad-asset-path.json");
        write_validate_artifact_with_bundle_and_binaries_and_assets(
            &report_path,
            REQUIRED_WEB_UI_EXAMPLE_BUNDLE,
            "ok",
            "passed",
            vec!["VestyWebUIDemo.vst3/Contents/MacOS/VestyWebUIDemo"],
            Some("assets.manifest.json".to_string()),
            2,
        );

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(
            validator_check
                .value
                .contains("static_check.asset_manifest does not belong")
        );
        assert!(validator_check.value.contains("assets.manifest.json"));
    }

    #[test]
    fn example_coverage_rejects_web_ui_report_with_suffix_spoofed_asset_manifest_path() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyWebUIDemo.spoofed-asset-path.json");
        write_validate_artifact_with_bundle_and_binaries_and_assets(
            &report_path,
            REQUIRED_WEB_UI_EXAMPLE_BUNDLE,
            "ok",
            "passed",
            vec!["VestyWebUIDemo.vst3/Contents/MacOS/VestyWebUIDemo"],
            Some(
                "target/NotVestyWebUIDemo.vst3/Contents/Resources/assets.manifest.json".to_string(),
            ),
            2,
        );

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(
            validator_check
                .value
                .contains("static_check.asset_manifest does not belong")
        );
    }

    #[test]
    fn example_coverage_rejects_example_report_without_parameter_manifest_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.no-params.json");
        write_example_validate_artifact(&report_path, "VestyGain.vst3");
        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.parameter_manifest = None;
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(
            validator_check
                .value
                .contains("missing parameter manifest evidence")
        );
        assert!(
            validator_check
                .value
                .contains("static_check.parameter_manifest: <missing>")
        );

        let static_report_path = root.join("VestyGain.no-params.static-validate.json");
        write_example_static_validate_artifact(&static_report_path, "VestyGain.vst3", "macos");
        let mut static_report = read_validate_report(&static_report_path).unwrap();
        static_report.static_check.parameter_manifest = None;
        fs::write(
            &static_report_path,
            serde_json::to_string_pretty(&static_report).unwrap(),
        )
        .unwrap();
        let static_check = example_static_validate_coverage_release_check(
            std::slice::from_ref(&static_report_path),
            false,
        );
        assert_eq!(static_check.status, "failed");
        assert!(
            static_check
                .value
                .contains("missing parameter manifest evidence")
        );
    }

    #[test]
    fn example_coverage_rejects_example_report_with_non_bundle_parameter_manifest_path() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.bad-param-path.json");
        write_example_validate_artifact(&report_path, "VestyGain.vst3");
        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.parameter_manifest = Some(PARAMETER_MANIFEST_FILE.to_string());
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(
            validator_check
                .value
                .contains("static_check.parameter_manifest does not belong")
        );
        assert!(validator_check.value.contains("parameters.manifest.json"));
    }

    #[test]
    fn example_coverage_rejects_example_report_with_suffix_spoofed_parameter_manifest_path() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("VestyGain.spoofed-param-path.json");
        write_example_validate_artifact(&report_path, "VestyGain.vst3");
        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.parameter_manifest = Some(format!(
            "target/NotVestyGain.vst3/Contents/Resources/{PARAMETER_MANIFEST_FILE}"
        ));
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let validator_check =
            example_validate_coverage_release_check(std::slice::from_ref(&report_path), false);
        assert_eq!(validator_check.status, "failed");
        assert!(
            validator_check
                .value
                .contains("static_check.parameter_manifest does not belong")
        );
    }

    #[test]
    fn non_example_static_validate_report_does_not_require_parameter_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("third-party.json");
        write_validate_artifact_with_bundle_and_binaries(
            &report_path,
            "ThirdParty.vst3",
            "ok",
            "skipped",
            vec!["ThirdParty.vst3/Contents/MacOS/ThirdParty"],
        );

        let static_check = static_validate_reports_release_check(&[report_path], false);

        assert_eq!(static_check.status, "ok");
        assert!(static_check.value.contains("ThirdParty.vst3"));
    }

    #[test]
    fn release_check_accepts_static_validate_reports_as_ci_smoke_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let static_report = root.join("validate-static-only.json");
        write_validate_artifact(&static_report, "ok", "skipped");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            static_validate_reports: vec![static_report],
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 static validate reports"
                && check.status == "ok"
                && check.value.contains("Gain.vst3")
                && check.value.contains("platforms: macos")
        }));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "vst3 validate reports" && check.status == "skipped"
            })
        );
    }

    #[test]
    fn static_validate_reports_reject_validator_run_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let passed_report = root.join("validator-passed.json");
        write_validate_artifact(&passed_report, "ok", "passed");
        let failed_report = root.join("validator-failed.json");
        write_validate_artifact(&failed_report, "ok", "failed");
        let mut failed = read_validate_report(&failed_report).unwrap();
        failed.validator.path = Some("/tools/validator".to_string());
        failed.validator.exit_code = Some(1);
        failed.validator.error = Some("validator failed".to_string());
        fs::write(
            &failed_report,
            serde_json::to_string_pretty(&failed).unwrap(),
        )
        .unwrap();

        for path in [&passed_report, &failed_report] {
            let report = read_validate_report(path).unwrap();
            let error = validate_static_validate_report(&report)
                .expect_err("validator-run report should not count as static-only evidence")
                .to_string();
            assert!(
                error.contains("static validate report must be static-only"),
                "{path}: {error}"
            );
        }

        let check = static_validate_reports_release_check(&[passed_report, failed_report], false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("static validate report must be static-only")
        );
    }

    #[test]
    fn static_validate_reports_include_platform_coverage() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let macos = root.join("macos.json");
        let windows = root.join("windows.json");
        let linux = root.join("linux.json");
        write_validate_artifact_with_binaries(
            &macos,
            "ok",
            "skipped",
            vec!["Gain.vst3/Contents/MacOS/Gain"],
        );
        write_validate_artifact_with_binaries(
            &windows,
            "ok",
            "skipped",
            vec!["Gain.vst3/Contents/x86_64-win/Gain.vst3"],
        );
        write_validate_artifact_with_binaries(
            &linux,
            "ok",
            "skipped",
            vec!["Gain.vst3/Contents/x86_64-linux/Gain.so"],
        );

        let check = static_validate_reports_release_check(&[macos, windows, linux], false);

        assert_eq!(check.status, "ok");
        assert!(
            check
                .value
                .contains("platforms: linux-x64, macos, windows-x64")
        );
    }

    #[test]
    fn example_static_validate_coverage_accepts_single_platform_ci_matrix() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let paths = write_example_static_validate_platform(&root, "macos");

        let check = example_static_validate_coverage_release_check(&paths, false);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 example/platform entries"));
        assert!(check.value.contains("platforms: macos"));
    }

    #[test]
    fn example_static_validate_coverage_rejects_partial_platform_matrix() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let mut paths = Vec::new();
        for bundle in ["VestyGain.vst3", "VestyWebUIDemo.vst3"] {
            let path = root.join(format!("{bundle}.macos.json"));
            write_example_static_validate_artifact(&path, bundle, "macos");
            paths.push(path);
        }

        let check = example_static_validate_coverage_release_check(&paths, false);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("VestyMIDISynth.vst3@macos"));
        assert!(check.hint.as_deref().is_some_and(|hint| {
            hint.contains("vesty validate --static-only --strict --report <path>")
        }));
    }

    #[test]
    fn example_static_validate_coverage_requires_all_release_platforms() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let macos_only = write_example_static_validate_platform(&root.join("macos-only"), "macos");

        let partial = example_static_validate_coverage_release_check(&macos_only, true);
        assert_eq!(partial.status, "failed");
        assert!(partial.value.contains("VestyGain.vst3@linux-x64"));
        assert!(partial.value.contains("VestyGain.vst3@windows-x64"));
        assert!(partial.hint.as_deref().is_some_and(|hint| {
            hint.contains("vesty validate --static-only --strict --report <path>")
        }));

        let full = write_example_static_validate_matrix(&root.join("full"));
        let complete = example_static_validate_coverage_release_check(&full, true);
        assert_eq!(complete.status, "ok");
        assert!(complete.value.contains("9 example/platform entries"));
        assert!(complete.value.contains("full release coverage"));
    }

    #[test]
    fn release_check_rejects_failed_static_validate_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let static_report = root.join("validate-static-failed.json");
        write_validate_artifact(&static_report, "failed", "skipped");

        let check = static_validate_reports_release_check(&[static_report], false);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("static bundle check status is failed"));
    }

    #[test]
    fn release_check_report_shape_rejects_non_boolean_daw_platform_support() {
        let mut report = ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };
        report.daw_matrix[0]["platform_supported"] = serde_json::Value::String("true".to_string());

        let error = validate_release_check_report_shape(&report)
            .expect_err("platform_supported should remain a boolean")
            .to_string();

        assert!(
            error.contains("platform_supported"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("must be boolean"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn release_check_report_shape_rejects_unknown_daw_matrix_fields() {
        let mut report = ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };
        report.daw_matrix[0]["generatedBy"] =
            serde_json::Value::String("external-tool".to_string());

        let error = validate_release_check_report_shape(&report)
            .expect_err("daw_matrix rows should reject unknown fields")
            .to_string();

        assert!(error.contains("unknown field"));
        assert!(error.contains("generatedBy"));
    }

    #[test]
    fn release_check_report_shape_requires_complete_daw_matrix_fields() {
        let mut report = ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };
        report.daw_matrix[0]
            .as_object_mut()
            .expect("complete DAW matrix rows are objects")
            .remove("meter_stream");

        let error = validate_release_check_report_shape(&report)
            .expect_err("daw_matrix rows should require the full field set")
            .to_string();

        assert!(error.contains("missing required field"));
        assert!(error.contains("meter_stream"));
    }

    #[test]
    fn release_check_report_shape_rejects_non_string_daw_matrix_metadata() {
        for key in ["platform", "evidence"] {
            let mut report = ReleaseCheckReport {
                status: "failed".to_string(),
                os: None,
                ci_run_url: None,
                checks: vec![ReleaseCheckItem {
                    name: "dummy".to_string(),
                    status: "failed".to_string(),
                    value: "dummy failure".to_string(),
                    hint: None,
                }],
                daw_matrix: complete_release_rows(),
            };
            report.daw_matrix[0][key] = serde_json::Value::Bool(true);

            let error = validate_release_check_report_shape(&report)
                .expect_err("daw_matrix metadata should require string values")
                .to_string();

            assert!(error.contains(key), "{key}: {error}");
            assert!(error.contains("must be a string"), "{key}: {error}");
        }
    }

    #[test]
    fn release_check_report_shape_rejects_inconsistent_daw_platform_support() {
        let mut unsupported_true = ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };
        let ableton_index = unsupported_true
            .daw_matrix
            .iter()
            .position(|row| row["host"].as_str() == Some("Ableton Live"))
            .unwrap();
        unsupported_true.daw_matrix[ableton_index]["platform"] =
            serde_json::Value::String("Linux X11 / edited report".to_string());
        unsupported_true.daw_matrix[ableton_index]["platform_supported"] =
            serde_json::Value::Bool(true);

        let error = validate_release_check_report_shape(&unsupported_true)
            .expect_err("unsupported platform should not be marked supported")
            .to_string();
        assert!(error.contains("platform_supported=true"), "{error}");
        assert!(error.contains("Ableton Live"), "{error}");

        let mut supported_false = unsupported_true.clone();
        supported_false.daw_matrix[ableton_index]["platform"] =
            serde_json::Value::String("macOS arm64 / Ableton smoke".to_string());
        supported_false.daw_matrix[ableton_index]["platform_supported"] =
            serde_json::Value::Bool(false);

        let error = validate_release_check_report_shape(&supported_false)
            .expect_err("supported platform should not be marked unsupported")
            .to_string();
        assert!(error.contains("platform_supported=false"), "{error}");
        assert!(error.contains("Ableton Live"), "{error}");
    }

    #[test]
    fn release_check_report_shape_rejects_unknown_daw_matrix_hosts() {
        let mut report = ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };
        report.daw_matrix[0] = complete_release_row("Imaginary Host");

        let error = validate_release_check_report_shape(&report)
            .expect_err("unknown DAW matrix host should be rejected")
            .to_string();

        assert!(error.contains("host set mismatch"), "{error}");
        assert!(
            error.contains("unknown profile rows: Imaginary Host"),
            "{error}"
        );
    }

    #[test]
    fn release_check_report_shape_requires_exact_canonical_daw_matrix_hosts() {
        let base_report = || ReleaseCheckReport {
            status: "failed".to_string(),
            os: None,
            ci_run_url: None,
            checks: vec![ReleaseCheckItem {
                name: "dummy".to_string(),
                status: "failed".to_string(),
                value: "dummy failure".to_string(),
                hint: None,
            }],
            daw_matrix: complete_release_rows(),
        };

        let mut missing = base_report();
        missing.daw_matrix.pop();
        let error = validate_release_check_report_shape(&missing)
            .expect_err("missing DAW matrix row should be rejected")
            .to_string();
        assert!(error.contains("exactly"), "{error}");

        let mut duplicate = base_report();
        duplicate.daw_matrix[1] = complete_release_row("REAPER");
        let error = validate_release_check_report_shape(&duplicate)
            .expect_err("duplicate canonical DAW matrix row should be rejected")
            .to_string();
        assert!(error.contains("host set mismatch"), "{error}");
        assert!(
            error.contains("missing profile rows: Cubase/Nuendo"),
            "{error}"
        );
        assert!(error.contains("duplicate profile rows: REAPER"), "{error}");

        let mut alias = base_report();
        alias.daw_matrix[0] = complete_release_row("reaper");
        let error = validate_release_check_report_shape(&alias)
            .expect_err("alias DAW matrix host should be rejected")
            .to_string();
        assert!(error.contains("host set mismatch"), "{error}");
        assert!(
            error.contains("non-canonical profile rows: reaper -> REAPER"),
            "{error}"
        );
    }

    #[test]
    fn release_check_report_shape_requires_current_check_set() {
        let base_report = || {
            build_release_check_report(
                complete_release_rows(),
                Utf8Path::new("unused"),
                true,
                &ReleaseEvidenceOptions::default(),
            )
        };

        let mut missing = base_report();
        missing.checks.retain(|check| check.name != "ci run url");
        let error = validate_release_check_report_shape(&missing)
            .expect_err("release-check report missing a current gate should be rejected")
            .to_string();
        assert!(error.contains("check set must match"), "{error}");
        assert!(error.contains("missing check(s): ci run url"), "{error}");

        let mut extra = base_report();
        extra.checks.push(ReleaseCheckItem {
            name: "manual extra gate".to_string(),
            status: "skipped".to_string(),
            value: "not part of current Vesty release gate".to_string(),
            hint: None,
        });
        let error = validate_release_check_report_shape(&extra)
            .expect_err("release-check report with unknown gate should be rejected")
            .to_string();
        assert!(error.contains("check set must match"), "{error}");
        assert!(
            error.contains("unknown check(s): manual extra gate"),
            "{error}"
        );
    }

    #[test]
    fn release_check_report_shape_validates_optional_os_label() {
        let mut bad_os = build_release_check_report(
            complete_release_rows(),
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );
        bad_os.os = Some("Solaris".to_string());

        let error = validate_release_check_report_shape(&bad_os)
            .expect_err("unknown release-check os label should be rejected")
            .to_string();

        assert!(
            error.contains("invalid release-check os `Solaris`"),
            "{error}"
        );

        let mut control_os = bad_os;
        control_os.os = Some("Linux\nforged".to_string());
        let error = validate_release_check_report_shape(&control_os)
            .expect_err("control chars in release-check os should be rejected")
            .to_string();
        assert!(
            error.contains("release-check os must not contain control characters"),
            "{error}"
        );
    }

    #[test]
    fn release_check_report_rejects_unknown_json_fields() {
        let mut report = serde_json::to_value(test_ci_release_check_report()).unwrap();
        report["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<ReleaseCheckReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut report = serde_json::to_value(test_ci_release_check_report()).unwrap();
        report["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<ReleaseCheckReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));
    }

    #[test]
    fn release_check_report_shape_rejects_daw_summary_drift() {
        let rows = complete_release_rows();
        let mut report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        report
            .checks
            .iter_mut()
            .find(|check| check.name == "daw matrix")
            .unwrap()
            .status = "failed".to_string();
        report.status = "failed".to_string();

        let error = validate_release_check_report_shape(&report)
            .expect_err("DAW matrix summary status should match daw_matrix details")
            .to_string();

        assert!(error.contains("daw matrix"), "{error}");
        assert!(error.contains("inconsistent with daw_matrix"), "{error}");
    }

    #[test]
    fn release_check_report_shape_rejects_daw_smoke_row_drift() {
        let mut rows = complete_release_rows();
        rows[0]["load"] = serde_json::Value::Bool(false);
        let mut report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == "daw smoke: REAPER")
            .unwrap();
        check.status = "ok".to_string();
        check.value = "all smoke checks pass".to_string();

        let error = validate_release_check_report_shape(&report)
            .expect_err("per-host DAW smoke check should match daw_matrix row details")
            .to_string();

        assert!(error.contains("daw smoke: REAPER"), "{error}");
        assert!(error.contains("expected failed (missing: load)"), "{error}");
    }

    #[test]
    fn release_check_report_can_be_written_to_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("reports/release-check.json");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        write_release_check_report(Some(&report_path), &report).unwrap();

        let text = fs::read_to_string(report_path).unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&text).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["checks"][0]["name"], "host profiles");
    }

    #[test]
    fn release_check_report_writer_rejects_malformed_report_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("reports/release-check.json");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        let mut duplicate_check = report.clone();
        duplicate_check
            .checks
            .push(duplicate_check.checks[0].clone());
        let error = write_release_check_report(Some(&report_path), &duplicate_check)
            .expect_err("duplicate check names should be rejected before write")
            .to_string();
        assert!(error.contains("duplicate check name"));
        assert!(!report_path.exists());

        let mut unsafe_hint = report.clone();
        unsafe_hint.checks[0].hint = Some("profile\u{202E}".to_string());
        let error = write_release_check_report(Some(&report_path), &unsafe_hint)
            .expect_err("unsafe Unicode in release-check hint should be rejected")
            .to_string();
        assert!(error.contains("unsafe Unicode"));
        assert!(!report_path.exists());

        let mut bad_daw = report;
        bad_daw.daw_matrix[0]["scan"] = serde_json::Value::String("true".to_string());
        let error = write_release_check_report(Some(&report_path), &bad_daw)
            .expect_err("non-boolean DAW matrix check should be rejected")
            .to_string();
        assert!(error.contains("must be boolean"));
        assert!(!report_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn release_check_report_writer_rejects_symlink_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-release-check.json");
        let report_path = root.join("reports/release-check.json");
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, &report_path).unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        let error = write_release_check_report(Some(&report_path), &report)
            .expect_err("release-check report writer should reject symlink output")
            .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn release_check_report_writer_rejects_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external_reports =
            Utf8PathBuf::from_path_buf(temp.path().join("external-reports")).unwrap();
        fs::create_dir(&external_reports).unwrap();
        unix_fs::symlink(&external_reports, root.join("reports")).unwrap();
        let report_path = root.join("reports/release-check.json");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let report = build_release_check_report(
            rows,
            Utf8Path::new("unused"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        let error = write_release_check_report(Some(&report_path), &report)
            .expect_err("release-check report writer should reject symlink output parents")
            .to_string();

        assert!(error.contains("output file parent must not be a symlink"));
        assert!(!external_reports.join("release-check.json").exists());
    }

    fn write_doctor_artifact(path: &Utf8Path, os: &str) {
        let mut names = vec![
            "rustc",
            "cargo",
            "node",
            "npm",
            "vst3 binding baseline",
            "vst3 SDK headers",
            "vst3 validator",
            "system webview",
        ];
        match os {
            "Linux" => names.push("signing: linux release policy"),
            "macOS" => {
                names.push("signing: codesign");
                names.push("signing: notarytool");
            }
            "Windows" => names.push("signing: signtool"),
            _ => {}
        }
        let report = DoctorReport {
            os: Some(os.to_string()),
            ci_run_url: None,
            checks: names
                .into_iter()
                .map(|name| DoctorCheck {
                    name: name.to_string(),
                    status: if name == "vst3 SDK headers" {
                        "skipped".to_string()
                    } else {
                        "ok".to_string()
                    },
                    value: if name == "vst3 SDK headers" {
                        "VESTY_VST3_SDK_DIR is not set; upstream vst3 crate backend is active"
                            .to_string()
                    } else {
                        "test".to_string()
                    },
                    hint: None,
                })
                .collect(),
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn write_code_resources_plist(bundle: &Utf8Path) {
        fs::write(
            bundle.join("Contents/_CodeSignature/CodeResources"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>files2</key>
  <dict>
    <key>Contents/MacOS/VestyGain</key>
    <dict/>
  </dict>
</dict>
</plist>
"#,
        )
        .unwrap();
    }

    fn set_doctor_ci_run_url(path: &Utf8Path, ci_run_url: &str) {
        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        report.ci_run_url = Some(ci_run_url.to_string());
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn set_doctor_report_os(path: &Utf8Path, os: Option<&str>) {
        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        report.os = os.map(str::to_string);
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn set_doctor_check_status(path: &Utf8Path, name: &str, status: &str) {
        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == name)
            .unwrap_or_else(|| panic!("missing doctor check: {name}"));
        check.status = status.to_string();
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn remove_doctor_check(path: &Utf8Path, name: &str) {
        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        report.checks.retain(|check| check.name != name);
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn write_validate_artifact(path: &Utf8Path, static_status: &str, validator_status: &str) {
        let binaries = if static_status == "ok" {
            vec!["Gain.vst3/Contents/MacOS/Gain"]
        } else {
            Vec::new()
        };
        write_validate_artifact_with_binaries(path, static_status, validator_status, binaries);
    }

    fn test_publish_plan() -> PublishPlan {
        PublishPlan {
            packages: vec![
                PublishPlanPackage {
                    order: 1,
                    level: 1,
                    name: "vesty-params".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty-params/Cargo.toml".to_string(),
                    internal_dependencies: Vec::new(),
                },
                PublishPlanPackage {
                    order: 2,
                    level: 2,
                    name: "vesty-core".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty-core/Cargo.toml".to_string(),
                    internal_dependencies: vec!["vesty-params".to_string()],
                },
                PublishPlanPackage {
                    order: 3,
                    level: 3,
                    name: "vesty".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty/Cargo.toml".to_string(),
                    internal_dependencies: vec!["vesty-core".to_string()],
                },
            ],
            skipped_private: vec!["vesty-example-gain".to_string()],
        }
    }

    fn write_publish_plan_artifact(path: &Utf8Path) {
        write_publish_plan_artifact_with_plan(path, &test_publish_plan());
    }

    fn write_publish_plan_artifact_with_plan(path: &Utf8Path, plan: &PublishPlan) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(plan).unwrap()).unwrap();
    }

    fn test_crate_package_report() -> CratePackageReport {
        CratePackageReport {
            version: CRATE_PACKAGE_REPORT_VERSION,
            generator: CRATE_PACKAGE_REPORT_GENERATOR.to_string(),
            status: "ok".to_string(),
            publish_plan: test_publish_plan(),
            packages: vec![
                CratePackageEntry {
                    name: "vesty-params".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty-params/Cargo.toml".to_string(),
                    publish_order: 1,
                    internal_dependencies: Vec::new(),
                    status: "packaged".to_string(),
                    reason: None,
                },
                CratePackageEntry {
                    name: "vesty-core".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty-core/Cargo.toml".to_string(),
                    publish_order: 2,
                    internal_dependencies: vec!["vesty-params".to_string()],
                    status: "deferred".to_string(),
                    reason: Some(
                        "requires published internal dependencies: vesty-params".to_string(),
                    ),
                },
                CratePackageEntry {
                    name: "vesty".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: "/workspace/crates/vesty/Cargo.toml".to_string(),
                    publish_order: 3,
                    internal_dependencies: vec!["vesty-core".to_string()],
                    status: "deferred".to_string(),
                    reason: Some(
                        "requires published internal dependencies: vesty-core".to_string(),
                    ),
                },
            ],
        }
    }

    fn write_crate_package_artifact(path: &Utf8Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        write_crate_package_report(path, &test_crate_package_report()).unwrap();
    }

    fn test_npm_pack_report() -> Vec<NpmPackEntry> {
        [(
            "vesty-plugin-ui",
            "vesty-plugin-ui-0.1.0.tgz",
            vec![
                "dist/index.d.ts",
                "dist/index.js",
                "dist/protocol/index.d.ts",
                "dist/protocol/index.js",
                "dist/react.d.ts",
                "dist/react.js",
                "dist/svelte.d.ts",
                "dist/svelte.js",
                "dist/vue.d.ts",
                "dist/vue.js",
                "package.json",
            ],
        )]
        .into_iter()
        .map(|(name, filename, files)| NpmPackEntry {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            filename: filename.to_string(),
            files: files
                .into_iter()
                .map(|path| NpmPackFile {
                    path: path.to_string(),
                })
                .collect(),
        })
        .collect()
    }

    fn write_npm_pack_artifact(path: &Utf8Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            path,
            serde_json::to_string_pretty(&test_npm_pack_report()).unwrap(),
        )
        .unwrap();
    }

    fn write_dependency_baseline_latest_artifact(path: &Utf8Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        write_dependency_baseline_report(path, &report).unwrap();
    }

    fn test_ci_release_check_report() -> ReleaseCheckReport {
        test_ci_release_check_report_for_os(None)
    }

    fn test_ci_release_check_report_for_os(os: Option<&str>) -> ReleaseCheckReport {
        let mut daw_matrix = complete_release_rows();
        daw_matrix[0]["load"] = serde_json::Value::Bool(false);
        let release_evidence = ReleaseEvidenceOptions {
            ci_run_url: Some(
                "https://github.com/vesty-rs/vesty/actions/runs/1234567890".to_string(),
            ),
            ..ReleaseEvidenceOptions::default()
        };
        let mut report = build_release_check_report(
            daw_matrix,
            Utf8Path::new("target/vesty-protocol"),
            true,
            &release_evidence,
        );
        report.os = os.map(str::to_string);
        report
    }

    fn write_ci_release_check_artifact(path: &Utf8Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let os = artifact_os_from_path(path);
        fs::write(
            path,
            serde_json::to_string_pretty(&test_ci_release_check_report_for_os(os)).unwrap(),
        )
        .unwrap();
    }

    fn test_release_action_plan() -> ReleaseActionPlan {
        let failed = 1;
        let skipped = 1;
        let ok = expected_release_check_names().len() - failed - skipped;
        ReleaseActionPlan {
            version: 1,
            status: "failed".to_string(),
            summary: ReleaseActionPlanSummary {
                ok,
                failed,
                skipped,
                action_count: failed + skipped,
            },
            protocol_snapshot: "target/vesty-protocol".to_string(),
            evidence_root: Some("target/daw-evidence".to_string()),
            release_evidence_dir: Some("target/release-evidence".to_string()),
            actions: vec![
                ReleaseActionItem {
                    check: "daw smoke: REAPER".to_string(),
                    status: "failed".to_string(),
                    priority: "required".to_string(),
                    value: "missing: ui".to_string(),
                    hint: Some("collect REAPER evidence".to_string()),
                    evidence_path: Some("target/daw-evidence/reaper".to_string()),
                    commands: vec![
                        "vesty daw-matrix --write-report --host reaper --ui \"ui=true\""
                            .to_string(),
                    ],
                },
                ReleaseActionItem {
                    check: "vst3 SDK header manifest".to_string(),
                    status: "skipped".to_string(),
                    priority: "optional".to_string(),
                    value: "not requested".to_string(),
                    hint: Some("optional generated-headers audit".to_string()),
                    evidence_path: Some(
                        "target/release-evidence/vst3-sdk/vst3-sdk-headers.json".to_string(),
                    ),
                    commands: vec![
                        "vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK".to_string(),
                    ],
                },
            ],
        }
    }

    fn write_release_action_plan_artifact(path: &Utf8Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            path,
            serde_json::to_string_pretty(&test_release_action_plan()).unwrap(),
        )
        .unwrap();
    }

    fn write_platform_smoke_matrix(root: &Utf8Path) {
        for (platform, _) in REQUIRED_PLATFORM_SMOKE_PLATFORMS {
            write_platform_smoke_artifact(&root.join(format!("{platform}.json")), platform);
        }
    }

    fn write_platform_smoke_artifact(path: &Utf8Path, platform: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let report = PlatformSmokeReport {
            platform: platform.to_string(),
            os: Some(platform.to_string()),
            host: Some("Vesty test host".to_string()),
            checks: vec![
                PlatformSmokeCheck {
                    name: "system_webview".to_string(),
                    status: "ok".to_string(),
                    value: match platform {
                        "macos" => "WebKit.framework loaded",
                        "windows-x64" => "WebView2 runtime loaded",
                        "linux-x11" => "WebKitGTK loaded; X11 display active",
                        other => other,
                    }
                    .to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "vst3_validator".to_string(),
                    status: "ok".to_string(),
                    value: "Steinberg validator passed 47 tests, 0 failed".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "vst3_example_scan".to_string(),
                    status: "ok".to_string(),
                    value: "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "webview_attach".to_string(),
                    status: "ok".to_string(),
                    value: "webview_attach=true".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "webview_resize".to_string(),
                    status: "ok".to_string(),
                    value: "webview_resize=true width=640 height=420".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "asset_protocol".to_string(),
                    status: "ok".to_string(),
                    value: "asset_protocol=true assets.manifest.json served".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "jsbridge_roundtrip".to_string(),
                    status: "ok".to_string(),
                    value: "jsbridge_roundtrip=true readyAck reply".to_string(),
                    hint: None,
                },
                PlatformSmokeCheck {
                    name: "meter_stream".to_string(),
                    status: "ok".to_string(),
                    value: "meter_flush sent=3".to_string(),
                    hint: None,
                },
            ],
        };
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn test_platform_smoke_report_input(platform: &str) -> PlatformSmokeReportInput {
        PlatformSmokeReportInput {
            platform: Some(platform.to_string()),
            os: None,
            host: Some("Vesty smoke host".to_string()),
            system_webview: Some(
                match platform {
                    "windows-x64" => "WebView2 runtime loaded",
                    "linux-x11" => "WebKitGTK loaded; X11 display active",
                    _ => "WebKit.framework loaded",
                }
                .to_string(),
            ),
            vst3_validator: Some("Steinberg validator passed 47 tests, 0 failed".to_string()),
            vst3_example_scan: Some(
                "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3".to_string(),
            ),
            webview_attach: Some("webview_attach=true".to_string()),
            webview_resize: Some("webview_resize=true width=640 height=420".to_string()),
            asset_protocol: Some("asset_protocol=true assets.manifest.json served".to_string()),
            jsbridge_roundtrip: Some("jsbridge_roundtrip=true readyAck reply".to_string()),
            meter_stream: Some("meter_flush sent=3".to_string()),
        }
    }

    fn write_validate_artifact_with_binaries(
        path: &Utf8Path,
        static_status: &str,
        validator_status: &str,
        binaries: Vec<&str>,
    ) {
        write_validate_artifact_with_bundle_and_binaries(
            path,
            "Gain.vst3",
            static_status,
            validator_status,
            binaries,
        );
    }

    fn write_validate_artifact_with_bundle_and_binaries(
        path: &Utf8Path,
        bundle: &str,
        static_status: &str,
        validator_status: &str,
        binaries: Vec<&str>,
    ) {
        let has_web_ui_assets = static_status == "ok" && bundle == REQUIRED_WEB_UI_EXAMPLE_BUNDLE;
        let asset_manifest =
            has_web_ui_assets.then(|| format!("{bundle}/Contents/Resources/assets.manifest.json"));
        let asset_count = if has_web_ui_assets { 2 } else { 0 };
        write_validate_artifact_with_bundle_and_binaries_and_assets(
            path,
            bundle,
            static_status,
            validator_status,
            binaries,
            asset_manifest,
            asset_count,
        );
    }

    fn write_validate_artifact_with_bundle_and_binaries_and_assets(
        path: &Utf8Path,
        bundle: &str,
        static_status: &str,
        validator_status: &str,
        binaries: Vec<&str>,
        asset_manifest: Option<String>,
        asset_count: usize,
    ) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let has_example_parameter_manifest =
            static_status == "ok" && REQUIRED_EXAMPLE_BUNDLES.contains(&bundle);
        let parameter_manifest = has_example_parameter_manifest
            .then(|| format!("{bundle}/Contents/Resources/{PARAMETER_MANIFEST_FILE}"));
        let report = ValidateReport {
            bundle: bundle.to_string(),
            static_check: StaticBundleCheck {
                status: static_status.to_string(),
                moduleinfo: (static_status == "ok")
                    .then(|| format!("{bundle}/Contents/Resources/moduleinfo.json")),
                binaries: binaries
                    .iter()
                    .map(|binary| (*binary).to_string())
                    .collect(),
                binary_exports: if static_status == "ok" {
                    binaries
                        .iter()
                        .filter_map(|binary| test_binary_export_check(binary))
                        .collect()
                } else {
                    Vec::new()
                },
                parameter_manifest,
                asset_manifest,
                asset_count,
                error: (static_status != "ok").then(|| "static validation failed".to_string()),
            },
            validator: ValidatorCheck {
                status: validator_status.to_string(),
                path: (validator_status == "passed").then(|| "/tools/validator".to_string()),
                exit_code: (validator_status == "passed").then_some(0),
                tests_passed: (validator_status == "passed").then_some(47),
                tests_failed: (validator_status == "passed").then_some(0),
                stdout: None,
                stderr: None,
                reason: (validator_status == "skipped").then(|| "--static-only".to_string()),
                error: (validator_status == "failed").then(|| "validator failed".to_string()),
            },
        };
        fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    fn test_binary_export_check(binary: &str) -> Option<BinaryExportCheck> {
        let platform = infer_validate_binary_platform(binary)?;
        let required_symbols = vesty_vst3_sys::required_binary_export_tool_symbols(platform)
            .unwrap_or_else(|| panic!("unsupported test platform: {platform}"))
            .iter()
            .map(|symbol| (*symbol).to_string())
            .collect::<Vec<_>>();
        Some(BinaryExportCheck {
            binary: binary.to_string(),
            platform: platform.to_string(),
            status: "ok".to_string(),
            tool: Some("test-symbol-surface".to_string()),
            required_symbols: required_symbols.clone(),
            found_symbols: required_symbols,
            missing_symbols: Vec::new(),
            error: None,
        })
    }

    fn write_example_validate_reports(root: &Utf8Path) -> Vec<Utf8PathBuf> {
        let mut paths = REQUIRED_EXAMPLE_BUNDLES
            .iter()
            .map(|bundle| {
                let path = root.join(format!("{bundle}.validator.json"));
                write_example_validate_artifact(&path, bundle);
                path
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn write_example_validate_matrix(root: &Utf8Path) -> Vec<Utf8PathBuf> {
        let mut paths = REQUIRED_EXAMPLE_VALIDATE_PLATFORMS
            .iter()
            .flat_map(|platform| {
                REQUIRED_EXAMPLE_BUNDLES.iter().map(move |bundle| {
                    let path = root.join(format!("{bundle}.{platform}.validator.json"));
                    write_example_validate_artifact_for_platform(&path, bundle, platform);
                    path
                })
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn write_example_validate_artifact(path: &Utf8Path, bundle: &str) {
        write_example_validate_artifact_for_platform(path, bundle, "macos");
    }

    fn write_example_validate_artifact_for_platform(path: &Utf8Path, bundle: &str, platform: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let binary = example_binary_path(bundle, platform);
        write_validate_artifact_with_bundle_and_binaries(
            path,
            bundle,
            "ok",
            "passed",
            vec![binary.as_str()],
        );
    }

    fn write_example_static_validate_matrix(root: &Utf8Path) -> Vec<Utf8PathBuf> {
        REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS
            .iter()
            .flat_map(|platform| write_example_static_validate_platform(root, platform))
            .collect()
    }

    fn write_example_static_validate_platform(root: &Utf8Path, platform: &str) -> Vec<Utf8PathBuf> {
        REQUIRED_EXAMPLE_BUNDLES
            .iter()
            .map(|bundle| {
                let path = root.join(format!("{bundle}.{platform}.validate.json"));
                write_example_static_validate_artifact(&path, bundle, platform);
                path
            })
            .collect()
    }

    fn write_example_static_validate_artifact(path: &Utf8Path, bundle: &str, platform: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let binary = example_binary_path(bundle, platform);
        write_validate_artifact_with_bundle_and_binaries(
            path,
            bundle,
            "ok",
            "skipped",
            vec![binary.as_str()],
        );
    }

    fn example_binary_path(bundle: &str, platform: &str) -> String {
        let binary_name = bundle.strip_suffix(".vst3").unwrap_or(bundle);
        match platform {
            "macos" => format!("{bundle}/Contents/MacOS/{binary_name}"),
            "windows-x64" => format!("{bundle}/Contents/x86_64-win/{binary_name}.vst3"),
            "linux-x64" => format!("{bundle}/Contents/x86_64-linux/{binary_name}.so"),
            other => panic!("unsupported test platform: {other}"),
        }
    }
