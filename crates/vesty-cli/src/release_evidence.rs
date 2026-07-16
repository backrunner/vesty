use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollectLocalOptions {
    pub(super) template: bool,
    pub(super) protocol: bool,
    pub(super) publish_plan: bool,
    pub(super) crate_package: bool,
    pub(super) npm_pack: bool,
    pub(super) dependency_baseline_latest: bool,
    pub(super) vst3_sdk_dir: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_bindings_module: Utf8PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct LocalReleaseEvidenceReport {
    pub(super) evidence_dir: String,
    pub(super) workspace: String,
    pub(super) protocol_snapshot: Option<String>,
    pub(super) items: Vec<LocalReleaseEvidenceItem>,
    pub(super) external_evidence_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct LocalReleaseEvidenceItem {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) path: Option<String>,
    pub(super) value: String,
}

pub(super) const RELEASE_EVIDENCE_REPORT_MAX_ITEMS: usize = 1024;

pub(super) fn collect_local_release_evidence(
    workspace: &Utf8Path,
    dir: &Utf8Path,
    protocol_snapshot: &Utf8Path,
    options: CollectLocalOptions,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", dir)?;
    let mut items = Vec::new();

    if options.template {
        let created = write_release_evidence_templates(&dir.to_path_buf())?;
        items.push(LocalReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(dir)),
            value: format!("{created} template file(s) created; existing files preserved"),
        });
    }

    if options.protocol {
        let report = vesty_ipc::export_protocol_bindings(protocol_snapshot)?;
        check_protocol_export(protocol_snapshot)?;
        items.push(LocalReleaseEvidenceItem {
            name: "protocol snapshot".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(protocol_snapshot)),
            value: format!(
                "{} TypeScript file(s), {} JSON schema file(s)",
                report.typescript_files, report.json_schema_files
            ),
        });
    }

    if options.publish_plan {
        let path = dir.join("publish-plan/publish-plan.json");
        let metadata = cargo_metadata_json(workspace)?;
        let plan = workspace_publish_plan(&metadata)?;
        write_publish_plan_report(&path, &plan)?;
        let evidence = validate_publish_plan_report(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "crate publish plan".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} publishable crates; {} private skipped; final crate: {}",
                evidence.package_count, evidence.skipped_private_count, evidence.final_package
            ),
        });
    }

    if options.crate_package {
        let path = dir.join("crate-package/crate-package.json");
        let report = crate_package_report(workspace, true)?;
        write_crate_package_report(&path, &report)?;
        let evidence = validate_crate_package_report_path(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "crate package readiness".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} packageable now; {} deferred until internal dependencies publish",
                evidence.packaged_count, evidence.deferred_count
            ),
        });
    }

    if options.npm_pack {
        let path = dir.join("npm-pack/npm-pack.json");
        let raw_report = npm_pack_dry_run_json(workspace)?;
        let entries = parse_npm_pack_report_text(&raw_report)?;
        let evidence = validate_npm_pack_entries(&entries)?;
        write_text_file(&path, &(serde_json::to_string_pretty(&entries)? + "\n"))?;
        items.push(LocalReleaseEvidenceItem {
            name: "npm package pack report".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} package(s), {} file(s): {}",
                evidence.package_count,
                evidence.total_files,
                evidence.packages.join(", ")
            ),
        });
    }

    if options.dependency_baseline_latest {
        let path = dir.join("dependency-baseline/dependency-baseline-latest.json");
        let report =
            dependency_baseline_report_with_latest(workspace, &CommandLatestDependencyFetcher)?;
        write_dependency_baseline_report(&path, &report)?;
        let evidence = validate_dependency_baseline_latest_report(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "dependency latest baseline".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} baseline check(s), {} latest registry check(s)",
                evidence.baseline_checks, evidence.latest_checks
            ),
        });
    }

    if let Some(sdk_dir) = options.vst3_sdk_dir.as_deref() {
        let manifest_path = dir.join("vst3-sdk/vst3-sdk-headers.json");
        let manifest = vesty_vst3_sys::sdk_header_input_manifest(sdk_dir)?;
        write_text_file(
            &manifest_path,
            &(serde_json::to_string_pretty(&manifest)? + "\n"),
        )?;
        let manifest_evidence = validate_vst3_sdk_header_manifest(&manifest_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&manifest_path)),
            value: format!(
                "{} required header(s); Steinberg SDK {}; upstream vst3 crate {}",
                manifest_evidence.header_count,
                manifest_evidence.baseline,
                manifest_evidence.upstream_vst3_crate
            ),
        });

        let binding_plan_path = dir.join("vst3-sdk/generated-bindings-plan.json");
        let binding_plan =
            vesty_vst3_sys::generated_bindings_plan(sdk_dir, &options.vst3_sdk_bindings_module)?;
        write_text_file(
            &binding_plan_path,
            &(serde_json::to_string_pretty(&binding_plan)? + "\n"),
        )?;
        let plan_evidence = validate_vst3_sdk_binding_plan(&binding_plan_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&binding_plan_path)),
            value: format!(
                "{}; {} required header(s); active backend {}; module {}",
                plan_evidence.status,
                plan_evidence.header_count,
                plan_evidence.active_backend,
                plan_evidence.bindings_module
            ),
        });

        let binding_surface_path = dir.join("vst3-sdk/generated-bindings-surface.json");
        let binding_surface = vesty_vst3_sys::generated_bindings_surface(sdk_dir)?;
        write_text_file(
            &binding_surface_path,
            &(serde_json::to_string_pretty(&binding_surface)? + "\n"),
        )?;
        let surface_evidence = validate_vst3_sdk_binding_surface(&binding_surface_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&binding_surface_path)),
            value: format!(
                "{}; {} required header(s); {} symbol(s); active backend {}; bindings generated false",
                surface_evidence.status,
                surface_evidence.header_count,
                surface_evidence.symbol_count,
                surface_evidence.active_backend
            ),
        });

        let scaffold_path = dir.join("vst3-sdk/generated.rs");
        let scaffold = vesty_vst3_sys::generated_bindings_scaffold(
            sdk_dir,
            &options.vst3_sdk_bindings_module,
        )?;
        write_text_file(&scaffold_path, &scaffold.module)?;
        validate_vst3_sdk_generated_bindings_scaffold_text(&scaffold.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings scaffold".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&scaffold_path)),
            value: format!(
                "metadata scaffold; {} required header(s); active backend {}; bindings generated false",
                scaffold.plan.header_manifest.headers.len(),
                scaffold.plan.active_backend
            ),
        });

        let abi_seed_path = dir.join("vst3-sdk/generated-abi-seed.rs");
        let abi_seed = vesty_vst3_sys::generated_bindings_abi_seed(sdk_dir, &abi_seed_path)?;
        write_text_file(&abi_seed_path, &abi_seed.module)?;
        validate_vst3_sdk_generated_bindings_abi_seed_text(&abi_seed.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings ABI seed".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&abi_seed_path)),
            value: format!(
                "ABI seed aliases/constants; {} required header(s); {} symbol(s); active backend {}; full COM bindings generated false",
                abi_seed.plan.header_manifest.headers.len(),
                abi_seed.surface.symbols.len(),
                abi_seed.plan.active_backend
            ),
        });

        let abi_path = dir.join("vst3-sdk/generated-abi.rs");
        let abi = vesty_vst3_sys::generated_bindings_abi(sdk_dir, &abi_path)?;
        write_text_file(&abi_path, &abi.module)?;
        validate_vst3_sdk_generated_bindings_abi_text(&abi.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings ABI layout".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&abi_path)),
            value: format!(
                "ABI layout module; {} required header(s); {} symbol(s); active backend {}; layout fingerprints present; full COM bindings generated false",
                abi.plan.header_manifest.headers.len(),
                abi.surface.symbols.len(),
                abi.plan.active_backend
            ),
        });

        let interface_skeleton_path = dir.join("vst3-sdk/generated-interface-skeleton.rs");
        let interface_skeleton = vesty_vst3_sys::generated_bindings_interface_skeleton(
            sdk_dir,
            &interface_skeleton_path,
        )?;
        write_text_file(&interface_skeleton_path, &interface_skeleton.module)?;
        validate_vst3_sdk_generated_bindings_interface_skeleton_text(&interface_skeleton.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings interface skeleton".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&interface_skeleton_path)),
            value: format!(
                "interface/vtable skeleton module; {} required header(s); {} symbol(s); active backend {}; full COM bindings generated false",
                interface_skeleton.plan.header_manifest.headers.len(),
                interface_skeleton.surface.symbols.len(),
                interface_skeleton.plan.active_backend
            ),
        });
    }

    let report = LocalReleaseEvidenceReport {
        evidence_dir: portable_report_path(dir),
        workspace: portable_report_path(workspace),
        protocol_snapshot: options
            .protocol
            .then(|| portable_report_path(protocol_snapshot)),
        items,
        external_evidence_note: "collect-local only gathers local protocol/publish/npm, opt-in dependency latest review and explicitly requested VST3 SDK audit evidence; DAW matrix, platform smoke, validator, CI, signing and notarization evidence must come from real external runs".to_string(),
    };
    validate_local_release_evidence_report_shape(&report)?;
    let report_path = dir.join("local-collect-report.json");
    write_text_file(
        &report_path,
        &(serde_json::to_string_pretty(&report)? + "\n"),
    )?;
    print_local_release_evidence_report(&report, format, &report_path)
}

