use super::*;

pub(super) fn run_vst3_sdk_manifest(
    sdk_dir: Option<Utf8PathBuf>,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let manifest = vesty_vst3_sys::sdk_header_input_manifest(&sdk_dir)?;
    validate_vst3_sdk_header_manifest_shape(&manifest)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <sdk-header-manifest.json>")?;
        let text = read_text_file_no_symlink("VST3 SDK header manifest check input", out_path)?;
        let expected: vesty_vst3_sys::SdkHeaderInputManifest = serde_json::from_str(&text)?;
        vesty_vst3_sys::check_sdk_header_input_manifest(&sdk_dir, &expected)?;
        print_vst3_sdk_manifest_report(&sdk_dir, Some(out_path), &manifest, true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&manifest)? + "\n"))?;
        print_vst3_sdk_manifest_report(&sdk_dir, Some(out_path), &manifest, false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        print_vst3_sdk_manifest_report(&sdk_dir, None, &manifest, false, format)?;
    }

    Ok(())
}

pub(super) fn run_vst3_sdk_binding_plan(
    sdk_dir: Option<Utf8PathBuf>,
    bindings_module: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let plan = vesty_vst3_sys::generated_bindings_plan(&sdk_dir, &bindings_module)?;
    validate_vst3_sdk_binding_plan_shape(&plan)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <generated-bindings-plan.json>")?;
        let text =
            read_text_file_no_symlink("VST3 SDK generated bindings plan check input", out_path)?;
        let expected: vesty_vst3_sys::GeneratedBindingsPlan = serde_json::from_str(&text)?;
        let mut differences = Vec::new();
        if expected != plan {
            differences = generated_bindings_plan_differences(&expected, &plan);
        }
        if !differences.is_empty() {
            return Err(format!(
                "VST3 SDK generated bindings plan drift: {}",
                differences.join("; ")
            )
            .into());
        }
        print_vst3_sdk_binding_plan_report(&plan, Some(out_path), true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&plan)? + "\n"))?;
        print_vst3_sdk_binding_plan_report(&plan, Some(out_path), false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        print_vst3_sdk_binding_plan_report(&plan, None, false, format)?;
    }

    Ok(())
}

pub(super) fn run_vst3_sdk_binding_surface(
    sdk_dir: Option<Utf8PathBuf>,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let surface = vesty_vst3_sys::generated_bindings_surface(&sdk_dir)?;
    validate_vst3_sdk_binding_surface_shape(&surface)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <generated-bindings-surface.json>")?;
        let text =
            read_text_file_no_symlink("VST3 SDK generated bindings surface check input", out_path)?;
        let expected: vesty_vst3_sys::GeneratedBindingsSurface = serde_json::from_str(&text)?;
        let differences =
            vesty_vst3_sys::generated_bindings_surface_differences(&expected, &surface);
        if !differences.is_empty() {
            return Err(format!(
                "VST3 SDK generated bindings surface drift: {}",
                differences.join("; ")
            )
            .into());
        }
        print_vst3_sdk_binding_surface_report(&surface, Some(out_path), true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&surface)? + "\n"))?;
        print_vst3_sdk_binding_surface_report(&surface, Some(out_path), false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&surface)?);
    } else {
        print_vst3_sdk_binding_surface_report(&surface, None, false, format)?;
    }

    Ok(())
}

