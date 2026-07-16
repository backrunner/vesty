use super::*;

pub(super) fn parse_bundle_platform(
    platform: &str,
) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    match platform.trim().to_ascii_lowercase().as_str() {
        "macos" | "darwin" => Ok(BundlePlatform::Macos),
        "windows-x64" | "windows" | "win64" => Ok(BundlePlatform::WindowsX64),
        "linux-x64" | "linux" => Ok(BundlePlatform::LinuxX64),
        _ => Err(format!("unsupported platform '{platform}'").into()),
    }
}

pub(super) fn infer_signing_bundle_platform(
    bundle: &Utf8Path,
) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    if bundle.extension() != Some("vst3") {
        return Err(format!("signing evidence source must be a .vst3 directory: {bundle}").into());
    }
    require_existing_directory_no_symlink("signing evidence source", bundle)?;

    let has_macos = existing_directory_no_symlink(
        "macOS signing payload directory",
        &bundle.join("Contents/MacOS"),
    )? || existing_directory_no_symlink(
        "macOS signing payload directory",
        &bundle.join("Contents/_CodeSignature"),
    )?;
    let has_windows = existing_directory_no_symlink(
        "Windows signing payload directory",
        &bundle.join("Contents/x86_64-win"),
    )?;
    match (has_macos, has_windows) {
        (true, false) => Ok(BundlePlatform::Macos),
        (false, true) => Ok(BundlePlatform::WindowsX64),
        (true, true) => Err(
            "bundle contains both macOS and Windows payloads; pass --platform explicitly".into(),
        ),
        (false, false) => match current_bundle_platform() {
            Some(BundlePlatform::Macos) => Ok(BundlePlatform::Macos),
            Some(BundlePlatform::WindowsX64) => Ok(BundlePlatform::WindowsX64),
            _ => Err(
                "could not infer signing platform from bundle contents; pass --platform macos or windows-x64"
                    .into(),
            ),
        },
    }
}

pub(super) fn signing_platform_for_bundle_platform(
    platform: BundlePlatform,
) -> Result<SigningEvidencePlatform, Box<dyn std::error::Error>> {
    match platform {
        BundlePlatform::Macos => Ok(SigningEvidencePlatform::Macos),
        BundlePlatform::WindowsX64 => Ok(SigningEvidencePlatform::Windows),
        BundlePlatform::LinuxX64 => Err(
            "Linux VST3 signing evidence is release-channel specific; collect distro/package signing evidence outside `vesty release-evidence collect-signing`"
                .into(),
        ),
    }
}

pub(super) fn default_signing_evidence_path(
    dir: &Utf8Path,
    platform: BundlePlatform,
) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => dir.join("signing-macos.log"),
        BundlePlatform::WindowsX64 => dir.join("signing-windows.log"),
        BundlePlatform::LinuxX64 => dir.join("signing-linux.log"),
    }
}

pub(super) fn current_bundle_platform() -> Option<BundlePlatform> {
    if cfg!(target_os = "macos") {
        Some(BundlePlatform::Macos)
    } else if cfg!(target_os = "windows") {
        Some(BundlePlatform::WindowsX64)
    } else if cfg!(target_os = "linux") {
        Some(BundlePlatform::LinuxX64)
    } else {
        None
    }
}

pub(super) fn resolve_bundle_platform(
    platform: Option<&str>,
) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    match platform {
        Some(platform) => parse_bundle_platform(platform),
        None => current_bundle_platform()
            .ok_or_else(|| "unsupported host OS; pass --platform explicitly".into()),
    }
}

