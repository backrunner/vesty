use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PublishPlan {
    pub(super) packages: Vec<PublishPlanPackage>,
    pub(super) skipped_private: Vec<String>,
}

pub(super) const PUBLISH_PLAN_MAX_PACKAGES: usize = 128;
pub(super) const PUBLISH_PLAN_MAX_DEPENDENCIES: usize = 128;
pub(super) const PUBLISH_PLAN_MAX_SKIPPED_PRIVATE: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PublishPlanPackage {
    pub(super) order: usize,
    pub(super) level: usize,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) manifest_path: String,
    pub(super) internal_dependencies: Vec<String>,
}

pub(super) fn run_publish_plan(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty publish-plan --check` requires `--out <report>`".into());
        };
        let plan = read_publish_plan_report(report_path)?;
        validate_publish_plan(&plan)?;
        print_publish_plan(&plan, format, Some(report_path))?;
        return Ok(());
    }

    let metadata = cargo_metadata_json(workspace)?;
    let plan = workspace_publish_plan(&metadata)?;
    if let Some(path) = out {
        write_publish_plan_report(path, &plan)?;
    }
    print_publish_plan(&plan, format, out)
}

pub(super) const CRATE_PACKAGE_REPORT_VERSION: u32 = 1;
pub(super) const CRATE_PACKAGE_REPORT_GENERATOR: &str = "vesty-cli.crate-package.v1";
pub(super) const CRATE_PACKAGE_MAX_PACKAGES: usize = 128;
pub(super) const CRATE_PACKAGE_MAX_DEPENDENCIES: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CratePackageReport {
    pub(super) version: u32,
    pub(super) generator: String,
    pub(super) status: String,
    pub(super) publish_plan: PublishPlan,
    pub(super) packages: Vec<CratePackageEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CratePackageEntry {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) manifest_path: String,
    pub(super) publish_order: usize,
    pub(super) internal_dependencies: Vec<String>,
    pub(super) status: String,
    pub(super) reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CratePackageEvidence {
    pub(super) package_count: usize,
    pub(super) packaged_count: usize,
    pub(super) deferred_count: usize,
}

pub(super) fn run_crate_package(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty crate-package --check` requires `--out <report>`".into());
        };
        let report = read_crate_package_report(report_path)?;
        let evidence = validate_crate_package_report(&report)?;
        print_crate_package_report(&report, &evidence, format, Some(report_path))?;
        return Ok(());
    }

    let report = crate_package_report(workspace, true)?;
    let evidence = validate_crate_package_report(&report)?;
    if let Some(path) = out {
        write_crate_package_report(path, &report)?;
    }
    print_crate_package_report(&report, &evidence, format, out)?;
    if report.status == "ok" {
        Ok(())
    } else {
        Err("crate package readiness failed; inspect failed package entries".into())
    }
}

pub(super) fn crate_package_report(
    workspace: &Utf8Path,
    run_package: bool,
) -> Result<CratePackageReport, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata_json(workspace)?;
    let plan = workspace_publish_plan(&metadata)?;
    validate_publish_plan(&plan)?;

    let mut entries = Vec::new();
    for package in &plan.packages {
        if package.internal_dependencies.is_empty() {
            let (status, reason) = if run_package {
                match cargo_package_workspace_crate(workspace, &package.name) {
                    Ok(()) => ("packaged".to_string(), None),
                    Err(error) => ("failed".to_string(), Some(error)),
                }
            } else {
                ("packaged".to_string(), None)
            };
            entries.push(CratePackageEntry {
                name: package.name.clone(),
                version: package.version.clone(),
                manifest_path: package.manifest_path.clone(),
                publish_order: package.order,
                internal_dependencies: Vec::new(),
                status,
                reason,
            });
        } else {
            let dependencies = package.internal_dependencies.clone();
            entries.push(CratePackageEntry {
                name: package.name.clone(),
                version: package.version.clone(),
                manifest_path: package.manifest_path.clone(),
                publish_order: package.order,
                internal_dependencies: dependencies.clone(),
                status: "deferred".to_string(),
                reason: Some(format!(
                    "requires published internal dependencies: {}",
                    dependencies.join(", ")
                )),
            });
        }
    }

    let status = if entries.iter().any(|entry| entry.status == "failed") {
        "failed"
    } else {
        "ok"
    };
    Ok(CratePackageReport {
        version: CRATE_PACKAGE_REPORT_VERSION,
        generator: CRATE_PACKAGE_REPORT_GENERATOR.to_string(),
        status: status.to_string(),
        publish_plan: plan,
        packages: entries,
    })
}

pub(super) fn cargo_package_workspace_crate(
    workspace: &Utf8Path,
    package: &str,
) -> Result<(), String> {
    let output = Command::new("cargo")
        .current_dir(workspace)
        .args(["package", "-p", package, "--allow-dirty", "--no-verify"])
        .output()
        .map_err(|error| format!("failed to run cargo package for {package}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo package failed for {package}: status {}; {}",
        output.status,
        captured_output_summary(&output)
    ))
}

pub(super) fn captured_output_summary(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut parts = Vec::new();
    if !stdout.trim().is_empty() {
        parts.push(format!(
            "stdout: {}",
            truncate_for_report(stdout.trim(), 2_000)
        ));
    }
    if !stderr.trim().is_empty() {
        parts.push(format!(
            "stderr: {}",
            truncate_for_report(stderr.trim(), 4_000)
        ));
    }
    if parts.is_empty() {
        "no stdout/stderr captured".to_string()
    } else {
        parts.join("; ")
    }
}

pub(super) fn truncate_for_report(text: &str, max_chars: usize) -> String {
    let mut sanitized = String::new();
    let mut previous_space = false;
    let mut truncated = false;
    let mut char_count = 0;
    for raw in text.chars() {
        let ch = if raw.is_control() || is_release_action_unsafe_format_char(raw) {
            ' '
        } else {
            raw
        };
        if ch.is_whitespace() {
            if !sanitized.is_empty() && !previous_space {
                if char_count >= max_chars {
                    truncated = true;
                    break;
                }
                sanitized.push(' ');
                previous_space = true;
                char_count += 1;
            }
            continue;
        }
        if char_count >= max_chars {
            truncated = true;
            break;
        }
        sanitized.push(ch);
        previous_space = false;
        char_count += 1;
    }

    let mut sanitized = sanitized.trim().to_string();
    if truncated {
        sanitized.push_str("...");
    }
    sanitized
}

pub(super) fn write_crate_package_report(
    path: &Utf8Path,
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;
    write_text_file(path, &(serde_json::to_string_pretty(report)? + "\n"))
}

