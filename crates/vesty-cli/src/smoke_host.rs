use super::*;

pub(super) const SMOKE_HOST_GENERATOR: &str = "vesty-cli.smoke-host.v1";
pub(super) const SMOKE_HOST_EXAMPLES: [(&str, &str, bool); 3] = [
    ("gain", "Vesty Gain", false),
    ("midi-synth", "Vesty MIDI Synth", false),
    ("web-ui-param-demo", "Vesty Web UI Demo", true),
];
pub(super) const SMOKE_HOST_MAX_CHECKS: usize = 64;

pub(super) fn expected_smoke_host_check_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.insert(normalize_smoke_host_check_name("workspace manifest"));
    for (example, _, has_ui) in SMOKE_HOST_EXAMPLES {
        names.insert(normalize_smoke_host_check_name(&format!(
            "{example} config"
        )));
        names.insert(normalize_smoke_host_check_name(&format!(
            "{example} parameter sidecar"
        )));
        if has_ui {
            names.insert(normalize_smoke_host_check_name(&format!(
                "{example} UI assets"
            )));
        }
    }
    names.insert(normalize_smoke_host_check_name("JSBridge trace"));
    names.insert(normalize_smoke_host_check_name("meter stream"));
    names
}

#[derive(Clone, Debug)]
pub(super) struct SmokeHostOptions {
    pub(super) workspace: Utf8PathBuf,
    pub(super) bridge_trace: Option<Utf8PathBuf>,
    pub(super) meter_log: Option<Utf8PathBuf>,
    pub(super) out: Option<Utf8PathBuf>,
    pub(super) check: bool,
    pub(super) strict: bool,
    pub(super) format: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SmokeHostReport {
    pub(super) version: u32,
    pub(super) generator: String,
    pub(super) workspace: String,
    pub(super) status: String,
    pub(super) checks: Vec<SmokeHostCheck>,
    pub(super) external_evidence_note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SmokeHostCheck {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) hint: Option<String>,
}

pub(super) fn run_smoke_host(options: SmokeHostOptions) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    let report = build_smoke_host_report(
        &options.workspace,
        options.bridge_trace.as_deref(),
        options.meter_log.as_deref(),
    );
    validate_smoke_host_report(&report)?;

    if options.check {
        let out = options
            .out
            .as_deref()
            .ok_or("--check requires --out <smoke-host.json>")?;
        let text = read_text_file_no_symlink("smoke-host report", out)?;
        let expected: SmokeHostReport = serde_json::from_str(&text)?;
        validate_smoke_host_report(&expected)?;
        if expected != report {
            return Err(format!("smoke-host report is out of date: {out}").into());
        }
    } else if let Some(out) = options.out.as_deref() {
        write_text_file(out, &(serde_json::to_string_pretty(&report)? + "\n"))?;
    }

    print_smoke_host_report(&report, format, options.out.as_deref())?;
    if options.strict && !smoke_host_report_all_ok(&report) {
        return Err("smoke-host checks are incomplete; fix failed/skipped local checks".into());
    }
    Ok(())
}

pub(super) fn build_smoke_host_report(
    workspace: &Utf8Path,
    bridge_trace: Option<&Utf8Path>,
    meter_log: Option<&Utf8Path>,
) -> SmokeHostReport {
    let workspace_root = canonicalize_utf8_or_original(workspace);
    let mut checks = vec![smoke_host_workspace_check(&workspace_root)];
    for (example, expected_name, has_ui) in SMOKE_HOST_EXAMPLES {
        checks.extend(smoke_host_example_checks(
            &workspace_root,
            example,
            expected_name,
            has_ui,
        ));
    }
    checks.push(smoke_host_bridge_trace_check(bridge_trace));
    checks.push(smoke_host_meter_log_check(meter_log));

    let status = if checks.iter().any(|check| check.status == "failed") {
        "failed"
    } else if checks.iter().any(|check| check.status == "skipped") {
        "partial"
    } else {
        "ok"
    };

    SmokeHostReport {
        version: 1,
        generator: SMOKE_HOST_GENERATOR.to_string(),
        workspace: workspace_root.to_string(),
        status: status.to_string(),
        checks,
        external_evidence_note: "Vesty smoke-host is a local headless framework self-check. It does not load plugin binaries and does not replace real DAW, platform WebView, Steinberg validator, signing or notarization evidence.".to_string(),
    }
}

