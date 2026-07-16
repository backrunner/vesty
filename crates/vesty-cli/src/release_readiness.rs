use super::*;

pub(super) fn host_profile_release_check(rows: &[serde_json::Value]) -> ReleaseCheckItem {
    let diff = daw_matrix_host_set_diff(rows);
    let issues = daw_matrix_host_set_issues(&diff);
    if issues.is_empty() {
        ReleaseCheckItem {
            name: "host profiles".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} release host profiles covered: {}",
                vesty_core::host_profiles().len(),
                release_host_profile_names().join(", ")
            ),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "host profiles".to_string(),
            status: "failed".to_string(),
            value: issues.join("; "),
            hint: Some("keep daw-matrix rows aligned with vesty-core host profiles".to_string()),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct DawMatrixHostSetDiff {
    pub(super) missing: Vec<String>,
    pub(super) duplicate: Vec<String>,
    pub(super) unknown: Vec<String>,
    pub(super) non_canonical: Vec<String>,
}

pub(super) fn release_host_profile_names() -> Vec<&'static str> {
    vesty_core::host_profiles()
        .iter()
        .map(|profile| profile.name)
        .collect()
}

pub(super) fn daw_matrix_host_set_diff(rows: &[serde_json::Value]) -> DawMatrixHostSetDiff {
    let mut seen_profile_ids = BTreeSet::new();
    let mut duplicate = Vec::new();
    let mut unknown = Vec::new();
    let mut non_canonical = Vec::new();
    for row in rows {
        let Some(host) = row.get("host").and_then(serde_json::Value::as_str) else {
            unknown.push("<missing host>".to_string());
            continue;
        };
        let Some(profile) = vesty_core::find_host_profile(host) else {
            unknown.push(host.to_string());
            continue;
        };
        if host != profile.name {
            non_canonical.push(format!("{host} -> {}", profile.name));
        }
        if !seen_profile_ids.insert(profile.id) {
            duplicate.push(profile.name.to_string());
        }
    }
    let missing = vesty_core::host_profiles()
        .iter()
        .filter(|profile| !seen_profile_ids.contains(profile.id))
        .map(|profile| profile.name.to_string())
        .collect();
    DawMatrixHostSetDiff {
        missing,
        duplicate,
        unknown,
        non_canonical,
    }
}

pub(super) fn daw_matrix_host_set_issues(diff: &DawMatrixHostSetDiff) -> Vec<String> {
    let mut issues = Vec::new();
    if !diff.missing.is_empty() {
        issues.push(format!("missing profile rows: {}", diff.missing.join(", ")));
    }
    if !diff.duplicate.is_empty() {
        issues.push(format!(
            "duplicate profile rows: {}",
            diff.duplicate.join(", ")
        ));
    }
    if !diff.unknown.is_empty() {
        issues.push(format!("unknown profile rows: {}", diff.unknown.join(", ")));
    }
    if !diff.non_canonical.is_empty() {
        issues.push(format!(
            "non-canonical profile rows: {}",
            diff.non_canonical.join(", ")
        ));
    }
    issues
}

pub(super) fn daw_matrix_release_check(rows: &[serde_json::Value]) -> ReleaseCheckItem {
    if daw_matrix_complete(rows) {
        ReleaseCheckItem {
            name: "daw matrix".to_string(),
            status: "ok".to_string(),
            value: "all required host smoke checks pass".to_string(),
            hint: None,
        }
    } else {
        let missing_hosts = rows
            .iter()
            .filter(|row| !missing_smoke_checks(row).is_empty())
            .filter_map(|row| row["host"].as_str())
            .collect::<Vec<_>>();
        ReleaseCheckItem {
            name: "daw matrix".to_string(),
            status: "failed".to_string(),
            value: format!("incomplete hosts: {}", missing_hosts.join(", ")),
            hint: Some(
                "run `vesty daw-matrix --write-template` and collect real host evidence"
                    .to_string(),
            ),
        }
    }
}

pub(super) fn host_row_release_check(row: &serde_json::Value) -> ReleaseCheckItem {
    let host = row["host"].as_str().unwrap_or("unknown host");
    let missing = missing_smoke_checks(row);
    if missing.is_empty() {
        ReleaseCheckItem {
            name: format!("daw smoke: {host}"),
            status: "ok".to_string(),
            value: "all smoke checks pass".to_string(),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: format!("daw smoke: {host}"),
            status: "failed".to_string(),
            value: format!("missing: {}", missing.join(", ")),
            hint: row["evidence"].as_str().map(|evidence| {
                format!("collect evidence in `{evidence}`; install detection is not enough")
            }),
        }
    }
}

pub(super) fn missing_smoke_checks(row: &serde_json::Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if row["platform_supported"].as_bool() != Some(true) {
        missing.push("platform");
    }
    missing.extend(
        vesty_core::RELEASE_SMOKE_CHECKS
            .iter()
            .copied()
            .filter(|key| row[*key].as_bool() != Some(true)),
    );
    missing
}

pub(super) fn protocol_release_check(
    protocol_snapshot: &Utf8Path,
    skip_protocol: bool,
    required: bool,
) -> ReleaseCheckItem {
    if skip_protocol {
        if required {
            return ReleaseCheckItem {
                name: "protocol snapshot".to_string(),
                status: "failed".to_string(),
                value: "cannot skip protocol snapshot when --require-release-artifacts is set"
                    .to_string(),
                hint: Some(format!(
                    "run `vesty export-types --out {protocol_snapshot} --check` in final release evidence"
                )),
            };
        }
        return ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "skipped".to_string(),
            value: "skipped by --skip-protocol".to_string(),
            hint: Some("release CI should normally run protocol drift check".to_string()),
        };
    }
    match check_protocol_export(protocol_snapshot) {
        Ok(()) => ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "ok".to_string(),
            value: protocol_snapshot.to_string(),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(format!(
                "run `vesty export-types --out {protocol_snapshot}` and commit/update the snapshot"
            )),
        },
    }
}

pub(super) fn binding_baseline_release_check() -> ReleaseCheckItem {
    ReleaseCheckItem {
        name: "vst3 binding baseline".to_string(),
        status: "ok".to_string(),
        value: binding_baseline_value(),
        hint: binding_baseline_hint(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PublishPlanEvidence {
    pub(super) package_count: usize,
    pub(super) skipped_private_count: usize,
    pub(super) final_package: String,
}

pub(super) fn publish_plan_release_check(
    path: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "crate publish plan",
            required,
            "pass `--publish-plan-report <path>` from `vesty publish-plan --out <path>`, or include `publish-plan/publish-plan.json` in release evidence",
        );
    };

    match validate_publish_plan_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "crate publish plan".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} publishable crates; {} private skipped; final crate: {}",
                evidence.package_count, evidence.skipped_private_count, evidence.final_package
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "crate publish plan".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some("regenerate with `vesty publish-plan --out publish-plan.json`".to_string()),
        },
    }
}

