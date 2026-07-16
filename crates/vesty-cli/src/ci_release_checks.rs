use super::*;

#[derive(Clone, Debug)]
pub(super) struct DoctorArtifact {
    pub(super) path: Utf8PathBuf,
    pub(super) os: Option<&'static str>,
    pub(super) report: DoctorReport,
}

#[derive(Clone, Debug)]
pub(super) struct CiReleaseCheckArtifact {
    pub(super) path: Utf8PathBuf,
    pub(super) os: Option<&'static str>,
    pub(super) report: ReleaseCheckReport,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DoctorArtifactCoverage {
    pub(super) linux: bool,
    pub(super) macos: bool,
    pub(super) windows: bool,
    pub(super) os_mismatches: Vec<String>,
    pub(super) missing_checks: Vec<String>,
    pub(super) run_mismatches: Vec<String>,
}

impl DoctorArtifactCoverage {
    pub(super) fn from_reports(
        reports: &[DoctorArtifact],
        expected_ci_run_url: Option<&str>,
    ) -> Self {
        let mut coverage = DoctorArtifactCoverage::default();
        let expected_run = expected_ci_run_url.and_then(github_actions_run_key);
        for artifact in reports {
            let Some(artifact_os) = artifact.os else {
                continue;
            };
            let Some(os) = doctor_report_os_matches_artifact(artifact_os, artifact, &mut coverage)
            else {
                continue;
            };
            match os {
                "Linux" => coverage.linux = true,
                "macOS" => coverage.macos = true,
                "Windows" => coverage.windows = true,
                _ => {}
            }
            coverage
                .missing_checks
                .extend(missing_doctor_checks(os, &artifact.report));
            if let (Some(expected), Some(actual_url)) =
                (expected_run.as_ref(), artifact.report.ci_run_url.as_deref())
            {
                match github_actions_run_key(actual_url) {
                    Some(actual) if actual == *expected => {}
                    Some(actual) => coverage.run_mismatches.push(format!(
                        "{os} expected {}/{} run {}, got {}/{} run {}",
                        expected.owner,
                        expected.repo,
                        expected.run_id,
                        actual.owner,
                        actual.repo,
                        actual.run_id
                    )),
                    None => coverage
                        .run_mismatches
                        .push(format!("{os} has invalid ci_run_url `{actual_url}`")),
                }
            }
        }
        coverage.os_mismatches.sort();
        coverage.os_mismatches.dedup();
        coverage.missing_checks.sort();
        coverage.missing_checks.dedup();
        coverage.run_mismatches.sort();
        coverage.run_mismatches.dedup();
        coverage
    }

    pub(super) fn present_os(&self) -> Vec<&'static str> {
        [
            ("Linux", self.linux),
            ("macOS", self.macos),
            ("Windows", self.windows),
        ]
        .into_iter()
        .filter_map(|(os, present)| present.then_some(os))
        .collect()
    }

    pub(super) fn missing_os(&self) -> Vec<&'static str> {
        [
            ("Linux", self.linux),
            ("macOS", self.macos),
            ("Windows", self.windows),
        ]
        .into_iter()
        .filter_map(|(os, present)| (!present).then_some(os))
        .collect()
    }

    pub(super) fn missing_checks(&self) -> &[String] {
        &self.missing_checks
    }

    pub(super) fn os_mismatches(&self) -> &[String] {
        &self.os_mismatches
    }

    pub(super) fn run_mismatches(&self) -> &[String] {
        &self.run_mismatches
    }
}

pub(super) fn doctor_report_os_matches_artifact<'a>(
    artifact_os: &'a str,
    artifact: &DoctorArtifact,
    coverage: &mut DoctorArtifactCoverage,
) -> Option<&'a str> {
    let Some(report_os) = artifact.report.os.as_deref() else {
        return Some(artifact_os);
    };
    match normalize_doctor_os_label(report_os) {
        Some(normalized) if normalized == artifact_os => Some(artifact_os),
        Some(normalized) => {
            coverage.os_mismatches.push(format!(
                "{} path indicates {artifact_os}, but report os is {normalized}",
                artifact.path
            ));
            None
        }
        None => {
            coverage.os_mismatches.push(format!(
                "{} has unrecognized report os `{report_os}`",
                artifact.path
            ));
            None
        }
    }
}