pub(super) fn canonicalize_utf8_or_original(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize()
        .ok()
        .and_then(|path| Utf8PathBuf::from_path_buf(path).ok())
        .unwrap_or_else(|| path.to_path_buf())
}

pub(super) fn smoke_host_workspace_check(workspace: &Utf8Path) -> SmokeHostCheck {
    let manifest = workspace.join("Cargo.toml");
    if manifest.is_file() {
        smoke_host_ok(
            "workspace manifest",
            format!("workspace Cargo.toml found at {manifest}"),
        )
    } else {
        smoke_host_failed(
            "workspace manifest",
            format!("missing workspace Cargo.toml at {manifest}"),
            "run smoke-host from the Vesty workspace root or pass --workspace <path>",
        )
    }
}

pub(super) fn smoke_host_example_checks(
    workspace: &Utf8Path,
    example: &str,
    expected_name: &str,
    has_ui: bool,
) -> Vec<SmokeHostCheck> {
    let example_dir = workspace.join("examples").join(example);
    let config_path = example_dir.join("vesty.toml");
    let specs_path = example_dir.join("params.specs.json");
    let manifest_path = example_dir.join("vesty-parameters.json");
    let mut checks = Vec::new();

    let config = match read_config(&config_path) {
        Ok(config) => {
            let mut failures = Vec::new();
            if config.plugin.name != expected_name {
                failures.push(format!(
                    "expected plugin name `{expected_name}`, got `{}`",
                    config.plugin.name
                ));
            }
            if config
                .package
                .as_ref()
                .and_then(|package| package.parameter_manifest.as_deref())
                != Some("vesty-parameters.json")
            {
                failures.push(
                    "missing [package].parameter_manifest = \"vesty-parameters.json\"".to_string(),
                );
            }
            if has_ui && config.ui.is_none() {
                failures.push("missing [ui] section".to_string());
            }
            if failures.is_empty() {
                checks.push(smoke_host_ok(
                    format!("{example} config"),
                    format!(
                        "{} v{} ({})",
                        config.plugin.name, config.plugin.version, config.plugin.kind
                    ),
                ));
            } else {
                checks.push(smoke_host_failed(
                    format!("{example} config"),
                    failures.join("; "),
                    "fix example vesty.toml metadata",
                ));
            }
            Some(config)
        }
        Err(error) => {
            checks.push(smoke_host_failed(
                format!("{example} config"),
                format!("{config_path}: {error}"),
                "add a valid vesty.toml for the example",
            ));
            None
        }
    };

    match smoke_host_parameter_sidecar_value(&specs_path, &manifest_path) {
        Ok(value) => checks.push(smoke_host_ok(format!("{example} parameter sidecar"), value)),
        Err(error) => checks.push(smoke_host_failed(
            format!("{example} parameter sidecar"),
            error,
            "run `vesty param-manifest --specs params.specs.json --out vesty-parameters.json` in the example",
        )),
    }

    if has_ui {
        let ui_check = config
            .as_ref()
            .and_then(|config| config.ui.as_ref())
            .map(|ui| smoke_host_ui_assets_check(&example_dir, example, ui))
            .unwrap_or_else(|| {
                smoke_host_failed(
                    format!("{example} UI assets"),
                    "missing UI config".to_string(),
                    "restore the example [ui] section",
                )
            });
        checks.push(ui_check);
    }

    checks
}

