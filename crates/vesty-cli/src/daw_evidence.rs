use super::*;

pub(super) struct DawEvidencePaths {
    pub(super) reaper: Utf8PathBuf,
    pub(super) cubase: Utf8PathBuf,
    pub(super) bitwig: Utf8PathBuf,
    pub(super) ableton: Utf8PathBuf,
    pub(super) studio_one: Utf8PathBuf,
}

impl DawEvidencePaths {
    pub(super) fn from_root(root: &Utf8Path) -> Self {
        Self {
            reaper: root.join("reaper"),
            cubase: root.join("cubase"),
            bitwig: root.join("bitwig"),
            ableton: root.join("ableton"),
            studio_one: root.join("studio-one"),
        }
    }
}

pub(super) fn resolve_daw_evidence_paths(
    evidence_root: Option<Utf8PathBuf>,
    reaper: Utf8PathBuf,
    cubase: Utf8PathBuf,
    bitwig: Utf8PathBuf,
    ableton: Utf8PathBuf,
    studio_one: Utf8PathBuf,
) -> DawEvidencePaths {
    evidence_root
        .as_deref()
        .map(DawEvidencePaths::from_root)
        .unwrap_or(DawEvidencePaths {
            reaper,
            cubase,
            bitwig,
            ableton,
            studio_one,
        })
}

#[derive(Clone, Debug, Default)]
pub(super) struct DawSmokeReportInput {
    pub(super) host: Option<String>,
    pub(super) platform: Option<String>,
    pub(super) scan: Option<String>,
    pub(super) load: Option<String>,
    pub(super) ui: Option<String>,
    pub(super) ui_host_param: Option<String>,
    pub(super) meter_stream: Option<String>,
    pub(super) automation: Option<String>,
    pub(super) buffer_sample_rate_change: Option<String>,
    pub(super) save_restore: Option<String>,
    pub(super) offline_render: Option<String>,
}

pub(super) const DAW_SMOKE_MARKER_MAX_BYTES: usize = 256 * 1024;

pub(super) fn write_daw_smoke_report(
    evidence: &DawEvidencePaths,
    input: DawSmokeReportInput,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let host_raw = input
        .host
        .as_deref()
        .ok_or("DAW smoke report requires `--host <reaper|cubase|bitwig|ableton|studio-one>`")?;
    validate_release_action_text("DAW smoke report host", host_raw)?;
    let profile = vesty_core::find_host_profile(host_raw)
        .ok_or_else(|| format!("unknown host profile '{host_raw}'"))?;
    let dir = daw_evidence_dir_for_host(evidence, profile.id).to_path_buf();
    create_directory_no_parent_or_leaf_symlink("DAW evidence directory", &dir)?;

    let platform = required_daw_platform_marker(profile, input.platform)?;
    let scan = required_daw_marker("scan", input.scan)?;
    let load = required_daw_marker("load", input.load)?;
    let ui = required_daw_marker("ui", input.ui)?;
    let ui_host_param = required_daw_marker("ui-host-param", input.ui_host_param)?;
    let meter_stream = required_daw_marker("meter-stream", input.meter_stream)?;
    let automation = required_daw_marker("automation", input.automation)?;
    let buffer_sample_rate_change =
        required_daw_marker("buffer-sample-rate-change", input.buffer_sample_rate_change)?;
    let save_restore = required_daw_marker("save-restore", input.save_restore)?;
    let offline_render = required_daw_marker("offline-render", input.offline_render)?;

    validate_daw_smoke_report_markers(DawSmokeReportMarkers {
        profile,
        evidence_dir: &dir,
        scan: &scan,
        load: &load,
        ui: &ui,
        ui_host_param: &ui_host_param,
        meter_stream: &meter_stream,
        automation: &automation,
        buffer_sample_rate_change: &buffer_sample_rate_change,
        save_restore: &save_restore,
        offline_render: &offline_render,
    })?;

    write_text_file(&dir.join("platform.txt"), &(platform + "\n"))?;
    write_text_file(&dir.join("scan-smoke.log"), &(scan + "\n"))?;
    write_text_file(&dir.join("load-smoke.log"), &(load + "\n"))?;
    write_text_file(&dir.join("ui-smoke.log"), &(ui + "\n"))?;
    write_text_file(&dir.join("ui-host-smoke.log"), &(ui_host_param + "\n"))?;
    write_text_file(&dir.join("meter-stream.log"), &(meter_stream + "\n"))?;
    write_text_file(&dir.join("automation-smoke.log"), &(automation + "\n"))?;
    write_text_file(
        &dir.join("buffer-sample-rate.log"),
        &(buffer_sample_rate_change + "\n"),
    )?;
    write_text_file(&dir.join("restore-smoke.log"), &(save_restore + "\n"))?;
    write_text_file(&dir.join("offline-render.log"), &(offline_render + "\n"))?;

    let row = collect_daw_evidence_for_profile(profile, &dir);
    if !daw_row_complete(&row) {
        let missing = daw_missing_checks(&row);
        return Err(format!(
            "written DAW smoke evidence did not pass parser for {}: missing {}",
            profile.name,
            missing.join(", ")
        )
        .into());
    }
    Ok(dir)
}

pub(super) struct DawSmokeReportMarkers<'a> {
    pub(super) profile: &'a vesty_core::HostProfile,
    pub(super) evidence_dir: &'a Utf8Path,
    pub(super) scan: &'a str,
    pub(super) load: &'a str,
    pub(super) ui: &'a str,
    pub(super) ui_host_param: &'a str,
    pub(super) meter_stream: &'a str,
    pub(super) automation: &'a str,
    pub(super) buffer_sample_rate_change: &'a str,
    pub(super) save_restore: &'a str,
    pub(super) offline_render: &'a str,
}

pub(super) fn validate_daw_smoke_report_markers(
    markers: DawSmokeReportMarkers<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let checks = [
        (
            "scan",
            daw_marker_matches_for_profile(markers.profile, markers.scan, generic_scan_ok),
        ),
        (
            "load",
            daw_marker_matches_for_profile(markers.profile, markers.load, generic_load_ok),
        ),
        (
            "ui",
            daw_marker_matches_for_profile(markers.profile, markers.ui, generic_ui_ok),
        ),
        (
            "ui-host-param",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.ui_host_param,
                generic_ui_host_ok,
            ),
        ),
        (
            "meter-stream",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.meter_stream,
                meter_stream_delivered,
            ),
        ),
        (
            "automation",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.automation,
                generic_automation_ok,
            ),
        ),
        (
            "buffer-sample-rate-change",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.buffer_sample_rate_change,
                generic_buffer_sample_rate_change_ok,
            ),
        ),
        (
            "save-restore",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.save_restore,
                generic_restore_ok,
            ),
        ),
        (
            "offline-render",
            daw_marker_matches_for_profile(markers.profile, markers.offline_render, |text| {
                generic_offline_render_ok(text)
                    || render_file_evidence_ok(text, markers.evidence_dir)
            }),
        ),
    ];

    let missing = checks
        .iter()
        .filter_map(|(name, ok)| (!ok).then_some(*name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "DAW smoke report markers did not pass parser: {}",
            missing.join(", ")
        )
        .into())
    }
}

pub(super) fn daw_evidence_dir_for_host<'a>(
    evidence: &'a DawEvidencePaths,
    host_id: &str,
) -> &'a Utf8PathBuf {
    match host_id {
        "reaper" => &evidence.reaper,
        "cubase-nuendo" => &evidence.cubase,
        "bitwig" => &evidence.bitwig,
        "ableton-live" => &evidence.ableton,
        "studio-one" => &evidence.studio_one,
        _ => &evidence.reaper,
    }
}

