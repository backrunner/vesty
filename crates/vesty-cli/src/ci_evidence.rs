use super::*;

#[derive(Clone, Debug)]
pub(super) struct ImportCiOptions {
    pub(super) source: Utf8PathBuf,
    pub(super) dir: Utf8PathBuf,
    pub(super) ci_run_url: Option<String>,
    pub(super) ci_run_url_file: Option<Utf8PathBuf>,
    pub(super) template: bool,
    pub(super) overwrite: bool,
    pub(super) format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportCiReleaseEvidenceReport {
    pub(super) evidence_dir: String,
    pub(super) source: String,
    pub(super) items: Vec<ImportCiReleaseEvidenceItem>,
    pub(super) external_evidence_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportCiReleaseEvidenceItem {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) source: Option<String>,
    pub(super) path: Option<String>,
    pub(super) value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImportWriteOutcome {
    Imported,
    SkippedExisting,
}

impl ImportWriteOutcome {
    const fn status(self) -> &'static str {
        match self {
            ImportWriteOutcome::Imported => "imported",
            ImportWriteOutcome::SkippedExisting => "skipped",
        }
    }

    pub(super) const fn value(self) -> &'static str {
        match self {
            ImportWriteOutcome::Imported => "copied into release evidence",
            ImportWriteOutcome::SkippedExisting => {
                "destination exists; pass --overwrite to replace"
            }
        }
    }
}

pub(super) fn import_ci_release_evidence(
    options: ImportCiOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    require_existing_directory_no_symlink("CI artifact source", &options.source)?;
    reject_existing_path_symlink("release evidence dir", &options.dir)?;
    reject_existing_output_parent_symlink("release evidence dir", &options.dir)?;
    validate_import_ci_source_and_dir(&options.source, &options.dir)?;
    fs::create_dir_all(&options.dir)?;
    require_existing_directory_no_symlink("release evidence dir", &options.dir)?;

    let mut items = Vec::new();
    if options.template {
        let created = write_release_evidence_templates(&options.dir)?;
        items.push(import_ci_item(
            "release evidence template",
            "ok",
            None,
            Some(options.dir.as_path()),
            format!("{created} template file(s) created; existing files preserved"),
        ));
    }

    let text_files = collect_evidence_text_files_recursive(&options.source)?;
    let ci_run_url = import_ci_run_url_evidence(&options, &text_files, &mut items)?;
    let protocol_snapshot_dirs = import_protocol_snapshot_evidence(&options, &mut items)?;

    for path in collect_json_files_recursive(&options.source)? {
        if path_is_under_any(&path, &protocol_snapshot_dirs) {
            continue;
        }
        import_ci_json_artifact(&path, &options, ci_run_url.as_deref(), &mut items)?;
    }
    for path in collect_rust_files_recursive(&options.source)? {
        import_ci_rust_artifact(&path, &options, &mut items)?;
    }
    for path in text_files {
        import_ci_text_artifact(&path, &options, &mut items)?;
    }
    for path in collect_vst3_bundle_dirs_recursive(&options.source)? {
        import_ci_signed_bundle_artifact(&path, &options, &mut items)?;
    }

    let report = ImportCiReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        source: portable_report_path(&options.source),
        items,
        external_evidence_note: "import-ci only copies artifacts whose content is recognized by the release evidence parsers; it does not create DAW, platform smoke, validator-passed, signing or notarization passes by itself".to_string(),
    };
    validate_import_ci_release_evidence_report_shape(&report)?;
    let report_path = options.dir.join("import-ci-report.json");
    write_text_file(
        &report_path,
        &(serde_json::to_string_pretty(&report)? + "\n"),
    )?;
    print_import_ci_release_evidence_report(&report, format, &report_path)
}