pub(super) fn validate_publish_plan_report(
    path: &Utf8Path,
) -> Result<PublishPlanEvidence, Box<dyn std::error::Error>> {
    let plan = read_publish_plan_report(path)?;
    validate_publish_plan(&plan)?;
    let final_package = plan
        .packages
        .iter()
        .max_by_key(|package| package.order)
        .map(|package| package.name.clone())
        .ok_or("publish plan has no packages")?;
    Ok(PublishPlanEvidence {
        package_count: plan.packages.len(),
        skipped_private_count: plan.skipped_private.len(),
        final_package,
    })
}

pub(super) fn validate_publish_plan(plan: &PublishPlan) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;

    if plan.packages.is_empty() {
        return Err("publish plan has no publishable packages".into());
    }

    let mut order_by_name = BTreeMap::<&str, usize>::new();
    let mut level_by_name = BTreeMap::<&str, usize>::new();
    let mut seen_orders = BTreeSet::<usize>::new();
    for (index, package) in plan.packages.iter().enumerate() {
        if package.name.trim().is_empty() {
            return Err(format!("publish plan package at index {index} has empty name").into());
        }
        if package.version.trim().is_empty() {
            return Err(format!("publish plan package {} has empty version", package.name).into());
        }
        if package.manifest_path.trim().is_empty() {
            return Err(format!(
                "publish plan package {} has empty manifest path",
                package.name
            )
            .into());
        }
        if package.order != index + 1 {
            return Err(format!(
                "publish plan package {} has order {}, expected {}",
                package.name,
                package.order,
                index + 1
            )
            .into());
        }
        if package.level == 0 {
            return Err(format!("publish plan package {} has zero level", package.name).into());
        }
        if order_by_name
            .insert(package.name.as_str(), package.order)
            .is_some()
        {
            return Err(format!("publish plan contains duplicate package {}", package.name).into());
        }
        if !seen_orders.insert(package.order) {
            return Err(format!("publish plan contains duplicate order {}", package.order).into());
        }
        level_by_name.insert(package.name.as_str(), package.level);
    }

    for package in &plan.packages {
        for dependency in &package.internal_dependencies {
            let Some(dependency_order) = order_by_name.get(dependency.as_str()) else {
                return Err(format!(
                    "publish plan package {} depends on unknown package {}",
                    package.name, dependency
                )
                .into());
            };
            if *dependency_order >= package.order {
                return Err(format!(
                    "publish plan orders dependency {} after dependent {}",
                    dependency, package.name
                )
                .into());
            }
            if level_by_name.get(dependency.as_str()).copied().unwrap_or(0) >= package.level {
                return Err(format!(
                    "publish plan level for dependency {} is not lower than {}",
                    dependency, package.name
                )
                .into());
            }
        }
    }

    Ok(())
}