pub(super) fn required_daw_marker(
    name: &str,
    value: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err(format!("DAW smoke report requires `--{name}`").into());
    };
    validate_daw_smoke_marker_text(name, &value)?;
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || lower == "pending"
        || lower == "false"
        || lower.starts_with("pending ")
        || lower.contains("manual platform pending")
        || lower.contains("replace with real")
        || daw_marker_has_missing_assignment(trimmed)
        || daw_marker_has_negative_evidence(trimmed)
    {
        return Err(format!("DAW smoke report `{name}` is missing positive evidence").into());
    }
    if name == "meter-stream" && !meter_stream_delivered(trimmed) {
        return Err(format!("DAW smoke report `{name}` is missing nonzero meter evidence").into());
    }
    Ok(trimmed.to_string())
}

pub(super) fn required_daw_platform_marker(
    profile: &vesty_core::HostProfile,
    value: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err("DAW smoke report requires `--platform`".into());
    };
    validate_daw_smoke_marker_text("platform", &value)?;
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || lower == "pending"
        || lower.contains("manual platform pending")
        || lower.contains("replace with real")
        || lower.contains("wayland")
        || daw_marker_has_negative_evidence(trimmed)
    {
        return Err(
            "DAW smoke report `platform` is missing supported host/platform evidence".into(),
        );
    }
    let Some(platform) = daw_smoke_platform_from_text(trimmed) else {
        return Err(format!(
            "DAW smoke report `platform` must mention a supported platform for {}: {}",
            profile.name,
            profile.platforms.join(", ")
        )
        .into());
    };
    if !profile.platforms.contains(&platform) {
        return Err(format!(
            "DAW smoke report `platform` `{platform}` is not supported by {} profile (expected one of {})",
            profile.name,
            profile.platforms.join(", ")
        )
        .into());
    }
    Ok(trimmed.to_string())
}

pub(super) fn daw_smoke_platform_from_text(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("macos")
        || lower.contains("mac os")
        || lower.contains("darwin")
        || lower.contains("osx")
    {
        Some("macos")
    } else if lower.contains("windows") || lower.contains("win64") || lower.contains("win32") {
        Some("windows")
    } else if lower.contains("linux") && lower.contains("x11") {
        Some("linux-x11")
    } else if lower.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

pub(super) fn validate_daw_smoke_marker_text(
    name: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let label = format!("DAW smoke report `{name}`");
    if name == "platform" {
        validate_release_action_text(&label, value)?;
        return Ok(());
    }
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty").into());
    }
    if value.len() > DAW_SMOKE_MARKER_MAX_BYTES {
        return Err(format!("{label} must be at most {DAW_SMOKE_MARKER_MAX_BYTES} bytes").into());
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(
            format!("{label} must not contain control characters other than tab/newline").into(),
        );
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!("{label} must not contain unsafe Unicode format characters").into());
    }
    Ok(())
}

pub(super) fn daw_marker_has_negative_evidence(text: &str) -> bool {
    text.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        if split_marker_assignment(&lower)
            .is_some_and(|(key, _)| key.trim().replace('-', "_") == "render_file")
        {
            return false;
        }
        [
            "not found",
            "not installed",
            "unavailable",
            "failed",
            "failure",
            "error",
            "timeout",
            "timed out",
            "crashed",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    })
}

pub(super) fn daw_marker_has_missing_assignment(text: &str) -> bool {
    text.lines().any(|line| {
        line.split(';').any(|fragment| {
            let normalized = fragment.trim().to_ascii_lowercase().replace('-', "_");
            let Some((_, raw_value)) = split_marker_assignment(&normalized) else {
                return false;
            };
            let value = raw_value
                .trim()
                .trim_start_matches(['`', '"', '\''])
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .find(|token| !token.is_empty())
                .unwrap_or_default();
            matches!(value, "pending" | "false")
        })
    })
}

pub(super) fn write_daw_evidence_templates(
    evidence: &DawEvidencePaths,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut created = 0;
    created += write_host_evidence_template("REAPER", &evidence.reaper)?;
    created += write_host_evidence_template("Cubase/Nuendo", &evidence.cubase)?;
    created += write_host_evidence_template("Bitwig Studio", &evidence.bitwig)?;
    created += write_host_evidence_template("Ableton Live", &evidence.ableton)?;
    created += write_host_evidence_template("Studio One", &evidence.studio_one)?;
    Ok(created)
}

pub(super) fn write_host_evidence_template(
    host: &str,
    dir: &Utf8PathBuf,
) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &host_evidence_readme(host))?;
    created += write_template_file(dir.join("platform.txt"), "manual platform pending\n")?;
    created += write_template_file(dir.join("scan-smoke.log"), "scan=pending\n")?;
    created += write_template_file(dir.join("load-smoke.log"), "load=pending\n")?;
    created += write_template_file(dir.join("ui-smoke.log"), "ui=pending\n")?;
    created += write_template_file(dir.join("ui-host-smoke.log"), "ui_host_param=pending\n")?;
    created += write_template_file(dir.join("meter-stream.log"), "meter_flush sent=0\n")?;
    created += write_template_file(dir.join("automation-smoke.log"), "automation=pending\n")?;
    created += write_template_file(
        dir.join("buffer-sample-rate.log"),
        "buffer_sample_rate_change=pending\n",
    )?;
    created += write_template_file(dir.join("restore-smoke.log"), "save_restore=pending\n")?;
    created += write_template_file(dir.join("offline-render.log"), "offline_render=pending\n")?;
    Ok(created)
}

pub(super) fn host_evidence_readme(host: &str) -> String {
    let profile = vesty_core::find_host_profile(host);
    let mut text = String::new();
    text.push_str(&format!("# {host} Vesty Smoke Evidence\n\n"));
    text.push_str(
        "Fill these files after a real host smoke run. Leave `pending` values untouched until the evidence exists.\n\n",
    );
    text.push_str(
        "Templates, pending values and `vesty doctor` install detection do not count as pass evidence.\n\n",
    );

    if let Some(profile) = profile {
        text.push_str("## Host Profile\n\n");
        text.push_str(&format!("- Host id: `{}`\n", profile.id));
        text.push_str(&format!("- Platforms: {}\n", profile.platforms.join(", ")));
        text.push_str(&format!(
            "- Required smoke checks: {}\n",
            profile.required_smoke_checks.join(", ")
        ));
        if !profile.notes.is_empty() {
            text.push_str("- Notes:\n");
            for note in profile.notes {
                text.push_str(&format!("  - {note}\n"));
            }
        }
        text.push('\n');

        if !profile.quirks.is_empty() {
            text.push_str("## Host Quirks\n\n");
            text.push_str("| Area | Severity | Summary | Mitigation |\n");
            text.push_str("| --- | --- | --- | --- |\n");
            for quirk in profile.quirks {
                text.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    host_quirk_area_text(quirk.area),
                    host_quirk_severity_text(quirk.severity),
                    markdown_cell(quirk.summary),
                    markdown_cell(quirk.mitigation)
                ));
            }
            text.push('\n');
        }
    } else {
        text.push_str("## Expected Checks\n\n");
        text.push_str(&format!(
            "- Required smoke checks: {}\n\n",
            vesty_core::RELEASE_SMOKE_CHECKS.join(", ")
        ));
    }

    text.push_str("## Evidence Files\n\n");
    text.push_str("- `platform.txt`: exact OS, architecture and host version.\n");
    text.push_str("- `scan-smoke.log`: plugin scan result for all Vesty examples.\n");
    text.push_str("- `load-smoke.log`: instance creation/load result.\n");
    text.push_str("- `ui-smoke.log`: editor open/close result.\n");
    text.push_str("- `ui-host-smoke.log`: UI parameter edit observed by the host.\n");
    text.push_str(
        "- `meter-stream.log`: nonzero `meter.main` stream or `meter_flush sent=N` evidence.\n",
    );
    text.push_str("- `automation-smoke.log`: host automation record/playback result.\n");
    text.push_str("- `buffer-sample-rate.log`: buffer size and sample-rate change result.\n");
    text.push_str("- `restore-smoke.log`: save, close and restore result.\n");
    text.push_str("- `offline-render.log`: offline render result or `render_file=/absolute/path.wav` marker.\n");
    text.push_str("\n## Accepted Pass Markers\n\n");
    text.push_str(
        "Use these exact markers when the corresponding real smoke check has passed:\n\n",
    );
    text.push_str("```text\n");
    text.push_str("scan=true\n");
    text.push_str("load=true\n");
    text.push_str("ui=true\n");
    text.push_str("ui_host_param=true\n");
    text.push_str("meter_flush sent=1\n");
    text.push_str("automation=true\n");
    text.push_str("buffer_sample_rate_change=true\n");
    text.push_str("save_restore=true\n");
    text.push_str("offline_render=true\n");
    text.push_str("render_file=/absolute/path/to/rendered.wav\n");
    text.push_str("```\n\n");
    text.push_str("After filling evidence, run:\n\n");
    text.push_str("```bash\n");
    text.push_str("vesty daw-matrix --evidence-root target/daw-evidence --format markdown\n");
    text.push_str("vesty daw-matrix --evidence-root target/daw-evidence --strict\n");
    text.push_str("```\n");
    text
}