pub(super) fn collect_doctor_reports(
    root: &Utf8Path,
) -> Result<Vec<DoctorArtifact>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("doctor artifact path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
    };
    if files.is_empty() {
        return Err(format!("no JSON doctor artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<DoctorReport>(&text)?;
            Ok(DoctorArtifact {
                path: path.clone(),
                os: artifact_os_from_path(&path),
                report,
            })
        })
        .collect()
}

pub(super) fn collect_ci_release_check_reports(
    root: &Utf8Path,
) -> Result<Vec<CiReleaseCheckArtifact>, Box<dyn std::error::Error>> {
    let metadata =
        require_existing_file_or_directory_no_symlink("release-check artifact path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
            .into_iter()
            .filter(|path| is_ci_release_check_report_path(path))
            .collect()
    };
    if files.is_empty() {
        return Err(format!("no JSON release-check artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<ReleaseCheckReport>(&text)?;
            Ok(CiReleaseCheckArtifact {
                path: path.clone(),
                os: artifact_os_from_path(&path),
                report,
            })
        })
        .collect()
}

pub(super) fn is_ci_release_check_report_path(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name.starts_with("release-check") && name.ends_with(".json")
    })
}

pub(super) fn validate_ci_release_check_report(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_check_report_shape(report)?;

    for (name, accepted_statuses) in [
        ("host profiles", &["ok"][..]),
        ("protocol snapshot", &["ok", "skipped"][..]),
        ("vst3 binding baseline", &["ok"][..]),
    ] {
        let Some(check) = report.checks.iter().find(|check| check.name == name) else {
            return Err(format!("missing local invariant check `{name}`").into());
        };
        if !accepted_statuses.contains(&check.status.as_str()) {
            return Err(format!(
                "local invariant check `{name}` status {} ({})",
                check.status, check.value
            )
            .into());
        }
        validate_ci_release_check_invariant_value(name, check)?;
    }

    let unexpected_failures = report
        .checks
        .iter()
        .filter(|check| check.status == "failed")
        .filter(|check| !ci_release_check_allows_failed_check(&check.name))
        .map(|check| format!("{}: {}", check.name, check.value))
        .collect::<Vec<_>>();
    if !unexpected_failures.is_empty() {
        return Err(format!(
            "unexpected failed local checks: {}",
            unexpected_failures.join("; ")
        )
        .into());
    }

    Ok(())
}

pub(super) fn validate_ci_release_check_report_os_matches_path(
    artifact: &CiReleaseCheckArtifact,
) -> Result<(), String> {
    let Some(path_os) = artifact.os else {
        return Ok(());
    };
    let Some(report_os) = artifact.report.os.as_deref() else {
        return Ok(());
    };
    match normalize_doctor_os_label(report_os) {
        Some(normalized) if normalized == path_os => Ok(()),
        Some(normalized) => Err(format!(
            "{} path indicates {path_os}, but report os is {normalized}",
            artifact.path
        )),
        None => Err(format!(
            "{} has unrecognized report os `{report_os}`",
            artifact.path
        )),
    }
}

