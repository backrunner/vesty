use super::*;

pub(super) const REQUIRED_PLATFORM_SMOKE_PLATFORMS: [(&str, &str); 3] = [
    ("macos", "macOS"),
    ("windows-x64", "Windows x64"),
    ("linux-x11", "Linux X11"),
];

pub(super) const REQUIRED_PLATFORM_SMOKE_CHECKS: [(&str, &str); 8] = [
    ("system_webview", "system WebView"),
    ("vst3_validator", "VST3 validator"),
    ("vst3_example_scan", "VST3 example scan"),
    ("webview_attach", "WebView attach"),
    ("webview_resize", "WebView resize"),
    ("asset_protocol", "asset protocol"),
    ("jsbridge_roundtrip", "JSBridge roundtrip"),
    ("meter_stream", "meter stream"),
];
pub(super) const PLATFORM_SMOKE_MAX_CHECKS: usize = 32;

pub(super) fn expected_platform_smoke_check_names() -> BTreeSet<String> {
    REQUIRED_PLATFORM_SMOKE_CHECKS
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PlatformSmokeReport {
    pub(super) platform: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) host: Option<String>,
    pub(super) checks: Vec<PlatformSmokeCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PlatformSmokeCheck {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) hint: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PlatformSmokeReportInput {
    pub(super) platform: Option<String>,
    pub(super) os: Option<String>,
    pub(super) host: Option<String>,
    pub(super) system_webview: Option<String>,
    pub(super) vst3_validator: Option<String>,
    pub(super) vst3_example_scan: Option<String>,
    pub(super) webview_attach: Option<String>,
    pub(super) webview_resize: Option<String>,
    pub(super) asset_protocol: Option<String>,
    pub(super) jsbridge_roundtrip: Option<String>,
    pub(super) meter_stream: Option<String>,
}

pub(super) fn write_platform_smoke_report(
    dir: &Utf8Path,
    input: PlatformSmokeReportInput,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let platform_raw = input
        .platform
        .as_deref()
        .ok_or("platform smoke report requires `--platform <macos|windows-x64|linux-x11>`")?;
    let platform = normalize_platform_smoke_platform(platform_raw)
        .ok_or_else(|| format!("unsupported platform `{platform_raw}`"))?;
    let platform_label = REQUIRED_PLATFORM_SMOKE_PLATFORMS
        .iter()
        .find_map(|(key, label)| (*key == platform).then_some(*label))
        .unwrap_or(platform);

    let report = PlatformSmokeReport {
        platform: platform.to_string(),
        os: input.os.or_else(|| Some(platform_label.to_string())),
        host: input.host,
        checks: vec![
            platform_smoke_check_from_value("system_webview", input.system_webview)?,
            platform_smoke_check_from_value("vst3_validator", input.vst3_validator)?,
            platform_smoke_check_from_value("vst3_example_scan", input.vst3_example_scan)?,
            platform_smoke_check_from_value("webview_attach", input.webview_attach)?,
            platform_smoke_check_from_value("webview_resize", input.webview_resize)?,
            platform_smoke_check_from_value("asset_protocol", input.asset_protocol)?,
            platform_smoke_check_from_value("jsbridge_roundtrip", input.jsbridge_roundtrip)?,
            platform_smoke_check_from_value("meter_stream", input.meter_stream)?,
        ],
    };
    validate_platform_smoke_report(&report)?;

    create_directory_no_parent_or_leaf_symlink("platform smoke report dir", dir)?;
    let path = dir.join(format!("{platform}.json"));
    write_text_file(&path, &(serde_json::to_string_pretty(&report)? + "\n"))?;
    Ok(path)
}

pub(super) fn platform_smoke_check_from_value(
    name: &str,
    value: Option<String>,
) -> Result<PlatformSmokeCheck, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err(format!(
            "platform smoke report requires `--{}`",
            name.replace('_', "-")
        )
        .into());
    };
    Ok(PlatformSmokeCheck {
        name: name.to_string(),
        status: "ok".to_string(),
        value,
        hint: None,
    })
}