pub(super) fn write_release_evidence_templates(
    dir: &Utf8PathBuf,
) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    create_template_dir(&dir.join("ci-doctor"))?;
    create_template_dir(&dir.join("ci-release-checks"))?;
    create_template_dir(&dir.join("platform-smoke"))?;
    create_template_dir(&dir.join("validator"))?;
    create_template_dir(&dir.join("package"))?;
    create_template_dir(&dir.join("publish-plan"))?;
    create_template_dir(&dir.join("crate-package"))?;
    create_template_dir(&dir.join("npm-pack"))?;
    create_template_dir(&dir.join("dependency-baseline"))?;
    create_template_dir(&dir.join("vst3-sdk"))?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &release_evidence_readme())?;
    created += write_template_file(
        dir.join("ci-run-url.txt"),
        "ci_run_url=pending\n# pass the real URL with --ci-run-url\n",
    )?;
    let validate_report_template = pending_validate_report_template()?;
    created += write_template_file(dir.join("validate-report.json"), &validate_report_template)?;
    let static_validate_report_template = pending_static_validate_report_template()?;
    created += write_template_file(
        dir.join("static-validate-report.json"),
        &static_validate_report_template,
    )?;
    created += write_template_file(dir.join("ci-doctor/README.md"), &ci_doctor_readme())?;
    created += write_template_file(
        dir.join("ci-release-checks/README.md"),
        &ci_release_checks_readme(),
    )?;
    created += write_template_file(
        dir.join("platform-smoke/README.md"),
        &platform_smoke_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("publish-plan/README.md"),
        &publish_plan_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("crate-package/README.md"),
        &crate_package_evidence_readme(),
    )?;
    created += write_template_file(dir.join("npm-pack/README.md"), &npm_pack_evidence_readme())?;
    created += write_template_file(
        dir.join("dependency-baseline/README.md"),
        &dependency_baseline_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("vst3-sdk/README.md"),
        &vst3_sdk_manifest_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("signing-macos.log"),
        "signed=pending\ncodesign verify=pending\n",
    )?;
    created += write_template_file(
        dir.join("signing-windows.log"),
        "signed=pending\nsigntool verify=pending\n",
    )?;
    created += write_template_file(
        dir.join("notary.log"),
        "notarization=pending\nstapled=pending\n",
    )?;
    Ok(created)
}