pub(super) fn validate_publish_plan_shape(
    plan: &PublishPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    if plan.packages.is_empty() {
        return Err("publish plan has no publishable packages".into());
    }
    if plan.packages.len() > PUBLISH_PLAN_MAX_PACKAGES {
        return Err(format!(
            "publish plan has too many packages: {} exceeds maximum {PUBLISH_PLAN_MAX_PACKAGES}",
            plan.packages.len()
        )
        .into());
    }
    if plan.skipped_private.len() > PUBLISH_PLAN_MAX_SKIPPED_PRIVATE {
        return Err(format!(
            "publish plan has too many skipped private packages: {} exceeds maximum {PUBLISH_PLAN_MAX_SKIPPED_PRIVATE}",
            plan.skipped_private.len()
        )
        .into());
    }

    let mut seen_skipped_private = BTreeSet::new();
    for package in &plan.skipped_private {
        validate_release_action_text("publish plan skipped private package", package)?;
        if !seen_skipped_private.insert(package.as_str()) {
            return Err(format!(
                "publish plan contains duplicate skipped private package `{package}`"
            )
            .into());
        }
    }

    for package in &plan.packages {
        validate_release_action_text("publish plan package name", &package.name)?;
        validate_release_action_text(
            &format!("publish plan package `{}` version", package.name),
            &package.version,
        )?;
        validate_release_action_text(
            &format!("publish plan package `{}` manifest path", package.name),
            &package.manifest_path,
        )?;
        if package.internal_dependencies.len() > PUBLISH_PLAN_MAX_DEPENDENCIES {
            return Err(format!(
                "publish plan package `{}` has too many internal dependencies: {} exceeds maximum {PUBLISH_PLAN_MAX_DEPENDENCIES}",
                package.name,
                package.internal_dependencies.len()
            )
            .into());
        }
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &package.internal_dependencies {
            validate_release_action_text(
                &format!(
                    "publish plan package `{}` internal dependency",
                    package.name
                ),
                dependency,
            )?;
            if !seen_dependencies.insert(dependency.as_str()) {
                return Err(format!(
                    "publish plan package `{}` has duplicate internal dependency `{dependency}`",
                    package.name
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn crate_package_release_check(
    path: Option<&Utf8Path>,
    publish_plan_path: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "crate package readiness",
            required,
            "pass `--crate-package-report <path>` from `vesty crate-package --out <path>`, or include `crate-package/crate-package.json` in release evidence",
        );
    };

    match validate_crate_package_report_path_with_publish_plan(path, publish_plan_path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "crate package readiness".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} workspace crates; {} packageable now; {} deferred until internal dependencies publish",
                evidence.package_count, evidence.packaged_count, evidence.deferred_count
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "crate package readiness".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty crate-package --out crate-package.json`".to_string(),
            ),
        },
    }
}

pub(super) const REQUIRED_NPM_PACKAGES: [&str; 1] = ["vesty-plugin-ui"];
pub(super) const NPM_PACK_MAX_PACKAGES: usize = 16;
pub(super) const NPM_PACK_MAX_FILES_PER_PACKAGE: usize = 512;
pub(super) const NPM_PACK_MAX_TOTAL_FILES: usize = 2048;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NpmPackEntry {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) filename: String,
    pub(super) files: Vec<NpmPackFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NpmPackFile {
    pub(super) path: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct NpmPackCommandEntry {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) filename: String,
    pub(super) files: Vec<NpmPackCommandFile>,
}

#[derive(Debug, Deserialize)]
pub(super) struct NpmPackCommandFile {
    pub(super) path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NpmPackEvidence {
    pub(super) package_count: usize,
    pub(super) total_files: usize,
    pub(super) packages: Vec<String>,
}

pub(super) fn npm_pack_release_check(path: Option<&Utf8Path>, required: bool) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "npm package pack report",
            required,
            "pass `--npm-pack-report <path>` from `vesty npm-pack --out <path>`, or include `npm-pack/npm-pack.json` in release evidence",
        );
    };

    match validate_npm_pack_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "npm package pack report".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} package(s), {} file(s): {}",
                evidence.package_count,
                evidence.total_files,
                evidence.packages.join(", ")
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "npm package pack report".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty npm-pack --out npm-pack.json` after building JS packages"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn validate_npm_pack_report(
    path: &Utf8Path,
) -> Result<NpmPackEvidence, Box<dyn std::error::Error>> {
    let entries = read_npm_pack_report(path)?;
    validate_npm_pack_entries(&entries)
}

pub(super) fn read_npm_pack_report(
    path: &Utf8Path,
) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("npm pack report", path)?;
    parse_npm_pack_report_text(&text)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DependencyBaselineEvidence {
    pub(super) baseline_checks: usize,
    pub(super) latest_checks: usize,
}

pub(super) fn dependency_baseline_latest_release_check(
    path: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "dependency latest baseline",
            required,
            "pass `--dependency-baseline-report <path>` from `vesty dependency-baseline --latest --out <path>`, or include `dependency-baseline/dependency-baseline-latest.json` in release evidence",
        );
    };

    match validate_dependency_baseline_latest_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "dependency latest baseline".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} baseline check(s), {} latest registry check(s)",
                evidence.baseline_checks, evidence.latest_checks
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "dependency latest baseline".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty dependency-baseline --latest --out dependency-baseline-latest.json` after registry review"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn validate_dependency_baseline_latest_report(
    path: &Utf8Path,
) -> Result<DependencyBaselineEvidence, Box<dyn std::error::Error>> {
    let report = read_dependency_baseline_report(path)?;
    validate_dependency_baseline_report(&report)?;

    let expected_latest = expected_dependency_latest_check_names();
    let has_workspace_baseline_coverage = report.checks.iter().any(|check| {
        check.name == DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME && check.kind == "cargo-baseline"
    });
    if !has_workspace_baseline_coverage {
        return Err(format!(
            "dependency baseline report is missing required baseline check: {DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME}"
        )
        .into());
    }

    let actual_latest = report
        .checks
        .iter()
        .filter(|check| matches!(check.kind.as_str(), "cargo-registry" | "npm-registry"))
        .map(|check| check.name.clone())
        .collect::<BTreeSet<_>>();

    let missing = expected_latest
        .iter()
        .filter(|name| !actual_latest.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "dependency baseline report is missing latest registry checks: {}",
            missing.join(", ")
        )
        .into());
    }

    let unexpected = actual_latest
        .iter()
        .filter(|name| !expected_latest.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(format!(
            "dependency baseline report contains unexpected latest registry checks: {}",
            unexpected.join(", ")
        )
        .into());
    }

    Ok(DependencyBaselineEvidence {
        baseline_checks: report.checks.len().saturating_sub(actual_latest.len()),
        latest_checks: actual_latest.len(),
    })
}

pub(super) fn expected_dependency_latest_check_names() -> BTreeSet<String> {
    let mut expected = REQUIRED_RUST_BASELINE_DEPENDENCIES
        .iter()
        .map(|(name, _)| format!("crates.io latest `{name}`"))
        .collect::<BTreeSet<_>>();
    expected.insert("npm registry latest `typescript`".to_string());
    expected.extend(
        REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES
            .iter()
            .map(|dependency| format!("npm registry latest `{}`", dependency.dependency)),
    );
    expected
}

pub(super) fn parse_npm_pack_report_text(
    text: &str,
) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    serde_json::from_str::<Vec<NpmPackEntry>>(text)
        .map_err(|error| format!("invalid npm pack report JSON: {error}").into())
}

pub(super) fn parse_npm_pack_command_output(
    text: &str,
) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    let entries = serde_json::from_str::<Vec<NpmPackCommandEntry>>(text)
        .map_err(|error| format!("invalid npm pack command JSON: {error}"))?;
    Ok(entries
        .into_iter()
        .map(|entry| NpmPackEntry {
            name: entry.name,
            version: entry.version,
            filename: entry.filename,
            files: entry
                .files
                .into_iter()
                .map(|file| NpmPackFile { path: file.path })
                .collect(),
        })
        .collect())
}

pub(super) fn validate_npm_pack_entries(
    entries: &[NpmPackEntry],
) -> Result<NpmPackEvidence, Box<dyn std::error::Error>> {
    validate_npm_pack_entries_shape(entries)?;

    if entries.is_empty() {
        return Err("npm pack report has no packages".into());
    }

    let required = REQUIRED_NPM_PACKAGES.into_iter().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::<String>::new();
    let mut total_files = 0;
    for entry in entries {
        if !required.contains(entry.name.as_str()) {
            return Err(format!("unexpected npm package {}", entry.name).into());
        }
        if !seen.insert(entry.name.clone()) {
            return Err(format!("duplicate npm package {}", entry.name).into());
        }
        if entry.version.trim().is_empty() {
            return Err(format!("npm package {} has empty version", entry.name).into());
        }
        if !entry.filename.ends_with(".tgz") {
            return Err(
                format!("npm package {} filename does not end with .tgz", entry.name).into(),
            );
        }
        if entry.files.is_empty() {
            return Err(format!("npm package {} has no packed files", entry.name).into());
        }

        let mut has_package_json = false;
        let mut has_dist = false;
        for file in &entry.files {
            validate_npm_pack_file_path(&entry.name, &file.path)?;
            has_package_json |= file.path == "package.json";
            has_dist |= file.path.starts_with("dist/");
            total_files += 1;
        }
        if !has_package_json {
            return Err(format!("npm package {} is missing package.json", entry.name).into());
        }
        if !has_dist {
            return Err(format!("npm package {} has no dist files", entry.name).into());
        }
    }

    let missing = required
        .iter()
        .filter(|package| !seen.contains(**package))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("npm pack report missing packages: {}", missing.join(", ")).into());
    }

    let packages = REQUIRED_NPM_PACKAGES
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    Ok(NpmPackEvidence {
        package_count: entries.len(),
        total_files,
        packages,
    })
}

pub(super) fn validate_npm_pack_entries_shape(
    entries: &[NpmPackEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    if entries.is_empty() {
        return Err("npm pack report has no packages".into());
    }
    if entries.len() > NPM_PACK_MAX_PACKAGES {
        return Err(format!(
            "npm pack report has too many packages: {} exceeds maximum {NPM_PACK_MAX_PACKAGES}",
            entries.len()
        )
        .into());
    }

    let mut total_files = 0usize;
    for entry in entries {
        validate_release_action_text("npm package name", &entry.name)?;
        validate_release_action_text(
            &format!("npm package `{}` version", entry.name),
            &entry.version,
        )?;
        validate_release_action_text(
            &format!("npm package `{}` filename", entry.name),
            &entry.filename,
        )?;
        if entry.files.is_empty() {
            return Err(format!("npm package {} has no packed files", entry.name).into());
        }
        if entry.files.len() > NPM_PACK_MAX_FILES_PER_PACKAGE {
            return Err(format!(
                "npm package {} has too many packed files: {} exceeds maximum {NPM_PACK_MAX_FILES_PER_PACKAGE}",
                entry.name,
                entry.files.len()
            )
            .into());
        }
        total_files += entry.files.len();
        if total_files > NPM_PACK_MAX_TOTAL_FILES {
            return Err(format!(
                "npm pack report has too many packed files: {total_files} exceeds maximum {NPM_PACK_MAX_TOTAL_FILES}"
            )
            .into());
        }

        let mut seen_paths = BTreeSet::new();
        for file in &entry.files {
            validate_release_action_text(
                &format!("npm package `{}` packed path", entry.name),
                &file.path,
            )?;
            if !seen_paths.insert(file.path.as_str()) {
                return Err(format!(
                    "npm package {} has duplicate packed path `{}`",
                    entry.name, file.path
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn validate_npm_pack_file_path(
    package: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.trim() != path || path.is_empty() {
        return Err(format!("npm package {package} has invalid packed path `{path}`").into());
    }
    if path.contains('\\') || path.starts_with('/') || path == ".." || path.contains("../") {
        return Err(format!("npm package {package} has unsafe packed path `{path}`").into());
    }
    if path == "package.json" || path.starts_with("dist/") {
        Ok(())
    } else {
        Err(format!(
            "npm package {package} includes non-release file `{path}`; expected package.json or dist/**"
        )
        .into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Vst3SdkHeaderManifestEvidence {
    pub(super) header_count: usize,
    pub(super) baseline: String,
    pub(super) upstream_vst3_crate: String,
    pub(super) version_hint: Option<String>,
}

pub(super) fn vst3_sdk_manifest_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out vst3-sdk-headers.json` when auditing generated-header inputs"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_header_manifest(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} required header(s); Steinberg SDK {}; upstream vst3 crate {}{}",
                evidence.header_count,
                evidence.baseline,
                evidence.upstream_vst3_crate,
                evidence
                    .version_hint
                    .as_deref()
                    .map(|hint| format!("; {hint}"))
                    .unwrap_or_default()
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate from the official Steinberg VST3 SDK checkout with `vesty vst3-sdk manifest`"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn validate_vst3_sdk_header_manifest(
    path: &Utf8Path,
) -> Result<Vst3SdkHeaderManifestEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK header manifest", path)?;
    let manifest: vesty_vst3_sys::SdkHeaderInputManifest = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK header manifest JSON: {error}"))?;
    validate_vst3_sdk_header_manifest_content(&manifest)
}

