use super::*;

fn create_sdk_root(missing: &[&str]) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("pluginterfaces/base")).unwrap();
    std::fs::create_dir_all(root.join("pluginterfaces/vst")).unwrap();
    std::fs::write(root.join("README.md"), "VST3 SDK v3.8.0_build_66").unwrap();
    for header in REQUIRED_GENERATED_HEADER_INPUTS {
        if missing.contains(header) {
            continue;
        }
        std::fs::create_dir_all(root.join(header).parent().unwrap()).unwrap();
        std::fs::write(root.join(header), test_header_text(header)).unwrap();
    }
    temp
}

fn test_header_text(header: &str) -> String {
    let mut text = format!("/* {header} */\n");
    for symbol in generated_bindings_surface_symbol_names_for_header(header) {
        text.push_str(symbol);
        text.push('\n');
    }
    text
}

#[test]
fn binding_baseline_matches_framework_plan() {
    assert_eq!(BINDING_BASELINE.steinberg_sdk, "v3.8.0_build_66");
    assert_eq!(BINDING_BASELINE.upstream_vst3_crate, "0.3.0");
    let expected_backend = if cfg!(feature = "generated-headers") {
        BindingBackend::GeneratedHeadersReserved
    } else if cfg!(feature = "upstream-vst3") {
        BindingBackend::UpstreamVst3Crate
    } else {
        BindingBackend::MetadataOnly
    };
    assert_eq!(BINDING_BASELINE.backend, expected_backend);
}

#[test]
fn upstream_probe_follows_feature_flag() {
    assert_eq!(upstream_vst3_available(), cfg!(feature = "upstream-vst3"));
}

#[test]
fn sdk_header_probe_reports_missing_and_present_headers() {
    let temp = create_sdk_root(&["pluginterfaces/vst/ivstmessage.h"]);
    let root = temp.path();

    let probe = probe_sdk_headers(root);

    assert!(!probe.ready_for_generated_headers());
    assert_eq!(
        probe.missing_headers,
        vec!["pluginterfaces/vst/ivstmessage.h"]
    );
    assert!(probe.version_hint.as_deref().unwrap().contains("3.8"));

    std::fs::write(
        root.join("pluginterfaces/vst/ivstmessage.h"),
        "/* header */",
    )
    .unwrap();
    let probe = probe_sdk_headers(root);
    assert!(probe.ready_for_generated_headers());
    assert!(probe.missing_headers.is_empty());
}

#[cfg(unix)]
#[test]
fn sdk_header_probe_treats_symlink_inputs_as_missing() {
    use std::os::unix::fs::symlink;

    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);
    let target = temp.path().join("README.md");
    symlink(target, temp.path().join("pluginterfaces/base/funknown.h")).unwrap();

    let probe = probe_sdk_headers(temp.path());

    assert!(!probe.ready_for_generated_headers());
    assert!(
        probe
            .missing_headers
            .contains(&"pluginterfaces/base/funknown.h")
    );
    assert!(
        !probe
            .present_headers
            .contains(&"pluginterfaces/base/funknown.h")
    );
}

#[test]
fn sdk_header_manifest_records_required_inputs_and_detects_drift() {
    let temp = create_sdk_root(&[]);
    let root = temp.path();

    let manifest = sdk_header_input_manifest(root).unwrap();

    assert_eq!(manifest.version, SDK_HEADER_MANIFEST_VERSION);
    assert_eq!(manifest.generator, SDK_HEADER_MANIFEST_GENERATOR);
    assert_eq!(manifest.steinberg_sdk_baseline, "v3.8.0_build_66");
    assert_eq!(manifest.upstream_vst3_crate_baseline, "0.3.0");
    assert!(manifest.complete);
    assert!(manifest.missing_headers.is_empty());
    assert_eq!(
        manifest.headers.len(),
        REQUIRED_GENERATED_HEADER_INPUTS.len()
    );
    let header = manifest
        .header("pluginterfaces/vst/vsttypes.h")
        .expect("vsttypes header manifest entry");
    assert_eq!(
        header.size,
        test_header_text("pluginterfaces/vst/vsttypes.h").len() as u64
    );
    assert_eq!(header.sha256.len(), 64);
    assert!(header.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));

    check_sdk_header_input_manifest(root, &manifest).unwrap();

    std::fs::write(
        root.join("pluginterfaces/vst/vsttypes.h"),
        "/* changed header */\n",
    )
    .unwrap();
    let error = check_sdk_header_input_manifest(root, &manifest).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("SDK header manifest drift"));
    assert!(text.contains("pluginterfaces/vst/vsttypes.h"));
}

#[test]
fn sdk_header_manifest_reports_missing_inputs() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let manifest = sdk_header_input_manifest(temp.path()).unwrap();

    assert!(!manifest.complete);
    assert_eq!(
        manifest.missing_headers,
        vec!["pluginterfaces/base/funknown.h"]
    );
    assert!(manifest.header("pluginterfaces/base/funknown.h").is_none());
}