pub(super) fn validate_import_ci_source_and_dir(
    source: &Utf8Path,
    dir: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = canonicalize_utf8(source)?;
    let dir = canonicalize_existing_or_parent(dir)?;
    if source == dir {
        return Err("CI artifact source and release evidence dir must be different".into());
    }
    if dir.starts_with(&source) {
        return Err(format!(
            "release evidence dir must not be inside CI artifact source: {dir} is under {source}"
        )
        .into());
    }
    if source.starts_with(&dir) {
        return Err(format!(
            "CI artifact source must not be inside release evidence dir: {source} is under {dir}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn canonicalize_existing_or_parent(
    path: &Utf8Path,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let absolute = absolute_utf8_path(path)?;
    if absolute.exists() {
        return canonicalize_utf8(&absolute);
    }

    let mut cursor = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        if cursor.exists() {
            let mut canonical = canonicalize_utf8(cursor)?;
            for part in suffix.iter().rev() {
                canonical.push(part);
            }
            return Ok(canonical);
        }
        let Some(file_name) = cursor.file_name() else {
            return Err(format!("cannot resolve parent for path: {path}").into());
        };
        suffix.push(file_name.to_string());
        cursor = cursor
            .parent()
            .ok_or_else(|| format!("cannot resolve parent for path: {path}"))?;
    }
}

pub(super) fn absolute_utf8_path(
    path: &Utf8Path,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| "current directory is not valid utf-8")?;
    Ok(cwd.join(path))
}

pub(super) fn canonicalize_utf8(
    path: &Utf8Path,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    Utf8PathBuf::from_path_buf(path.canonicalize()?)
        .map_err(|_| format!("path is not valid utf-8 after canonicalization: {path}").into())
}

pub(super) fn import_ci_run_url_evidence(
    options: &ImportCiOptions,
    text_files: &[Utf8PathBuf],
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let discovered = match (
        options.ci_run_url.as_deref(),
        options.ci_run_url_file.as_deref(),
    ) {
        (Some(url), Some(path)) => {
            let cli_url = url.trim();
            let file_url = read_required_ci_run_url_file(path)?;
            validate_import_ci_explicit_run_urls_match(cli_url, &file_url, path)?;
            Some(DiscoveredCiRunUrl {
                url: cli_url.to_string(),
                source: Some(path.to_path_buf()),
                had_failed_candidate: false,
            })
        }
        (Some(url), None) => Some(DiscoveredCiRunUrl {
            url: url.trim().to_string(),
            source: None,
            had_failed_candidate: false,
        }),
        (None, Some(path)) => Some(DiscoveredCiRunUrl {
            url: read_required_ci_run_url_file(path)?,
            source: Some(path.to_path_buf()),
            had_failed_candidate: false,
        }),
        (None, None) => auto_discover_ci_run_url_evidence(text_files, items)?,
    };

    let Some(discovered) = discovered else {
        items.push(import_ci_item(
            "ci run url",
            "skipped",
            None,
            None,
            "no GitHub Actions run URL found; pass --ci-run-url or --ci-run-url-file",
        ));
        return Ok(None);
    };
    if discovered.url.is_empty() {
        if discovered.had_failed_candidate {
            return Ok(None);
        }
        items.push(import_ci_item(
            "ci run url",
            "skipped",
            None,
            None,
            "no GitHub Actions run URL found; pass --ci-run-url or --ci-run-url-file",
        ));
        return Ok(None);
    }
    if !is_github_actions_run_url(&discovered.url) {
        return Err(format!("invalid GitHub Actions run URL: {}", discovered.url).into());
    }

    let destination = options.dir.join("ci-run-url.txt");
    let outcome = import_write_ci_run_url_file(&destination, &discovered.url, options.overwrite)?;
    items.push(import_ci_item(
        "ci run url",
        outcome.status(),
        discovered.source.as_deref(),
        Some(destination.as_path()),
        if matches!(outcome, ImportWriteOutcome::Imported) {
            discovered.url.clone()
        } else {
            outcome.value().to_string()
        },
    ));
    Ok(Some(discovered.url))
}

#[derive(Clone, Debug)]
pub(super) struct DiscoveredCiRunUrl {
    pub(super) url: String,
    pub(super) source: Option<Utf8PathBuf>,
    pub(super) had_failed_candidate: bool,
}

pub(super) fn auto_discover_ci_run_url_evidence(
    text_files: &[Utf8PathBuf],
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Option<DiscoveredCiRunUrl>, Box<dyn std::error::Error>> {
    let mut had_failed_candidate = false;
    for path in text_files {
        let Some(url) = read_ci_run_url_file(path)? else {
            continue;
        };
        if is_github_actions_run_url(&url) {
            return Ok(Some(DiscoveredCiRunUrl {
                url,
                source: Some(path.clone()),
                had_failed_candidate,
            }));
        }
        if ci_run_url_evidence_path(path) {
            had_failed_candidate = true;
            items.push(import_ci_item(
                "ci run url",
                "failed",
                Some(path),
                None,
                format!("invalid GitHub Actions run URL in CI run URL evidence: {url}"),
            ));
        }
    }
    Ok(had_failed_candidate.then_some(DiscoveredCiRunUrl {
        url: String::new(),
        source: None,
        had_failed_candidate: true,
    }))
}

pub(super) fn ci_run_url_evidence_path(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "ci-run-url.txt" | "ci_run_url.txt" | "ci-run-url.log" | "ci_run_url.log"
        )
    })
}

pub(super) fn read_required_ci_run_url_file(
    path: &Utf8Path,
) -> Result<String, Box<dyn std::error::Error>> {
    read_ci_run_url_file(path)?.ok_or_else(|| {
        format!("CI run URL file did not contain a valid GitHub Actions run URL: {path}").into()
    })
}

pub(super) fn validate_import_ci_explicit_run_urls_match(
    cli_url: &str,
    file_url: &str,
    file_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_explicit_ci_run_urls_match(
        cli_url,
        "--ci-run-url",
        file_url,
        &format!("--ci-run-url-file {file_path}"),
    )
}

pub(super) fn validate_explicit_ci_run_urls_match(
    left_url: &str,
    left_label: &str,
    right_url: &str,
    right_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let left_url = left_url.trim();
    let right_url = right_url.trim();
    let Some(left_run) = github_actions_run_key(left_url) else {
        return Err(
            format!("{left_label} is not a valid GitHub Actions run URL: {left_url}").into(),
        );
    };
    let Some(right_run) = github_actions_run_key(right_url) else {
        return Err(
            format!("{right_label} is not a valid GitHub Actions run URL: {right_url}").into(),
        );
    };
    if left_run == right_run {
        return Ok(());
    }
    Err(format!(
        "{left_label} ({}/{}, run {}) and {right_label} ({}/{}, run {}) refer to different GitHub Actions runs",
        left_run.owner,
        left_run.repo,
        left_run.run_id,
        right_run.owner,
        right_run.repo,
        right_run.run_id,
    )
    .into())
}

pub(super) fn import_protocol_snapshot_evidence(
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let candidates = collect_protocol_snapshot_dirs_recursive(&options.source)?;
    if candidates.is_empty() {
        items.push(import_ci_item(
            "protocol snapshot",
            "skipped",
            None,
            None,
            "no protocol snapshot directory found",
        ));
        return Ok(Vec::new());
    }

    let destination = options.dir.join("vesty-protocol");
    for candidate in &candidates {
        match check_protocol_export(candidate) {
            Ok(()) => {
                let outcome = import_copy_dir_contents(candidate, &destination, options.overwrite)?;
                items.push(import_ci_item(
                    "protocol snapshot",
                    outcome.status(),
                    Some(candidate.as_path()),
                    Some(destination.as_path()),
                    outcome.value(),
                ));
                return Ok(candidates);
            }
            Err(error) => items.push(import_ci_item(
                "protocol snapshot",
                "failed",
                Some(candidate.as_path()),
                None,
                error.to_string(),
            )),
        }
    }
    Ok(candidates)
}