pub(super) fn read_crate_package_report(
    path: &Utf8Path,
) -> Result<CratePackageReport, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("crate package report", path)?;
    serde_json::from_str::<CratePackageReport>(&text)
        .map_err(|error| format!("invalid crate package report JSON: {error}").into())
}

pub(super) fn validate_crate_package_report_path(
    path: &Utf8Path,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    let report = read_crate_package_report(path)?;
    validate_crate_package_report(&report)
}

pub(super) fn validate_crate_package_report_path_with_publish_plan(
    path: &Utf8Path,
    publish_plan_path: Option<&Utf8Path>,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    let report = read_crate_package_report(path)?;
    let evidence = validate_crate_package_report(&report)?;
    if let Some(publish_plan_path) = publish_plan_path {
        let plan = read_publish_plan_report(publish_plan_path)?;
        validate_publish_plan(&plan)?;
        validate_crate_package_publish_plan_identity(&report.publish_plan, &plan).map_err(
            |error| {
                format!("crate package report does not match crate publish plan evidence: {error}")
            },
        )?;
    }
    Ok(evidence)
}

pub(super) fn validate_crate_package_report(
    report: &CratePackageReport,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;

    if report.version != CRATE_PACKAGE_REPORT_VERSION {
        return Err(format!(
            "unsupported crate package report version {}; expected {}",
            report.version, CRATE_PACKAGE_REPORT_VERSION
        )
        .into());
    }
    if report.generator != CRATE_PACKAGE_REPORT_GENERATOR {
        return Err(format!(
            "unsupported crate package report generator {}; expected {}",
            report.generator, CRATE_PACKAGE_REPORT_GENERATOR
        )
        .into());
    }
    if report.packages.is_empty() {
        return Err("crate package report has no packages".into());
    }
    validate_publish_plan(&report.publish_plan)
        .map_err(|error| format!("crate package report publish plan is invalid: {error}"))?;
    validate_crate_package_report_entries_match_publish_plan(report)?;

    let mut seen_names = BTreeSet::<&str>::new();
    let mut seen_orders = BTreeSet::<usize>::new();
    let mut packaged_count = 0;
    let mut deferred_count = 0;
    let mut errors = Vec::new();
    let mut order_by_name = BTreeMap::<&str, usize>::new();

    for (index, package) in report.packages.iter().enumerate() {
        if package.name.trim().is_empty() {
            errors.push(format!("package at index {index} has empty name"));
        }
        if package.version.trim().is_empty() {
            errors.push(format!("package {} has empty version", package.name));
        }
        if package.manifest_path.trim().is_empty() {
            errors.push(format!("package {} has empty manifest path", package.name));
        }
        if package.publish_order != index + 1 {
            errors.push(format!(
                "package {} has publishOrder {}, expected {}",
                package.name,
                package.publish_order,
                index + 1
            ));
        }
        if !seen_names.insert(package.name.as_str()) {
            errors.push(format!("duplicate package {}", package.name));
        }
        if !seen_orders.insert(package.publish_order) {
            errors.push(format!("duplicate publishOrder {}", package.publish_order));
        }
        order_by_name.insert(package.name.as_str(), package.publish_order);

        match (
            package.internal_dependencies.is_empty(),
            package.status.as_str(),
        ) {
            (true, "packaged") => {
                packaged_count += 1;
                if package.reason.is_some() {
                    errors.push(format!(
                        "package {} is packaged but has a reason",
                        package.name
                    ));
                }
            }
            (true, "failed") => errors.push(format!(
                "package {} failed packaging: {}",
                package.name,
                package.reason.as_deref().unwrap_or("<missing reason>")
            )),
            (true, other) => errors.push(format!(
                "package {} has no internal dependencies but status is {other}, expected packaged",
                package.name
            )),
            (false, "deferred") => {
                deferred_count += 1;
                if package
                    .reason
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
                {
                    errors.push(format!("deferred package {} has no reason", package.name));
                }
            }
            (false, other) => errors.push(format!(
                "package {} has internal dependencies but status is {other}, expected deferred",
                package.name
            )),
        }
    }

    for package in &report.packages {
        let Some(package_order) = order_by_name.get(package.name.as_str()).copied() else {
            continue;
        };
        for dependency in &package.internal_dependencies {
            match order_by_name.get(dependency.as_str()).copied() {
                Some(dependency_order) if dependency_order < package_order => {}
                Some(_) => errors.push(format!(
                    "package {} depends on {} but the dependency is not earlier in publish order",
                    package.name, dependency
                )),
                None => errors.push(format!(
                    "package {} depends on unknown package {}",
                    package.name, dependency
                )),
            }
        }
    }

    if report.status != "ok" {
        errors.push(format!("report status is {}", report.status));
    }
    if packaged_count == 0 {
        errors.push("crate package report has no immediately packageable crates".to_string());
    }

    if !errors.is_empty() {
        return Err(format!("crate package readiness failed: {}", errors.join("; ")).into());
    }

    Ok(CratePackageEvidence {
        package_count: report.packages.len(),
        packaged_count,
        deferred_count,
    })
}

pub(super) fn validate_crate_package_report_entries_match_publish_plan(
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    if report.packages.len() != report.publish_plan.packages.len() {
        errors.push(format!(
            "crate package report has {} package entries but embedded publish plan has {}",
            report.packages.len(),
            report.publish_plan.packages.len()
        ));
    }

    for (index, expected) in report.publish_plan.packages.iter().enumerate() {
        let Some(actual) = report.packages.get(index) else {
            errors.push(format!(
                "embedded publish plan package {} is missing from crate package entries",
                expected.name
            ));
            continue;
        };
        if actual.name != expected.name {
            errors.push(format!(
                "package entry at index {index} is {}, expected {} from embedded publish plan",
                actual.name, expected.name
            ));
        }
        if actual.version != expected.version {
            errors.push(format!(
                "package {} has version {}, expected {} from embedded publish plan",
                actual.name, actual.version, expected.version
            ));
        }
        if actual.manifest_path != expected.manifest_path {
            errors.push(format!(
                "package {} has manifest path {}, expected {} from embedded publish plan",
                actual.name, actual.manifest_path, expected.manifest_path
            ));
        }
        if actual.publish_order != expected.order {
            errors.push(format!(
                "package {} has publishOrder {}, expected {} from embedded publish plan",
                actual.name, actual.publish_order, expected.order
            ));
        }
        if actual.internal_dependencies != expected.internal_dependencies {
            errors.push(format!(
                "package {} has internal dependencies {:?}, expected {:?} from embedded publish plan",
                actual.name, actual.internal_dependencies, expected.internal_dependencies
            ));
        }
    }

    for actual in report
        .packages
        .iter()
        .skip(report.publish_plan.packages.len())
    {
        errors.push(format!(
            "crate package entry {} is not present in embedded publish plan",
            actual.name
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "crate package report is out of sync with embedded publish plan: {}",
            errors.join("; ")
        )
        .into())
    }
}