pub(super) fn smoke_host_parameter_sidecar_value(
    specs_path: &Utf8Path,
    manifest_path: &Utf8Path,
) -> Result<String, String> {
    let specs_text = read_text_file_no_symlink("parameter specs", specs_path)
        .map_err(|error| format!("failed to read {specs_path}: {error}"))?;
    let expected = parameter_manifest_from_specs_json(&specs_text)
        .map_err(|error| format!("invalid parameter specs {specs_path}: {error}"))?;
    let actual = read_parameter_manifest(manifest_path)
        .map_err(|error| format!("invalid parameter manifest {manifest_path}: {error}"))?;
    if actual != expected {
        return Err(format!(
            "parameter manifest is out of date: {manifest_path}; regenerate from {specs_path}"
        ));
    }
    Ok(format!(
        "{} parameter(s), idAlgorithm {}",
        actual.parameters.len(),
        actual.id_algorithm
    ))
}

pub(super) fn smoke_host_ui_assets_check(
    example_dir: &Utf8Path,
    example: &str,
    ui: &UiConfig,
) -> SmokeHostCheck {
    let dist = ui.dist.as_deref().unwrap_or("dist");
    let root = example_dir.join(&ui.dir).join(dist);
    match AssetManifest::from_dir(&root, "index.html") {
        Ok(manifest) => smoke_host_ok(
            format!("{example} UI assets"),
            format!("{} file(s), entry {}", manifest.files.len(), manifest.entry),
        ),
        Err(error) => smoke_host_failed(
            format!("{example} UI assets"),
            format!("{}: {error}", portable_report_path(&root)),
            "run the example UI build command before smoke-host",
        ),
    }
}

pub(super) fn smoke_host_bridge_trace_check(path: Option<&Utf8Path>) -> SmokeHostCheck {
    let Some(path) = path else {
        return smoke_host_skipped(
            "JSBridge trace",
            "no --bridge-trace evidence supplied",
            "pass a bridge trace containing readyAck/reply or param gesture packets",
        );
    };
    match read_text_file_no_symlink("bridge trace", path) {
        Ok(text)
            if daw_marker_positive(&text)
                && (bridge_trace_relayed_param_gesture(&text)
                    || (text.contains("readyAck") && text.contains("reply"))) =>
        {
            smoke_host_ok(
                "JSBridge trace",
                format!("accepted bridge evidence from {path}"),
            )
        }
        Ok(_) => smoke_host_failed(
            "JSBridge trace",
            format!("{path}: no accepted bridge roundtrip or param gesture markers"),
            "capture bridge.hello/readyAck/reply or param begin/perform/end trace",
        ),
        Err(error) => smoke_host_failed(
            "JSBridge trace",
            format!("failed to read {path}: {error}"),
            "pass a readable bridge trace file",
        ),
    }
}

pub(super) fn smoke_host_meter_log_check(path: Option<&Utf8Path>) -> SmokeHostCheck {
    let Some(path) = path else {
        return smoke_host_skipped(
            "meter stream",
            "no --meter-log evidence supplied",
            "pass a meter log or bridge trace containing a nonzero meter.main frame",
        );
    };
    match read_text_file_no_symlink("meter log", path) {
        Ok(text) if daw_marker_matches(&text, meter_stream_delivered) => smoke_host_ok(
            "meter stream",
            format!("accepted meter evidence from {path}"),
        ),
        Ok(_) => smoke_host_failed(
            "meter stream",
            format!("{path}: no accepted nonzero meter stream markers"),
            "capture meter_flush sent>0, meter lane packets, or meter.main peaks/rms",
        ),
        Err(error) => smoke_host_failed(
            "meter stream",
            format!("failed to read {path}: {error}"),
            "pass a readable meter log file",
        ),
    }
}

pub(super) fn smoke_host_ok(name: impl Into<String>, value: impl Into<String>) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "ok".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: None,
    }
}

pub(super) fn smoke_host_skipped(
    name: impl Into<String>,
    value: impl Into<String>,
    hint: impl Into<String>,
) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "skipped".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: Some(smoke_host_sanitize_report_text(hint)),
    }
}