pub(super) const VST3_SDK_MAX_HEADERS: usize = 128;
pub(super) const VST3_SDK_MAX_PLAN_CHECKS: usize = 32;
pub(super) const VST3_SDK_MAX_TEXT_LIST_ITEMS: usize = 128;
pub(super) const VST3_SDK_MAX_SURFACE_SYMBOLS: usize = 512;

pub(super) fn expected_vst3_sdk_binding_plan_check_names() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "sdk header inputs",
        "bindings module path",
        "binding emitter",
    ])
}

pub(super) fn expected_vst3_sdk_binding_surface_symbol_names() -> BTreeSet<String> {
    expected_vst3_sdk_binding_surface_symbol_specs()
        .keys()
        .cloned()
        .collect()
}

pub(super) fn expected_vst3_sdk_binding_surface_symbol_specs()
-> BTreeMap<String, vesty_vst3_sys::GeneratedBindingsSurfaceSymbolSpec> {
    vesty_vst3_sys::generated_bindings_surface_symbol_specs()
        .into_iter()
        .map(|symbol| (symbol.name.to_string(), symbol))
        .collect()
}

pub(super) fn validate_vst3_sdk_header_manifest_shape(
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK manifest generator", &manifest.generator)?;
    validate_release_action_text(
        "VST3 SDK manifest Steinberg SDK baseline",
        &manifest.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK manifest upstream vst3 crate baseline",
        &manifest.upstream_vst3_crate_baseline,
    )?;
    if let Some(version_hint) = &manifest.version_hint {
        validate_release_action_text("VST3 SDK manifest version hint", version_hint)?;
    }
    if manifest.headers.len() > VST3_SDK_MAX_HEADERS {
        return Err(format!(
            "VST3 SDK manifest has too many headers: {} exceeds maximum {VST3_SDK_MAX_HEADERS}",
            manifest.headers.len()
        )
        .into());
    }
    if manifest.missing_headers.len() > VST3_SDK_MAX_HEADERS {
        return Err(format!(
            "VST3 SDK manifest has too many missing headers: {} exceeds maximum {VST3_SDK_MAX_HEADERS}",
            manifest.missing_headers.len()
        )
        .into());
    }

    let mut seen_headers = BTreeSet::new();
    for header in &manifest.headers {
        validate_vst3_sdk_header_path_text("VST3 SDK manifest header path", &header.path)?;
        validate_release_action_text("VST3 SDK manifest header sha256", &header.sha256)?;
        if !seen_headers.insert(header.path.as_str()) {
            return Err(format!("duplicate VST3 SDK manifest header `{}`", header.path).into());
        }
    }
    validate_vst3_sdk_text_list(
        "VST3 SDK manifest missing header",
        &manifest.missing_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    Ok(())
}

pub(super) fn validate_vst3_sdk_binding_plan_shape(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK binding plan generator", &plan.generator)?;
    validate_release_action_text("VST3 SDK binding plan status", &plan.status)?;
    validate_release_action_text(
        "VST3 SDK binding plan Steinberg SDK baseline",
        &plan.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding plan upstream vst3 crate baseline",
        &plan.upstream_vst3_crate_baseline,
    )?;
    validate_release_action_text("VST3 SDK binding plan active backend", &plan.active_backend)?;
    validate_release_action_text("VST3 SDK binding plan sdk dir", &plan.sdk_dir)?;
    validate_release_action_text("VST3 SDK binding plan module", &plan.bindings_module)?;
    validate_vst3_sdk_header_manifest_shape(&plan.header_manifest)?;
    if plan.checks.is_empty() {
        return Err("VST3 SDK binding plan has no checks".into());
    }
    if plan.checks.len() > VST3_SDK_MAX_PLAN_CHECKS {
        return Err(format!(
            "VST3 SDK binding plan has too many checks: {} exceeds maximum {VST3_SDK_MAX_PLAN_CHECKS}",
            plan.checks.len()
        )
        .into());
    }
    let mut seen_checks = BTreeSet::new();
    for check in &plan.checks {
        validate_release_action_text("VST3 SDK binding plan check name", &check.name)?;
        validate_release_action_text(
            &format!("VST3 SDK binding plan `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(
            &format!("VST3 SDK binding plan `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(
                &format!("VST3 SDK binding plan `{}` hint", check.name),
                hint,
            )?;
        }
        if !seen_checks.insert(check.name.as_str()) {
            return Err(format!("duplicate VST3 SDK binding plan check `{}`", check.name).into());
        }
    }
    let expected_checks = expected_vst3_sdk_binding_plan_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .copied()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown VST3 SDK binding plan check(s): {}",
            unknown_checks.join(", ")
        )
        .into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .copied()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "VST3 SDK binding plan missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    validate_vst3_sdk_text_list(
        "VST3 SDK binding plan blocker",
        &plan.blockers,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding plan next step",
        &plan.next_steps,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        false,
    )?;
    Ok(())
}