#[test]
fn binary_export_symbol_plan_public_api_matches_required_symbol_helpers() {
    assert_eq!(binary_export_symbol_plans().len(), 11);
    assert_eq!(required_binary_export_symbol_count("macos"), 5);
    assert_eq!(required_binary_export_symbol_count("windows-x64"), 3);
    assert_eq!(required_binary_export_symbol_count("linux-x64"), 3);
    assert_eq!(required_binary_export_symbol_count("wayland"), 0);

    let macos = required_binary_export_tool_symbols("macos").unwrap();
    assert_eq!(
        macos,
        [
            "_GetPluginFactory",
            "_bundleEntry",
            "_bundleExit",
            "_BundleEntry",
            "_BundleExit"
        ]
    );
    assert_eq!(
        required_binary_export_tool_symbols("windows-x64").unwrap(),
        ["GetPluginFactory", "InitDll", "ExitDll"]
    );
    assert_eq!(
        required_binary_export_tool_symbols("linux-x64").unwrap(),
        ["GetPluginFactory", "ModuleEntry", "ModuleExit"]
    );
    assert!(required_binary_export_tool_symbols("linux-x11").is_none());
    assert_eq!(binary_export_inspection_tool_plans().len(), 6);
    assert!(binary_export_inspection_tools("linux-x11").is_none());
    assert_eq!(
        binary_export_inspection_tools("macos")
            .unwrap()
            .iter()
            .map(BinaryExportInspectionToolPlan::display)
            .collect::<Vec<_>>(),
        vec!["nm -gU".to_string(), "llvm-nm -gU".to_string()]
    );
    assert_eq!(
        binary_export_inspection_tools("windows-x64")
            .unwrap()
            .iter()
            .map(BinaryExportInspectionToolPlan::display)
            .collect::<Vec<_>>(),
        vec![
            "llvm-objdump -p".to_string(),
            "dumpbin /exports".to_string()
        ]
    );
    assert_eq!(
        binary_export_inspection_tools("linux-x64")
            .unwrap()
            .iter()
            .map(BinaryExportInspectionToolPlan::display)
            .collect::<Vec<_>>(),
        vec![
            "nm -D --defined-only".to_string(),
            "llvm-nm -D --defined-only".to_string()
        ]
    );

    for platform in ["macos", "windows-x64", "linux-x64"] {
        let required = required_binary_export_tool_symbols(platform).unwrap();
        let tools = binary_export_inspection_tools(platform).unwrap();
        assert_eq!(
            tools,
            binary_export_inspection_tool_plans()
                .iter()
                .filter(|tool| tool.platform == platform)
                .copied()
                .collect::<Vec<_>>()
        );
        let planned = binary_export_symbol_plans()
            .iter()
            .filter(|plan| plan.platform == platform && plan.required)
            .map(|plan| plan.tool_symbol)
            .collect::<Vec<_>>();
        assert_eq!(planned, required);
        for symbol in required {
            let plan = binary_export_symbol_plan(platform, symbol)
                .expect("required symbol should have a public plan");
            assert_eq!(plan.platform, platform);
            assert_eq!(plan.tool_symbol, *symbol);
            assert!(plan.required);
            assert!(!plan.verified_by_generated_bindings);
        }
    }

    assert!(binary_export_required_symbols_present(
        "linux-x64",
        &["GetPluginFactory", "ModuleEntry", "ModuleExit"]
    ));
    assert_eq!(
        first_missing_binary_export_symbol("windows-x64", &["GetPluginFactory", "InitDll"]),
        Some("ExitDll")
    );
    assert!(!binary_export_required_symbols_present(
        "windows-x64",
        &["GetPluginFactory", "InitDll"]
    ));
    assert!(binary_export_symbol_plan("macos", "bundleEntry").is_none());
}

#[test]
fn generated_bindings_plan_reports_ready_inputs_without_claiming_generated_bindings() {
    let temp = create_sdk_root(&[]);

    let plan = generated_bindings_plan(temp.path(), "target/vst3-sdk/generated.rs").unwrap();

    assert_eq!(plan.version, GENERATED_BINDINGS_PLAN_VERSION);
    assert_eq!(plan.generator, GENERATED_BINDINGS_PLAN_GENERATOR);
    assert_eq!(plan.status, "ready-for-binding-generator");
    assert!(!plan.bindings_generated);
    assert!(plan.header_manifest.complete);
    assert!(plan.blockers.is_empty());
    assert!(plan.checks.iter().any(|check| {
        check.name == "binding emitter"
            && check.status == "reserved"
            && check.value.contains("not enabled yet")
    }));
}

#[test]
fn generated_bindings_plan_reports_missing_inputs_and_bad_output_path() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let plan = generated_bindings_plan(temp.path(), "target/vst3-sdk/generated.txt").unwrap();

    assert_eq!(plan.status, "blocked");
    assert!(!plan.bindings_generated);
    assert_eq!(plan.blockers.len(), 2);
    assert!(plan.blockers[0].contains("pluginterfaces/base/funknown.h"));
    assert!(plan.blockers[1].contains(".rs"));
    assert!(
        plan.checks
            .iter()
            .any(|check| { check.name == "bindings module path" && check.status == "failed" })
    );
}

#[test]
fn generated_bindings_surface_locks_expected_symbols_without_claiming_bindings() {
    let temp = create_sdk_root(&[]);

    let surface = generated_bindings_surface(temp.path()).unwrap();
    let repeated = generated_bindings_surface(temp.path()).unwrap();

    assert_eq!(surface, repeated);
    assert_eq!(surface.version, GENERATED_BINDINGS_SURFACE_VERSION);
    assert_eq!(surface.generator, GENERATED_BINDINGS_SURFACE_GENERATOR);
    assert_eq!(surface.status, "ready-for-binding-emitter");
    assert!(!surface.bindings_generated);
    assert!(surface.header_manifest.complete);
    assert!(surface.blockers.is_empty());
    assert_eq!(
        surface.required_headers.len(),
        REQUIRED_GENERATED_HEADER_INPUTS.len()
    );
    assert!(surface.symbols.iter().all(|symbol| symbol.header_present));
    assert!(surface.symbols.iter().all(|symbol| symbol.symbol_present));
    assert!(surface.missing_symbols.is_empty());
    assert!(surface.symbols.iter().any(|symbol| {
        symbol.name == "IPlugView" && symbol.header == "pluginterfaces/gui/iplugview.h"
    }));
    assert!(surface.symbols.iter().any(|symbol| {
        symbol.name == "IMidiMapping" && symbol.header == "pluginterfaces/vst/ivstmidicontrollers.h"
    }));
    assert!(surface.symbols.iter().any(|symbol| {
        symbol.name == "INoteExpressionController"
            && symbol.header == "pluginterfaces/vst/ivstnoteexpression.h"
    }));
    assert!(surface.symbols.iter().any(|symbol| {
        symbol.name == "IProgramListData" && symbol.header == "pluginterfaces/vst/ivstunits.h"
    }));
    assert!(
        surface
            .notes
            .iter()
            .any(|note| note.contains("does not parse C++ AST"))
    );
}

