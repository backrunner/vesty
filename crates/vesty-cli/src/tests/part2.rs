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

    #[test]
    fn import_ci_release_evidence_normalizes_downloaded_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        let ci_run_url = "https://github.com/vesty-rs/vesty/actions/runs/1234567890";

        vesty_ipc::export_protocol_bindings(source.join("vesty-protocol")).unwrap();
        for os in ["Linux", "macOS", "Windows"] {
            let path = source.join(format!("doctor-{os}/doctor-{os}.json"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            write_doctor_artifact(&path, os);
            set_doctor_ci_run_url(&path, ci_run_url);

            write_ci_release_check_artifact(
                &source.join(format!("release-check-{os}/release-check-{os}.json")),
            );
            write_release_action_plan_artifact(
                &source.join(format!("release-check-{os}/release-action-plan-{os}.json")),
            );
        }
        write_publish_plan_artifact(&source.join("vesty-publish-plan/publish-plan.json"));
        write_crate_package_artifact(&source.join("vesty-crate-package/crate-package.json"));
        write_npm_pack_artifact(&source.join("vesty-npm-pack/npm-pack.json"));
        write_dependency_baseline_latest_artifact(
            &source.join("vesty-dependency-baseline/dependency-baseline-latest.json"),
        );
        write_example_static_validate_matrix(&source.join("linux-vst3-static-validate"));
        write_example_validate_artifact_for_platform(
            &source.join("validator/VestyGain.macos.validate.json"),
            "VestyGain.vst3",
            "macos",
        );
        write_test_vst3_sdk_manifest(&source.join("vst3-sdk-artifact/vst3-sdk-headers.json"), &[]);
        write_test_vst3_sdk_binding_plan(
            &source.join("vst3-sdk-artifact/generated-bindings-plan.json"),
            &[],
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        write_test_vst3_sdk_binding_surface(
            &source.join("vst3-sdk-artifact/generated-bindings-surface.json"),
            &[],
        );
        write_test_vst3_sdk_scaffold(
            &source.join("vst3-sdk-artifact/generated.rs"),
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        write_test_vst3_sdk_abi_seed(&source.join("vst3-sdk-artifact/generated-abi-seed.rs"));
        write_test_vst3_sdk_abi(&source.join("vst3-sdk-artifact/generated-abi.rs"));
        write_test_vst3_sdk_interface_skeleton(
            &source.join("vst3-sdk-artifact/generated-interface-skeleton.rs"),
        );

        import_ci_release_evidence(ImportCiOptions {
            source: source.clone(),
            dir: evidence.clone(),
            ci_run_url: Some(ci_run_url.to_string()),
            ci_run_url_file: None,
            template: true,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(evidence.join("ci-run-url.txt").is_file());
        assert!(evidence.join("vesty-protocol/typescript").is_dir());
        assert!(evidence.join("vesty-protocol/json-schema").is_dir());
        assert!(evidence.join("ci-doctor/doctor-Linux.json").is_file());
        assert!(evidence.join("ci-doctor/doctor-macOS.json").is_file());
        assert!(evidence.join("ci-doctor/doctor-Windows.json").is_file());
        assert!(
            evidence
                .join("ci-release-checks/release-check-Linux.json")
                .is_file()
        );
        assert!(
            evidence
                .join("ci-release-checks/release-action-plan-Linux.json")
                .is_file()
        );
        assert!(evidence.join("publish-plan/publish-plan.json").is_file());
        assert!(evidence.join("crate-package/crate-package.json").is_file());
        assert!(evidence.join("npm-pack/npm-pack.json").is_file());
        assert!(
            evidence
                .join("dependency-baseline/dependency-baseline-latest.json")
                .is_file()
        );
        assert!(
            evidence
                .join("package/VestyGain.vst3.linux-x64.static-validate.json")
                .is_file()
        );
        assert!(
            evidence
                .join("validator/VestyGain.vst3.macos.validate.json")
                .is_file()
        );
        assert!(evidence.join("vst3-sdk/vst3-sdk-headers.json").is_file());
        assert!(
            evidence
                .join("vst3-sdk/generated-bindings-plan.json")
                .is_file()
        );
        assert!(
            evidence
                .join("vst3-sdk/generated-bindings-surface.json")
                .is_file()
        );
        assert!(evidence.join("vst3-sdk/generated.rs").is_file());
        assert!(evidence.join("vst3-sdk/generated-abi-seed.rs").is_file());
        assert!(evidence.join("vst3-sdk/generated-abi.rs").is_file());
        assert!(
            evidence
                .join("vst3-sdk/generated-interface-skeleton.rs")
                .is_file()
        );

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(
            report
                .items
                .iter()
                .any(|item| item.name == "protocol snapshot" && item.status == "imported")
        );
        assert!(
            report
                .items
                .iter()
                .any(|item| item.name == "vst3 static validate report"
                    && item.status == "imported")
        );
        assert!(
            report.items.iter().any(|item| {
                item.name == "vst3 SDK header manifest" && item.status == "imported"
            })
        );
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings plan" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings surface" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings scaffold" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI seed" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI layout" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings interface skeleton"
                && item.status == "imported"
        }));
        assert!(
            report.items.iter().any(|item| {
                item.name == "crate package readiness" && item.status == "imported"
            })
        );
        assert!(report.items.iter().any(|item| {
            item.name == "release action plan sidecar" && item.status == "imported"
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "dependency latest baseline" && item.status == "imported"
        }));
        assert!(
            report.items.iter().all(|item| item.status != "failed"),
            "{:?}",
            report.items
        );

        let mut options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        apply_release_evidence_dir(&mut options, &evidence).unwrap();
        assert_eq!(options.ci_run_url.as_deref(), Some(ci_run_url));
        assert_eq!(options.ci_doctor_dir, Some(evidence.join("ci-doctor")));
        assert_eq!(
            options.ci_release_check_dir,
            Some(evidence.join("ci-release-checks"))
        );
        assert_eq!(
            options.publish_plan_report,
            Some(evidence.join("publish-plan/publish-plan.json"))
        );
        assert_eq!(
            options.crate_package_report,
            Some(evidence.join("crate-package/crate-package.json"))
        );
        assert_eq!(
            options.npm_pack_report,
            Some(evidence.join("npm-pack/npm-pack.json"))
        );
        assert_eq!(
            options.dependency_baseline_report,
            Some(evidence.join("dependency-baseline/dependency-baseline-latest.json"))
        );
        assert_eq!(
            options.vst3_sdk_manifest,
            Some(evidence.join("vst3-sdk/vst3-sdk-headers.json"))
        );
        assert_eq!(
            options.vst3_sdk_binding_plan,
            Some(evidence.join("vst3-sdk/generated-bindings-plan.json"))
        );
        assert_eq!(
            options.vst3_sdk_binding_surface,
            Some(evidence.join("vst3-sdk/generated-bindings-surface.json"))
        );
        assert_eq!(options.validate_reports.len(), 1);
        assert_eq!(options.static_validate_reports.len(), 9);
        check_protocol_export(&evidence.join("vesty-protocol")).unwrap();

        assert_eq!(
            ci_doctor_artifacts_release_check(
                options.ci_doctor_dir.as_deref(),
                true,
                options.ci_run_url.as_deref()
            )
            .status,
            "ok"
        );
        assert_eq!(
            ci_release_check_artifacts_release_check(
                options.ci_release_check_dir.as_deref(),
                true,
                options.ci_run_url.as_deref()
            )
            .status,
            "ok"
        );
        assert_eq!(
            example_static_validate_coverage_release_check(&options.static_validate_reports, true)
                .status,
            "ok"
        );
        assert_eq!(
            publish_plan_release_check(options.publish_plan_report.as_deref(), true).status,
            "ok"
        );
        assert_eq!(
            crate_package_release_check(
                options.crate_package_report.as_deref(),
                options.publish_plan_report.as_deref(),
                true
            )
            .status,
            "ok"
        );
        assert_eq!(
            npm_pack_release_check(options.npm_pack_report.as_deref(), true).status,
            "ok"
        );
        assert_eq!(
            dependency_baseline_latest_release_check(
                options.dependency_baseline_report.as_deref(),
                true
            )
            .status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_manifest_release_check(options.vst3_sdk_manifest.as_deref()).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_binding_plan_release_check(options.vst3_sdk_binding_plan.as_deref()).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_binding_surface_release_check(options.vst3_sdk_binding_surface.as_deref())
                .status,
            "ok"
        );
    }

    #[test]
    fn import_ci_rejects_validate_artifact_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");

        write_example_validate_artifact_for_platform(
            &source.join("Windows/VestyGain.validate.json"),
            "VestyGain.vst3",
            "macos",
        );
        write_example_validate_artifact_for_platform(
            &source.join("validator/VestyGain.macos.windows-x64.validate.json"),
            "VestyGain.vst3",
            "macos",
        );
        write_validate_artifact_with_bundle_and_binaries(
            &source.join("macOS/ThirdParty.static-validate.json"),
            "ThirdParty.vst3",
            "ok",
            "skipped",
            vec!["ThirdParty.vst3/Contents/x86_64-win/ThirdParty.vst3"],
        );
        write_example_static_validate_matrix(&source.join("linux-vst3-static-validate"));

        import_ci_release_evidence(ImportCiOptions {
            source: source.clone(),
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(
            !evidence
                .join("validator/VestyGain.vst3.macos.validate.json")
                .exists()
        );
        assert!(
            !evidence
                .join("package/ThirdParty.vst3.windows-x64.static-validate.json")
                .exists()
        );
        assert!(
            evidence
                .join("package/VestyGain.vst3.linux-x64.static-validate.json")
                .is_file()
        );
        assert!(
            evidence
                .join("package/VestyWebUIDemo.vst3.macos.static-validate.json")
                .is_file()
        );
        assert!(
            evidence
                .join("package/VestyMIDISynth.vst3.windows-x64.static-validate.json")
                .is_file()
        );

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 validate report"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "Windows/VestyGain.validate.json")
                })
                && item.value.contains("artifact path indicates windows-x64")
                && item.value.contains("report platform is macos")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 validate report"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(
                        source,
                        "validator/VestyGain.macos.windows-x64.validate.json",
                    )
                })
                && item
                    .value
                    .contains("validate report file name contains multiple platform labels")
                && item.value.contains("macos")
                && item.value.contains("windows-x64")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 static validate report"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "macOS/ThirdParty.static-validate.json")
                })
                && item.value.contains("artifact path indicates macos")
                && item.value.contains("report platform is windows-x64")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 static validate report"
                && item.status == "imported"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| source.contains("linux-vst3-static-validate"))
        }));
    }

    #[test]
    fn import_ci_rejects_cross_os_doctor_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        let doctor = source.join("doctor-Linux.json");
        fs::create_dir_all(doctor.parent().unwrap()).unwrap();
        write_doctor_artifact(&doctor, "Linux");
        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&doctor).unwrap()).unwrap();
        report.checks.push(DoctorCheck {
            name: "signing: notarytool".to_string(),
            status: "ok".to_string(),
            value: "notarytool belongs to macOS doctor evidence".to_string(),
            hint: None,
        });
        fs::write(&doctor, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source: source.clone(),
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("ci-doctor/doctor-Linux.json").exists());
        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "ci doctor artifact"
                && item.status == "failed"
                && item
                    .value
                    .contains("Linux/signing: notarytool unexpected for Linux doctor report")
                && item.value.contains("signing: notarytool")
        }));
    }

    #[test]
    fn import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");

        let doctor = source.join("doctor-Linux.json");
        fs::create_dir_all(doctor.parent().unwrap()).unwrap();
        write_doctor_artifact(&doctor, "Linux");
        remove_doctor_check(&doctor, "vst3 binding baseline");
        write_validate_artifact(&source.join("failed-validate.json"), "failed", "skipped");
        let offline_baseline = dependency_baseline_report(&workspace_root()).unwrap();
        write_dependency_baseline_report(
            &source.join("dependency-baseline.json"),
            &offline_baseline,
        )
        .unwrap();
        let mut latest_without_coverage = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        latest_without_coverage
            .checks
            .retain(|check| check.name != DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME);
        let latest_without_coverage_path =
            source.join("vesty-dependency-baseline/dependency-baseline-latest.json");
        if let Some(parent) = latest_without_coverage_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            latest_without_coverage_path,
            serde_json::to_string_pretty(&latest_without_coverage).unwrap(),
        )
        .unwrap();
        let mut invalid_action_plan = test_release_action_plan();
        invalid_action_plan.summary.action_count += 1;
        let invalid_action_plan_path =
            source.join("release-check-Linux/release-action-plan-Linux.json");
        fs::create_dir_all(invalid_action_plan_path.parent().unwrap()).unwrap();
        fs::write(
            invalid_action_plan_path,
            serde_json::to_string_pretty(&invalid_action_plan).unwrap(),
        )
        .unwrap();
        let mut invalid_crate_package = test_crate_package_report();
        invalid_crate_package.packages[0].status = "deferred".to_string();
        write_crate_package_report(
            &source.join("vesty-crate-package/crate-package.json"),
            &invalid_crate_package,
        )
        .unwrap();
        fs::write(source.join("notary.log"), r#"{ "status": "Accepted" }"#).unwrap();
        write_platform_smoke_templates(&source.join("platform-smoke")).unwrap();
        write_platform_smoke_artifact(&source.join("Windows/platform-smoke.json"), "macos");
        write_platform_smoke_artifact(
            &source.join("macos-windows/platform-smoke.json"),
            "windows-x64",
        );
        write_test_vst3_sdk_manifest(
            &source.join("vst3-sdk/incomplete-vst3-sdk-headers.json"),
            &["pluginterfaces/vst/ivstmessage.h"],
        );
        write_test_vst3_sdk_binding_plan(
            &source.join("vst3-sdk/blocked-generated-bindings-plan.json"),
            &["pluginterfaces/vst/ivstmessage.h"],
            Utf8Path::new("target/vst3-sdk/generated.txt"),
        );
        write_test_vst3_sdk_binding_surface(
            &source.join("vst3-sdk/blocked-generated-bindings-surface.json"),
            &["pluginterfaces/gui/iplugview.h"],
        );
        fs::write(
            source.join("vst3-sdk/generated.rs"),
            format!(
                "// @generated by {}\n\
                 pub const STATUS: &str = \"metadata-scaffold\";\n\
                 pub const PLAN_GENERATOR: &str = \"{}\";\n\
                 pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";\n\
                 pub const BINDINGS_GENERATED: bool = true;\n",
                vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
            ),
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-abi-seed.rs"),
            format!(
                "// @generated by {}\n\
                 pub const STATUS: &str = \"abi-seed\";\n\
                 pub const PLAN_GENERATOR: &str = \"{}\";\n\
                 pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";\n\
                 pub const ABI_SEED_GENERATED: bool = true;\n\
                 pub const BINDINGS_GENERATED: bool = false;\n\
                 pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
            ),
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-abi.rs"),
            format!(
                "// @generated by {}\n\
                 pub const STATUS: &str = \"abi-layout\";\n\
                 pub const PLAN_GENERATOR: &str = \"{}\";\n\
                 pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";\n\
                 pub const SURFACE_GENERATOR: &str = \"{}\";\n\
                 pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";\n\
                 pub const ABI_LAYOUT_GENERATED: bool = true;\n\
                 pub const BINDINGS_GENERATED: bool = false;\n\
                 pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
            ),
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-interface-skeleton.rs"),
            format!(
                "// @generated by {}\n\
                 pub const STATUS: &str = \"interface-skeleton\";\n\
                 pub const PLAN_GENERATOR: &str = \"{}\";\n\
                 pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";\n\
                 pub const SURFACE_GENERATOR: &str = \"{}\";\n\
                 pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";\n\
                 pub const INTERFACE_SKELETON_GENERATED: bool = true;\n\
                 pub const BINDINGS_GENERATED: bool = false;\n\
                 pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
                vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR,
                vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
            ),
        )
        .unwrap();
        write_stale_vst3_sdk_interface_skeleton_with_wrong_inspection_tool(
            &source.join("vst3-sdk/stale-generated-interface-skeleton.rs"),
        );

        import_ci_release_evidence(ImportCiOptions {
            source: source.clone(),
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("ci-doctor/doctor-Linux.json").exists());
        assert!(!evidence.join("notary.log").exists());
        assert!(!evidence.join("validator").exists());
        assert!(!evidence.join("package").exists());
        assert!(!evidence.join("platform-smoke/macos.json").exists());
        assert!(!evidence.join("platform-smoke/windows-x64.json").exists());
        assert!(
            !evidence
                .join("dependency-baseline/dependency-baseline-latest.json")
                .exists()
        );
        assert!(
            !evidence
                .join("ci-release-checks/release-action-plan-Linux.json")
                .exists()
        );
        assert!(!evidence.join("crate-package/crate-package.json").exists());
        assert!(!evidence.join("vst3-sdk/vst3-sdk-headers.json").exists());
        assert!(
            !evidence
                .join("vst3-sdk/generated-bindings-plan.json")
                .exists()
        );
        assert!(
            !evidence
                .join("vst3-sdk/generated-bindings-surface.json")
                .exists()
        );
        assert!(!evidence.join("vst3-sdk/generated.rs").exists());
        assert!(!evidence.join("vst3-sdk/generated-abi-seed.rs").exists());
        assert!(!evidence.join("vst3-sdk/generated-abi.rs").exists());
        assert!(
            !evidence
                .join("vst3-sdk/generated-interface-skeleton.rs")
                .exists()
        );

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "ci doctor artifact"
                && item.status == "failed"
                && item.value.contains("vst3 binding baseline")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 validate report"
                && item.status == "failed"
                && item.value.contains("static bundle check status is failed")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "notarization log"
                && item.status == "failed"
                && item.value.contains("stapler success")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "platform smoke artifact"
                && item.status == "skipped"
                && item.value.contains("pending")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "platform smoke artifact"
                && item.status == "failed"
                && item.value.contains("artifact path indicates windows-x64")
                && item.value.contains("report platform is macos")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "platform smoke artifact"
                && item.status == "failed"
                && item
                    .value
                    .contains("artifact path contains multiple platform tokens: macos, windows-x64")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "dependency latest baseline"
                && item.status == "failed"
                && item.value.contains("missing latest registry checks")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "dependency latest baseline"
                && item.status == "failed"
                && item.value.contains("missing required check(s)")
                && item.value.contains(DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME)
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "release action plan sidecar"
                && item.status == "failed"
                && item.value.contains("action count mismatch")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "crate package readiness"
                && item.status == "failed"
                && item
                    .value
                    .contains("no internal dependencies but status is deferred")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK header manifest"
                && item.status == "failed"
                && item.value.contains("manifest is incomplete")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings plan"
                && item.status == "failed"
                && item.value.contains("plan status is `blocked`")
                && item.value.contains("pluginterfaces/vst/ivstmessage.h")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings surface"
                && item.status == "failed"
                && item.value.contains("surface status is `blocked`")
                && item.value.contains("IPlugView")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings scaffold"
                && item.status == "failed"
                && item
                    .value
                    .contains("must not claim SDK bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI seed"
                && item.status == "failed"
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI layout"
                && item.status == "failed"
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings interface skeleton"
                && item.status == "failed"
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings interface skeleton"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(
                        source,
                        "vst3-sdk/stale-generated-interface-skeleton.rs",
                    )
                })
                && item.value.contains(
                    "missing vesty-vst3-sys binary export inspection tool plan `linux-x64/llvm-nm`",
                )
        }));

        let mut options = ReleaseEvidenceOptions::default();
        apply_release_evidence_dir(&mut options, &evidence).unwrap();
        assert!(options.ci_doctor_dir.is_none());
        assert!(options.validate_reports.is_empty());
        assert!(options.static_validate_reports.is_empty());
        assert!(options.crate_package_report.is_none());
        assert!(options.dependency_baseline_report.is_none());
        assert!(options.vst3_sdk_manifest.is_none());
        assert!(options.vst3_sdk_binding_plan.is_none());
        assert!(options.vst3_sdk_binding_surface.is_none());
        assert!(options.notarization_log.is_none());
    }

    #[test]
    fn import_ci_reports_malformed_named_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(source.join("validator")).unwrap();
        fs::create_dir_all(source.join("package")).unwrap();
        fs::create_dir_all(source.join("platform-smoke")).unwrap();
        fs::create_dir_all(source.join("ci-release-checks")).unwrap();

        fs::write(source.join("doctor-Linux.json"), "{").unwrap();
        fs::write(
            source.join("ci-release-checks/release-check-Windows.json"),
            r#"{ "status": "ok" }"#,
        )
        .unwrap();
        fs::write(
            source.join("validator/VestyGain.macos.validate.json"),
            r#"{ "status": "passed" }"#,
        )
        .unwrap();
        fs::write(
            source.join("package/VestyGain.linux-x64.static-validate.json"),
            r#"{ "kind": "not-a-validate-report" }"#,
        )
        .unwrap();
        fs::write(
            source.join("package/report.json"),
            r#"{ "kind": "broken" }"#,
        )
        .unwrap();
        fs::create_dir_all(source.join("vst3-sdk")).unwrap();
        fs::write(
            source.join("vst3-sdk/vst3-sdk-headers.json"),
            r#"{ "kind": "broken" }"#,
        )
        .unwrap();
        fs::write(source.join("generated-bindings-plan.json"), "{").unwrap();
        fs::create_dir_all(source.join("vesty-vst3-sdk")).unwrap();
        fs::write(
            source.join("vesty-vst3-sdk/generated-bindings-surface.json"),
            r#"{ "kind": "broken" }"#,
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated.rs"),
            "pub const BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-abi-seed.rs"),
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-abi.rs"),
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            source.join("vst3-sdk/generated-interface-skeleton.rs"),
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(source.join("platform-smoke/macos.json"), "{").unwrap();
        fs::write(source.join("notes.json"), r#"{ "note": true }"#).unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("ci-doctor/doctor-Linux.json").exists());
        assert!(
            !evidence
                .join("ci-release-checks/release-check-Windows.json")
                .exists()
        );
        assert!(!evidence.join("validator").exists());
        assert!(!evidence.join("package").exists());
        assert!(!evidence.join("platform-smoke").exists());

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "ci doctor artifact"
                && item.status == "failed"
                && item.value.contains("invalid JSON")
                && item.value.contains("CI doctor artifact")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "ci release-check artifact"
                && item.status == "failed"
                && item
                    .value
                    .contains("did not match the expected release evidence schema")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 validate report"
                && item.status == "failed"
                && item
                    .value
                    .contains("did not match the expected release evidence schema")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 static validate report"
                && item.status == "failed"
                && item
                    .value
                    .contains("did not match the expected release evidence schema")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 static validate report"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "package/report.json")
                })
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "platform smoke artifact"
                && item.status == "failed"
                && item.value.contains("invalid JSON")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK header manifest"
                && item.status == "failed"
                && item
                    .value
                    .contains("did not match the expected release evidence schema")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings plan"
                && item.status == "failed"
                && item.value.contains("invalid JSON")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings surface"
                && item.status == "failed"
                && item
                    .value
                    .contains("did not match the expected release evidence schema")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings scaffold"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "vst3-sdk/generated.rs")
                })
                && item
                    .value
                    .contains("must not claim SDK bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI seed"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "vst3-sdk/generated-abi-seed.rs")
                })
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings ABI layout"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "vst3-sdk/generated-abi.rs")
                })
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "vst3 SDK generated bindings interface skeleton"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(
                        source,
                        "vst3-sdk/generated-interface-skeleton.rs",
                    )
                })
                && item
                    .value
                    .contains("must not claim full COM bindings are generated")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "json artifact"
                && item.status == "skipped"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "notes.json"))
        }));
    }

    #[test]
    fn import_ci_reports_failed_signing_and_notarization_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("codesign-failed.log"),
            "codesign=pass\nVestyGain.vst3: invalid signature\n",
        )
        .unwrap();
        fs::write(
            source.join("signtool-output.log"),
            "signtool verify /pa /v VestyGain.vst3\nNumber of errors: 1\n",
        )
        .unwrap();
        fs::write(
            source.join("notarytool-rejected.log"),
            r#"{ "status": "Rejected" }"#,
        )
        .unwrap();
        fs::write(
            source.join("stapler-failed.log"),
            "status: Accepted\nstapler failed: ticket not found\n",
        )
        .unwrap();
        fs::write(source.join("notes.txt"), "ordinary artifact notes\n").unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("signing-macos.log").exists());
        assert!(!evidence.join("signing-windows.log").exists());
        assert!(!evidence.join("notary.log").exists());

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        let signing_failures = report
            .items
            .iter()
            .filter(|item| item.name == "signed bundle evidence" && item.status == "failed")
            .count();
        let notary_failures = report
            .items
            .iter()
            .filter(|item| item.name == "notarization log" && item.status == "failed")
            .count();
        assert_eq!(signing_failures, 2, "{:?}", report.items);
        assert_eq!(notary_failures, 2, "{:?}", report.items);
        assert!(report.items.iter().any(|item| {
            item.name == "text artifact"
                && item.status == "skipped"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "notes.txt"))
        }));
    }

    #[test]
    fn import_ci_rejects_generic_platformless_signing_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("signed-marker.log"), "signed=true\n").unwrap();
        fs::write(source.join("notes.txt"), "signature=ok\n").unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("signing-macos.log").exists());
        assert!(!evidence.join("signing-windows.log").exists());
        assert!(!evidence.join("signing").exists());

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "signed-marker.log")
                })
                && item.value.contains("no positive signing marker found")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "text artifact"
                && item.status == "skipped"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "notes.txt"))
        }));
    }

    #[test]
    fn import_ci_rejects_generic_notarization_acceptance_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("notary-generic.log"),
            "notarization=pass\nnotary=ok\nThe staple and validate action worked!\n",
        )
        .unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("notary.log").exists());
        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "notarization log"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "notary-generic.log")
                })
                && item
                    .value
                    .contains("accepted notarytool output and stapler success")
        }));
    }

    #[test]
    fn import_ci_rejects_notarization_artifact_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(source.join("Windows")).unwrap();
        fs::create_dir_all(source.join("Linux")).unwrap();
        fs::write(
            source.join("Windows/notary.log"),
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            source.join("Linux/stapler.log"),
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            source.join("notary-macos-windows.log"),
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("notary.log").exists());
        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "notarization log"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "Windows/notary.log")
                })
                && item.value.contains("artifact path indicates Windows")
                && item.value.contains("notarization evidence is macOS-only")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "notarization log"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "Linux/stapler.log")
                })
                && item.value.contains("artifact path indicates Linux")
                && item.value.contains("notarization evidence is macOS-only")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "notarization log"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "notary-macos-windows.log")
                })
                && item
                    .value
                    .contains("notarization evidence file name contains multiple platform labels")
                && item.value.contains("macOS")
                && item.value.contains("Windows")
        }));
    }

    #[test]
    fn import_ci_rejects_signing_artifact_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(source.join("Windows")).unwrap();
        fs::create_dir_all(source.join("signing")).unwrap();
        fs::create_dir_all(source.join("signing-artifacts")).unwrap();
        fs::write(source.join("Windows/signing.log"), "codesign=pass\n").unwrap();
        fs::write(
            source.join("signing/signing-macos-windows.log"),
            "codesign=pass\n",
        )
        .unwrap();
        fs::write(
            source.join("signing-artifacts/macos-codesign-output.log"),
            "codesign=pass\n",
        )
        .unwrap();
        fs::write(
            source.join("signing-artifacts/windows-signtool-output.log"),
            "signtool=pass\n",
        )
        .unwrap();
        let misplaced_bundle = source.join("Windows/VestyGain.vst3");
        fs::create_dir_all(misplaced_bundle.join("Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&misplaced_bundle);

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert_eq!(
            fs::read_to_string(evidence.join("signing-macos.log")).unwrap(),
            "codesign=pass\n"
        );
        assert_eq!(
            fs::read_to_string(evidence.join("signing-windows.log")).unwrap(),
            "signtool=pass\n"
        );
        assert!(!evidence.join("signing/signing-macos-windows.log").exists());
        assert!(!evidence.join("signed-bundles/VestyGain.vst3").exists());

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "Windows/signing.log")
                })
                && item.value.contains("artifact path indicates Windows")
                && item.value.contains("signing evidence platform is macOS")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "signing/signing-macos-windows.log")
                })
                && item
                    .value
                    .contains("signing evidence file name contains multiple platform labels")
                && item.value.contains("macOS")
                && item.value.contains("Windows")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_path_ends_with(source, "Windows/VestyGain.vst3")
                })
                && item.value.contains("artifact path indicates Windows")
                && item.value.contains("signing evidence platform is macOS")
        }));
    }

    #[test]
    fn import_ci_reports_failed_signed_bundle_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        let valid_bundle = source.join("macos/VestyGain.vst3");
        let placeholder_bundle = source.join("macos/VestyPlaceholder.vst3");
        let missing_code_resources_bundle = source.join("macos/VestyMissingCodeResources.vst3");

        fs::create_dir_all(valid_bundle.join("Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&valid_bundle);
        fs::create_dir_all(placeholder_bundle.join("Contents/_CodeSignature")).unwrap();
        fs::write(
            placeholder_bundle.join("Contents/_CodeSignature/CodeResources"),
            "signed bundle marker\n",
        )
        .unwrap();
        fs::create_dir_all(missing_code_resources_bundle.join("Contents/_CodeSignature")).unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(
            evidence
                .join("signed-bundles/VestyGain.vst3/Contents/_CodeSignature/CodeResources")
                .is_file()
        );
        assert!(
            !evidence
                .join("signed-bundles/VestyPlaceholder.vst3")
                .exists()
        );
        assert!(
            !evidence
                .join("signed-bundles/VestyMissingCodeResources.vst3")
                .exists()
        );

        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "imported"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_paths_equal(source, valid_bundle.as_str()))
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_paths_equal(source, placeholder_bundle.as_str())
                })
                && item.path.is_none()
                && item
                    .value
                    .contains("CodeResources is not a parseable plist")
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "signed bundle evidence"
                && item.status == "failed"
                && item.source.as_deref().is_some_and(|source| {
                    release_report_paths_equal(source, missing_code_resources_bundle.as_str())
                })
                && item.path.is_none()
                && item
                    .value
                    .contains("missing Contents/_CodeSignature/CodeResources")
        }));
    }

    #[test]
    fn import_ci_release_evidence_rejects_overlapping_source_and_output_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence_inside_source = source.join("release-evidence");
        let source_inside_evidence = root.join("release-evidence/downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&source_inside_evidence).unwrap();

        let output_inside_source = import_ci_release_evidence(ImportCiOptions {
            source: source.clone(),
            dir: evidence_inside_source,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .expect_err("release evidence dir inside source should be rejected")
        .to_string();

        let source_inside_output = import_ci_release_evidence(ImportCiOptions {
            source: source_inside_evidence,
            dir: evidence,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .expect_err("source inside release evidence dir should be rejected")
        .to_string();

        assert!(output_inside_source.contains("must not be inside CI artifact source"));
        assert!(source_inside_output.contains("must not be inside release evidence dir"));
    }

    #[cfg(unix)]
    #[test]
    fn import_ci_release_evidence_rejects_symlink_source_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external_source = root.join("external-downloaded-artifacts");
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&external_source).unwrap();
        unix_fs::symlink(&external_source, &source).unwrap();

        let error = import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .expect_err("import-ci source root symlink should be rejected")
        .to_string();

        assert!(error.contains("CI artifact source must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn import_ci_release_evidence_rejects_existing_symlink_output_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let external_evidence = root.join("external-release-evidence");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&external_evidence).unwrap();
        unix_fs::symlink(&external_evidence, &evidence).unwrap();

        let error = import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .expect_err("import-ci output dir symlink should be rejected")
        .to_string();

        assert!(error.contains("release evidence dir must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn import_ci_release_evidence_rejects_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let external_parent = root.join("external-release-evidence-parent");
        let parent_link = root.join("release-evidence-parent");
        let evidence = parent_link.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir(&external_parent).unwrap();
        unix_fs::symlink(&external_parent, &parent_link).unwrap();

        let error = import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .expect_err("import-ci output parent symlink should be rejected before creation")
        .to_string();

        assert!(error.contains("release evidence dir parent must not be a symlink"));
        assert!(!external_parent.join("release-evidence").exists());
    }

    #[cfg(unix)]
    #[test]
    fn import_ci_artifact_readers_reject_symlink_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();

        let options = ImportCiOptions {
            source: source.clone(),
            dir: evidence,
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };

        let external_json = root.join("external-release-check.json");
        write_ci_release_check_artifact(&external_json);
        let json_link = source.join("release-check.json");
        unix_fs::symlink(&external_json, &json_link).unwrap();
        let mut items = Vec::new();
        let error = import_ci_json_artifact(&json_link, &options, None, &mut items)
            .expect_err("import-ci JSON artifact reader should reject symlink files")
            .to_string();
        assert!(error.contains("CI JSON artifact must not be a symlink"));
        assert!(items.is_empty());

        let external_rust = root.join("external-generated.rs");
        write_test_vst3_sdk_scaffold(
            &external_rust,
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let rust_link = source.join("generated.rs");
        unix_fs::symlink(&external_rust, &rust_link).unwrap();
        let mut items = Vec::new();
        let error = import_ci_rust_artifact(&rust_link, &options, &mut items)
            .expect_err("import-ci Rust artifact reader should reject symlink files")
            .to_string();
        assert!(error.contains("CI Rust artifact must not be a symlink"));
        assert!(items.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn import_ci_writers_reject_symlink_output_parents() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source_file = root.join("source.log");
        let source_dir = root.join("source-bundle");
        fs::write(&source_file, "codesign=pass\n").unwrap();
        fs::create_dir_all(source_dir.join("Contents")).unwrap();
        fs::write(source_dir.join("Contents/moduleinfo.json"), "{}\n").unwrap();

        let evidence = root.join("release-evidence");
        fs::create_dir(&evidence).unwrap();

        let external_copy = root.join("external-copy");
        fs::create_dir(&external_copy).unwrap();
        unix_fs::symlink(&external_copy, evidence.join("copy")).unwrap();
        let copy_destination = evidence.join("copy/signing-macos.log");
        let error = import_copy_file(&source_file, &copy_destination, false)
            .expect_err("import copy should reject symlinked output parents")
            .to_string();
        assert!(error.contains("CI artifact import destination parent must not be a symlink"));
        assert!(!external_copy.join("signing-macos.log").exists());

        let external_write = root.join("external-write");
        fs::create_dir(&external_write).unwrap();
        unix_fs::symlink(&external_write, evidence.join("write")).unwrap();
        let write_destination = evidence.join("write/ci-run-url.txt");
        let error = import_write_text_file(
            &write_destination,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/42\n",
            false,
        )
        .expect_err("import write should reject symlinked output parents")
        .to_string();
        assert!(error.contains("CI artifact import destination parent must not be a symlink"));
        assert!(!external_write.join("ci-run-url.txt").exists());

        let external_dir = root.join("external-dir");
        fs::create_dir(&external_dir).unwrap();
        unix_fs::symlink(&external_dir, evidence.join("signed-bundles")).unwrap();
        let dir_destination = evidence.join("signed-bundles/VestyGain.vst3");
        let error = import_copy_dir_contents(&source_dir, &dir_destination, false)
            .expect_err("import directory copy should reject symlinked output parents")
            .to_string();
        assert!(error.contains("CI artifact import destination parent must not be a symlink"));
        assert!(!external_dir.join("VestyGain.vst3").exists());
    }

    #[cfg(unix)]
    #[test]
    fn import_overwrite_unlinks_destination_symlink_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("source.log");
        let external_dir = root.join("external-target");
        let destination = root.join("release-evidence/signing-macos.log");
        fs::create_dir_all(&external_dir).unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&source, "codesign=pass\n").unwrap();
        fs::write(external_dir.join("keep.txt"), "do not remove\n").unwrap();
        unix_fs::symlink(&external_dir, &destination).unwrap();

        let outcome = import_copy_file(&source, &destination, true).unwrap();

        assert_eq!(outcome, ImportWriteOutcome::Imported);
        assert_eq!(fs::read_to_string(&destination).unwrap(), "codesign=pass\n");
        assert!(external_dir.join("keep.txt").is_file());
        assert!(
            !fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[test]
    fn import_overwrite_unlinks_dangling_destination_symlink_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("source.log");
        let external_dir = root.join("external-target");
        let external_target = external_dir.join("escape.log");
        let destination = root.join("release-evidence/signing-macos.log");
        fs::create_dir_all(&external_dir).unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&source, "codesign=pass\n").unwrap();
        unix_fs::symlink(&external_target, &destination).unwrap();

        let outcome = import_copy_file(&source, &destination, true).unwrap();

        assert_eq!(outcome, ImportWriteOutcome::Imported);
        assert_eq!(fs::read_to_string(&destination).unwrap(), "codesign=pass\n");
        assert!(!external_target.exists());
        assert!(
            !fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn strict_signing_evidence_requires_macos_and_windows_coverage() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let macos = root.join("codesign.log");
        let windows = root.join("signtool.log");
        fs::write(&macos, "codesign=pass\n").unwrap();
        fs::write(&windows, "signtool=pass\n").unwrap();

        let partial = signed_bundle_evidence_release_check(std::slice::from_ref(&macos), true);
        assert_eq!(partial.status, "failed");
        assert!(partial.value.contains("Windows signtool"));

        let optional = signed_bundle_evidence_release_check(std::slice::from_ref(&macos), false);
        assert_eq!(optional.status, "ok");

        let complete = signed_bundle_evidence_release_check(&[macos, windows], true);
        assert_eq!(complete.status, "ok");
        assert!(complete.value.contains("macOS"));
        assert!(complete.value.contains("Windows"));
    }

    #[test]
    fn signing_evidence_rejects_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let swapped_macos = root.join("signing-macos.log");
        let swapped_windows = root.join("Windows/signing.log");
        fs::create_dir_all(swapped_windows.parent().unwrap()).unwrap();
        fs::write(&swapped_macos, "signtool=pass\n").unwrap();
        fs::write(&swapped_windows, "codesign=pass\n").unwrap();

        let check = signed_bundle_evidence_release_check(&[swapped_macos, swapped_windows], true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("artifact path indicates macOS"));
        assert!(check.value.contains("signing evidence platform is Windows"));
        assert!(check.value.contains("artifact path indicates Windows"));
        assert!(check.value.contains("signing evidence platform is macOS"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_evidence_rejects_symlink_log() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-codesign.log");
        let log = root.join("codesign.log");
        fs::write(&external, "codesign=pass\n").unwrap();
        unix_fs::symlink(&external, &log).unwrap();

        let check = signed_bundle_evidence_release_check(&[log], true);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("signing evidence path must not be a symlink")
        );
    }

    #[test]
    fn signing_evidence_accepts_parseable_macos_code_resources_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        fs::create_dir_all(bundle.join("Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&bundle);

        let platforms = signing_evidence_platforms(&bundle).unwrap();

        assert!(platforms.contains(&SigningEvidencePlatform::Macos));
    }

    #[test]
    fn signing_evidence_rejects_placeholder_code_resources_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        fs::create_dir_all(bundle.join("Contents/_CodeSignature")).unwrap();
        fs::write(
            bundle.join("Contents/_CodeSignature/CodeResources"),
            "signed bundle marker\n",
        )
        .unwrap();

        let error = signing_evidence_platforms(&bundle).unwrap_err().to_string();

        assert!(error.contains("CodeResources is not a parseable plist"));
    }

    #[test]
    fn signing_evidence_rejects_code_resources_without_file_dictionary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        fs::create_dir_all(bundle.join("Contents/_CodeSignature")).unwrap();
        fs::write(
            bundle.join("Contents/_CodeSignature/CodeResources"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>files2</key>
  <string>placeholder</string>
</dict>
</plist>
"#,
        )
        .unwrap();

        let error = signing_evidence_platforms(&bundle).unwrap_err().to_string();

        assert!(error.contains("files or files2 dictionary entries"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_evidence_rejects_symlinked_code_resources() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let signature_dir = bundle.join("Contents/_CodeSignature");
        fs::create_dir_all(&signature_dir).unwrap();
        let external = root.join("external-CodeResources");
        fs::create_dir_all(root.join("external-bundle.vst3/Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&root.join("external-bundle.vst3"));
        fs::rename(
            root.join("external-bundle.vst3/Contents/_CodeSignature/CodeResources"),
            &external,
        )
        .unwrap();
        unix_fs::symlink(&external, signature_dir.join("CodeResources")).unwrap();

        let error = signing_evidence_platforms(&bundle).unwrap_err().to_string();

        assert!(error.contains("macOS CodeResources must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_evidence_rejects_symlinked_code_signature_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let contents = bundle.join("Contents");
        let external = root.join("external-CodeSignature");
        fs::create_dir_all(&contents).unwrap();
        fs::create_dir(&external).unwrap();
        fs::create_dir_all(root.join("external-bundle.vst3/Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&root.join("external-bundle.vst3"));
        fs::rename(
            root.join("external-bundle.vst3/Contents/_CodeSignature/CodeResources"),
            external.join("CodeResources"),
        )
        .unwrap();
        unix_fs::symlink(&external, contents.join("_CodeSignature")).unwrap();

        let error = signing_evidence_platforms(&bundle).unwrap_err().to_string();

        assert!(error.contains("macOS CodeSignature directory must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn signing_evidence_rejects_symlinked_contents_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("VestyGain.vst3");
        let external_bundle = root.join("external-bundle.vst3");
        fs::create_dir(&bundle).unwrap();
        fs::create_dir_all(external_bundle.join("Contents/_CodeSignature")).unwrap();
        write_code_resources_plist(&external_bundle);
        unix_fs::symlink(external_bundle.join("Contents"), bundle.join("Contents")).unwrap();

        let error = signing_evidence_platforms(&bundle).unwrap_err().to_string();

        assert!(error.contains("macOS bundle Contents must not be a symlink"));
    }

    #[test]
    fn signing_evidence_accepts_signtool_summary_success() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("signtool-summary.log");
        fs::write(
            &log,
            "signtool verify /pa /v VestyGain.vst3\n\
             Number of files successfully Verified: 1\n\
             Number of warnings: 0\n\
             Number of errors: 0\n",
        )
        .unwrap();

        assert!(validate_signing_evidence(&log).is_ok());
    }

    #[test]
    fn signing_evidence_rejects_signtool_sign_without_verify() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("signtool-sign-only.log");
        fs::write(
            &log,
            "signtool sign /fd SHA256 VestyGain.vst3\nSuccessfully signed: VestyGain.vst3\n",
        )
        .unwrap();

        let error = signing_evidence_platforms(&log)
            .expect_err("signtool sign output should not prove verification evidence")
            .to_string();
        assert!(error.contains("no positive signing marker found"));
    }

    #[test]
    fn signing_evidence_accepts_exact_colon_truthy_marker() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("codesign-marker.log");
        fs::write(&log, "codesign: pass\n").unwrap();

        assert!(validate_signing_evidence(&log).is_ok());
    }

    #[test]
    fn signing_evidence_rejects_substring_and_instructional_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let substring = root.join("substring.log");
        let instructional = root.join("instructional.log");
        fs::write(&substring, "unsigned=true\nsignature=pending\n").unwrap();
        fs::write(
            &instructional,
            "note: signed=true is the marker to paste after real verification\n",
        )
        .unwrap();

        assert!(validate_signing_evidence(&substring).is_err());
        assert!(validate_signing_evidence(&instructional).is_err());
    }

    #[test]
    fn signing_evidence_rejects_generic_platformless_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let signed = root.join("signed-marker.log");
        let signature = root.join("signature-marker.log");
        fs::write(&signed, "signed=true\n").unwrap();
        fs::write(&signature, "signature=ok\nsigning=pass\n").unwrap();

        let signed_error = validate_signing_evidence(&signed).unwrap_err().to_string();
        let signature_error = validate_signing_evidence(&signature)
            .unwrap_err()
            .to_string();
        let optional_check = signed_bundle_evidence_release_check(&[signed], false);

        assert!(signed_error.contains("no positive signing marker found"));
        assert!(signature_error.contains("no positive signing marker found"));
        assert_eq!(optional_check.status, "failed");
        assert!(
            optional_check
                .value
                .contains("no positive signing marker found")
        );
    }

    #[test]
    fn signing_evidence_rejects_signtool_summary_errors() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("signtool-summary-failed.log");
        fs::write(
            &log,
            "signtool verify /pa /v VestyGain.vst3\n\
             SignTool Error: A certificate chain processed, but terminated in a root certificate\n\
             Number of errors: 1\n",
        )
        .unwrap();

        assert!(validate_signing_evidence(&log).is_err());
    }

    #[test]
    fn signing_evidence_rejects_contradictory_success_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let codesign = root.join("codesign-contradictory.log");
        let signtool = root.join("signtool-contradictory.log");
        let inline_codesign = root.join("codesign-inline-contradictory.log");
        let inline_signtool = root.join("signtool-inline-contradictory.log");
        fs::write(
            &codesign,
            "codesign=pass\nVestyGain.vst3: invalid signature\n",
        )
        .unwrap();
        fs::write(&signtool, "signtool=pass\nNumber of errors: 1\n").unwrap();
        fs::write(&inline_codesign, "codesign=pass; codesign=false\n").unwrap();
        fs::write(&inline_signtool, "signtool=pass; signtool=failed\n").unwrap();

        let codesign_error = validate_signing_evidence(&codesign)
            .expect_err("contradictory codesign output should fail")
            .to_string();
        let signtool_error = validate_signing_evidence(&signtool)
            .expect_err("contradictory signtool output should fail")
            .to_string();
        let inline_codesign_error = validate_signing_evidence(&inline_codesign)
            .expect_err("inline contradictory codesign markers should fail")
            .to_string();
        let inline_signtool_error = validate_signing_evidence(&inline_signtool)
            .expect_err("inline contradictory signtool markers should fail")
            .to_string();

        assert!(codesign_error.contains("negative signing evidence"));
        assert!(signtool_error.contains("negative signing evidence"));
        assert!(inline_codesign_error.contains("negative signing evidence"));
        assert!(inline_signtool_error.contains("negative signing evidence"));
    }

    #[test]
    fn signing_and_notarization_evidence_reject_malformed_log_text() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let signing_nul = root.join("codesign-nul.log");
        fs::write(&signing_nul, "codesign=pass\0\n").unwrap();
        let error = validate_signing_evidence(&signing_nul)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("signing evidence log must not contain control characters"),
            "{error}"
        );

        let signing_unsafe = root.join("codesign-unsafe.log");
        fs::write(&signing_unsafe, "codesign=pass\u{202e}hidden\n").unwrap();
        let error = validate_signing_evidence(&signing_unsafe)
            .unwrap_err()
            .to_string();
        assert!(
            error
                .contains("signing evidence log must not contain unsafe Unicode format characters"),
            "{error}"
        );

        let signing_too_large = root.join("codesign-too-large.log");
        fs::write(
            &signing_too_large,
            format!(
                "codesign=pass\n{}",
                "x".repeat(SIGNING_NOTARIZATION_LOG_MAX_BYTES)
            ),
        )
        .unwrap();
        let error = validate_signing_evidence(&signing_too_large)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("signing evidence log must be at most"),
            "{error}"
        );

        let notary_nul = root.join("notary-nul.log");
        fs::write(
            &notary_nul,
            "status: Accepted\nThe staple and validate action worked!\0\n",
        )
        .unwrap();
        let error = validate_notarization_evidence(&notary_nul)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("notarization evidence log must not contain control characters"),
            "{error}"
        );

        let notary_unsafe = root.join("notary-unsafe.log");
        fs::write(
            &notary_unsafe,
            "status: Accepted\nThe staple and validate action worked!\u{202e}hidden\n",
        )
        .unwrap();
        let error = validate_notarization_evidence(&notary_unsafe)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(
                "notarization evidence log must not contain unsafe Unicode format characters"
            ),
            "{error}"
        );

        let notary_too_large = root.join("notary-too-large.log");
        fs::write(
            &notary_too_large,
            format!(
                "status: Accepted\nThe staple and validate action worked!\n{}",
                "x".repeat(SIGNING_NOTARIZATION_LOG_MAX_BYTES)
            ),
        )
        .unwrap();
        let error = validate_notarization_evidence(&notary_too_large)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("notarization evidence log must be at most"),
            "{error}"
        );
    }

    #[test]
    fn notarization_evidence_rejects_pending_and_false_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("notary-pending.log");
        fs::write(
            &log,
            "notarization=pending\nstapled: false\nstatus: rejected\n",
        )
        .unwrap();

        assert!(validate_notarization_evidence(&log).is_err());
    }

    #[test]
    fn notarization_evidence_rejects_generic_acceptance_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let generic = root.join("notary-generic.log");
        let prose = root.join("notary-prose.log");
        let notarytool = root.join("notarytool-marker.log");
        fs::write(
            &generic,
            "notarization=pass\nnotary=ok\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            &prose,
            "note: paste status: accepted from real notarytool output here\n\
             note: paste The staple and validate action worked! from real stapler output here\n",
        )
        .unwrap();
        fs::write(
            &notarytool,
            "notarytool=pass\nThe staple and validate action worked!\n",
        )
        .unwrap();

        let generic_evidence =
            notarization_evidence(&generic).expect("stapler output remains positive evidence");
        let strict_check = notarization_log_release_check(Some(&generic), true);
        let prose_check = notarization_log_release_check(Some(&prose), true);
        let prose_error = notarization_evidence(&prose).unwrap_err().to_string();

        assert!(!generic_evidence.accepted);
        assert!(generic_evidence.stapled);
        assert!(prose_error.contains("no accepted notarization marker found"));
        assert_eq!(strict_check.status, "failed");
        assert!(strict_check.value.contains("accepted notarytool result"));
        assert_eq!(prose_check.status, "failed");
        assert!(
            prose_check
                .value
                .contains("no accepted notarization marker found")
        );
        assert!(validate_notarization_evidence(&notarytool).is_ok());
    }

    #[test]
    fn notarization_evidence_rejects_contradictory_success_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let rejected = root.join("notary-contradictory.log");
        let stapler_failed = root.join("stapler-contradictory.log");
        let inline_rejected = root.join("notary-inline-contradictory.log");
        let inline_stapled = root.join("stapler-inline-contradictory.log");
        fs::write(
            &rejected,
            "status: Accepted\nstatus: Rejected\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            &stapler_failed,
            "status: Accepted\nstapled=true\nstapler failed: ticket not found\n",
        )
        .unwrap();
        fs::write(
            &inline_rejected,
            "status: Accepted; status: Rejected\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            &inline_stapled,
            "status: Accepted\nstapled=true; stapled=false\n",
        )
        .unwrap();

        let rejected_error = validate_notarization_evidence(&rejected)
            .expect_err("rejected notary output should fail")
            .to_string();
        let stapler_error = validate_notarization_evidence(&stapler_failed)
            .expect_err("failed stapler output should fail")
            .to_string();
        let inline_rejected_error = validate_notarization_evidence(&inline_rejected)
            .expect_err("inline rejected notary markers should fail")
            .to_string();
        let inline_stapled_error = validate_notarization_evidence(&inline_stapled)
            .expect_err("inline stapled markers should fail")
            .to_string();

        assert!(rejected_error.contains("negative notarization evidence"));
        assert!(stapler_error.contains("negative notarization evidence"));
        assert!(inline_rejected_error.contains("negative notarization evidence"));
        assert!(inline_stapled_error.contains("negative notarization evidence"));
    }

    #[test]
    fn notarization_evidence_accepts_notarytool_json_status() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("notarytool.json");
        let collected = root.join("notary-collected.log");
        let prose = root.join("notarytool-json-prose.log");
        fs::write(
            &log,
            "{\n  \"id\": \"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\",\n  \"status\": \"Accepted\"\n}\n",
        )
        .unwrap();
        fs::write(
            &collected,
            "notary_log=/tmp/notarytool.json\n\n[notarytool]\n{\n  \"status\": \"Accepted\"\n}\n\nstapler_log=/tmp/stapler.log\n\n[stapler]\nThe staple and validate action worked!\n",
        )
        .unwrap();
        fs::write(
            &prose,
            "note: paste {\"status\":\"Accepted\"} from real notarytool JSON here\n",
        )
        .unwrap();

        assert!(validate_notarization_evidence(&log).is_ok());
        assert!(validate_notarization_evidence(&collected).is_ok());
        let error = validate_notarization_evidence(&prose)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no accepted notarization marker found"));
    }

    #[test]
    fn notarization_evidence_rejects_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let log = root.join("Windows/notary.log");
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        fs::write(
            &log,
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();

        let check = notarization_log_release_check(Some(&log), true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("artifact path indicates Windows"));
        assert!(check.value.contains("notarization evidence is macOS-only"));

        let ambiguous = root.join("notary-macos-windows.log");
        fs::write(
            &ambiguous,
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        let ambiguous_check = notarization_log_release_check(Some(&ambiguous), true);

        assert_eq!(ambiguous_check.status, "failed");
        assert!(
            ambiguous_check
                .value
                .contains("notarization evidence file name contains multiple platform labels")
        );
        assert!(ambiguous_check.value.contains("macOS"));
        assert!(ambiguous_check.value.contains("Windows"));
    }

    #[test]
    fn strict_notarization_requires_accepted_and_stapled_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let accepted = root.join("notarytool.json");
        let stapled = root.join("stapler.log");
        let complete = root.join("notary-complete.log");
        fs::write(&accepted, r#"{ "status": "Accepted" }"#).unwrap();
        fs::write(&stapled, "The staple and validate action worked!\n").unwrap();
        fs::write(
            &complete,
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();

        let optional = notarization_log_release_check(Some(&accepted), false);
        assert_eq!(optional.status, "ok");

        let missing_staple = notarization_log_release_check(Some(&accepted), true);
        assert_eq!(missing_staple.status, "failed");
        assert!(missing_staple.value.contains("stapler success"));

        let missing_accepted = notarization_log_release_check(Some(&stapled), true);
        assert_eq!(missing_accepted.status, "failed");
        assert!(
            missing_accepted
                .value
                .contains("accepted notarytool result")
        );

        let strict = notarization_log_release_check(Some(&complete), true);
        assert_eq!(strict.status, "ok");
        assert!(strict.value.contains("accepted=true"));
        assert!(strict.value.contains("stapled=true"));
    }

    #[test]
    fn explicit_truthy_marker_rejects_substring_keys_and_false_values() {
        assert!(!explicit_truthy_marker("rescan=true\n", &["scan"]));
        assert!(!explicit_truthy_marker("scan=false\n", &["scan"]));
        assert!(!explicit_truthy_marker(
            "note: scan=true after smoke\n",
            &["scan"]
        ));
        assert!(explicit_truthy_marker("scan: ok\n", &["scan"]));
        assert!(explicit_truthy_marker("scan_ok=pass\n", &["scan"]));
    }

    #[test]
    fn generic_daw_evidence_accepts_explicit_smoke_markers() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("platform.txt"), "macOS arm64\n").unwrap();
        fs::write(dir.join("scan-smoke.log"), "scan=true\n").unwrap();
        fs::write(dir.join("load-smoke.log"), "load=pass\n").unwrap();
        fs::write(dir.join("ui-smoke.log"), "ui_ok=true\n").unwrap();
        fs::write(dir.join("ui-host-smoke.log"), "ui_host_param=true\n").unwrap();
        fs::write(dir.join("meter-stream.log"), "meter_flush sent=1\n").unwrap();
        fs::write(dir.join("automation-smoke.log"), "automation=true\n").unwrap();
        fs::write(
            dir.join("buffer-sample-rate.log"),
            "buffer_sample_rate_change=true\n",
        )
        .unwrap();
        fs::write(dir.join("restore-smoke.log"), "save_restore=pass\n").unwrap();
        fs::write(dir.join("offline-render.log"), "offline_render=ok\n").unwrap();

        let row = collect_generic_daw_evidence("Bitwig Studio", &dir);
        assert_eq!(row["platform"], "macOS arm64");
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
            assert_eq!(row[key], true, "{key}");
        }
    }

    #[test]
    fn generic_daw_evidence_rejects_contradictory_positive_markers() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("platform.txt"), "macOS arm64\n").unwrap();
        fs::write(
            dir.join("scan-smoke.log"),
            "scan=true\nscan failed after rescan\n",
        )
        .unwrap();
        fs::write(dir.join("load-smoke.log"), "load=true\nload=false\n").unwrap();
        fs::write(dir.join("ui-smoke.log"), "ui=true\nui timeout\n").unwrap();
        fs::write(
            dir.join("ui-host-smoke.log"),
            "ui_host_param=true\nhost_param=pending\n",
        )
        .unwrap();
        fs::write(
            dir.join("meter-stream.log"),
            "meter_flush sent=3\nmeter stream failed\n",
        )
        .unwrap();
        fs::write(
            dir.join("automation-smoke.log"),
            "automation=true\nautomation error\n",
        )
        .unwrap();
        fs::write(
            dir.join("buffer-sample-rate.log"),
            "buffer_sample_rate_change=true\nsample_rate_change=false\n",
        )
        .unwrap();
        fs::write(
            dir.join("restore-smoke.log"),
            "save_restore=true\nrestore failed\n",
        )
        .unwrap();
        fs::write(
            dir.join("offline-render.log"),
            "offline_render=true\nrender failed\n",
        )
        .unwrap();

        let row = collect_generic_daw_evidence("Bitwig Studio", &dir);
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
            assert_eq!(row[key], false, "{key}");
        }
    }

    #[test]
    fn daw_evidence_rejects_explicit_host_scope_mismatch_markers() {
        let bitwig = vesty_core::find_host_profile("bitwig").unwrap();
        let ableton = vesty_core::find_host_profile("ableton").unwrap();

        assert!(daw_marker_host_scope_matches(
            bitwig,
            "host=Bitwig Studio\nscan=true\n"
        ));
        assert!(daw_marker_host_scope_matches(
            bitwig,
            "daw = bitwig-studio; scan=true\n"
        ));
        assert!(!daw_marker_host_scope_matches(
            bitwig,
            "host=Ableton Live\nscan=true\n"
        ));
        assert!(!daw_marker_host_scope_matches(
            bitwig,
            "host=Unknown Host\nscan=true\n"
        ));
        assert!(daw_marker_host_scope_matches(ableton, "scan=true\n"));

        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("platform.txt"), "macOS arm64\n").unwrap();
        fs::write(dir.join("scan-smoke.log"), "host=Ableton Live\nscan=true\n").unwrap();
        fs::write(dir.join("load-smoke.log"), "host=Ableton Live\nload=true\n").unwrap();
        fs::write(dir.join("ui-smoke.log"), "host=Ableton Live\nui=true\n").unwrap();
        fs::write(
            dir.join("ui-host-smoke.log"),
            "host=Ableton Live\nui_host_param=true\n",
        )
        .unwrap();
        fs::write(
            dir.join("meter-stream.log"),
            "host=Ableton Live\nmeter_flush sent=3\n",
        )
        .unwrap();
        fs::write(
            dir.join("automation-smoke.log"),
            "host=Ableton Live\nautomation=true\n",
        )
        .unwrap();
        fs::write(
            dir.join("buffer-sample-rate.log"),
            "host=Ableton Live\nbuffer_sample_rate_change=true\n",
        )
        .unwrap();
        fs::write(
            dir.join("restore-smoke.log"),
            "host=Ableton Live\nsave_restore=true\n",
        )
        .unwrap();
        fs::write(
            dir.join("offline-render.log"),
            "host=Ableton Live\noffline_render=true\n",
        )
        .unwrap();

        let row = collect_generic_daw_evidence("Bitwig Studio", &dir);
        assert_eq!(row["platform_supported"], true);
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
            assert_eq!(row[key], false, "{key}");
        }
    }

    #[test]
    fn daw_evidence_rejects_contradictory_bridge_trace_markers() {
        let temp = tempfile::tempdir().unwrap();
        let generic_dir = Utf8PathBuf::from_path_buf(temp.path().join("generic")).unwrap();
        let reaper_dir = Utf8PathBuf::from_path_buf(temp.path().join("reaper")).unwrap();
        fs::create_dir_all(&generic_dir).unwrap();
        fs::create_dir_all(&reaper_dir).unwrap();
        let trace = r#"
ipc: {"type":"param.begin"}
ipc: {"type":"param.perform"}
ipc: {"type":"param.end"}
packet: {"lane":"meter","type":"meter.main","payload":{"peaks":[0.75],"rms":[0.5]}}
relay: ok result=0
bridge timeout
"#;
        fs::write(generic_dir.join("platform.txt"), "macOS arm64\n").unwrap();
        fs::write(generic_dir.join("bridge-trace.log"), trace).unwrap();
        fs::write(reaper_dir.join("bridge-trace.log"), trace).unwrap();

        let generic = collect_generic_daw_evidence("Bitwig Studio", &generic_dir);
        assert_eq!(generic["ui_host_param"], false);
        assert_eq!(generic["meter_stream"], false);

        let reaper = collect_reaper_evidence(&reaper_dir);
        assert_eq!(reaper["ui_host_param"], false);
        assert_eq!(reaper["meter_stream"], false);
    }

    #[cfg(unix)]
    #[test]
    fn generic_daw_evidence_ignores_symlinked_marker_files() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        fs::create_dir_all(&dir).unwrap();

        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-marker.log")).unwrap();
        fs::write(
            &external,
            "macOS arm64\nscan=true\nload=true\nui=true\nui_host_param=true\nmeter_flush sent=5\nautomation=true\nbuffer_sample_rate_change=true\nsave_restore=true\noffline_render=true\n",
        )
        .unwrap();
        for file in [
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
            unix_fs::symlink(&external, dir.join(file)).unwrap();
        }

        let row = collect_generic_daw_evidence("Bitwig Studio", &dir);
        assert_eq!(row["platform"], "manual evidence");
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
            assert_eq!(row[key], false, "{key}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn generic_daw_evidence_ignores_symlinked_evidence_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let external_root = Utf8PathBuf::from_path_buf(temp.path().join("external-root")).unwrap();
        let external_bitwig = external_root.join("bitwig");
        fs::create_dir_all(&external_bitwig).unwrap();
        fs::write(external_bitwig.join("platform.txt"), "macOS arm64\n").unwrap();
        fs::write(external_bitwig.join("scan-smoke.log"), "scan=true\n").unwrap();
        fs::write(external_bitwig.join("load-smoke.log"), "load=true\n").unwrap();
        fs::write(external_bitwig.join("ui-smoke.log"), "ui=true\n").unwrap();
        fs::write(
            external_bitwig.join("ui-host-smoke.log"),
            "ui_host_param=true\n",
        )
        .unwrap();
        fs::write(
            external_bitwig.join("meter-stream.log"),
            "meter_flush sent=5\n",
        )
        .unwrap();
        fs::write(
            external_bitwig.join("automation-smoke.log"),
            "automation=true\n",
        )
        .unwrap();
        fs::write(
            external_bitwig.join("buffer-sample-rate.log"),
            "buffer_sample_rate_change=true\n",
        )
        .unwrap();
        fs::write(
            external_bitwig.join("restore-smoke.log"),
            "save_restore=true\n",
        )
        .unwrap();
        fs::write(
            external_bitwig.join("offline-render.log"),
            "offline_render=true\n",
        )
        .unwrap();
        unix_fs::symlink(&external_root, &root).unwrap();

        let row = collect_generic_daw_evidence("Bitwig Studio", &root.join("bitwig"));
        assert_eq!(row["platform"], "manual matrix pending");
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
            assert_eq!(row[key], false, "{key}");
        }
    }

    fn test_daw_smoke_report_input(host: &str) -> DawSmokeReportInput {
        DawSmokeReportInput {
            host: Some(host.to_string()),
            platform: Some("macOS arm64 / host smoke".to_string()),
            scan: Some("VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3".to_string()),
            load: Some("load=true".to_string()),
            ui: Some("ui=true".to_string()),
            ui_host_param: Some("ui_host_param=true".to_string()),
            meter_stream: Some("meter_flush sent=3".to_string()),
            automation: Some("automation=true".to_string()),
            buffer_sample_rate_change: Some("buffer_sample_rate_change=true".to_string()),
            save_restore: Some("save_restore=true".to_string()),
            offline_render: Some("offline_render=true".to_string()),
        }
    }

    #[test]
    fn daw_matrix_write_report_writes_and_validates_host_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let path = write_daw_smoke_report(&paths, test_daw_smoke_report_input("bitwig")).unwrap();
        assert_eq!(path, paths.bitwig);
        for file in [
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
            assert!(paths.bitwig.join(file).is_file(), "{file}");
        }

        let row = collect_generic_daw_evidence("Bitwig Studio", &paths.bitwig);
        assert!(daw_row_complete(&row));
        assert_eq!(row["platform"], "macOS arm64 / host smoke");
    }

    #[test]
    fn daw_matrix_write_report_validates_host_platform_scope() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let mut bitwig_x11 = test_daw_smoke_report_input("bitwig");
        bitwig_x11.platform = Some("Linux X11 x86_64 / Bitwig smoke".to_string());
        let path = write_daw_smoke_report(&paths, bitwig_x11).unwrap();
        assert_eq!(path, paths.bitwig);
        assert_eq!(
            fs::read_to_string(paths.bitwig.join("platform.txt")).unwrap(),
            "Linux X11 x86_64 / Bitwig smoke\n"
        );

        let mut bitwig_wayland = test_daw_smoke_report_input("bitwig");
        bitwig_wayland.platform = Some("Linux Wayland / Bitwig smoke".to_string());
        let error = write_daw_smoke_report(&paths, bitwig_wayland)
            .expect_err("Wayland DAW smoke should not satisfy MVP platform scope")
            .to_string();
        assert!(error.contains("platform"));
        assert!(error.contains("supported"));
        assert_eq!(
            fs::read_to_string(paths.bitwig.join("platform.txt")).unwrap(),
            "Linux X11 x86_64 / Bitwig smoke\n"
        );

        let mut ableton_linux = test_daw_smoke_report_input("ableton");
        ableton_linux.platform = Some("Linux X11 / Ableton smoke".to_string());
        let error = write_daw_smoke_report(&paths, ableton_linux)
            .expect_err("Ableton profile does not support Linux")
            .to_string();
        assert!(error.contains("not supported by Ableton Live profile"));
        assert!(!paths.ableton.join("platform.txt").exists());

        let mut vague = test_daw_smoke_report_input("studio-one");
        vague.platform = Some("host smoke rig".to_string());
        let error = write_daw_smoke_report(&paths, vague)
            .expect_err("vague platform text should be rejected")
            .to_string();
        assert!(error.contains("must mention a supported platform for Studio One"));
        assert!(!paths.studio_one.join("platform.txt").exists());
    }

    #[test]
    fn daw_matrix_read_rejects_host_unsupported_platform_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        write_daw_smoke_report(&paths, test_daw_smoke_report_input("ableton")).unwrap();
        fs::write(
            paths.ableton.join("platform.txt"),
            "Linux X11 / edited after capture\n",
        )
        .unwrap();

        let row = collect_daw_evidence_for_host("ableton", &paths.ableton);

        assert_eq!(row["platform"], "Linux X11 / edited after capture");
        assert_eq!(row["platform_supported"], false);
        assert_eq!(row["scan"], true);
        assert!(!daw_row_complete(&row));
        assert_eq!(daw_missing_checks(&row), vec!["platform"]);
    }

    #[test]
    fn daw_matrix_read_rejects_wayland_and_generic_linux_platform_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let mut bitwig_x11 = test_daw_smoke_report_input("bitwig");
        bitwig_x11.platform = Some("Linux X11 / Bitwig smoke".to_string());
        write_daw_smoke_report(&paths, bitwig_x11).unwrap();
        let x11 = collect_daw_evidence_for_host("bitwig", &paths.bitwig);
        assert_eq!(x11["platform_supported"], true);
        assert!(daw_row_complete(&x11));

        fs::write(
            paths.bitwig.join("platform.txt"),
            "Linux Wayland / edited\n",
        )
        .unwrap();
        let wayland = collect_daw_evidence_for_host("bitwig", &paths.bitwig);
        assert_eq!(wayland["platform_supported"], false);
        assert!(!daw_row_complete(&wayland));

        fs::write(
            paths.bitwig.join("platform.txt"),
            "Linux / no windowing proof\n",
        )
        .unwrap();
        let generic_linux = collect_daw_evidence_for_host("bitwig", &paths.bitwig);
        assert_eq!(generic_linux["platform_supported"], false);
        assert!(!daw_row_complete(&generic_linux));
    }

    #[cfg(unix)]
    #[test]
    fn daw_matrix_write_report_rejects_symlinked_evidence_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let external_root = Utf8PathBuf::from_path_buf(temp.path().join("external-root")).unwrap();
        fs::create_dir(&external_root).unwrap();
        unix_fs::symlink(&external_root, &root).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let error = write_daw_smoke_report(&paths, test_daw_smoke_report_input("bitwig"))
            .expect_err("DAW smoke report writer should reject symlink evidence roots")
            .to_string();

        assert!(error.contains("DAW evidence directory parent must not be a symlink"));
        assert!(!external_root.join("bitwig/platform.txt").exists());
    }

    #[test]
    fn daw_matrix_write_report_accepts_reaper_generic_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let path = write_daw_smoke_report(&paths, test_daw_smoke_report_input("reaper")).unwrap();
        assert_eq!(path, paths.reaper);

        let row = collect_reaper_evidence(&paths.reaper);
        assert!(daw_row_complete(&row), "{row:?}");
        assert_eq!(row["platform"], "macOS arm64 / host smoke");
    }

    #[test]
    fn reaper_evidence_rejects_contradictory_marker_files() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("scan-smoke.log"), "scan=true\nscan failed\n").unwrap();
        fs::write(dir.join("load-smoke.log"), "load=true\nload failed\n").unwrap();
        fs::write(dir.join("ui-smoke.log"), "ui=true\nui timeout\n").unwrap();
        fs::write(
            dir.join("ui-host-smoke.log"),
            "ui_host_param=true\nhost_param=false\n",
        )
        .unwrap();
        fs::write(
            dir.join("meter-stream.log"),
            "meter_flush sent=2\nmeter unavailable\n",
        )
        .unwrap();
        fs::write(
            dir.join("automation-smoke.log"),
            "automation=true\nautomation failed\n",
        )
        .unwrap();
        fs::write(
            dir.join("buffer-sample-rate.log"),
            "buffer_sample_rate_change=true\nbuffer_change=false\n",
        )
        .unwrap();
        fs::write(
            dir.join("restore-smoke.log"),
            "save_restore=true\nrestore failed\n",
        )
        .unwrap();
        fs::write(
            dir.join("offline-render.log"),
            "offline_render=true\nrender failed\n",
        )
        .unwrap();

        let row = collect_reaper_evidence(&dir);
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
            assert_eq!(row[key], false, "{key}");
        }
    }

    #[test]
    fn daw_matrix_write_report_rejects_malformed_marker_text() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let mut bad_host = test_daw_smoke_report_input("bitwig");
        bad_host.host = Some("bitwig\nforged".to_string());
        let error = write_daw_smoke_report(&paths, bad_host).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("DAW smoke report host must not contain control characters")
        );
        assert!(!paths.bitwig.join("platform.txt").exists());

        let mut bad_platform = test_daw_smoke_report_input("bitwig");
        bad_platform.platform = Some("macOS arm64\nscan=true".to_string());
        let error = write_daw_smoke_report(&paths, bad_platform).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("DAW smoke report `platform` must not contain control characters")
        );
        assert!(!paths.bitwig.join("platform.txt").exists());

        let mut nul_scan = test_daw_smoke_report_input("bitwig");
        nul_scan.scan =
            Some("VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\0".to_string());
        let error = write_daw_smoke_report(&paths, nul_scan).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain control characters other than tab/newline")
        );
        assert!(!paths.bitwig.join("scan-smoke.log").exists());

        let mut unsafe_ui = test_daw_smoke_report_input("bitwig");
        unsafe_ui.ui = Some("ui=true\u{202e}hidden".to_string());
        let error = write_daw_smoke_report(&paths, unsafe_ui).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );
        assert!(!paths.bitwig.join("ui-smoke.log").exists());

        let mut too_long = test_daw_smoke_report_input("bitwig");
        too_long.load = Some(format!(
            "load=true\n{}",
            "x".repeat(DAW_SMOKE_MARKER_MAX_BYTES)
        ));
        let error = write_daw_smoke_report(&paths, too_long).unwrap_err();
        assert!(error.to_string().contains("must be at most"));
        assert!(!paths.bitwig.join("load-smoke.log").exists());
    }

    #[test]
    fn daw_matrix_write_report_accepts_multiline_marker_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);
        let mut input = test_daw_smoke_report_input("bitwig");
        input.scan =
            Some("VestyGain.vst3\nVestyWebUIDemo.vst3\nVestyMIDISynth.vst3\tok".to_string());
        input.load = Some("load=true\nhost loaded all examples\tok".to_string());
        input.automation =
            Some("automation=true\nmidi_note_inserted=true\tproject_ready=true".to_string());

        let path = write_daw_smoke_report(&paths, input).unwrap();

        assert_eq!(path, paths.bitwig);
        let row = collect_generic_daw_evidence("Bitwig Studio", &paths.bitwig);
        assert!(daw_row_complete(&row), "{row:?}");
    }

    #[test]
    fn daw_matrix_write_report_rejects_pending_or_zero_meter() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);

        let mut pending = test_daw_smoke_report_input("bitwig");
        pending.scan = Some("pending".to_string());
        let error =
            write_daw_smoke_report(&paths, pending).expect_err("pending marker should be rejected");
        assert!(error.to_string().contains("missing positive evidence"));
        assert!(!paths.bitwig.join("scan-smoke.log").exists());

        let mut zero_meter = test_daw_smoke_report_input("bitwig");
        zero_meter.meter_stream = Some("meter_flush sent=0".to_string());
        let error = write_daw_smoke_report(&paths, zero_meter)
            .expect_err("zero meter marker should be rejected");
        assert!(error.to_string().contains("nonzero meter evidence"));
        assert!(!paths.bitwig.join("meter-stream.log").exists());

        let mut missing_buffer_sample_rate = test_daw_smoke_report_input("bitwig");
        missing_buffer_sample_rate.buffer_sample_rate_change = None;
        let error = write_daw_smoke_report(&paths, missing_buffer_sample_rate)
            .expect_err("buffer/sample-rate evidence should be required");
        assert!(
            error
                .to_string()
                .contains("DAW smoke report requires `--buffer-sample-rate-change`")
        );
        assert!(!paths.bitwig.join("buffer-sample-rate.log").exists());

        let mut invalid_marker = test_daw_smoke_report_input("bitwig");
        invalid_marker.automation = Some("automation recorded in host".to_string());
        let error = write_daw_smoke_report(&paths, invalid_marker)
            .expect_err("semantic marker parser should reject vague automation evidence");
        assert!(
            error
                .to_string()
                .contains("DAW smoke report markers did not pass parser: automation")
        );
        assert!(!paths.bitwig.join("platform.txt").exists());

        let mut pending_platform = test_daw_smoke_report_input("bitwig");
        pending_platform.platform = Some("manual platform pending".to_string());
        let error = write_daw_smoke_report(&paths, pending_platform)
            .expect_err("pending platform should be rejected");
        assert!(
            error
                .to_string()
                .contains("missing supported host/platform evidence")
        );
        assert!(!paths.bitwig.join("platform.txt").exists());

        let mut negative_scan = test_daw_smoke_report_input("bitwig");
        negative_scan.scan = Some("scan=true\nplugin scan failed".to_string());
        let error = write_daw_smoke_report(&paths, negative_scan)
            .expect_err("negative scan evidence should be rejected");
        assert!(error.to_string().contains("missing positive evidence"));
        assert!(!paths.bitwig.join("scan-smoke.log").exists());
    }

    #[test]
    fn daw_matrix_write_report_rejects_explicit_host_scope_mismatch_marker() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        let paths = DawEvidencePaths::from_root(&root);
        let mut input = test_daw_smoke_report_input("bitwig");
        input.scan = Some("host=Ableton Live\nscan=true".to_string());

        let error = write_daw_smoke_report(&paths, input)
            .expect_err("host-scoped evidence from another DAW should be rejected")
            .to_string();

        assert!(error.contains("DAW smoke report markers did not pass parser: scan"));
        assert!(!paths.bitwig.join("platform.txt").exists());
        assert!(!paths.bitwig.join("scan-smoke.log").exists());
    }

    #[test]
    fn render_file_evidence_accepts_quoted_nonempty_file_and_rejects_empty_file() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let rendered = dir.join("rendered.wav");
        let empty = dir.join("empty.wav");
        fs::write(&rendered, b"RIFFvesty").unwrap();
        fs::write(&empty, b"").unwrap();

        let parsed = render_file_from_log(&format!("render_file = \"{rendered}\"\n")).unwrap();
        assert_eq!(parsed, rendered);
        assert!(render_file_exists_and_nonempty(parsed));

        let empty = render_file_from_log(&format!("render_file='{empty}'\n")).unwrap();
        assert!(!render_file_exists_and_nonempty(empty));
        assert!(render_file_from_log("render_file = \n").is_none());
    }

    #[test]
    fn render_file_evidence_resolves_relative_paths_inside_evidence_dir() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("rendered.wav"), b"RIFFvesty").unwrap();
        fs::write(temp.path().join("outside.wav"), b"RIFFoutside").unwrap();

        assert!(render_file_evidence_ok("render_file=rendered.wav\n", &dir));
        assert!(render_file_evidence_ok(
            "render_file = './rendered.wav'\n",
            &dir
        ));
        assert!(!render_file_evidence_ok("render_file=missing.wav\n", &dir));
        assert!(!render_file_evidence_ok(
            "render_file=../outside.wav\n",
            &dir
        ));
    }

    #[cfg(unix)]
    #[test]
    fn render_file_evidence_rejects_symlinked_render_targets() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().join("daw-evidence")).unwrap();
        fs::create_dir_all(&dir).unwrap();
        let external_render =
            Utf8PathBuf::from_path_buf(temp.path().join("external-rendered.wav")).unwrap();
        fs::write(&external_render, b"RIFFexternal").unwrap();
        unix_fs::symlink(&external_render, dir.join("rendered.wav")).unwrap();

        assert!(!render_file_exists_and_nonempty(dir.join("rendered.wav")));
        assert!(!render_file_evidence_ok("render_file=rendered.wav\n", &dir));

        fs::write(dir.join("offline-render.log"), "render_file=rendered.wav\n").unwrap();
        let row = collect_generic_daw_evidence("Studio One", &dir);
        assert_eq!(row["offline_render"], false);
    }

    #[test]
    fn reaper_evidence_accepts_explicit_offline_render_marker() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("render-smoke.log"), "offline_render=pass\n").unwrap();

        let row = collect_reaper_evidence(&dir);
        assert_eq!(row["offline_render"], true);
    }

    #[test]
    fn generic_daw_evidence_accepts_relative_render_file_marker() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        fs::write(dir.join("rendered.wav"), b"RIFFvesty").unwrap();
        fs::write(dir.join("offline-render.log"), "render_file=rendered.wav\n").unwrap();

        let row = collect_generic_daw_evidence("Studio One", &dir);
        assert_eq!(row["offline_render"], true);
    }

    #[test]
    fn new_project_templates_use_params_derive() {
        let effect = effect_template(
            "Gain",
            "GainPlugin",
            "GainParams",
            "GainKernel",
            "&[0; 16]",
            true,
        );
        assert!(effect.contains("#[derive(Params)]"));
        assert!(!effect.contains("impl ParamCollection for GainParams"));
        assert!(effect.contains("gain: ParamHandle"));
        assert!(effect.contains("gain: self.params.resolve_or_invalid(\"gain\")"));
        assert!(!effect.contains(".resolve(\"gain\").expect("));
        assert!(!effect.contains(".resolve(\"gain\").unwrap("));
        assert!(effect.contains("ParamAutomationSegments::new(events, self.gain"));
        assert!(effect.contains("context.param_normalized(self.gain)"));
        assert!(effect.contains("copy_input_to_output_range"));
        assert!(effect.contains(".with_size(900, 560)"));
        assert!(effect.contains(".with_min_size(640, 420)"));
        assert!(effect.contains(".with_resizable(true)"));
        assert!(!effect.contains("context.params().get_normalized(\"gain\")"));

        let instrument = instrument_template(
            "Synth",
            "SynthPlugin",
            "SynthParams",
            "SynthKernel",
            "&[0; 16]",
            true,
        );
        assert!(instrument.contains("#[derive(Params)]"));
        assert!(!instrument.contains("impl ParamCollection for SynthParams"));
        assert!(instrument.contains("volume: ParamHandle"));
        assert!(instrument.contains("volume: self.params.resolve_or_invalid(\"volume\")"));
        assert!(!instrument.contains(".resolve(\"volume\").expect("));
        assert!(!instrument.contains(".resolve(\"volume\").unwrap("));
        assert!(instrument.contains("ParamAutomationSegments::new(events, self.volume"));
        assert!(instrument.contains("context.param_normalized(self.volume)"));
        assert!(instrument.contains(".with_size(900, 560)"));
        assert!(instrument.contains(".with_min_size(640, 420)"));
        assert!(instrument.contains(".with_resizable(true)"));
        assert!(!instrument.contains("context.params().get_normalized(\"volume\")"));
    }

    #[test]
    fn cargo_template_can_use_local_vesty_path() {
        let published = cargo_toml("Demo Plugin", "demo", None);
        assert!(published.contains(&format!(r#"vesty = "={}""#, env!("CARGO_PKG_VERSION"))));
        assert!(published.contains("[workspace]"));
        assert!(published.contains("publish = false"));
        assert!(published.contains(r#"description = "Demo Plugin VST3 plugin""#));

        let local = cargo_toml("Demo Plugin", "demo", Some(Utf8Path::new("/tmp/vesty")));
        assert!(local.contains(r#"vesty = { path = "/tmp/vesty" }"#));
    }

    #[test]
    fn project_readme_template_documents_generated_project_flow() {
        let readme = project_readme("My Plugin", "effect", UiTemplate::React);
        assert!(readme.contains("# My Plugin"));
        assert!(readme.contains("VST3 audio effect"));
        assert!(readme.contains("`react` Web UI"));
        assert!(readme.contains("publish = false"));
        assert!(readme.contains("npm run build"));
        assert!(readme.contains("target/release/libmy_plugin.dylib"));
        assert!(readme.contains("target/vesty/MyPlugin.vst3"));
        assert!(readme.contains("vesty doctor"));

        let headless = project_readme("My Synth", "instrument", UiTemplate::None);
        assert!(headless.contains("VST3 instrument"));
        assert!(headless.contains("headless"));
        assert!(!headless.contains("npm run build"));
    }

    #[test]
    fn vesty_toml_template_includes_package_metadata() {
        let effect = vesty_toml(
            "My Gain",
            "effect",
            "01234567-89ab-cdef-0123-456789abcdef",
            "",
            "dev.vesty.my-gain",
            package_category_for_kind("effect"),
        );
        assert!(effect.contains("[package]"));
        assert!(effect.contains(r#"bundle_id = "dev.vesty.my-gain""#));
        assert!(effect.contains(r#"category = "Fx""#));

        let instrument = vesty_toml(
            "My Synth",
            "instrument",
            "01234567-89ab-cdef-0123-456789abcdef",
            "",
            "dev.vesty.my-synth",
            package_category_for_kind("instrument"),
        );
        assert!(instrument.contains(r#"category = "Instrument""#));
    }

    #[test]
    fn create_project_generates_parameter_sidecar_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("my-gain")).unwrap();

        create_project(
            root.as_str(),
            Some("effect"),
            Some("none"),
            None,
            None,
            None,
        )
        .unwrap();

        let config_text = fs::read_to_string(root.join("vesty.toml")).unwrap();
        assert!(config_text.contains(r#"parameter_manifest = "vesty-parameters.json""#));

        let specs_text = fs::read_to_string(root.join("params.specs.json")).unwrap();
        assert!(specs_text.contains(r#""programChange": false"#));
        let expected_manifest = parameter_manifest_from_specs_json(&specs_text).unwrap();
        let manifest = read_parameter_manifest(&root.join("vesty-parameters.json")).unwrap();
        assert_eq!(manifest, expected_manifest);
        assert_eq!(manifest.parameters[0].id, "gain");
        assert_eq!(manifest.parameters[0].vst3_param_id, 1_983_572_582);
        assert!(!manifest.parameters[0].spec.flags.program_change);
        let manifest_text = fs::read_to_string(root.join("vesty-parameters.json")).unwrap();
        assert!(manifest_text.contains(r#""programChange": false"#));

        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("params.specs.json"));
        assert!(readme.contains("vesty param-manifest --specs params.specs.json"));
    }

    #[cfg(unix)]
    #[test]
    fn create_project_rejects_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-workspace");
        let parent_link = root.join("workspace-link");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let error = create_project(
            parent_link.join("my-gain").as_str(),
            Some("effect"),
            Some("none"),
            None,
            None,
            None,
        )
        .expect_err("project scaffolder should reject symlinked output parents")
        .to_string();

        assert!(error.contains("project path parent must not be a symlink"));
        assert!(!external.join("my-gain").exists());
    }

    #[test]
    fn create_project_uses_leaf_directory_as_project_name() {
        let temp = tempfile::tempdir().unwrap();
        let nested = Utf8PathBuf::from_path_buf(temp.path().join("nested/my-gain")).unwrap();

        create_project(
            nested.as_str(),
            Some("effect"),
            Some("none"),
            None,
            None,
            None,
        )
        .unwrap();

        let cargo = fs::read_to_string(nested.join("Cargo.toml")).unwrap();
        assert!(cargo.contains(r#"name = "my-gain""#));
        assert!(cargo.contains(r#"description = "my-gain VST3 plugin""#));

        let config = fs::read_to_string(nested.join("vesty.toml")).unwrap();
        assert!(config.contains(r#"name = "my-gain""#));
        assert!(config.contains(r#"bundle_id = "dev.vesty.my-gain""#));
        assert!(!config.contains("nested/my-gain"));

        let source = fs::read_to_string(nested.join("src/lib.rs")).unwrap();
        assert!(source.contains("pub struct MyGain"));
        assert!(source.contains(r#"name: "my-gain""#));
        assert!(!source.contains("NestedMyGain"));

        let readme = fs::read_to_string(nested.join("README.md")).unwrap();
        assert!(readme.contains("# my-gain"));
        assert!(!readme.contains("nested/my-gain"));
    }

    #[test]
    fn project_template_gallery_resolves_starters_and_aliases() {
        let gain = resolve_project_template("gain").unwrap();
        assert_eq!(gain.kind, "effect");
        assert_eq!(gain.ui, "none");

        let default = resolve_project_template("default").unwrap();
        assert_eq!(default.id, "web-ui-param-demo");
        assert_eq!(default.kind, "effect");
        assert_eq!(default.ui, "react");

        let instrument = resolve_project_template("headless-instrument").unwrap();
        assert_eq!(instrument.id, "midi-synth");
        assert_eq!(instrument.kind, "instrument");
        assert_eq!(instrument.ui, "none");

        let json = serde_json::to_value(PROJECT_TEMPLATES).unwrap();
        assert_eq!(json.as_array().unwrap().len(), PROJECT_TEMPLATES.len());
        assert!(
            json.as_array()
                .unwrap()
                .iter()
                .any(|template| template["id"] == "svelte-ui-param-demo")
        );

        let error = resolve_project_template("unknown-starter")
            .unwrap_err()
            .to_string();
        assert!(error.contains("vesty templates"));
    }

    #[test]
    fn create_project_accepts_template_gallery_defaults_and_overrides() {
        let temp = tempfile::tempdir().unwrap();
        let synth = Utf8PathBuf::from_path_buf(temp.path().join("template-synth")).unwrap();
        let web = Utf8PathBuf::from_path_buf(temp.path().join("template-web")).unwrap();
        let overridden = Utf8PathBuf::from_path_buf(temp.path().join("template-override")).unwrap();

        create_project(synth.as_str(), None, None, Some("midi-synth"), None, None).unwrap();
        let synth_config = fs::read_to_string(synth.join("vesty.toml")).unwrap();
        assert!(synth_config.contains(r#"kind = "instrument""#));
        assert!(!synth_config.contains("[ui]"));
        assert!(!synth.join("ui").exists());

        create_project(
            web.as_str(),
            None,
            None,
            Some("web-ui-param-demo"),
            None,
            None,
        )
        .unwrap();
        let web_config = fs::read_to_string(web.join("vesty.toml")).unwrap();
        assert!(web_config.contains(r#"kind = "effect""#));
        assert!(web_config.contains("[ui]"));
        assert!(web.join("ui/src/App.tsx").is_file());

        create_project(
            overridden.as_str(),
            None,
            Some("none"),
            Some("web-ui-param-demo"),
            None,
            None,
        )
        .unwrap();
        let overridden_config = fs::read_to_string(overridden.join("vesty.toml")).unwrap();
        assert!(overridden_config.contains(r#"kind = "effect""#));
        assert!(!overridden_config.contains("[ui]"));
        assert!(!overridden.join("ui").exists());
    }

    #[test]
    fn every_builtin_project_template_generates_expected_files() {
        let temp = tempfile::tempdir().unwrap();

        for template in PROJECT_TEMPLATES {
            let root =
                Utf8PathBuf::from_path_buf(temp.path().join(template.id)).expect("utf-8 path");
            create_project(root.as_str(), None, None, Some(template.id), None, None).unwrap();

            assert!(root.join("Cargo.toml").is_file(), "{}", template.id);
            assert!(root.join("vesty.toml").is_file(), "{}", template.id);
            assert!(root.join("README.md").is_file(), "{}", template.id);
            assert!(root.join("src/lib.rs").is_file(), "{}", template.id);
            assert!(root.join("params.specs.json").is_file(), "{}", template.id);
            assert!(
                root.join("vesty-parameters.json").is_file(),
                "{}",
                template.id
            );

            let config = fs::read_to_string(root.join("vesty.toml")).unwrap();
            assert!(
                config.contains(&format!(r#"kind = "{}""#, template.kind)),
                "{}",
                template.id
            );

            if template.ui == "none" {
                assert!(!config.contains("[ui]"), "{}", template.id);
                assert!(!root.join("ui").exists(), "{}", template.id);
            } else {
                assert!(config.contains("[ui]"), "{}", template.id);
                assert!(root.join("ui/package.json").is_file(), "{}", template.id);
                assert!(root.join("ui/index.html").is_file(), "{}", template.id);
                assert!(root.join("ui/tsconfig.json").is_file(), "{}", template.id);

                let ui_param = UiParamTemplate::for_kind(template.kind);
                let mut ui_source = fs::read_to_string(root.join("ui/index.html")).unwrap();
                for entry in fs::read_dir(root.join("ui/src")).unwrap() {
                    let entry = entry.unwrap();
                    if entry.file_type().unwrap().is_file() {
                        ui_source.push_str(&fs::read_to_string(entry.path()).unwrap());
                    }
                }
                assert!(ui_source.contains(ui_param.id), "{}", template.id);
                assert!(ui_source.contains(ui_param.label), "{}", template.id);
                if template.kind == "instrument" {
                    assert!(!ui_source.contains(r#"beginParamEdit("gain")"#));
                    assert!(!ui_source.contains(r#"useVestyParamEdit("gain")"#));
                    assert!(!ui_source.contains(r#"vestyParamEdit("gain")"#));
                }
            }
        }
    }

    #[test]
    fn project_kind_is_canonicalized_for_templates() {
        assert_eq!(canonical_project_kind("effect").unwrap(), "effect");
        assert_eq!(canonical_project_kind("Fx").unwrap(), "effect");
        assert_eq!(canonical_project_kind("audio-effect").unwrap(), "effect");
        assert_eq!(canonical_project_kind("audio_effect").unwrap(), "effect");
        assert_eq!(canonical_project_kind("instrument").unwrap(), "instrument");

        let error = canonical_project_kind("surround-generator")
            .unwrap_err()
            .to_string();
        assert!(error.contains("--kind"));
    }

    #[test]
    fn ui_package_template_can_use_local_plugin_ui_path() {
        let package_paths = UiPackagePaths {
            plugin_ui: Some(Utf8PathBuf::from("/tmp/vesty plugin-ui")),
        };
        let package_json = ui_package_json(UiTemplate::Vanilla, &package_paths);
        let value = serde_json::from_str::<serde_json::Value>(&package_json).unwrap();
        assert_eq!(value["private"], true);
        assert_eq!(
            value["dependencies"]["vesty-plugin-ui"],
            "file:/tmp/vesty plugin-ui"
        );

        let published = ui_package_json(UiTemplate::Vanilla, &UiPackagePaths::default());
        let value = serde_json::from_str::<serde_json::Value>(&published).unwrap();
        assert_eq!(value["private"], true);
        assert_eq!(
            value["dependencies"]["vesty-plugin-ui"],
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn ui_templates_emit_framework_specific_files() {
        fn assert_ready_param_binding(source: &str) {
            assert!(source.contains("type BridgeReadyPayload"));
            assert!(source.contains("type ParamChangedEvent"));
            assert!(source.contains("paramValues"));
            assert!(
                source.contains(
                    "ready.paramValues.find((param) => param.id === PARAM_ID)?.normalized"
                ),
                "{source}"
            );
            assert!(source.contains("bridge.subscribe<ParamChangedEvent>(\"param.changed\""));
            assert!(source.contains("event.id === PARAM_ID"));
            assert!(!source.contains("bridge.getSnapshot"));
        }

        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("plugin")).unwrap();

        fs::create_dir_all(root.join("ui/src")).unwrap();
        write_ui_template(
            &root,
            "Demo",
            UiTemplate::Vanilla,
            &UiPackagePaths::default(),
            UiParamTemplate::for_kind("effect"),
        )
        .unwrap();
        let vanilla = fs::read_to_string(root.join("ui/src/index.ts")).unwrap();
        assert!(vanilla.contains(
            "import { createBridge, type BridgeReadyPayload, type ParamChangedEvent } from \"vesty-plugin-ui\""
        ));
        assert!(vanilla.contains("const ready = await bridge.ready()"));
        assert!(vanilla.contains("ready.snapshot"));
        assert_ready_param_binding(&vanilla);
        assert!(vanilla.contains("setPointerCapture"));
        assert!(vanilla.contains("\"pointercancel\""));
        assert!(vanilla.contains("\"lostpointercapture\""));
        assert!(root.join("ui/src/index.ts").is_file());
        fs::remove_dir_all(root.join("ui")).unwrap();

        for template in [UiTemplate::React, UiTemplate::Vue, UiTemplate::Svelte] {
            fs::create_dir_all(root.join("ui/src")).unwrap();
            write_ui_template(
                &root,
                "Demo",
                template,
                &UiPackagePaths::default(),
                UiParamTemplate::for_kind("effect"),
            )
            .unwrap();
            let package_json = fs::read_to_string(root.join("ui/package.json")).unwrap();
            let index_html = fs::read_to_string(root.join("ui/index.html")).unwrap();
            let vite_config = fs::read_to_string(root.join("ui/vite.config.ts")).unwrap();
            let package = serde_json::from_str::<serde_json::Value>(&package_json).unwrap();
            assert_eq!(package["private"], true);

            match template {
                UiTemplate::React => {
                    let app = fs::read_to_string(root.join("ui/src/App.tsx")).unwrap();
                    assert_eq!(
                        package["dependencies"]["vesty-plugin-ui"],
                        env!("CARGO_PKG_VERSION")
                    );
                    assert!(package_json.contains("\"react\""));
                    assert!(package_json.contains("\"@vitejs/plugin-react\""));
                    assert_eq!(package["scripts"]["typecheck"], "tsc --noEmit");
                    assert_eq!(package["devDependencies"]["@types/react"], "latest");
                    assert_eq!(package["devDependencies"]["@types/react-dom"], "latest");
                    assert!(index_html.contains("/src/main.tsx"));
                    assert!(vite_config.contains("react()"));
                    assert!(app.contains("vesty-plugin-ui/react"));
                    assert!(app.contains("useVestyParamEdit"));
                    assert_ready_param_binding(&app);
                    assert!(app.contains("setPointerCapture"));
                    assert!(app.contains("onPointerCancel={end}"));
                    assert!(app.contains("onLostPointerCapture={end}"));
                    assert!(root.join("ui/src/App.tsx").is_file());
                    assert!(root.join("ui/src/main.tsx").is_file());
                }
                UiTemplate::Vue => {
                    let app = fs::read_to_string(root.join("ui/src/App.vue")).unwrap();
                    assert_eq!(
                        package["dependencies"]["vesty-plugin-ui"],
                        env!("CARGO_PKG_VERSION")
                    );
                    assert!(package_json.contains("\"vue\""));
                    assert!(package_json.contains("\"@vitejs/plugin-vue\""));
                    assert_eq!(package["scripts"]["typecheck"], "vue-tsc --noEmit");
                    assert_eq!(package["devDependencies"]["vue-tsc"], "3.3.7");
                    assert_eq!(package["devDependencies"]["typescript"], "6.0.3");
                    assert!(index_html.contains("/src/main.ts"));
                    assert!(vite_config.contains("vue()"));
                    assert!(app.contains("vesty-plugin-ui/vue"));
                    assert!(app.contains("useVestyParamEdit"));
                    assert_ready_param_binding(&app);
                    assert!(app.contains("setPointerCapture"));
                    assert!(app.contains("@pointercancel=\"end\""));
                    assert!(app.contains("@lostpointercapture=\"end\""));
                    assert!(root.join("ui/src/App.vue").is_file());
                    assert!(root.join("ui/src/main.ts").is_file());
                }
                UiTemplate::Svelte => {
                    let app = fs::read_to_string(root.join("ui/src/App.svelte")).unwrap();
                    assert_eq!(
                        package["dependencies"]["vesty-plugin-ui"],
                        env!("CARGO_PKG_VERSION")
                    );
                    assert!(package_json.contains("\"svelte\""));
                    assert!(package_json.contains("\"@sveltejs/vite-plugin-svelte\""));
                    assert_eq!(
                        package["scripts"]["typecheck"],
                        "svelte-check --tsconfig ./tsconfig.json"
                    );
                    assert_eq!(package["devDependencies"]["svelte-check"], "4.7.2");
                    assert_eq!(package["devDependencies"]["typescript"], "6.0.3");
                    assert!(index_html.contains("/src/main.ts"));
                    assert!(vite_config.contains("svelte()"));
                    assert!(app.contains("vesty-plugin-ui/svelte"));
                    assert!(app.contains("vestyParamEdit"));
                    assert_ready_param_binding(&app);
                    assert!(app.contains("setPointerCapture"));
                    assert!(app.contains("onpointercancel={end}"));
                    assert!(app.contains("onlostpointercapture={end}"));
                    assert!(root.join("ui/src/App.svelte").is_file());
                    assert!(root.join("ui/src/main.ts").is_file());
                }
                UiTemplate::None | UiTemplate::Vanilla => unreachable!(),
            }

            fs::remove_dir_all(root.join("ui")).unwrap();
        }

        assert!(parse_ui_template("vanilla").is_ok());
        assert!(parse_ui_template("nope").is_err());
    }

    fn workspace_root() -> Utf8PathBuf {
        let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn write_smoke_host_workspace(root: &std::path::Path) -> Utf8PathBuf {
        let workspace = Utf8PathBuf::from_path_buf(root.join("workspace")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            workspace.join("Cargo.toml"),
            r#"[workspace]
members = ["examples/gain", "examples/midi-synth", "examples/web-ui-param-demo"]
"#,
        )
        .unwrap();

        write_smoke_host_example(&workspace, "gain", "Vesty Gain", "Fx", false);
        write_smoke_host_example(
            &workspace,
            "midi-synth",
            "Vesty MIDI Synth",
            "Instrument",
            false,
        );
        write_smoke_host_example(
            &workspace,
            "web-ui-param-demo",
            "Vesty Web UI Demo",
            "Fx",
            true,
        );
        workspace
    }

    fn write_smoke_host_example(
        workspace: &Utf8Path,
        example: &str,
        plugin_name: &str,
        kind: &str,
        has_ui: bool,
    ) {
        let dir = workspace.join("examples").join(example);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vesty.toml"),
            smoke_host_vesty_toml(plugin_name, kind, has_ui),
        )
        .unwrap();
        let specs_json = if kind.eq_ignore_ascii_case("instrument") {
            default_param_specs_json("instrument")
        } else {
            default_param_specs_json("effect")
        };
        fs::write(dir.join("params.specs.json"), specs_json).unwrap();
        let manifest = parameter_manifest_from_specs_json(specs_json).unwrap();
        write_parameter_manifest(&dir.join("vesty-parameters.json"), &manifest).unwrap();
        if has_ui {
            let dist = dir.join("ui/dist");
            fs::create_dir_all(&dist).unwrap();
            fs::write(dist.join("index.html"), "<!doctype html><main>Vesty</main>").unwrap();
            fs::write(dist.join("index.js"), "console.log('vesty smoke host');").unwrap();
        }
    }

    fn smoke_host_vesty_toml(plugin_name: &str, kind: &str, has_ui: bool) -> String {
        let ui = if has_ui {
            r#"
[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420
"#
        } else {
            ""
        };
        let bundle_id = format!(
            "dev.vesty.{}",
            plugin_name.to_ascii_lowercase().replace(' ', "-")
        );
        format!(
            r#"[plugin]
name = "{plugin_name}"
vendor = "Vesty"
version = "0.1.0"
kind = "{kind}"
class_id = "56455354-4947-4149-4e30-303030303031"
{ui}
[package]
bundle_id = "{bundle_id}"
category = "{kind}"
parameter_manifest = "vesty-parameters.json"
"#
        )
    }

    fn successful_test_output(stdout: &str, stderr: &str) -> Output {
        Output {
            status: success_exit_status(),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[cfg(unix)]
    fn success_exit_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    fn success_exit_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }

    fn test_ui_config(dir: &str, build: Option<&str>, dist: Option<&str>) -> UiConfig {
        UiConfig {
            dir: dir.to_string(),
            dev_url: None,
            build: build.map(str::to_string),
            dist: dist.map(str::to_string),
            width: None,
            height: None,
            min_width: None,
            min_height: None,
        }
    }

