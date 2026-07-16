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