pub(super) fn release_evidence_readme() -> String {
    [
        "# Vesty Release Artifact Evidence",
        "",
        "Fill these files after real release verification. Leave `pending` values untouched until the evidence exists.",
        "",
        "Templates and pending values do not count as pass evidence.",
        "",
        "Local evidence helper:",
        "",
        "```bash",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol",
        "",
        "# Optional explicit online dependency latest review:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --dependency-baseline-latest",
        "",
        "# Optional crate package readiness smoke:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --crate-package",
        "",
        "# Optional VST3 SDK audit evidence when an official SDK checkout is available:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --vst3-sdk-dir /path/to/VST_SDK \\",
        "  --vst3-sdk-bindings-module target/vst3-sdk/generated.rs",
        "```",
        "",
        "`collect-local` writes the template, exports/checks the protocol snapshot, and generates crate publish-plan plus npm pack dry-run reports from real local commands. With `--crate-package`, it also runs `vesty crate-package` and writes `crate-package/crate-package.json`; this proves currently packageable leaf crates and records deferred crates that require already published internal dependencies. With `--dependency-baseline-latest`, it also runs the explicit online crates.io/npm latest dependency review and writes `dependency-baseline/dependency-baseline-latest.json`. With `--vst3-sdk-dir <official-vst3sdk>`, it also writes `vst3-sdk/vst3-sdk-headers.json`, `vst3-sdk/generated-bindings-plan.json`, `vst3-sdk/generated-bindings-surface.json`, metadata-only `vst3-sdk/generated.rs`, ABI seed `vst3-sdk/generated-abi-seed.rs`, ABI layout `vst3-sdk/generated-abi.rs`, and interface/vtable skeleton plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata, pure IID/dispatch lookup helpers, and pure binary export required-symbol / inspection-tool helpers in `vst3-sdk/generated-interface-skeleton.rs` as optional generated-headers audit evidence. The binding plan and surface must keep `bindingsGenerated: false`; the scaffold must keep `BINDINGS_GENERATED = false`; the ABI seed, ABI layout and interface skeleton must keep `FULL_COM_BINDINGS_GENERATED = false`; the ABI layout must keep `ABI_LAYOUT_GENERATED = true` plus layout size/alignment and field-offset fingerprints; the interface skeleton must keep `INTERFACE_SKELETON_GENERATED = true` and preserve audit-only method order/signature metadata plus vtable local slot/field/callback type alias/field layout seeds, offset fingerprints, interface IID records, queryInterface planned dispatch entries, pure lookup helpers, Vesty COM object interface exposure records, object FUnknown identity plans, per-object queryInterface dispatch records, factory export plan records, processor/controller factory class plan records, platform module export plan records, binary export symbol plan records, binary export inspection tool plan records, and binary export required-symbol lookup/missing-symbol helpers. These are readiness/surface/drift/interface/ABI/COM identity/dispatch/exposure-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan artifacts, not proof that full SDK 3.8 bindings are generated. It does not inspect plugin binaries and does not create DAW, platform smoke, validator-passed, CI, signing or notarization evidence.",
        "",
        "CI artifact import helper:",
        "",
        "```bash",
        "vesty release-evidence import-ci \\",
        "  --source target/downloaded-artifacts \\",
        "  --dir target/release-evidence \\",
        "  --ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>",
        "```",
        "",
        "`import-ci` scans an already-downloaded GitHub Actions artifact directory, content-validates recognized artifacts, and copies only accepted evidence into the standard release evidence layout. It can import crate package readiness into `crate-package/crate-package.json`, the explicit latest dependency review into `dependency-baseline/dependency-baseline-latest.json`, and valid per-OS release action plan sidecars into `ci-release-checks/release-action-plan-<OS>.json`; an offline `dependency-baseline.json` drift report is not accepted as release latest evidence. Invalid action plan sidecars are recorded as failed and not copied, and valid sidecars remain checklist metadata rather than release-check pass evidence. For optional VST3 SDK artifacts it can import valid header manifest, generated-bindings plan, generated-bindings surface, metadata-only `generated.rs` scaffold, `generated-abi-seed.rs` ABI seed, `generated-abi.rs` ABI layout, and `generated-interface-skeleton.rs` interface/vtable skeleton plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata and pure IID/dispatch lookup helper artifacts into `vst3-sdk/`; release-check validates these SDK audit files when they are present, but they remain drift/audit metadata rather than proof that full SDK 3.8 bindings or final release readiness exist. It writes `import-ci-report.json`, preserves existing files by default, and supports `--overwrite` for a deliberate re-import. It does not synthesize CI, DAW, validator, signing or notarization passes.",
        "",
        "Signing and notarization evidence helpers:",
        "",
        "```bash",
        "vesty release-evidence collect-signing target/vesty/MyPlugin.vst3 \\",
        "  --platform macos \\",
        "  --dir target/release-evidence",
        "vesty release-evidence collect-signing target/vesty/MyPlugin.vst3 \\",
        "  --platform windows-x64 \\",
        "  --dir target/release-evidence",
        "vesty release-evidence collect-notarization \\",
        "  --notary-log target/notarytool.log \\",
        "  --stapler-log target/stapler.log \\",
        "  --dir target/release-evidence",
        "```",
        "",
        "`collect-signing` runs real `codesign --verify` or `signtool verify` and writes the captured log only after the evidence parser accepts the platform coverage. `collect-notarization` combines real notarytool and stapler logs and writes `notary.log` only when both acceptance and stapler success are present. Linux signing remains release-channel specific and is not produced by these helpers.",
        "",
        "Suggested strict gate:",
        "",
        "```bash",
        "vesty daw-matrix --write-template --evidence-root target/daw-evidence",
        "vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke",
        "vesty export-types --out target/vesty-protocol --check",
        "vesty validate target/vesty/MyPlugin.vst3 --strict --format json --report target/release-evidence/validator/MyPlugin.macos.validate.json --validator-log target/release-evidence/validator/MyPlugin.macos.validator.log",
        "vesty release-check --strict --require-release-artifacts \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --evidence-root target/daw-evidence \\",
        "  --release-evidence-dir target/release-evidence \\",
        "  --plan target/release-evidence/release-action-plan.json \\",
        "  --report target/release-evidence/release-check.json",
        "```",
        "",
        "`--plan` writes a machine-readable action plan derived from the current release-check result. It lists failed/skipped checks, evidence paths and suggested commands, but it is not pass evidence and does not change the release gate result.",
        "",
        "Do not pass `--skip-protocol` to the final `--require-release-artifacts` gate; protocol drift must be checked for release evidence.",
        "",
        "Optional CI static packaging smoke:",
        "",
        "```bash",
        "vesty validate target/vesty/MyPlugin.vst3 --static-only --strict --format json --report target/release-evidence/package/MyPlugin.macos.static-validate.json",
        "vesty release-check --static-validate-report target/release-evidence/package/MyPlugin.macos.static-validate.json",
        "```",
        "",
        "Evidence files:",
        "",
        "- `ci-run-url.txt`: paste the real GitHub Actions run URL, then pass it through `--ci-run-url`.",
        "- `import-ci-report.json`: written by `vesty release-evidence import-ci`; records imported, skipped and failed artifacts from a real downloaded CI artifact directory. This report is audit metadata, not pass evidence by itself.",
        "- `ci-release-checks/`: optional conventional directory for downloaded per-OS `release-check-*` artifacts from CI. `--require-release-artifacts` requires Linux, macOS and Windows snapshots whose local invariant checks passed; OS coverage may be inferred from the `release-check*.json` file name or from parent directories such as `Linux/release-check.json`, `macOS/release-check.json` and `Windows/release-check.json`. When `ci-run-url.txt` is present, each snapshot must carry the same `ci_run_url` repo/run id. Valid `release-action-plan-<OS>.json` sidecars may also be preserved here by `import-ci` for human follow-up, but `--ci-release-check-dir` only reads `release-check*.json` and never treats action plans as pass evidence. These snapshots may be generated with `--skip-protocol` on matrix runners, but the final strict release gate must run the protocol snapshot check and must not pass `--skip-protocol`. These snapshots do not replace DAW, validator, signing or notarization evidence.",
        "- `platform-smoke/`: place platform smoke JSON reports from macOS, Windows x64 and Linux X11 real host/platform runs. `--require-release-artifacts` requires all three platforms and checks system WebView, VST3 validator, VST3 example scan, WebView attach/resize, manifest asset protocol, JSBridge roundtrip and nonzero meter stream evidence. Linux Wayland remains experimental and does not satisfy the Linux X11 gate.",
        "- `publish-plan/publish-plan.json`: place the downloaded `vesty-publish-plan` artifact from CI, or generate it with `vesty publish-plan --out publish-plan/publish-plan.json`. The report must preserve dependency-safe crate order.",
        "- `crate-package/crate-package.json`: crate package readiness report from CI, or generate it with `vesty crate-package --out crate-package/crate-package.json`. It is required by `release-check --require-release-artifacts` and must show zero-internal-dependency crates as `packaged` and internal-dependent crates as `deferred` until their workspace dependencies are published.",
        "- `npm-pack/npm-pack.json`: place the downloaded npm pack dry-run artifact from CI, or generate it with `vesty npm-pack --out npm-pack/npm-pack.json`. The report must include `vesty-plugin-ui`, and each packed file must stay inside `dist/**` or `package.json`.",
        "- `dependency-baseline/dependency-baseline-latest.json`: place the downloaded latest dependency review artifact from CI, or generate it with `vesty dependency-baseline --latest --out dependency-baseline/dependency-baseline-latest.json`. The report must include the `cargo workspace external dependency baseline coverage` check plus crates.io/npm registry latest checks. `dependency-baseline.json` without registry latest checks is useful drift evidence, but it does not satisfy the final `--require-release-artifacts` gate.",
        "- `vst3-sdk/vst3-sdk-headers.json`: optional official Steinberg VST3 SDK header input manifest generated by `vesty vst3-sdk manifest`. When present, `release-check` verifies manifest version, generator, SDK/crate baselines, required header set, `missing_headers`, SHA-256 shape and completeness. This is audit evidence for the reserved generated-headers backend; absence stays `skipped` while the upstream `vst3` crate backend is active.",
        "- `vst3-sdk/generated-bindings-plan.json`: optional generated-bindings readiness plan generated by `vesty vst3-sdk binding-plan`. When present, `release-check` verifies the locked SDK header manifest, `.rs` output module path, active backend baseline and reserved binding emitter check. This does not claim full SDK 3.8 bindings are generated; `bindingsGenerated` must remain false until the emitter exists.",
        "- `vst3-sdk/generated-bindings-surface.json`: optional generated-bindings symbol surface generated by `vesty vst3-sdk binding-surface`. When present, `release-check` verifies the locked SDK header manifest, required symbol/header surface, active backend baseline and audit notes. This locks the expected generated-headers surface only; `bindingsGenerated` must remain false and no C++ AST parsing, ABI verification or Rust bindings generation is implied.",
        "- `vst3-sdk/generated.rs`: optional metadata-only scaffold generated by `vesty vst3-sdk emit-scaffold` and copied by `import-ci` when valid. It fixes generator/header/baseline metadata for drift checks and must keep `BINDINGS_GENERATED = false`; when present, `release-check` validates its scaffold markers, but it is not proof that full SDK bindings are generated.",
        "- `vst3-sdk/generated-abi-seed.rs`: optional ABI seed generated by `vesty vst3-sdk emit-abi-seed` and copied by `import-ci` when valid. It fixes basic VST3 ABI aliases/constants such as `TResult`, `ParamID`, `ParamValue`, `TChar`, `TUID`, result constants and platform type strings, while keeping `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; when present, `release-check` validates its ABI seed markers, but it is not proof that full SDK bindings are generated.",
        "- `vst3-sdk/generated-abi.rs`: optional foundational ABI layout module generated by `vesty vst3-sdk emit-abi` and copied by `import-ci` when valid. It fixes deterministic `repr(C)` layouts such as `TUID`, `FUnknownVTable`, `FUnknown`, `ViewRect`, `ProgramListInfo`, `UnitInfo`, `NoteExpressionValueDescription`, `NoteExpressionTypeInfo`, `PhysicalUIMap` and `PhysicalUIMapList`, plus basic aliases/constants and layout size/alignment/field-offset fingerprints, while keeping `ABI_LAYOUT_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; when present, `release-check` validates its ABI layout markers, but it is not proof that full SDK bindings or complete ABI verification exist.",
        "- `vst3-sdk/generated-interface-skeleton.rs`: optional interface/vtable skeleton module generated by `vesty vst3-sdk emit-interface-skeleton` and copied by `import-ci` when valid. It fixes deterministic `repr(C)` placeholders plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata for discovered VST3 interfaces such as `IPlugViewVTable`, `IEditControllerVTable`, `IMidiMappingVTable`, `IUnitInfoVTable`, `IProgramListDataVTable` and `INoteExpressionControllerVTable`, while keeping `INTERFACE_SKELETON_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; its method order entries are audit-only per-interface order numbers, its vtable slot seeds only fix future emitter local slot, field, callback type alias names, signature intent, repr(C) callback field layout and field offset fingerprints, its IID/queryInterface entries fix upstream IID words, a future interface dispatch plan and pure lookup helpers such as `interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()`, its `COM_OBJECT_INTERFACES` entries only fix the current Vesty adapter object-to-interface exposure plan, its `COM_OBJECT_IDENTITY_PLANS` / `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` entries only fix per-object FUnknown identity and dispatch behavior, its `FACTORY_EXPORT_PLAN` / `FACTORY_CLASS_PLANS` entries only fix the current `VestyFactory` class-count, category, CID derivation and `createInstance` dispatch/error policy, its `MODULE_EXPORT_PLANS` entries only fix the current `export_vst3!` platform entry symbol plan, and its `BINARY_EXPORT_SYMBOL_PLANS` / `BINARY_EXPORT_INSPECTION_TOOL_PLANS` entries plus `binary_export_symbol_plan_by_platform_and_symbol()` / `binary_export_inspection_tools()` / `required_binary_export_symbol_count()` / `first_missing_binary_export_symbol()` / `binary_export_required_symbols_present()` helpers fix expected per-platform export names/tool spellings, inspection tool order and pure required-symbol checks. When present, `release-check` validates its interface skeleton markers and metadata, but it does not emit callable `queryInterface` glue, factory glue, generated factory exports, generated module exports, binary inspection tooling, Steinberg method implementations or full COM bindings.",
        "- `validator/`: recommended directory for validator-passed release reports and logs. `release-check --plan` writes suggested `vesty validate --strict` commands to this directory, and `release-check --release-evidence-dir` auto-discovers valid validator-passed JSON reports recursively.",
        "- `validate-report.json`: legacy/single-plugin pending template; replace it with `vesty validate --strict --report` output from a validator-passed run when you are collecting one report manually. Framework release matrices should prefer `validator/<bundle>.<platform>.validate.json` plus matching validator logs.",
        "- Vesty framework release gates expect Steinberg validator-passed reports for `VestyGain.vst3`, `VestyWebUIDemo.vst3` and `VestyMIDISynth.vst3` on `linux-x64`, `macos` and `windows-x64`. Per-OS validator jobs may provide one platform at a time; `--require-release-artifacts` requires the full 3x3 validator matrix. Each example/platform report must include `static_check.parameter_manifest` pointing at that bundle's `Contents/Resources/parameters.manifest.json`; `VestyWebUIDemo.vst3` must also include `static_check.asset_manifest` pointing at `Contents/Resources/assets.manifest.json` and a nonzero `asset_count`. In the final strict gate, each example/platform report must also include an `ok` `static_check.binary_exports` entry for the matching platform, proving `GetPluginFactory` plus platform entry/exit exports were observed by `vesty validate`; generate release validator reports with `vesty validate --strict` so missing/skipped export-symbol evidence fails at collection time.",
        "- `package/`: recommended directory for CI package/static validate reports. `release-check --plan` writes suggested `vesty validate --static-only --strict` commands to this directory, and `release-check --release-evidence-dir` auto-discovers valid static-only JSON reports recursively.",
        "- `static-validate-report.json`: legacy/single-plugin pending template; replace it with `vesty validate --static-only --strict --report` output from CI packaging smoke when you are collecting one report manually. Framework release matrices should prefer `package/<bundle>.<platform>.static-validate.json`. This does not replace validator-passed release evidence.",
        "- Vesty framework release gates expect CI static validate reports for `VestyGain.vst3`, `VestyWebUIDemo.vst3` and `VestyMIDISynth.vst3` on `linux-x64`, `macos` and `windows-x64`. Per-OS package jobs may provide one platform at a time; `--require-release-artifacts` requires the full 3x3 matrix. Each example/platform report must include `static_check.parameter_manifest` pointing at that bundle's `Contents/Resources/parameters.manifest.json`; `VestyWebUIDemo.vst3` must also include UI asset manifest evidence pointing at `Contents/Resources/assets.manifest.json`. In the final strict gate, each example/platform static report must also include an `ok` `static_check.binary_exports` entry for the matching platform.",
        "- `ci-doctor/`: optional conventional directory for downloaded `doctor-Linux.json`, `doctor-macOS.json` and `doctor-Windows.json` artifacts. OS coverage may also be inferred from parent directories such as `Linux/doctor.json`, `macOS/doctor.json` and `Windows/doctor.json`. Each artifact must include toolchain, Node/npm, VST3 binding baseline, validator, system WebView and signing/notarization preflight checks. The template README alone does not count.",
        "- `signing-macos.log`: paste `codesign --verify --deep --strict --verbose=2` or equivalent positive signing evidence. Additional nested `.log`/`.txt` files and signed macOS `.vst3` bundles are auto-discovered by content.",
        "- `signing-windows.log`: paste `signtool verify /pa /v` or equivalent positive signing evidence. Additional nested `.log`/`.txt` files are auto-discovered by content.",
        "- `--require-release-artifacts` requires both macOS codesign and Windows signtool coverage. Generic `signed=true` / `signature=ok` markers are rejected because they do not prove either platform by themselves.",
        "- `notary.log`: paste accepted `xcrun notarytool submit --wait` and stapler output. Additional nested `.log`/`.txt` files are auto-discovered by content.",
        "- `--require-release-artifacts` requires both accepted notarytool output and stapler success evidence. Generic `notarization=pass` / `notary=ok` markers are rejected because they do not prove notarytool accepted status.",
        "",
        "Accepted release artifact markers:",
        "",
        "Marker lines are parsed as exact `key=value` or `key: value` pairs; pending, false and instructional text do not count.",
        "Positive signing/notarization markers do not override explicit failure evidence in the same log: invalid signatures, nonzero signtool error counts, rejected/invalid notary statuses and stapler failures are rejected.",
        "",
        "```text",
        "codesign=pass",
        "signtool=pass",
        "signtool ... Number of errors: 0",
        "notarytool=pass",
        "stapled=true",
        "status: Accepted",
        "The staple and validate action worked!",
        "```",
        "",
        "DAW smoke evidence is kept separately under `target/daw-evidence/<host>` by convention; generate it with `vesty daw-matrix --write-template --evidence-root target/daw-evidence` and pass the same root to `vesty release-check --evidence-root target/daw-evidence`.",
        "",
    ]
    .join("\n")
}