pub(super) fn validate_crate_package_publish_plan_identity(
    embedded: &PublishPlan,
    external: &PublishPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    if embedded.packages.len() != external.packages.len() {
        errors.push(format!(
            "embedded publish plan has {} packages but external publish plan has {}",
            embedded.packages.len(),
            external.packages.len()
        ));
    }
    if embedded.skipped_private != external.skipped_private {
        errors.push(format!(
            "embedded publish plan skipped_private {:?}, expected {:?}",
            embedded.skipped_private, external.skipped_private
        ));
    }

    for (index, expected) in external.packages.iter().enumerate() {
        let Some(actual) = embedded.packages.get(index) else {
            errors.push(format!(
                "external publish plan package {} is missing from embedded publish plan",
                expected.name
            ));
            continue;
        };
        if actual.order != expected.order {
            errors.push(format!(
                "publish plan package {} has order {}, expected {}",
                actual.name, actual.order, expected.order
            ));
        }
        if actual.level != expected.level {
            errors.push(format!(
                "publish plan package {} has level {}, expected {}",
                actual.name, actual.level, expected.level
            ));
        }
        if actual.name != expected.name {
            errors.push(format!(
                "publish plan package at index {index} is {}, expected {}",
                actual.name, expected.name
            ));
        }
        if actual.version != expected.version {
            errors.push(format!(
                "publish plan package {} has version {}, expected {}",
                actual.name, actual.version, expected.version
            ));
        }
        if actual.manifest_path != expected.manifest_path {
            errors.push(format!(
                "publish plan package {} has manifest path {}, expected {}",
                actual.name, actual.manifest_path, expected.manifest_path
            ));
        }
        if actual.internal_dependencies != expected.internal_dependencies {
            errors.push(format!(
                "publish plan package {} has internal dependencies {:?}, expected {:?}",
                actual.name, actual.internal_dependencies, expected.internal_dependencies
            ));
        }
    }

    for actual in embedded.packages.iter().skip(external.packages.len()) {
        errors.push(format!(
            "embedded publish plan package {} is not present in external publish plan",
            actual.name
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; ").into())
    }
}

pub(super) fn validate_crate_package_report_shape(
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("crate package report generator", &report.generator)?;
    validate_release_action_text("crate package report status", &report.status)?;
    validate_publish_plan_shape(&report.publish_plan)?;
    if report.packages.is_empty() {
        return Err("crate package report has no packages".into());
    }
    if report.packages.len() > CRATE_PACKAGE_MAX_PACKAGES {
        return Err(format!(
            "crate package report has too many packages: {} exceeds maximum {CRATE_PACKAGE_MAX_PACKAGES}",
            report.packages.len()
        )
        .into());
    }

    for package in &report.packages {
        validate_release_action_text("crate package name", &package.name)?;
        validate_release_action_text(
            &format!("crate package `{}` version", package.name),
            &package.version,
        )?;
        validate_release_action_text(
            &format!("crate package `{}` manifest path", package.name),
            &package.manifest_path,
        )?;
        validate_release_action_text(
            &format!("crate package `{}` status", package.name),
            &package.status,
        )?;
        if let Some(reason) = &package.reason {
            validate_release_action_text(
                &format!("crate package `{}` reason", package.name),
                reason,
            )?;
        }
        if package.internal_dependencies.len() > CRATE_PACKAGE_MAX_DEPENDENCIES {
            return Err(format!(
                "crate package `{}` has too many internal dependencies: {} exceeds maximum {CRATE_PACKAGE_MAX_DEPENDENCIES}",
                package.name,
                package.internal_dependencies.len()
            )
            .into());
        }
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &package.internal_dependencies {
            validate_release_action_text(
                &format!("crate package `{}` internal dependency", package.name),
                dependency,
            )?;
            if !seen_dependencies.insert(dependency.as_str()) {
                return Err(format!(
                    "crate package `{}` has duplicate internal dependency `{dependency}`",
                    package.name
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn print_crate_package_report(
    report: &CratePackageReport,
    evidence: &CratePackageEvidence,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!(
                "crate package readiness: {} ({} packageable now, {} deferred)",
                report.status, evidence.packaged_count, evidence.deferred_count
            );
            for package in &report.packages {
                let deps = if package.internal_dependencies.is_empty() {
                    "-".to_string()
                } else {
                    package.internal_dependencies.join(", ")
                };
                let reason = package
                    .reason
                    .as_deref()
                    .map(|reason| format!("; {reason}"))
                    .unwrap_or_default();
                println!(
                    "{:>2}. [{}] {} {} (deps: {}){}",
                    package.publish_order,
                    package.status,
                    package.name,
                    package.version,
                    deps,
                    reason
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

pub(super) fn run_npm_pack(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty npm-pack --check` requires `--out <report>`".into());
        };
        let entries = read_npm_pack_report(report_path)?;
        let evidence = validate_npm_pack_entries(&entries)?;
        print_npm_pack_report(&entries, &evidence, format, Some(report_path))?;
        return Ok(());
    }

    let raw_report = npm_pack_dry_run_json(workspace)?;
    let entries = parse_npm_pack_command_output(&raw_report)?;
    let evidence = validate_npm_pack_entries(&entries)?;
    let report_text = serde_json::to_string_pretty(&entries)? + "\n";
    if let Some(path) = out {
        write_text_file(path, &report_text)?;
    }
    print_npm_pack_report(&entries, &evidence, format, out)
}

pub(super) const DEPENDENCY_BASELINE_REPORT_VERSION: u32 = 1;
pub(super) const DEPENDENCY_BASELINE_REPORT_GENERATOR: &str = "vesty-cli.dependency-baseline.v1";
pub(super) const DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME: &str =
    "cargo workspace external dependency baseline coverage";
pub(super) const DEPENDENCY_BASELINE_MAX_CHECKS: usize = 256;
pub(super) const DEPENDENCY_BASELINE_HINT_MAX_BYTES: usize = 64 * 1024;
pub(super) const TYPESCRIPT_BASELINE_RANGE: &str = "^7.0.2";
pub(super) const TYPESCRIPT_BASELINE_LOCK_VERSION: &str = "7.0.2";
pub(super) const REQUIRED_JS_BASELINE_PACKAGES: &[&str] = &["plugin-ui"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct JsLatestBaselineDependency {
    pub(super) workspace_package: &'static str,
    pub(super) dependency: &'static str,
    pub(super) node_package_path: &'static str,
    pub(super) expected_range: &'static str,
    pub(super) expected_lock_version: &'static str,
}

pub(super) const REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES: &[JsLatestBaselineDependency] = &[
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "react",
        node_package_path: "node_modules/react",
        expected_range: "latest",
        expected_lock_version: "19.2.7",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "@types/react",
        node_package_path: "node_modules/@types/react",
        expected_range: "latest",
        expected_lock_version: "19.2.17",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "vue",
        node_package_path: "node_modules/vue",
        expected_range: "latest",
        expected_lock_version: "3.5.40",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "svelte",
        node_package_path: "node_modules/svelte",
        expected_range: "latest",
        expected_lock_version: "5.56.5",
    },
];
pub(super) const REQUIRED_RUST_BASELINE_DEPENDENCIES: &[(&str, &str)] = &[
    ("arc-swap", "1.9.2"),
    ("atomic_float", "1.1.0"),
    ("camino", "1.2.4"),
    ("cargo_metadata", "0.23.1"),
    ("wry", "0.55.1"),
    ("vst3", vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE),
    ("raw-window-handle", "0.6.2"),
    ("rtrb", "0.3.4"),
    ("serde", "1.0.228"),
    ("serde_json", "1.0.150"),
    ("ts-rs", "12.0.1"),
    ("clap", "4.6.2"),
    ("mime_guess", "2.0.5"),
    ("plist", "1.10.0"),
    ("proc-macro-crate", "3.5.0"),
    ("proc-macro2", "1.0.106"),
    ("quote", "1.0.46"),
    ("schemars", "1.2.1"),
    ("syn", "2.0.119"),
    ("toml", "1.1.3"),
    ("sha2", "0.11.0"),
    ("tempfile", "3.27.0"),
    ("thiserror", "2.0.18"),
    ("tracing", "0.1.44"),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DependencyBaselineReport {
    pub(super) version: u32,
    pub(super) generator: String,
    pub(super) status: String,
    pub(super) checks: Vec<DependencyBaselineCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DependencyBaselineCheck {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) path: String,
    pub(super) expected: String,
    pub(super) actual: Option<String>,
    pub(super) status: String,
    pub(super) hint: Option<String>,
}

pub(super) fn run_dependency_baseline(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    latest: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let current = if latest {
        dependency_baseline_report_with_latest(workspace, &CommandLatestDependencyFetcher)?
    } else {
        dependency_baseline_report(workspace)?
    };

    if check {
        let Some(report_path) = out else {
            return Err("`vesty dependency-baseline --check` requires `--out <report>`".into());
        };
        let expected = read_dependency_baseline_report(report_path)?;
        validate_dependency_baseline_report(&expected)?;
        if expected != current {
            return Err(
                "dependency baseline report drift; regenerate with `vesty dependency-baseline --out <report>`"
                    .into(),
            );
        }
        print_dependency_baseline_report(&current, format, Some(report_path))?;
        return Ok(());
    }

    if let Some(path) = out {
        write_dependency_baseline_report(path, &current)?;
    }
    print_dependency_baseline_report(&current, format, out)?;
    validate_dependency_baseline_report(&current)
}

pub(super) fn dependency_baseline_report(
    workspace: &Utf8Path,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    dependency_baseline_report_with_optional_latest(workspace, None)
}

pub(super) fn dependency_baseline_report_with_latest(
    workspace: &Utf8Path,
    latest: &dyn LatestDependencyFetcher,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    dependency_baseline_report_with_optional_latest(workspace, Some(latest))
}

pub(super) fn dependency_baseline_report_with_optional_latest(
    workspace: &Utf8Path,
    latest: Option<&dyn LatestDependencyFetcher>,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    let mut checks = Vec::new();
    let cargo_manifest = read_toml_file(&workspace.join("Cargo.toml"))?;
    checks.push(workspace_dependency_baseline_coverage_check(
        &cargo_manifest,
    ));
    for (name, expected) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        checks.push(dependency_baseline_check(
            &format!("cargo workspace dependency `{name}`"),
            "cargo",
            "Cargo.toml",
            expected,
            workspace_dependency_version(&cargo_manifest, name),
            Some("update the workspace dependency baseline only after registry/latest-deps review"),
        ));
    }
    checks.push(dependency_baseline_check(
        "Steinberg VST3 SDK baseline",
        "vst3-sdk",
        "crates/vesty-vst3-sys/src/lib.rs",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE,
        Some(vesty_vst3_sys::BINDING_BASELINE.steinberg_sdk.to_string()),
        Some("Steinberg SDK remains the VST3 source of truth; keep generated-header audit docs in sync"),
    ));
    checks.push(dependency_baseline_check(
        "upstream `vst3` crate binding baseline",
        "vst3-sdk",
        "crates/vesty-vst3-sys/src/lib.rs",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE,
        Some(vesty_vst3_sys::BINDING_BASELINE.upstream_vst3_crate.to_string()),
        Some("if upstream VST3 bindings change, update vesty-vst3-sys baseline and adapter tests together"),
    ));

    for package in REQUIRED_JS_BASELINE_PACKAGES {
        let path = format!("packages/{package}/package.json");
        let package_json = read_json_file(&workspace.join(&path))?;
        checks.push(dependency_baseline_check(
            "npm package `vesty-plugin-ui` TypeScript devDependency",
            "npm",
            &path,
            TYPESCRIPT_BASELINE_RANGE,
            json_dependency_version(&package_json, "devDependencies", "typescript"),
            Some(
                "run `npm install --workspaces typescript@latest --save-dev` after registry review",
            ),
        ));
    }

    let lock_path = "package-lock.json";
    let package_lock = read_json_file(&workspace.join(lock_path))?;
    checks.push(dependency_baseline_check(
        "npm lockfile `typescript` installed version",
        "npm-lock",
        lock_path,
        TYPESCRIPT_BASELINE_LOCK_VERSION,
        package_lock_node_package_version(&package_lock, "node_modules/typescript"),
        Some("run `npm install` after changing JS package dependency ranges"),
    ));
    for package in REQUIRED_JS_BASELINE_PACKAGES {
        checks.push(dependency_baseline_check(
            "npm lockfile `vesty-plugin-ui` TypeScript devDependency",
            "npm-lock",
            lock_path,
            TYPESCRIPT_BASELINE_RANGE,
            package_lock_workspace_dev_dependency(
                &package_lock,
                &format!("packages/{package}"),
                "typescript",
            ),
            Some("package-lock workspace entries must match package.json dependency ranges"),
        ));
    }
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        let package_path = format!("packages/{}", dependency.workspace_package);
        let package_json_path = format!("{package_path}/package.json");
        let package_json = read_json_file(&workspace.join(&package_json_path))?;
        checks.push(dependency_baseline_check(
            &format!(
                "npm package `vesty-plugin-ui` {} devDependency range",
                dependency.dependency
            ),
            "npm",
            &package_json_path,
            dependency.expected_range,
            json_dependency_version(&package_json, "devDependencies", dependency.dependency),
            Some("framework adapter devDependencies should stay on `latest` until a compatibility floor is intentionally pinned"),
        ));
        checks.push(dependency_baseline_check(
            &format!("npm lockfile `{}` installed version", dependency.dependency),
            "npm-lock",
            lock_path,
            dependency.expected_lock_version,
            package_lock_node_package_version(&package_lock, dependency.node_package_path),
            Some("run `npm install --workspaces` after reviewing the latest framework adapter dependency"),
        ));
    }

    if let Some(latest) = latest {
        append_dependency_latest_checks(&mut checks, latest);
    }

    let complete = checks.iter().all(|check| check.status == "ok");
    Ok(DependencyBaselineReport {
        version: DEPENDENCY_BASELINE_REPORT_VERSION,
        generator: DEPENDENCY_BASELINE_REPORT_GENERATOR.to_string(),
        status: if complete { "ok" } else { "failed" }.to_string(),
        checks,
    })
}

pub(super) trait LatestDependencyFetcher {
    fn latest_crate_version(&self, name: &str) -> Result<String, String>;
    fn latest_npm_version(&self, name: &str) -> Result<String, String>;
}

pub(super) struct CommandLatestDependencyFetcher;

impl LatestDependencyFetcher for CommandLatestDependencyFetcher {
    fn latest_crate_version(&self, name: &str) -> Result<String, String> {
        latest_crate_version_from_crates_io(name)
    }

    fn latest_npm_version(&self, name: &str) -> Result<String, String> {
        latest_npm_version_from_npm_view(name)
    }
}

pub(super) fn append_dependency_latest_checks(
    checks: &mut Vec<DependencyBaselineCheck>,
    latest: &dyn LatestDependencyFetcher,
) {
    for (name, expected) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        let registry_expected = rust_registry_latest_expected(name, expected);
        checks.push(dependency_latest_check(
            &format!("crates.io latest `{name}`"),
            "cargo-registry",
            "crates.io",
            &registry_expected,
            latest.latest_crate_version(name),
            "review crates.io release notes, then update workspace dependency and baseline together",
        ));
    }
    checks.push(dependency_latest_check(
        "npm registry latest `typescript`",
        "npm-registry",
        "registry.npmjs.org",
        TYPESCRIPT_BASELINE_LOCK_VERSION,
        latest.latest_npm_version("typescript"),
        "review TypeScript release notes, then run `npm install --workspaces typescript@latest --save-dev`",
    ));
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        checks.push(dependency_latest_check(
            &format!("npm registry latest `{}`", dependency.dependency),
            "npm-registry",
            "registry.npmjs.org",
            dependency.expected_lock_version,
            latest.latest_npm_version(dependency.dependency),
            "review framework adapter dependency release notes, then run `npm install --workspaces`",
        ));
    }
}

pub(super) fn dependency_baseline_check_key(kind: &str, name: &str) -> String {
    format!("{}:{}", kind.trim(), name.trim())
}

pub(super) fn expected_dependency_baseline_check_keys(include_latest: bool) -> BTreeSet<String> {
    let mut expected = BTreeSet::new();
    expected.insert(dependency_baseline_check_key(
        "cargo-baseline",
        DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME,
    ));
    for (name, _) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        expected.insert(dependency_baseline_check_key(
            "cargo",
            &format!("cargo workspace dependency `{name}`"),
        ));
    }
    expected.insert(dependency_baseline_check_key(
        "vst3-sdk",
        "Steinberg VST3 SDK baseline",
    ));
    expected.insert(dependency_baseline_check_key(
        "vst3-sdk",
        "upstream `vst3` crate binding baseline",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm",
        "npm package `vesty-plugin-ui` TypeScript devDependency",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm-lock",
        "npm lockfile `typescript` installed version",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm-lock",
        "npm lockfile `vesty-plugin-ui` TypeScript devDependency",
    ));
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        expected.insert(dependency_baseline_check_key(
            "npm",
            &format!(
                "npm package `vesty-plugin-ui` {} devDependency range",
                dependency.dependency
            ),
        ));
        expected.insert(dependency_baseline_check_key(
            "npm-lock",
            &format!("npm lockfile `{}` installed version", dependency.dependency),
        ));
    }

    if include_latest {
        for (name, _) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
            expected.insert(dependency_baseline_check_key(
                "cargo-registry",
                &format!("crates.io latest `{name}`"),
            ));
        }
        expected.insert(dependency_baseline_check_key(
            "npm-registry",
            "npm registry latest `typescript`",
        ));
        for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
            expected.insert(dependency_baseline_check_key(
                "npm-registry",
                &format!("npm registry latest `{}`", dependency.dependency),
            ));
        }
    }

    expected
}

pub(super) fn dependency_baseline_check(
    name: &str,
    kind: &str,
    path: &str,
    expected: &str,
    actual: Option<String>,
    hint: Option<&str>,
) -> DependencyBaselineCheck {
    let status = if actual.as_deref() == Some(expected) {
        "ok"
    } else {
        "failed"
    };
    DependencyBaselineCheck {
        name: name.to_string(),
        kind: kind.to_string(),
        path: path.to_string(),
        expected: expected.to_string(),
        actual,
        status: status.to_string(),
        hint: (status != "ok").then(|| {
            hint.unwrap_or("restore the dependency baseline")
                .to_string()
        }),
    }
}

pub(super) fn rust_registry_latest_expected(name: &str, manifest_expected: &str) -> String {
    match name {
        // Cargo dependency requirements ignore SemVer build metadata, but crates.io
        // exposes toml's current release with the spec marker in the version string.
        "toml" => "1.1.3+spec-1.1.0".to_string(),
        _ => manifest_expected.to_string(),
    }
}

pub(super) fn workspace_dependency_baseline_coverage_check(
    manifest: &toml::Value,
) -> DependencyBaselineCheck {
    let expected = REQUIRED_RUST_BASELINE_DEPENDENCIES
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect::<BTreeSet<_>>();
    let actual = workspace_external_dependency_names(manifest);
    dependency_baseline_check(
        DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME,
        "cargo-baseline",
        "Cargo.toml",
        &format_dependency_name_set(&expected),
        Some(format_dependency_name_set(&actual)),
        Some(
            "add every external [workspace.dependencies] entry to REQUIRED_RUST_BASELINE_DEPENDENCIES after latest-deps review",
        ),
    )
}

pub(super) fn workspace_external_dependency_names(manifest: &toml::Value) -> BTreeSet<String> {
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter(|(_, dependency)| !workspace_dependency_is_internal_path(dependency))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn workspace_dependency_is_internal_path(dependency: &toml::Value) -> bool {
    dependency
        .as_table()
        .is_some_and(|table| table.contains_key("path"))
}

pub(super) fn format_dependency_name_set(names: &BTreeSet<String>) -> String {
    names.iter().cloned().collect::<Vec<_>>().join(", ")
}

pub(super) fn dependency_latest_check(
    name: &str,
    kind: &str,
    path: &str,
    expected: &str,
    actual: Result<String, String>,
    hint: &str,
) -> DependencyBaselineCheck {
    match actual {
        Ok(actual) => {
            dependency_baseline_check(name, kind, path, expected, Some(actual), Some(hint))
        }
        Err(error) => DependencyBaselineCheck {
            name: name.to_string(),
            kind: kind.to_string(),
            path: path.to_string(),
            expected: expected.to_string(),
            actual: None,
            status: "failed".to_string(),
            hint: Some(format!("{hint}; latest registry query failed: {error}")),
        },
    }
}

pub(super) fn latest_crate_version_from_cargo_search(name: &str) -> Result<String, String> {
    let search_result = latest_crate_version_from_cargo_search_only(name);
    if search_result.is_ok() {
        return search_result;
    }
    let search_error = search_result
        .err()
        .unwrap_or_else(|| "cargo search failed".to_string());
    latest_crate_version_from_cargo_info(name)
        .map_err(|info_error| format!("{search_error}; cargo info fallback failed: {info_error}"))
}

pub(super) fn latest_crate_version_from_crates_io(name: &str) -> Result<String, String> {
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(format!("invalid crates.io crate name `{name}`"));
    }

    let url = format!("https://crates.io/api/v1/crates/{name}");
    let user_agent = format!(
        "User-Agent: vesty-cli/{} (https://github.com/backrunner/vesty)",
        env!("CARGO_PKG_VERSION")
    );
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--retry-delay",
            "1",
            "--retry-all-errors",
            "--max-time",
            "30",
            "--header",
            &user_agent,
            &url,
        ])
        .output();

    let api_result = match output {
        Ok(output) if output.status.success() => {
            let body = String::from_utf8_lossy(&output.stdout);
            parse_crates_io_latest_version(&body)
                .ok_or_else(|| format!("crates.io response did not include a version for `{name}`"))
        }
        Ok(output) => Err(format!(
            "crates.io API failed with status {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => Err(format!("failed to run curl for crates.io API: {error}")),
    };

    if api_result.is_ok() {
        return api_result;
    }
    let api_error = api_result
        .err()
        .unwrap_or_else(|| "crates.io API query failed".to_string());
    latest_crate_version_from_cargo_search(name)
        .map_err(|cargo_error| format!("{api_error}; cargo fallback failed: {cargo_error}"))
}

pub(super) fn parse_crates_io_latest_version(body: &str) -> Option<String> {
    let response: serde_json::Value = serde_json::from_str(body).ok()?;
    let crate_data = response.get("crate")?;
    crate_data
        .get("max_stable_version")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            crate_data
                .get("max_version")
                .and_then(serde_json::Value::as_str)
        })
        .filter(|version| !version.is_empty())
        .map(str::to_string)
}

pub(super) fn latest_crate_version_from_cargo_search_only(name: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["search", name, "--limit", "1"])
        .output()
        .map_err(|error| format!("failed to run cargo search: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "cargo search failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    parse_cargo_search_latest_version(name, &stdout)
        .ok_or_else(|| format!("cargo search did not return an exact `{name}` version line"))
}

pub(super) fn parse_cargo_search_latest_version(name: &str, stdout: &str) -> Option<String> {
    let prefix = format!("{name} = \"");
    stdout.lines().map(str::trim).find_map(|line| {
        line.strip_prefix(&prefix)?
            .split('"')
            .next()
            .map(str::to_string)
    })
}

pub(super) fn latest_crate_version_from_cargo_info(name: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["info", name])
        .output()
        .map_err(|error| format!("failed to run cargo info: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "cargo info failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    parse_cargo_info_version(&stdout)
        .ok_or_else(|| format!("cargo info did not return a `version:` line for `{name}`"))
}

pub(super) fn parse_cargo_info_version(stdout: &str) -> Option<String> {
    stdout.lines().map(str::trim).find_map(|line| {
        line.strip_prefix("version:")
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .map(str::to_string)
    })
}

pub(super) fn latest_npm_version_from_npm_view(name: &str) -> Result<String, String> {
    let output = Command::new("npm")
        .args(["view", name, "version"])
        .output()
        .map_err(|error| format!("failed to run npm view: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "npm view failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    let version = stdout.trim();
    if version.is_empty() {
        Err(format!("npm view did not return a version for `{name}`"))
    } else {
        Ok(version.to_string())
    }
}

pub(super) fn read_toml_file(path: &Utf8Path) -> Result<toml::Value, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("TOML input", path)?;
    toml::from_str::<toml::Value>(&text)
        .map_err(|error| format!("invalid TOML in {path}: {error}").into())
}

pub(super) fn read_json_file(
    path: &Utf8Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("JSON input", path)?;
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("invalid JSON in {path}: {error}").into())
}

pub(super) fn workspace_dependency_version(manifest: &toml::Value, name: &str) -> Option<String> {
    let dependency = manifest.get("workspace")?.get("dependencies")?.get(name)?;
    match dependency {
        toml::Value::String(version) => Some(version.clone()),
        toml::Value::Table(table) => table
            .get("version")
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

pub(super) fn json_dependency_version(
    package_json: &serde_json::Value,
    section: &str,
    name: &str,
) -> Option<String> {
    package_json
        .get(section)?
        .get(name)?
        .as_str()
        .map(str::to_string)
}

pub(super) fn package_lock_node_package_version(
    package_lock: &serde_json::Value,
    package_path: &str,
) -> Option<String> {
    package_lock
        .get("packages")?
        .get(package_path)?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

pub(super) fn package_lock_workspace_dev_dependency(
    package_lock: &serde_json::Value,
    workspace_path: &str,
    name: &str,
) -> Option<String> {
    package_lock
        .get("packages")?
        .get(workspace_path)?
        .get("devDependencies")?
        .get(name)?
        .as_str()
        .map(str::to_string)
}

pub(super) fn validate_dependency_baseline_report(
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;

    if report.version != DEPENDENCY_BASELINE_REPORT_VERSION {
        return Err(format!(
            "unsupported dependency baseline report version {}; expected {}",
            report.version, DEPENDENCY_BASELINE_REPORT_VERSION
        )
        .into());
    }
    if report.generator != DEPENDENCY_BASELINE_REPORT_GENERATOR {
        return Err(format!(
            "unsupported dependency baseline report generator {}; expected {}",
            report.generator, DEPENDENCY_BASELINE_REPORT_GENERATOR
        )
        .into());
    }
    if !matches!(report.status.as_str(), "ok" | "failed") {
        return Err(format!(
            "unsupported dependency baseline report status `{}`",
            report.status
        )
        .into());
    }

    let mut failed = Vec::new();
    for check in &report.checks {
        if !matches!(check.status.as_str(), "ok" | "failed") {
            return Err(format!(
                "unsupported dependency baseline check status `{}` for {}",
                check.status, check.name
            )
            .into());
        }

        let matches_expected = check.actual.as_deref() == Some(check.expected.as_str());
        match (check.status.as_str(), matches_expected) {
            ("ok", false) => {
                return Err(format!(
                    "dependency baseline check `{}` status ok is inconsistent with expected {}, actual {}",
                    check.name,
                    check.expected,
                    check.actual.as_deref().unwrap_or("<missing>")
                )
                .into());
            }
            ("failed", true) => {
                return Err(format!(
                    "dependency baseline check `{}` status failed is inconsistent with matching expected/actual {}",
                    check.name, check.expected
                )
                .into());
            }
            ("failed", false) => failed.push(format_dependency_baseline_failure(check)),
            ("ok", true) => {}
            _ => unreachable!("dependency baseline check status validated"),
        }
    }

    let expected_report_status = if failed.is_empty() { "ok" } else { "failed" };
    if report.status != expected_report_status {
        return Err(format!(
            "dependency baseline report status `{}` is inconsistent with {} failed check(s)",
            report.status,
            failed.len()
        )
        .into());
    }

    if !failed.is_empty() {
        return Err(format!("dependency baseline check failed: {}", failed.join("; ")).into());
    }
    Ok(())
}

pub(super) fn validate_dependency_baseline_report_shape(
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("dependency baseline report generator", &report.generator)?;
    validate_release_action_text("dependency baseline report status", &report.status)?;
    if report.checks.is_empty() {
        return Err("dependency baseline report has no checks".into());
    }
    if report.checks.len() > DEPENDENCY_BASELINE_MAX_CHECKS {
        return Err(format!(
            "dependency baseline report has too many checks: {} exceeds maximum {DEPENDENCY_BASELINE_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("dependency baseline check name", &check.name)?;
        validate_release_action_text(
            &format!("dependency baseline `{}` kind", check.name),
            &check.kind,
        )?;
        validate_release_action_text(
            &format!("dependency baseline `{}` path", check.name),
            &check.path,
        )?;
        validate_release_action_text(
            &format!("dependency baseline `{}` expected", check.name),
            &check.expected,
        )?;
        if let Some(actual) = &check.actual {
            validate_release_action_text(
                &format!("dependency baseline `{}` actual", check.name),
                actual,
            )?;
        }
        validate_release_action_text(
            &format!("dependency baseline `{}` status", check.name),
            &check.status,
        )?;
        if let Some(hint) = &check.hint {
            validate_dependency_baseline_hint_text(
                &format!("dependency baseline `{}` hint", check.name),
                hint,
            )?;
        }
        let key = dependency_baseline_check_key(&check.kind, &check.name);
        if !seen_checks.insert(key.clone()) {
            return Err(format!("duplicate dependency baseline check `{key}`").into());
        }
    }
    let include_latest = seen_checks
        .iter()
        .any(|key| key.starts_with("cargo-registry:") || key.starts_with("npm-registry:"));
    let expected_checks = expected_dependency_baseline_check_keys(include_latest);
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown dependency baseline check(s): {}",
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
            "dependency baseline report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_dependency_baseline_hint_text(
    label: &str,
    value: &str,
) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > DEPENDENCY_BASELINE_HINT_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {DEPENDENCY_BASELINE_HINT_MAX_BYTES} bytes"
        ));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(format!(
            "{label} must not contain control characters other than tab/newline"
        ));
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!(
            "{label} must not contain unsafe Unicode format characters"
        ));
    }
    Ok(())
}

pub(super) fn format_dependency_baseline_failure(check: &DependencyBaselineCheck) -> String {
    format!(
        "{} expected {}, actual {}",
        check.name,
        check.expected,
        check.actual.as_deref().unwrap_or("<missing>")
    )
}

pub(super) fn write_dependency_baseline_report(
    path: &Utf8Path,
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;
    write_text_file(path, &(serde_json::to_string_pretty(report)? + "\n"))
}

pub(super) fn read_dependency_baseline_report(
    path: &Utf8Path,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("dependency baseline report", path)?;
    serde_json::from_str::<DependencyBaselineReport>(&text)
        .map_err(|error| format!("invalid dependency baseline report JSON: {error}").into())
}

pub(super) fn print_dependency_baseline_report(
    report: &DependencyBaselineReport,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("dependency baseline: {}", report.status);
            for check in &report.checks {
                println!(
                    "- [{}] {}: expected {}, actual {}",
                    check.status,
                    check.name,
                    check.expected,
                    check.actual.as_deref().unwrap_or("<missing>")
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

pub(super) fn print_npm_pack_report(
    entries: &[NpmPackEntry],
    evidence: &NpmPackEvidence,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_npm_pack_entries_shape(entries)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(entries)?);
        }
        OutputFormat::Text => {
            println!(
                "npm pack dry-run: {} package(s), {} file(s)",
                evidence.package_count, evidence.total_files
            );
            println!("packages: {}", evidence.packages.join(", "));
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

pub(super) fn npm_pack_dry_run_json(
    workspace: &Utf8Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if !workspace.is_dir() {
        return Err(format!("npm workspace directory does not exist: {workspace}").into());
    }
    let output = Command::new("npm")
        .args(["pack", "--workspaces", "--dry-run", "--json"])
        .current_dir(workspace)
        .output()
        .map_err(|error| format!("failed to run npm pack in {workspace}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "npm pack dry-run failed in {workspace}: status {}; stdout: {}; stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        )
        .into());
    }
    Ok(stdout.into_owned())
}

pub(super) fn print_publish_plan(
    plan: &PublishPlan,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(plan)?);
        }
        OutputFormat::Text => {
            println!("publishable workspace crates: {}", plan.packages.len());
            for package in &plan.packages {
                let deps = if package.internal_dependencies.is_empty() {
                    "-".to_string()
                } else {
                    package.internal_dependencies.join(", ")
                };
                println!(
                    "{:>2}. {} {} (level {}, deps: {})",
                    package.order, package.name, package.version, package.level, deps
                );
            }
            if !plan.skipped_private.is_empty() {
                println!(
                    "private workspace packages skipped: {}",
                    plan.skipped_private.join(", ")
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

pub(super) fn write_publish_plan_report(
    path: &Utf8Path,
    plan: &PublishPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;
    let text = serde_json::to_string_pretty(plan)? + "\n";
    write_text_file(path, &text)
}

pub(super) fn read_publish_plan_report(
    path: &Utf8Path,
) -> Result<PublishPlan, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("publish plan report", path)?;
    serde_json::from_str::<PublishPlan>(&text)
        .map_err(|error| format!("invalid publish plan JSON: {error}").into())
}

pub(super) fn workspace_publish_plan(
    metadata: &serde_json::Value,
) -> Result<PublishPlan, Box<dyn std::error::Error>> {
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not report packages")?;
    let workspace_member_ids = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not report workspace_members")?;

    let mut package_by_id = BTreeMap::<String, &serde_json::Value>::new();
    for package in packages {
        let id = package_string_field(package, "id")?.to_string();
        package_by_id.insert(id, package);
    }

    let mut workspace_packages = Vec::new();
    for member in workspace_member_ids {
        let member_id = member
            .as_str()
            .ok_or("cargo metadata workspace member id is not a string")?;
        let package = package_by_id.get(member_id).ok_or_else(|| {
            format!("workspace member package is missing from metadata: {member_id}")
        })?;
        workspace_packages.push(*package);
    }

    let mut workspace_index = BTreeMap::<String, usize>::new();
    let mut publishable_names = BTreeSet::<String>::new();
    let mut private_names = BTreeSet::<String>::new();
    let mut skipped_private = Vec::new();

    for (index, package) in workspace_packages.iter().enumerate() {
        let name = package_string_field(package, "name")?.to_string();
        workspace_index.insert(name.clone(), index);
        if package_is_publishable(package) {
            publishable_names.insert(name);
        } else {
            private_names.insert(name.clone());
            skipped_private.push(name);
        }
    }

    let mut dependencies = BTreeMap::<String, Vec<String>>::new();
    let mut blockers = Vec::new();
    for package in &workspace_packages {
        let name = package_string_field(package, "name")?.to_string();
        if !publishable_names.contains(&name) {
            continue;
        }
        let mut internal = Vec::new();
        for dependency in package
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("cargo metadata package did not report dependencies: {name}"))?
        {
            if dependency.get("kind").and_then(serde_json::Value::as_str) == Some("dev") {
                continue;
            }
            let dependency_name = dependency_string_field(dependency, "name")?.to_string();
            if publishable_names.contains(&dependency_name) {
                internal.push(dependency_name);
            } else if private_names.contains(&dependency_name) {
                blockers.push(format!(
                    "{name} depends on private workspace package {dependency_name}"
                ));
            }
        }
        internal.sort_by_key(|name| workspace_index.get(name).copied().unwrap_or(usize::MAX));
        internal.dedup();
        dependencies.insert(name, internal);
    }

    if !blockers.is_empty() {
        return Err(format!(
            "publish plan has private workspace dependency blockers: {}",
            blockers.join("; ")
        )
        .into());
    }

    let mut levels = BTreeMap::<String, usize>::new();
    let mut visiting = BTreeSet::<String>::new();
    for name in &publishable_names {
        publish_level(name, &dependencies, &mut levels, &mut visiting)?;
    }

    let mut package_rows = Vec::new();
    for package in workspace_packages {
        let name = package_string_field(package, "name")?.to_string();
        if !publishable_names.contains(&name) {
            continue;
        }
        let level = *levels
            .get(&name)
            .ok_or_else(|| format!("publish level missing for package {name}"))?;
        package_rows.push(PublishPlanPackage {
            order: 0,
            level,
            name: name.clone(),
            version: package_string_field(package, "version")?.to_string(),
            manifest_path: package_string_field(package, "manifest_path")?.to_string(),
            internal_dependencies: dependencies.get(&name).cloned().unwrap_or_default(),
        });
    }
    package_rows.sort_by_key(|package| {
        (
            package.level,
            workspace_index
                .get(&package.name)
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
    for (index, package) in package_rows.iter_mut().enumerate() {
        package.order = index + 1;
    }

    Ok(PublishPlan {
        packages: package_rows,
        skipped_private,
    })
}

pub(super) fn publish_level(
    name: &str,
    dependencies: &BTreeMap<String, Vec<String>>,
    levels: &mut BTreeMap<String, usize>,
    visiting: &mut BTreeSet<String>,
) -> Result<usize, Box<dyn std::error::Error>> {
    if let Some(level) = levels.get(name) {
        return Ok(*level);
    }
    if !visiting.insert(name.to_string()) {
        return Err(format!("workspace publish dependency cycle includes {name}").into());
    }

    let mut level = 1;
    for dependency in dependencies
        .get(name)
        .ok_or_else(|| format!("publish dependencies missing for package {name}"))?
    {
        level = level.max(publish_level(dependency, dependencies, levels, visiting)? + 1);
    }
    visiting.remove(name);
    levels.insert(name.to_string(), level);
    Ok(level)
}

pub(super) fn package_is_publishable(package: &serde_json::Value) -> bool {
    !matches!(
        package.get("publish").and_then(serde_json::Value::as_array),
        Some(registries) if registries.is_empty()
    )
}

pub(super) fn package_string_field<'a>(
    package: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    package
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("cargo metadata package field is not a string: {field}").into())
}

pub(super) fn dependency_string_field<'a>(
    dependency: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    dependency
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("cargo metadata dependency field is not a string: {field}").into())
}