pub(super) fn platform_smoke_release_check(
    dir: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "platform smoke artifacts",
            required,
            "pass `--platform-smoke-dir <dir>` containing macOS, Windows x64 and Linux X11 platform smoke JSON reports",
        );
    };

    let reports = match collect_platform_smoke_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "platform smoke artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some(
                    "expected parseable `vesty platform-smoke --format json` reports".to_string(),
                ),
            };
        }
    };

    let mut platforms = BTreeSet::new();
    let mut failures = Vec::new();
    for (path, report) in &reports {
        if let Err(error) = validate_platform_smoke_report_shape(report) {
            failures.push(format!("{path}: {error}"));
            continue;
        }
        if platform_smoke_report_is_pending_template(report) {
            continue;
        }
        let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
            failures.push(format!(
                "{path}: unsupported platform `{}`",
                report.platform
            ));
            continue;
        };
        match platform_smoke_platform_from_artifact_path(path) {
            Ok(Some(path_platform)) if path_platform != platform => {
                failures.push(format!(
                    "{path}: artifact path indicates {path_platform}, but report platform is {platform}"
                ));
                continue;
            }
            Ok(_) => {}
            Err(error) => {
                failures.push(format!("{path}: {error}"));
                continue;
            }
        }
        if !platforms.insert(platform) {
            failures.push(format!(
                "{path}: duplicate platform smoke report for {platform}"
            ));
        }
        if let Err(error) = validate_platform_smoke_report(report) {
            failures.push(format!("{path}: {error}"));
        }
    }

    if !failures.is_empty() {
        return ReleaseCheckItem {
            name: "platform smoke artifacts".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "collect real platform smoke with system WebView, validator, editor attach/resize, JSBridge and meter evidence"
                    .to_string(),
            ),
        };
    }

    if platforms.is_empty() {
        return optional_release_check_missing(
            "platform smoke artifacts",
            required,
            "no passing platform smoke reports found",
        );
    }

    let missing = REQUIRED_PLATFORM_SMOKE_PLATFORMS
        .iter()
        .filter_map(|(platform, label)| (!platforms.contains(platform)).then_some(*label))
        .collect::<Vec<_>>();
    if required && !missing.is_empty() {
        return ReleaseCheckItem {
            name: "platform smoke artifacts".to_string(),
            status: "failed".to_string(),
            value: format!("missing platform smoke reports: {}", missing.join(", ")),
            hint: Some(
                "run platform smoke on macOS, Windows x64 and Linux X11; Wayland remains experimental"
                    .to_string(),
            ),
        };
    }

    let labels = platforms
        .iter()
        .filter_map(|platform| {
            REQUIRED_PLATFORM_SMOKE_PLATFORMS
                .iter()
                .find_map(|(key, label)| (*key == *platform).then_some(*label))
        })
        .collect::<Vec<_>>();
    ReleaseCheckItem {
        name: "platform smoke artifacts".to_string(),
        status: "ok".to_string(),
        value: format!(
            "{} platform smoke report(s): {}",
            platforms.len(),
            labels.join(", ")
        ),
        hint: (!missing.is_empty()).then(|| {
            format!(
                "full release coverage still needs platform smoke for {}",
                missing.join(", ")
            )
        }),
    }
}

pub(super) fn collect_platform_smoke_reports(
    root: &Utf8Path,
) -> Result<Vec<(Utf8PathBuf, PlatformSmokeReport)>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("platform smoke path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
    };
    if files.is_empty() {
        return Err(format!("no JSON platform smoke artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<PlatformSmokeReport>(&text)
                .map_err(|error| format!("invalid platform smoke JSON: {error}"))?;
            Ok((path, report))
        })
        .collect()
}

pub(super) fn validate_platform_smoke_report(
    report: &PlatformSmokeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_platform_smoke_report_shape(report)?;
    let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
        return Err(format!("unsupported platform `{}`", report.platform).into());
    };
    validate_platform_smoke_os_matches_platform(report, platform)?;

    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for (key, label) in REQUIRED_PLATFORM_SMOKE_CHECKS {
        let Some(check) = report
            .checks
            .iter()
            .find(|check| normalize_platform_smoke_check_name(&check.name) == key)
        else {
            missing.push(label);
            continue;
        };
        if let Err(error) = validate_platform_smoke_check(platform, key, check) {
            failed.push(format!("{label}: {error}"));
        }
    }

    if !missing.is_empty() || !failed.is_empty() {
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("missing checks: {}", missing.join(", ")));
        }
        if !failed.is_empty() {
            parts.push(format!("failed checks: {}", failed.join("; ")));
        }
        return Err(parts.join("; ").into());
    }
    Ok(())
}