pub(super) fn vst3_sdk_manifest_evidence_readme() -> String {
    [
        "# VST3 SDK Generated Bindings Evidence",
        "",
        "Place `vst3-sdk-headers.json` here when auditing the official Steinberg VST3 SDK header inputs reserved for generated bindings. Place `generated-bindings-plan.json` and `generated-bindings-surface.json` here when auditing generated-bindings readiness and expected symbol surface without claiming generated bindings are complete. `generated.rs`, `generated-abi-seed.rs`, `generated-abi.rs` and `generated-interface-skeleton.rs` are optional drift/audit modules; `release-check` validates them when present, but they are not proof that complete SDK bindings or final release readiness exist.",
        "",
        "Expected header manifest commands:",
        "",
        "```bash",
        "vesty vst3-sdk manifest \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/vst3-sdk-headers.json",
        "vesty vst3-sdk manifest \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/vst3-sdk-headers.json \\",
        "  --check",
        "```",
        "",
        "Expected generated-bindings plan commands:",
        "",
        "```bash",
        "vesty vst3-sdk binding-plan \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --bindings-module target/vst3-sdk/generated.rs \\",
        "  --out vst3-sdk/generated-bindings-plan.json",
        "vesty vst3-sdk binding-plan \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --bindings-module target/vst3-sdk/generated.rs \\",
        "  --out vst3-sdk/generated-bindings-plan.json \\",
        "  --check",
        "vesty vst3-sdk binding-surface \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/generated-bindings-surface.json",
        "vesty vst3-sdk binding-surface \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/generated-bindings-surface.json \\",
        "  --check",
        "vesty vst3-sdk emit-scaffold \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated.rs",
        "vesty vst3-sdk emit-scaffold \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated.rs \\",
        "  --check",
        "vesty vst3-sdk emit-abi-seed \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi-seed.rs",
        "vesty vst3-sdk emit-abi-seed \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi-seed.rs \\",
        "  --check",
        "vesty vst3-sdk emit-abi \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi.rs",
        "vesty vst3-sdk emit-abi \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi.rs \\",
        "  --check",
        "vesty vst3-sdk emit-interface-skeleton \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-interface-skeleton.rs",
        "vesty vst3-sdk emit-interface-skeleton \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-interface-skeleton.rs \\",
        "  --check",
        "```",
        "",
        "`release-check` treats the manifest, plan, surface and generated Rust audit modules as optional because the active MVP backend still uses the upstream `vst3` crate. If any JSON file or `.rs` audit file exists in this directory, it must be complete and match the expected Vesty generator, baselines, markers and required header/symbol/interface surface. The generated-bindings plan and surface must report `bindingsGenerated: false`; the surface locks expected symbols for a future emitter, but does not parse C++ ASTs, verify ABI, or generate Rust bindings. `emit-scaffold` writes a deterministic metadata-only Rust module at the planned `.rs` path so CI can check output drift, and `import-ci` can copy that module to `vst3-sdk/generated.rs` after validating its scaffold markers. `emit-abi-seed` writes a deterministic ABI seed module with basic aliases/constants and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-abi-seed.rs` after validation. `emit-abi` writes a deterministic foundational ABI layout module with `repr(C)` `TUID`, `FUnknownVTable`, `FUnknown`, `ViewRect`, `ProgramListInfo`, `UnitInfo`, `NoteExpressionValueDescription`, `NoteExpressionTypeInfo`, `PhysicalUIMap`, `PhysicalUIMapList`, aliases/constants, `ABI_LAYOUT_RECORDS` size/alignment fingerprints, `ABI_FIELD_OFFSETS` field-offset fingerprints, `ABI_LAYOUT_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-abi.rs` after validation. `emit-interface-skeleton` writes a deterministic interface/vtable skeleton module with `repr(C)` interface placeholders, method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan audit metadata, pure IID/dispatch lookup helpers and pure binary export required-symbol / inspection-tool helpers, including `QUERY_INTERFACE_IID_LOOKUP_SCOPE`, `interface_id_for_iid()`, `query_interface_entry_by_interface()`, `query_interface_entry_for_iid()`, `com_object_query_interface_dispatch_by_interface()`, `com_object_query_interface_dispatch_for_iid()`, `binary_export_symbol_plan_by_platform_and_symbol()`, `binary_export_inspection_tools()`, `required_binary_export_symbol_count()`, `first_missing_binary_export_symbol()`, `binary_export_required_symbols_present()`, `COM_OBJECT_INTERFACES`, `COM_OBJECT_IDENTITY_PLANS`, `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`, `FACTORY_EXPORT_PLAN`, `FACTORY_CLASS_PLANS`, `MODULE_EXPORT_PLANS`, `BINARY_EXPORT_SYMBOL_PLANS`, `BINARY_EXPORT_INSPECTION_TOOL_PLANS`, `BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED = true`, `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`, `INTERFACE_SKELETON_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-interface-skeleton.rs` after validation. None of these modules generate callable Steinberg VST3 COM/API method implementations, generated factory exports, generated module exports, binary inspection tooling or full COM bindings.",
        "",
        "This README is only a placeholder. It does not count as VST3 SDK header manifest, generated bindings plan, generated bindings surface, generated scaffold, ABI seed, ABI layout, or interface skeleton evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn publish_plan_evidence_readme() -> String {
    [
        "# Publish Plan Evidence",
        "",
        "Place the `publish-plan.json` file from the `vesty-publish-plan` CI artifact here.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty publish-plan --out publish-plan/publish-plan.json",
        "```",
        "",
        "Use `vesty publish-plan --check --out publish-plan/publish-plan.json` to verify an existing report.",
        "",
        "This README is only a placeholder. It does not count as publish-plan evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn crate_package_evidence_readme() -> String {
    [
        "# Crate Package Readiness Evidence",
        "",
        "Place the `crate-package.json` file from the `vesty-crate-package` CI artifact here.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty crate-package --out crate-package/crate-package.json",
        "```",
        "",
        "Use `vesty crate-package --check --out crate-package/crate-package.json` to verify an existing report.",
        "",
        "`vesty crate-package` runs `cargo package -p <crate> --allow-dirty --no-verify` for workspace crates that have no internal dependencies. Crates that still depend on unpublished Vesty workspace crates are recorded as `deferred` until those dependencies are published in dependency order.",
        "",
        "This README is only a placeholder. It does not count as crate package readiness evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn npm_pack_evidence_readme() -> String {
    [
        "# NPM Pack Evidence",
        "",
        "Place the npm pack dry-run JSON artifact here after building the JS packages.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty npm-pack --out npm-pack/npm-pack.json",
        "```",
        "",
        "`vesty npm-pack` runs `npm pack --workspaces --dry-run --json` and then applies the same validation used by `release-check`: the report must include `vesty-plugin-ui`, with packed files limited to `dist/**` and `package.json`.",
        "",
        "This README is only a placeholder. It does not count as npm pack evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn dependency_baseline_evidence_readme() -> String {
    [
        "# Dependency Baseline Evidence",
        "",
        "Place `dependency-baseline-latest.json` here after an explicit online dependency latest review.",
        "",
        "Expected commands:",
        "",
        "```bash",
        "vesty dependency-baseline --latest \\",
        "  --out dependency-baseline/dependency-baseline-latest.json",
        "vesty dependency-baseline --latest \\",
        "  --check \\",
        "  --out dependency-baseline/dependency-baseline-latest.json",
        "```",
        "",
        "`dependency-baseline-latest.json` must include the `cargo workspace external dependency baseline coverage` check plus crates.io and npm registry latest checks. A plain `dependency-baseline.json` generated without `--latest` is useful CI drift evidence, but it does not prove release-time latest dependency review.",
        "",
        "This README is only a placeholder. It does not count as dependency latest baseline evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn ci_doctor_readme() -> String {
    [
        "# CI Doctor Artifacts",
        "",
        "Place downloaded GitHub Actions doctor artifacts here after a real CI run.",
        "",
        "Recommended flat files:",
        "",
        "- `doctor-Linux.json`",
        "- `doctor-macOS.json`",
        "- `doctor-Windows.json`",
        "",
        "Directory-grouped downloads are also accepted because OS coverage is inferred from the full artifact path:",
        "",
        "- `Linux/doctor.json`",
        "- `macOS/doctor.json`",
        "- `Windows/doctor.json`",
        "",
        "Each artifact should be generated with `vesty doctor --format json` and include `vst3 binding baseline`, toolchain, Node/npm, validator, system WebView and signing/notarization preflight checks.",
        "",
        "This README is only a placeholder. It does not count as CI doctor evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn ci_release_checks_readme() -> String {
    [
        "# CI Release-Check Artifacts",
        "",
        "Place downloaded per-OS `release-check-*` GitHub Actions artifacts here after a real CI run.",
        "",
        "Recommended flat files:",
        "",
        "- `release-check-Linux.json`",
        "- `release-check-macOS.json`",
        "- `release-check-Windows.json`",
        "",
        "Directory-grouped downloads are also accepted because OS coverage is inferred from the full artifact path:",
        "",
        "- `Linux/release-check.json`",
        "- `macOS/release-check.json`",
        "- `Windows/release-check.json`",
        "",
        "Optional checklist sidecars preserved by `release-evidence import-ci`:",
        "",
        "- `release-action-plan-Linux.json`",
        "- `release-action-plan-macOS.json`",
        "- `release-action-plan-Windows.json`",
        "",
        "Each artifact should be generated by `vesty release-check --skip-protocol --ci-run-url <run-url> --format json --report ...` on its own runner. The gate checks that local invariants such as host profile coverage and the VST3 binding baseline passed on all three OSes, and that `ci_run_url` matches `ci-run-url.txt` when an expected run URL is provided.",
        "",
        "Action plan sidecars are content-validated checklist metadata for human follow-up. `--ci-release-check-dir` only reads `release-check*.json` case-insensitively; it ignores `release-action-plan-*.json` and never treats them as pass evidence.",
        "",
        "`--skip-protocol` is only for these per-OS CI snapshots; the final `vesty release-check --strict --require-release-artifacts` command must check the protocol snapshot and will fail if `--skip-protocol` is used.",
        "",
        "These snapshots do not replace DAW smoke, validator, signing or notarization evidence; those are checked by separate release gates.",
        "",
        "This README is only a placeholder. It does not count as CI release-check evidence.",
        "",
    ]
    .join("\n")
}

pub(super) fn platform_smoke_evidence_readme() -> String {
    [
        "# Platform Smoke Evidence",
        "",
        "Place real platform smoke JSON reports here after running Vesty on each release platform.",
        "",
        "Expected coverage:",
        "",
        "- `macos`",
        "- `windows-x64`",
        "- `linux-x11`",
        "",
        "Each report must prove system WebView availability, Steinberg validator execution, VST3 example scan, WebView attach/resize, manifest-backed asset protocol, JSBridge roundtrip and a nonzero meter stream.",
        "",
        "System WebView evidence is platform-specific: macOS must mention WebKit.framework or WKWebView, Windows x64 must mention WebView2, and Linux X11 must mention both WebKitGTK and active X11 evidence without Wayland/fallback wording. Steinberg validator evidence must identify Steinberg or VST3 validator output and include passed tests with zero failures.",
        "",
        "Generate starter templates with:",
        "",
        "```bash",
        "vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke",
        "```",
        "",
        "After a real host/platform run, write a normalized report from explicit evidence markers:",
        "",
        "```bash",
        "vesty platform-smoke --write-report --dir target/release-evidence/platform-smoke \\",
        "  --platform macos \\",
        "  --host \"REAPER 7.73\" \\",
        "  --system-webview \"WebKit.framework loaded\" \\",
        "  --vst3-validator \"Steinberg validator passed 47 tests, 0 failed\" \\",
        "  --vst3-example-scan \"VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\" \\",
        "  --webview-attach \"webview_attach=true\" \\",
        "  --webview-resize \"webview_resize=true width=640 height=420\" \\",
        "  --asset-protocol \"asset_protocol=true assets.manifest.json served\" \\",
        "  --jsbridge-roundtrip \"jsbridge_roundtrip=true readyAck reply\" \\",
        "  --meter-stream \"meter_flush sent=3\"",
        "```",
        "",
        "Then run:",
        "",
        "```bash",
        "vesty platform-smoke --dir target/release-evidence/platform-smoke --strict",
        "```",
        "",
        "This README and pending JSON values do not count as platform smoke evidence. Linux Wayland is experimental and does not satisfy the Linux X11 release gate.",
        "",
    ]
    .join("\n")
}

pub(super) fn pending_validate_report_template() -> Result<String, serde_json::Error> {
    let report = ValidateReport {
        bundle: "replace-with-bundle.vst3".to_string(),
        static_check: StaticBundleCheck {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(
                "pending: replace with `vesty validate --strict --report` output".to_string(),
            ),
        },
        validator: ValidatorCheck {
            status: "not_run".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some("pending real Steinberg validator run".to_string()),
            error: None,
        },
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

pub(super) fn pending_static_validate_report_template() -> Result<String, serde_json::Error> {
    let report = ValidateReport {
        bundle: "replace-with-bundle.vst3".to_string(),
        static_check: StaticBundleCheck {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(
                "pending: replace with `vesty validate --static-only --strict --report` output"
                    .to_string(),
            ),
        },
        validator: ValidatorCheck::skipped("--static-only pending template"),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

pub(super) fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

pub(super) fn existing_directory_no_parent_or_leaf_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    reject_existing_output_parent_symlink(label, path)?;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    if !metadata.is_dir() {
        return Err(format!("{label} must be a directory: {path}").into());
    }
    Ok(true)
}

pub(super) fn create_directory_no_parent_or_leaf_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if existing_directory_no_parent_or_leaf_symlink(label, path)? {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    if existing_directory_no_parent_or_leaf_symlink(label, path)? {
        Ok(())
    } else {
        Err(format!("{label} was not created: {path}").into())
    }
}

pub(super) fn create_template_dir(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(
                    format!("template output directory must not be a symlink: {path}").into(),
                );
            }
            if !metadata.is_dir() {
                return Err(
                    format!("template output directory must be a directory: {path}").into(),
                );
            }
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    create_directory_no_parent_or_leaf_symlink("template output directory", path)
}

pub(super) fn write_template_file(
    path: Utf8PathBuf,
    contents: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!("template output file must not be a symlink: {path}").into());
            }
            return Ok(0);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    write_text_file(&path, contents)?;
    Ok(1)
}

pub(super) fn write_platform_smoke_templates(
    dir: &Utf8PathBuf,
) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &platform_smoke_evidence_readme())?;
    for (platform, label) in REQUIRED_PLATFORM_SMOKE_PLATFORMS {
        let report = pending_platform_smoke_report(platform, label)?;
        created += write_template_file(dir.join(format!("{platform}.json")), &report)?;
    }
    Ok(created)
}

pub(super) fn pending_platform_smoke_report(
    platform: &str,
    label: &str,
) -> Result<String, serde_json::Error> {
    let report = PlatformSmokeReport {
        platform: platform.to_string(),
        os: Some(label.to_string()),
        host: Some("pending real host/platform run".to_string()),
        checks: REQUIRED_PLATFORM_SMOKE_CHECKS
            .into_iter()
            .map(|(name, label)| PlatformSmokeCheck {
                name: name.to_string(),
                status: "pending".to_string(),
                value: format!("replace with real {label} evidence"),
                hint: Some("pending template values do not count as pass evidence".to_string()),
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

pub(super) fn print_single_release_check_item(
    item: &ReleaseCheckItem,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(item)?),
        "markdown" | "md" => {
            println!("| Check | Status | Value | Hint |");
            println!("| --- | --- | --- | --- |");
            println!(
                "| {} | {} | {} | {} |",
                item.name,
                item.status,
                markdown_cell(&item.value),
                markdown_cell(item.hint.as_deref().unwrap_or(""))
            );
        }
        "text" | "plain" => {
            println!("{}: {} ({})", item.name, item.status, item.value);
            if let Some(hint) = item.hint.as_deref() {
                println!("hint: {hint}");
            }
        }
        _ => return Err(format!("unsupported platform smoke format '{format}'").into()),
    }
    Ok(())
}

pub(super) fn print_daw_matrix(
    rows: &[serde_json::Value],
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(rows)?),
        "markdown" | "md" => {
            println!(
                "| Host | Platform | Scan | Load | UI | UI->Host | Meter Stream | Automation | Buffer/Sample Rate | Save/Restore | Offline Render | Evidence |"
            );
            println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
            for row in rows {
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    row["host"].as_str().unwrap_or(""),
                    daw_platform_text(row),
                    status_text(&row["scan"]),
                    status_text(&row["load"]),
                    status_text(&row["ui"]),
                    status_text(&row["ui_host_param"]),
                    status_text(&row["meter_stream"]),
                    status_text(&row["automation"]),
                    status_text(&row["buffer_sample_rate_change"]),
                    status_text(&row["save_restore"]),
                    status_text(&row["offline_render"]),
                    row["evidence"].as_str().unwrap_or("")
                );
            }
        }
        _ => return Err(format!("unsupported matrix format '{format}'").into()),
    }
    Ok(())
}

pub(super) fn daw_platform_text(row: &serde_json::Value) -> String {
    let platform = row["platform"].as_str().unwrap_or("").trim();
    match row["platform_supported"].as_bool() {
        Some(true) => platform.to_string(),
        Some(false) if platform.is_empty() => "missing (unsupported)".to_string(),
        Some(false) => format!("{platform} (unsupported)"),
        None if platform.is_empty() => "unknown".to_string(),
        None => format!("{platform} (unknown)"),
    }
}

pub(super) fn daw_matrix_complete(rows: &[serde_json::Value]) -> bool {
    !rows.is_empty() && rows.iter().all(daw_row_complete)
}

pub(super) fn daw_row_complete(row: &serde_json::Value) -> bool {
    if row["platform_supported"].as_bool() != Some(true) {
        return false;
    }
    [
        "scan",
        "load",
        "ui",
        "ui_host_param",
        "meter_stream",
        "automation",
        "buffer_sample_rate_change",
        "save_restore",
        "offline_render",
    ]
    .iter()
    .all(|key| row[*key].as_bool() == Some(true))
}

pub(super) fn daw_missing_checks(row: &serde_json::Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if row["platform_supported"].as_bool() != Some(true) {
        missing.push("platform");
    }
    missing.extend(
        [
            "scan",
            "load",
            "ui",
            "ui_host_param",
            "meter_stream",
            "automation",
            "buffer_sample_rate_change",
            "save_restore",
            "offline_render",
        ]
        .into_iter()
        .filter(|key| row[*key].as_bool() != Some(true)),
    );
    missing
}

pub(super) fn status_text(value: &serde_json::Value) -> &'static str {
    match value.as_bool() {
        Some(true) => "pass",
        Some(false) => "missing",
        None => "unknown",
    }
}

pub(super) fn daw_matrix_rows(evidence: &DawEvidencePaths) -> Vec<serde_json::Value> {
    vec![
        collect_daw_evidence_for_host("reaper", &evidence.reaper),
        collect_daw_evidence_for_host("cubase-nuendo", &evidence.cubase),
        collect_daw_evidence_for_host("bitwig", &evidence.bitwig),
        collect_daw_evidence_for_host("ableton-live", &evidence.ableton),
        collect_daw_evidence_for_host("studio-one", &evidence.studio_one),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DawEvidenceDirStatus {
    Present,
    Missing,
    Blocked,
}

pub(super) fn daw_evidence_dir_status(dir: &Utf8Path) -> DawEvidenceDirStatus {
    match existing_directory_no_parent_or_leaf_symlink("DAW evidence directory", dir) {
        Ok(true) => DawEvidenceDirStatus::Present,
        Ok(false) => DawEvidenceDirStatus::Missing,
        Err(_) => DawEvidenceDirStatus::Blocked,
    }
}

pub(super) fn collect_daw_evidence_for_profile(
    profile: &vesty_core::HostProfile,
    dir: &Utf8PathBuf,
) -> serde_json::Value {
    if profile.id == "reaper" {
        collect_reaper_evidence_for_profile(profile, dir)
    } else {
        collect_generic_daw_evidence_for_profile(profile, dir)
    }
}

pub(super) fn collect_daw_evidence_for_host(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return missing_daw_row(host, dir);
    };
    collect_daw_evidence_for_profile(profile, dir)
}

pub(super) fn print_host_quirks(
    host: Option<&str>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = selected_host_profiles(host)?;
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&profiles)?);
        }
        "markdown" | "md" => {
            println!(
                "| Host | Platforms | Required Smoke | Area | Severity | Summary | Mitigation |"
            );
            println!("| --- | --- | --- | --- | --- | --- | --- |");
            for profile in &profiles {
                let platforms = profile.platforms.join(", ");
                let required = profile.required_smoke_checks.join(", ");
                for quirk in profile.quirks {
                    println!(
                        "| {} | {} | {} | {} | {} | {} | {} |",
                        profile.name,
                        platforms,
                        required,
                        host_quirk_area_text(quirk.area),
                        host_quirk_severity_text(quirk.severity),
                        quirk.summary,
                        quirk.mitigation
                    );
                }
            }
            println!();
            println!(
                "Install detection and host quirk notes do not prove compatibility; release still requires real DAW smoke evidence."
            );
        }
        _ => return Err(format!("unsupported host quirk format '{format}'").into()),
    }
    Ok(())
}