#[test]
fn generated_bindings_surface_reports_missing_symbol_tokens() {
    let temp = create_sdk_root(&[]);
    let root = temp.path();
    std::fs::write(
        root.join("pluginterfaces/vst/ivstmidicontrollers.h"),
        "/* header exists but target token is absent */\nIMidiMappingExtra\n",
    )
    .unwrap();

    let surface = generated_bindings_surface(root).unwrap();

    assert_eq!(surface.status, "blocked");
    assert!(!surface.bindings_generated);
    assert!(
        surface
            .missing_symbols
            .iter()
            .any(|entry| entry.contains("IMidiMapping"))
    );
    assert!(
        surface
            .blockers
            .iter()
            .any(|blocker| blocker.contains("absent from their locked headers"))
    );
    assert!(surface.symbols.iter().any(|symbol| {
        symbol.name == "IMidiMapping" && symbol.header_present && !symbol.symbol_present
    }));
}

#[test]
fn generated_bindings_surface_reports_missing_symbol_headers() {
    let temp = create_sdk_root(&["pluginterfaces/gui/iplugview.h"]);

    let surface = generated_bindings_surface(temp.path()).unwrap();

    assert_eq!(surface.status, "blocked");
    assert!(!surface.bindings_generated);
    assert!(
        surface
            .blockers
            .iter()
            .any(|blocker| blocker.contains("pluginterfaces/gui/iplugview.h"))
    );
    assert!(
        surface
            .missing_headers
            .iter()
            .any(|entry| entry.contains("IPlugView"))
    );
}

#[test]
fn generated_bindings_scaffold_emits_deterministic_metadata_without_claiming_bindings() {
    let temp = create_sdk_root(&[]);

    let scaffold =
        generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.rs").unwrap();
    let repeated =
        generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.rs").unwrap();

    assert_eq!(scaffold.module, repeated.module);
    assert_eq!(scaffold.plan.status, "ready-for-binding-generator");
    assert_eq!(scaffold.surface.status, "ready-for-binding-emitter");
    assert!(!scaffold.plan.bindings_generated);
    assert!(!scaffold.surface.bindings_generated);
    assert!(
        scaffold
            .module
            .contains(GENERATED_BINDINGS_SCAFFOLD_GENERATOR)
    );
    assert!(
        scaffold
            .module
            .contains("pub const BINDINGS_GENERATED: bool = false;")
    );
    assert!(
        scaffold
            .module
            .contains("pub const STATUS: &str = \"metadata-scaffold\";")
    );
    assert!(
        scaffold
            .module
            .contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";")
    );
    assert!(
        scaffold
            .module
            .contains("pub const HEADER_INPUTS: &[HeaderInput] = &[")
    );
    assert!(
        scaffold
            .module
            .contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[")
    );
    assert!(scaffold.module.contains("IPlugView"));
    assert!(scaffold.module.contains("IMidiMapping"));
    assert!(scaffold.module.contains("pluginterfaces/base/funknown.h"));
    assert!(
        scaffold
            .module
            .contains("pub const MISSING_HEADERS: &[&str] = &[")
    );
    assert!(!scaffold.module.contains("BINDINGS_GENERATED: bool = true"));
}

#[test]
fn generated_bindings_scaffold_is_blocked_until_inputs_and_module_path_are_ready() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let error =
        generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.txt").unwrap_err();

    let text = error.to_string();
    assert!(text.contains("generated bindings scaffold is blocked"));
    assert!(text.contains("pluginterfaces/base/funknown.h"));
    assert!(text.contains(".rs"));
}

#[test]
fn generated_bindings_abi_seed_emits_deterministic_seed_without_claiming_full_bindings() {
    let temp = create_sdk_root(&[]);

    let seed =
        generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.rs").unwrap();
    let repeated =
        generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.rs").unwrap();

    assert_eq!(seed.module, repeated.module);
    assert_eq!(seed.plan.status, "ready-for-binding-generator");
    assert_eq!(seed.surface.status, "ready-for-binding-emitter");
    assert!(!seed.plan.bindings_generated);
    assert!(!seed.surface.bindings_generated);
    assert!(seed.module.contains(GENERATED_BINDINGS_ABI_SEED_GENERATOR));
    assert!(
        seed.module
            .contains("pub const STATUS: &str = \"abi-seed\";")
    );
    assert!(
        seed.module
            .contains("pub const ABI_SEED_GENERATED: bool = true;")
    );
    assert!(
        seed.module
            .contains("pub const BINDINGS_GENERATED: bool = false;")
    );
    assert!(
        seed.module
            .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
    );
    assert!(seed.module.contains("pub type TResult = i32;"));
    assert!(seed.module.contains("pub type ParamID = u32;"));
    assert!(seed.module.contains("pub type ParamValue = f64;"));
    assert!(seed.module.contains("pub type TChar = u16;"));
    assert!(
        seed.module
            .contains("pub type TUID = [std::os::raw::c_char; 16];")
    );
    assert!(seed.module.contains("pub const kResultOk: TResult = 0;"));
    assert!(
        seed.module
            .contains("pub const kInvalidArgument: TResult = 2;")
    );
    assert!(
        seed.module
            .contains("pub const kNotImplemented: TResult = 3;")
    );
    assert!(
        seed.module
            .contains("pub const kPlatformTypeHWND: PlatformType = \"HWND\";")
    );
    assert!(
        seed.module
            .contains("pub const kPlatformTypeNSView: PlatformType = \"NSView\";")
    );
    assert!(
        seed.module.contains(
            "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";"
        )
    );
    assert!(seed.module.contains("IPlugView"));
    assert!(seed.module.contains("IMidiMapping"));
    assert!(seed.module.contains("pluginterfaces/base/funknown.h"));
    assert!(
        !seed
            .module
            .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
    );
}