pub(super) fn validate_vst3_sdk_binding_surface_shape(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK binding surface generator", &surface.generator)?;
    validate_release_action_text("VST3 SDK binding surface status", &surface.status)?;
    validate_release_action_text(
        "VST3 SDK binding surface Steinberg SDK baseline",
        &surface.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding surface upstream vst3 crate baseline",
        &surface.upstream_vst3_crate_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding surface active backend",
        &surface.active_backend,
    )?;
    validate_release_action_text("VST3 SDK binding surface sdk dir", &surface.sdk_dir)?;
    validate_vst3_sdk_header_manifest_shape(&surface.header_manifest)?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface required header",
        &surface.required_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface missing header",
        &surface.missing_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface missing symbol",
        &surface.missing_symbols,
        VST3_SDK_MAX_SURFACE_SYMBOLS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface blocker",
        &surface.blockers,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface note",
        &surface.notes,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        false,
    )?;
    if surface.symbols.is_empty() {
        return Err("VST3 SDK binding surface has no symbols".into());
    }
    if surface.symbols.len() > VST3_SDK_MAX_SURFACE_SYMBOLS {
        return Err(format!(
            "VST3 SDK binding surface has too many symbols: {} exceeds maximum {VST3_SDK_MAX_SURFACE_SYMBOLS}",
            surface.symbols.len()
        )
        .into());
    }
    let mut seen_symbols = BTreeSet::new();
    for symbol in &surface.symbols {
        validate_release_action_text("VST3 SDK binding surface symbol name", &symbol.name)?;
        validate_release_action_text(
            &format!("VST3 SDK binding surface `{}` kind", symbol.name),
            &symbol.kind,
        )?;
        validate_vst3_sdk_header_path_text(
            &format!("VST3 SDK binding surface `{}` header", symbol.name),
            &symbol.header,
        )?;
        validate_release_action_text(
            &format!("VST3 SDK binding surface `{}` purpose", symbol.name),
            &symbol.purpose,
        )?;
        let key = (&symbol.name, &symbol.kind, &symbol.header);
        if !seen_symbols.insert(key) {
            return Err(format!(
                "duplicate VST3 SDK binding surface symbol `{}` kind `{}` header `{}`",
                symbol.name, symbol.kind, symbol.header
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn validate_vst3_sdk_text_list(
    label: &str,
    values: &[String],
    max_items: usize,
    allow_empty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !allow_empty && values.is_empty() {
        return Err(format!("{label} list must not be empty").into());
    }
    if values.len() > max_items {
        return Err(format!(
            "{label} list has too many items: {} exceeds maximum {max_items}",
            values.len()
        )
        .into());
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_release_action_text(label, value)?;
        if !seen.insert(value.as_str()) {
            return Err(format!("duplicate {label} `{value}`").into());
        }
    }
    Ok(())
}

pub(super) fn validate_vst3_sdk_header_path_text(label: &str, value: &str) -> Result<(), String> {
    validate_release_action_text(label, value)?;
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(format!("{label} must be a relative normalized header path"));
    }
    Ok(())
}

pub(super) fn validate_vst3_sdk_header_manifest_content(
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
) -> Result<Vst3SdkHeaderManifestEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_header_manifest_shape(manifest)?;
    if manifest.version != vesty_vst3_sys::SDK_HEADER_MANIFEST_VERSION {
        return Err(format!(
            "manifest version is {}, expected {}",
            manifest.version,
            vesty_vst3_sys::SDK_HEADER_MANIFEST_VERSION
        )
        .into());
    }
    if manifest.generator != vesty_vst3_sys::SDK_HEADER_MANIFEST_GENERATOR {
        return Err(format!(
            "manifest generator is `{}`, expected `{}`",
            manifest.generator,
            vesty_vst3_sys::SDK_HEADER_MANIFEST_GENERATOR
        )
        .into());
    }
    if manifest.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        return Err(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            manifest.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        )
        .into());
    }
    if manifest.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        return Err(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            manifest.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        )
        .into());
    }

    let required = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut duplicate_headers = Vec::new();
    let mut unexpected_headers = Vec::new();
    let mut invalid_headers = Vec::new();
    for header in &manifest.headers {
        let path = header.path.as_str();
        if !required.contains(path) {
            unexpected_headers.push(path.to_string());
        }
        if !seen.insert(path) {
            duplicate_headers.push(path.to_string());
        }
        if header.size == 0 {
            invalid_headers.push(format!("{path} has zero size"));
        }
        if !is_lowercase_sha256_hex(&header.sha256) {
            invalid_headers.push(format!("{path} has invalid sha256"));
        }
    }

    let expected_missing = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .filter(|path| !seen.contains(**path))
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    let unexpected_missing = manifest
        .missing_headers
        .iter()
        .filter(|path| !required.contains(path.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let mut errors = Vec::new();
    if !duplicate_headers.is_empty() {
        errors.push(format!(
            "duplicate header entries: {}",
            duplicate_headers.join(", ")
        ));
    }
    if !unexpected_headers.is_empty() {
        errors.push(format!(
            "unexpected header entries: {}",
            unexpected_headers.join(", ")
        ));
    }
    if !unexpected_missing.is_empty() {
        errors.push(format!(
            "unexpected missing header entries: {}",
            unexpected_missing.join(", ")
        ));
    }
    if manifest.missing_headers != expected_missing {
        errors.push(format!(
            "missing_headers is {:?}, expected {:?}",
            manifest.missing_headers, expected_missing
        ));
    }
    if manifest.complete != expected_missing.is_empty() {
        errors.push(format!(
            "complete is {}, expected {}",
            manifest.complete,
            expected_missing.is_empty()
        ));
    }
    errors.extend(invalid_headers);
    if !manifest.complete {
        errors.push(format!(
            "manifest is incomplete; missing headers: {}",
            manifest.missing_headers.join(", ")
        ));
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkHeaderManifestEvidence {
        header_count: manifest.headers.len(),
        baseline: manifest.steinberg_sdk_baseline.clone(),
        upstream_vst3_crate: manifest.upstream_vst3_crate_baseline.clone(),
        version_hint: manifest.version_hint.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Vst3SdkBindingPlanEvidence {
    pub(super) header_count: usize,
    pub(super) status: String,
    pub(super) active_backend: String,
    pub(super) bindings_module: String,
}

pub(super) fn vst3_sdk_binding_plan_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --out generated-bindings-plan.json` when auditing generated-header readiness"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_binding_plan(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{}; {} required header(s); active backend {}; module {}",
                evidence.status,
                evidence.header_count,
                evidence.active_backend,
                evidence.bindings_module
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate with `vesty vst3-sdk binding-plan` from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn validate_vst3_sdk_binding_plan(
    path: &Utf8Path,
) -> Result<Vst3SdkBindingPlanEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK generated bindings plan", path)?;
    let plan: vesty_vst3_sys::GeneratedBindingsPlan = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK generated bindings plan JSON: {error}"))?;
    validate_vst3_sdk_binding_plan_content(&plan)
}

pub(super) fn validate_vst3_sdk_binding_plan_content(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
) -> Result<Vst3SdkBindingPlanEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_plan_shape(plan)?;
    let mut errors = Vec::new();
    if plan.version != vesty_vst3_sys::GENERATED_BINDINGS_PLAN_VERSION {
        errors.push(format!(
            "plan version is {}, expected {}",
            plan.version,
            vesty_vst3_sys::GENERATED_BINDINGS_PLAN_VERSION
        ));
    }
    if plan.generator != vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR {
        errors.push(format!(
            "plan generator is `{}`, expected `{}`",
            plan.generator,
            vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
        ));
    }
    if plan.bindings_generated {
        errors
            .push("generated bindings plan must not claim bindings are generated yet".to_string());
    }
    if plan.status != "ready-for-binding-generator" {
        errors.push(format!(
            "plan status is `{}`, expected `ready-for-binding-generator`",
            plan.status
        ));
    }
    if !plan.blockers.is_empty() {
        errors.push(format!("plan has blockers: {}", plan.blockers.join("; ")));
    }
    if plan.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        errors.push(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            plan.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        ));
    }
    if plan.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        errors.push(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            plan.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        ));
    }
    let expected_backend =
        vesty_vst3_sys::binding_backend_name(vesty_vst3_sys::BINDING_BASELINE.backend);
    if plan.active_backend != expected_backend {
        errors.push(format!(
            "active backend is `{}`, expected `{expected_backend}`",
            plan.active_backend
        ));
    }
    if !plan.bindings_module.ends_with(".rs") {
        errors.push(format!(
            "bindings module `{}` must point at a Rust .rs file",
            plan.bindings_module
        ));
    }
    if plan.sdk_dir.trim().is_empty() {
        errors.push("sdk dir is empty".to_string());
    }
    if let Err(error) = validate_vst3_sdk_header_manifest_content(&plan.header_manifest) {
        errors.push(format!("embedded header manifest invalid: {error}"));
    }
    if !plan.header_manifest.complete {
        errors.push("embedded header manifest is incomplete".to_string());
    }
    if !plan.checks.iter().any(|check| {
        check.name == "binding emitter"
            && check.status == "reserved"
            && check.value.contains("not enabled yet")
    }) {
        errors.push("plan must include reserved binding emitter check".to_string());
    }
    if plan.next_steps.is_empty() {
        errors.push("plan must include next steps".to_string());
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkBindingPlanEvidence {
        header_count: plan.header_manifest.headers.len(),
        status: plan.status.clone(),
        active_backend: plan.active_backend.clone(),
        bindings_module: plan.bindings_module.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Vst3SdkBindingSurfaceEvidence {
    pub(super) header_count: usize,
    pub(super) symbol_count: usize,
    pub(super) status: String,
    pub(super) active_backend: String,
}

pub(super) fn vst3_sdk_binding_surface_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out generated-bindings-surface.json` when auditing the reserved generated binding symbol surface"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_binding_surface(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{}; {} required header(s); {} symbol(s); active backend {}; bindings generated false",
                evidence.status,
                evidence.header_count,
                evidence.symbol_count,
                evidence.active_backend
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate with `vesty vst3-sdk binding-surface` from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn validate_vst3_sdk_binding_surface(
    path: &Utf8Path,
) -> Result<Vst3SdkBindingSurfaceEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK generated bindings surface", path)?;
    let surface: vesty_vst3_sys::GeneratedBindingsSurface = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK generated bindings surface JSON: {error}"))?;
    validate_vst3_sdk_binding_surface_content(&surface)
}

pub(super) fn validate_vst3_sdk_binding_surface_content(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
) -> Result<Vst3SdkBindingSurfaceEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_surface_shape(surface)?;
    let mut errors = Vec::new();
    if surface.version != vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_VERSION {
        errors.push(format!(
            "surface version is {}, expected {}",
            surface.version,
            vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_VERSION
        ));
    }
    if surface.generator != vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR {
        errors.push(format!(
            "surface generator is `{}`, expected `{}`",
            surface.generator,
            vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
        ));
    }
    if surface.bindings_generated {
        errors.push(
            "generated bindings surface must not claim bindings are generated yet".to_string(),
        );
    }
    if surface.status != "ready-for-binding-emitter" {
        errors.push(format!(
            "surface status is `{}`, expected `ready-for-binding-emitter`",
            surface.status
        ));
    }
    if !surface.blockers.is_empty() {
        errors.push(format!(
            "surface has blockers: {}",
            surface.blockers.join("; ")
        ));
    }
    if surface.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        errors.push(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            surface.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        ));
    }
    if surface.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        errors.push(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            surface.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        ));
    }
    let expected_backend =
        vesty_vst3_sys::binding_backend_name(vesty_vst3_sys::BINDING_BASELINE.backend);
    if surface.active_backend != expected_backend {
        errors.push(format!(
            "active backend is `{}`, expected `{expected_backend}`",
            surface.active_backend
        ));
    }
    if surface.sdk_dir.trim().is_empty() {
        errors.push("sdk dir is empty".to_string());
    }
    if let Err(error) = validate_vst3_sdk_header_manifest_content(&surface.header_manifest) {
        errors.push(format!("embedded header manifest invalid: {error}"));
    }
    if !surface.header_manifest.complete {
        errors.push("embedded header manifest is incomplete".to_string());
    }

    let required = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .map(|header| (*header).to_string())
        .collect::<Vec<_>>();
    let required_set = required.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if surface.required_headers != required {
        errors.push(
            "required_headers does not match the locked generated-header input set".to_string(),
        );
    }
    if !surface.missing_headers.is_empty() {
        errors.push(format!(
            "surface missing headers: {}",
            surface.missing_headers.join("; ")
        ));
    }
    let expected_missing_symbols = surface
        .symbols
        .iter()
        .filter(|symbol| symbol.header_present && !symbol.symbol_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if surface.missing_symbols != expected_missing_symbols {
        errors.push(format!(
            "surface missing_symbols does not match absent symbol tokens: expected {:?}, actual {:?}",
            expected_missing_symbols, surface.missing_symbols
        ));
    }
    if !surface.missing_symbols.is_empty() {
        errors.push(format!(
            "surface missing symbols: {}",
            surface.missing_symbols.join("; ")
        ));
    }
    if surface.symbols.is_empty() {
        errors.push("surface must include binding symbols".to_string());
    }
    let mut seen_symbols = BTreeSet::new();
    for symbol in &surface.symbols {
        if symbol.name.trim().is_empty() {
            errors.push("surface symbol has empty name".to_string());
        }
        if symbol.kind.trim().is_empty() {
            errors.push(format!("surface symbol `{}` has empty kind", symbol.name));
        }
        if !matches!(symbol.kind.as_str(), "interface" | "type" | "constant") {
            errors.push(format!(
                "surface symbol `{}` has unsupported kind `{}`",
                symbol.name, symbol.kind
            ));
        }
        if symbol.header.trim().is_empty() {
            errors.push(format!("surface symbol `{}` has empty header", symbol.name));
        } else if !required_set.contains(symbol.header.as_str()) {
            errors.push(format!(
                "surface symbol `{}` references header outside required set `{}`",
                symbol.name, symbol.header
            ));
        }
        if symbol.purpose.trim().is_empty() {
            errors.push(format!(
                "surface symbol `{}` has empty purpose",
                symbol.name
            ));
        }
        if !symbol.header_present {
            errors.push(format!(
                "surface symbol `{}` references missing header `{}`",
                symbol.name, symbol.header
            ));
        }
        if !symbol.symbol_present {
            errors.push(format!(
                "surface symbol `{}` is absent from header `{}`",
                symbol.name, symbol.header
            ));
        }
        if !seen_symbols.insert(symbol.name.as_str()) {
            errors.push(format!("duplicate surface symbol `{}`", symbol.name));
        }
    }
    let expected_symbol_names = expected_vst3_sdk_binding_surface_symbol_names();
    let actual_symbol_names = seen_symbols
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    let unknown_symbols = actual_symbol_names
        .difference(&expected_symbol_names)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_symbols.is_empty() {
        errors.push(format!(
            "surface contains unknown symbol(s): {}",
            unknown_symbols.join(", ")
        ));
    }
    let missing_symbols = expected_symbol_names
        .difference(&actual_symbol_names)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_symbols.is_empty() {
        errors.push(format!(
            "surface missing expected symbol(s): {}",
            missing_symbols.join(", ")
        ));
    }
    let expected_symbol_specs = expected_vst3_sdk_binding_surface_symbol_specs();
    for symbol in &surface.symbols {
        let Some(expected) = expected_symbol_specs.get(&symbol.name) else {
            continue;
        };
        if symbol.kind != expected.kind {
            errors.push(format!(
                "surface symbol `{}` kind is `{}`, expected `{}`",
                symbol.name, symbol.kind, expected.kind
            ));
        }
        if symbol.header != expected.header {
            errors.push(format!(
                "surface symbol `{}` header is `{}`, expected `{}`",
                symbol.name, symbol.header, expected.header
            ));
        }
        if symbol.purpose != expected.purpose {
            errors.push(format!(
                "surface symbol `{}` purpose is `{}`, expected `{}`",
                symbol.name, symbol.purpose, expected.purpose
            ));
        }
    }
    for required_symbol in [
        "IPlugView",
        "IAudioProcessor",
        "IEditController",
        "IParameterChanges",
        "IEventList",
        "IMidiMapping",
        "IBStream",
    ] {
        if !seen_symbols.contains(required_symbol) {
            errors.push(format!(
                "surface missing required symbol `{required_symbol}`"
            ));
        }
    }
    if surface.notes.is_empty() {
        errors.push("surface must include audit notes".to_string());
    } else if !surface
        .notes
        .iter()
        .any(|note| note.contains("does not") || note.contains("false"))
    {
        errors.push("surface notes must clarify that bindings are not generated yet".to_string());
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkBindingSurfaceEvidence {
        header_count: surface.header_manifest.headers.len(),
        symbol_count: surface.symbols.len(),
        status: surface.status.clone(),
        active_backend: surface.active_backend.clone(),
    })
}

