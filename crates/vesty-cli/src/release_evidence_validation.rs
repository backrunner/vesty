use super::*;

pub(super) fn validate_local_release_evidence_report_shape(
    report: &LocalReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("local release evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("local release evidence dir", &report.evidence_dir)?;
    validate_release_action_text("local release workspace", &report.workspace)?;
    if let Some(protocol_snapshot) = &report.protocol_snapshot {
        validate_release_action_text("local release protocol snapshot", protocol_snapshot)?;
        validate_release_report_root_path("local release protocol snapshot", protocol_snapshot)?;
    }
    validate_release_action_text(
        "local release external evidence note",
        &report.external_evidence_note,
    )?;
    validate_release_evidence_item_count(
        "local release evidence report",
        report.items.len(),
        true,
    )?;
    for item in &report.items {
        validate_local_release_evidence_item_shape(
            "local release evidence item",
            item,
            &["ok", "skipped", "failed"],
        )?;
        validate_local_release_evidence_item_paths(report, item)?;
    }
    validate_local_release_evidence_protocol_consistency(report)?;
    Ok(())
}

pub(super) fn validate_import_ci_release_evidence_report_shape(
    report: &ImportCiReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("import-ci evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("import-ci evidence dir", &report.evidence_dir)?;
    validate_release_action_text("import-ci source", &report.source)?;
    validate_release_report_root_path("import-ci source", &report.source)?;
    validate_release_action_text(
        "import-ci external evidence note",
        &report.external_evidence_note,
    )?;
    validate_release_evidence_item_count("import-ci report", report.items.len(), false)?;
    for item in &report.items {
        validate_import_ci_release_evidence_item_shape(item)?;
        validate_import_ci_release_evidence_item_paths(report, item)?;
    }
    validate_import_ci_release_evidence_success_output_uniqueness(report)?;
    Ok(())
}

pub(super) fn validate_collected_release_evidence_report_shape(
    report: &CollectedReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("collected release evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("collected release evidence dir", &report.evidence_dir)?;
    validate_release_action_text("collected release evidence kind", &report.kind)?;
    validate_release_action_text("collected release evidence output", &report.output)?;
    validate_release_report_path_under(
        "collected release evidence output",
        &report.output,
        "collected release evidence dir",
        &report.evidence_dir,
    )?;
    if !matches!(report.kind.as_str(), "signing" | "notarization") {
        return Err(format!(
            "unsupported collected release evidence kind `{}`",
            report.kind
        )
        .into());
    }
    if let Some(expected) = expected_collected_release_evidence_output_path(report)?
        && !release_report_paths_equal(&report.output, &expected)
    {
        return Err(format!(
            "collected release evidence output `{}` does not match expected `{expected}` for kind `{}`",
            report.output, report.kind
        )
        .into());
    }
    validate_release_evidence_item_count(
        "collected release evidence report",
        report.items.len(),
        true,
    )?;
    for item in &report.items {
        validate_local_release_evidence_item_shape(
            "collected release evidence item",
            item,
            &["ok"],
        )?;
        if let Some(path) = &item.path {
            validate_release_report_path_under(
                &format!("collected release evidence item `{}` path", item.name),
                path,
                "collected release evidence dir",
                &report.evidence_dir,
            )?;
            if !release_report_paths_equal(path, &report.output) {
                return Err(format!(
                    "collected release evidence item `{}` path `{path}` must match report output `{}`",
                    item.name, report.output
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn expected_collected_release_evidence_output_path(
    report: &CollectedReleaseEvidenceReport,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let relative = match report.kind.as_str() {
        "notarization" => Some("notary.log"),
        "signing" => {
            let mut expected = BTreeSet::new();
            for item in &report.items {
                match item.name.as_str() {
                    "macOS signing verification" => {
                        expected.insert("signing-macos.log");
                    }
                    "Windows signing verification" => {
                        expected.insert("signing-windows.log");
                    }
                    _ => {}
                }
            }
            if expected.len() > 1 {
                return Err(
                    "collected signing report must not mix macOS and Windows output items".into(),
                );
            }
            expected.into_iter().next()
        }
        _ => None,
    };
    Ok(relative.map(|relative| format!("{}/{}", report.evidence_dir, relative)))
}

pub(super) fn validate_local_release_evidence_item_paths(
    report: &LocalReleaseEvidenceReport,
    item: &LocalReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "protocol snapshot" {
        if let Some(path) = &item.path {
            validate_release_report_root_path(
                &format!("local release evidence item `{}` path", item.name),
                path,
            )?;
        }
        return Ok(());
    }
    if let Some(path) = &item.path {
        validate_release_report_path_under(
            &format!("local release evidence item `{}` path", item.name),
            path,
            "local release evidence dir",
            &report.evidence_dir,
        )?;
        validate_release_evidence_template_item_path(
            &format!("local release evidence item `{}` path", item.name),
            &item.name,
            path,
            &report.evidence_dir,
        )?;
        if let Some(expected) = expected_local_release_evidence_item_path(report, item)
            && !release_report_paths_equal(path, &expected)
        {
            return Err(format!(
                "local release evidence item `{}` path `{path}` does not match expected `{expected}`",
                item.name
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn expected_local_release_evidence_item_path(
    report: &LocalReleaseEvidenceReport,
    item: &LocalReleaseEvidenceItem,
) -> Option<String> {
    let relative = fixed_release_evidence_item_relative_path(item.name.as_str())?;
    Some(format!("{}/{}", report.evidence_dir, relative))
}

pub(super) fn validate_local_release_evidence_protocol_consistency(
    report: &LocalReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol_items = report
        .items
        .iter()
        .filter(|item| item.name == "protocol snapshot")
        .collect::<Vec<_>>();
    match report.protocol_snapshot.as_deref() {
        Some(protocol_snapshot) => {
            if protocol_items.len() != 1 {
                return Err(format!(
                    "local release protocol snapshot `{protocol_snapshot}` must match exactly one protocol snapshot item"
                )
                .into());
            }
            let item = protocol_items[0];
            if item.status != "ok"
                || !item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, protocol_snapshot))
            {
                return Err(format!(
                    "local release protocol snapshot `{protocol_snapshot}` does not match protocol snapshot item status/path"
                )
                .into());
            }
        }
        None if !protocol_items.is_empty() => {
            return Err(
                "local release protocol snapshot item present but top-level protocol_snapshot is missing"
                    .into(),
            );
        }
        None => {}
    }
    Ok(())
}

pub(super) fn validate_release_evidence_item_count(
    label: &str,
    count: usize,
    require_nonempty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if require_nonempty && count == 0 {
        return Err(format!("{label} has no items").into());
    }
    if count > RELEASE_EVIDENCE_REPORT_MAX_ITEMS {
        return Err(format!(
            "{label} has too many items: {count} exceeds maximum {RELEASE_EVIDENCE_REPORT_MAX_ITEMS}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_local_release_evidence_item_shape(
    label: &str,
    item: &LocalReleaseEvidenceItem,
    allowed_statuses: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text(&format!("{label} name"), &item.name)?;
    validate_release_action_text(&format!("{label} status"), &item.status)?;
    if !allowed_statuses.contains(&item.status.as_str()) {
        return Err(format!(
            "{label} `{}` has unsupported status `{}`",
            item.name, item.status
        )
        .into());
    }
    if let Some(path) = &item.path {
        validate_release_action_text(&format!("{label} `{}` path", item.name), path)?;
    }
    validate_release_action_text(&format!("{label} `{}` value", item.name), &item.value)?;
    validate_release_evidence_template_item_status(label, &item.name, &item.status)?;
    if item.status == "ok" && item.path.is_none() {
        return Err(format!(
            "{label} `{}` status ok must include an evidence path",
            item.name
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_import_ci_release_evidence_item_shape(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("import-ci item name", &item.name)?;
    validate_release_action_text(&format!("import-ci `{}` status", item.name), &item.status)?;
    if !matches!(
        item.status.as_str(),
        "ok" | "imported" | "skipped" | "failed"
    ) {
        return Err(format!(
            "import-ci item `{}` has unsupported status `{}`",
            item.name, item.status
        )
        .into());
    }
    if let Some(source) = &item.source {
        validate_release_action_text(&format!("import-ci `{}` source", item.name), source)?;
    }
    if let Some(path) = &item.path {
        validate_release_action_text(&format!("import-ci `{}` path", item.name), path)?;
    }
    validate_release_action_text(&format!("import-ci `{}` value", item.name), &item.value)?;
    validate_release_evidence_template_item_status("import-ci item", &item.name, &item.status)?;
    validate_import_ci_release_evidence_item_source_semantics(item)?;
    validate_import_ci_release_evidence_item_value(item)?;
    match item.status.as_str() {
        "ok" => {
            if item.path.is_none() {
                return Err(format!(
                    "import-ci item `{}` status ok must include an output path",
                    item.name
                )
                .into());
            }
        }
        "imported" => {
            if item.path.is_none() {
                return Err(format!(
                    "import-ci item `{}` status imported must include an output path",
                    item.name
                )
                .into());
            }
        }
        "failed" => {
            if item.path.is_some() {
                return Err(format!(
                    "import-ci item `{}` status failed must not include an output path",
                    item.name
                )
                .into());
            }
        }
        "skipped"
            if item.path.is_some()
                && !item
                    .value
                    .contains(ImportWriteOutcome::SkippedExisting.value()) =>
        {
            return Err(format!(
                "import-ci item `{}` status skipped may include an output path only when the destination already exists",
                item.name
            )
            .into());
        }
        "skipped" => {}
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_release_evidence_template_item_status(
    label: &str,
    item_name: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if item_name == "release evidence template" && status != "ok" {
        return Err(format!("{label} `release evidence template` status must be ok").into());
    }
    Ok(())
}

pub(super) fn validate_import_ci_release_evidence_item_source_semantics(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "release evidence template" && item.source.is_some() {
        return Err("import-ci item `release evidence template` must not include a source".into());
    }
    Ok(())
}

pub(super) fn validate_import_ci_release_evidence_item_value(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "ci run url"
        && matches!(item.status.as_str(), "ok" | "imported")
        && !is_github_actions_run_url(&item.value)
    {
        return Err(format!(
            "import-ci item `ci run url` status {} must contain a GitHub Actions run URL value",
            item.status
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_import_ci_release_evidence_item_paths(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(source) = &item.source {
        if import_ci_item_source_may_be_external(item) {
            validate_release_report_root_path(
                &format!("import-ci `{}` source", item.name),
                source,
            )?;
        } else {
            validate_release_report_path_under(
                &format!("import-ci `{}` source", item.name),
                source,
                "import-ci source",
                &report.source,
            )?;
        }
    }
    if let Some(path) = &item.path {
        validate_release_report_path_under(
            &format!("import-ci `{}` path", item.name),
            path,
            "import-ci evidence dir",
            &report.evidence_dir,
        )?;
        validate_release_evidence_template_item_path(
            &format!("import-ci `{}` path", item.name),
            &item.name,
            path,
            &report.evidence_dir,
        )?;
        if let Some(expected) = expected_import_ci_release_evidence_item_path(report, item)
            && !release_report_paths_equal(path, &expected)
        {
            return Err(format!(
                "import-ci item `{}` path `{path}` does not match expected `{expected}`",
                item.name
            )
            .into());
        }
        validate_import_ci_release_evidence_item_dynamic_path(report, item, path)?;
    }
    Ok(())
}

pub(super) fn expected_import_ci_release_evidence_item_path(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
) -> Option<String> {
    let relative = fixed_release_evidence_item_relative_path(item.name.as_str())?;
    Some(format!("{}/{}", report.evidence_dir, relative))
}

pub(super) fn fixed_release_evidence_item_relative_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "ci run url" => "ci-run-url.txt",
        "protocol snapshot" => "vesty-protocol",
        "crate publish plan" => "publish-plan/publish-plan.json",
        "crate package readiness" => "crate-package/crate-package.json",
        "npm package pack report" => "npm-pack/npm-pack.json",
        "dependency latest baseline" => "dependency-baseline/dependency-baseline-latest.json",
        "vst3 SDK header manifest" => "vst3-sdk/vst3-sdk-headers.json",
        "vst3 SDK generated bindings plan" => "vst3-sdk/generated-bindings-plan.json",
        "vst3 SDK generated bindings surface" => "vst3-sdk/generated-bindings-surface.json",
        "vst3 SDK generated bindings scaffold" => "vst3-sdk/generated.rs",
        "vst3 SDK generated bindings ABI seed" => "vst3-sdk/generated-abi-seed.rs",
        "vst3 SDK generated bindings ABI layout" => "vst3-sdk/generated-abi.rs",
        "vst3 SDK generated bindings interface skeleton" => {
            "vst3-sdk/generated-interface-skeleton.rs"
        }
        "notarization log" => "notary.log",
        _ => return None,
    })
}

pub(super) fn validate_release_evidence_template_item_path(
    label: &str,
    item_name: &str,
    path: &str,
    evidence_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if item_name == "release evidence template" && !release_report_paths_equal(path, evidence_dir) {
        return Err(
            format!("{label} `{path}` must match release evidence dir `{evidence_dir}`").into(),
        );
    }
    Ok(())
}

pub(super) fn import_ci_item_source_may_be_external(item: &ImportCiReleaseEvidenceItem) -> bool {
    item.name == "ci run url"
}

pub(super) fn validate_import_ci_release_evidence_item_dynamic_path(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(expectation) = import_ci_dynamic_output_expectation(item.name.as_str()) else {
        return Ok(());
    };

    let evidence_root =
        lexical_release_report_root_path(&report.evidence_dir).map_err(|error| {
            format!(
                "import-ci evidence dir `{}` is not a safe report path: {error}",
                report.evidence_dir
            )
        })?;
    let item_path = lexical_release_report_path(path).map_err(|error| {
        format!(
            "import-ci `{}` path `{path}` is not a safe report path: {error}",
            item.name
        )
    })?;
    let relative = item_path.strip_prefix(&evidence_root).ok_or_else(|| {
        format!(
            "import-ci `{}` path `{path}` must be under import-ci evidence dir `{}`",
            item.name, report.evidence_dir
        )
    })?;
    if import_ci_relative_output_path_is_allowed(item.name.as_str(), relative) {
        Ok(())
    } else {
        Err(format!(
            "import-ci `{}` path `{path}` must be {expectation} under `{}`",
            item.name, report.evidence_dir
        )
        .into())
    }
}

pub(super) fn import_ci_dynamic_output_expectation(item_name: &str) -> Option<&'static str> {
    match item_name {
        "ci doctor artifact" => Some("`ci-doctor/doctor-<OS>.json`"),
        "ci release-check artifact" => Some("`ci-release-checks/release-check-<OS>.json`"),
        "release action plan sidecar" => Some("`ci-release-checks/release-action-plan-<OS>.json`"),
        "platform smoke artifact" => Some("`platform-smoke/<platform>.json`"),
        "vst3 validate report" => Some("`validator/<safe-bundle>.<platform>.validate.json`"),
        "vst3 static validate report" => {
            Some("`package/<safe-bundle>.<platform>.static-validate.json`")
        }
        "signed bundle evidence" => Some(
            "`signing-macos.log`, `signing-windows.log`, `signing/<safe-name>.log`, or `signed-bundles/<safe-bundle>.vst3`",
        ),
        _ => None,
    }
}

pub(super) fn import_ci_relative_output_path_is_allowed(
    item_name: &str,
    relative: &[String],
) -> bool {
    match item_name {
        "ci doctor artifact" => ci_doctor_relative_output_path_is_allowed(relative),
        "ci release-check artifact" => ci_release_check_relative_output_path_is_allowed(relative),
        "release action plan sidecar" => {
            release_action_plan_sidecar_relative_output_path_is_allowed(relative)
        }
        "platform smoke artifact" => platform_smoke_relative_output_path_is_allowed(relative),
        "vst3 validate report" => {
            validate_report_relative_output_path_is_allowed(relative, "validator", "validate.json")
        }
        "vst3 static validate report" => validate_report_relative_output_path_is_allowed(
            relative,
            "package",
            "static-validate.json",
        ),
        "signed bundle evidence" => {
            signed_bundle_evidence_relative_output_path_is_allowed(relative)
        }
        _ => false,
    }
}

pub(super) fn ci_doctor_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "ci-doctor" && matches!(
        file.as_str(),
        "doctor-Linux.json" | "doctor-macOS.json" | "doctor-Windows.json"
    ))
}

pub(super) fn ci_release_check_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "ci-release-checks" && matches!(
        file.as_str(),
        "release-check-Linux.json" | "release-check-macOS.json" | "release-check-Windows.json"
    ))
}

pub(super) fn release_action_plan_sidecar_relative_output_path_is_allowed(
    relative: &[String],
) -> bool {
    matches!(relative, [dir, file] if dir == "ci-release-checks" && matches!(
        file.as_str(),
        "release-action-plan-Linux.json"
            | "release-action-plan-macOS.json"
            | "release-action-plan-Windows.json"
    ))
}

pub(super) fn platform_smoke_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "platform-smoke" && matches!(
        file.as_str(),
        "macos.json" | "windows-x64.json" | "linux-x11.json"
    ))
}

pub(super) fn validate_report_relative_output_path_is_allowed(
    relative: &[String],
    directory: &str,
    suffix: &str,
) -> bool {
    match relative {
        [dir, file] if dir == directory => {
            file.ends_with(suffix) && safe_evidence_filename_part(file) == *file
        }
        _ => false,
    }
}

pub(super) fn signed_bundle_evidence_relative_output_path_is_allowed(relative: &[String]) -> bool {
    match relative {
        [file] if matches!(file.as_str(), "signing-macos.log" | "signing-windows.log") => true,
        [dir, file] if dir == "signing" => {
            file.ends_with(".log") && safe_evidence_filename_part(file) == *file
        }
        [dir, bundle] if dir == "signed-bundles" => {
            bundle.ends_with(".vst3") && safe_evidence_filename_part(bundle) == *bundle
        }
        _ => false,
    }
}

pub(super) fn validate_import_ci_release_evidence_success_output_uniqueness(
    report: &ImportCiReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen_paths = BTreeMap::<String, &str>::new();
    for item in &report.items {
        if !matches!(item.status.as_str(), "ok" | "imported") {
            continue;
        }
        let Some(path) = item.path.as_deref() else {
            continue;
        };
        let key = lexical_release_report_path(path)
            .map(|path| format!("{}|{}", path.prefix, path.components.join("/")))
            .map_err(|error| {
                format!(
                    "import-ci item `{}` output path `{path}` is not a safe report path: {error}",
                    item.name
                )
            })?;
        if let Some(previous_name) = seen_paths.insert(key, item.name.as_str()) {
            return Err(format!(
                "import-ci item `{}` output path `{path}` duplicates successful item `{previous_name}`",
                item.name
            )
            .into());
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LexicalReleaseReportPath {
    pub(super) prefix: String,
    pub(super) components: Vec<String>,
}

impl LexicalReleaseReportPath {
    pub(super) fn starts_with(&self, root: &Self) -> bool {
        self.prefix == root.prefix && self.components.starts_with(&root.components)
    }

    pub(super) fn strip_prefix(&self, root: &Self) -> Option<&[String]> {
        self.starts_with(root)
            .then(|| &self.components[root.components.len()..])
    }
}

pub(super) fn validate_release_report_path_under(
    child_label: &str,
    child: &str,
    root_label: &str,
    root: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let child_path = lexical_release_report_path(child)
        .map_err(|error| format!("{child_label} `{child}` is not a safe report path: {error}"))?;
    let root_path = lexical_release_report_root_path(root)
        .map_err(|error| format!("{root_label} `{root}` is not a safe report path: {error}"))?;
    if !child_path.starts_with(&root_path) {
        return Err(format!("{child_label} `{child}` must be under {root_label} `{root}`").into());
    }
    Ok(())
}

pub(super) fn validate_release_report_root_path(
    label: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    lexical_release_report_root_path(path)
        .map(|_| ())
        .map_err(|error| format!("{label} `{path}` is not a safe report path: {error}").into())
}

pub(super) fn lexical_release_report_root_path(
    path: &str,
) -> Result<LexicalReleaseReportPath, String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized
        .split('/')
        .any(|component| component.trim() == "..")
    {
        return Err("root path must not contain parent-directory components".to_string());
    }
    lexical_release_report_path(path)
}

pub(super) fn lexical_release_report_path(path: &str) -> Result<LexicalReleaseReportPath, String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("path is empty".to_string());
    }

    let mut prefix = String::new();
    let mut rest = normalized.as_str();
    if rest.starts_with('/') {
        prefix.push('/');
        rest = rest.trim_start_matches('/');
    } else if let Some(drive) = windows_drive_prefix(rest) {
        prefix.push_str(&drive.to_ascii_lowercase());
        rest = rest[drive.len()..].trim_start_matches('/');
    }

    let absolute = !prefix.is_empty();
    let mut components = Vec::new();
    for component in rest.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            if components.last().is_some_and(|last| last != "..") {
                components.pop();
            } else if absolute {
                return Err("path escapes an absolute root".to_string());
            } else {
                components.push(component.to_string());
            }
            continue;
        }
        components.push(component.to_string());
    }

    Ok(LexicalReleaseReportPath { prefix, components })
}

pub(super) fn windows_drive_prefix(path: &str) -> Option<&str> {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic() {
        Some(&path[..2])
    } else {
        None
    }
}

pub(super) fn path_is_under_any(path: &Utf8Path, roots: &[Utf8PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

pub(super) fn print_import_ci_release_evidence_report(
    report: &ImportCiReleaseEvidenceReport,
    format: OutputFormat,
    report_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_import_ci_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("CI release evidence import: {}", report.evidence_dir);
            for item in &report.items {
                let source = item
                    .source
                    .as_deref()
                    .map(|source| format!(" from {source}"))
                    .unwrap_or_default();
                let path = item
                    .path
                    .as_deref()
                    .map(|path| format!(" -> {path}"))
                    .unwrap_or_default();
                println!(
                    "- {}: {}{}{} - {}",
                    item.name, item.status, source, path, item.value
                );
            }
            println!("- report: {report_path}");
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}

pub(super) fn collect_protocol_snapshot_dirs_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut dirs = Vec::new();
    collect_protocol_snapshot_dirs_recursive_inner(root, &mut dirs)?;
    dirs.sort();
    Ok(dirs)
}

pub(super) fn collect_protocol_snapshot_dirs_recursive_inner(
    current: &Utf8Path,
    dirs: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(current)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("protocol artifact contains symlink: {current}").into());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    if current.join("typescript").is_dir() && current.join("json-schema").is_dir() {
        dirs.push(current.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "protocol artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("protocol artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_protocol_snapshot_dirs_recursive_inner(&path, dirs)?;
        }
    }
    Ok(())
}

pub(super) fn artifact_os_from_path(path: &Utf8Path) -> Option<&'static str> {
    doctor_artifact_os(path).or_else(|| os_from_artifact_path_tokens(&artifact_path_tokens(path)))
}

pub(super) fn artifact_path_tokens(path: &Utf8Path) -> Vec<String> {
    path.as_str()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn os_from_artifact_path_tokens(tokens: &[String]) -> Option<&'static str> {
    if tokens.iter().any(|token| token == "linux") {
        Some("Linux")
    } else if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        Some("macOS")
    } else if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "windows" | "windowsx64" | "win" | "win32" | "win64"
        )
    }) {
        Some("Windows")
    } else {
        None
    }
}

pub(super) fn import_doctor_artifact_os(
    path: &Utf8Path,
    report: &DoctorReport,
) -> Result<&'static str, String> {
    let file_os = artifact_os_from_path(path);
    let report_os = report.os.as_deref().and_then(normalize_doctor_os_label);
    match (file_os, report_os) {
        (Some(file_os), Some(report_os)) if file_os == report_os => Ok(file_os),
        (Some(file_os), Some(report_os)) => Err(format!(
            "artifact path indicates {file_os}, but report os is {report_os}"
        )),
        (Some(file_os), None) => Ok(file_os),
        (None, Some(report_os)) => Ok(report_os),
        (None, None) => Err("could not infer OS from artifact path or report".to_string()),
    }
}

pub(super) fn validate_import_ci_run_match(
    actual_url: Option<&str>,
    expected_url: Option<&str>,
) -> Result<(), String> {
    let (Some(actual_url), Some(expected_url)) = (actual_url, expected_url) else {
        return Ok(());
    };
    let Some(actual) = github_actions_run_key(actual_url) else {
        return Err(format!("artifact has invalid ci_run_url `{actual_url}`"));
    };
    let Some(expected) = github_actions_run_key(expected_url) else {
        return Err(format!("expected CI run URL is invalid `{expected_url}`"));
    };
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "artifact is from {}/{} run {}, expected {}/{} run {}",
            actual.owner,
            actual.repo,
            actual.run_id,
            expected.owner,
            expected.repo,
            expected.run_id
        ))
    }
}

pub(super) fn validate_report_artifact_path_platform(
    path: &Utf8Path,
    report: &ValidateReport,
) -> Result<(), String> {
    let Some(path_platform) = validate_report_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    let platforms = validate_report_platforms(report);
    if platforms.contains(path_platform) {
        Ok(())
    } else {
        let report_platforms = if platforms.is_empty() {
            "unknown".to_string()
        } else {
            format_platform_set(&platforms)
        };
        Err(format!(
            "artifact path indicates {path_platform}, but report platform is {report_platforms}"
        ))
    }
}

pub(super) fn validate_report_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    match validate_report_file_name_platform(path, &["linux-x64", "macos", "windows-x64"]) {
        Ok(Some(platform)) => Ok(Some(platform)),
        Ok(None) => Ok(validate_report_parent_dir_platform(path)),
        Err(error) => Err(error.to_string()),
    }
}

pub(super) fn validate_report_parent_dir_platform(path: &Utf8Path) -> Option<&'static str> {
    let parent = path.parent()?.file_name()?;
    let tokens = path_component_tokens(parent);
    match tokens
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["linux", "x64"] | ["linuxx64"] => Some("linux-x64"),
        ["macos"] | ["mac"] | ["darwin"] | ["osx"] => Some("macos"),
        ["windows"] | ["windows", "x64"] | ["windowsx64"] | ["win64"] => Some("windows-x64"),
        _ => None,
    }
}

pub(super) fn path_component_tokens(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn validate_report_import_filename(report: &ValidateReport, kind: &str) -> String {
    let bundle = safe_evidence_filename_part(&validate_report_bundle_name(report));
    let mut platforms = validate_report_platforms(report)
        .into_iter()
        .collect::<Vec<_>>();
    platforms.sort_unstable();
    let platform = if platforms.is_empty() {
        "unknown".to_string()
    } else {
        platforms.join("+")
    };
    format!(
        "{bundle}.{platform}.{kind}.json",
        platform = safe_evidence_filename_part(&platform)
    )
}

pub(super) fn signing_import_destination(
    dir: &Utf8Path,
    source: &Utf8Path,
    platforms: &BTreeSet<SigningEvidencePlatform>,
) -> Utf8PathBuf {
    if platforms.contains(&SigningEvidencePlatform::Macos)
        && !platforms.contains(&SigningEvidencePlatform::Windows)
    {
        dir.join("signing-macos.log")
    } else if platforms.contains(&SigningEvidencePlatform::Windows)
        && !platforms.contains(&SigningEvidencePlatform::Macos)
    {
        dir.join("signing-windows.log")
    } else {
        dir.join("signing").join(format!(
            "{}.log",
            safe_evidence_filename_part(source.file_stem().unwrap_or("signing"))
        ))
    }
}

pub(super) fn safe_evidence_filename_part(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            ch
        } else {
            '-'
        };
        if mapped == '-' {
            if !last_dash {
                out.push(mapped);
            }
            last_dash = true;
        } else {
            out.push(mapped);
            last_dash = false;
        }
    }
    let trimmed = out.trim_matches(['-', '.', '_']).to_string();
    if trimmed.is_empty() {
        "artifact".to_string()
    } else {
        trimmed
    }
}