pub(super) fn build_release_mode(
    debug: bool,
    release: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    if debug && release {
        return Err("use only one of --debug or --release".into());
    }
    Ok(!debug)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ValidateReport {
    pub(super) bundle: String,
    pub(super) static_check: StaticBundleCheck,
    pub(super) validator: ValidatorCheck,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StaticBundleCheck {
    pub(super) status: String,
    pub(super) moduleinfo: Option<String>,
    pub(super) binaries: Vec<String>,
    #[serde(default)]
    pub(super) binary_exports: Vec<BinaryExportCheck>,
    #[serde(default)]
    pub(super) parameter_manifest: Option<String>,
    pub(super) asset_manifest: Option<String>,
    pub(super) asset_count: usize,
    pub(super) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ValidatorCheck {
    pub(super) status: String,
    pub(super) path: Option<String>,
    pub(super) exit_code: Option<i32>,
    pub(super) tests_passed: Option<u32>,
    pub(super) tests_failed: Option<u32>,
    pub(super) stdout: Option<String>,
    pub(super) stderr: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) error: Option<String>,
}

impl StaticBundleCheck {
    pub(super) fn passed(report: &BundleValidationReport) -> Self {
        Self {
            status: "ok".to_string(),
            moduleinfo: Some(report.moduleinfo_path.to_string()),
            binaries: report
                .binary_paths
                .iter()
                .map(ToString::to_string)
                .collect(),
            binary_exports: report.binary_export_checks.clone(),
            parameter_manifest: report
                .parameter_manifest_path
                .as_ref()
                .map(ToString::to_string),
            asset_manifest: report.asset_manifest_path.as_ref().map(ToString::to_string),
            asset_count: report.asset_count,
            error: None,
        }
    }

    pub(super) fn failed(error: impl ToString) -> Self {
        Self {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(error.to_string()),
        }
    }
}

impl ValidatorCheck {
    pub(super) fn not_run(reason: impl ToString) -> Self {
        Self {
            status: "not_run".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some(reason.to_string()),
            error: None,
        }
    }

    pub(super) fn skipped(reason: impl ToString) -> Self {
        Self {
            status: "skipped".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some(reason.to_string()),
            error: None,
        }
    }

    pub(super) fn not_found() -> Self {
        Self {
            status: "not_found".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some("pass --validator or set VST3_VALIDATOR".to_string()),
            error: None,
        }
    }

    pub(super) fn process_error(path: &Utf8Path, error: impl ToString) -> Self {
        Self {
            status: "failed".to_string(),
            path: Some(path.to_string()),
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: None,
            error: Some(error.to_string()),
        }
    }
}

pub(super) fn run_validate(
    bundle: Utf8PathBuf,
    validator: Option<Utf8PathBuf>,
    static_only: bool,
    strict: bool,
    format: &str,
    report_path: Option<Utf8PathBuf>,
    validator_log_path: Option<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if format == OutputFormat::Text {
        println!("validating bundle: {bundle}");
    }

    let bundle_report = match validate_vst3_bundle(&bundle) {
        Ok(report) => report,
        Err(error) => {
            if format == OutputFormat::Json {
                let report = ValidateReport {
                    bundle: bundle.to_string(),
                    static_check: StaticBundleCheck::failed(error.to_string()),
                    validator: ValidatorCheck::not_run("static bundle validation failed"),
                };
                write_validate_report(report_path.as_deref(), &report)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let report = ValidateReport {
                    bundle: bundle.to_string(),
                    static_check: StaticBundleCheck::failed(error.to_string()),
                    validator: ValidatorCheck::not_run("static bundle validation failed"),
                };
                write_validate_report(report_path.as_deref(), &report)?;
            }
            return Err(Box::new(error));
        }
    };

    let static_check = StaticBundleCheck::passed(&bundle_report);
    if format == OutputFormat::Text {
        println!(
            "bundle structure: ok (binaries: {}, binary export checks: {}, ui assets: {})",
            bundle_report.binary_paths.len(),
            bundle_report.binary_export_checks.len(),
            bundle_report.asset_count
        );
    }

    if static_only {
        let validator_check = ValidatorCheck::skipped("--static-only");
        let strict_error = strict
            .then(|| strict_static_bundle_check_error(&static_check))
            .flatten();
        let report = ValidateReport {
            bundle: bundle.to_string(),
            static_check,
            validator: validator_check,
        };
        print_validate_report(format, &report, report_path.as_deref())?;
        if let Some(error) = strict_error {
            return Err(error.into());
        }
        return Ok(());
    }

    let Some(validator) = discover_validator(validator) else {
        let report = ValidateReport {
            bundle: bundle.to_string(),
            static_check,
            validator: ValidatorCheck::not_found(),
        };
        print_validate_report(format, &report, report_path.as_deref())?;
        return Err("validator not configured; pass --validator or set VST3_VALIDATOR".into());
    };

    if format == OutputFormat::Text {
        println!("validator: {validator}");
    }

    let validator_check = match Command::new(validator.as_std_path()).arg(&bundle).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let summary =
                validator_test_summary(&stdout).or_else(|| validator_test_summary(&stderr));
            if format == OutputFormat::Text {
                print_child_output(&stdout, &stderr);
            }
            write_validator_log(
                validator_log_path.as_deref(),
                &validator,
                &bundle,
                &stdout,
                &stderr,
            )?;
            ValidatorCheck {
                status: if output.status.success() {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
                path: Some(validator.to_string()),
                exit_code: output.status.code(),
                tests_passed: summary.map(|(passed, _)| passed),
                tests_failed: summary.map(|(_, failed)| failed),
                stdout: (!stdout.is_empty()).then_some(stdout),
                stderr: (!stderr.is_empty()).then_some(stderr),
                reason: None,
                error: None,
            }
        }
        Err(error) => {
            if let Some(path) = validator_log_path.as_deref() {
                write_text_file(
                    path,
                    &format!("validator={validator}\nbundle={bundle}\nerror={error}\n"),
                )?;
            }
            ValidatorCheck::process_error(&validator, error)
        }
    };

    let validator_passed = validator_check.status == "passed";
    let process_error = validator_check.error.clone();
    let report = ValidateReport {
        bundle: bundle.to_string(),
        static_check,
        validator: validator_check,
    };
    print_validate_report(format, &report, report_path.as_deref())?;
    if !validator_passed {
        if let Some(error) = process_error {
            return Err(format!("VST3 validator failed to run: {error}").into());
        }
        return Err("VST3 validator failed".into());
    }
    if let Some(error) = strict
        .then(|| strict_static_bundle_check_error(&report.static_check))
        .flatten()
    {
        return Err(error.into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ParamManifestReport {
    pub(super) status: String,
    pub(super) specs: String,
    pub(super) manifest: Option<String>,
    pub(super) parameters: usize,
    pub(super) id_algorithm: String,
    pub(super) check: bool,
}

pub(super) fn run_param_manifest(
    specs: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let specs_text = read_text_file_no_symlink("parameter specs", &specs)?;
    let manifest = parameter_manifest_from_specs_json(&specs_text)?;
    let out_path = out.as_deref();

    if check {
        let Some(out_path) = out_path else {
            return Err("--check requires --out <manifest.json>".into());
        };
        let existing = read_parameter_manifest(out_path)?;
        if existing != manifest {
            return Err(format!(
                "parameter manifest is out of date: {out_path}; regenerate from {specs}"
            )
            .into());
        }
        print_param_manifest_report(
            format,
            &manifest,
            &ParamManifestReport {
                status: "ok".to_string(),
                specs: specs.to_string(),
                manifest: Some(out_path.to_string()),
                parameters: manifest.parameters.len(),
                id_algorithm: manifest.id_algorithm.clone(),
                check: true,
            },
        )?;
        return Ok(());
    }

    let report = ParamManifestReport {
        status: "generated".to_string(),
        specs: specs.to_string(),
        manifest: out_path.map(ToString::to_string),
        parameters: manifest.parameters.len(),
        id_algorithm: manifest.id_algorithm.clone(),
        check: false,
    };
    if let Some(out_path) = out_path {
        write_parameter_manifest(out_path, &manifest)?;
        print_param_manifest_report(format, &manifest, &report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    }
    Ok(())
}

pub(super) fn write_parameter_manifest(
    path: &Utf8Path,
    manifest: &ParameterManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    write_text_file(path, &(serde_json::to_string_pretty(manifest)? + "\n"))
}

pub(super) fn print_param_manifest_report(
    format: OutputFormat,
    manifest: &ParameterManifest,
    report: &ParamManifestReport,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Text => match report.status.as_str() {
            "ok" => println!(
                "parameter manifest: ok ({} params, {})",
                report.parameters, report.id_algorithm
            ),
            _ => {
                if let Some(path) = &report.manifest {
                    println!(
                        "parameter manifest: {path} ({} params, {})",
                        report.parameters, report.id_algorithm
                    );
                } else {
                    println!("{}", serde_json::to_string_pretty(manifest)?);
                }
            }
        },
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
    }
    Ok(())
}

pub(super) fn print_validate_report(
    format: OutputFormat,
    report: &ValidateReport,
    report_path: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_shape(report)?;
    write_validate_report(report_path, report)?;
    match format {
        OutputFormat::Text => match report.validator.status.as_str() {
            "skipped" => println!(
                "validator: skipped ({})",
                report.validator.reason.as_deref().unwrap_or("requested")
            ),
            "not_found" => println!(
                "validator: not found ({})",
                report
                    .validator
                    .reason
                    .as_deref()
                    .unwrap_or("configure validator")
            ),
            "failed" if report.validator.error.is_some() => println!(
                "validator: failed to run ({})",
                report.validator.error.as_deref().unwrap_or("")
            ),
            _ => {}
        },
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
    }
    Ok(())
}

pub(super) fn write_validate_report(
    path: Option<&Utf8Path>,
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else {
        return Ok(());
    };
    validate_validate_report_shape(report)?;
    let text = serde_json::to_string_pretty(report)?;
    write_text_file(path, &(text + "\n"))
}

pub(super) fn write_validator_log(
    path: Option<&Utf8Path>,
    validator: &Utf8Path,
    bundle: &Utf8Path,
    stdout: &str,
    stderr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut text = format!("validator={validator}\nbundle={bundle}\n");
    if !stdout.is_empty() {
        text.push_str("\n[stdout]\n");
        text.push_str(stdout);
        if !stdout.ends_with('\n') {
            text.push('\n');
        }
    }
    if !stderr.is_empty() {
        text.push_str("\n[stderr]\n");
        text.push_str(stderr);
        if !stderr.ends_with('\n') {
            text.push('\n');
        }
    }
    write_text_file(path, &text)
}

pub(super) fn write_text_file(
    path: &Utf8Path,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reject_existing_path_symlink("output file", path)?;
    reject_existing_output_parent_symlink("output file", path)?;
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

pub(super) fn reject_existing_output_parent_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    for ancestor in parent.ancestors() {
        if ancestor.as_str().is_empty() {
            continue;
        }
        // macOS temp paths commonly pass through root-owned /var or /tmp symlinks.
        if ancestor.is_absolute() && ancestor.components().count() <= 2 {
            continue;
        }
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(format!("{label} parent must not be a symlink: {ancestor}").into());
        }
        if !metadata.is_dir() {
            return Err(format!("{label} parent must be a directory: {ancestor}").into());
        }
    }
    Ok(())
}

pub(super) fn print_child_output(stdout: &str, stderr: &str) {
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
}

pub(super) fn validator_test_summary(text: &str) -> Option<(u32, u32)> {
    let mut passed = None;
    let mut failed = None;
    for line in text.lines() {
        let line = line.to_ascii_lowercase();
        if passed.is_none() {
            passed = extract_count_near_any_marker(&line, &["tests passed", "test passed"]);
        }
        if failed.is_none() {
            failed = extract_count_near_any_marker(&line, &["tests failed", "test failed"]);
        }
        if let (Some(passed), Some(failed)) = (passed, failed) {
            return Some((passed, failed));
        }
    }
    None
}

pub(super) fn extract_count_near_any_marker(line: &str, markers: &[&str]) -> Option<u32> {
    markers
        .iter()
        .find_map(|marker| extract_count_near_marker(line, marker))
}

pub(super) fn extract_count_near_marker(line: &str, marker: &str) -> Option<u32> {
    let index = line.find(marker)?;
    last_number_before(&line[..index]).or_else(|| first_number_after(&line[index + marker.len()..]))
}

pub(super) fn last_number_before(text: &str) -> Option<u32> {
    let end = text.rfind(|char: char| char.is_ascii_digit())?;
    let start = text[..=end]
        .rfind(|char: char| !char.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    text[start..=end].parse().ok()
}

pub(super) fn first_number_after(text: &str) -> Option<u32> {
    let start = text.find(|char: char| char.is_ascii_digit())?;
    let end = text[start..]
        .find(|char: char| !char.is_ascii_digit())
        .map(|index| start + index)
        .unwrap_or(text.len());
    text[start..end].parse().ok()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DirectoryDiff {
    pub(super) missing: Vec<String>,
    pub(super) changed: Vec<String>,
    pub(super) extra: Vec<String>,
}

impl DirectoryDiff {
    pub(super) const MAX_REPORTED_PATHS_PER_KIND: usize = 12;

    pub(super) fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.changed.is_empty() && self.extra.is_empty()
    }

    pub(super) fn summary(&self) -> String {
        format!(
            "{} missing, {} changed, {} extra",
            self.missing.len(),
            self.changed.len(),
            self.extra.len()
        )
    }

    pub(super) fn summary_with_paths(&self) -> String {
        let details = [
            Self::format_paths("missing", &self.missing),
            Self::format_paths("changed", &self.changed),
            Self::format_paths("extra", &self.extra),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if details.is_empty() {
            self.summary()
        } else {
            format!("{} ({})", self.summary(), details.join("; "))
        }
    }

    pub(super) fn format_paths(kind: &str, paths: &[String]) -> Option<String> {
        if paths.is_empty() {
            return None;
        }

        let shown = paths
            .iter()
            .take(Self::MAX_REPORTED_PATHS_PER_KIND)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if paths.len() > Self::MAX_REPORTED_PATHS_PER_KIND {
            Some(format!(
                "{kind}: {shown}, ... (+{} more)",
                paths.len() - Self::MAX_REPORTED_PATHS_PER_KIND
            ))
        } else {
            Some(format!("{kind}: {shown}"))
        }
    }
}

pub(super) fn check_protocol_export(out: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    if !out.is_dir() {
        return Err(format!("protocol snapshot directory does not exist: {out}").into());
    }

    let temp_dir = unique_temp_dir("vesty-protocol-check")?;
    let result = (|| -> Result<DirectoryDiff, Box<dyn std::error::Error>> {
        vesty_ipc::export_protocol_bindings(&temp_dir)?;
        diff_directories(out, &temp_dir)
    })();
    let _ = fs::remove_dir_all(&temp_dir);

    let diff = result?;
    if diff.is_empty() {
        return Ok(());
    }

    for missing in &diff.missing {
        eprintln!("protocol missing: {missing}");
    }
    for changed in &diff.changed {
        eprintln!("protocol changed: {changed}");
    }
    for extra in &diff.extra {
        eprintln!("protocol extra: {extra}");
    }
    Err(format!(
        "protocol export drift detected: {}",
        diff.summary_with_paths()
    )
    .into())
}

pub(super) fn unique_temp_dir(prefix: &str) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir(&path)?;
    Utf8PathBuf::from_path_buf(path)
        .map_err(|_| "temporary protocol export path is not valid utf-8".into())
}

pub(super) fn diff_directories(
    expected_dir: &Utf8Path,
    actual_dir: &Utf8Path,
) -> Result<DirectoryDiff, Box<dyn std::error::Error>> {
    let expected = collect_relative_files(expected_dir)?;
    let actual = collect_relative_files(actual_dir)?;
    let mut diff = DirectoryDiff::default();

    for path in expected.keys() {
        if !actual.contains_key(path) {
            diff.missing.push(path.clone());
        }
    }
    for (path, actual_bytes) in &actual {
        match expected.get(path) {
            Some(expected_bytes) if expected_bytes != actual_bytes => {
                diff.changed.push(path.clone())
            }
            Some(_) => {}
            None => diff.extra.push(path.clone()),
        }
    }
    Ok(diff)
}

pub(super) fn collect_relative_files(
    root: &Utf8Path,
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut files = BTreeMap::new();
    collect_relative_files_inner(root, root, &mut files)?;
    Ok(files)
}

pub(super) fn collect_relative_files_inner(
    root: &Utf8Path,
    current: &Utf8Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "protocol export path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("protocol export contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_relative_files_inner(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "protocol export path escaped root")?
                .as_str()
                .replace('\\', "/");
            files.insert(relative, fs::read(&path)?);
        }
    }
    Ok(())
}