pub(super) fn vst3_sdk_generated_scaffold_release_check(
    path: Option<&Utf8Path>,
) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings scaffold",
        "optional: generate with `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out generated.rs` when auditing generated-header scaffold drift",
        "metadata scaffold; bindings generated false",
        validate_vst3_sdk_generated_bindings_scaffold_text,
    )
}

pub(super) fn vst3_sdk_generated_abi_seed_release_check(
    path: Option<&Utf8Path>,
) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings ABI seed",
        "optional: generate with `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out generated-abi-seed.rs` when auditing foundational ABI aliases/constants",
        "ABI seed aliases/constants; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_abi_seed_text,
    )
}

pub(super) fn vst3_sdk_generated_abi_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings ABI layout",
        "optional: generate with `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out generated-abi.rs` when auditing foundational ABI layout fingerprints",
        "ABI layout fingerprints present; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_abi_text,
    )
}

pub(super) fn vst3_sdk_generated_interface_skeleton_release_check(
    path: Option<&Utf8Path>,
) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings interface skeleton",
        "optional: generate with `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out generated-interface-skeleton.rs` when auditing interface/vtable skeleton metadata",
        "interface/vtable skeleton metadata present; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_interface_skeleton_text,
    )
}

pub(super) fn optional_vst3_sdk_rust_artifact_release_check(
    path: Option<&Utf8Path>,
    name: &str,
    missing_hint: &str,
    ok_value: &str,
    validate: fn(&str) -> Result<(), String>,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: name.to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(missing_hint.to_string()),
        };
    };

    match read_text_file_no_symlink(name, path).and_then(|text| {
        validate(&text)
            .map(|_| ())
            .map_err(|error| -> Box<dyn std::error::Error> { error.into() })
    }) {
        Ok(()) => ReleaseCheckItem {
            name: name.to_string(),
            status: "ok".to_string(),
            value: ok_value.to_string(),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: name.to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate this optional VST3 SDK audit artifact from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn ci_run_url_release_check(url: Option<&str>, required: bool) -> ReleaseCheckItem {
    let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
        return optional_release_check_missing(
            "ci run url",
            required,
            "pass `--ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>` for release evidence",
        );
    };

    if is_github_actions_run_url(url) {
        ReleaseCheckItem {
            name: "ci run url".to_string(),
            status: "ok".to_string(),
            value: url.to_string(),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "ci run url".to_string(),
            status: "failed".to_string(),
            value: url.to_string(),
            hint: Some(
                "expected a GitHub Actions run URL, not a local log or project page".to_string(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GithubActionsRunKey {
    pub(super) owner: String,
    pub(super) repo: String,
    pub(super) run_id: String,
}

pub(super) fn is_github_actions_run_url(url: &str) -> bool {
    github_actions_run_key(url).is_some()
}

pub(super) fn github_actions_run_key(url: &str) -> Option<GithubActionsRunKey> {
    if url.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = url.strip_prefix("https://github.com/")?;
    let path = rest.split(['?', '#']).next().unwrap_or_default();
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 5
        || segments[0].is_empty()
        || segments[1].is_empty()
        || segments[2] != "actions"
        || segments[3] != "runs"
        || segments[4].is_empty()
        || !segments[4].chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let valid_shape = matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _] | [_, _, "actions", "runs", _, ""]
    ) || matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _, "attempts", attempt]
            if attempt.chars().all(|ch| ch.is_ascii_digit())
    ) || matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _, "attempts", attempt, ""]
            if attempt.chars().all(|ch| ch.is_ascii_digit())
    );

    valid_shape.then(|| GithubActionsRunKey {
        owner: segments[0].to_string(),
        repo: segments[1].to_string(),
        run_id: segments[4].to_string(),
    })
}