#[test]
fn generated_bindings_abi_seed_is_blocked_until_inputs_and_module_path_are_ready() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let error = generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.txt")
        .unwrap_err();

    let text = error.to_string();
    assert!(text.contains("generated bindings scaffold is blocked"));
    assert!(text.contains("pluginterfaces/base/funknown.h"));
    assert!(text.contains(".rs"));
}

#[test]
fn generated_bindings_abi_emits_deterministic_compileable_layout_without_claiming_full_bindings() {
    let temp = create_sdk_root(&[]);

    let abi = generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.rs").unwrap();
    let repeated = generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.rs").unwrap();

    assert_eq!(abi.module, repeated.module);
    assert_eq!(abi.plan.status, "ready-for-binding-generator");
    assert_eq!(abi.surface.status, "ready-for-binding-emitter");
    assert!(!abi.plan.bindings_generated);
    assert!(!abi.surface.bindings_generated);
    assert!(abi.module.contains(GENERATED_BINDINGS_ABI_GENERATOR));
    assert!(
        abi.module
            .contains("pub const STATUS: &str = \"abi-layout\";")
    );
    assert!(
        abi.module
            .contains("pub const ABI_LAYOUT_GENERATED: bool = true;")
    );
    assert!(
        abi.module
            .contains("pub const BINDINGS_GENERATED: bool = false;")
    );
    assert!(
        abi.module
            .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
    );
    assert!(abi.module.contains("#[repr(C)]"));
    assert!(abi.module.contains("pub struct TUID"));
    assert!(abi.module.contains("pub struct FUnknownVTable"));
    assert!(abi.module.contains("pub struct FUnknown"));
    assert!(abi.module.contains("pub struct ViewRect"));
    assert!(
        abi.module
            .contains("pub type String128 = [TChar; STRING128_CODE_UNITS];")
    );
    assert!(abi.module.contains("pub type UnitID = int32;"));
    assert!(abi.module.contains("pub type ProgramListID = int32;"));
    assert!(
        abi.module
            .contains("pub type NoteExpressionTypeID = uint32;")
    );
    assert!(abi.module.contains("pub type NoteExpressionValue = f64;"));
    assert!(abi.module.contains("pub type PhysicalUITypeID = uint32;"));
    assert!(abi.module.contains("pub struct ProgramListInfo"));
    assert!(abi.module.contains("pub struct UnitInfo"));
    assert!(
        abi.module
            .contains("pub struct NoteExpressionValueDescription")
    );
    assert!(abi.module.contains("pub struct NoteExpressionTypeInfo"));
    assert!(abi.module.contains("pub struct PhysicalUIMap"));
    assert!(abi.module.contains("pub struct PhysicalUIMapList"));
    assert!(abi.module.contains("pub const kRootUnitId: UnitID = 0;"));
    assert!(
        abi.module
            .contains("pub const kNoProgramListId: ProgramListID = -1;")
    );
    assert!(
        abi.module
            .contains("pub type FUnknownQueryInterface = unsafe extern \"system\" fn(")
    );
    assert!(abi.module.contains("pub type ParamID = u32;"));
    assert!(abi.module.contains("pub type ParamValue = f64;"));
    assert!(abi.module.contains("pub type Sample32 = f32;"));
    assert!(abi.module.contains("pub type Sample64 = f64;"));
    assert!(
        abi.module
            .contains("pub const ABI_LAYOUT_TYPES: &[&str] = &[")
    );
    assert!(
        abi.module
            .contains("pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &[")
    );
    assert!(
        abi.module.contains(
            "type_name: \"ProgramListInfo\", size: std::mem::size_of::<ProgramListInfo>()"
        )
    );
    assert!(abi.module.contains(
        "type_name: \"NoteExpressionTypeInfo\", size: std::mem::size_of::<NoteExpressionTypeInfo>()"
    ));
    assert!(
        abi.module
            .contains("pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &[")
    );
    assert!(
        abi.module
            .contains("owner: \"ProgramListInfo\", field: \"programCount\", offset: std::mem::offset_of!(ProgramListInfo, programCount)")
    );
    assert!(
        abi.module
            .contains("owner: \"NoteExpressionTypeInfo\", field: \"valueDesc\", offset: std::mem::offset_of!(NoteExpressionTypeInfo, valueDesc)")
    );
    assert!(
        abi.module
            .contains("owner: \"PhysicalUIMapList\", field: \"map\", offset: std::mem::offset_of!(PhysicalUIMapList, map)")
    );
    assert!(abi.module.contains("IPlugView"));
    assert!(abi.module.contains("IMidiMapping"));
    assert!(abi.module.contains("pluginterfaces/base/funknown.h"));
    assert!(
        !abi.module
            .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
    );

    let module_path = temp.path().join("generated_abi.rs");
    std::fs::write(&module_path, &abi.module).unwrap();
    let output_path = temp.path().join("libgenerated_abi.rlib");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(&rustc)
        .arg("--edition=2021")
        .arg("--crate-type")
        .arg("lib")
        .arg(&module_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated ABI layout module should compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_bindings_abi_is_blocked_until_inputs_and_module_path_are_ready() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let error =
        generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.txt").unwrap_err();

    let text = error.to_string();
    assert!(text.contains("generated bindings scaffold is blocked"));
    assert!(text.contains("pluginterfaces/base/funknown.h"));
    assert!(text.contains(".rs"));
}

#[test]
fn generated_bindings_interface_skeleton_emits_deterministic_compileable_skeleton_without_claiming_full_bindings()
 {
    let temp = create_sdk_root(&[]);

    let skeleton = generated_bindings_interface_skeleton(
        temp.path(),
        "target/vst3-sdk/generated-interface-skeleton.rs",
    )
    .unwrap();
    let repeated = generated_bindings_interface_skeleton(
        temp.path(),
        "target/vst3-sdk/generated-interface-skeleton.rs",
    )
    .unwrap();

    assert_eq!(skeleton.module, repeated.module);
    assert_eq!(skeleton.plan.status, "ready-for-binding-generator");
    assert_eq!(skeleton.surface.status, "ready-for-binding-emitter");
    assert!(!skeleton.plan.bindings_generated);
    assert!(!skeleton.surface.bindings_generated);
    assert!(
        skeleton
            .module
            .contains(GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR)
    );
    assert!(
        skeleton
            .module
            .contains("pub const STATUS: &str = \"interface-skeleton\";")
    );
    assert!(
        skeleton
            .module
            .contains("pub const INTERFACE_SKELETON_GENERATED: bool = true;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const BINDINGS_GENERATED: bool = false;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
    );
    assert!(skeleton.module.contains("pub struct FUnknownVTable"));
    assert!(skeleton.module.contains("pub struct FUnknown"));
    assert!(skeleton.module.contains("pub struct IPlugViewVTable"));
    assert!(skeleton.module.contains("pub struct IPlugView"));
    assert!(skeleton.module.contains("pub struct IEditControllerVTable"));
    assert!(skeleton.module.contains("pub struct IMidiMappingVTable"));
    assert!(skeleton.module.contains("pub struct IUnitInfoVTable"));
    assert!(
        skeleton
            .module
            .contains("pub struct IProgramListDataVTable")
    );
    assert!(
        skeleton
            .module
            .contains("pub struct INoteExpressionControllerVTable")
    );
    assert!(skeleton.module.contains("pub struct InterfaceMethod"));
    assert!(skeleton.module.contains("pub struct InterfaceVTableSlot"));
    assert!(skeleton.module.contains("pub struct InterfaceCallbackType"));
    assert!(
        skeleton
            .module
            .contains("pub struct InterfaceVTableFieldOffset")
    );
    assert!(skeleton.module.contains("pub struct ComObjectInterface"));
    assert!(skeleton.module.contains("pub struct ComObjectIdentityPlan"));
    assert!(
        skeleton
            .module
            .contains("pub struct ComObjectQueryInterfaceDispatchEntry")
    );
    assert!(skeleton.module.contains("pub struct FactoryExportPlan"));
    assert!(skeleton.module.contains("pub struct FactoryClassPlan"));
    assert!(skeleton.module.contains("pub struct ModuleExportPlan"));
    assert!(
        skeleton
            .module
            .contains("pub struct BinaryExportSymbolPlan")
    );
    assert!(
        skeleton
            .module
            .contains("pub struct BinaryExportInspectionToolPlan")
    );
    assert!(skeleton.module.contains("pub slot: usize,"));
    assert!(skeleton.module.contains("pub signature: &'static str,"));
    assert!(skeleton.module.contains("pub local_slot: usize,"));
    assert!(skeleton.module.contains("pub global_slot: usize,"));
    assert!(skeleton.module.contains("pub callback_type: &'static str,"));
    assert!(skeleton.module.contains("pub object: &'static str,"));
    assert!(skeleton.module.contains("pub required: bool,"));
    assert!(
        skeleton
            .module
            .contains("pub root_interface: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub root_iid_const: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub funknown_identity: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub unknown_iid_result: &'static str,")
    );
    assert!(skeleton.module.contains("pub returns_same_identity: bool,"));
    assert!(skeleton.module.contains("pub add_ref_on_success: bool,"));
    assert!(
        skeleton
            .module
            .contains("pub factory_object: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub factory_iid_const: &'static str,")
    );
    assert!(skeleton.module.contains("pub class_count: usize,"));
    assert!(skeleton.module.contains("pub class_kind: &'static str,"));
    assert!(skeleton.module.contains("pub class_index: usize,"));
    assert!(skeleton.module.contains("pub cid_policy: &'static str,"));
    assert!(
        skeleton
            .module
            .contains("pub create_instance_root_iid_const: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub requested_iid_dispatch: &'static str,")
    );
    assert!(skeleton.module.contains("pub symbol: &'static str,"));
    assert!(skeleton.module.contains("pub platforms: &'static str,"));
    assert!(skeleton.module.contains("pub generated_callable: bool,"));
    assert!(skeleton.module.contains("pub binary_format: &'static str,"));
    assert!(skeleton.module.contains("pub tool_symbol: &'static str,"));
    assert!(
        skeleton
            .module
            .contains("pub inspection_tool: &'static str,")
    );
    assert!(
        skeleton
            .module
            .contains("pub verified_by_generated_bindings: bool,")
    );
    assert!(skeleton.module.contains("pub program: &'static str,"));
    assert!(
        skeleton
            .module
            .contains("pub args: &'static [&'static str],")
    );
    assert!(
        skeleton.module.contains(
            "pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";"
        )
    );
    assert!(skeleton.module.contains(
        "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const QUERY_INTERFACE_IID_LOOKUP_SCOPE: &str = \"pure-iid-dispatch-lookup-seed-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const BINARY_EXPORT_SYMBOL_PLAN_SCOPE: &str = \"vesty-binary-export-symbol-plan-audit\";"
    ));
    assert!(skeleton.module.contains(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";"
    ));
    assert!(
        skeleton
            .module
            .contains("pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const FACTORY_CLASS_PLAN_COUNT: usize = 2;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const MODULE_EXPORT_PLAN_COUNT: usize = 9;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = 11;")
    );
    assert!(
        skeleton
            .module
            .contains("pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = 6;")
    );
    assert!(skeleton.module.contains("pub const kNoInterface: TResult"));
    assert!(
        skeleton
            .module
            .contains("pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
    );
    assert!(
        skeleton.module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        )
    );
    assert!(
        skeleton
            .module
            .contains("pub fn interface_vtable_field_offset_by_interface_and_field(")
    );
    assert!(
        skeleton
            .module
            .contains("pub const INTERFACE_METHODS: &[InterfaceMethod] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn interface_vtable_slot_by_interface_and_method(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn interface_vtable_slot_by_interface_and_global_slot(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn interface_id_for_iid(iid: &TUID) -> Option<&'static InterfaceId>")
    );
    assert!(skeleton.module.contains(
        "pub fn query_interface_entry_by_interface(interface: &str) -> Option<&'static QueryInterfaceEntry>"
    ));
    assert!(skeleton.module.contains(
        "pub fn query_interface_entry_for_iid(iid: &TUID) -> Option<&'static QueryInterfaceEntry>"
    ));
    assert!(
        skeleton
            .module
            .contains("pub const COM_OBJECTS: &[&str] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[")
    );
    assert!(skeleton.module.contains(
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &["
    ));
    assert!(
        skeleton
            .module
            .contains("pub fn com_object_query_interface_dispatch_by_interface(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn com_object_query_interface_dispatch_for_iid(")
    );
    assert!(
        skeleton
            .module
            .contains("pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const VESTYPROCESSOR_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const VESTYCONTROLLER_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const GETPLUGINFACTORY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const WINDOWS_INITDLL_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const LINUX_MODULEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
    );
    assert!(
        skeleton
            .module
            .contains("pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &[")
    );
    assert!(skeleton.module.contains(
        "pub const WINDOWS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
    ));
    assert!(skeleton.module.contains(
        "pub const MACOS_BUNDLEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
    ));
    assert!(skeleton.module.contains(
        "pub const LINUX_MODULEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
    ));
    assert!(
        skeleton
            .module
            .contains("pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[")
    );
    assert!(skeleton.module.contains(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &["
    ));
    assert!(
        skeleton
            .module
            .contains("pub fn binary_export_inspection_tools(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn binary_export_symbol_plan_by_platform_and_symbol(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn required_binary_export_symbol_count(platform: &str) -> usize")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn first_missing_binary_export_symbol(")
    );
    assert!(
        skeleton
            .module
            .contains("pub fn binary_export_required_symbols_present(")
    );
    assert!(
        skeleton
            .module
            .contains("pub const VESTYPROCESSOR_INTERFACES: &[ComObjectInterface] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const VESTYCONTROLLER_INTERFACES: &[ComObjectInterface] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const VESTYPROCESSOR_IDENTITY_PLAN: ComObjectIdentityPlan = ")
    );
    assert!(skeleton.module.contains(
        "pub const VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH: &[ComObjectQueryInterfaceDispatchEntry] = &["
    ));
    assert!(
        skeleton
            .module
            .contains("pub const IAUDIOPROCESSOR_METHODS: &[InterfaceMethod] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const IAUDIOPROCESSOR_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const IAUDIOPROCESSOR_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
    );
    assert!(skeleton.module.contains(
        "pub const IAUDIOPROCESSOR_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
    ));
    assert!(
        skeleton
            .module
            .contains("pub type IAudioProcessorProcess = unsafe extern \"system\" fn(")
    );
    assert!(
        skeleton
            .module
            .contains("pub process: IAudioProcessorProcess,")
    );
    assert!(
        skeleton
            .module
            .contains("offset: std::mem::offset_of!(IAudioProcessorVTable, process)")
    );
    assert!(skeleton.module.contains("slot: 6, interface: \"IAudioProcessor\", name: \"process\", purpose: \"realtime audio/MIDI/process callback\", realtime: true, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
    assert!(skeleton.module.contains("local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
    assert!(skeleton.module.contains("InterfaceCallbackType { interface: \"IAudioProcessor\", method: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
    assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
    assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IProcessContextRequirements\", iid_const: \"IPROCESSCONTEXTREQUIREMENTS_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
    assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyController\", interface: \"IEditController\", iid_const: \"IEDITCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
    assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyPlugView\", interface: \"IPlugView\", iid_const: \"IPLUGVIEW_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
    assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
    assert!(skeleton.module.contains("ComObjectIdentityPlan { object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }"));
    assert!(skeleton.module.contains("ComObjectIdentityPlan { object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }"));
    assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"FUnknown\", iid_const: \"FUNKNOWN_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
    assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
    assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", root_interface: \"IEditController\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
    assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", root_interface: \"IPluginFactory\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
    assert!(skeleton.module.contains("FactoryExportPlan { factory_object: \"VestyFactory\", factory_interface: \"IPluginFactory\", factory_iid_const: \"IPLUGINFACTORY_IID\", class_count: 2, count_classes_result: \"2\", get_factory_info_source: \"PluginInfo vendor/url/email + kUnicode\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
    assert!(skeleton.module.contains("FactoryClassPlan { class_kind: \"processor\", class_index: 0, class_object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", category: \"Audio Module Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id\", cid_policy: \"processor-cid-is-plugin-class-id\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyProcessor\", create_instance_root_interface: \"IComponent\", create_instance_root_iid_const: \"ICOMPONENT_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
    assert!(skeleton.module.contains("FactoryClassPlan { class_kind: \"controller\", class_index: 1, class_object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", category: \"Component Controller Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id[15].wrapping_add(1)\", cid_policy: \"controller-cid-last-byte-wrapping-add-1\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyController\", create_instance_root_interface: \"IEditController\", create_instance_root_iid_const: \"IEDITCONTROLLER_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
    assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"GetPluginFactory\", platforms: \"windows,macos,linux\", signature: \"extern \\\"system\\\" fn() -> *mut IPluginFactory\", purpose: \"return VST3 plugin factory pointer\", implementation: \"vesty_vst3::create_plugin_factory::<Plugin>()\", return_policy: \"returns owned COM factory pointer for host discovery\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"bundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS VST3 bundle initialization entry\", implementation: \"vesty_vst3::set_macos_bundle_ref(bundle_ref); return true\", return_policy: \"bundle resources path is captured when possible\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"ModuleEntry\", platforms: \"linux\", signature: \"extern \\\"system\\\" fn(library_handle: *mut c_void) -> bool\", purpose: \"Linux VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"GetPluginFactory\", tool_symbol: \"GetPluginFactory\", inspection_tool: \"dumpbin /exports or llvm-objdump -p\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"macos\", binary_format: \"Mach-O\", symbol: \"bundleEntry\", tool_symbol: \"_bundleEntry\", inspection_tool: \"nm -gU or llvm-nm -gU\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"linux-x64\", binary_format: \"ELF\", symbol: \"ModuleEntry\", tool_symbol: \"ModuleEntry\", inspection_tool: \"nm -D --defined-only or llvm-nm -D --defined-only\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
    assert!(skeleton.module.contains(
        "BinaryExportInspectionToolPlan { platform: \"macos\", program: \"nm\", args: &[\"-gU\"] }"
    ));
    assert!(skeleton.module.contains("BinaryExportInspectionToolPlan { platform: \"windows-x64\", program: \"llvm-objdump\", args: &[\"-p\"] }"));
    assert!(skeleton.module.contains("BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-nm\", args: &[\"-D\", \"--defined-only\"] }"));
    assert!(
        skeleton
            .module
            .contains("pub const IUNITINFO_METHODS: &[InterfaceMethod] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const IUNITINFO_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub const IUNITINFO_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
    );
    assert!(
        skeleton.module.contains(
            "pub const IUNITINFO_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        )
    );
    assert!(
        skeleton
            .module
            .contains("pub type IUnitInfoGetProgramListInfo = unsafe extern \"system\" fn(")
    );
    assert!(
        skeleton
            .module
            .contains("pub getProgramListInfo: IUnitInfoGetProgramListInfo,")
    );
    assert!(
        skeleton
            .module
            .contains("offset: std::mem::offset_of!(IUnitInfoVTable, getProgramListInfo)")
    );
    assert!(skeleton.module.contains("name: \"getProgramListInfo\", purpose: \"return program-list metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
    assert!(skeleton.module.contains("local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
    assert!(
        skeleton
            .module
            .contains("pub const IPROGRAMLISTDATA_METHODS: &[InterfaceMethod] = &[")
    );
    assert!(
        skeleton
            .module
            .contains("pub type IProgramListDataGetProgramData = unsafe extern \"system\" fn(")
    );
    assert!(
        skeleton
            .module
            .contains("pub getProgramData: IProgramListDataGetProgramData,")
    );
    assert!(
        skeleton
            .module
            .contains("offset: std::mem::offset_of!(IProgramListDataVTable, getProgramData)")
    );
    assert!(skeleton.module.contains("name: \"getProgramData\", purpose: \"save program data to stream\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
    assert!(skeleton.module.contains("local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
    assert!(
        skeleton
            .module
            .contains("pub const INOTEEXPRESSIONCONTROLLER_METHODS: &[InterfaceMethod] = &[")
    );
    assert!(skeleton.module.contains(
        "pub type INoteExpressionControllerGetNoteExpressionInfo = unsafe extern \"system\" fn("
    ));
    assert!(
        skeleton
            .module
            .contains("pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,")
    );
    assert!(skeleton.module.contains(
        "offset: std::mem::offset_of!(INoteExpressionControllerVTable, getNoteExpressionInfo)"
    ));
    assert!(skeleton.module.contains("name: \"getNoteExpressionInfo\", purpose: \"return Note Expression type metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
    assert!(skeleton.module.contains("local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
    assert!(
        skeleton
            .module
            .contains("pub const INTERFACE_SKELETON_TYPES: &[&str] = &[")
    );
    assert!(skeleton.module.contains("pluginterfaces/base/funknown.h"));
    assert!(
        skeleton
            .module
            .contains("pluginterfaces/vst/ivstmidicontrollers.h")
    );
    assert!(
        skeleton
            .module
            .contains("pluginterfaces/vst/ivstnoteexpression.h")
    );
    assert!(skeleton.module.contains("pluginterfaces/vst/ivstunits.h"));
    assert!(
        !skeleton
            .module
            .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
    );

    let module_path = temp.path().join("generated_interface_skeleton.rs");
    std::fs::write(&module_path, &skeleton.module).unwrap();
    let output_path = temp.path().join("libgenerated_interface_skeleton.rlib");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(&rustc)
        .arg("--edition=2021")
        .arg("--crate-type")
        .arg("lib")
        .arg(&module_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated interface skeleton module should compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let helper_check_path = temp.path().join("generated_interface_skeleton_check.rs");
    std::fs::write(
        &helper_check_path,
        r#"
#[path = "generated_interface_skeleton.rs"]
mod generated_interface_skeleton;

use generated_interface_skeleton::*;

fn main() {
let interface = interface_id_for_iid(&IAUDIOPROCESSOR_IID)
    .expect("IAudioProcessor IID should resolve");
assert_eq!(interface.interface, "IAudioProcessor");

let query = query_interface_entry_for_iid(&IAUDIOPROCESSOR_IID)
    .expect("IAudioProcessor queryInterface entry should resolve");
assert_eq!(query.interface, "IAudioProcessor");
assert!(query.inherits_funknown);

let dispatch =
    com_object_query_interface_dispatch_for_iid("VestyProcessor", &IAUDIOPROCESSOR_IID)
        .expect("VestyProcessor should expose IAudioProcessor");
assert_eq!(dispatch.object, "VestyProcessor");
assert_eq!(dispatch.interface, "IAudioProcessor");
assert!(dispatch.returns_same_identity);
assert_eq!(dispatch.success_result, "kResultOk");

assert!(
    com_object_query_interface_dispatch_for_iid("VestyPlugView", &IAUDIOPROCESSOR_IID)
        .is_none()
);
assert!(query_interface_entry_for_iid(&TUID::ZERO).is_none());

let process_slot = INTERFACE_VTABLE_SLOTS
    .iter()
    .find(|slot| slot.interface == "IAudioProcessor" && slot.method == "process")
    .expect("IAudioProcessor process vtable slot should be emitted");
assert_eq!(process_slot.local_slot, 6);
assert_eq!(process_slot.global_slot, 9);
let process_slot_by_method =
    interface_vtable_slot_by_interface_and_method("IAudioProcessor", "process")
        .expect("IAudioProcessor process slot lookup should resolve");
assert_eq!(process_slot_by_method.global_slot, 9);
let process_slot_by_global =
    interface_vtable_slot_by_interface_and_global_slot("IAudioProcessor", 9)
        .expect("IAudioProcessor global slot lookup should resolve");
assert_eq!(process_slot_by_global.method, "process");
assert!(
    interface_vtable_slot_by_interface_and_global_slot("IAudioProcessor", 99).is_none()
);
let process_offset =
    interface_vtable_field_offset_by_interface_and_field("IAudioProcessor", "process")
        .expect("IAudioProcessor process field offset lookup should resolve");
assert_eq!(process_offset.callback_type, "IAudioProcessorProcess");
assert_eq!(
    process_offset.offset,
    std::mem::offset_of!(IAudioProcessorVTable, process)
);
assert!(
    interface_vtable_field_offset_by_interface_and_field("IAudioProcessor", "missing")
        .is_none()
);

let release_slot = FUNKNOWN_VTABLE_SLOTS
    .iter()
    .find(|slot| slot.method == "release")
    .expect("FUnknown release slot should be emitted");
assert_eq!(release_slot.local_slot, 2);
assert_eq!(release_slot.global_slot, 2);

let macos_bundle_entry =
    binary_export_symbol_plan_by_platform_and_symbol("macos", "_bundleEntry")
        .expect("macOS bundleEntry binary export plan should resolve");
assert_eq!(macos_bundle_entry.symbol, "bundleEntry");
assert_eq!(macos_bundle_entry.binary_format, "Mach-O");
assert_eq!(required_binary_export_symbol_count("macos"), 5);
assert_eq!(required_binary_export_symbol_count("windows-x64"), 3);
assert_eq!(required_binary_export_symbol_count("linux-x64"), 3);
assert_eq!(required_binary_export_symbol_count("wayland"), 0);
assert!(binary_export_required_symbols_present(
    "linux-x64",
    &["GetPluginFactory", "ModuleEntry", "ModuleExit"]
));
assert!(binary_export_required_symbols_present(
    "macos",
    &[
        "_GetPluginFactory",
        "_bundleEntry",
        "_bundleExit",
        "_BundleEntry",
        "_BundleExit"
    ]
));
assert_eq!(
    first_missing_binary_export_symbol("windows-x64", &["GetPluginFactory", "InitDll"]),
    Some("ExitDll")
);
assert!(
    !binary_export_required_symbols_present("windows-x64", &["GetPluginFactory", "InitDll"])
);
assert!(
    binary_export_symbol_plan_by_platform_and_symbol("macos", "bundleEntry").is_none()
);
}
"#,
    )
    .unwrap();
    let helper_binary = if cfg!(windows) {
        "generated_interface_skeleton_check.exe"
    } else {
        "generated_interface_skeleton_check"
    };
    let helper_output_path = temp.path().join(helper_binary);
    let output = std::process::Command::new(&rustc)
        .arg("--edition=2021")
        .arg(&helper_check_path)
        .arg("-o")
        .arg(&helper_output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated interface skeleton helper check should compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(&helper_output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated interface skeleton helper check should run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_bindings_interface_skeleton_is_blocked_until_inputs_and_module_path_are_ready() {
    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

    let error = generated_bindings_interface_skeleton(
        temp.path(),
        "target/vst3-sdk/generated-interface-skeleton.txt",
    )
    .unwrap_err();

    let text = error.to_string();
    assert!(text.contains("generated bindings scaffold is blocked"));
    assert!(text.contains("pluginterfaces/base/funknown.h"));
    assert!(text.contains(".rs"));
}

#[cfg(unix)]
#[test]
fn sdk_header_manifest_rejects_symlink_inputs() {
    use std::os::unix::fs::symlink;

    let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);
    let target = temp.path().join("README.md");
    symlink(target, temp.path().join("pluginterfaces/base/funknown.h")).unwrap();

    let error = sdk_header_input_manifest(temp.path()).unwrap_err();

    assert!(error.to_string().contains("not a symlink"));
}