pub(super) fn import_ci_json_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("CI JSON artifact", path)?;

    if let Ok(report) = serde_json::from_str::<ReleaseCheckReport>(&text) {
        return import_ci_release_check_artifact(path, report, options, expected_ci_run_url, items);
    }
    if let Ok(plan) = serde_json::from_str::<ReleaseActionPlan>(&text) {
        return import_ci_release_action_plan_artifact(path, &plan, options, items);
    }
    if let Ok(report) = serde_json::from_str::<PlatformSmokeReport>(&text) {
        return import_ci_platform_smoke_artifact(path, report, options, items);
    }
    if let Ok(surface) = serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsSurface>(&text) {
        return import_ci_vst3_sdk_binding_surface_artifact(path, &surface, options, items);
    }
    if let Ok(plan) = serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsPlan>(&text) {
        return import_ci_vst3_sdk_binding_plan_artifact(path, &plan, options, items);
    }
    if let Ok(manifest) = serde_json::from_str::<vesty_vst3_sys::SdkHeaderInputManifest>(&text) {
        return import_ci_vst3_sdk_manifest_artifact(path, &manifest, options, items);
    }
    if let Ok(report) = serde_json::from_str::<DoctorReport>(&text) {
        return import_ci_doctor_artifact(path, report, options, expected_ci_run_url, items);
    }
    if let Ok(report) = serde_json::from_str::<ValidateReport>(&text) {
        return import_ci_validate_artifact(path, report, options, items);
    }
    if serde_json::from_str::<PublishPlan>(&text).is_ok() {
        return import_ci_publish_plan_artifact(path, options, items);
    }
    if serde_json::from_str::<CratePackageReport>(&text).is_ok() {
        return import_ci_crate_package_artifact(path, options, items);
    }
    if serde_json::from_str::<DependencyBaselineReport>(&text).is_ok() {
        return import_ci_dependency_baseline_artifact(path, options, items);
    }
    if parse_npm_pack_report_text(&text).is_ok() {
        return import_ci_npm_pack_artifact(path, options, items);
    }
    if let Some((name, reason)) = recognized_json_artifact_name_from_path(path) {
        let parse_error = serde_json::from_str::<serde_json::Value>(&text)
            .err()
            .map(|error| format!("invalid JSON: {error}"))
            .unwrap_or_else(|| {
                "JSON did not match the expected release evidence schema".to_string()
            });
        items.push(import_ci_item(
            name,
            "failed",
            Some(path),
            None,
            format!("{reason}; {parse_error}"),
        ));
        return Ok(());
    }
    items.push(import_ci_item(
        "json artifact",
        "skipped",
        Some(path),
        None,
        "unrecognized JSON artifact",
    ));
    Ok(())
}

pub(super) fn recognized_json_artifact_name_from_path(
    path: &Utf8Path,
) -> Option<(&'static str, String)> {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name()?.to_ascii_lowercase();
    let stem_lower = path.file_stem().unwrap_or("").to_ascii_lowercase();

    let recognized = if file_lower.starts_with("doctor-")
        || path_lower.contains("/ci-doctor/")
        || path_lower.contains("/doctor-")
    {
        Some(("ci doctor artifact", "path looks like a CI doctor artifact"))
    } else if file_lower.starts_with("release-check-")
        || path_lower.contains("/ci-release-checks/")
        || path_lower.contains("/release-check-")
    {
        Some((
            "ci release-check artifact",
            "path looks like a CI release-check artifact",
        ))
    } else if file_lower.starts_with("release-action-plan-")
        || path_lower.contains("/release-action-plan-")
    {
        Some((
            "release action plan sidecar",
            "path looks like a release action plan sidecar",
        ))
    } else if file_lower.contains("platform-smoke") || path_lower.contains("/platform-smoke/") {
        Some((
            "platform smoke artifact",
            "path looks like a platform smoke artifact",
        ))
    } else if file_lower.contains("static-validate")
        || path_lower.contains("/static-validate")
        || path_lower.contains("/package/")
    {
        Some((
            "vst3 static validate report",
            "path looks like a VST3 static validate report",
        ))
    } else if file_lower.contains("validate") || path_lower.contains("/validator/") {
        Some((
            "vst3 validate report",
            "path looks like a VST3 validate report",
        ))
    } else if file_lower == "publish-plan.json" || path_lower.contains("/publish-plan/") {
        Some(("crate publish plan", "path looks like a crate publish plan"))
    } else if file_lower == "crate-package.json" || path_lower.contains("/crate-package/") {
        Some((
            "crate package readiness",
            "path looks like a crate package readiness report",
        ))
    } else if file_lower == "npm-pack.json" || path_lower.contains("/npm-pack/") {
        Some((
            "npm package pack report",
            "path looks like an npm package pack report",
        ))
    } else if file_lower == "dependency-baseline-latest.json"
        || path_lower.contains("/dependency-baseline/")
    {
        Some((
            "dependency latest baseline",
            "path looks like a dependency latest baseline report",
        ))
    } else if file_lower == "vst3-sdk-headers.json" || stem_lower.contains("vst3-sdk-headers") {
        Some((
            "vst3 SDK header manifest",
            "path looks like a VST3 SDK header manifest",
        ))
    } else if file_lower == "generated-bindings-plan.json"
        || stem_lower.contains("generated-bindings-plan")
    {
        Some((
            "vst3 SDK generated bindings plan",
            "path looks like a VST3 SDK generated bindings plan",
        ))
    } else if file_lower == "generated-bindings-surface.json"
        || stem_lower.contains("generated-bindings-surface")
    {
        Some((
            "vst3 SDK generated bindings surface",
            "path looks like a VST3 SDK generated bindings surface",
        ))
    } else if path_lower.contains("/vst3-sdk/")
        || path_lower.contains("/vesty-vst3-sdk/")
        || stem_lower.contains("vst3-sdk")
    {
        Some((
            "vst3 SDK artifact",
            "path looks like a VST3 SDK release artifact",
        ))
    } else {
        None
    }?;

    Some((recognized.0, recognized.1.to_string()))
}