pub(super) fn smoke_host_failed(
    name: impl Into<String>,
    value: impl Into<String>,
    hint: impl Into<String>,
) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "failed".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: Some(smoke_host_sanitize_report_text(hint)),
    }
}

pub(super) fn smoke_host_sanitize_report_text(value: impl Into<String>) -> String {
    sanitize_release_report_text(value)
}

pub(super) fn validate_smoke_host_report(
    report: &SmokeHostReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_smoke_host_report_shape(report)?;

    let mut errors = Vec::new();
    if report.version != 1 {
        errors.push(format!(
            "unsupported smoke-host report version {}",
            report.version
        ));
    }
    if report.generator != SMOKE_HOST_GENERATOR {
        errors.push(format!(
            "unexpected smoke-host generator `{}`",
            report.generator
        ));
    }
    if !matches!(report.status.as_str(), "ok" | "partial" | "failed") {
        errors.push(format!(
            "invalid smoke-host report status `{}`",
            report.status
        ));
    }
    for check in &report.checks {
        if !matches!(check.status.as_str(), "ok" | "skipped" | "failed") {
            errors.push(format!(
                "smoke-host check `{}` has invalid status `{}`",
                check.name, check.status
            ));
        }
    }
    let expected_status = if report.checks.iter().any(|check| check.status == "failed") {
        "failed"
    } else if report.checks.iter().any(|check| check.status == "skipped") {
        "partial"
    } else {
        "ok"
    };
    if report.status != expected_status {
        errors.push(format!(
            "smoke-host report status must be `{expected_status}` for its checks"
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("invalid smoke-host report: {}", errors.join("; ")).into())
    }
}

pub(super) fn validate_smoke_host_report_shape(
    report: &SmokeHostReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("smoke-host generator", &report.generator)?;
    validate_release_action_text("smoke-host workspace", &report.workspace)?;
    validate_release_action_text("smoke-host status", &report.status)?;
    validate_release_action_text(
        "smoke-host external evidence note",
        &report.external_evidence_note,
    )?;
    if report.checks.is_empty() {
        return Err("smoke-host report has no checks".into());
    }
    if report.checks.len() > SMOKE_HOST_MAX_CHECKS {
        return Err(format!(
            "smoke-host report has too many checks: {} exceeds maximum {SMOKE_HOST_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("smoke-host check name", &check.name)?;
        validate_release_action_text(
            &format!("smoke-host `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(&format!("smoke-host `{}` value", check.name), &check.value)?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("smoke-host `{}` hint", check.name), hint)?;
        }
        let normalized = normalize_smoke_host_check_name(&check.name);
        if normalized.is_empty() {
            return Err(format!(
                "smoke-host check `{}` does not normalize to a stable check name",
                check.name
            )
            .into());
        }
        if !seen_checks.insert(normalized.clone()) {
            return Err(format!("duplicate smoke-host check `{normalized}`").into());
        }
    }
    let expected_checks = expected_smoke_host_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!("unknown smoke-host check(s): {}", unknown_checks.join(", ")).into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "smoke-host report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

pub(super) fn normalize_smoke_host_check_name(value: &str) -> String {
    normalize_platform_smoke_check_name(value)
}

pub(super) fn smoke_host_report_all_ok(report: &SmokeHostReport) -> bool {
    report.status == "ok" && report.checks.iter().all(|check| check.status == "ok")
}

pub(super) fn print_smoke_host_report(
    report: &SmokeHostReport,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_smoke_host_report(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("smoke-host: {}", report.status);
            println!("workspace: {}", report.workspace);
            for check in &report.checks {
                println!("- {}: {} - {}", check.name, check.status, check.value);
                if let Some(hint) = &check.hint {
                    println!("  hint: {hint}");
                }
            }
            if let Some(out) = out {
                println!("report: {out}");
            }
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}