pub(super) fn validate_platform_smoke_os_matches_platform(
    report: &PlatformSmokeReport,
    platform: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = report.os.as_deref() else {
        return Ok(());
    };
    let tokens = artifact_path_tokens(Utf8Path::new(os));
    let matches_platform = match platform {
        "macos" => tokens
            .iter()
            .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx")),
        "windows-x64" => tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "windows" | "windowsx64" | "win" | "win32" | "win64"
            )
        }),
        "linux-x11" => {
            tokens.iter().any(|token| token == "linux")
                && tokens.iter().any(|token| token == "x11")
                && !tokens.iter().any(|token| token == "wayland")
        }
        _ => false,
    };
    if matches_platform {
        Ok(())
    } else {
        Err(format!("platform smoke os `{os}` does not match platform `{platform}`").into())
    }
}

pub(super) fn validate_platform_smoke_report_shape(
    report: &PlatformSmokeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("platform smoke platform", &report.platform)?;
    if let Some(os) = &report.os {
        validate_release_action_text("platform smoke os", os)?;
    }
    if let Some(host) = &report.host {
        validate_release_action_text("platform smoke host", host)?;
    }
    if report.checks.is_empty() {
        return Err("platform smoke report has no checks".into());
    }
    if report.checks.len() > PLATFORM_SMOKE_MAX_CHECKS {
        return Err(format!(
            "platform smoke report has too many checks: {} exceeds maximum {PLATFORM_SMOKE_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("platform smoke check name", &check.name)?;
        validate_release_action_text(
            &format!("platform smoke `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(
            &format!("platform smoke `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("platform smoke `{}` hint", check.name), hint)?;
        }

        let normalized = normalize_platform_smoke_check_name(&check.name);
        if normalized.is_empty() {
            return Err(format!(
                "platform smoke check `{}` does not normalize to a stable check name",
                check.name
            )
            .into());
        }
        if !seen_checks.insert(normalized.clone()) {
            return Err(format!("duplicate platform smoke check `{normalized}`").into());
        }
    }
    let expected_checks = expected_platform_smoke_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown platform smoke check(s): {}",
            unknown_checks.join(", ")
        )
        .into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "platform smoke report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

pub(super) fn platform_smoke_report_is_pending_template(report: &PlatformSmokeReport) -> bool {
    !report.checks.is_empty()
        && report.checks.iter().all(|check| {
            check.status.trim().eq_ignore_ascii_case("pending")
                || check
                    .value
                    .to_ascii_lowercase()
                    .contains("replace with real")
        })
}

pub(super) fn validate_platform_smoke_check(
    platform: &str,
    key: &str,
    check: &PlatformSmokeCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    if check.status.trim().eq_ignore_ascii_case("pending")
        || check.status.trim().eq_ignore_ascii_case("skipped")
        || !check.status.trim().eq_ignore_ascii_case("ok")
    {
        return Err(format!("status is {}", check.status).into());
    }
    let value = check.value.trim();
    let lower_value = value.to_ascii_lowercase();
    if value.is_empty()
        || value.eq_ignore_ascii_case("pending")
        || value.eq_ignore_ascii_case("false")
        || lower_value.starts_with("pending ")
        || lower_value.contains("replace with real")
        || platform_smoke_value_has_contradiction(key, value)
    {
        return Err("missing positive evidence value".into());
    }

    let ok = match key {
        "system_webview" => platform_system_webview_evidence_ok(platform, value),
        "vst3_validator" => platform_vst3_validator_evidence_ok(value),
        "vst3_example_scan" => generic_scan_ok(value),
        "webview_attach" => explicit_truthy_marker(value, &["webview_attach", "attach"]),
        "webview_resize" => explicit_truthy_marker(value, &["webview_resize", "resize"]),
        "asset_protocol" => explicit_truthy_marker(
            value,
            &["asset_protocol", "asset_manifest", "custom_protocol"],
        ),
        "jsbridge_roundtrip" => {
            explicit_truthy_marker(
                value,
                &["jsbridge_roundtrip", "bridge_roundtrip", "roundtrip"],
            ) || bridge_trace_relayed_param_gesture(value)
                || (value.contains("readyAck") && value.contains("reply"))
        }
        "meter_stream" => meter_stream_delivered(value),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("value does not match accepted smoke evidence markers".into())
    }
}

pub(super) fn platform_smoke_value_has_contradiction(key: &str, value: &str) -> bool {
    if daw_marker_has_missing_assignment(value) {
        return true;
    }
    if key == "vst3_validator" {
        return false;
    }
    daw_marker_has_negative_evidence(value)
}

pub(super) fn platform_system_webview_evidence_ok(platform: &str, value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.contains("not found")
        || lower.contains("missing")
        || lower.contains("unavailable")
        || lower.contains("not installed")
        || lower.contains("failed")
        || lower.contains("error")
        || explicit_truthy_marker(value, &["system_webview", "webview"])
        || platform_system_webview_has_negative_platform_evidence(platform, &lower)
    {
        return false;
    }

    match platform {
        "macos" => lower.contains("webkit.framework") || lower.contains("wkwebview"),
        "windows-x64" => lower.contains("webview2"),
        "linux-x11" => lower.contains("webkitgtk") && lower.contains("x11"),
        _ => false,
    }
}

pub(super) fn platform_system_webview_has_negative_platform_evidence(
    platform: &str,
    lower: &str,
) -> bool {
    match platform {
        "macos" => [
            "not webkit.framework",
            "no webkit.framework",
            "without webkit.framework",
            "not wkwebview",
            "no wkwebview",
            "without wkwebview",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        "windows-x64" => [
            "not webview2",
            "no webview2",
            "without webview2",
            "webview2 disabled",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        "linux-x11" => [
            "wayland",
            "not x11",
            "no x11",
            "without x11",
            "x11 disabled",
            "x11 unavailable",
            "not webkitgtk",
            "no webkitgtk",
            "without webkitgtk",
            "webkitgtk disabled",
            "experimental",
            "fallback",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        _ => true,
    }
}

pub(super) fn platform_vst3_validator_evidence_ok(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if vst3_validator_has_runtime_failure(&lower)
        || explicit_truthy_marker(value, &["validator", "vst3_validator"])
    {
        return false;
    }

    let has_validator_marker = lower.contains("steinberg") || lower.contains("vst3 validator");
    let Some((passed, failed)) = smoke_validator_test_summary(value) else {
        return false;
    };
    has_validator_marker && passed > 0 && failed == 0
}

pub(super) fn vst3_validator_has_runtime_failure(lower: &str) -> bool {
    [
        "not found",
        "missing",
        "unavailable",
        "not installed",
        "failed to run",
        "validator error",
        "vst3 validator error",
        "validator timeout",
        "validator timed out",
        "vst3 validator timeout",
        "vst3 validator timed out",
        "validator crashed",
        "vst3 validator crashed",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(super) fn smoke_validator_test_summary(text: &str) -> Option<(u32, u32)> {
    let mut passed = None;
    let mut failed = None;
    for line in text.lines() {
        let line = line.to_ascii_lowercase();
        if passed.is_none() {
            passed = extract_count_after_any_marker(&line, &["passed=", "passed:"]).or_else(|| {
                extract_count_near_any_marker(
                    &line,
                    &[
                        "tests passed",
                        "test passed",
                        "passed tests",
                        "passed test",
                        "passed",
                    ],
                )
            });
        }
        if failed.is_none() {
            failed = extract_count_after_any_marker(&line, &["failed=", "failed:"]).or_else(|| {
                extract_count_near_any_marker(
                    &line,
                    &[
                        "tests failed",
                        "test failed",
                        "failed tests",
                        "failed test",
                        "failed",
                    ],
                )
            });
        }
        if let (Some(passed), Some(failed)) = (passed, failed) {
            return Some((passed, failed));
        }
    }
    None
}

pub(super) fn extract_count_after_any_marker(line: &str, markers: &[&str]) -> Option<u32> {
    markers
        .iter()
        .find_map(|marker| extract_count_after_marker(line, marker))
}

pub(super) fn extract_count_after_marker(line: &str, marker: &str) -> Option<u32> {
    let index = line.find(marker)?;
    first_number_after(&line[index + marker.len()..])
}

pub(super) fn normalize_platform_smoke_platform(value: &str) -> Option<&'static str> {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "macos" | "darwin" | "osx" => Some("macos"),
        "windowsx64" | "windows" | "win64" => Some("windows-x64"),
        value if value.contains("linux") && value.contains("x11") => Some("linux-x11"),
        _ => None,
    }
}

pub(super) fn platform_smoke_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    let tokens = artifact_path_tokens(path);
    let mut platforms = Vec::new();
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        platforms.push("macos");
    }
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "windows" | "windowsx64" | "win" | "win32" | "win64"
        )
    }) {
        platforms.push("windows-x64");
    }
    if tokens.iter().any(|token| token == "linux") && tokens.iter().any(|token| token == "x11") {
        platforms.push("linux-x11");
    }
    match platforms.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "artifact path contains multiple platform tokens: {}",
            platforms.join(", ")
        )),
    }
}

pub(super) fn normalize_platform_smoke_check_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