pub(super) fn import_ci_rust_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("CI Rust artifact", path)?;
    if is_vst3_sdk_generated_bindings_abi_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_abi_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings ABI layout",
                    path,
                    &options.dir.join("vst3-sdk/generated-abi.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings ABI layout",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if is_vst3_sdk_generated_bindings_abi_seed_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_abi_seed_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings ABI seed",
                    path,
                    &options.dir.join("vst3-sdk/generated-abi-seed.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings ABI seed",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if is_vst3_sdk_generated_bindings_interface_skeleton_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_interface_skeleton_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings interface skeleton",
                    path,
                    &options.dir.join("vst3-sdk/generated-interface-skeleton.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings interface skeleton",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if !is_vst3_sdk_generated_bindings_scaffold_candidate(path, &text) {
        return Ok(());
    }

    match validate_vst3_sdk_generated_bindings_scaffold_text(&text) {
        Ok(()) => import_copy_file_item(
            "vst3 SDK generated bindings scaffold",
            path,
            &options.dir.join("vst3-sdk/generated.rs"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings scaffold",
                "failed",
                Some(path),
                None,
                error,
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_release_check_artifact(
    path: &Utf8Path,
    report: ReleaseCheckReport,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = artifact_os_from_path(path) else {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            "could not infer OS from artifact path",
        ));
        return Ok(());
    };
    if let Err(error) = validate_ci_release_check_report(&report) {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }
    if let Err(error) =
        validate_import_ci_run_match(report.ci_run_url.as_deref(), expected_ci_run_url)
    {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join(format!("ci-release-checks/release-check-{os}.json"));
    import_copy_file_item(
        "ci release-check artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

pub(super) fn import_ci_release_action_plan_artifact(
    path: &Utf8Path,
    plan: &ReleaseActionPlan,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = artifact_os_from_path(path) else {
        items.push(import_ci_item(
            "release action plan sidecar",
            "skipped",
            Some(path),
            None,
            "could not infer OS from action plan path; not a per-OS CI sidecar",
        ));
        return Ok(());
    };
    if let Err(error) = validate_release_action_plan_sidecar(plan) {
        items.push(import_ci_item(
            "release action plan sidecar",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join(format!("ci-release-checks/release-action-plan-{os}.json"));
    import_copy_file_item(
        "release action plan sidecar",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

pub(super) fn validate_release_action_plan_sidecar(plan: &ReleaseActionPlan) -> Result<(), String> {
    if plan.version != 1 {
        return Err(format!(
            "unsupported release action plan version {}",
            plan.version
        ));
    }
    if !matches!(plan.status.as_str(), "ok" | "failed") {
        return Err(format!(
            "unexpected release action plan status `{}`",
            plan.status
        ));
    }
    validate_release_action_text(
        "release action plan protocol snapshot",
        &plan.protocol_snapshot,
    )?;
    validate_release_action_safe_path(
        "release action plan protocol snapshot",
        &plan.protocol_snapshot,
    )?;
    if let Some(evidence_root) = &plan.evidence_root {
        validate_release_action_text("release action plan evidence root", evidence_root)?;
        validate_release_action_safe_path("release action plan evidence root", evidence_root)?;
    }
    if let Some(release_evidence_dir) = &plan.release_evidence_dir {
        validate_release_action_text(
            "release action plan release evidence dir",
            release_evidence_dir,
        )?;
        validate_release_action_safe_path(
            "release action plan release evidence dir",
            release_evidence_dir,
        )?;
    }
    if plan.summary.action_count != plan.actions.len() {
        return Err(format!(
            "action count mismatch: summary={} actual={}",
            plan.summary.action_count,
            plan.actions.len()
        ));
    }

    let failed = plan
        .actions
        .iter()
        .filter(|action| action.status == "failed")
        .count();
    let skipped = plan
        .actions
        .iter()
        .filter(|action| action.status == "skipped")
        .count();
    if plan.summary.failed != failed {
        return Err(format!(
            "failed count mismatch: summary={} actions={failed}",
            plan.summary.failed
        ));
    }
    if plan.summary.skipped != skipped {
        return Err(format!(
            "skipped count mismatch: summary={} actions={skipped}",
            plan.summary.skipped
        ));
    }
    let pending = failed + skipped;
    if plan.summary.action_count != pending {
        return Err(format!(
            "action pending count mismatch: action_count={} failed+skipped={pending}",
            plan.summary.action_count
        ));
    }
    let summary_checks = plan.summary.ok + plan.summary.failed + plan.summary.skipped;
    if summary_checks == 0 {
        return Err("release action plan summary must contain at least one check".to_string());
    }
    if summary_checks > RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS {
        return Err(format!(
            "release action plan summary check count {summary_checks} exceeds maximum {RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS}"
        ));
    }
    validate_release_action_plan_summary_check_count(summary_checks)?;
    if plan.status == "ok" && plan.summary.failed != 0 {
        return Err("ok release action plan must not contain failed actions".to_string());
    }
    if plan.status == "failed" && plan.actions.is_empty() {
        return Err("failed release action plan must contain at least one action".to_string());
    }
    if plan.status == "failed" && plan.summary.failed == 0 {
        return Err(
            "failed release action plan must contain at least one failed action".to_string(),
        );
    }

    let expected_checks = expected_release_check_names();
    let mut seen_actions = BTreeSet::new();
    for action in &plan.actions {
        validate_release_action_item(action)?;
        validate_release_action_item_evidence_path(plan, action)?;
        if !seen_actions.insert(action.check.as_str()) {
            return Err(format!("duplicate release action check `{}`", action.check));
        }
        if !expected_checks.contains(action.check.as_str()) {
            return Err(format!(
                "unknown release action check `{}`; action plan sidecar must use the current Vesty release gate",
                action.check
            ));
        }
    }
    Ok(())
}

pub(super) const RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS: usize = 128;
pub(super) const RELEASE_ACTION_MAX_COMMANDS: usize = 16;
pub(super) const RELEASE_ACTION_TEXT_MAX_BYTES: usize = 8 * 1024;
pub(super) const RELEASE_CHECK_MAX_CHECKS: usize = 128;

pub(super) fn validate_release_action_plan_summary_check_count(
    summary_checks: usize,
) -> Result<(), String> {
    let expected = expected_release_check_names().len();
    if summary_checks == expected {
        return Ok(());
    }

    Err(format!(
        "release action plan summary check count must match current Vesty release gate: summary={summary_checks} expected={expected}"
    ))
}

pub(super) fn validate_release_action_item(action: &ReleaseActionItem) -> Result<(), String> {
    validate_release_action_text("check name", &action.check)?;
    validate_release_action_text(
        &format!("release action `{}` value", action.check),
        &action.value,
    )?;
    if let Some(hint) = &action.hint {
        validate_release_action_text(&format!("release action `{}` hint", action.check), hint)?;
    }
    if let Some(evidence_path) = &action.evidence_path {
        validate_release_action_text(
            &format!("release action `{}` evidence path", action.check),
            evidence_path,
        )?;
        validate_release_action_safe_path(
            &format!("release action `{}` evidence path", action.check),
            evidence_path,
        )?;
    }
    if action.commands.is_empty() {
        return Err(format!(
            "release action `{}` has no suggested commands",
            action.check
        ));
    }
    if action.commands.len() > RELEASE_ACTION_MAX_COMMANDS {
        return Err(format!(
            "release action `{}` has too many suggested commands: {} exceeds maximum {RELEASE_ACTION_MAX_COMMANDS}",
            action.check,
            action.commands.len()
        ));
    }
    for (index, command) in action.commands.iter().enumerate() {
        let label = format!("release action `{}` command {index}", action.check);
        validate_release_action_text(&label, command)?;
        validate_release_action_command_syntax(&label, command)?;
    }
    match (action.status.as_str(), action.priority.as_str()) {
        ("failed", "required") | ("skipped", "optional") => Ok(()),
        ("failed", other) | ("skipped", other) => Err(format!(
            "release action `{}` status {} has invalid priority `{other}`",
            action.check, action.status
        )),
        (other, _) => Err(format!(
            "release action `{}` has unexpected status `{other}`",
            action.check
        )),
    }
}

pub(super) fn validate_release_action_item_evidence_path(
    plan: &ReleaseActionPlan,
    action: &ReleaseActionItem,
) -> Result<(), String> {
    let Some(expected) = expected_release_action_evidence_path(plan, &action.check) else {
        return Ok(());
    };
    match action.evidence_path.as_deref() {
        Some(actual) if release_report_paths_equal(actual, &expected) => Ok(()),
        Some(actual) => Err(format!(
            "release action `{}` evidence path `{actual}` does not match expected `{expected}`",
            action.check
        )),
        None => Err(format!(
            "release action `{}` is missing expected evidence path `{expected}`",
            action.check
        )),
    }
}

pub(super) fn validate_release_action_safe_path(label: &str, path: &str) -> Result<(), String> {
    validate_release_report_root_path(label, path).map_err(|error| error.to_string())
}

pub(super) fn expected_release_action_evidence_path(
    plan: &ReleaseActionPlan,
    check: &str,
) -> Option<String> {
    if let Some(host) = check.strip_prefix("daw smoke: ") {
        let evidence_root = plan
            .evidence_root
            .as_deref()
            .unwrap_or("target/daw-evidence");
        let host_dir = release_action_host_dir_name(host)?;
        return Some(format!("{evidence_root}/{host_dir}"));
    }
    if check == "daw matrix" {
        return Some(
            plan.evidence_root
                .clone()
                .unwrap_or_else(|| "target/daw-evidence".to_string()),
        );
    }
    if check == "protocol snapshot" {
        return Some(plan.protocol_snapshot.clone());
    }

    let release_evidence_dir = plan
        .release_evidence_dir
        .as_deref()
        .unwrap_or("target/release-evidence");
    let relative = match check {
        "ci run url" => "ci-run-url.txt",
        "ci doctor artifacts" => "ci-doctor",
        "ci release-check artifacts" => "ci-release-checks",
        "platform smoke artifacts" => "platform-smoke",
        "vst3 validate reports" | "vst3 example validator coverage" => "validator",
        "vst3 static validate reports" | "ci example static validate coverage" => "package",
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
        "signed bundle evidence" => {
            return Some(format!(
                "{release_evidence_dir}/signing-macos.log and {release_evidence_dir}/signing-windows.log"
            ));
        }
        "notarization log" => "notary.log",
        _ => return None,
    };
    Some(format!("{release_evidence_dir}/{relative}"))
}

pub(super) fn release_action_host_dir_name(host: &str) -> Option<&'static str> {
    match host {
        "REAPER" => Some("reaper"),
        "Cubase/Nuendo" => Some("cubase"),
        "Bitwig Studio" => Some("bitwig"),
        "Ableton Live" => Some("ableton"),
        "Studio One" => Some("studio-one"),
        _ => None,
    }
}

pub(super) fn validate_release_action_command_syntax(
    label: &str,
    command: &str,
) -> Result<(), String> {
    let command = command.trim();
    if !release_action_command_starts_with_vesty(command) {
        return Ok(());
    }
    let argv = split_release_action_command(command)?;
    Cli::try_parse_from(&argv)
        .map(|_| ())
        .map_err(|error| format!("{label} does not parse with current CLI: {error}"))
}

pub(super) fn release_action_command_starts_with_vesty(command: &str) -> bool {
    let Some(remainder) = command.strip_prefix("vesty") else {
        return false;
    };
    remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace)
}

pub(super) fn split_release_action_command(command: &str) -> Result<Vec<String>, String> {
    let mut args = vec!["vesty".to_string()];
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut chars = command.trim().chars().peekable();

    for _ in 0.."vesty".len() {
        chars.next();
    }

    for ch in chars {
        match ch {
            '"' => in_double_quotes = !in_double_quotes,
            '#' if !in_double_quotes => break,
            ch if ch.is_whitespace() && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_double_quotes {
        return Err("release action command contains an unterminated double quote".to_string());
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

pub(super) fn validate_release_action_text(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > RELEASE_ACTION_TEXT_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {RELEASE_ACTION_TEXT_MAX_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} must not contain control characters"));
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!(
            "{label} must not contain unsafe Unicode format characters"
        ));
    }
    Ok(())
}

pub(super) fn sanitize_release_report_text(value: impl Into<String>) -> String {
    let value = value.into();
    let mut out = String::new();
    let mut previous_space = false;
    for raw in value.chars() {
        let ch = if raw.is_control() || is_release_action_unsafe_format_char(raw) {
            ' '
        } else {
            raw
        };
        if ch.is_whitespace() {
            if !out.is_empty() && !previous_space {
                if out.len() + 1 > RELEASE_ACTION_TEXT_MAX_BYTES {
                    break;
                }
                out.push(' ');
                previous_space = true;
            }
            continue;
        }
        if out.len() + ch.len_utf8() > RELEASE_ACTION_TEXT_MAX_BYTES {
            break;
        }
        out.push(ch);
        previous_space = false;
    }

    let trimmed = out.trim();
    if trimmed.is_empty() {
        "<empty>".to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn is_release_action_unsafe_format_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF
    )
}

pub(super) fn import_ci_doctor_artifact(
    path: &Utf8Path,
    report: DoctorReport,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = validate_doctor_report(&report) {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }
    let os = match import_doctor_artifact_os(path, &report) {
        Ok(os) => os,
        Err(error) => {
            items.push(import_ci_item(
                "ci doctor artifact",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
    };
    let missing = missing_doctor_checks(os, &report);
    if !missing.is_empty() {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            format!("missing checks: {}", missing.join("; ")),
        ));
        return Ok(());
    }
    if let Err(error) =
        validate_import_ci_run_match(report.ci_run_url.as_deref(), expected_ci_run_url)
    {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options.dir.join(format!("ci-doctor/doctor-{os}.json"));
    import_copy_file_item(
        "ci doctor artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

pub(super) fn import_ci_validate_artifact(
    path: &Utf8Path,
    report: ValidateReport,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (name, destination) = if validate_release_validate_report(&report).is_ok() {
        if let Err(error) = validate_report_artifact_path_platform(path, &report) {
            items.push(import_ci_item(
                "vst3 validate report",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
        (
            "vst3 validate report",
            options
                .dir
                .join("validator")
                .join(validate_report_import_filename(&report, "validate")),
        )
    } else if validate_static_validate_report(&report).is_ok() {
        if let Err(error) = validate_report_artifact_path_platform(path, &report) {
            items.push(import_ci_item(
                "vst3 static validate report",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
        (
            "vst3 static validate report",
            options
                .dir
                .join("package")
                .join(validate_report_import_filename(&report, "static-validate")),
        )
    } else {
        let release_error = validate_release_validate_report(&report)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "not a validator-passed report".to_string());
        let static_error = validate_static_validate_report(&report)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "not a static validate report".to_string());
        items.push(import_ci_item(
            "vst3 validate report",
            "failed",
            Some(path),
            None,
            format!("{release_error}; {static_error}"),
        ));
        return Ok(());
    };

    import_copy_file_item(name, path, &destination, options.overwrite, items)
}

pub(super) fn import_ci_publish_plan_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_publish_plan_report(path) {
        Ok(_) => import_copy_file_item(
            "crate publish plan",
            path,
            &options.dir.join("publish-plan/publish-plan.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "crate publish plan",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_crate_package_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_crate_package_report_path(path) {
        Ok(_) => import_copy_file_item(
            "crate package readiness",
            path,
            &options.dir.join("crate-package/crate-package.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "crate package readiness",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_npm_pack_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_npm_pack_report(path) {
        Ok(_) => import_copy_file_item(
            "npm package pack report",
            path,
            &options.dir.join("npm-pack/npm-pack.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "npm package pack report",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_dependency_baseline_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_dependency_baseline_latest_report(path) {
        Ok(_) => import_copy_file_item(
            "dependency latest baseline",
            path,
            &options
                .dir
                .join("dependency-baseline/dependency-baseline-latest.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "dependency latest baseline",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_vst3_sdk_manifest_artifact(
    path: &Utf8Path,
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_header_manifest_content(manifest) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK header manifest",
            path,
            &options.dir.join("vst3-sdk/vst3-sdk-headers.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK header manifest",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_vst3_sdk_binding_plan_artifact(
    path: &Utf8Path,
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_binding_plan_content(plan) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK generated bindings plan",
            path,
            &options.dir.join("vst3-sdk/generated-bindings-plan.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings plan",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_vst3_sdk_binding_surface_artifact(
    path: &Utf8Path,
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_binding_surface_content(surface) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK generated bindings surface",
            path,
            &options.dir.join("vst3-sdk/generated-bindings-surface.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings surface",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

pub(super) fn import_ci_platform_smoke_artifact(
    path: &Utf8Path,
    report: PlatformSmokeReport,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if platform_smoke_report_is_pending_template(&report) {
        items.push(import_ci_item(
            "platform smoke artifact",
            "skipped",
            Some(path),
            None,
            "pending platform smoke template",
        ));
        return Ok(());
    }
    let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
        items.push(import_ci_item(
            "platform smoke artifact",
            "failed",
            Some(path),
            None,
            format!("unsupported platform `{}`", report.platform),
        ));
        return Ok(());
    };
    match platform_smoke_platform_from_artifact_path(path) {
        Ok(Some(path_platform)) if path_platform != platform => {
            items.push(import_ci_item(
                "platform smoke artifact",
                "failed",
                Some(path),
                None,
                format!(
                    "artifact path indicates {path_platform}, but report platform is {platform}"
                ),
            ));
            return Ok(());
        }
        Ok(_) => {}
        Err(error) => {
            items.push(import_ci_item(
                "platform smoke artifact",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
    }
    if let Err(error) = validate_platform_smoke_report(&report) {
        items.push(import_ci_item(
            "platform smoke artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }

    let destination = options.dir.join(format!("platform-smoke/{platform}.json"));
    import_copy_file_item(
        "platform smoke artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

pub(super) fn import_ci_text_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if read_ci_run_url_file(path)?.is_some_and(|url| is_github_actions_run_url(&url)) {
        return Ok(());
    }

    match signing_evidence_platforms(path) {
        Ok(platforms) => {
            if let Err(error) = validate_signing_artifact_path_platform(path, &platforms) {
                items.push(import_ci_item(
                    "signed bundle evidence",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
            let destination = signing_import_destination(&options.dir, path, &platforms);
            return import_copy_file_item(
                "signed bundle evidence",
                path,
                &destination,
                options.overwrite,
                items,
            );
        }
        Err(error) if signing_evidence_import_failure_is_actionable(path, &error.to_string()) => {
            items.push(import_ci_item(
                "signed bundle evidence",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
        Err(_) => {}
    }

    match notarization_evidence(path) {
        Ok(evidence) => {
            if let Err(error) = validate_notarization_artifact_path_platform(path) {
                items.push(import_ci_item(
                    "notarization log",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
            if evidence.accepted && evidence.stapled {
                return import_copy_file_item(
                    "notarization log",
                    path,
                    &options.dir.join("notary.log"),
                    options.overwrite,
                    items,
                );
            }
            items.push(import_ci_item(
                "notarization log",
                "failed",
                Some(path),
                None,
                "notarization evidence must include both accepted notarytool output and stapler success",
            ));
            return Ok(());
        }
        Err(error)
            if notarization_evidence_import_failure_is_actionable(path, &error.to_string()) =>
        {
            items.push(import_ci_item(
                "notarization log",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
        Err(_) => {}
    }

    items.push(import_ci_item(
        "text artifact",
        "skipped",
        Some(path),
        None,
        "unrecognized text artifact",
    ));
    Ok(())
}

pub(super) fn signing_evidence_import_failure_is_actionable(path: &Utf8Path, error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("negative signing evidence")
        || release_artifact_file_name_contains_any(
            path,
            &["codesign", "signtool", "signed", "signing", "signature"],
        )
}

pub(super) fn notarization_evidence_import_failure_is_actionable(
    path: &Utf8Path,
    error: &str,
) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("negative notarization evidence")
        || release_artifact_file_name_contains_any(
            path,
            &["notary", "notarytool", "notarization", "stapler", "staple"],
        )
}

pub(super) fn validate_notarization_artifact_path_platform(path: &Utf8Path) -> Result<(), String> {
    let Some(path_platform) = notarization_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    if path_platform == "macOS" {
        Ok(())
    } else {
        Err(format!(
            "artifact path indicates {path_platform}, but notarization evidence is macOS-only"
        ))
    }
}

pub(super) fn notarization_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    match notarization_file_name_platform(path)? {
        Some(platform) => Ok(Some(platform)),
        None => notarization_parent_dir_platform(path),
    }
}

pub(super) fn notarization_file_name_platform(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    let tokens = file_name_tokens(path);
    platform_label_from_tokens(&tokens, "notarization evidence file name")
}

pub(super) fn notarization_parent_dir_platform(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    let Some(parent) = path.parent().and_then(Utf8Path::file_name) else {
        return Ok(None);
    };
    let tokens = path_component_tokens(parent);
    platform_label_from_tokens(&tokens, "notarization evidence parent directory")
}

pub(super) fn platform_label_from_tokens(
    tokens: &[String],
    source: &str,
) -> Result<Option<&'static str>, String> {
    let mut found = Vec::new();
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        found.push("macOS");
    }
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "windows" | "windowsx64" | "win32" | "win64"))
    {
        found.push("Windows");
    }
    if tokens.iter().any(|token| token == "linux") {
        found.push("Linux");
    }
    match found.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "{source} contains multiple platform labels: {}",
            found.join(", ")
        )),
    }
}

pub(super) fn release_artifact_file_name_contains_any(path: &Utf8Path, needles: &[&str]) -> bool {
    let name = path.file_name().unwrap_or("").to_ascii_lowercase();
    needles.iter().any(|needle| name.contains(needle))
}

pub(super) fn validate_signing_artifact_path_platform(
    path: &Utf8Path,
    platforms: &BTreeSet<SigningEvidencePlatform>,
) -> Result<(), String> {
    let Some(path_platform) = signing_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    if platforms.contains(&path_platform) {
        Ok(())
    } else {
        Err(format!(
            "artifact path indicates {}, but signing evidence platform is {}",
            path_platform.label(),
            format_signing_platform_set(platforms)
        ))
    }
}

pub(super) fn signing_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<SigningEvidencePlatform>, String> {
    match signing_file_name_platform(path)? {
        Some(platform) => Ok(Some(platform)),
        None => Ok(signing_parent_dir_platform(path)),
    }
}

pub(super) fn signing_file_name_platform(
    path: &Utf8Path,
) -> Result<Option<SigningEvidencePlatform>, String> {
    let tokens = file_name_tokens(path);
    let mut found = [
        (
            SigningEvidencePlatform::Macos,
            &["macos", "mac", "darwin", "osx"][..],
        ),
        (
            SigningEvidencePlatform::Windows,
            &["windows", "windowsx64", "win64"][..],
        ),
    ]
    .into_iter()
    .filter_map(|(platform, aliases)| {
        tokens
            .iter()
            .any(|token| aliases.contains(&token.as_str()))
            .then_some(platform)
    })
    .collect::<Vec<_>>();
    found.sort_unstable();
    found.dedup();
    match found.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "signing evidence file name contains multiple platform labels: {}",
            format_signing_platform_list(&found)
        )),
    }
}

pub(super) fn signing_parent_dir_platform(path: &Utf8Path) -> Option<SigningEvidencePlatform> {
    let parent = path.parent()?.file_name()?;
    let tokens = path_component_tokens(parent);
    match tokens
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["macos"] | ["mac"] | ["darwin"] | ["osx"] => Some(SigningEvidencePlatform::Macos),
        ["windows"] | ["windows", "x64"] | ["windowsx64"] | ["win64"] => {
            Some(SigningEvidencePlatform::Windows)
        }
        _ => None,
    }
}

pub(super) fn format_signing_platform_set(platforms: &BTreeSet<SigningEvidencePlatform>) -> String {
    if platforms.is_empty() {
        "unknown".to_string()
    } else {
        format_signing_platform_list(&platforms.iter().copied().collect::<Vec<_>>())
    }
}

pub(super) fn format_signing_platform_list(platforms: &[SigningEvidencePlatform]) -> String {
    platforms
        .iter()
        .map(|platform| platform.label())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn import_ci_signed_bundle_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let platforms = match signing_evidence_platforms(path) {
        Ok(platforms) => platforms,
        Err(error) => {
            items.push(import_ci_item(
                "signed bundle evidence",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
    };
    if let Err(error) = validate_signing_artifact_path_platform(path, &platforms) {
        items.push(import_ci_item(
            "signed bundle evidence",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join("signed-bundles")
        .join(path.file_name().unwrap_or("signed.vst3"));
    match import_copy_dir_contents(path, &destination, options.overwrite)? {
        ImportWriteOutcome::Imported => items.push(import_ci_item(
            "signed bundle evidence",
            "imported",
            Some(path),
            Some(destination.as_path()),
            "copied signed macOS .vst3 bundle",
        )),
        ImportWriteOutcome::SkippedExisting => items.push(import_ci_item(
            "signed bundle evidence",
            "skipped",
            Some(path),
            Some(destination.as_path()),
            ImportWriteOutcome::SkippedExisting.value(),
        )),
    }
    Ok(())
}

pub(super) fn import_copy_file_item(
    name: &str,
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = import_copy_file(source, destination, overwrite)?;
    items.push(import_ci_item(
        name,
        outcome.status(),
        Some(source),
        Some(destination),
        outcome.value(),
    ));
    Ok(())
}

pub(super) fn import_copy_file(
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("CI artifact import refuses symlink source: {source}").into());
    }
    if !metadata.is_file() {
        return Err(format!("CI artifact import source is not a file: {source}").into());
    }
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    if let Some(parent) = destination.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(ImportWriteOutcome::Imported)
}

pub(super) fn import_write_text_file(
    destination: &Utf8Path,
    text: &str,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    import_write_bytes_file(destination, text.as_bytes(), overwrite)
}

pub(super) fn import_write_ci_run_url_file(
    destination: &Utf8Path,
    url: &str,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite && read_ci_run_url_file(destination)?.is_some() {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    import_write_text_file(destination, &format!("ci_run_url={url}\n"), true)
}

pub(super) fn import_write_bytes_file(
    destination: &Utf8Path,
    bytes: &[u8],
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    if let Some(parent) = destination.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, bytes)?;
    Ok(ImportWriteOutcome::Imported)
}

pub(super) fn import_copy_dir_contents(
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    fs::create_dir_all(destination)?;
    for (relative, bytes) in collect_relative_files(source)? {
        let target = destination.join(&relative);
        import_write_bytes_file(&target, &bytes, true)?;
    }
    Ok(ImportWriteOutcome::Imported)
}

pub(super) fn remove_existing_path(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub(super) fn path_exists_no_follow(path: &Utf8Path) -> Result<bool, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn import_ci_item(
    name: &str,
    status: &str,
    source: Option<&Utf8Path>,
    path: Option<&Utf8Path>,
    value: impl Into<String>,
) -> ImportCiReleaseEvidenceItem {
    ImportCiReleaseEvidenceItem {
        name: name.to_string(),
        status: status.to_string(),
        source: source.map(portable_report_path),
        path: path.map(portable_report_path),
        value: sanitize_release_report_text(value),
    }
}