pub(super) fn run_vst3_sdk_emit_scaffold(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let scaffold = vesty_vst3_sys::generated_bindings_scaffold(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings scaffold check input", &out)?;
        if expected != scaffold.module {
            return Err(format!("VST3 SDK generated bindings scaffold drift: {out}").into());
        }
        print_vst3_sdk_emit_scaffold_report(&scaffold, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &scaffold.module)?;
    print_vst3_sdk_emit_scaffold_report(&scaffold, &out, false, format)?;
    Ok(())
}

pub(super) fn run_vst3_sdk_emit_abi_seed(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let seed = vesty_vst3_sys::generated_bindings_abi_seed(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings ABI seed check input", &out)?;
        if expected != seed.module {
            return Err(format!("VST3 SDK generated bindings ABI seed drift: {out}").into());
        }
        print_vst3_sdk_emit_abi_seed_report(&seed, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &seed.module)?;
    print_vst3_sdk_emit_abi_seed_report(&seed, &out, false, format)?;
    Ok(())
}

pub(super) fn run_vst3_sdk_emit_abi(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let abi = vesty_vst3_sys::generated_bindings_abi(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings ABI layout check input", &out)?;
        if expected != abi.module {
            return Err(format!("VST3 SDK generated bindings ABI layout drift: {out}").into());
        }
        print_vst3_sdk_emit_abi_report(&abi, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &abi.module)?;
    print_vst3_sdk_emit_abi_report(&abi, &out, false, format)?;
    Ok(())
}

pub(super) fn run_vst3_sdk_emit_interface_skeleton(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let skeleton = vesty_vst3_sys::generated_bindings_interface_skeleton(&sdk_dir, &out)?;

    if check {
        let expected = read_text_file_no_symlink(
            "VST3 SDK generated bindings interface skeleton check input",
            &out,
        )?;
        if expected != skeleton.module {
            return Err(
                format!("VST3 SDK generated bindings interface skeleton drift: {out}").into(),
            );
        }
        print_vst3_sdk_emit_interface_skeleton_report(&skeleton, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &skeleton.module)?;
    print_vst3_sdk_emit_interface_skeleton_report(&skeleton, &out, false, format)?;
    Ok(())
}

pub(super) fn generated_bindings_plan_differences(
    expected: &vesty_vst3_sys::GeneratedBindingsPlan,
    actual: &vesty_vst3_sys::GeneratedBindingsPlan,
) -> Vec<String> {
    let mut differences = Vec::new();
    if expected.version != actual.version {
        differences.push(format!(
            "version expected {} actual {}",
            expected.version, actual.version
        ));
    }
    if expected.generator != actual.generator {
        differences.push(format!(
            "generator expected {} actual {}",
            expected.generator, actual.generator
        ));
    }
    if expected.status != actual.status {
        differences.push(format!(
            "status expected {} actual {}",
            expected.status, actual.status
        ));
    }
    if expected.bindings_generated != actual.bindings_generated {
        differences.push(format!(
            "bindings_generated expected {} actual {}",
            expected.bindings_generated, actual.bindings_generated
        ));
    }
    if expected.steinberg_sdk_baseline != actual.steinberg_sdk_baseline {
        differences.push(format!(
            "Steinberg SDK baseline expected {} actual {}",
            expected.steinberg_sdk_baseline, actual.steinberg_sdk_baseline
        ));
    }
    if expected.upstream_vst3_crate_baseline != actual.upstream_vst3_crate_baseline {
        differences.push(format!(
            "upstream vst3 crate baseline expected {} actual {}",
            expected.upstream_vst3_crate_baseline, actual.upstream_vst3_crate_baseline
        ));
    }
    if expected.active_backend != actual.active_backend {
        differences.push(format!(
            "active backend expected {} actual {}",
            expected.active_backend, actual.active_backend
        ));
    }
    if expected.sdk_dir != actual.sdk_dir {
        differences.push(format!(
            "sdk dir expected {} actual {}",
            expected.sdk_dir, actual.sdk_dir
        ));
    }
    if expected.bindings_module != actual.bindings_module {
        differences.push(format!(
            "bindings module expected {} actual {}",
            expected.bindings_module, actual.bindings_module
        ));
    }
    differences.extend(vesty_vst3_sys::sdk_header_manifest_differences(
        &expected.header_manifest,
        &actual.header_manifest,
    ));
    if expected.blockers != actual.blockers {
        differences.push(format!(
            "blockers expected {:?} actual {:?}",
            expected.blockers, actual.blockers
        ));
    }
    if expected.checks != actual.checks {
        differences.push("checks changed".to_string());
    }
    if expected.next_steps != actual.next_steps {
        differences.push("next steps changed".to_string());
    }
    differences
}

pub(super) fn resolve_vst3_sdk_dir(
    sdk_dir: Option<Utf8PathBuf>,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    resolve_vst3_sdk_dir_from_env_value(sdk_dir, std::env::var_os(vesty_vst3_sys::VST3_SDK_DIR_ENV))
}

pub(super) fn resolve_vst3_sdk_dir_from_env_value(
    sdk_dir: Option<Utf8PathBuf>,
    env_value: Option<std::ffi::OsString>,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if let Some(sdk_dir) = sdk_dir {
        return Ok(sdk_dir);
    }
    let raw = env_value.ok_or_else(|| {
        format!(
            "pass --sdk-dir <path> or set {}",
            vesty_vst3_sys::VST3_SDK_DIR_ENV
        )
    })?;
    Utf8PathBuf::from_path_buf(std::path::PathBuf::from(raw)).map_err(|path| {
        format!(
            "{} is not valid UTF-8: {}",
            vesty_vst3_sys::VST3_SDK_DIR_ENV,
            path.display()
        )
        .into()
    })
}

pub(super) fn print_vst3_sdk_manifest_report(
    sdk_dir: &Utf8Path,
    out: Option<&Utf8Path>,
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_header_manifest_shape(manifest)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(manifest)?);
        }
        OutputFormat::Text => {
            let status = if checked {
                "matches"
            } else if manifest.complete {
                "complete"
            } else {
                "incomplete"
            };
            println!("VST3 SDK header manifest: {status}");
            println!("sdk dir: {sdk_dir}");
            if let Some(out) = out {
                println!("manifest: {out}");
            }
            println!("baseline: {}", manifest.steinberg_sdk_baseline);
            println!("generator: {}", manifest.generator);
            println!("headers: {}", manifest.headers.len());
            println!("missing headers: {}", manifest.missing_headers.len());
            if !manifest.missing_headers.is_empty() {
                println!("missing: {}", manifest.missing_headers.join(", "));
            }
            if let Some(version_hint) = manifest.version_hint.as_deref() {
                println!("version hint: {version_hint}");
            }
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_binding_plan_report(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
    out: Option<&Utf8Path>,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_plan_shape(plan)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(plan)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings plan: {}",
                if checked {
                    "matches"
                } else {
                    plan.status.as_str()
                }
            );
            println!("sdk dir: {}", plan.sdk_dir);
            println!("bindings module: {}", plan.bindings_module);
            if let Some(out) = out {
                println!("plan: {out}");
            }
            println!("active backend: {}", plan.active_backend);
            println!("bindings generated: {}", plan.bindings_generated);
            println!("headers: {}", plan.header_manifest.headers.len());
            println!(
                "missing headers: {}",
                plan.header_manifest.missing_headers.len()
            );
            if !plan.blockers.is_empty() {
                println!("blockers: {}", plan.blockers.join("; "));
            }
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_binding_surface_report(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
    out: Option<&Utf8Path>,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_surface_shape(surface)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(surface)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings surface: {}",
                if checked {
                    "matches"
                } else {
                    surface.status.as_str()
                }
            );
            println!("sdk dir: {}", surface.sdk_dir);
            if let Some(out) = out {
                println!("surface: {out}");
            }
            println!("active backend: {}", surface.active_backend);
            println!("bindings generated: {}", surface.bindings_generated);
            println!("required headers: {}", surface.required_headers.len());
            println!("symbols: {}", surface.symbols.len());
            println!(
                "missing headers: {}",
                surface.header_manifest.missing_headers.len()
            );
            if !surface.missing_headers.is_empty() {
                println!(
                    "missing surface headers: {}",
                    surface.missing_headers.join("; ")
                );
            }
            println!("missing symbols: {}", surface.missing_symbols.len());
            if !surface.missing_symbols.is_empty() {
                println!(
                    "missing surface symbols: {}",
                    surface.missing_symbols.join("; ")
                );
            }
            if !surface.blockers.is_empty() {
                println!("blockers: {}", surface.blockers.join("; "));
            }
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_emit_scaffold_report(
    scaffold: &vesty_vst3_sys::GeneratedBindingsScaffold,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR,
                "bindingsGenerated": false,
                "planStatus": scaffold.plan.status,
                "headerCount": scaffold.plan.header_manifest.headers.len(),
                "activeBackend": scaffold.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings scaffold: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR
            );
            println!("bindings generated: false");
            println!("plan status: {}", scaffold.plan.status);
            println!("active backend: {}", scaffold.plan.active_backend);
            println!("headers: {}", scaffold.plan.header_manifest.headers.len());
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_emit_abi_seed_report(
    seed: &vesty_vst3_sys::GeneratedBindingsAbiSeed,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR,
                "abiSeedGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": seed.plan.status,
                "surfaceStatus": seed.surface.status,
                "headerCount": seed.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": seed.surface.symbols.len(),
                "activeBackend": seed.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings ABI seed: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR
            );
            println!("ABI seed generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", seed.plan.status);
            println!("surface status: {}", seed.surface.status);
            println!("active backend: {}", seed.plan.active_backend);
            println!("headers: {}", seed.plan.header_manifest.headers.len());
            println!("surface symbols: {}", seed.surface.symbols.len());
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_emit_abi_report(
    abi: &vesty_vst3_sys::GeneratedBindingsAbi,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR,
                "abiLayoutGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": abi.plan.status,
                "surfaceStatus": abi.surface.status,
                "headerCount": abi.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": abi.surface.symbols.len(),
                "activeBackend": abi.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings ABI layout: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR
            );
            println!("ABI layout generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", abi.plan.status);
            println!("surface status: {}", abi.surface.status);
            println!("active backend: {}", abi.plan.active_backend);
            println!("headers: {}", abi.plan.header_manifest.headers.len());
            println!("surface symbols: {}", abi.surface.symbols.len());
        }
    }
    Ok(())
}

pub(super) fn print_vst3_sdk_emit_interface_skeleton_report(
    skeleton: &vesty_vst3_sys::GeneratedBindingsInterfaceSkeleton,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR,
                "interfaceSkeletonGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": skeleton.plan.status,
                "surfaceStatus": skeleton.surface.status,
                "headerCount": skeleton.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": skeleton.surface.symbols.len(),
                "interfaceCount": skeleton
                    .surface
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == "interface")
                    .count(),
                "activeBackend": skeleton.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings interface skeleton: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR
            );
            println!("interface skeleton generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", skeleton.plan.status);
            println!("surface status: {}", skeleton.surface.status);
            println!("active backend: {}", skeleton.plan.active_backend);
            println!("headers: {}", skeleton.plan.header_manifest.headers.len());
            println!("surface symbols: {}", skeleton.surface.symbols.len());
            println!(
                "interfaces: {}",
                skeleton
                    .surface
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == "interface")
                    .count()
            );
        }
    }
    Ok(())
}