pub(super) fn host_quirk_area_text(area: vesty_core::HostQuirkArea) -> &'static str {
    match area {
        vesty_core::HostQuirkArea::Scanning => "scanning",
        vesty_core::HostQuirkArea::Editor => "editor",
        vesty_core::HostQuirkArea::Automation => "automation",
        vesty_core::HostQuirkArea::State => "state",
        vesty_core::HostQuirkArea::Render => "render",
        vesty_core::HostQuirkArea::Meter => "meter",
        vesty_core::HostQuirkArea::Platform => "platform",
        vesty_core::HostQuirkArea::Packaging => "packaging",
    }
}

pub(super) fn host_quirk_severity_text(severity: vesty_core::HostQuirkSeverity) -> &'static str {
    match severity {
        vesty_core::HostQuirkSeverity::Info => "info",
        vesty_core::HostQuirkSeverity::Warning => "warning",
        vesty_core::HostQuirkSeverity::Required => "required",
    }
}

pub(super) fn selected_host_profiles(
    host: Option<&str>,
) -> Result<Vec<&'static vesty_core::HostProfile>, Box<dyn std::error::Error>> {
    match host {
        Some(host) => vesty_core::find_host_profile(host)
            .map(|profile| vec![profile])
            .ok_or_else(|| format!("unknown host profile '{host}'").into()),
        None => Ok(vesty_core::host_profiles().iter().collect()),
    }
}
