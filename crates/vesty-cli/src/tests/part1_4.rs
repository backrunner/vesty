    #[test]
    fn ci_release_check_artifacts_reject_mismatched_ci_run_url_when_expected() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        let mut report = test_ci_release_check_report();
        report.ci_run_url = Some("https://github.com/vesty-rs/other/actions/runs/999".to_string());
        fs::write(
            root.join("release-check-Windows.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890"),
        );

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("expected vesty-rs/vesty run 1234567890")
        );
        assert!(check.value.contains("got vesty-rs/other run 999"));
    }

    #[test]
    fn platform_smoke_release_check_requires_real_platform_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        assert_eq!(write_platform_smoke_templates(&root).unwrap(), 4);
        assert_eq!(write_platform_smoke_templates(&root).unwrap(), 0);
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("Platform Smoke Evidence"));
        assert!(readme.contains("Linux Wayland is experimental"));

        let pending_local = platform_smoke_release_check(Some(&root), false);
        assert_eq!(pending_local.status, "skipped");
        assert_eq!(pending_local.value, "not requested");

        let pending_required = platform_smoke_release_check(Some(&root), true);
        assert_eq!(pending_required.status, "failed");
        assert_eq!(pending_required.value, "required evidence missing");
        assert!(
            pending_required
                .hint
                .as_deref()
                .unwrap()
                .contains("no passing platform smoke reports found")
        );

        fs::remove_file(root.join("macos.json")).unwrap();
        fs::remove_file(root.join("windows-x64.json")).unwrap();
        fs::remove_file(root.join("linux-x11.json")).unwrap();
        write_platform_smoke_artifact(&root.join("macos.json"), "macos");

        let local = platform_smoke_release_check(Some(&root), false);
        assert_eq!(local.status, "ok");
        assert!(local.value.contains("macOS"));
        assert!(local.hint.as_deref().unwrap().contains("Windows x64"));

        let required = platform_smoke_release_check(Some(&root), true);
        assert_eq!(required.status, "failed");
        assert!(required.value.contains("Windows x64"));
        assert!(required.value.contains("Linux X11"));

        write_platform_smoke_artifact(&root.join("windows-x64.json"), "windows-x64");
        write_platform_smoke_artifact(&root.join("linux-x11.json"), "linux-x11");
        let complete = platform_smoke_release_check(Some(&root), true);
        assert_eq!(complete.status, "ok");
        assert!(complete.value.contains("3 platform smoke report"));
        assert!(complete.value.contains("Linux X11"));
    }

    #[test]
    fn platform_smoke_release_check_accepts_platform_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("macOS/platform-smoke.json"), "macos");
        write_platform_smoke_artifact(&root.join("Windows/platform-smoke.json"), "windows-x64");
        write_platform_smoke_artifact(&root.join("Linux-X11/platform-smoke.json"), "linux-x11");

        let check = platform_smoke_release_check(Some(&root), true);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 platform smoke report"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows x64"));
        assert!(check.value.contains("Linux X11"));
    }

    #[test]
    fn platform_smoke_release_check_rejects_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("Windows/platform-smoke.json"), "macos");

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("artifact path indicates windows-x64"));
        assert!(check.value.contains("report platform is macos"));
    }

    #[test]
    fn platform_smoke_release_check_rejects_ambiguous_path_platform_tokens() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("macos-windows/platform-smoke.json"), "macos");
        write_platform_smoke_artifact(
            &root.join("linux-x11-windows/platform-smoke.json"),
            "linux-x11",
        );

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("artifact path contains multiple platform tokens: macos, windows-x64")
        );
        assert!(
            check.value.contains(
                "artifact path contains multiple platform tokens: windows-x64, linux-x11"
            ),
            "{}",
            check.value
        );
    }

    #[test]
    fn platform_smoke_path_platform_inference_requires_linux_x11_token() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("linux-wayland/platform-smoke.json"), "macos");

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
        assert!(!check.value.contains("artifact path indicates linux-x11"));
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_release_check_rejects_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&external).unwrap();
        write_platform_smoke_artifact(&external.join("macos.json"), "macos");
        unix_fs::symlink(&external, &root).unwrap();

        let check = platform_smoke_release_check(Some(&root), true);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke path must not be a symlink")
        );
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_templates_reject_symlink_root_dir() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &root).unwrap();

        let error = write_platform_smoke_templates(&root)
            .expect_err("platform smoke templates should reject symlink roots")
            .to_string();

        assert!(error.contains("template output directory must not be a symlink"));
        assert!(!external.join("README.md").exists());
    }

    #[test]
    fn platform_smoke_write_report_validates_and_writes_normalized_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let path =
            write_platform_smoke_report(&root, test_platform_smoke_report_input("darwin")).unwrap();

        assert_eq!(path.file_name(), Some("macos.json"));
        let report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(report.platform, "macos");
        assert_eq!(report.os.as_deref(), Some("macOS"));
        assert_eq!(report.host.as_deref(), Some("Vesty smoke host"));
        assert_eq!(report.checks.len(), REQUIRED_PLATFORM_SMOKE_CHECKS.len());

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_write_report_rejects_symlink_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-macos.json")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, root.join("macos.json")).unwrap();

        let error = write_platform_smoke_report(&root, test_platform_smoke_report_input("macos"))
            .expect_err("platform smoke writer should reject symlink output")
            .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_write_report_rejects_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let parent_link = root.join("platform-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let error = write_platform_smoke_report(
            &parent_link.join("platform-smoke"),
            test_platform_smoke_report_input("macos"),
        )
        .expect_err("platform smoke writer should reject symlinked output parents")
        .to_string();

        assert!(error.contains("platform smoke report dir parent must not be a symlink"));
        assert!(!external.join("platform-smoke/macos.json").exists());
    }

    #[test]
    fn platform_smoke_write_report_rejects_invalid_or_zero_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let wayland =
            write_platform_smoke_report(&root, test_platform_smoke_report_input("linux-wayland"))
                .unwrap_err()
                .to_string();
        assert!(wayland.contains("unsupported platform"));

        let mut zero_meter = test_platform_smoke_report_input("linux-x11");
        zero_meter.meter_stream = Some("meter_flush sent=0".to_string());
        let error = write_platform_smoke_report(&root, zero_meter)
            .unwrap_err()
            .to_string();
        assert!(error.contains("meter stream"));
        assert!(!root.join("linux-x11.json").exists());
    }

    #[test]
    fn platform_smoke_requires_platform_specific_webview_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        for (platform, evidence) in [
            ("macos", "system_webview=true"),
            ("macos", "not WKWebView; WebKit.framework shim unavailable"),
            ("windows-x64", "WebKit.framework loaded"),
            ("windows-x64", "not WebView2; WebView2 disabled"),
            ("linux-x11", "WebKitGTK loaded"),
            (
                "linux-x11",
                "WebKitGTK loaded; Wayland compositor; X11 fallback not active",
            ),
            ("linux-x11", "WebKitGTK loaded; not X11"),
        ] {
            let mut input = test_platform_smoke_report_input(platform);
            input.system_webview = Some(evidence.to_string());
            let error = write_platform_smoke_report(&root, input)
                .unwrap_err()
                .to_string();
            assert!(error.contains("system WebView"), "{platform}: {error}");
        }
    }

    #[test]
    fn platform_smoke_requires_validator_identity_and_zero_fail_summary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        for evidence in [
            "vst3_validator=true",
            "Steinberg validator passed",
            "custom checker passed 47 tests, 0 failed",
            "Steinberg validator passed 47 tests, 1 failed",
            "Steinberg validator passed 0 tests, 0 failed",
            "Steinberg validator passed 47 tests, 0 failed; validator timeout",
            "VST3 validator: passed=47 failed=0; validator crashed",
            "Steinberg validator passed 47 tests, 0 failed; validator error: log truncated",
        ] {
            let mut input = test_platform_smoke_report_input("macos");
            input.vst3_validator = Some(evidence.to_string());
            let error = write_platform_smoke_report(&root, input)
                .unwrap_err()
                .to_string();
            assert!(error.contains("VST3 validator"), "{evidence}: {error}");
            assert!(
                error.contains("accepted smoke evidence markers"),
                "{evidence}: {error}"
            );
        }
    }

    #[test]
    fn platform_smoke_accepts_alternate_system_webview_and_validator_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let mut macos = test_platform_smoke_report_input("macos");
        macos.system_webview = Some("WKWebView created inside editor parent".to_string());
        macos.vst3_validator = Some("VST3 validator: passed=47 failed=0".to_string());
        let path = write_platform_smoke_report(&root, macos).unwrap();
        assert_eq!(path.file_name(), Some("macos.json"));

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn platform_smoke_rejects_placeholder_values_even_when_status_is_ok() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let mut report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "system_webview")
            .unwrap()
            .value = "replace with real system WebView evidence".to_string();
        fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("system WebView"));
        assert!(check.value.contains("missing positive evidence value"));
    }

    #[test]
    fn platform_smoke_rejects_contradictory_positive_values() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        for (name, value, expected_label) in [
            (
                "webview_attach",
                "webview_attach=true; attach failed",
                "WebView attach",
            ),
            (
                "meter_stream",
                "meter_flush sent=1; meter stream error",
                "meter stream",
            ),
            (
                "jsbridge_roundtrip",
                "jsbridge_roundtrip=true; roundtrip=false",
                "JSBridge roundtrip",
            ),
        ] {
            let mut candidate = report.clone();
            candidate
                .checks
                .iter_mut()
                .find(|check| check.name == name)
                .unwrap()
                .value = value.to_string();
            fs::write(&path, serde_json::to_string_pretty(&candidate).unwrap()).unwrap();

            let check = platform_smoke_release_check(Some(&root), false);
            assert_eq!(check.status, "failed", "{name}");
            assert!(
                check.value.contains(expected_label),
                "{name}: {}",
                check.value
            );
            assert!(
                check.value.contains("missing positive evidence value"),
                "{name}: {}",
                check.value
            );
        }
    }

    #[test]
    fn platform_smoke_rejects_os_metadata_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let macos = root.join("macos.json");
        write_platform_smoke_artifact(&macos, "macos");
        let mut macos_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&macos).unwrap()).unwrap();
        macos_report.os = Some("Windows 11 x64".to_string());
        fs::write(&macos, serde_json::to_string_pretty(&macos_report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke os `Windows 11 x64` does not match platform `macos`")
        );

        fs::remove_file(&macos).unwrap();
        let linux = root.join("linux-x11.json");
        write_platform_smoke_artifact(&linux, "linux-x11");
        let mut linux_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        linux_report.os = Some("Linux Wayland session".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&linux_report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains(
            "platform smoke os `Linux Wayland session` does not match platform `linux-x11`"
        ));
    }

    #[test]
    fn platform_smoke_rejects_malformed_report_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let valid_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&valid_report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<PlatformSmokeReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&valid_report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<PlatformSmokeReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = valid_report.clone();
        duplicate.checks.push(PlatformSmokeCheck {
            name: "system-webview".to_string(),
            status: "ok".to_string(),
            value: "WKWebView created".to_string(),
            hint: None,
        });
        fs::write(&path, serde_json::to_string_pretty(&duplicate).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate platform smoke check"));
        assert!(check.value.contains("system_webview"));

        let mut unknown = valid_report.clone();
        unknown.checks.push(PlatformSmokeCheck {
            name: "extra-check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        fs::write(&path, serde_json::to_string_pretty(&unknown).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unknown platform smoke check(s)"));
        assert!(check.value.contains("extra_check"));

        let mut missing = valid_report.clone();
        missing
            .checks
            .retain(|check| check.name != "jsbridge_roundtrip");
        fs::write(&path, serde_json::to_string_pretty(&missing).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke report missing required check(s)")
        );
        assert!(check.value.contains("jsbridge_roundtrip"));

        let mut control_host = valid_report.clone();
        control_host.host = Some("Vesty smoke host\nforged".to_string());
        fs::write(&path, serde_json::to_string_pretty(&control_host).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke host must not contain control characters")
        );

        let mut unsafe_hint = valid_report.clone();
        unsafe_hint.host = Some("Vesty smoke host".to_string());
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        fs::write(&path, serde_json::to_string_pretty(&unsafe_hint).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut too_many = valid_report;
        too_many.checks[0].hint = None;
        while too_many.checks.len() <= PLATFORM_SMOKE_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(PlatformSmokeCheck {
                name: format!("extra_check_{index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        fs::write(&path, serde_json::to_string_pretty(&too_many).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke report has too many checks")
        );
    }

    #[test]
    fn platform_smoke_rejects_wayland_and_zero_meter_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let wayland = root.join("linux-wayland.json");
        write_platform_smoke_artifact(&wayland, "linux-wayland");

        let check = platform_smoke_release_check(Some(&root), true);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unsupported platform"));

        fs::remove_file(&wayland).unwrap();
        let linux = root.join("linux-x11.json");
        write_platform_smoke_artifact(&linux, "linux-x11");
        let mut report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "meter_stream")
            .unwrap()
            .value = "meter_flush sent=0".to_string();
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("meter stream"));
    }

    #[test]
    fn ci_run_url_requires_exact_github_actions_run_shape() {
        for url in [
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890/",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890?check_suite_focus=true",
        ] {
            let check = ci_run_url_release_check(Some(url), true);
            assert_eq!(check.status, "ok", "{url}");
        }

        for url in [
            "https://github.com/vesty-rs/vesty/actions",
            "https://github.com/vesty-rs/vesty/actions/runs/",
            "https://github.com/vesty-rs/vesty/actions/runs/latest",
            "https://github.com/vesty-rs/vesty/actions/runs/123/jobs/456",
            "https://github.com/vesty-rs/vesty/actions/runs/123 456",
            "http://github.com/vesty-rs/vesty/actions/runs/123",
        ] {
            let check = ci_run_url_release_check(Some(url), true);
            assert_eq!(check.status, "failed", "{url}");
        }
    }

    #[test]
    fn ci_doctor_artifacts_reject_failed_required_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_check_status(&root.join("doctor-Linux.json"), "node", "missing");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("Linux/node status missing"));
    }

    #[test]
    fn ci_doctor_artifacts_require_sdk_headers_check() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        remove_doctor_check(&root.join("doctor-macOS.json"), "vst3 SDK headers");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("macOS/vst3 SDK headers missing"));
    }

    #[test]
    fn ci_doctor_artifacts_accept_linux_signing_policy_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_check_status(
            &root.join("doctor-Linux.json"),
            "signing: linux release policy",
            "unknown",
        );

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_infer_os_from_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("Linux/doctor.json"), "Linux");
        write_doctor_artifact(&root.join("macOS/doctor.json"), "macOS");
        write_doctor_artifact(&root.join("Windows/doctor.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 doctor report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_doctor_artifacts_infer_os_from_path_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("Linux/doctor.json"), "Linux");
        write_doctor_artifact(&root.join("macOS/doctor.json"), "macOS");
        write_doctor_artifact(&root.join("swing-state/doctor.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: Windows"));
        assert!(!check.value.contains("OS mismatch"));
        assert!(!check.value.contains("duplicate OS reports: Windows"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_mismatched_ci_run_url_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_ci_run_url(
            &root.join("doctor-Linux.json"),
            "https://github.com/vesty/vesty/actions/runs/99",
        );
        set_doctor_ci_run_url(
            &root.join("doctor-macOS.json"),
            "https://github.com/vesty/vesty/actions/runs/42/attempts/2",
        );
        set_doctor_ci_run_url(
            &root.join("doctor-Windows.json"),
            "https://github.com/vesty/vesty/actions/runs/42",
        );

        let check = ci_doctor_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty/vesty/actions/runs/42/attempts/1"),
        );

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("run URL mismatch"));
        assert!(check.value.contains("Linux expected vesty/vesty run 42"));
        assert!(!check.value.contains("macOS expected"));
        assert!(!check.value.contains("Windows expected"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_duplicate_os_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir_all(root.join("linux-copy")).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("linux-copy/doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate OS reports: Linux"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_os_label_mismatch_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_report_os(&root.join("doctor-Linux.json"), Some("macOS"));

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("OS mismatch"));
        assert!(check.value.contains("doctor-Linux.json"));
        assert!(check.value.contains("path indicates Linux"));
        assert!(check.value.contains("report os is macOS"));
    }

    #[test]
    fn ci_doctor_artifacts_allow_legacy_reports_without_os_label() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        for file in [
            "doctor-Linux.json",
            "doctor-macOS.json",
            "doctor-Windows.json",
        ] {
            set_doctor_report_os(&root.join(file), None);
        }

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_allow_legacy_reports_without_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty/vesty/actions/runs/42"),
        );

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_reject_cross_os_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.checks.push(DoctorCheck {
            name: "signing: codesign".to_string(),
            status: "ok".to_string(),
            value: "codesign from the wrong platform".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("Linux/signing: codesign unexpected for Linux doctor report")
        );
    }

    #[test]
    fn ci_doctor_artifacts_reject_legacy_cross_os_checks_from_path_os() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.os = None;
        report.checks.push(DoctorCheck {
            name: "signing: signtool".to_string(),
            status: "ok".to_string(),
            value: "signtool from the wrong platform".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("Linux/signing: signtool unexpected for Linux doctor report")
        );
    }

    #[test]
    fn ci_doctor_artifacts_reject_malformed_report_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        let valid_linux: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&valid_linux).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<DoctorReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&valid_linux).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<DoctorReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut invalid_url = valid_linux.clone();
        invalid_url.ci_run_url = Some("https://github.com/vesty/vesty/actions/runs/latest".into());
        fs::write(&linux, serde_json::to_string_pretty(&invalid_url).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("invalid doctor report ci_run_url"));

        let mut unknown = valid_linux.clone();
        unknown.checks.push(DoctorCheck {
            name: "manual extra doctor check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&unknown).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unknown doctor check"));
        assert!(check.value.contains("manual extra doctor check"));

        let mut duplicate = valid_linux.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        fs::write(&linux, serde_json::to_string_pretty(&duplicate).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate doctor check"));

        let mut control_os = valid_linux.clone();
        control_os.os = Some("Linux\nforged".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&control_os).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("doctor report os must not contain control characters")
        );

        let mut unsafe_hint = valid_linux.clone();
        unsafe_hint.os = Some("Linux".to_string());
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&unsafe_hint).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut unexpected_status = valid_linux.clone();
        unexpected_status.checks[0].hint = None;
        unexpected_status.checks[0].status = "passed".to_string();
        fs::write(
            &linux,
            serde_json::to_string_pretty(&unexpected_status).unwrap(),
        )
        .unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unexpected status `passed`"));

        let mut too_many = valid_linux;
        while too_many.checks.len() <= DOCTOR_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(DoctorCheck {
                name: format!("extra doctor check {index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        fs::write(&linux, serde_json::to_string_pretty(&too_many).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("doctor report has too many checks"));
    }

    #[cfg(unix)]
    #[test]
    fn ci_doctor_artifacts_reject_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-ci-doctor")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&external).unwrap();
        write_doctor_artifact(&external.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&external.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&external.join("doctor-Windows.json"), "Windows");
        unix_fs::symlink(&external, &root).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("doctor artifact path must not be a symlink")
        );
    }

    #[cfg(unix)]
    #[test]
    fn ci_release_check_artifacts_reject_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-ci-release-checks")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&external).unwrap();
        write_ci_release_check_artifact(&external.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&external.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&external.join("release-check-Windows.json"));
        unix_fs::symlink(&external, &root).unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("release-check artifact path must not be a symlink")
        );
    }

    #[test]
    fn release_check_accepts_ci_signing_and_notarization_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let doctor_dir = root.join("ci-doctor");
        fs::create_dir(&doctor_dir).unwrap();
        write_doctor_artifact(&doctor_dir.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&doctor_dir.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&doctor_dir.join("doctor-Windows.json"), "Windows");
        let ci_release_check_dir = root.join("ci-release-checks");
        fs::create_dir(&ci_release_check_dir).unwrap();
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Windows.json"));
        let macos_signing_log = root.join("signing-macos.log");
        fs::write(&macos_signing_log, "codesign=pass\n").unwrap();
        let windows_signing_log = root.join("signing-windows.log");
        fs::write(&windows_signing_log, "signtool=pass\n").unwrap();
        let notary_log = root.join("notary.log");
        fs::write(
            &notary_log,
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        let validate_report = root.join("validate-report.json");
        write_validate_artifact(&validate_report, "ok", "passed");
        let mut validate_reports = vec![validate_report];
        validate_reports.extend(write_example_validate_matrix(&root.join("validator")));
        let static_reports = write_example_static_validate_matrix(&root.join("package"));
        let platform_smoke_dir = root.join("platform-smoke");
        write_platform_smoke_matrix(&platform_smoke_dir);
        let publish_plan_report = root.join("publish-plan.json");
        write_publish_plan_artifact(&publish_plan_report);
        let crate_package_report = root.join("crate-package.json");
        write_crate_package_artifact(&crate_package_report);
        let npm_pack_report = root.join("npm-pack.json");
        write_npm_pack_artifact(&npm_pack_report);
        let dependency_baseline_report = root.join("dependency-baseline-latest.json");
        write_dependency_baseline_latest_artifact(&dependency_baseline_report);
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let protocol = root.join("protocol");
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let options = ReleaseEvidenceOptions {
            ci_doctor_dir: Some(doctor_dir),
            ci_release_check_dir: Some(ci_release_check_dir),
            platform_smoke_dir: Some(platform_smoke_dir),
            ci_run_url: Some(
                "https://github.com/vesty-rs/vesty/actions/runs/1234567890".to_string(),
            ),
            validate_reports,
            static_validate_reports: static_reports,
            publish_plan_report: Some(publish_plan_report),
            crate_package_report: Some(crate_package_report),
            npm_pack_report: Some(npm_pack_report),
            dependency_baseline_report: Some(dependency_baseline_report),
            vst3_sdk_manifest: None,
            vst3_sdk_binding_plan: None,
            vst3_sdk_binding_surface: None,
            vst3_sdk_scaffold: None,
            vst3_sdk_abi_seed: None,
            vst3_sdk_abi: None,
            vst3_sdk_interface_skeleton: None,
            signed_bundle_evidence: vec![macos_signing_log, windows_signing_log],
            notarization_log: Some(notary_log),
            require_release_artifacts: true,
        };

        let report = build_release_check_report(rows, &protocol, false, &options);

        assert!(release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci doctor artifacts"
                && check.status == "ok"
                && check.value.contains("3 doctor reports parsed")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci release-check artifacts"
                && check.status == "ok"
                && check.value.contains("3 release-check report")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "platform smoke artifacts"
                && check.status == "ok"
                && check.value.contains("macOS")
                && check.value.contains("Windows x64")
                && check.value.contains("Linux X11")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "signed bundle evidence"
                && check.status == "ok"
                && check.value.contains("macOS")
                && check.value.contains("Windows")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "notarization log" && check.status == "ok" })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 validate reports" && check.status == "ok" })
        );
        assert!(
            report.checks.iter().any(|check| {
                check.name == "vst3 static validate reports" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "crate publish plan"
                && check.status == "ok"
                && check.value.contains("publishable crates")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "crate package readiness"
                && check.status == "ok"
                && check.value.contains("packageable now")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm package pack report"
                && check.status == "ok"
                && check.value.contains("vesty-plugin-ui")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "dependency latest baseline"
                && check.status == "ok"
                && check.value.contains("latest registry check")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "ok"
        }));
    }

    #[test]
    fn release_evidence_dir_populates_standard_evidence_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        let doctor_dir = root.join("ci-doctor");
        fs::create_dir(&doctor_dir).unwrap();
        write_doctor_artifact(&doctor_dir.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&doctor_dir.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&doctor_dir.join("doctor-Windows.json"), "Windows");
        let ci_release_check_dir = root.join("ci-release-checks");
        fs::create_dir(&ci_release_check_dir).unwrap();
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Windows.json"));
        let platform_smoke_dir = root.join("platform-smoke");
        write_platform_smoke_matrix(&platform_smoke_dir);
        write_validate_artifact(&root.join("validate-report.json"), "ok", "passed");
        let example_validate_reports =
            write_example_validate_matrix(&root.join("downloaded-artifacts/validator"));
        write_validate_artifact(&root.join("static-validate-report.json"), "ok", "skipped");
        let static_matrix_reports =
            write_example_static_validate_matrix(&root.join("downloaded-artifacts/package"));
        let publish_plan_report = root.join("publish-plan/publish-plan.json");
        write_publish_plan_artifact(&publish_plan_report);
        let crate_package_report = root.join("crate-package/crate-package.json");
        write_crate_package_artifact(&crate_package_report);
        let npm_pack_report = root.join("npm-pack/npm-pack.json");
        write_npm_pack_artifact(&npm_pack_report);
        let dependency_baseline_report =
            root.join("dependency-baseline/dependency-baseline-latest.json");
        write_dependency_baseline_latest_artifact(&dependency_baseline_report);
        let vst3_sdk_manifest = root.join("vst3-sdk/vst3-sdk-headers.json");
        write_test_vst3_sdk_manifest(&vst3_sdk_manifest, &[]);
        let vst3_sdk_binding_plan = root.join("vst3-sdk/generated-bindings-plan.json");
        write_test_vst3_sdk_binding_plan(
            &vst3_sdk_binding_plan,
            &[],
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let vst3_sdk_binding_surface = root.join("vst3-sdk/generated-bindings-surface.json");
        write_test_vst3_sdk_binding_surface(&vst3_sdk_binding_surface, &[]);
        let vst3_sdk_scaffold = root.join("vst3-sdk/generated.rs");
        write_test_vst3_sdk_scaffold(
            &vst3_sdk_scaffold,
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let vst3_sdk_abi_seed = root.join("vst3-sdk/generated-abi-seed.rs");
        write_test_vst3_sdk_abi_seed(&vst3_sdk_abi_seed);
        let vst3_sdk_abi = root.join("vst3-sdk/generated-abi.rs");
        write_test_vst3_sdk_abi(&vst3_sdk_abi);
        let vst3_sdk_interface_skeleton = root.join("vst3-sdk/generated-interface-skeleton.rs");
        write_test_vst3_sdk_interface_skeleton(&vst3_sdk_interface_skeleton);
        fs::write(
            root.join("signing-macos.log"),
            "signed=true\ncodesign=pass\n",
        )
        .unwrap();
        fs::write(root.join("signing-windows.log"), "signtool=pass\n").unwrap();
        fs::write(
            root.join("notary.log"),
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let protocol = root.join("protocol");
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let mut options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        apply_release_evidence_dir(&mut options, &root).unwrap();
        let report = build_release_check_report(rows, &protocol, false, &options);

        assert!(release_check_complete(&report));
        assert_eq!(
            options.ci_run_url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890")
        );
        assert_eq!(options.ci_doctor_dir.as_deref(), Some(doctor_dir.as_path()));
        assert_eq!(
            options.ci_release_check_dir.as_deref(),
            Some(ci_release_check_dir.as_path())
        );
        assert_eq!(
            options.platform_smoke_dir.as_deref(),
            Some(platform_smoke_dir.as_path())
        );
        assert_eq!(
            options.publish_plan_report.as_deref(),
            Some(publish_plan_report.as_path())
        );
        assert_eq!(
            options.crate_package_report.as_deref(),
            Some(crate_package_report.as_path())
        );
        assert_eq!(
            options.npm_pack_report.as_deref(),
            Some(npm_pack_report.as_path())
        );
        assert_eq!(
            options.dependency_baseline_report.as_deref(),
            Some(dependency_baseline_report.as_path())
        );
        assert_eq!(
            options.vst3_sdk_manifest.as_deref(),
            Some(vst3_sdk_manifest.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_plan.as_deref(),
            Some(vst3_sdk_binding_plan.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_surface.as_deref(),
            Some(vst3_sdk_binding_surface.as_path())
        );
        assert_eq!(
            options.vst3_sdk_scaffold.as_deref(),
            Some(vst3_sdk_scaffold.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi_seed.as_deref(),
            Some(vst3_sdk_abi_seed.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi.as_deref(),
            Some(vst3_sdk_abi.as_path())
        );
        assert_eq!(
            options.vst3_sdk_interface_skeleton.as_deref(),
            Some(vst3_sdk_interface_skeleton.as_path())
        );
        assert_eq!(options.validate_reports, {
            let mut expected = vec![root.join("validate-report.json")];
            expected.extend(example_validate_reports.clone());
            expected
        });
        assert!(
            options
                .static_validate_reports
                .contains(&root.join("static-validate-report.json"))
        );
        for path in static_matrix_reports {
            assert!(options.static_validate_reports.contains(&path));
        }
        assert_eq!(options.notarization_log, Some(root.join("notary.log")));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "vst3 static validate reports" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "ok"
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "crate package readiness" && check.status == "ok" })
        );
        assert!(
            report.checks.iter().any(|check| {
                check.name == "dependency latest baseline" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "ok"
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 SDK header manifest" && check.status == "ok" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings plan" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings surface" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings scaffold" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings ABI seed" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings ABI layout" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings interface skeleton" && check.status == "ok"
        }));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_report_paths_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(root.join("publish-plan")).unwrap();
        fs::create_dir_all(root.join("crate-package")).unwrap();
        fs::create_dir_all(root.join("npm-pack")).unwrap();
        fs::create_dir_all(root.join("dependency-baseline")).unwrap();
        let publish_plan = root.join("publish-plan/publish-plan.json");
        let crate_package = root.join("crate-package/crate-package.json");
        let npm_pack = root.join("npm-pack/npm-pack.json");
        let dependency_baseline = root.join("dependency-baseline/dependency-baseline-latest.json");
        fs::write(&publish_plan, "{").unwrap();
        fs::write(&crate_package, r#"{ "packages": [] }"#).unwrap();
        fs::write(&npm_pack, r#"{ "not": "an npm pack array" }"#).unwrap();
        fs::write(&dependency_baseline, r#"{ "checks": [] }"#).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(
            options.publish_plan_report.as_deref(),
            Some(publish_plan.as_path())
        );
        assert_eq!(
            options.crate_package_report.as_deref(),
            Some(crate_package.as_path())
        );
        assert_eq!(options.npm_pack_report.as_deref(), Some(npm_pack.as_path()));
        assert_eq!(
            options.dependency_baseline_report.as_deref(),
            Some(dependency_baseline.as_path())
        );

        let publish_check =
            publish_plan_release_check(options.publish_plan_report.as_deref(), true);
        let crate_check = crate_package_release_check(
            options.crate_package_report.as_deref(),
            options.publish_plan_report.as_deref(),
            true,
        );
        let npm_check = npm_pack_release_check(options.npm_pack_report.as_deref(), true);
        let dependency_check = dependency_baseline_latest_release_check(
            options.dependency_baseline_report.as_deref(),
            true,
        );

        assert_eq!(publish_check.status, "failed");
        assert!(publish_check.value.contains("invalid publish plan JSON"));
        assert_eq!(crate_check.status, "failed");
        assert!(crate_check.value.contains("invalid crate package report"));
        assert_eq!(npm_check.status, "failed");
        assert!(npm_check.value.contains("invalid npm pack report JSON"));
        assert_eq!(dependency_check.status, "failed");
        assert!(!dependency_check.value.contains("required evidence missing"));
        assert!(dependency_check.value.contains("dependency"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_vst3_sdk_artifacts_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(root.join("vst3-sdk")).unwrap();
        let manifest = root.join("vst3-sdk/vst3-sdk-headers.json");
        let binding_plan = root.join("vst3-sdk/generated-bindings-plan.json");
        let binding_surface = root.join("vst3-sdk/generated-bindings-surface.json");
        let scaffold = root.join("vst3-sdk/generated.rs");
        let abi_seed = root.join("vst3-sdk/generated-abi-seed.rs");
        let abi = root.join("vst3-sdk/generated-abi.rs");
        let interface_skeleton = root.join("vst3-sdk/generated-interface-skeleton.rs");
        fs::write(&manifest, r#"{ "kind": "broken" }"#).unwrap();
        fs::write(&binding_plan, "{").unwrap();
        fs::write(
            &binding_surface,
            r#"{ "bindingsGenerated": true, "status": "generated" }"#,
        )
        .unwrap();
        fs::write(&scaffold, "pub const BINDINGS_GENERATED: bool = true;\n").unwrap();
        fs::write(
            &abi_seed,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            &abi,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            &interface_skeleton,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(
            options.vst3_sdk_manifest.as_deref(),
            Some(manifest.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_plan.as_deref(),
            Some(binding_plan.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_surface.as_deref(),
            Some(binding_surface.as_path())
        );
        assert_eq!(
            options.vst3_sdk_scaffold.as_deref(),
            Some(scaffold.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi_seed.as_deref(),
            Some(abi_seed.as_path())
        );
        assert_eq!(options.vst3_sdk_abi.as_deref(), Some(abi.as_path()));
        assert_eq!(
            options.vst3_sdk_interface_skeleton.as_deref(),
            Some(interface_skeleton.as_path())
        );

        let manifest_check = vst3_sdk_manifest_release_check(options.vst3_sdk_manifest.as_deref());
        let plan_check =
            vst3_sdk_binding_plan_release_check(options.vst3_sdk_binding_plan.as_deref());
        let surface_check =
            vst3_sdk_binding_surface_release_check(options.vst3_sdk_binding_surface.as_deref());
        let scaffold_check =
            vst3_sdk_generated_scaffold_release_check(options.vst3_sdk_scaffold.as_deref());
        let abi_seed_check =
            vst3_sdk_generated_abi_seed_release_check(options.vst3_sdk_abi_seed.as_deref());
        let abi_check = vst3_sdk_generated_abi_release_check(options.vst3_sdk_abi.as_deref());
        let interface_check = vst3_sdk_generated_interface_skeleton_release_check(
            options.vst3_sdk_interface_skeleton.as_deref(),
        );

        assert_eq!(manifest_check.status, "failed");
        assert!(
            manifest_check
                .value
                .contains(&portable_report_path(&manifest))
        );
        assert!(
            manifest_check
                .value
                .contains("invalid VST3 SDK header manifest JSON"),
            "{}",
            manifest_check.value
        );
        assert_eq!(plan_check.status, "failed");
        assert!(
            plan_check
                .value
                .contains(&portable_report_path(&binding_plan))
        );
        assert!(
            plan_check
                .value
                .contains("invalid VST3 SDK generated bindings plan JSON"),
            "{}",
            plan_check.value
        );
        assert_eq!(surface_check.status, "failed");
        assert!(
            surface_check
                .value
                .contains(&portable_report_path(&binding_surface))
        );
        assert!(
            surface_check
                .value
                .contains("invalid VST3 SDK generated bindings surface JSON"),
            "{}",
            surface_check.value
        );
        assert_eq!(scaffold_check.status, "failed");
        assert!(
            scaffold_check
                .value
                .contains(&portable_report_path(&scaffold))
        );
        assert!(
            scaffold_check
                .value
                .contains("must not claim SDK bindings are generated"),
            "{}",
            scaffold_check.value
        );
        assert_eq!(abi_seed_check.status, "failed");
        assert!(
            abi_seed_check
                .value
                .contains(&portable_report_path(&abi_seed))
        );
        assert!(
            abi_seed_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            abi_seed_check.value
        );
        assert_eq!(abi_check.status, "failed");
        assert!(abi_check.value.contains(&portable_report_path(&abi)));
        assert!(
            abi_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            abi_check.value
        );
        assert_eq!(interface_check.status, "failed");
        assert!(
            interface_check
                .value
                .contains(&portable_report_path(&interface_skeleton))
        );
        assert!(
            interface_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            interface_check.value
        );
        for check in [
            manifest_check,
            plan_check,
            surface_check,
            scaffold_check,
            abi_seed_check,
            abi_check,
            interface_check,
        ] {
            assert!(!check.value.contains("not requested"), "{check:?}");
        }
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_validate_reports_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        let validate_report = root.join("validate-report.json");
        let static_validate_report = root.join("static-validate-report.json");
        fs::write(&validate_report, "{").unwrap();
        write_validate_artifact(&static_validate_report, "failed", "skipped");
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.validate_reports, vec![validate_report.clone()]);
        assert_eq!(
            options.static_validate_reports,
            vec![static_validate_report.clone()]
        );

        let validate_check = validate_reports_release_check(&options.validate_reports, true);
        let static_check =
            static_validate_reports_release_check(&options.static_validate_reports, false);

        assert_eq!(validate_check.status, "failed");
        assert!(validate_check.value.contains(validate_report.as_str()));
        assert!(!validate_check.value.contains("required evidence missing"));
        assert_eq!(static_check.status, "failed");
        assert!(
            static_check
                .value
                .contains("static bundle check status is failed")
        );
        assert!(!static_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let validator_dir = root.join("validator");
        let package_dir = root.join("package");
        fs::create_dir_all(&validator_dir).unwrap();
        fs::create_dir_all(&package_dir).unwrap();
        let validate_report = validator_dir.join("VestyGain.macos.validate.json");
        let static_validate_report = package_dir.join("VestyGain.linux-x64.static-validate.json");
        let release_check_sidecar = package_dir.join("static-validate-release-check.json");
        let notes = root.join("notes.json");
        fs::write(&validate_report, "{").unwrap();
        write_validate_artifact(&static_validate_report, "failed", "skipped");
        fs::write(&release_check_sidecar, r#"{"status":"failed","checks":[]}"#).unwrap();
        fs::write(&notes, r#"{ "note": true }"#).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.validate_reports, vec![validate_report.clone()]);
        assert_eq!(
            options.static_validate_reports,
            vec![static_validate_report.clone()]
        );
        assert!(!options.validate_reports.contains(&notes));
        assert!(!options.static_validate_reports.contains(&notes));
        assert!(
            !options
                .static_validate_reports
                .contains(&release_check_sidecar)
        );

        let validate_check = validate_reports_release_check(&options.validate_reports, true);
        let static_check =
            static_validate_reports_release_check(&options.static_validate_reports, false);

        assert_eq!(validate_check.status, "failed");
        assert!(validate_check.value.contains(validate_report.as_str()));
        assert!(
            validate_check
                .value
                .contains("invalid validate report JSON")
        );
        assert!(!validate_check.value.contains("required evidence missing"));
        assert_eq!(static_check.status, "failed");
        assert!(
            static_check
                .value
                .contains("static bundle check status is failed")
        );
        assert!(!static_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_signing_and_notary_logs_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        let macos_signing = root.join("signing-macos.log");
        let windows_signing = root.join("signing-windows.log");
        let notary_log = root.join("notary.log");
        fs::write(&macos_signing, "codesign=pass\ninvalid signature\n").unwrap();
        fs::write(
            &windows_signing,
            "signtool verify /pa /v VestyGain.vst3\nNumber of errors: 1\n",
        )
        .unwrap();
        fs::write(&notary_log, "status: Rejected\n").unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert!(options.signed_bundle_evidence.contains(&macos_signing));
        assert!(options.signed_bundle_evidence.contains(&windows_signing));
        assert_eq!(options.notarization_log, Some(notary_log.clone()));

        let signing_check =
            signed_bundle_evidence_release_check(&options.signed_bundle_evidence, true);
        let notary_check =
            notarization_log_release_check(options.notarization_log.as_deref(), true);

        assert_eq!(signing_check.status, "failed");
        assert!(signing_check.value.contains("invalid signature"));
        assert!(signing_check.value.contains("number of errors: 1"));
        assert!(!signing_check.value.contains("required evidence missing"));
        assert_eq!(notary_check.status, "failed");
        assert!(notary_check.value.contains("status: rejected"));
        assert!(
            notary_check
                .value
                .contains("negative notarization evidence")
        );
        assert!(!notary_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_accepts_matching_explicit_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/2\n",
        )
        .unwrap();
        let cli_url = "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1";
        let mut options = ReleaseEvidenceOptions {
            ci_run_url: Some(cli_url.to_string()),
            ..ReleaseEvidenceOptions::default()
        };

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.ci_run_url.as_deref(), Some(cli_url));
    }

    #[test]
    fn release_evidence_dir_rejects_mismatched_explicit_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/other/actions/runs/1234567890\n",
        )
        .unwrap();
        let mut options = ReleaseEvidenceOptions {
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ..ReleaseEvidenceOptions::default()
        };

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("mismatched explicit release evidence URL should fail")
            .to_string();

        assert!(error.contains("refer to different GitHub Actions runs"));
        assert!(error.contains("vesty-rs/vesty, run 42"));
        assert!(error.contains("vesty-rs/other, run 1234567890"));
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_ci_run_url_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-ci-run-url.txt")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &external,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        unix_fs::symlink(&external, root.join("ci-run-url.txt")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked ci-run-url.txt")
            .to_string();

        assert!(error.contains("CI run URL evidence must not be a symlink"));
        assert!(options.ci_run_url.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_root_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-release-evidence")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&external).unwrap();
        write_publish_plan_artifact(&external.join("publish-plan.json"));
        unix_fs::symlink(&external, &root).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir root symlink should be rejected")
            .to_string();

        assert!(error.contains("release evidence dir must not be a symlink"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_standard_file_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-publish-plan.json")).unwrap();
        fs::create_dir_all(root.join("publish-plan")).unwrap();
        write_publish_plan_artifact(&external);
        unix_fs::symlink(&external, root.join("publish-plan/publish-plan.json")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked standard files")
            .to_string();

        assert!(error.contains("release evidence path must not be a symlink"));
        assert!(error.contains("publish-plan.json"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_standard_dir_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-publish-plan")).unwrap();
        fs::create_dir_all(&external).unwrap();
        write_publish_plan_artifact(&external.join("publish-plan.json"));
        fs::create_dir_all(&root).unwrap();
        unix_fs::symlink(&external, root.join("publish-plan")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked standard dirs")
            .to_string();

        assert!(error.contains("release evidence path must not be a symlink"));
        assert!(error.contains("publish-plan"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_validator_and_package_symlink_dirs() {
        for dir_name in ["validator", "package"] {
            let temp = tempfile::tempdir().unwrap();
            let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
            let external =
                Utf8PathBuf::from_path_buf(temp.path().join(format!("external-{dir_name}")))
                    .unwrap();
            fs::create_dir_all(&root).unwrap();
            fs::create_dir_all(&external).unwrap();
            write_validate_artifact(
                &external.join(format!("{dir_name}.json")),
                "ok",
                if dir_name == "validator" {
                    "passed"
                } else {
                    "skipped"
                },
            );
            unix_fs::symlink(&external, root.join(dir_name)).unwrap();
            let mut options = ReleaseEvidenceOptions::default();

            let error = apply_release_evidence_dir(&mut options, &root)
                .expect_err("release evidence dir must reject symlinked matrix dirs")
                .to_string();

            assert!(error.contains("JSON artifact contains symlink"));
            assert!(error.contains(dir_name));
            assert!(options.validate_reports.is_empty());
            assert!(options.static_validate_reports.is_empty());
        }
    }

    #[test]
    fn ci_run_url_file_accepts_raw_url_and_named_key_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "# release evidence\n\
             note=https://github.com/vesty-rs/vesty/actions/runs/111\n\
             ci-run-url=pending\n\
             ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();

        let url = read_ci_run_url_file(&file).unwrap();

        assert_eq!(
            url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890")
        );

        fs::write(&file, "https://github.com/vesty-rs/vesty/actions/runs/42\n").unwrap();

        assert_eq!(
            read_ci_run_url_file(&file).unwrap().as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/42")
        );
    }

    #[test]
    fn ci_run_url_file_ignores_unrelated_key_value_lines() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "note=https://github.com/vesty-rs/vesty/actions/runs/111\n\
             ci_run_url=PENDING\n",
        )
        .unwrap();

        assert_eq!(read_ci_run_url_file(&file).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn ci_run_url_file_rejects_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-ci-run-url.txt");
        let file = root.join("ci-run-url.txt");
        fs::write(
            &external,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        unix_fs::symlink(&external, &file).unwrap();

        let error = read_ci_run_url_file(&file)
            .expect_err("CI run URL evidence symlink should be rejected")
            .to_string();

        assert!(error.contains("CI run URL evidence must not be a symlink"));
    }

    #[test]
    fn import_ci_run_url_evidence_accepts_matching_cli_and_file_urls() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        let file = root.join("ci-run-url.txt");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            &file,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/2\n",
        )
        .unwrap();
        let options = ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: Some(
                "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1".to_string(),
            ),
            ci_run_url_file: Some(file.clone()),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let url = import_ci_run_url_evidence(&options, &[], &mut items).unwrap();

        assert_eq!(
            url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1")
        );
        assert_eq!(
            read_ci_run_url_file(&evidence.join("ci-run-url.txt"))
                .unwrap()
                .as_deref(),
            url.as_deref()
        );
        assert!(items.iter().any(|item| {
            item.name == "ci run url"
                && item.status == "imported"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_paths_equal(source, file.as_str()))
        }));
    }

    #[test]
    fn import_ci_run_url_evidence_rejects_mismatched_cli_and_file_urls() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "ci_run_url=https://github.com/vesty-rs/other/actions/runs/1234567890\n",
        )
        .unwrap();
        let options = ImportCiOptions {
            source: root.join("downloaded-artifacts"),
            dir: root.join("release-evidence"),
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ci_run_url_file: Some(file),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let error = import_ci_run_url_evidence(&options, &[], &mut items)
            .expect_err("mismatched explicit CI run URL sources should fail")
            .to_string();

        assert!(error.contains("refer to different GitHub Actions runs"));
        assert!(error.contains("vesty-rs/vesty, run 42"));
        assert!(error.contains("vesty-rs/other, run 1234567890"));
        assert!(items.is_empty());
    }

    #[test]
    fn import_ci_run_url_evidence_rejects_invalid_file_even_when_cli_url_is_valid() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(&file, "ci_run_url=not-a-github-run-url\n").unwrap();
        let options = ImportCiOptions {
            source: root.join("downloaded-artifacts"),
            dir: root.join("release-evidence"),
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ci_run_url_file: Some(file),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let error = import_ci_run_url_evidence(&options, &[], &mut items)
            .expect_err("invalid CI run URL file should fail even with a valid CLI URL")
            .to_string();

        assert!(error.contains("--ci-run-url-file"));
        assert!(error.contains("is not a valid GitHub Actions run URL"));
        assert!(items.is_empty());
    }

    #[test]
    fn import_ci_reports_invalid_auto_discovered_ci_run_url_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("ci-run-url.txt"), "ci_run_url=not-a-run-url\n").unwrap();
        fs::write(
            source.join("notes.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/latest\n",
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

        assert!(!evidence.join("ci-run-url.txt").exists());
        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "ci run url"
                && item.status == "failed"
                && item.value.contains("invalid GitHub Actions run URL")
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "ci-run-url.txt"))
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
