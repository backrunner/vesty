use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::*;

pub fn generated_bindings_plan(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsPlan, SdkHeaderManifestError> {
    let root = root.as_ref();
    let bindings_module = bindings_module.as_ref();
    let manifest = sdk_header_input_manifest(root)?;
    let mut checks = Vec::new();
    let mut blockers = Vec::new();

    if manifest.complete {
        checks.push(GeneratedBindingsPlanCheck {
            name: "sdk header inputs".to_string(),
            status: "ok".to_string(),
            value: format!("{} required header(s)", manifest.headers.len()),
            hint: None,
        });
    } else {
        let missing = manifest.missing_headers.join(", ");
        blockers.push(format!("missing SDK header inputs: {missing}"));
        checks.push(GeneratedBindingsPlanCheck {
            name: "sdk header inputs".to_string(),
            status: "failed".to_string(),
            value: format!(
                "missing {} required header(s)",
                manifest.missing_headers.len()
            ),
            hint: Some(format!(
                "use an official Steinberg VST3 SDK {STEINBERG_VST3_SDK_BASELINE} checkout"
            )),
        });
    }

    let module_check = generated_bindings_module_path_check(bindings_module);
    if module_check.status == "failed" {
        blockers.push(module_check.value.clone());
    }
    checks.push(module_check);
    checks.push(GeneratedBindingsPlanCheck {
        name: "binding emitter".to_string(),
        status: "reserved".to_string(),
        value: "Vesty has locked SDK inputs; the full SDK 3.8 Rust binding emitter is not enabled yet"
            .to_string(),
        hint: Some(
            "keep using the upstream `vst3` crate backend until generated bindings are implemented and validated"
                .to_string(),
        ),
    });

    let status = if blockers.is_empty() {
        "ready-for-binding-generator"
    } else {
        "blocked"
    };

    Ok(GeneratedBindingsPlan {
        version: GENERATED_BINDINGS_PLAN_VERSION,
        generator: GENERATED_BINDINGS_PLAN_GENERATOR.to_string(),
        status: status.to_string(),
        bindings_generated: false,
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        active_backend: binding_backend_name(BINDING_BASELINE.backend).to_string(),
        sdk_dir: root.display().to_string(),
        bindings_module: bindings_module.display().to_string(),
        header_manifest: manifest,
        checks,
        blockers,
        next_steps: vec![
            "keep the SDK header manifest under release evidence when auditing generated-header inputs".to_string(),
            "implement the generated binding emitter from the locked header set".to_string(),
            "compare generated bindings against upstream `vst3` crate coverage before switching the adapter backend".to_string(),
            "run Steinberg validator and DAW smoke before claiming generated SDK 3.8 backend support".to_string(),
        ],
    })
}

pub fn generated_bindings_surface(
    root: impl AsRef<Path>,
) -> Result<GeneratedBindingsSurface, SdkHeaderManifestError> {
    let root = root.as_ref();
    let manifest = sdk_header_input_manifest(root)?;
    let present_headers = manifest
        .headers
        .iter()
        .map(|header| header.path.as_str())
        .collect::<BTreeSet<_>>();
    let required_headers = REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .map(|header| (*header).to_string())
        .collect::<Vec<_>>();
    let mut header_texts = BTreeMap::new();
    for header in &manifest.headers {
        let path = root.join(&header.path);
        let Some((_, bytes)) = sdk_header_bytes_no_symlink(&path)? else {
            continue;
        };
        header_texts.insert(
            header.path.as_str(),
            String::from_utf8_lossy(&bytes).into_owned(),
        );
    }
    let symbols = GENERATED_BINDINGS_SURFACE_SYMBOLS
        .iter()
        .map(|symbol| {
            let header_present = present_headers.contains(symbol.header);
            let symbol_present = header_present
                && header_texts
                    .get(symbol.header)
                    .is_some_and(|text| contains_identifier_token(text, symbol.name));
            GeneratedBindingsSurfaceSymbol {
                name: symbol.name.to_string(),
                kind: symbol.kind.to_string(),
                header: symbol.header.to_string(),
                purpose: symbol.purpose.to_string(),
                header_present,
                symbol_present,
            }
        })
        .collect::<Vec<_>>();

    let mut blockers = Vec::new();
    if !manifest.complete {
        blockers.push(format!(
            "missing SDK header inputs: {}",
            manifest.missing_headers.join(", ")
        ));
    }

    let unknown_symbol_headers = symbols
        .iter()
        .filter(|symbol| {
            !REQUIRED_GENERATED_HEADER_INPUTS
                .iter()
                .any(|header| *header == symbol.header)
        })
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !unknown_symbol_headers.is_empty() {
        blockers.push(format!(
            "surface symbols reference headers outside the locked input set: {}",
            unknown_symbol_headers.join(", ")
        ));
    }

    let missing_symbol_headers = symbols
        .iter()
        .filter(|symbol| !symbol.header_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !missing_symbol_headers.is_empty() {
        blockers.push(format!(
            "surface symbols reference missing headers: {}",
            missing_symbol_headers.join(", ")
        ));
    }

    let missing_symbols = symbols
        .iter()
        .filter(|symbol| symbol.header_present && !symbol.symbol_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !missing_symbols.is_empty() {
        blockers.push(format!(
            "surface symbols are absent from their locked headers: {}",
            missing_symbols.join(", ")
        ));
    }

    let status = if blockers.is_empty() {
        "ready-for-binding-emitter"
    } else {
        "blocked"
    };

    Ok(GeneratedBindingsSurface {
        version: GENERATED_BINDINGS_SURFACE_VERSION,
        generator: GENERATED_BINDINGS_SURFACE_GENERATOR.to_string(),
        status: status.to_string(),
        bindings_generated: false,
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        active_backend: binding_backend_name(BINDING_BASELINE.backend).to_string(),
        sdk_dir: root.display().to_string(),
        header_manifest: manifest,
        required_headers,
        missing_headers: missing_symbol_headers,
        missing_symbols,
        symbols,
        blockers,
        notes: vec![
            "this report locks the expected VST3 binding symbol surface for the future generated-header backend and verifies identifier tokens are present in the locked headers".to_string(),
            "it does not parse C++ AST, verify ABI layout, or claim generated bindings are complete".to_string(),
            "bindingsGenerated must remain false until a real emitter generates and validates usable Rust COM bindings".to_string(),
        ],
    })
}

pub fn generated_bindings_surface_differences(
    expected: &GeneratedBindingsSurface,
    actual: &GeneratedBindingsSurface,
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
    differences.extend(sdk_header_manifest_differences(
        &expected.header_manifest,
        &actual.header_manifest,
    ));
    if expected.required_headers != actual.required_headers {
        differences.push("required headers changed".to_string());
    }
    if expected.missing_headers != actual.missing_headers {
        differences.push(format!(
            "missing surface headers expected {:?} actual {:?}",
            expected.missing_headers, actual.missing_headers
        ));
    }
    if expected.missing_symbols != actual.missing_symbols {
        differences.push(format!(
            "missing surface symbols expected {:?} actual {:?}",
            expected.missing_symbols, actual.missing_symbols
        ));
    }
    if expected.symbols != actual.symbols {
        differences.push("surface symbols changed".to_string());
    }
    if expected.blockers != actual.blockers {
        differences.push(format!(
            "blockers expected {:?} actual {:?}",
            expected.blockers, actual.blockers
        ));
    }
    if expected.notes != actual.notes {
        differences.push("notes changed".to_string());
    }
    differences
}