pub(super) fn ci_doctor_artifacts_release_check(
    dir: Option<&Utf8Path>,
    required: bool,
    ci_run_url: Option<&str>,
) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "ci doctor artifacts",
            required,
            "download/upload Linux, macOS and Windows doctor JSON artifacts and pass `--ci-doctor-dir <dir>`",
        );
    };

    let reports = match collect_doctor_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "ci doctor artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some("expected parseable doctor JSON artifacts from CI".to_string()),
            };
        }
    };

    let mut invalid_reports = Vec::new();
    let mut duplicate_os = Vec::new();
    let mut seen_os = BTreeSet::new();
    let valid_reports = reports
        .iter()
        .filter_map(|artifact| {
            if let Err(error) = validate_doctor_report(&artifact.report) {
                invalid_reports.push(format!("{}: {error}", artifact.path));
                return None;
            }
            if let Some(os) = artifact.os
                && !seen_os.insert(os)
            {
                duplicate_os.push(os);
                return None;
            }
            Some(artifact.clone())
        })
        .collect::<Vec<_>>();

    let coverage = DoctorArtifactCoverage::from_reports(&valid_reports, ci_run_url);
    let missing_os = coverage.missing_os();
    let os_mismatches = coverage.os_mismatches();
    let missing_checks = coverage.missing_checks();
    let run_mismatches = coverage.run_mismatches();
    if missing_os.is_empty()
        && invalid_reports.is_empty()
        && duplicate_os.is_empty()
        && os_mismatches.is_empty()
        && missing_checks.is_empty()
        && run_mismatches.is_empty()
    {
        ReleaseCheckItem {
            name: "ci doctor artifacts".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} doctor reports parsed ({})",
                reports.len(),
                coverage.present_os().join(", ")
            ),
            hint: None,
        }
    } else {
        let mut value = Vec::new();
        if !missing_os.is_empty() {
            value.push(format!("missing OS reports: {}", missing_os.join(", ")));
        }
        if !invalid_reports.is_empty() {
            value.push(format!("invalid reports: {}", invalid_reports.join("; ")));
        }
        if !duplicate_os.is_empty() {
            duplicate_os.sort_unstable();
            duplicate_os.dedup();
            value.push(format!("duplicate OS reports: {}", duplicate_os.join(", ")));
        }
        if !os_mismatches.is_empty() {
            value.push(format!("OS mismatch: {}", os_mismatches.join("; ")));
        }
        if !missing_checks.is_empty() {
            value.push(format!("missing checks: {}", missing_checks.join("; ")));
        }
        if !run_mismatches.is_empty() {
            value.push(format!("run URL mismatch: {}", run_mismatches.join("; ")));
        }
        ReleaseCheckItem {
            name: "ci doctor artifacts".to_string(),
            status: "failed".to_string(),
            value: value.join("; "),
            hint: Some(
                "CI should upload Linux, macOS and Windows doctor JSON snapshots from the same GitHub Actions run"
                    .to_string(),
            ),
        }
    }
}