pub(super) fn validate_release_check_report_shape(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    match report.status.as_str() {
        "ok" | "failed" => {}
        other => return Err(format!("unexpected report status `{other}`").into()),
    }
    if let Some(os) = &report.os {
        validate_release_action_text("release-check os", os)?;
        if normalize_doctor_os_label(os).is_none() {
            return Err(format!("invalid release-check os `{os}`").into());
        }
    }
    if let Some(ci_run_url) = &report.ci_run_url {
        validate_release_action_text("release-check ci run url", ci_run_url)?;
        if !is_github_actions_run_url(ci_run_url) {
            return Err(format!("invalid ci_run_url `{ci_run_url}`").into());
        }
    }
    if report.checks.is_empty() {
        return Err("release-check report must contain at least one check".into());
    }
    if report.checks.len() > RELEASE_CHECK_MAX_CHECKS {
        return Err(format!(
            "release-check report has too many checks: {} exceeds maximum {RELEASE_CHECK_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut check_names = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("release-check check name", &check.name)?;
        validate_release_action_text(
            &format!("release-check `{}` status", check.name),
            &check.status,
        )?;
        if !matches!(check.status.as_str(), "ok" | "skipped" | "failed") {
            return Err(format!("unexpected check status: {}={}", check.name, check.status).into());
        }
        validate_release_action_text(
            &format!("release-check `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("release-check `{}` hint", check.name), hint)?;
        }
        if !check_names.insert(check.name.as_str()) {
            return Err(format!("duplicate check name(s): {}", check.name).into());
        }
    }
    let has_failed_checks = report.checks.iter().any(|check| check.status == "failed");
    match (report.status.as_str(), has_failed_checks) {
        ("ok", true) => {
            return Err("report status `ok` is inconsistent with failed checks".into());
        }
        ("failed", false) => {
            return Err("report status `failed` is inconsistent with no failed checks".into());
        }
        _ => {}
    }

    validate_release_check_daw_matrix_shape(&report.daw_matrix)?;
    validate_release_check_check_set(&check_names)?;
    validate_release_check_daw_check_consistency(report)?;
    Ok(())
}

pub(super) fn validate_release_check_check_set(
    check_names: &BTreeSet<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = expected_release_check_names();
    let missing = expected
        .iter()
        .filter(|expected| !check_names.contains(expected.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let extra = check_names
        .iter()
        .filter(|actual| !expected.contains(**actual))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() && extra.is_empty() {
        return Ok(());
    }

    let mut issues = Vec::new();
    if !missing.is_empty() {
        issues.push(format!("missing check(s): {}", missing.join(", ")));
    }
    if !extra.is_empty() {
        issues.push(format!("unknown check(s): {}", extra.join(", ")));
    }
    Err(format!(
        "release-check report check set must match current Vesty release gate: {}",
        issues.join("; ")
    )
    .into())
}

pub(super) fn expected_release_check_names() -> BTreeSet<String> {
    let rows = vesty_core::host_profiles()
        .iter()
        .map(|profile| missing_daw_row(profile.name, &Utf8PathBuf::from("target/daw-evidence")))
        .collect::<Vec<_>>();
    build_release_check_report(
        rows,
        Utf8Path::new("target/vesty-protocol"),
        true,
        &ReleaseEvidenceOptions::default(),
    )
    .checks
    .into_iter()
    .map(|check| check.name)
    .collect()
}

pub(super) fn validate_release_check_daw_check_consistency(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_check_item_matches_daw_matrix(
        report,
        &host_profile_release_check(&report.daw_matrix),
    )?;
    validate_release_check_item_matches_daw_matrix(
        report,
        &daw_matrix_release_check(&report.daw_matrix),
    )?;

    let mut expected_daw_smoke_checks = BTreeSet::new();
    for row in &report.daw_matrix {
        let expected = host_row_release_check(row);
        expected_daw_smoke_checks.insert(expected.name.clone());
        validate_release_check_item_matches_daw_matrix(report, &expected)?;
    }

    for check in &report.checks {
        if check.name.starts_with("daw smoke: ") && !expected_daw_smoke_checks.contains(&check.name)
        {
            return Err(format!(
                "release-check `{}` has no matching canonical daw_matrix row",
                check.name
            )
            .into());
        }
    }

    Ok(())
}

pub(super) fn validate_release_check_item_matches_daw_matrix(
    report: &ReleaseCheckReport,
    expected: &ReleaseCheckItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(actual) = report
        .checks
        .iter()
        .find(|check| check.name == expected.name)
    else {
        return Err(format!(
            "release-check report is missing `{}` check for current daw_matrix",
            expected.name
        )
        .into());
    };
    if actual.status != expected.status || actual.value != expected.value {
        return Err(format!(
            "release-check `{}` is inconsistent with daw_matrix: expected {} ({}) but found {} ({})",
            expected.name, expected.status, expected.value, actual.status, actual.value
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_release_check_daw_matrix_shape(
    rows: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_count = vesty_core::host_profiles().len();
    if rows.len() != expected_count {
        return Err(format!(
            "release-check daw_matrix must contain exactly {expected_count} release host profile rows, found {}",
            rows.len()
        )
        .into());
    }
    let host_diff = daw_matrix_host_set_diff(rows);
    let host_issues = daw_matrix_host_set_issues(&host_diff);
    if !host_issues.is_empty() {
        return Err(format!(
            "release-check daw_matrix host set mismatch: {}",
            host_issues.join("; ")
        )
        .into());
    }
    let mut seen_hosts = BTreeSet::new();
    for row in rows {
        let object = row
            .as_object()
            .ok_or("release-check daw_matrix rows must be objects")?;
        validate_release_check_daw_matrix_row_keys(object)?;
        let Some(host) = object.get("host").and_then(serde_json::Value::as_str) else {
            return Err("release-check daw_matrix row is missing host".into());
        };
        validate_release_action_text("release-check daw_matrix host", host)?;
        let Some(profile) = vesty_core::find_host_profile(host) else {
            return Err(format!("release-check daw_matrix row has unknown host `{host}`").into());
        };
        if host != profile.name {
            return Err(format!(
                "release-check daw_matrix row host `{host}` must use canonical host name `{}`",
                profile.name
            )
            .into());
        }
        if !seen_hosts.insert(host) {
            return Err(format!("duplicate release-check daw_matrix host `{host}`").into());
        }
        for key in ["evidence", "platform"] {
            let value = object
                .get(key)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    format!("release-check daw_matrix `{host}` field `{key}` must be a string")
                })?;
            validate_release_action_text(
                &format!("release-check daw_matrix `{host}` {key}"),
                value,
            )?;
        }
        for key in [
            "platform_supported",
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
            if let Some(value) = object.get(key)
                && !value.is_boolean()
            {
                return Err(format!(
                    "release-check daw_matrix `{host}` field `{key}` must be boolean"
                )
                .into());
            }
        }
        validate_release_check_daw_matrix_platform_consistency(host, object)?;
    }
    Ok(())
}

pub(super) fn validate_release_check_daw_matrix_row_keys(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    const ALLOWED_KEYS: &[&str] = &[
        "host",
        "platform",
        "platform_supported",
        "scan",
        "load",
        "ui",
        "ui_host_param",
        "meter_stream",
        "automation",
        "buffer_sample_rate_change",
        "save_restore",
        "offline_render",
        "evidence",
    ];
    let allowed = ALLOWED_KEYS.iter().copied().collect::<BTreeSet<_>>();
    let unknown = object
        .keys()
        .filter(|key| !allowed.contains(key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "release-check daw_matrix row contains unknown field(s): {}",
            unknown.join(", ")
        )
        .into());
    }
    let missing = ALLOWED_KEYS
        .iter()
        .copied()
        .filter(|key| !object.contains_key(*key))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "release-check daw_matrix row is missing required field(s): {}",
            missing.join(", ")
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_release_check_daw_matrix_platform_consistency(
    host: &str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return Ok(());
    };
    let Some(platform_supported) = object
        .get("platform_supported")
        .and_then(serde_json::Value::as_bool)
    else {
        return Ok(());
    };
    let platform = object
        .get("platform")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let platform_is_supported = daw_platform_evidence_supported(profile, platform);
    if platform_supported != platform_is_supported {
        return Err(format!(
            "release-check daw_matrix `{host}` platform_supported={platform_supported} is inconsistent with platform `{platform}` for {}",
            profile.name
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_ci_release_check_invariant_value(
    name: &str,
    check: &ReleaseCheckItem,
) -> Result<(), Box<dyn std::error::Error>> {
    match name {
        "host profiles" => {
            let expected_count = vesty_core::host_profiles().len() as u32;
            let expected_value = format!(
                "{expected_count} release host profiles covered: {}",
                release_host_profile_names().join(", ")
            );
            if check.value != expected_value {
                return Err(format!(
                    "local invariant check `host profiles` value `{}` does not exactly match current host profile set `{expected_value}`",
                    check.value,
                )
                .into());
            }
        }
        "protocol snapshot" => {
            if check.status == "skipped" && !check.value.contains("--skip-protocol") {
                return Err(format!(
                    "local invariant check `protocol snapshot` skipped with unexpected value `{}`",
                    check.value
                )
                .into());
            }
            if check.status == "ok" && check.value.trim().is_empty() {
                return Err(
                    "local invariant check `protocol snapshot` ok has empty snapshot path".into(),
                );
            }
        }
        "vst3 binding baseline" => {
            let baseline = vesty_vst3_sys::BINDING_BASELINE;
            let backend = binding_backend_text(baseline.backend);
            if !check.value.contains(baseline.steinberg_sdk)
                || !check.value.contains(baseline.upstream_vst3_crate)
                || !check.value.contains(backend)
            {
                return Err(format!(
                    "local invariant check `vst3 binding baseline` value `{}` does not match current baseline {} / {} / {}",
                    check.value, baseline.steinberg_sdk, baseline.upstream_vst3_crate, backend
                )
                .into());
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn ci_release_check_allows_failed_check(name: &str) -> bool {
    name.starts_with("daw smoke:")
        || matches!(
            name,
            "daw matrix"
                | "daw smoke matrix"
                | "ci run url"
                | "ci doctor artifacts"
                | "ci release-check artifacts"
                | "platform smoke artifacts"
                | "vst3 validate reports"
                | "vst3 example validator coverage"
                | "vst3 static validate reports"
                | "ci example static validate coverage"
                | "crate publish plan"
                | "crate package readiness"
                | "npm package pack report"
                | "dependency latest baseline"
                | "vst3 SDK header manifest"
                | "vst3 SDK generated bindings plan"
                | "vst3 SDK generated bindings surface"
                | "vst3 SDK generated bindings scaffold"
                | "vst3 SDK generated bindings ABI seed"
                | "vst3 SDK generated bindings ABI layout"
                | "vst3 SDK generated bindings interface skeleton"
                | "signed bundle evidence"
                | "notarization log"
        )
}

pub(super) fn collect_json_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_json_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

pub(super) fn collect_evidence_text_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_evidence_text_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

pub(super) fn collect_rust_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_rust_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

pub(super) fn collect_vst3_bundle_dirs_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut bundles = Vec::new();
    collect_vst3_bundle_dirs_recursive_inner(root, &mut bundles)?;
    bundles.sort();
    Ok(bundles)
}

pub(super) fn collect_json_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "JSON artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("JSON artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_json_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn collect_evidence_text_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "evidence artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("evidence artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_evidence_text_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("log") || extension.eq_ignore_ascii_case("txt")
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn collect_rust_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "Rust artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("Rust artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_rust_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn collect_vst3_bundle_dirs_recursive_inner(
    current: &Utf8Path,
    bundles: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "evidence artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("evidence artifact contains symlink: {path}").into());
        }
        if !metadata.is_dir() {
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
        {
            bundles.push(path);
        } else {
            collect_vst3_bundle_dirs_recursive_inner(&path, bundles)?;
        }
    }
    Ok(())
}

pub(super) fn doctor_artifact_os(path: &Utf8Path) -> Option<&'static str> {
    let name = Utf8Path::new(path.file_name()?);
    os_from_artifact_path_tokens(&artifact_path_tokens(name))
}

pub(super) fn normalize_doctor_os_label(value: &str) -> Option<&'static str> {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "linux" => Some("Linux"),
        "macos" | "darwin" | "osx" => Some("macOS"),
        "windows" | "win32" | "win64" => Some("Windows"),
        _ => None,
    }
}

pub(super) fn missing_doctor_checks(os: &str, report: &DoctorReport) -> Vec<String> {
    const OK: &[&str] = &["ok"];
    const OK_OR_UNKNOWN: &[&str] = &["ok", "unknown"];
    const OK_OR_SKIPPED: &[&str] = &["ok", "skipped"];

    let mut required = vec![
        ("rustc", OK),
        ("cargo", OK),
        ("node", OK),
        ("npm", OK),
        ("vst3 binding baseline", OK),
        ("vst3 SDK headers", OK_OR_SKIPPED),
        ("vst3 validator", OK),
        ("system webview", OK),
    ];
    match os {
        "Linux" => required.push(("signing: linux release policy", OK_OR_UNKNOWN)),
        "macOS" => {
            required.push(("signing: codesign", OK));
            required.push(("signing: notarytool", OK));
        }
        "Windows" => required.push(("signing: signtool", OK)),
        _ => {}
    }

    required
        .into_iter()
        .filter_map(|(required, accepted_statuses)| {
            let check = report.checks.iter().find(|check| check.name == required)?;
            (!accepted_statuses.contains(&check.status.as_str()))
                .then(|| format!("{os}/{required} status {}", check.status))
        })
        .chain(
            required_doctor_check_names(os)
                .into_iter()
                .filter(|required| !report.checks.iter().any(|check| check.name == *required))
                .map(|required| format!("{os}/{required} missing")),
        )
        .chain(
            unexpected_doctor_checks_for_os(os, report)
                .into_iter()
                .map(|check| format!("{os}/{check} unexpected for {os} doctor report")),
        )
        .collect()
}

pub(super) fn required_doctor_check_names(os: &str) -> Vec<&'static str> {
    let mut required = vec![
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
        "Linux" => required.push("signing: linux release policy"),
        "macOS" => {
            required.push("signing: codesign");
            required.push("signing: notarytool");
        }
        "Windows" => required.push("signing: signtool"),
        _ => {}
    }
    required
}

pub(super) fn validate_signing_evidence(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    signing_evidence_platforms(path).map(|_| ())
}

pub(super) const SIGNING_NOTARIZATION_LOG_MAX_BYTES: usize = 512 * 1024;
pub(super) const MACOS_CODE_RESOURCES_MAX_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn signing_evidence_platforms(
    path: &Utf8Path,
) -> Result<BTreeSet<SigningEvidencePlatform>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("signing evidence path", path)?;
    if metadata.is_dir() {
        if path.extension() == Some("vst3") {
            validate_macos_code_resources(path)?;
            return Ok(BTreeSet::from([SigningEvidencePlatform::Macos]));
        }
        return Err("directory is not a macOS signed .vst3 bundle".into());
    }

    let text = read_text_file_no_symlink("signing evidence path", path)?;
    signing_evidence_platforms_from_text(&text)
}

pub(super) fn signing_evidence_platforms_from_text(
    text: &str,
) -> Result<BTreeSet<SigningEvidencePlatform>, Box<dyn std::error::Error>> {
    validate_release_evidence_log_text("signing evidence log", text)?;

    let lower = text.to_ascii_lowercase();
    if let Some(reason) = signing_negative_evidence(&lower) {
        return Err(format!("negative signing evidence found: {reason}").into());
    }

    let mut platforms = BTreeSet::new();
    if explicit_truthy_marker(&lower, &["codesign"]) {
        platforms.insert(SigningEvidencePlatform::Macos);
    }
    if explicit_truthy_marker(&lower, &["signtool"]) {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    let macos_codesign =
        lower.contains("valid on disk") && lower.contains("satisfies its designated requirement");
    if macos_codesign {
        platforms.insert(SigningEvidencePlatform::Macos);
    }
    if lower.contains("successfully verified") {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    let windows_signtool_summary =
        lower.contains("signtool") && signtool_error_count(&lower) == Some(0);
    if windows_signtool_summary {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    if platforms.is_empty() {
        Err("no positive signing marker found".into())
    } else {
        Ok(platforms)
    }
}

pub(super) fn signing_negative_evidence(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if explicit_falsy_marker_line(
            line,
            &["signed", "signing", "signature", "codesign", "signtool"],
        ) {
            return Some(line.to_string());
        }
        if extract_count_near_marker(line, "number of errors").is_some_and(|errors| errors > 0) {
            return Some(line.to_string());
        }
        if line_contains_any(
            line,
            &[
                "signtool error:",
                "sign tool error:",
                "codesign failed",
                "failed to verify",
                "verification failed",
                "signature verification failed",
                "code object is not signed",
                "not signed at all",
                "invalid signature",
                "signature is invalid",
                "sealed resource is missing or invalid",
                "no signature found",
            ],
        ) {
            return Some(line.to_string());
        }
    }
    None
}

pub(super) fn signtool_error_count(text: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| extract_count_near_marker(line, "number of errors"))
}

pub(super) fn validate_macos_code_resources(
    bundle: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    require_existing_directory_no_symlink("macOS bundle Contents", &bundle.join("Contents"))?;
    require_existing_directory_no_symlink(
        "macOS CodeSignature directory",
        &bundle.join("Contents/_CodeSignature"),
    )?;
    let code_resources = bundle.join("Contents/_CodeSignature/CodeResources");
    let metadata = match require_existing_file_no_symlink("macOS CodeResources", &code_resources) {
        Ok(metadata) => metadata,
        Err(error) if error.to_string().contains("does not exist") => {
            return Err(
                "macOS signed bundle evidence is missing Contents/_CodeSignature/CodeResources"
                    .into(),
            );
        }
        Err(error) => return Err(error),
    };
    if metadata.len() > MACOS_CODE_RESOURCES_MAX_BYTES {
        return Err(format!(
            "macOS CodeResources is too large: {} bytes exceeds maximum {MACOS_CODE_RESOURCES_MAX_BYTES}",
            metadata.len()
        )
        .into());
    }

    let plist = plist::Value::from_file(&code_resources)
        .map_err(|error| format!("CodeResources is not a parseable plist: {error}"))?;
    let dict = plist
        .as_dictionary()
        .ok_or("CodeResources plist root must be a dictionary")?;
    if !has_code_resources_file_map(dict, "files") && !has_code_resources_file_map(dict, "files2") {
        return Err("CodeResources plist must contain files or files2 dictionary entries".into());
    }
    Ok(())
}

pub(super) fn has_code_resources_file_map(dict: &plist::Dictionary, key: &str) -> bool {
    dict.get(key)
        .and_then(plist::Value::as_dictionary)
        .is_some()
}

pub(super) fn validate_notarization_evidence(
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    notarization_evidence(path).map(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NotarizationEvidence {
    pub(super) accepted: bool,
    pub(super) stapled: bool,
}

impl NotarizationEvidence {
    pub(super) const fn is_positive(self) -> bool {
        self.accepted || self.stapled
    }
}

pub(super) fn notarization_evidence(
    path: &Utf8Path,
) -> Result<NotarizationEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("notarization evidence", path)?;
    notarization_evidence_from_text(&text)
}

pub(super) fn notarization_evidence_from_text(
    text: &str,
) -> Result<NotarizationEvidence, Box<dyn std::error::Error>> {
    validate_release_evidence_log_text("notarization evidence log", text)?;

    let lower = text.to_ascii_lowercase();
    if let Some(reason) = notarization_negative_evidence(&lower) {
        return Err(format!("negative notarization evidence found: {reason}").into());
    }

    let accepted = explicit_truthy_marker(&lower, &["notarytool"])
        || explicit_status_marker(&lower, "accepted")
        || notarytool_json_status_accepted(text);
    let stapled = explicit_truthy_marker(&lower, &["stapled"]) || explicit_stapler_success(&lower);
    let evidence = NotarizationEvidence { accepted, stapled };
    if evidence.is_positive() {
        Ok(evidence)
    } else {
        Err("no accepted notarization marker found".into())
    }
}

pub(super) fn notarization_negative_evidence(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if notarization_metadata_line(line) {
            continue;
        }
        if explicit_falsy_marker_line(line, &["notarization", "notary", "notarytool", "stapled"])
            || explicit_marker_line_matches(line, &["status"], &["rejected", "invalid", "failed"])
        {
            return Some(line.to_string());
        }
        if line_contains_any(
            line,
            &[
                "status: rejected",
                "status=rejected",
                r#""status":"rejected""#,
                r#""status": "rejected""#,
                "status: invalid",
                "status=invalid",
                r#""status":"invalid""#,
                r#""status": "invalid""#,
                "notarytool error",
                "notarization failed",
                "submission failed",
                "stapler failed",
                "staple failed",
                "validate action failed",
            ],
        ) {
            return Some(line.to_string());
        }
    }
    None
}

pub(super) fn explicit_status_marker(text: &str, value: &str) -> bool {
    text.lines()
        .any(|line| explicit_marker_line_matches(line, &["status"], &[value]))
}

pub(super) fn notarytool_json_status_accepted(text: &str) -> bool {
    json_status_accepted(text)
        || bracketed_log_section(text, "notarytool").is_some_and(|section| {
            json_status_accepted(&section)
                || bracketed_log_section(&section, "stdout")
                    .is_some_and(|stdout| json_status_accepted(&stdout))
        })
}

pub(super) fn json_status_accepted(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text.trim())
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_ascii_lowercase)
        })
        .is_some_and(|status| status == "accepted")
}

pub(super) fn bracketed_log_section(text: &str, section: &str) -> Option<String> {
    let header = format!("[{}]", section.to_ascii_lowercase());
    let mut found = false;
    let mut content = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let is_header = trimmed.starts_with('[') && trimmed.ends_with(']');
        if is_header {
            if found {
                break;
            }
            if trimmed.to_ascii_lowercase() == header {
                found = true;
            }
            continue;
        }
        if found && notarization_metadata_line(trimmed) {
            break;
        }
        if found {
            content.push_str(line);
            content.push('\n');
        }
    }
    found.then_some(content)
}

pub(super) fn explicit_stapler_success(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim() == "the staple and validate action worked!")
}

pub(super) fn notarization_metadata_line(line: &str) -> bool {
    split_marker_assignment(line).is_some_and(|(key, _)| {
        matches!(
            key.trim().replace('-', "_").as_str(),
            "notary_log" | "stapler_log"
        )
    })
}

pub(super) fn validate_release_evidence_log_text(
    label: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty").into());
    }
    if value.len() > SIGNING_NOTARIZATION_LOG_MAX_BYTES {
        return Err(
            format!("{label} must be at most {SIGNING_NOTARIZATION_LOG_MAX_BYTES} bytes").into(),
        );
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

pub(super) fn print_release_check_report(
    report: &ReleaseCheckReport,
    format: &str,
    report_path: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    write_release_check_report(report_path, report)?;
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(report)?),
        "markdown" | "md" => {
            println!("# Vesty Release Check\n");
            println!("status: {}", report.status);
            println!();
            println!("| Check | Status | Value | Hint |");
            println!("| --- | --- | --- | --- |");
            for check in &report.checks {
                println!(
                    "| {} | {} | {} | {} |",
                    check.name,
                    check.status,
                    check.value,
                    check.hint.as_deref().unwrap_or("")
                );
            }
            println!();
            print_daw_matrix(&report.daw_matrix, "markdown")?;
        }
        _ => return Err(format!("unsupported release check format '{format}'").into()),
    }
    Ok(())
}

pub(super) fn write_release_check_report(
    path: Option<&Utf8Path>,
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = validate_release_check_report_shape(report) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid release-check report: {error}"),
        )
        .into());
    }
    let Some(path) = path else {
        return Ok(());
    };
    let text = serde_json::to_string_pretty(report)?;
    write_text_file(path, &(text + "\n"))
}