pub(super) fn print_local_release_evidence_report(
    report: &LocalReleaseEvidenceReport,
    format: OutputFormat,
    report_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_local_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputFormat::Text => {
            println!("local release evidence: {}", report.evidence_dir);
            for item in &report.items {
                let path = item
                    .path
                    .as_deref()
                    .map(|path| format!(" ({path})"))
                    .unwrap_or_default();
                println!("- {}: {}{} - {}", item.name, item.status, path, item.value);
            }
            println!("- report: {report_path}");
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub(super) struct CollectSigningOptions {
    pub(super) bundle: Utf8PathBuf,
    pub(super) platform: Option<String>,
    pub(super) binary: Option<Utf8PathBuf>,
    pub(super) dir: Utf8PathBuf,
    pub(super) out: Option<Utf8PathBuf>,
    pub(super) tool: Option<Utf8PathBuf>,
    pub(super) format: String,
}

#[derive(Clone, Debug)]
pub(super) struct CollectNotarizationOptions {
    pub(super) notary_log: Utf8PathBuf,
    pub(super) stapler_log: Option<Utf8PathBuf>,
    pub(super) dir: Utf8PathBuf,
    pub(super) out: Option<Utf8PathBuf>,
    pub(super) format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct CollectedReleaseEvidenceReport {
    pub(super) evidence_dir: String,
    pub(super) kind: String,
    pub(super) output: String,
    pub(super) items: Vec<LocalReleaseEvidenceItem>,
}

pub(super) fn collect_signing_release_evidence(
    options: CollectSigningOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", &options.dir)?;
    let platform = match options.platform.as_deref() {
        Some(platform) => parse_bundle_platform(platform)?,
        None => infer_signing_bundle_platform(&options.bundle)?,
    };
    let output_path = options
        .out
        .unwrap_or_else(|| default_signing_evidence_path(&options.dir, platform));
    let command = signing_verification_command(
        platform,
        &options.bundle,
        options.binary.as_deref(),
        options.tool.as_deref(),
    )?;
    let output = run_command_spec_capture(&command, "signing verification")?;
    let text = captured_command_log(&command, &output);
    let platforms = signing_evidence_platforms_from_text(&text)?;
    let expected = signing_platform_for_bundle_platform(platform)?;
    if !platforms.contains(&expected) {
        return Err(format!(
            "signing verification output did not prove {} coverage",
            expected.label()
        )
        .into());
    }

    let report = CollectedReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        kind: "signing".to_string(),
        output: portable_report_path(&output_path),
        items: vec![LocalReleaseEvidenceItem {
            name: format!("{} signing verification", expected.label()),
            status: "ok".to_string(),
            path: Some(portable_report_path(&output_path)),
            value: format!("{} {}", command.program, command.args.join(" ")),
        }],
    };
    validate_collected_release_evidence_report_shape(&report)?;
    write_text_file(&output_path, &text)?;
    print_collected_release_evidence_report(&report, format)
}

pub(super) fn collect_notarization_release_evidence(
    options: CollectNotarizationOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", &options.dir)?;
    let output_path = options
        .out
        .unwrap_or_else(|| options.dir.join("notary.log"));
    let mut text = String::new();
    text.push_str(&format!("notary_log={}\n", options.notary_log));
    text.push_str("\n[notarytool]\n");
    text.push_str(&read_text_file_no_symlink(
        "notarytool log",
        &options.notary_log,
    )?);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(stapler_log) = options.stapler_log.as_ref() {
        text.push_str(&format!("\nstapler_log={stapler_log}\n"));
        text.push_str("\n[stapler]\n");
        text.push_str(&read_text_file_no_symlink("stapler log", stapler_log)?);
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    let evidence = notarization_evidence_from_text(&text)?;
    if !evidence.accepted || !evidence.stapled {
        let mut missing = Vec::new();
        if !evidence.accepted {
            missing.push("accepted notarytool result");
        }
        if !evidence.stapled {
            missing.push("stapler success");
        }
        return Err(format!(
            "notarization collection is incomplete: missing {}",
            missing.join(", ")
        )
        .into());
    }

    let report = CollectedReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        kind: "notarization".to_string(),
        output: portable_report_path(&output_path),
        items: vec![LocalReleaseEvidenceItem {
            name: "macOS notarization".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&output_path)),
            value: "accepted notarytool result and stapler success".to_string(),
        }],
    };
    validate_collected_release_evidence_report_shape(&report)?;
    write_text_file(&output_path, &text)?;
    print_collected_release_evidence_report(&report, format)
}

pub(super) fn print_collected_release_evidence_report(
    report: &CollectedReleaseEvidenceReport,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_collected_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("{} release evidence: {}", report.kind, report.output);
            for item in &report.items {
                println!("- {}: {} - {}", item.name, item.status, item.value);
            }
        }
    }
    Ok(())
}