pub(super) fn ci_release_check_artifacts_release_check(
    dir: Option<&Utf8Path>,
    required: bool,
    ci_run_url: Option<&str>,
) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "ci release-check artifacts",
            required,
            "pass `--ci-release-check-dir <dir>` containing Linux, macOS and Windows release-check*.json artifacts from CI",
        );
    };

    let reports = match collect_ci_release_check_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "ci release-check artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some("expected parseable release-check JSON artifacts from CI".to_string()),
            };
        }
    };

    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    let mut failures = Vec::new();
    let expected_run = ci_run_url.and_then(github_actions_run_key);
    for artifact in &reports {
        let Some(os) = artifact.os else {
            failures.push(format!(
                "{}: could not infer OS from artifact path",
                portable_report_path(&artifact.path)
            ));
            continue;
        };
        if !seen.insert(os) {
            duplicates.push(os);
        }
        if let Err(error) = validate_ci_release_check_report(&artifact.report) {
            failures.push(format!("{os} {}: {error}", artifact.path));
        }
        if let Err(error) = validate_ci_release_check_report_os_matches_path(artifact) {
            failures.push(error);
        }
        if let Some(expected) = expected_run.as_ref() {
            match artifact.report.ci_run_url.as_deref() {
                Some(actual_url) => match github_actions_run_key(actual_url) {
                    Some(actual) if actual == *expected => {}
                    Some(actual) => failures.push(format!(
                        "{os} {}: expected {}/{} run {}, got {}/{} run {}",
                        artifact.path,
                        expected.owner,
                        expected.repo,
                        expected.run_id,
                        actual.owner,
                        actual.repo,
                        actual.run_id
                    )),
                    None => failures.push(format!(
                        "{os} {}: invalid ci_run_url `{actual_url}`",
                        artifact.path
                    )),
                },
                None => failures.push(format!("{}: missing ci_run_url", artifact.path)),
            }
        }
    }

    let missing_os = ["Linux", "macOS", "Windows"]
        .into_iter()
        .filter(|os| !seen.contains(os))
        .collect::<Vec<_>>();

    if missing_os.is_empty() && duplicates.is_empty() && failures.is_empty() {
        ReleaseCheckItem {
            name: "ci release-check artifacts".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} release-check report(s) parsed ({})",
                reports.len(),
                seen.into_iter().collect::<Vec<_>>().join(", ")
            ),
            hint: None,
        }
    } else {
        let mut value = Vec::new();
        if !missing_os.is_empty() {
            value.push(format!("missing OS reports: {}", missing_os.join(", ")));
        }
        if !duplicates.is_empty() {
            duplicates.sort_unstable();
            duplicates.dedup();
            value.push(format!("duplicate OS reports: {}", duplicates.join(", ")));
        }
        if !failures.is_empty() {
            value.push(format!("invalid reports: {}", failures.join("; ")));
        }
        ReleaseCheckItem {
            name: "ci release-check artifacts".to_string(),
            status: "failed".to_string(),
            value: value.join("; "),
            hint: Some(
                "CI should upload Linux, macOS and Windows release-check*.json snapshots; external evidence gaps may remain failed, but local invariant checks must pass"
                    .to_string(),
            ),
        }
    }
}
