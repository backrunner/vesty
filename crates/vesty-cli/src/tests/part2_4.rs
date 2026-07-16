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
        let package_json = ui_package_json("My Plugin", UiTemplate::Vanilla, &package_paths);
        let value = serde_json::from_str::<serde_json::Value>(&package_json).unwrap();
        assert_eq!(value["private"], true);
        assert_eq!(value["name"], "my-plugin-editor");
        assert_ne!(value["name"], "vesty-plugin-ui");
        assert_eq!(
            value["dependencies"]["vesty-plugin-ui"],
            "file:/tmp/vesty plugin-ui"
        );

        let published = ui_package_json(
            "My Plugin",
            UiTemplate::Vanilla,
            &UiPackagePaths::default(),
        );
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
            assert!(!source.contains("pub(super)"), "{source}");
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
            let tsconfig = fs::read_to_string(root.join("ui/tsconfig.json")).unwrap();
            let index_html = fs::read_to_string(root.join("ui/index.html")).unwrap();
            let vite_config = fs::read_to_string(root.join("ui/vite.config.ts")).unwrap();
            let package = serde_json::from_str::<serde_json::Value>(&package_json).unwrap();
            assert_eq!(package["private"], true);
            assert!(tsconfig.contains("\"preserveSymlinks\": true"));
            assert!(vite_config.contains("preserveSymlinks: true"));

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
