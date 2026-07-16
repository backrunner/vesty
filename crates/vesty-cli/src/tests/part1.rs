    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    #[test]
    fn output_format_accepts_text_and_json_aliases() {
        assert_eq!(parse_output_format("text").unwrap(), OutputFormat::Text);
        assert_eq!(parse_output_format("plain").unwrap(), OutputFormat::Text);
        assert_eq!(parse_output_format("json").unwrap(), OutputFormat::Json);
        assert!(parse_output_format("xml").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn toml_and_json_read_helpers_reject_symlink_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let external_toml = root.join("external.toml");
        let toml_link = root.join("input.toml");
        fs::write(&external_toml, "name = \"vesty\"\n").unwrap();
        unix_fs::symlink(&external_toml, &toml_link).unwrap();
        let error = read_toml_file(&toml_link).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("TOML input must not be a symlink")
        );

        let external_json = root.join("external.json");
        let json_link = root.join("input.json");
        fs::write(&external_json, r#"{"name":"vesty"}"#).unwrap();
        unix_fs::symlink(&external_json, &json_link).unwrap();
        let error = read_json_file(&json_link).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("JSON input must not be a symlink")
        );
    }

    #[test]
    fn vst3_sdk_manifest_command_writes_and_checks_header_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out = Utf8PathBuf::from_path_buf(temp.path().join("sdk-header-manifest.json")).unwrap();

        run_vst3_sdk_manifest(Some(sdk.clone()), Some(out.clone()), false, "json").unwrap();

        let manifest: vesty_vst3_sys::SdkHeaderInputManifest =
            serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert!(manifest.complete);
        assert_eq!(
            manifest.headers.len(),
            vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS.len()
        );
        assert_eq!(manifest.steinberg_sdk_baseline, "v3.8.0_build_66");
        assert!(manifest.header("pluginterfaces/base/funknown.h").is_some());

        run_vst3_sdk_manifest(Some(sdk.clone()), Some(out.clone()), true, "text").unwrap();

        fs::write(sdk.join("pluginterfaces/base/funknown.h"), "changed\n").unwrap();
        let error = run_vst3_sdk_manifest(Some(sdk), Some(out), true, "text").unwrap_err();
        assert!(error.to_string().contains("SDK header manifest drift"));
        assert!(error.to_string().contains("pluginterfaces/base/funknown.h"));
    }

    #[test]
    fn vst3_sdk_manifest_command_requires_sdk_dir() {
        let error = resolve_vst3_sdk_dir_from_env_value(None, None).unwrap_err();

        assert!(error.to_string().contains("pass --sdk-dir"));
        assert!(error.to_string().contains(vesty_vst3_sys::VST3_SDK_DIR_ENV));
    }

    #[test]
    fn vst3_sdk_manifest_command_reports_missing_headers() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(
            temp.path().join("sdk"),
            &["pluginterfaces/vst/ivstmessage.h"],
        );

        let manifest = vesty_vst3_sys::sdk_header_input_manifest(&sdk).unwrap();

        assert!(!manifest.complete);
        assert_eq!(
            manifest.missing_headers,
            vec!["pluginterfaces/vst/ivstmessage.h"]
        );
    }

    #[test]
    fn vst3_sdk_manifest_release_check_is_optional_but_strict_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let manifest_path = root.join("vst3-sdk-headers.json");
        write_test_vst3_sdk_manifest(&manifest_path, &[]);

        let skipped = vst3_sdk_manifest_release_check(None);
        assert_eq!(skipped.status, "skipped");

        let ok = vst3_sdk_manifest_release_check(Some(&manifest_path));
        assert_eq!(ok.status, "ok");
        assert!(ok.value.contains("v3.8.0_build_66"));
        assert!(
            ok.value.contains(
                &vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
                    .len()
                    .to_string()
            )
        );

        let mut manifest: vesty_vst3_sys::SdkHeaderInputManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest.headers[0].sha256 = "ABC".to_string();
        let invalid_path = root.join("bad-vst3-sdk-headers.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let failed = vst3_sdk_manifest_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(failed.value.contains("invalid sha256"));
    }

    #[test]
    fn vst3_sdk_manifest_release_check_rejects_incomplete_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let manifest_path = root.join("incomplete-vst3-sdk-headers.json");
        write_test_vst3_sdk_manifest(&manifest_path, &["pluginterfaces/vst/ivstmessage.h"]);

        let check = vst3_sdk_manifest_release_check(Some(&manifest_path));

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("manifest is incomplete"));
        assert!(check.value.contains("pluginterfaces/vst/ivstmessage.h"));
    }

    #[test]
    fn vst3_sdk_json_artifacts_reject_malformed_shape_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let manifest_path = root.join("vst3-sdk-headers.json");
        write_test_vst3_sdk_manifest(&manifest_path, &[]);
        let manifest: vesty_vst3_sys::SdkHeaderInputManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        validate_vst3_sdk_header_manifest_shape(&manifest).unwrap();

        let mut bad_manifest = manifest.clone();
        bad_manifest.version_hint = Some("VST3 SDK\u{202E}".to_string());
        let error = validate_vst3_sdk_header_manifest_shape(&bad_manifest)
            .expect_err("unsafe Unicode in SDK manifest should be rejected")
            .to_string();
        assert!(error.contains("unsafe Unicode"));

        let mut unknown_manifest_field = serde_json::to_value(&manifest).unwrap();
        unknown_manifest_field["generatedBy"] = serde_json::json!("forged");
        let error = serde_json::from_value::<vesty_vst3_sys::SdkHeaderInputManifest>(
            unknown_manifest_field,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_header_field = serde_json::to_value(&manifest).unwrap();
        unknown_header_field["headers"][0]["checksum"] = serde_json::json!("forged");
        let error =
            serde_json::from_value::<vesty_vst3_sys::SdkHeaderInputManifest>(unknown_header_field)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));

        let mut duplicate_header = manifest.clone();
        duplicate_header
            .headers
            .push(duplicate_header.headers[0].clone());
        let error = validate_vst3_sdk_header_manifest_shape(&duplicate_header)
            .expect_err("duplicate SDK manifest header should be rejected")
            .to_string();
        assert!(error.contains("duplicate VST3 SDK manifest header"));

        let mut too_many_headers = manifest.clone();
        too_many_headers.headers = vec![manifest.headers[0].clone(); VST3_SDK_MAX_HEADERS + 1];
        let error = validate_vst3_sdk_header_manifest_shape(&too_many_headers)
            .expect_err("oversized SDK manifest should be rejected")
            .to_string();
        assert!(error.contains("too many headers"));

        let plan_path = root.join("generated-bindings-plan.json");
        write_test_vst3_sdk_binding_plan(
            &plan_path,
            &[],
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let plan: vesty_vst3_sys::GeneratedBindingsPlan =
            serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
        validate_vst3_sdk_binding_plan_shape(&plan).unwrap();

        let mut duplicate_check = plan.clone();
        duplicate_check
            .checks
            .push(duplicate_check.checks[0].clone());
        let error = validate_vst3_sdk_binding_plan_shape(&duplicate_check)
            .expect_err("duplicate binding plan check should be rejected")
            .to_string();
        assert!(error.contains("duplicate VST3 SDK binding plan check"));

        let mut unknown_plan_field = serde_json::to_value(&plan).unwrap();
        unknown_plan_field["generatedBy"] = serde_json::json!("forged");
        let error =
            serde_json::from_value::<vesty_vst3_sys::GeneratedBindingsPlan>(unknown_plan_field)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_plan_check_field = serde_json::to_value(&plan).unwrap();
        unknown_plan_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<vesty_vst3_sys::GeneratedBindingsPlan>(
            unknown_plan_check_field,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut empty_next_steps = plan.clone();
        empty_next_steps.next_steps.clear();
        let error = validate_vst3_sdk_binding_plan_shape(&empty_next_steps)
            .expect_err("empty binding plan next steps should be rejected")
            .to_string();
        assert!(error.contains("next step list must not be empty"));

        let surface_path = root.join("generated-bindings-surface.json");
        write_test_vst3_sdk_binding_surface(&surface_path, &[]);
        let surface: vesty_vst3_sys::GeneratedBindingsSurface =
            serde_json::from_str(&fs::read_to_string(&surface_path).unwrap()).unwrap();
        validate_vst3_sdk_binding_surface_shape(&surface).unwrap();

        let mut duplicate_symbol = surface.clone();
        duplicate_symbol
            .symbols
            .push(duplicate_symbol.symbols[0].clone());
        let error = validate_vst3_sdk_binding_surface_shape(&duplicate_symbol)
            .expect_err("duplicate binding surface symbol should be rejected")
            .to_string();
        assert!(error.contains("duplicate VST3 SDK binding surface symbol"));

        let mut unknown_surface_field = serde_json::to_value(&surface).unwrap();
        unknown_surface_field["generatedBy"] = serde_json::json!("forged");
        let error = serde_json::from_value::<vesty_vst3_sys::GeneratedBindingsSurface>(
            unknown_surface_field,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_surface_symbol_field = serde_json::to_value(&surface).unwrap();
        unknown_surface_symbol_field["symbols"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<vesty_vst3_sys::GeneratedBindingsSurface>(
            unknown_surface_symbol_field,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut empty_notes = surface.clone();
        empty_notes.notes.clear();
        let error = validate_vst3_sdk_binding_surface_shape(&empty_notes)
            .expect_err("empty binding surface notes should be rejected")
            .to_string();
        assert!(error.contains("note list must not be empty"));

        let mut bad_header_path = surface;
        bad_header_path.symbols[0].header = "../pluginterfaces/base/funknown.h".to_string();
        let error = validate_vst3_sdk_binding_surface_shape(&bad_header_path)
            .expect_err("unsafe binding surface header path should be rejected")
            .to_string();
        assert!(error.contains("relative normalized header path"));
    }

    #[test]
    fn vst3_sdk_binding_plan_command_writes_and_checks_generation_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let bindings_module =
            Utf8PathBuf::from_path_buf(temp.path().join("target/generated.rs")).unwrap();
        let out =
            Utf8PathBuf::from_path_buf(temp.path().join("generated-bindings-plan.json")).unwrap();

        run_vst3_sdk_binding_plan(
            Some(sdk.clone()),
            bindings_module.clone(),
            Some(out.clone()),
            false,
            "json",
        )
        .unwrap();

        let plan: vesty_vst3_sys::GeneratedBindingsPlan =
            serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(plan.status, "ready-for-binding-generator");
        assert!(!plan.bindings_generated);
        assert!(plan.header_manifest.complete);
        assert!(plan.checks.iter().any(|check| {
            check.name == "binding emitter"
                && check.status == "reserved"
                && check.value.contains("not enabled yet")
        }));

        run_vst3_sdk_binding_plan(
            Some(sdk.clone()),
            bindings_module.clone(),
            Some(out.clone()),
            true,
            "text",
        )
        .unwrap();

        fs::write(sdk.join("pluginterfaces/vst/vsttypes.h"), "changed\n").unwrap();
        let error = run_vst3_sdk_binding_plan(Some(sdk), bindings_module, Some(out), true, "text")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings plan drift")
        );
        assert!(error.to_string().contains("pluginterfaces/vst/vsttypes.h"));
    }

    #[test]
    fn vst3_sdk_binding_surface_command_writes_and_checks_symbol_surface() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out = Utf8PathBuf::from_path_buf(temp.path().join("generated-bindings-surface.json"))
            .unwrap();

        run_vst3_sdk_binding_surface(Some(sdk.clone()), Some(out.clone()), false, "json").unwrap();

        let surface: vesty_vst3_sys::GeneratedBindingsSurface =
            serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(surface.status, "ready-for-binding-emitter");
        assert!(!surface.bindings_generated);
        assert!(surface.header_manifest.complete);
        assert!(surface.missing_symbols.is_empty());
        assert_eq!(
            surface.required_headers.len(),
            vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS.len()
        );
        assert!(surface.symbols.iter().all(|symbol| symbol.symbol_present));
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IPlugView" && symbol.header == "pluginterfaces/gui/iplugview.h"
        }));
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IMidiMapping"
                && symbol.header == "pluginterfaces/vst/ivstmidicontrollers.h"
        }));
        assert!(
            surface
                .notes
                .iter()
                .any(|note| { note.contains("does not") || note.contains("bindingsGenerated") })
        );

        run_vst3_sdk_binding_surface(Some(sdk.clone()), Some(out.clone()), true, "text").unwrap();

        fs::write(sdk.join("pluginterfaces/gui/iplugview.h"), "changed\n").unwrap();
        let error = run_vst3_sdk_binding_surface(Some(sdk), Some(out), true, "text").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings surface drift")
        );
        assert!(error.to_string().contains("pluginterfaces/gui/iplugview.h"));
    }

    #[test]
    fn vst3_sdk_emit_scaffold_command_writes_checks_and_rejects_drift() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out =
            Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated.rs")).unwrap();

        run_vst3_sdk_emit_scaffold(Some(sdk.clone()), out.clone(), false, "json").unwrap();

        let module = fs::read_to_string(&out).unwrap();
        assert!(module.contains(vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR));
        assert!(module.contains("pub const BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub const STATUS: &str = \"metadata-scaffold\";"));
        assert!(module.contains("pluginterfaces/vst/vsttypes.h"));

        run_vst3_sdk_emit_scaffold(Some(sdk.clone()), out.clone(), true, "text").unwrap();

        fs::write(&out, module + "// drift\n").unwrap();
        let error = run_vst3_sdk_emit_scaffold(Some(sdk), out, true, "text").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings scaffold drift")
        );
    }

    #[test]
    fn vst3_sdk_emit_scaffold_command_rejects_blocked_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk =
            create_test_vst3_sdk(temp.path().join("sdk"), &["pluginterfaces/base/funknown.h"]);
        let out =
            Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated.txt")).unwrap();

        let error = run_vst3_sdk_emit_scaffold(Some(sdk), out, false, "text").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn vst3_sdk_emit_abi_seed_command_writes_checks_and_rejects_drift() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out =
            Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated-abi-seed.rs"))
                .unwrap();

        run_vst3_sdk_emit_abi_seed(Some(sdk.clone()), out.clone(), false, "json").unwrap();

        let module = fs::read_to_string(&out).unwrap();
        assert!(module.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR));
        assert!(module.contains("pub const ABI_SEED_GENERATED: bool = true;"));
        assert!(module.contains("pub const BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub type TResult = i32;"));
        assert!(module.contains("pub type ParamID = u32;"));
        assert!(module.contains("pub const kPlatformTypeNSView: PlatformType = \"NSView\";"));
        assert!(module.contains("pluginterfaces/vst/vsttypes.h"));

        validate_vst3_sdk_generated_bindings_abi_seed_text(&module).unwrap();
        run_vst3_sdk_emit_abi_seed(Some(sdk.clone()), out.clone(), true, "text").unwrap();

        fs::write(&out, module + "// drift\n").unwrap();
        let error = run_vst3_sdk_emit_abi_seed(Some(sdk), out, true, "text").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings ABI seed drift")
        );
    }

    #[test]
    fn vst3_sdk_emit_abi_seed_command_rejects_blocked_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk =
            create_test_vst3_sdk(temp.path().join("sdk"), &["pluginterfaces/base/funknown.h"]);
        let out =
            Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated-abi-seed.txt"))
                .unwrap();

        let error = run_vst3_sdk_emit_abi_seed(Some(sdk), out, false, "text").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn vst3_sdk_emit_abi_command_writes_checks_and_rejects_drift() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out = Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated-abi.rs"))
            .unwrap();

        run_vst3_sdk_emit_abi(Some(sdk.clone()), out.clone(), false, "json").unwrap();

        let module = fs::read_to_string(&out).unwrap();
        assert!(module.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR));
        assert!(module.contains("pub const ABI_LAYOUT_GENERATED: bool = true;"));
        assert!(module.contains("pub const BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("#[repr(C)]"));
        assert!(module.contains("pub struct FUnknownVTable"));
        assert!(module.contains("pub struct FUnknown"));
        assert!(module.contains("pub struct ViewRect"));
        assert!(module.contains("pub struct ProgramListInfo"));
        assert!(module.contains("pub struct UnitInfo"));
        assert!(module.contains("pub struct NoteExpressionTypeInfo"));
        assert!(module.contains("pub struct PhysicalUIMap"));
        assert!(module.contains("pub struct PhysicalUIMapList"));
        assert!(module.contains("pub type ParamID = u32;"));
        assert!(module.contains("pub type String128 = [TChar; STRING128_CODE_UNITS];"));
        assert!(module.contains("pub type ProgramListID = int32;"));
        assert!(module.contains("pub type NoteExpressionTypeID = uint32;"));
        assert!(module.contains("pub type Sample64 = f64;"));
        assert!(module.contains("pub const kRootUnitId: UnitID = 0;"));
        assert!(module.contains("pub const kNoProgramListId: ProgramListID = -1;"));
        assert!(module.contains("pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &["));
        assert!(module.contains("pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &["));
        assert!(module.contains(
            "type_name: \"ProgramListInfo\", size: std::mem::size_of::<ProgramListInfo>()"
        ));
        assert!(module.contains(
            "owner: \"NoteExpressionTypeInfo\", field: \"valueDesc\", offset: std::mem::offset_of!(NoteExpressionTypeInfo, valueDesc)"
        ));
        assert!(module.contains("pluginterfaces/base/funknown.h"));
        assert!(module.contains("pluginterfaces/vst/ivstunits.h"));
        assert!(module.contains("pluginterfaces/vst/ivstnoteexpression.h"));

        validate_vst3_sdk_generated_bindings_abi_text(&module).unwrap();
        run_vst3_sdk_emit_abi(Some(sdk.clone()), out.clone(), true, "text").unwrap();

        fs::write(&out, module + "// drift\n").unwrap();
        let error = run_vst3_sdk_emit_abi(Some(sdk), out, true, "text").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings ABI layout drift")
        );
    }

    #[test]
    fn vst3_sdk_emit_abi_command_rejects_blocked_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk =
            create_test_vst3_sdk(temp.path().join("sdk"), &["pluginterfaces/base/funknown.h"]);
        let out = Utf8PathBuf::from_path_buf(temp.path().join("target/vst3-sdk/generated-abi.txt"))
            .unwrap();

        let error = run_vst3_sdk_emit_abi(Some(sdk), out, false, "text").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn vst3_sdk_emit_interface_skeleton_command_writes_checks_and_rejects_drift() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out = Utf8PathBuf::from_path_buf(
            temp.path()
                .join("target/vst3-sdk/generated-interface-skeleton.rs"),
        )
        .unwrap();

        run_vst3_sdk_emit_interface_skeleton(Some(sdk.clone()), out.clone(), false, "json")
            .unwrap();

        let module = fs::read_to_string(&out).unwrap();
        assert!(module.contains(vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR));
        assert!(module.contains("pub const INTERFACE_SKELETON_GENERATED: bool = true;"));
        assert!(module.contains("pub const BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;"));
        assert!(module.contains("pub struct FUnknownVTable"));
        assert!(module.contains("pub struct IPlugViewVTable"));
        assert!(module.contains("pub struct IEditControllerVTable"));
        assert!(module.contains("pub struct IMidiMappingVTable"));
        assert!(module.contains("pub struct IUnitInfoVTable"));
        assert!(module.contains("pub struct IProgramListDataVTable"));
        assert!(module.contains("pub struct INoteExpressionControllerVTable"));
        assert!(module.contains("pub struct InterfaceMethod"));
        assert!(module.contains("pub struct InterfaceVTableSlot"));
        assert!(module.contains("pub struct InterfaceCallbackType"));
        assert!(module.contains("pub struct InterfaceVTableFieldOffset"));
        assert!(module.contains("pub struct ComObjectInterface"));
        assert!(module.contains("pub struct ComObjectIdentityPlan"));
        assert!(module.contains("pub struct ComObjectQueryInterfaceDispatchEntry"));
        assert!(module.contains("pub struct FactoryExportPlan"));
        assert!(module.contains("pub struct FactoryClassPlan"));
        assert!(module.contains("pub struct ModuleExportPlan"));
        assert!(module.contains("pub struct BinaryExportInspectionToolPlan"));
        assert!(module.contains("pub slot: usize,"));
        assert!(module.contains("pub signature: &'static str,"));
        assert!(module.contains("pub local_slot: usize,"));
        assert!(module.contains("pub global_slot: usize,"));
        assert!(module.contains("pub callback_type: &'static str,"));
        assert!(module.contains("pub object: &'static str,"));
        assert!(module.contains("pub required: bool,"));
        assert!(module.contains("pub root_interface: &'static str,"));
        assert!(module.contains("pub root_iid_const: &'static str,"));
        assert!(module.contains("pub funknown_identity: &'static str,"));
        assert!(module.contains("pub unknown_iid_result: &'static str,"));
        assert!(module.contains("pub returns_same_identity: bool,"));
        assert!(module.contains("pub add_ref_on_success: bool,"));
        assert!(module.contains("pub factory_object: &'static str,"));
        assert!(module.contains("pub factory_iid_const: &'static str,"));
        assert!(module.contains("pub class_count: usize,"));
        assert!(module.contains("pub class_kind: &'static str,"));
        assert!(module.contains("pub class_index: usize,"));
        assert!(module.contains("pub cid_policy: &'static str,"));
        assert!(module.contains("pub create_instance_root_iid_const: &'static str,"));
        assert!(module.contains("pub requested_iid_dispatch: &'static str,"));
        assert!(module.contains("pub symbol: &'static str,"));
        assert!(module.contains("pub platforms: &'static str,"));
        assert!(module.contains("pub generated_callable: bool,"));
        assert!(module.contains(
            "pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";"
        ));
        assert!(module.contains(
            "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";"
        ));
        assert!(module.contains(
            "pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;"
        ));
        assert!(module.contains(
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";"
        ));
        assert!(
            module.contains("pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;")
        );
        assert!(module.contains("pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;"));
        assert!(module.contains("pub const FACTORY_CLASS_PLAN_COUNT: usize = 2;"));
        assert!(module.contains("pub const MODULE_EXPORT_PLAN_COUNT: usize = 9;"));
        assert!(module.contains("pub const INTERFACE_METHODS: &[InterfaceMethod] = &["));
        assert!(module.contains("pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &["));
        assert!(module.contains("pub fn interface_vtable_slot_by_interface_and_method("));
        assert!(module.contains("pub fn interface_vtable_slot_by_interface_and_global_slot("));
        assert!(
            module.contains("pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
        );
        assert!(module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(module.contains("pub fn interface_vtable_field_offset_by_interface_and_field("));
        assert!(module.contains("pub const COM_OBJECTS: &[&str] = &["));
        assert!(module.contains("pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &["));
        assert!(
            module.contains("pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[")
        );
        assert!(module.contains("pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &["));
        assert!(module.contains("pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = "));
        assert!(module.contains("pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &["));
        assert!(
            module.contains("pub const VESTYPROCESSOR_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
        );
        assert!(
            module.contains("pub const VESTYCONTROLLER_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
        );
        assert!(
            module.contains("pub const GETPLUGINFACTORY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            module.contains("pub const WINDOWS_INITDLL_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            module.contains("pub const MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            module.contains("pub const LINUX_MODULEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(module.contains("pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &["));
        assert!(module.contains(
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &["
        ));
        assert!(module.contains("pub fn binary_export_inspection_tools("));
        assert!(module.contains("pub fn binary_export_symbol_plan_by_platform_and_symbol("));
        assert!(
            module.contains("pub fn required_binary_export_symbol_count(platform: &str) -> usize")
        );
        assert!(module.contains("pub fn first_missing_binary_export_symbol("));
        assert!(module.contains("pub fn binary_export_required_symbols_present("));
        assert!(module.contains("pub const VESTYPROCESSOR_INTERFACES: &[ComObjectInterface] = &["));
        assert!(
            module.contains("pub const VESTYCONTROLLER_INTERFACES: &[ComObjectInterface] = &[")
        );
        assert!(
            module.contains("pub const VESTYPROCESSOR_IDENTITY_PLAN: ComObjectIdentityPlan = ")
        );
        assert!(module.contains("pub const VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH: &[ComObjectQueryInterfaceDispatchEntry] = &["));
        assert!(module.contains("pub const IAUDIOPROCESSOR_METHODS: &[InterfaceMethod] = &["));
        assert!(
            module.contains("pub const IAUDIOPROCESSOR_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
        );
        assert!(
            module.contains(
                "pub const IAUDIOPROCESSOR_CALLBACK_TYPES: &[InterfaceCallbackType] = &["
            )
        );
        assert!(module.contains(
            "pub const IAUDIOPROCESSOR_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(module.contains("pub type IAudioProcessorProcess = unsafe extern \"system\" fn("));
        assert!(module.contains("pub process: IAudioProcessorProcess,"));
        assert!(module.contains("offset: std::mem::offset_of!(IAudioProcessorVTable, process)"));
        assert!(module.contains("slot: 6, interface: \"IAudioProcessor\", name: \"process\", purpose: \"realtime audio/MIDI/process callback\", realtime: true, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(module.contains("local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(module.contains("InterfaceCallbackType { interface: \"IAudioProcessor\", method: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IProcessContextRequirements\", iid_const: \"IPROCESSCONTEXTREQUIREMENTS_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(module.contains("ComObjectInterface { object: \"VestyController\", interface: \"IEditController\", iid_const: \"IEDITCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(module.contains("ComObjectInterface { object: \"VestyPlugView\", interface: \"IPlugView\", iid_const: \"IPLUGVIEW_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(module.contains("ComObjectInterface { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(module.contains("ComObjectIdentityPlan { object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }"));
        assert!(module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"FUnknown\", iid_const: \"FUNKNOWN_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", root_interface: \"IEditController\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(module.contains("FactoryExportPlan { factory_object: \"VestyFactory\", factory_interface: \"IPluginFactory\", factory_iid_const: \"IPLUGINFACTORY_IID\", class_count: 2, count_classes_result: \"2\", get_factory_info_source: \"PluginInfo vendor/url/email + kUnicode\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(module.contains("FactoryClassPlan { class_kind: \"processor\", class_index: 0, class_object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", category: \"Audio Module Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id\", cid_policy: \"processor-cid-is-plugin-class-id\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyProcessor\", create_instance_root_interface: \"IComponent\", create_instance_root_iid_const: \"ICOMPONENT_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(module.contains("FactoryClassPlan { class_kind: \"controller\", class_index: 1, class_object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", category: \"Component Controller Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id[15].wrapping_add(1)\", cid_policy: \"controller-cid-last-byte-wrapping-add-1\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyController\", create_instance_root_interface: \"IEditController\", create_instance_root_iid_const: \"IEDITCONTROLLER_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(module.contains("ModuleExportPlan { symbol: \"GetPluginFactory\", platforms: \"windows,macos,linux\", signature: \"extern \\\"system\\\" fn() -> *mut IPluginFactory\", purpose: \"return VST3 plugin factory pointer\", implementation: \"vesty_vst3::create_plugin_factory::<Plugin>()\", return_policy: \"returns owned COM factory pointer for host discovery\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(module.contains("ModuleExportPlan { symbol: \"bundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS VST3 bundle initialization entry\", implementation: \"vesty_vst3::set_macos_bundle_ref(bundle_ref); return true\", return_policy: \"bundle resources path is captured when possible\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(module.contains("ModuleExportPlan { symbol: \"ModuleEntry\", platforms: \"linux\", signature: \"extern \\\"system\\\" fn(library_handle: *mut c_void) -> bool\", purpose: \"Linux VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(module.contains("pub const IUNITINFO_METHODS: &[InterfaceMethod] = &["));
        assert!(module.contains("pub const IUNITINFO_VTABLE_SLOTS: &[InterfaceVTableSlot] = &["));
        assert!(
            module.contains("pub const IUNITINFO_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
        );
        assert!(module.contains(
            "pub const IUNITINFO_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(
            module.contains("pub type IUnitInfoGetProgramListInfo = unsafe extern \"system\" fn(")
        );
        assert!(module.contains("pub getProgramListInfo: IUnitInfoGetProgramListInfo,"));
        assert!(
            module.contains("offset: std::mem::offset_of!(IUnitInfoVTable, getProgramListInfo)")
        );
        assert!(module.contains("name: \"getProgramListInfo\", purpose: \"return program-list metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
        assert!(module.contains("local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
        assert!(module.contains("pub const IPROGRAMLISTDATA_METHODS: &[InterfaceMethod] = &["));
        assert!(
            module
                .contains("pub type IProgramListDataGetProgramData = unsafe extern \"system\" fn(")
        );
        assert!(module.contains("pub getProgramData: IProgramListDataGetProgramData,"));
        assert!(
            module.contains("offset: std::mem::offset_of!(IProgramListDataVTable, getProgramData)")
        );
        assert!(module.contains("name: \"getProgramData\", purpose: \"save program data to stream\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
        assert!(module.contains("local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
        assert!(
            module.contains("pub const INOTEEXPRESSIONCONTROLLER_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(module.contains(
            "pub type INoteExpressionControllerGetNoteExpressionInfo = unsafe extern \"system\" fn("
        ));
        assert!(module.contains(
            "pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,"
        ));
        assert!(module.contains(
            "offset: std::mem::offset_of!(INoteExpressionControllerVTable, getNoteExpressionInfo)"
        ));
        assert!(module.contains("name: \"getNoteExpressionInfo\", purpose: \"return Note Expression type metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
        assert!(module.contains("local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
        assert!(module.contains("pluginterfaces/vst/ivstmidicontrollers.h"));
        assert!(module.contains("pluginterfaces/vst/ivstunits.h"));
        assert!(module.contains("pluginterfaces/vst/ivstnoteexpression.h"));

        validate_vst3_sdk_generated_bindings_interface_skeleton_text(&module).unwrap();
        run_vst3_sdk_emit_interface_skeleton(Some(sdk.clone()), out.clone(), true, "text").unwrap();

        fs::write(&out, module + "// drift\n").unwrap();
        let error = run_vst3_sdk_emit_interface_skeleton(Some(sdk), out, true, "text").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("VST3 SDK generated bindings interface skeleton drift")
        );
    }

    #[test]
    fn vst3_sdk_interface_skeleton_validator_tracks_vst3_sys_export_plans() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let out = Utf8PathBuf::from_path_buf(
            temp.path()
                .join("target/vst3-sdk/generated-interface-skeleton.rs"),
        )
        .unwrap();

        run_vst3_sdk_emit_interface_skeleton(Some(sdk), out.clone(), false, "json").unwrap();
        let module = fs::read_to_string(&out).unwrap();
        validate_vst3_sdk_generated_bindings_interface_skeleton_text(&module).unwrap();

        let stale_count = module.replace(
            &format!(
                "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = {};",
                vesty_vst3_sys::binary_export_inspection_tool_plans().len()
            ),
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = 0;",
        );
        let error =
            validate_vst3_sdk_generated_bindings_interface_skeleton_text(&stale_count).unwrap_err();
        assert!(error.contains("binary export inspection tool plan count does not match"));

        let stale_tool = module.replace(
            "BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-nm\", args: &[\"-D\", \"--defined-only\"] }",
            "BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-readobj\", args: &[\"--symbols\"] }",
        );
        let error =
            validate_vst3_sdk_generated_bindings_interface_skeleton_text(&stale_tool).unwrap_err();
        assert!(error.contains(
            "missing vesty-vst3-sys binary export inspection tool plan `linux-x64/llvm-nm`"
        ));

        let stale_symbol = module.replace(
            "BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"InitDll\", tool_symbol: \"InitDll\"",
            "BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"DllMain\", tool_symbol: \"DllMain\"",
        );
        let error = validate_vst3_sdk_generated_bindings_interface_skeleton_text(&stale_symbol)
            .unwrap_err();
        assert!(
            error
                .contains("missing vesty-vst3-sys binary export symbol plan `windows-x64/InitDll`")
        );

        let extra_tool = module.replace(
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[\n",
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[\n    BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-readobj\", args: &[\"--symbols\"] },\n",
        );
        let error =
            validate_vst3_sdk_generated_bindings_interface_skeleton_text(&extra_tool).unwrap_err();
        assert!(
            error.contains(
                "binary export inspection tool plan array contains 7 record(s), expected 6"
            )
        );

        let duplicate_symbol = module.replace(
            "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[\n",
            "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[\n    BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"GetPluginFactory\", tool_symbol: \"GetPluginFactory\", inspection_tool: \"dumpbin /exports or llvm-objdump -p\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" },\n",
        );
        let error = validate_vst3_sdk_generated_bindings_interface_skeleton_text(&duplicate_symbol)
            .unwrap_err();
        assert!(
            error.contains("binary export symbol plan array contains 12 record(s), expected 11")
        );
        assert!(error.contains(
            "vesty-vst3-sys binary export symbol plan `windows-x64/GetPluginFactory` appears 2 time(s), expected exactly once"
        ));
    }

    #[test]
    fn vst3_sdk_emit_interface_skeleton_command_rejects_blocked_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk =
            create_test_vst3_sdk(temp.path().join("sdk"), &["pluginterfaces/base/funknown.h"]);
        let out = Utf8PathBuf::from_path_buf(
            temp.path()
                .join("target/vst3-sdk/generated-interface-skeleton.txt"),
        )
        .unwrap();

        let error =
            run_vst3_sdk_emit_interface_skeleton(Some(sdk), out, false, "text").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[cfg(unix)]
    #[test]
    fn vst3_sdk_check_commands_reject_symlink_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let sdk = create_test_vst3_sdk(temp.path().join("sdk"), &[]);
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let manifest = root.join("sdk-header-manifest.json");
        let manifest_link = root.join("sdk-header-manifest-link.json");
        run_vst3_sdk_manifest(Some(sdk.clone()), Some(manifest.clone()), false, "json").unwrap();
        unix_fs::symlink(&manifest, &manifest_link).unwrap();
        let error = run_vst3_sdk_manifest(Some(sdk.clone()), Some(manifest_link), true, "text")
            .unwrap_err()
            .to_string();
        assert!(error.contains("VST3 SDK header manifest check input must not be a symlink"));

        let bindings_module = root.join("target/vst3-sdk/generated.rs");
        let plan = root.join("generated-bindings-plan.json");
        let plan_link = root.join("generated-bindings-plan-link.json");
        run_vst3_sdk_binding_plan(
            Some(sdk.clone()),
            bindings_module.clone(),
            Some(plan.clone()),
            false,
            "json",
        )
        .unwrap();
        unix_fs::symlink(&plan, &plan_link).unwrap();
        let error = run_vst3_sdk_binding_plan(
            Some(sdk.clone()),
            bindings_module.clone(),
            Some(plan_link),
            true,
            "text",
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("VST3 SDK generated bindings plan check input must not be a symlink")
        );

        let surface = root.join("generated-bindings-surface.json");
        let surface_link = root.join("generated-bindings-surface-link.json");
        run_vst3_sdk_binding_surface(Some(sdk.clone()), Some(surface.clone()), false, "json")
            .unwrap();
        unix_fs::symlink(&surface, &surface_link).unwrap();
        let error =
            run_vst3_sdk_binding_surface(Some(sdk.clone()), Some(surface_link), true, "text")
                .unwrap_err()
                .to_string();
        assert!(
            error.contains("VST3 SDK generated bindings surface check input must not be a symlink")
        );

        let scaffold = root.join("generated.rs");
        let scaffold_link = root.join("generated-link.rs");
        run_vst3_sdk_emit_scaffold(Some(sdk.clone()), scaffold.clone(), false, "json").unwrap();
        unix_fs::symlink(&scaffold, &scaffold_link).unwrap();
        let error = run_vst3_sdk_emit_scaffold(Some(sdk.clone()), scaffold_link, true, "text")
            .unwrap_err()
            .to_string();
        assert!(
            error
                .contains("VST3 SDK generated bindings scaffold check input must not be a symlink")
        );

        let abi_seed = root.join("generated-abi-seed.rs");
        let abi_seed_link = root.join("generated-abi-seed-link.rs");
        run_vst3_sdk_emit_abi_seed(Some(sdk.clone()), abi_seed.clone(), false, "json").unwrap();
        unix_fs::symlink(&abi_seed, &abi_seed_link).unwrap();
        let error = run_vst3_sdk_emit_abi_seed(Some(sdk.clone()), abi_seed_link, true, "text")
            .unwrap_err()
            .to_string();
        assert!(
            error
                .contains("VST3 SDK generated bindings ABI seed check input must not be a symlink")
        );

        let abi = root.join("generated-abi.rs");
        let abi_link = root.join("generated-abi-link.rs");
        run_vst3_sdk_emit_abi(Some(sdk.clone()), abi.clone(), false, "json").unwrap();
        unix_fs::symlink(&abi, &abi_link).unwrap();
        let error = run_vst3_sdk_emit_abi(Some(sdk.clone()), abi_link, true, "text")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(
                "VST3 SDK generated bindings ABI layout check input must not be a symlink"
            )
        );

        let skeleton = root.join("generated-interface-skeleton.rs");
        let skeleton_link = root.join("generated-interface-skeleton-link.rs");
        run_vst3_sdk_emit_interface_skeleton(Some(sdk.clone()), skeleton.clone(), false, "json")
            .unwrap();
        unix_fs::symlink(&skeleton, &skeleton_link).unwrap();
        let error = run_vst3_sdk_emit_interface_skeleton(Some(sdk), skeleton_link, true, "text")
            .unwrap_err()
            .to_string();
        assert!(error.contains(
            "VST3 SDK generated bindings interface skeleton check input must not be a symlink"
        ));
    }

    #[test]
    fn vst3_sdk_binding_plan_reports_blockers_without_generating_bindings() {
        let temp = tempfile::tempdir().unwrap();
        let sdk =
            create_test_vst3_sdk(temp.path().join("sdk"), &["pluginterfaces/base/funknown.h"]);

        let plan = vesty_vst3_sys::generated_bindings_plan(
            &sdk,
            Utf8Path::new("target/vst3-sdk/generated.txt"),
        )
        .unwrap();

        assert_eq!(plan.status, "blocked");
        assert!(!plan.bindings_generated);
        assert!(plan.blockers.iter().any(|blocker| {
            blocker.contains("missing SDK header inputs")
                && blocker.contains("pluginterfaces/base/funknown.h")
        }));
        assert!(plan.blockers.iter().any(|blocker| blocker.contains(".rs")));
    }

    #[test]
    fn vst3_sdk_binding_plan_release_check_is_optional_but_strict_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let plan_path = root.join("generated-bindings-plan.json");
        write_test_vst3_sdk_binding_plan(
            &plan_path,
            &[],
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );

        let skipped = vst3_sdk_binding_plan_release_check(None);
        assert_eq!(skipped.status, "skipped");

        let ok = vst3_sdk_binding_plan_release_check(Some(&plan_path));
        assert_eq!(ok.status, "ok");
        assert!(ok.value.contains("ready-for-binding-generator"));
        assert!(ok.value.contains("target/vst3-sdk/generated.rs"));

        let plan: vesty_vst3_sys::GeneratedBindingsPlan =
            serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
        let mut unknown = plan.clone();
        unknown
            .checks
            .push(vesty_vst3_sys::GeneratedBindingsPlanCheck {
                name: "manual extra plan check".to_string(),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        let unknown_path = root.join("unknown-generated-bindings-plan.json");
        fs::write(
            &unknown_path,
            serde_json::to_string_pretty(&unknown).unwrap(),
        )
        .unwrap();
        let failed = vst3_sdk_binding_plan_release_check(Some(&unknown_path));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("unknown VST3 SDK binding plan check(s)")
        );
        assert!(failed.value.contains("manual extra plan check"));

        let mut missing = plan.clone();
        missing
            .checks
            .retain(|check| check.name != "binding emitter");
        let missing_path = root.join("missing-generated-bindings-plan.json");
        fs::write(
            &missing_path,
            serde_json::to_string_pretty(&missing).unwrap(),
        )
        .unwrap();
        let failed = vst3_sdk_binding_plan_release_check(Some(&missing_path));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("VST3 SDK binding plan missing required check(s)")
        );
        assert!(failed.value.contains("binding emitter"));

        let mut plan = plan;
        plan.bindings_generated = true;
        let invalid_path = root.join("claims-generated-bindings-plan.json");
        fs::write(&invalid_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

        let failed = vst3_sdk_binding_plan_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("must not claim bindings are generated yet")
        );
    }

    #[test]
    fn vst3_sdk_binding_plan_release_check_rejects_blocked_plan() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let plan_path = root.join("blocked-generated-bindings-plan.json");
        write_test_vst3_sdk_binding_plan(
            &plan_path,
            &["pluginterfaces/vst/ivstmessage.h"],
            Utf8Path::new("target/vst3-sdk/generated.txt"),
        );

        let check = vst3_sdk_binding_plan_release_check(Some(&plan_path));

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("plan status is `blocked`"));
        assert!(check.value.contains("plan has blockers"));
        assert!(check.value.contains("pluginterfaces/vst/ivstmessage.h"));
        assert!(check.value.contains(".rs"));
    }

    #[test]
    fn vst3_sdk_binding_surface_release_check_is_optional_but_strict_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let surface_path = root.join("generated-bindings-surface.json");
        write_test_vst3_sdk_binding_surface(&surface_path, &[]);

        let skipped = vst3_sdk_binding_surface_release_check(None);
        assert_eq!(skipped.status, "skipped");

        let ok = vst3_sdk_binding_surface_release_check(Some(&surface_path));
        assert_eq!(ok.status, "ok");
        assert!(ok.value.contains("ready-for-binding-emitter"));
        assert!(ok.value.contains("symbol"));
        assert!(ok.value.contains("bindings generated false"));

        let surface: vesty_vst3_sys::GeneratedBindingsSurface =
            serde_json::from_str(&fs::read_to_string(&surface_path).unwrap()).unwrap();
        let mut unknown = surface.clone();
        let mut unknown_symbol = unknown.symbols[0].clone();
        unknown_symbol.name = "ManualExtraSymbol".to_string();
        unknown.symbols.push(unknown_symbol);
        let invalid_path = root.join("unknown-symbol-generated-bindings-surface.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&unknown).unwrap(),
        )
        .unwrap();
        let failed = vst3_sdk_binding_surface_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(failed.value.contains("surface contains unknown symbol(s)"));
        assert!(failed.value.contains("ManualExtraSymbol"));

        let mut missing = surface.clone();
        missing
            .symbols
            .retain(|symbol| symbol.name != "IMidiMapping");
        let invalid_path = root.join("missing-expected-generated-bindings-surface.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&missing).unwrap(),
        )
        .unwrap();
        let failed = vst3_sdk_binding_surface_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(failed.value.contains("surface missing expected symbol(s)"));
        assert!(failed.value.contains("IMidiMapping"));

        let mut wrong_metadata = surface.clone();
        let symbol = wrong_metadata
            .symbols
            .iter_mut()
            .find(|symbol| symbol.name == "IPlugView")
            .expect("IPlugView surface symbol");
        symbol.kind = "type".to_string();
        symbol.header = "pluginterfaces/base/funknown.h".to_string();
        symbol.purpose = "forged purpose".to_string();
        let invalid_path = root.join("wrong-metadata-generated-bindings-surface.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&wrong_metadata).unwrap(),
        )
        .unwrap();
        let failed = vst3_sdk_binding_surface_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("surface symbol `IPlugView` kind is `type`, expected `interface`")
        );
        assert!(
            failed.value.contains(
                "surface symbol `IPlugView` header is `pluginterfaces/base/funknown.h`, expected `pluginterfaces/gui/iplugview.h`"
            )
        );
        assert!(
            failed
                .value
                .contains("surface symbol `IPlugView` purpose is `forged purpose`")
        );

        let mut surface = surface.clone();
        surface.bindings_generated = true;
        let invalid_path = root.join("claims-generated-bindings-surface.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&surface).unwrap(),
        )
        .unwrap();

        let failed = vst3_sdk_binding_surface_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("must not claim bindings are generated yet")
        );

        let mut surface: vesty_vst3_sys::GeneratedBindingsSurface =
            serde_json::from_str(&fs::read_to_string(&surface_path).unwrap()).unwrap();
        let missing = surface
            .symbols
            .iter_mut()
            .find(|symbol| symbol.name == "IMidiMapping")
            .expect("IMidiMapping surface symbol");
        missing.symbol_present = false;
        surface.missing_symbols = vec![format!("{} -> {}", missing.name, missing.header)];
        let invalid_path = root.join("missing-symbol-generated-bindings-surface.json");
        fs::write(
            &invalid_path,
            serde_json::to_string_pretty(&surface).unwrap(),
        )
        .unwrap();

        let failed = vst3_sdk_binding_surface_release_check(Some(&invalid_path));
        assert_eq!(failed.status, "failed");
        assert!(failed.value.contains("surface missing symbols"));
        assert!(failed.value.contains("IMidiMapping"));
        assert!(failed.value.contains("absent from header"));
    }

    #[test]
    fn vst3_sdk_binding_surface_release_check_rejects_blocked_surface() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let surface_path = root.join("blocked-generated-bindings-surface.json");
        write_test_vst3_sdk_binding_surface(&surface_path, &["pluginterfaces/gui/iplugview.h"]);

        let check = vst3_sdk_binding_surface_release_check(Some(&surface_path));

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("surface status is `blocked`"));
        assert!(check.value.contains("surface has blockers"));
        assert!(check.value.contains("IPlugView"));
        assert!(check.value.contains("pluginterfaces/gui/iplugview.h"));
    }

    #[test]
    fn vst3_sdk_generated_rust_artifact_release_checks_are_optional_but_strict_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let scaffold = root.join("generated.rs");
        write_test_vst3_sdk_scaffold(&scaffold, Utf8Path::new("target/vst3-sdk/generated.rs"));
        let abi_seed = root.join("generated-abi-seed.rs");
        write_test_vst3_sdk_abi_seed(&abi_seed);
        let abi = root.join("generated-abi.rs");
        write_test_vst3_sdk_abi(&abi);
        let interface_skeleton = root.join("generated-interface-skeleton.rs");
        write_test_vst3_sdk_interface_skeleton(&interface_skeleton);

        assert_eq!(
            vst3_sdk_generated_scaffold_release_check(None).status,
            "skipped"
        );
        assert_eq!(
            vst3_sdk_generated_scaffold_release_check(Some(&scaffold)).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_generated_abi_seed_release_check(Some(&abi_seed)).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_generated_abi_release_check(Some(&abi)).status,
            "ok"
        );
        assert_eq!(
            vst3_sdk_generated_interface_skeleton_release_check(Some(&interface_skeleton)).status,
            "ok"
        );

        let invalid_scaffold = root.join("invalid-generated.rs");
        fs::write(
            &invalid_scaffold,
            "pub const BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        let failed = vst3_sdk_generated_scaffold_release_check(Some(&invalid_scaffold));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("must not claim SDK bindings are generated")
        );

        let invalid_interface_skeleton = root.join("invalid-generated-interface-skeleton.rs");
        fs::write(
            &invalid_interface_skeleton,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        let failed =
            vst3_sdk_generated_interface_skeleton_release_check(Some(&invalid_interface_skeleton));
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .value
                .contains("must not claim full COM bindings are generated")
        );

        let stale_interface_skeleton = root.join("stale-tool-plan-generated-interface-skeleton.rs");
        write_stale_vst3_sdk_interface_skeleton_with_wrong_inspection_tool(
            &stale_interface_skeleton,
        );
        let failed =
            vst3_sdk_generated_interface_skeleton_release_check(Some(&stale_interface_skeleton));
        assert_eq!(failed.status, "failed");
        assert!(failed.value.contains(
            "missing vesty-vst3-sys binary export inspection tool plan `linux-x64/llvm-nm`"
        ));
    }

    fn create_test_vst3_sdk(root: std::path::PathBuf, missing: &[&str]) -> Utf8PathBuf {
        let root = Utf8PathBuf::from_path_buf(root).unwrap();
        fs::create_dir_all(root.join("pluginterfaces/base")).unwrap();
        fs::create_dir_all(root.join("pluginterfaces/vst")).unwrap();
        fs::write(root.join("README.md"), "VST3 SDK v3.8.0_build_66\n").unwrap();
        for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
            if missing.contains(header) {
                continue;
            }
            fs::create_dir_all(root.join(header).parent().unwrap()).unwrap();
            let mut text = format!("/* {header} */\n");
            for symbol in vesty_vst3_sys::generated_bindings_surface_symbol_names_for_header(header)
            {
                text.push_str(symbol);
                text.push('\n');
            }
            fs::write(root.join(header), text).unwrap();
        }
        root
    }

    fn write_test_vst3_sdk_manifest(path: &Utf8Path, missing: &[&str]) {
        let sdk = create_test_vst3_sdk(
            path.parent()
                .unwrap_or_else(|| Utf8Path::new("."))
                .join("sdk-input")
                .into_std_path_buf(),
            missing,
        );
        let manifest = vesty_vst3_sys::sdk_header_input_manifest(&sdk).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    }

    fn write_test_vst3_sdk_binding_plan(
        path: &Utf8Path,
        missing: &[&str],
        bindings_module: &Utf8Path,
    ) {
        let sdk = create_test_vst3_sdk(
            path.with_extension("sdk-input").into_std_path_buf(),
            missing,
        );
        let plan = vesty_vst3_sys::generated_bindings_plan(&sdk, bindings_module).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
    }

    fn write_test_vst3_sdk_binding_surface(path: &Utf8Path, missing: &[&str]) {
        let sdk = create_test_vst3_sdk(
            path.with_extension("sdk-input").into_std_path_buf(),
            missing,
        );
        let surface = vesty_vst3_sys::generated_bindings_surface(&sdk).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&surface).unwrap()).unwrap();
    }

    fn write_test_vst3_sdk_scaffold(path: &Utf8Path, bindings_module: &Utf8Path) {
        let sdk = create_test_vst3_sdk(path.with_extension("sdk-input").into_std_path_buf(), &[]);
        let scaffold = vesty_vst3_sys::generated_bindings_scaffold(&sdk, bindings_module).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, scaffold.module).unwrap();
    }

    fn write_test_vst3_sdk_abi_seed(path: &Utf8Path) {
        let sdk = create_test_vst3_sdk(path.with_extension("sdk-input").into_std_path_buf(), &[]);
        let seed = vesty_vst3_sys::generated_bindings_abi_seed(&sdk, path).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, seed.module).unwrap();
    }

    fn write_test_vst3_sdk_abi(path: &Utf8Path) {
        let sdk = create_test_vst3_sdk(path.with_extension("sdk-input").into_std_path_buf(), &[]);
        let abi = vesty_vst3_sys::generated_bindings_abi(&sdk, path).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, abi.module).unwrap();
    }

    fn write_test_vst3_sdk_interface_skeleton(path: &Utf8Path) {
        let sdk = create_test_vst3_sdk(path.with_extension("sdk-input").into_std_path_buf(), &[]);
        let skeleton = vesty_vst3_sys::generated_bindings_interface_skeleton(&sdk, path).unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, skeleton.module).unwrap();
    }

    fn write_stale_vst3_sdk_interface_skeleton_with_wrong_inspection_tool(path: &Utf8Path) {
        write_test_vst3_sdk_interface_skeleton(path);
        let module = fs::read_to_string(path).unwrap();
        let stale = module.replace(
            "BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-nm\", args: &[\"-D\", \"--defined-only\"] }",
            "BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-readobj\", args: &[\"--symbols\"] }",
        );
        assert_ne!(
            stale, module,
            "test skeleton fixture must contain the locked linux llvm-nm inspection plan"
        );
        fs::write(path, stale).unwrap();
    }

    #[test]
    fn bundle_platform_parser_accepts_release_targets() {
        assert_eq!(
            parse_bundle_platform("macos").unwrap(),
            BundlePlatform::Macos
        );
        assert_eq!(
            parse_bundle_platform("windows-x64").unwrap(),
            BundlePlatform::WindowsX64
        );
        assert_eq!(
            parse_bundle_platform("linux-x64").unwrap(),
            BundlePlatform::LinuxX64
        );
        assert_eq!(
            parse_bundle_platform("win64").unwrap(),
            BundlePlatform::WindowsX64
        );
        assert!(parse_bundle_platform("aix").is_err());
        assert_eq!(
            resolve_bundle_platform(Some("linux")).unwrap(),
            BundlePlatform::LinuxX64
        );
        assert!(resolve_bundle_platform(None).is_ok());
    }

    #[test]
    fn build_profile_defaults_to_release_and_accepts_debug() {
        assert!(build_release_mode(false, false).unwrap());
        assert!(build_release_mode(false, true).unwrap());
        assert!(!build_release_mode(true, false).unwrap());
        assert!(build_release_mode(true, true).is_err());
    }

    #[test]
    fn publish_plan_orders_workspace_crates_before_dependents() {
        let metadata = cargo_metadata_json(&workspace_root()).unwrap();
        let plan = workspace_publish_plan(&metadata).unwrap();
        let order = plan
            .packages
            .iter()
            .map(|package| (package.name.as_str(), package.order))
            .collect::<BTreeMap<_, _>>();

        for private in [
            "vesty-example-gain",
            "vesty-example-midi-synth",
            "vesty-example-web-ui-param-demo",
        ] {
            assert!(
                plan.skipped_private.iter().any(|name| name == private),
                "publish plan should skip private package {private}"
            );
            assert!(
                !order.contains_key(private),
                "private package {private} should not be publishable"
            );
        }

        for package in &plan.packages {
            for dependency in &package.internal_dependencies {
                assert!(
                    order[dependency.as_str()] < package.order,
                    "{} must publish before {}",
                    dependency,
                    package.name
                );
            }
        }

        for (dependency, dependent) in [
            ("vesty-params", "vesty-core"),
            ("vesty-core", "vesty-rt"),
            ("vesty-rt", "vesty-bridge"),
            ("vesty-ui-wry", "vesty-vst3"),
            ("vesty-vst3", "vesty"),
            ("vesty-build", "vesty-cli"),
        ] {
            assert!(
                order[dependency] < order[dependent],
                "{dependency} should publish before {dependent}"
            );
        }
    }

    #[test]
    fn publish_plan_rejects_publishable_crate_depending_on_private_workspace_package() {
        let metadata = serde_json::json!({
            "packages": [
                {
                    "id": "path+file:///workspace/crates/public#0.1.0",
                    "name": "vesty-public",
                    "version": "0.1.0",
                    "manifest_path": "/workspace/crates/public/Cargo.toml",
                    "publish": null,
                    "dependencies": [
                        {
                            "name": "vesty-private",
                            "kind": null
                        }
                    ]
                },
                {
                    "id": "path+file:///workspace/crates/private#0.1.0",
                    "name": "vesty-private",
                    "version": "0.1.0",
                    "manifest_path": "/workspace/crates/private/Cargo.toml",
                    "publish": [],
                    "dependencies": []
                }
            ],
            "workspace_members": [
                "path+file:///workspace/crates/public#0.1.0",
                "path+file:///workspace/crates/private#0.1.0"
            ]
        });

        let error = workspace_publish_plan(&metadata).unwrap_err();
        assert!(error.to_string().contains("private workspace dependency"));
        assert!(error.to_string().contains("vesty-private"));
    }

    #[test]
    fn ui_build_reports_missing_ui_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let ui = test_ui_config("missing-ui", Some("echo ok"), None);

        let error = build_configured_ui_assets(&project, &ui).unwrap_err();

        assert!(error.to_string().contains("ui directory does not exist"));
        assert!(error.to_string().contains("[ui].build"));
    }

    #[test]
    fn ui_build_reports_missing_dist_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        fs::create_dir_all(project.join("ui")).unwrap();
        let ui = test_ui_config("ui", None, Some("dist"));

        let error = build_configured_ui_assets(&project, &ui).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("ui dist directory does not exist")
        );
        assert!(error.to_string().contains("[ui].dist"));
    }

    #[test]
    fn run_shell_reports_missing_working_directory_without_spawning() {
        let dir = tempfile::tempdir().unwrap();
        let missing = Utf8PathBuf::from_path_buf(dir.path().join("missing")).unwrap();

        let error = run_shell("echo ok", Some(&missing)).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("command working directory does not exist")
        );
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn validate_report_serializes_static_and_validator_status() {
        let static_check = StaticBundleCheck {
            status: "ok".to_string(),
            moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
            binaries: vec!["Plugin.vst3/Contents/MacOS/Plugin".to_string()],
            binary_exports: vec![BinaryExportCheck {
                binary: "Plugin.vst3/Contents/MacOS/Plugin".to_string(),
                platform: "macos".to_string(),
                status: "ok".to_string(),
                tool: Some("nm -gU".to_string()),
                required_symbols: vec!["_GetPluginFactory".to_string()],
                found_symbols: vec!["_GetPluginFactory".to_string()],
                missing_symbols: Vec::new(),
                error: None,
            }],
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: None,
        };
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check,
            validator: ValidatorCheck::skipped("--static-only"),
        };

        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["bundle"], "Plugin.vst3");
        assert_eq!(value["static_check"]["status"], "ok");
        assert_eq!(
            value["static_check"]["binaries"][0],
            "Plugin.vst3/Contents/MacOS/Plugin"
        );
        assert_eq!(value["static_check"]["binary_exports"][0]["status"], "ok");
        assert_eq!(value["validator"]["status"], "skipped");
        assert_eq!(value["validator"]["reason"], "--static-only");
    }

    #[test]
    fn validate_report_accepts_legacy_json_without_binary_exports() {
        let text = r#"{
          "bundle": "Plugin.vst3",
          "static_check": {
            "status": "ok",
            "moduleinfo": "Plugin.vst3/Contents/Resources/moduleinfo.json",
            "binaries": ["Plugin.vst3/Contents/MacOS/Plugin"],
            "parameter_manifest": null,
            "asset_manifest": null,
            "asset_count": 0,
            "error": null
          },
          "validator": {
            "status": "skipped",
            "path": null,
            "exit_code": null,
            "tests_passed": null,
            "tests_failed": null,
            "stdout": null,
            "stderr": null,
            "reason": "--static-only",
            "error": null
          }
        }"#;

        let report: ValidateReport = serde_json::from_str(text).unwrap();

        assert!(report.static_check.binary_exports.is_empty());
        validate_static_validate_report(&report).unwrap();
    }

    #[test]
    fn validate_report_rejects_inconsistent_static_check_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();

        let ok_with_error = root.join("ok-with-error.json");
        write_validate_artifact(&ok_with_error, "ok", "skipped");
        let mut report = read_validate_report(&ok_with_error).unwrap();
        report.static_check.error = Some("manual edit left stale error".to_string());

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("static bundle check is ok but still lists error")
        );

        let failed_with_evidence = root.join("failed-with-evidence.json");
        write_validate_artifact(&failed_with_evidence, "failed", "skipped");
        let mut report = read_validate_report(&failed_with_evidence).unwrap();
        report.static_check.moduleinfo =
            Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string());

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("static bundle check failed but still lists moduleinfo")
        );
    }

    #[test]
    fn validate_report_rejects_inconsistent_validator_passed_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validator-passed.json");
        write_validate_artifact(&report_path, "ok", "passed");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.path = None;
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(error.to_string().contains("validator path is missing"));

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.error = Some("validator failed earlier".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator passed report includes error")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.reason = Some("--static-only".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator passed report includes reason")
        );
    }

    #[test]
    fn validate_report_rejects_validator_passed_log_summary_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validator-passed.json");
        write_validate_artifact(&report_path, "ok", "passed");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stdout = Some("Result: 47 tests passed, 1 tests failed\n".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stdout summary contradicts report counts"),
            "{error}"
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stderr = Some("Tests passed: 46\nTests failed: 0\n".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stderr summary contradicts report counts"),
            "{error}"
        );
    }

    #[test]
    fn validate_report_rejects_validator_passed_runtime_failure_logs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validator-passed.json");
        write_validate_artifact(&report_path, "ok", "passed");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stdout =
            Some("Steinberg validator passed 47 tests, 0 failed; validator timeout".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stdout includes runtime failure evidence"),
            "{error}"
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stderr =
            Some("VST3 validator: passed=47 failed=0; validator crashed".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stderr includes runtime failure evidence"),
            "{error}"
        );
    }

    #[test]
    fn validate_report_rejects_contradictory_validator_failed_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validator-failed.json");
        write_validate_artifact(&report_path, "ok", "failed");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.path = Some("/tools/validator".to_string());
        report.validator.exit_code = Some(0);
        report.validator.tests_passed = Some(47);
        report.validator.tests_failed = Some(0);
        report.validator.error = Some("manual stale failure marker".to_string());
        let error = validate_validator_check_self_consistent(&report.validator).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("exit code 0, no failed tests and no runtime failure evidence"),
            "{error}"
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.path = Some("/tools/validator".to_string());
        report.validator.tests_passed = Some(47);
        report.validator.tests_failed = None;
        report.validator.error = Some("validator failed".to_string());
        let error = validate_validator_check_self_consistent(&report.validator).unwrap_err();
        assert!(error.to_string().contains("partial test counts"), "{error}");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.path = Some("/tools/validator".to_string());
        report.validator.exit_code = Some(0);
        report.validator.tests_passed = Some(47);
        report.validator.tests_failed = Some(0);
        report.validator.error = Some("validator timeout".to_string());
        validate_validator_check_self_consistent(&report.validator).unwrap();
    }

    #[test]
    fn validate_report_rejects_skipped_validator_with_run_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("static-validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.path = Some("/tools/validator".to_string());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator skipped report includes path")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.tests_passed = Some(47);
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator skipped report includes passed test count")
        );
    }

    #[test]
    fn validate_report_rejects_unknown_static_or_validator_status() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("unknown-status.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.status = "maybe".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("static bundle check has unknown status")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.status = "green".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(error.to_string().contains("validator has unknown status"));
    }

    #[test]
    fn validate_report_rejects_malformed_shape_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate-shape.json");
        write_validate_artifact(&report_path, "ok", "passed");

        let mut report = read_validate_report(&report_path).unwrap();
        report.bundle = "Gain.vst3\nforged".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validate report bundle must not contain control characters")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report
            .static_check
            .binaries
            .push(report.static_check.binaries[0].clone());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate static bundle check binary")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0]
            .required_symbols
            .push("_GetPluginFactory".to_string());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate binary export required symbol")
        );

        let mut unknown_binary_export_field = serde_json::to_value(&report).unwrap();
        unknown_binary_export_field["static_check"]["binary_exports"][0]["checksum"] =
            serde_json::json!("forged");
        let error =
            serde_json::from_value::<ValidateReport>(unknown_binary_export_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));

        let mut report = read_validate_report(&report_path).unwrap();
        while report.static_check.binaries.len() <= VALIDATE_REPORT_MAX_BINARIES {
            let index = report.static_check.binaries.len();
            report
                .static_check
                .binaries
                .push(format!("Gain.vst3/Contents/MacOS/Gain{index}"));
        }
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(error.to_string().contains("too many binaries"));

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stdout = Some("line one\nline two\tok".to_string());
        validate_release_validate_report(&report).unwrap();

        report.validator.stdout = Some("validator\0output".to_string());
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stdout must not contain control characters")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.stderr = Some("x".repeat(VALIDATE_REPORT_LOG_MAX_BYTES + 1));
        let error = validate_release_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("validator stderr must be at most")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.validator.reason = Some("skipped\u{202e}hidden".to_string());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );
    }

    #[test]
    fn validate_report_rejects_paths_from_other_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.moduleinfo =
            Some("Other.vst3/Contents/Resources/moduleinfo.json".to_string());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("moduleinfo path does not belong to Gain.vst3")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binaries = vec!["Other.vst3/Contents/MacOS/Gain".to_string()];
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("binary path does not belong to Gain.vst3")
        );
    }

    #[test]
    fn validate_report_accepts_dot_prefixed_bundle_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.moduleinfo =
            Some("./Gain.vst3/Contents/Resources/moduleinfo.json".to_string());
        report.static_check.binaries = vec!["./Gain.vst3/Contents/MacOS/Gain".to_string()];
        report.static_check.binary_exports[0].binary =
            "./Gain.vst3/Contents/MacOS/Gain".to_string();

        validate_static_validate_report(&report).unwrap();
    }

    #[test]
    fn validate_report_rejects_duplicate_paths_after_normalization() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report
            .static_check
            .binaries
            .push("./Gain.vst3/Contents/MacOS/Gain".to_string());
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate static bundle check binary path")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        let mut duplicate = report.static_check.binary_exports[0].clone();
        duplicate.binary = ".\\Gain.vst3\\Contents\\MacOS\\Gain".to_string();
        report.static_check.binary_exports.push(duplicate);
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate binary export check `Gain.vst3/Contents/MacOS/Gain@macos`")
        );
    }

    #[test]
    fn strict_validate_matches_binary_exports_after_path_normalization() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("static-validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");
        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binaries = vec!["./Gain.vst3/Contents/MacOS/Gain".to_string()];
        report.static_check.binary_exports[0].binary =
            "Gain.vst3\\Contents\\MacOS\\Gain".to_string();

        validate_static_validate_report(&report).unwrap();
        assert!(strict_static_bundle_check_error(&report.static_check).is_none());
    }

    #[test]
    fn validate_report_rejects_mismatched_binary_export_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].binary = "Other.vst3/Contents/MacOS/Gain".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("binary export check path does not belong to Gain.vst3")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].binary =
            "Gain.vst3/Contents/MacOS/Unlisted".to_string();
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("binary not listed in static_check.binaries")
        );
    }

    #[test]
    fn validate_report_rejects_manifest_paths_from_other_bundle_or_bad_asset_count() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.parameter_manifest = Some(format!(
            "Other.vst3/Contents/Resources/{PARAMETER_MANIFEST_FILE}"
        ));
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("static_check.parameter_manifest does not belong to Gain.vst3")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.asset_manifest = Some(format!(
            "Gain.vst3/Contents/Resources/{ASSET_MANIFEST_FILE}"
        ));
        report.static_check.asset_count = 0;
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("asset_manifest is present but asset_count is 0")
        );

        let mut report = read_validate_report(&report_path).unwrap();
        report.static_check.asset_count = 2;
        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("asset_count is 2 but asset_manifest is missing")
        );
    }

    #[test]
    fn validate_report_rejects_failed_binary_export_checks() {
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/x86_64-linux/Plugin.so".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/x86_64-linux/Plugin.so".to_string(),
                    platform: "linux-x64".to_string(),
                    status: "failed".to_string(),
                    tool: Some("nm -D --defined-only".to_string()),
                    required_symbols: vec![
                        "GetPluginFactory".to_string(),
                        "ModuleEntry".to_string(),
                        "ModuleExit".to_string(),
                    ],
                    found_symbols: vec!["GetPluginFactory".to_string()],
                    missing_symbols: vec!["ModuleEntry".to_string(), "ModuleExit".to_string()],
                    error: None,
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck {
                status: "passed".to_string(),
                path: Some("/tools/validator".to_string()),
                exit_code: Some(0),
                tests_passed: Some(47),
                tests_failed: Some(0),
                stdout: None,
                stderr: None,
                reason: None,
                error: None,
            },
        };

        let static_error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            static_error
                .to_string()
                .contains("binary export check failed")
        );

        let release_error = validate_release_validate_report(&report).unwrap_err();
        assert!(release_error.to_string().contains("ModuleEntry"));
    }

    #[test]
    fn validate_report_binary_export_expectations_use_vst3_sys_plan() {
        for platform in ["macos", "windows-x64", "linux-x64"] {
            assert_eq!(
                expected_binary_export_symbols(platform),
                vesty_vst3_sys::required_binary_export_tool_symbols(platform)
            );
        }
        assert_eq!(expected_binary_export_symbols("linux-x11"), None);
    }

    #[test]
    fn validate_report_rejects_incomplete_ok_binary_export_checks() {
        let mut report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/MacOS/Plugin".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/MacOS/Plugin".to_string(),
                    platform: "macos".to_string(),
                    status: "ok".to_string(),
                    tool: Some("nm -gU".to_string()),
                    required_symbols: vec!["_GetPluginFactory".to_string()],
                    found_symbols: vec!["_GetPluginFactory".to_string()],
                    missing_symbols: Vec::new(),
                    error: None,
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("incomplete required symbol list")
        );
        assert!(error.to_string().contains("_bundleEntry"));

        report.static_check.binary_exports[0].required_symbols =
            expected_binary_export_symbols("macos")
                .unwrap()
                .iter()
                .map(|symbol| (*symbol).to_string())
                .collect();
        report.static_check.binary_exports[0].found_symbols = vec!["_GetPluginFactory".to_string()];

        let error = validate_static_validate_report(&report).unwrap_err();
        assert!(error.to_string().contains("did not record found symbols"));
        assert!(error.to_string().contains("_bundleEntry"));
    }

    #[test]
    fn validate_report_accepts_skipped_binary_export_checks_with_reason() {
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("Plugin.vst3/Contents/Resources/moduleinfo.json".to_string()),
                binaries: vec!["Plugin.vst3/Contents/x86_64-win/Plugin.vst3".to_string()],
                binary_exports: vec![BinaryExportCheck {
                    binary: "Plugin.vst3/Contents/x86_64-win/Plugin.vst3".to_string(),
                    platform: "windows-x64".to_string(),
                    status: "skipped".to_string(),
                    tool: None,
                    required_symbols: expected_binary_export_symbols("windows-x64")
                        .unwrap()
                        .iter()
                        .map(|symbol| (*symbol).to_string())
                        .collect(),
                    found_symbols: Vec::new(),
                    missing_symbols: Vec::new(),
                    error: Some("dumpbin unavailable on this host".to_string()),
                }],
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        validate_static_validate_report(&report).unwrap();
        let check = static_validate_reports_release_check(&[], false);
        assert_eq!(check.status, "skipped");
    }

    #[test]
    fn strict_validate_requires_ok_binary_export_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("static-validate.json");
        write_validate_artifact(&report_path, "ok", "skipped");
        let mut report = read_validate_report(&report_path).unwrap();

        assert!(strict_static_bundle_check_error(&report.static_check).is_none());

        report.static_check.binary_exports.clear();
        let error = strict_static_bundle_check_error(&report.static_check).unwrap();
        assert!(error.contains("strict validation requires binary export evidence"));
        assert!(error.contains("Gain.vst3/Contents/MacOS/Gain"));

        report = read_validate_report(&report_path).unwrap();
        report.static_check.binary_exports[0].status = "skipped".to_string();
        report.static_check.binary_exports[0].tool = None;
        report.static_check.binary_exports[0].found_symbols.clear();
        report.static_check.binary_exports[0].error =
            Some("nm unavailable on this runner".to_string());
        validate_static_validate_report(&report).unwrap();

        let error = strict_static_bundle_check_error(&report.static_check).unwrap();
        assert!(error.contains("strict validation requires ok binary export evidence"));
        assert!(error.contains("nm unavailable on this runner"));
    }

    #[test]
    fn validate_command_accepts_strict_flag() {
        let cli = Cli::try_parse_from([
            "vesty",
            "validate",
            "Plugin.vst3",
            "--static-only",
            "--strict",
            "--format",
            "json",
        ])
        .unwrap();

        match cli.command {
            Commands::Validate {
                static_only,
                strict,
                format,
                ..
            } => {
                assert!(static_only);
                assert!(strict);
                assert_eq!(format, "json");
            }
            _ => panic!("expected validate command"),
        }
    }

    #[test]
    fn daw_commands_share_release_action_default_evidence_layout() {
        for command in ["daw-matrix", "release-check"] {
            let cli = Cli::try_parse_from(["vesty", command]).unwrap();
            let (
                reaper_evidence,
                cubase_evidence,
                bitwig_evidence,
                ableton_evidence,
                studio_one_evidence,
            ) = match cli.command {
                Commands::DawMatrix {
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                    ..
                }
                | Commands::ReleaseCheck {
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                    ..
                } => (
                    reaper_evidence,
                    cubase_evidence,
                    bitwig_evidence,
                    ableton_evidence,
                    studio_one_evidence,
                ),
                _ => panic!("expected DAW evidence command"),
            };

            assert_eq!(
                reaper_evidence,
                Utf8PathBuf::from("target/daw-evidence/reaper")
            );
            assert_eq!(
                cubase_evidence,
                Utf8PathBuf::from("target/daw-evidence/cubase")
            );
            assert_eq!(
                bitwig_evidence,
                Utf8PathBuf::from("target/daw-evidence/bitwig")
            );
            assert_eq!(
                ableton_evidence,
                Utf8PathBuf::from("target/daw-evidence/ableton")
            );
            assert_eq!(
                studio_one_evidence,
                Utf8PathBuf::from("target/daw-evidence/studio-one")
            );
        }
    }

    #[test]
    fn param_manifest_command_writes_and_checks_stable_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let specs = root.join("params.json");
        let out = root.join("vesty-parameters.json");
        fs::write(
            &specs,
            r#"{
  "version": 1,
  "parameters": [
    {
      "id": "gain",
      "name": "Gain",
      "kind": { "float": { "min": -60.0, "max": 12.0 } },
      "defaultNormalized": 0.8333333333333334,
      "unit": "dB",
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    },
    {
      "id": "bypass",
      "name": "Bypass",
      "kind": "bool",
      "defaultNormalized": 0.0,
      "unit": null,
      "stepCount": 1,
      "flags": {
        "automatable": true,
        "bypass": true,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
"#,
        )
        .unwrap();

        run_param_manifest(specs.clone(), Some(out.clone()), false, "json").unwrap();
        let manifest = read_parameter_manifest(&out).unwrap();
        assert_eq!(manifest.parameters.len(), 2);
        assert_eq!(
            manifest.id_algorithm,
            "vesty.vst3.param.fnv1a31-positive.v2"
        );
        assert_eq!(manifest.parameters[0].id, "gain");
        assert_eq!(manifest.parameters[0].vst3_param_id, 1_983_572_582);
        assert!(!manifest.parameters[0].spec.flags.program_change);
        assert!(manifest.parameters[1].spec.flags.bypass);
        assert!(!manifest.parameters[1].spec.flags.program_change);
        let manifest_text = fs::read_to_string(&out).unwrap();
        assert!(manifest_text.contains(r#""programChange": false"#));

        run_param_manifest(specs.clone(), Some(out.clone()), true, "text").unwrap();

        let mut tampered = serde_json::to_value(&manifest).unwrap();
        tampered["parameters"][0]["vst3ParamId"] =
            serde_json::json!(manifest.parameters[0].vst3_param_id.wrapping_add(1));
        fs::write(&out, serde_json::to_string_pretty(&tampered).unwrap()).unwrap();

        let error = run_param_manifest(specs, Some(out), true, "text").unwrap_err();
        assert!(
            error.to_string().contains("vst3ParamId") || error.to_string().contains("out of date")
        );
    }

    #[test]
    fn param_manifest_report_rejects_unknown_json_fields() {
        let text = r#"{
          "status": "ok",
          "specs": "params.specs.json",
          "manifest": "vesty-parameters.json",
          "parameters": 1,
          "id_algorithm": "vesty.vst3.param.fnv1a31-positive.v2",
          "check": true,
          "generatedBy": "manual"
        }"#;

        let error = serde_json::from_str::<ParamManifestReport>(text).unwrap_err();

        assert!(error.to_string().contains("unknown field `generatedBy`"));
    }

    #[test]
    fn param_manifest_check_requires_output_path() {
        let temp = tempfile::tempdir().unwrap();
        let specs = Utf8PathBuf::from_path_buf(temp.path().join("params.json")).unwrap();
        fs::write(&specs, "[]").unwrap();

        let error = run_param_manifest(specs, None, true, "text").unwrap_err();

        assert!(error.to_string().contains("--check requires --out"));
    }

    #[cfg(unix)]
    #[test]
    fn param_manifest_rejects_symlinked_specs_input() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external_specs = root.join("external-params.json");
        let specs = root.join("params.json");
        fs::write(&external_specs, "[]").unwrap();
        std::os::unix::fs::symlink(&external_specs, &specs).unwrap();

        let error = run_param_manifest(
            specs,
            Some(root.join("vesty-parameters.json")),
            false,
            "text",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("parameter specs must not be a symlink")
        );
    }

    #[test]
    fn smoke_host_report_validates_local_examples_and_optional_bridge_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        fs::write(
            &meter_log,
            r#"{"lane":"meter","type":"meter.main","payload":{"peaks":[0.25]}}"#,
        )
        .unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "ok");
        assert!(smoke_host_report_all_ok(&report));
        assert!(
            report
                .external_evidence_note
                .contains("does not replace real DAW")
        );
        for name in [
            "gain config",
            "gain parameter sidecar",
            "midi-synth config",
            "midi-synth parameter sidecar",
            "web-ui-param-demo config",
            "web-ui-param-demo parameter sidecar",
            "web-ui-param-demo UI assets",
            "JSBridge trace",
            "meter stream",
        ] {
            assert!(
                report
                    .checks
                    .iter()
                    .any(|check| check.name == name && check.status == "ok"),
                "missing ok smoke-host check {name}: {report:#?}"
            );
        }
    }

    #[test]
    fn smoke_host_report_rejects_malformed_shape_fields() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let report = build_smoke_host_report(&workspace, None, None);

        let mut unknown_top_level = serde_json::to_value(&report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<SmokeHostReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<SmokeHostReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = report.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        let error = validate_smoke_host_report(&duplicate).unwrap_err();
        assert!(error.to_string().contains("duplicate smoke-host check"));
        let error = print_smoke_host_report(&duplicate, OutputFormat::Json, None).unwrap_err();
        assert!(error.to_string().contains("duplicate smoke-host check"));

        let mut unknown = report.clone();
        unknown.checks.push(SmokeHostCheck {
            name: "extra check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        let error = validate_smoke_host_report(&unknown).unwrap_err();
        assert!(error.to_string().contains("unknown smoke-host check(s)"));
        assert!(error.to_string().contains("extra_check"));

        let mut missing = report.clone();
        missing
            .checks
            .retain(|check| check.name != "JSBridge trace");
        let error = validate_smoke_host_report(&missing).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report missing required check(s)")
        );
        assert!(error.to_string().contains("jsbridge_trace"));

        let mut control_workspace = report.clone();
        control_workspace.workspace = "workspace\nforged".to_string();
        let error = validate_smoke_host_report(&control_workspace).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host workspace must not contain control characters")
        );

        let mut unsafe_hint = report.clone();
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        let error = validate_smoke_host_report(&unsafe_hint).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut too_many = report.clone();
        while too_many.checks.len() <= SMOKE_HOST_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(SmokeHostCheck {
                name: format!("extra check {index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        let error = validate_smoke_host_report(&too_many).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report has too many checks")
        );

        let mut generated = report.clone();
        generated.status = "failed".to_string();
        let check = generated
            .checks
            .iter_mut()
            .find(|check| check.name == "JSBridge trace")
            .unwrap();
        *check = smoke_host_failed(
            "JSBridge trace",
            "line one\nline two\u{202e}",
            "hint\twith newline\nnext",
        );
        validate_smoke_host_report(&generated).unwrap();
        let check = generated
            .checks
            .iter()
            .find(|check| check.name == "JSBridge trace")
            .unwrap();
        assert!(!check.value.contains('\n'));
        assert!(!check.value.contains('\u{202e}'));

        let diagnostic = smoke_host_failed(
            "multiline diagnostic",
            "line one\nline two\u{202e}",
            "hint\twith newline\nnext",
        );
        assert!(!diagnostic.name.contains('\n'));
        assert!(!diagnostic.value.contains('\n'));
        assert!(!diagnostic.value.contains('\u{202e}'));
    }

    #[test]
    fn smoke_host_check_mode_rejects_drift_and_strict_rejects_skipped_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        fs::write(&meter_log, "meter_flush sent=1\n").unwrap();
        let out = workspace.join("smoke-host.json");

        run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: Some(bridge_trace.clone()),
            meter_log: Some(meter_log.clone()),
            out: Some(out.clone()),
            check: false,
            strict: true,
            format: "json".to_string(),
        })
        .unwrap();
        run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: Some(bridge_trace),
            meter_log: Some(meter_log),
            out: Some(out.clone()),
            check: true,
            strict: true,
            format: "text".to_string(),
        })
        .unwrap();

        fs::write(
            workspace.join("examples/gain/vesty.toml"),
            smoke_host_vesty_toml("Wrong Gain", "Fx", false),
        )
        .unwrap();
        let error = run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: None,
            meter_log: None,
            out: Some(out),
            check: true,
            strict: false,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report is out of date")
        );

        let strict_error = run_smoke_host(SmokeHostOptions {
            workspace,
            bridge_trace: None,
            meter_log: None,
            out: None,
            check: false,
            strict: true,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            strict_error
                .to_string()
                .contains("smoke-host checks are incomplete")
        );
    }

    #[test]
    fn smoke_host_rejects_contradictory_bridge_and_meter_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        let bridge_trace = workspace.join("bridge-trace.log");
        let meter_log = workspace.join("meter.log");
        fs::write(
            &bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
readyAck reply
result=0
bridge timeout
"#,
        )
        .unwrap();
        fs::write(&meter_log, "meter_flush sent=1\nmeter stream failed\n").unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "JSBridge trace"
                && check.status == "failed"
                && check
                    .value
                    .contains("no accepted bridge roundtrip or param gesture markers")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "meter stream"
                && check.status == "failed"
                && check
                    .value
                    .contains("no accepted nonzero meter stream markers")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn smoke_host_rejects_symlinked_report_bridge_trace_meter_log_and_parameter_specs() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());

        let external_report = workspace.join("external-smoke-host.json");
        let report_path = workspace.join("smoke-host.json");
        fs::write(&external_report, "{}").unwrap();
        unix_fs::symlink(&external_report, &report_path).unwrap();
        let error = run_smoke_host(SmokeHostOptions {
            workspace: workspace.clone(),
            bridge_trace: None,
            meter_log: None,
            out: Some(report_path),
            check: true,
            strict: false,
            format: "text".to_string(),
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("smoke-host report must not be a symlink")
        );

        let external_bridge_trace = workspace.join("external-bridge-trace.log");
        let bridge_trace = workspace.join("bridge-trace.log");
        fs::write(
            &external_bridge_trace,
            r#"{"type":"param.begin","result":0}
{"type":"param.perform","result":0}
{"type":"param.end","result":0}
result=0
"#,
        )
        .unwrap();
        unix_fs::symlink(&external_bridge_trace, &bridge_trace).unwrap();

        let external_meter_log = workspace.join("external-meter.log");
        let meter_log = workspace.join("meter.log");
        fs::write(&external_meter_log, "meter_flush sent=1\n").unwrap();
        unix_fs::symlink(&external_meter_log, &meter_log).unwrap();

        let specs_path = workspace.join("examples/gain/params.specs.json");
        let external_specs = workspace.join("external-params.specs.json");
        fs::copy(&specs_path, &external_specs).unwrap();
        fs::remove_file(&specs_path).unwrap();
        unix_fs::symlink(&external_specs, &specs_path).unwrap();

        let report = build_smoke_host_report(&workspace, Some(&bridge_trace), Some(&meter_log));

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "gain parameter sidecar"
                && check.status == "failed"
                && check
                    .value
                    .contains("parameter specs must not be a symlink")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "JSBridge trace"
                && check.status == "failed"
                && check.value.contains("bridge trace must not be a symlink")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "meter stream"
                && check.status == "failed"
                && check.value.contains("meter log must not be a symlink")
        }));
    }

    #[test]
    fn smoke_host_report_flags_missing_ui_assets_without_claiming_release_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = write_smoke_host_workspace(temp.path());
        fs::remove_dir_all(workspace.join("examples/web-ui-param-demo/ui/dist")).unwrap();

        let report = build_smoke_host_report(&workspace, None, None);

        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "web-ui-param-demo UI assets"
                && check.status == "failed"
                && check.value.contains("ui/dist")
                && check.value.contains("missing required file")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "JSBridge trace" && check.status == "skipped" })
        );
        assert!(
            report
                .external_evidence_note
                .contains("does not load plugin binaries")
        );
    }

    #[test]
    fn validator_summary_extracts_passed_and_failed_counts() {
        let text = r#"
-------------------------------------------------------------
Result: 47 tests passed, 0 tests failed
-------------------------------------------------------------
"#;
        assert_eq!(validator_test_summary(text), Some((47, 0)));
        assert_eq!(
            validator_test_summary(
                "2026-06-08T10:00:00Z RESULT: 47 TESTS PASSED, 0 TESTS FAILED\n"
            ),
            Some((47, 0))
        );
        assert_eq!(
            validator_test_summary("Tests passed: 47\nTests failed: 0\n"),
            Some((47, 0))
        );
        assert_eq!(
            validator_test_summary("1 test passed\n2 tests failed\n"),
            Some((1, 2))
        );
        assert_eq!(validator_test_summary("Result: unavailable"), None);
        assert_eq!(validator_test_summary("Result: 47 tests passed"), None);
    }

    #[test]
    fn protocol_export_check_detects_snapshot_drift() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();

        check_protocol_export(&out).unwrap();

        fs::write(
            out.join("typescript/protocol/BridgePacket.ts"),
            "export type BridgePacket = never;\n",
        )
        .unwrap();
        let error = check_protocol_export(&out).unwrap_err().to_string();
        assert!(error.contains("protocol export drift detected"));
        assert!(error.contains("changed"));
        assert!(error.contains("typescript/protocol/BridgePacket.ts"));
    }

    #[test]
    fn protocol_release_check_reports_drift_paths_and_snapshot_command() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();
        fs::write(
            out.join("typescript/protocol/BridgePacket.ts"),
            "export type BridgePacket = never;\n",
        )
        .unwrap();

        let check = protocol_release_check(&out, false, false);

        assert_eq!(check.name, "protocol snapshot");
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("changed"));
        assert!(check.value.contains("typescript/protocol/BridgePacket.ts"));
        let expected_hint =
            format!("run `vesty export-types --out {out}` and commit/update the snapshot");
        assert_eq!(check.hint.as_deref(), Some(expected_hint.as_str()));
    }

    #[test]
    fn protocol_release_check_rejects_skip_when_release_artifacts_are_required() {
        let protocol_snapshot = Utf8Path::new("target/vesty-protocol");
        let check = protocol_release_check(protocol_snapshot, true, true);

        assert_eq!(check.name, "protocol snapshot");
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("cannot skip protocol snapshot when --require-release-artifacts is set")
        );
        assert_eq!(
            check.hint.as_deref(),
            Some(
                "run `vesty export-types --out target/vesty-protocol --check` in final release evidence"
            )
        );
    }

    #[test]
    fn plugin_ui_protocol_sources_match_generated_export() {
        let temp = tempfile::tempdir().unwrap();
        let out = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&out).unwrap();

        let generated = collect_relative_files(&out.join("typescript")).unwrap();
        let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
        let package_src = workspace_root.join("packages/plugin-ui/src");
        let package = collect_relative_files(&package_src).unwrap();

        for (relative, expected_bytes) in &generated {
            let actual_bytes = package
                .get(relative)
                .unwrap_or_else(|| panic!("missing generated protocol source: {relative}"));
            let expected = String::from_utf8_lossy(expected_bytes).replace("\r\n", "\n");
            let actual = String::from_utf8_lossy(actual_bytes).replace("\r\n", "\n");
            if actual != expected {
                assert_eq!(
                    actual, expected,
                    "vesty-plugin-ui protocol source drifted: {relative}"
                );
            }
        }

        let extra = package
            .keys()
            .filter(|relative| {
                (relative.starts_with("protocol/") || relative.starts_with("serde_json/"))
                    && relative.as_str() != "protocol/index.ts"
                    && !generated.contains_key(*relative)
            })
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            extra.is_empty(),
            "vesty-plugin-ui has stale generated protocol sources: {extra:?}"
        );
    }

    #[test]
    fn release_report_paths_are_portable_across_windows_and_unix_separators() {
        let windows = r"C:\artifacts\release-evidence\package\VestyGain.static-validate.json";
        let portable = "C:/artifacts/release-evidence/package/VestyGain.static-validate.json";

        assert!(release_report_paths_equal(windows, portable));
        assert!(release_report_path_ends_with(
            windows,
            "package/VestyGain.static-validate.json"
        ));
        assert_eq!(portable_report_path(Utf8Path::new(windows)), portable);
        assert_eq!(
            recognized_json_artifact_name_from_path(Utf8Path::new(windows)).map(|(name, _)| name),
            Some("vst3 static validate report")
        );
        assert!(validate_report_path_prefers_static(Utf8Path::new(windows)));
    }

    #[test]
    fn workspace_packages_have_release_metadata() {
        let root = workspace_root();
        let root_manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(!root_manifest.contains("example.com"));
        for expected in [
            "authors = [\"Vesty Contributors\"]",
            "categories = [",
            "homepage = \"https://github.com/backrunner/vesty\"",
            "keywords = [",
            "license = \"Apache-2.0\"",
            "repository = \"https://github.com/backrunner/vesty\"",
        ] {
            assert!(
                root_manifest.contains(expected),
                "workspace Cargo.toml missing release metadata: {expected}"
            );
        }
        assert!(root.join("README.md").is_file());
        assert!(root.join("LICENSE-APACHE").is_file());
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        for expected in [
            "FloatParam::new(\"gain\", \"Gain\", -60.0, 12.0, 0.0).with_unit(\"dB\")",
            "const INFO: PluginInfo = PluginInfo",
            "fn params(&self) -> &Self::Params",
            "fn create_kernel(&self, _init: KernelInit) -> Self::Kernel",
            "fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult",
            "audio.copy_input_to_output(channel, gain)",
        ] {
            assert!(
                readme.contains(expected),
                "README minimal plugin example is missing current API shape: {expected}"
            );
        }
        for stale in [
            "#[param(id",
            "fn create_kernel(params:",
            "copy_input_to_output(linear)",
        ] {
            assert!(
                !readme.contains(stale),
                "README minimal plugin example contains stale API shape: {stale}"
            );
        }

        for entry in fs::read_dir(root.join("crates")).unwrap() {
            let entry = entry.unwrap();
            let manifest_path = Utf8PathBuf::from_path_buf(entry.path())
                .unwrap()
                .join("Cargo.toml");
            let manifest = fs::read_to_string(&manifest_path).unwrap();
            for expected in [
                "description = ",
                "authors.workspace = true",
                "categories.workspace = true",
                "homepage.workspace = true",
                "keywords.workspace = true",
                "readme = \"../../README.md\"",
                "repository.workspace = true",
            ] {
                assert!(
                    manifest.contains(expected),
                    "{manifest_path} missing release metadata: {expected}"
                );
            }
            assert!(!manifest.contains("example.com"), "{manifest_path}");
        }

        for entry in fs::read_dir(root.join("examples")).unwrap() {
            let entry = entry.unwrap();
            let example_dir = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
            let manifest_path = example_dir.join("Cargo.toml");
            let manifest = fs::read_to_string(&manifest_path).unwrap();
            assert!(
                manifest.contains("publish = false"),
                "{manifest_path} should not be publishable"
            );
            let source = fs::read_to_string(example_dir.join("src/lib.rs")).unwrap();
            assert!(
                source.contains("url: \"https://github.com/backrunner/vesty\""),
                "{example_dir}/src/lib.rs should use the project URL"
            );
            assert!(
                !source.contains("example.com") && !source.contains("dev@example.com"),
                "{example_dir}/src/lib.rs should not contain placeholder contact metadata"
            );
        }

        for package in ["plugin-ui"] {
            let package_json =
                fs::read_to_string(root.join("packages").join(package).join("package.json"))
                    .unwrap();
            for expected in [
                "\"description\":",
                "\"license\": \"Apache-2.0\"",
                "\"repository\":",
                "\"homepage\": \"https://github.com/backrunner/vesty#readme\"",
                "\"keywords\": [",
                "\"exports\":",
                "\"files\": [",
            ] {
                assert!(
                    package_json.contains(expected),
                    "packages/{package}/package.json missing release metadata: {expected}"
                );
            }
            assert!(!package_json.contains("example.com"), "packages/{package}");
        }
    }

    #[test]
    fn ci_package_static_validate_uses_strict_binary_export_gate() {
        let workflow = fs::read_to_string(workspace_root().join(".github/workflows/ci.yml"))
            .expect("ci workflow should be readable");
        let static_validate_step = workflow
            .split("- name: Static validate packaged bundles")
            .nth(1)
            .and_then(|tail| tail.split("- uses: actions/upload-artifact").next())
            .expect("package static validate step should exist");

        assert_eq!(
            static_validate_step
                .matches("cargo run -p vesty-cli -- validate")
                .count(),
            3,
            "package smoke should static-validate all three examples"
        );
        assert_eq!(
            static_validate_step.matches("--static-only").count(),
            3,
            "package smoke validate commands should stay static-only"
        );
        assert_eq!(
            static_validate_step.matches("--strict").count(),
            3,
            "package smoke must fail early on skipped binary export evidence"
        );
        for bundle in ["VestyGain", "VestyWebUIDemo", "VestyMIDISynth"] {
            assert!(
                static_validate_step.contains(&format!("target/ci-package/{bundle}.vst3")),
                "package smoke should validate {bundle}.vst3"
            );
            assert!(
                static_validate_step.contains(&format!(
                    "--report target/ci-package/{bundle}.validate.json"
                )),
                "package smoke should upload {bundle} static validate report"
            );
        }
    }

    #[test]
    fn doctor_report_includes_toolchain_webview_and_validator_checks() {
        let report = doctor_report();
        validate_doctor_report(&report).unwrap();
        assert_eq!(report.os.as_deref(), Some(doctor_os_label()));
        let names = report
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"rustc"));
        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"node"));
        assert!(names.contains(&"npm"));
        assert!(names.contains(&"vst3 binding baseline"));
        assert!(names.contains(&"vst3 SDK headers"));
        assert!(names.contains(&"vst3 validator"));
        assert!(names.contains(&"system webview"));
        let baseline = report
            .checks
            .iter()
            .find(|check| check.name == "vst3 binding baseline")
            .expect("binding baseline check");
        assert_eq!(baseline.status, "ok");
        assert!(baseline.value.contains("v3.8.0_build_66"));
        assert!(baseline.value.contains("upstream vst3 crate 0.3.0"));
        let sdk_headers = report
            .checks
            .iter()
            .find(|check| check.name == "vst3 SDK headers")
            .expect("vst3 SDK headers check");
        assert!(matches!(
            sdk_headers.status.as_str(),
            "ok" | "missing" | "skipped"
        ));
        if cfg!(target_os = "macos") {
            assert!(names.contains(&"signing: codesign"));
            assert!(names.contains(&"signing: notarytool"));
        } else if cfg!(target_os = "windows") {
            assert!(names.contains(&"signing: signtool"));
        } else if cfg!(target_os = "linux") {
            assert!(names.contains(&"signing: linux release policy"));
        } else {
            assert!(names.contains(&"signing: release signing"));
        }
        assert!(names.contains(&"daw install: REAPER"));
        assert!(names.contains(&"daw install: Cubase/Nuendo"));
        assert!(names.contains(&"daw install: Bitwig Studio"));
        assert!(names.contains(&"daw install: Ableton Live"));
        assert!(names.contains(&"daw install: Studio One"));
    }

    #[test]
    fn command_presence_check_uses_candidate_paths_and_missing_hint() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let tool = root.join("tool");
        fs::write(&tool, "test").unwrap();

        let found = command_presence_check(
            "signing: test",
            "definitely-not-a-vesty-test-tool",
            std::slice::from_ref(&tool),
            "install it",
        );
        assert_eq!(found.status, "ok");
        assert_eq!(found.value, tool.to_string());
        assert_eq!(found.hint, None);

        let missing = command_presence_check(
            "signing: missing",
            "definitely-not-a-vesty-test-tool",
            &[],
            "install it",
        );
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.hint.as_deref(), Some("install it"));
    }

    #[test]
    fn bundle_signing_command_maps_platforms() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let binary = Utf8Path::new("/tmp/VestyGain.vst3/Contents/x86_64-win/VestyGain.vst3");

        let mac = bundle_signing_command(
            BundlePlatform::Macos,
            bundle,
            binary,
            "Developer ID Application: Example",
        )
        .unwrap();
        assert!(mac.program.ends_with("codesign"));
        assert_eq!(mac.args[0], "--force");
        assert!(mac.args.contains(&"--deep".to_string()));
        assert!(mac.args.contains(&"runtime".to_string()));
        assert!(
            mac.args
                .contains(&"Developer ID Application: Example".to_string())
        );
        assert_eq!(mac.args.last().map(String::as_str), Some(bundle.as_str()));

        let windows = bundle_signing_command(
            BundlePlatform::WindowsX64,
            bundle,
            binary,
            "Example Code Signing",
        )
        .unwrap();
        assert!(windows.program.ends_with("signtool.exe"));
        assert_eq!(windows.args[0], "sign");
        assert!(windows.args.contains(&"/fd".to_string()));
        assert!(windows.args.contains(&"SHA256".to_string()));
        assert!(windows.args.contains(&"/tr".to_string()));
        assert!(windows.args.contains(&"Example Code Signing".to_string()));
        assert_eq!(
            windows.args.last().map(String::as_str),
            Some(binary.as_str())
        );

        let linux = bundle_signing_command(BundlePlatform::LinuxX64, bundle, binary, "Example")
            .unwrap_err()
            .to_string();
        assert!(linux.contains("release-channel specific"));

        let empty = bundle_signing_command(BundlePlatform::Macos, bundle, binary, "  ")
            .unwrap_err()
            .to_string();
        assert!(empty.contains("cannot be empty"));
    }

    #[test]
    fn dev_install_mode_parses_copy_and_symlink() {
        assert_eq!(
            parse_dev_install_mode("copy").unwrap(),
            DevInstallMode::Copy
        );
        assert_eq!(
            parse_dev_install_mode("symlink").unwrap(),
            DevInstallMode::Symlink
        );
        assert_eq!(
            parse_dev_install_mode("link").unwrap(),
            DevInstallMode::Symlink
        );
        assert!(parse_dev_install_mode("move").is_err());
    }

    #[test]
    fn dev_binary_autodiscovery_uses_root_cdylib_target() {
        let metadata = serde_json::json!({
            "root_package": "path+file:///plugin#demo-plugin@0.1.0",
            "packages": [
                {
                    "id": "path+file:///plugin#demo-plugin@0.1.0",
                    "targets": [
                        {
                            "name": "demo-plugin",
                            "kind": ["cdylib", "rlib"]
                        }
                    ]
                }
            ],
            "target_directory": "/tmp/plugin/target"
        });

        assert_eq!(
            cdylib_target_name_from_metadata(&metadata, None).unwrap(),
            "demo-plugin"
        );
        let filename = cdylib_filename("demo-plugin");
        if cfg!(target_os = "windows") {
            assert_eq!(filename, "demo_plugin.dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(filename, "libdemo_plugin.dylib");
        } else {
            assert_eq!(filename, "libdemo_plugin.so");
        }
    }

    #[test]
    fn dev_binary_autodiscovery_uses_current_manifest_in_workspace_metadata() {
        let metadata = serde_json::json!({
            "packages": [
                {
                    "id": "path+file:///workspace/crates/vesty#0.1.0",
                    "manifest_path": "/workspace/crates/vesty/Cargo.toml",
                    "targets": [
                        {
                            "name": "vesty",
                            "kind": ["lib"]
                        }
                    ]
                },
                {
                    "id": "path+file:///workspace/examples/gain#vesty-example-gain@0.1.0",
                    "manifest_path": "/workspace/examples/gain/Cargo.toml",
                    "targets": [
                        {
                            "name": "vesty_example_gain",
                            "kind": ["rlib", "cdylib"]
                        }
                    ]
                }
            ],
            "workspace_default_members": [
                "path+file:///workspace/crates/vesty#0.1.0"
            ],
            "target_directory": "/workspace/target"
        });

        assert_eq!(
            cdylib_target_name_from_metadata(
                &metadata,
                Some(Utf8Path::new("/workspace/examples/gain/Cargo.toml")),
            )
            .unwrap(),
            "vesty_example_gain"
        );
    }

    #[test]
    fn install_dev_bundle_copies_and_replaces_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();
        let install_dir = root.join("user-vst3");

        let installed = install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed, install_dir.join("VestyGain.vst3"));
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=1"
        );

        fs::write(&resource, "version=2").unwrap();
        let installed_again =
            install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed_again, installed);
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=2"
        );
    }

    #[test]
    fn install_dev_bundle_rejects_non_vst3_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("NotAPlugin");
        fs::create_dir(&source).unwrap();

        let error = install_dev_bundle(&source, &root.join("user-vst3"), DevInstallMode::Copy)
            .unwrap_err()
            .to_string();

        assert!(error.contains(".vst3"));
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_rejects_symlinked_source_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();
        let bundle_link = root.join("build/VestyGainLink.vst3");
        unix_fs::symlink(&bundle, &bundle_link).unwrap();

        let error = install_dev_bundle(&bundle_link, &root.join("user-vst3"), DevInstallMode::Copy)
            .expect_err("dev install should reject symlinked source bundle")
            .to_string();

        assert!(error.contains("dev install source must not be a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_rejects_symlinked_install_dir_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();

        let external = root.join("external-vst3");
        let parent_link = root.join("linked-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let error = install_dev_bundle(
            &bundle,
            &parent_link.join("user-vst3"),
            DevInstallMode::Copy,
        )
        .expect_err("dev install should reject symlinked install dir parents")
        .to_string();

        assert!(error.contains("dev install directory parent must not be a symlink"));
        assert!(!external.join("user-vst3").exists());
    }

    #[cfg(unix)]
    #[test]
    fn install_dev_bundle_unlinks_existing_destination_symlink_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let bundle = root.join("build/VestyGain.vst3");
        let resource = bundle.join("Contents/Resources/moduleinfo.json");
        fs::create_dir_all(resource.parent().unwrap()).unwrap();
        fs::write(&resource, "version=1").unwrap();

        let install_dir = root.join("user-vst3");
        let external = root.join("external-installed");
        let destination = install_dir.join("VestyGain.vst3");
        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir(&external).unwrap();
        fs::write(external.join("keep.txt"), "do not remove\n").unwrap();
        unix_fs::symlink(&external, &destination).unwrap();

        let installed = install_dev_bundle(&bundle, &install_dir, DevInstallMode::Copy).unwrap();

        assert_eq!(installed, destination);
        assert_eq!(
            fs::read_to_string(external.join("keep.txt")).unwrap(),
            "do not remove\n"
        );
        assert!(
            !fs::symlink_metadata(&installed)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(installed.join("Contents/Resources/moduleinfo.json")).unwrap(),
            "version=1"
        );
    }

    #[test]
    fn notarization_credentials_reject_missing_or_mixed_modes() {
        assert_eq!(
            notarization_credentials(Some("Profile"), None, None, None).unwrap(),
            NotarizationCredentials::KeychainProfile("Profile".to_string())
        );
        assert_eq!(
            notarization_credentials(
                None,
                Some("dev@example.com"),
                Some("TEAMID"),
                Some("app-pass")
            )
            .unwrap(),
            NotarizationCredentials::AppleId {
                apple_id: "dev@example.com".to_string(),
                team_id: "TEAMID".to_string(),
                password: "app-pass".to_string(),
            }
        );
        assert!(
            notarization_credentials(None, Some("dev@example.com"), None, Some("app-pass"))
                .unwrap_err()
                .to_string()
                .contains("provide --keychain-profile")
        );
        assert!(
            notarization_credentials(Some("Profile"), Some("dev@example.com"), None, None)
                .unwrap_err()
                .to_string()
                .contains("not both")
        );
    }

    #[test]
    fn notarization_plan_builds_archive_submit_and_staple_commands() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let credentials = NotarizationCredentials::KeychainProfile("VestyNotary".to_string());
        let plan = notarization_plan(bundle, None, &credentials, true, true).unwrap();

        assert_eq!(plan.archive.as_str(), "/tmp/VestyGain.vst3.zip");
        assert_eq!(plan.commands.len(), 3);
        assert!(plan.commands[0].program.ends_with("ditto"));
        assert_eq!(plan.commands[0].args[0], "-c");
        assert!(plan.commands[0].args.contains(&"--keepParent".to_string()));
        assert_eq!(
            plan.commands[0].args.last().map(String::as_str),
            Some("/tmp/VestyGain.vst3.zip")
        );
        assert!(plan.commands[1].program.ends_with("xcrun"));
        assert_eq!(plan.commands[1].args[0], "notarytool");
        assert!(plan.commands[1].args.contains(&"--wait".to_string()));
        assert!(
            plan.commands[1]
                .args
                .contains(&"--keychain-profile".to_string())
        );
        assert!(plan.commands[1].args.contains(&"VestyNotary".to_string()));
        assert_eq!(
            plan.commands[2].args,
            ["stapler", "staple", bundle.as_str()]
        );
    }

    #[test]
    fn notarization_plan_supports_apple_id_without_wait_or_staple() {
        let bundle = Utf8Path::new("/tmp/VestyGain.vst3");
        let archive = Utf8Path::new("/tmp/VestyGain-notary.zip");
        let credentials = NotarizationCredentials::AppleId {
            apple_id: "dev@example.com".to_string(),
            team_id: "TEAMID".to_string(),
            password: "app-pass".to_string(),
        };
        let plan = notarization_plan(bundle, Some(archive), &credentials, false, false).unwrap();

        assert_eq!(plan.archive, archive);
        assert_eq!(plan.commands.len(), 2);
        assert!(!plan.commands[1].args.contains(&"--wait".to_string()));
        assert!(plan.commands[1].args.contains(&"--apple-id".to_string()));
        assert!(
            plan.commands[1]
                .args
                .contains(&"dev@example.com".to_string())
        );
        assert!(plan.commands[1].args.contains(&"--team-id".to_string()));
        assert!(plan.commands[1].args.contains(&"TEAMID".to_string()));
        assert!(plan.commands[1].args.contains(&"--password".to_string()));
        assert!(plan.commands[1].args.contains(&"app-pass".to_string()));

        let error = notarization_plan(bundle, Some(archive), &credentials, false, true)
            .unwrap_err()
            .to_string();
        assert!(error.contains("--no-wait"));
    }

    #[test]
    fn validate_report_and_validator_log_can_be_written_to_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("reports/validate.json");
        let validator_log_path = root.join("logs/validator.log");
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("moduleinfo.json".to_string()),
                binaries: vec!["Plugin".to_string()],
                binary_exports: Vec::new(),
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        write_validate_report(Some(&report_path), &report).unwrap();
        write_validator_log(
            Some(&validator_log_path),
            Utf8Path::new("/tools/validator"),
            Utf8Path::new("Plugin.vst3"),
            "Result: 1 tests passed, 0 tests failed\n",
            "",
        )
        .unwrap();

        let report_text = fs::read_to_string(report_path).unwrap();
        assert!(report_text.contains(r#""bundle": "Plugin.vst3""#));
        let log_text = fs::read_to_string(validator_log_path).unwrap();
        assert!(log_text.contains("validator=/tools/validator"));
        assert!(log_text.contains("[stdout]"));
    }

    #[cfg(unix)]
    #[test]
    fn report_writers_reject_symlink_output_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external.json");
        let report_path = root.join("reports/validate.json");
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, &report_path).unwrap();
        let report = ValidateReport {
            bundle: "Plugin.vst3".to_string(),
            static_check: StaticBundleCheck {
                status: "ok".to_string(),
                moduleinfo: Some("moduleinfo.json".to_string()),
                binaries: vec!["Plugin".to_string()],
                binary_exports: Vec::new(),
                parameter_manifest: None,
                asset_manifest: None,
                asset_count: 0,
                error: None,
            },
            validator: ValidatorCheck::skipped("--static-only"),
        };

        let error = write_validate_report(Some(&report_path), &report)
            .expect_err("report writer should reject symlink output")
            .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn validator_log_writer_rejects_symlink_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external.log");
        let validator_log_path = root.join("logs/validator.log");
        fs::create_dir_all(validator_log_path.parent().unwrap()).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, &validator_log_path).unwrap();

        let error = write_validator_log(
            Some(&validator_log_path),
            Utf8Path::new("/tools/validator"),
            Utf8Path::new("Plugin.vst3"),
            "Result: 1 tests passed, 0 tests failed\n",
            "",
        )
        .expect_err("validator log writer should reject symlink output")
        .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[test]
    fn daw_install_check_reports_detected_path_without_smoke_claim() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let app = root.join("REAPER.app");
        fs::create_dir(&app).unwrap();

        let check = daw_install_check("REAPER", vec![app.clone()]);
        assert_eq!(check.status, "ok");
        assert_eq!(check.value, app.to_string());
        assert!(
            check
                .hint
                .as_deref()
                .unwrap()
                .contains("smoke evidence is still required")
        );

        let missing = daw_install_check("REAPER", vec![root.join("Missing.app")]);
        assert_eq!(missing.status, "missing");
    }

    #[test]
    fn host_quirks_can_filter_by_alias() {
        let profiles = selected_host_profiles(Some("Live")).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "ableton-live");
        assert_eq!(
            profiles[0].required_smoke_checks,
            vesty_core::RELEASE_SMOKE_CHECKS
        );

        let all = selected_host_profiles(None).unwrap();
        assert_eq!(all.len(), 5);
        assert!(selected_host_profiles(Some("missing host")).is_err());
    }

    #[test]
    fn host_quirk_profiles_serialize_for_json_output() {
        let profiles = selected_host_profiles(Some("Cubase")).unwrap();
        let value = serde_json::to_value(&profiles).unwrap();
        assert_eq!(value[0]["name"], "Cubase/Nuendo");
        assert_eq!(value[0]["required_smoke_checks"][0], "scan");
        assert!(value[0]["quirks"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn host_evidence_readme_includes_profile_checks_and_quirks() {
        let readme = host_evidence_readme("Bitwig Studio");

        assert!(readme.contains("# Bitwig Studio Vesty Smoke Evidence"));
        assert!(readme.contains("Host id: `bitwig`"));
        assert!(readme.contains("Platforms: macos, windows, linux-x11"));
        assert!(readme.contains(
            "Required smoke checks: scan, load, ui, ui_host_param, meter_stream, automation, buffer_sample_rate_change, save_restore, offline_render"
        ));
        assert!(readme.contains("Wayland support is experimental"));
        assert!(readme.contains("Meter/analyzer streams are latest-wins"));
        assert!(readme.contains("Templates, pending values and `vesty doctor` install detection do not count as pass evidence."));
        assert!(readme.contains("Accepted Pass Markers"));
        assert!(readme.contains("scan=true"));
        assert!(readme.contains("ui_host_param=true"));
        assert!(readme.contains("meter_flush sent=1"));
        assert!(readme.contains("buffer_sample_rate_change=true"));
        assert!(readme.contains("render_file=/absolute/path/to/rendered.wav"));
        assert!(readme.contains("vesty daw-matrix --evidence-root target/daw-evidence --strict"));
    }

    #[test]
    fn release_check_passes_with_complete_matrix_and_protocol_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let protocol = Utf8PathBuf::from_path_buf(temp.path().join("protocol")).unwrap();
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();

        let report =
            build_release_check_report(rows, &protocol, false, &ReleaseEvidenceOptions::default());

        assert!(
            release_check_complete(&report),
            "expected complete release check, failed checks: {}",
            report
                .checks
                .iter()
                .filter(|check| check.status == "failed")
                .map(|check| format!("{} = {}", check.name, check.value))
                .collect::<Vec<_>>()
                .join("; ")
        );
        assert_eq!(report.status, "ok");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "protocol snapshot" && check.status == "ok")
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 binding baseline"
                && check.status == "ok"
                && check.value.contains("v3.8.0_build_66")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "ci doctor artifacts" && check.status == "skipped" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK header manifest" && check.status == "skipped"
        }));
    }

    #[test]
    fn release_check_reports_missing_host_evidence() {
        let rows = vec![
            complete_release_row("REAPER"),
            serde_json::json!({
                "host": "Cubase/Nuendo",
                "scan": true,
                "load": false,
                "ui": false,
                "ui_host_param": false,
                "meter_stream": false,
                "automation": false,
                "buffer_sample_rate_change": false,
                "save_restore": false,
                "offline_render": false,
                "evidence": "target/cubase-smoke",
            }),
        ];

        let report = build_release_check_report(
            rows,
            Utf8Path::new("target/missing-protocol"),
            true,
            &ReleaseEvidenceOptions::default(),
        );

        assert!(!release_check_complete(&report));
        assert_eq!(report.status, "failed");
        assert!(report.checks.iter().any(|check| {
            check.name == "daw smoke: Cubase/Nuendo"
                && check.status == "failed"
                && check.value.contains("load")
        }));
    }

    #[test]
    fn release_action_plan_lists_required_and_optional_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                if profile.name == "Cubase/Nuendo" {
                    serde_json::json!({
                        "host": "Cubase/Nuendo",
                        "platform": "macOS test platform",
                        "platform_supported": true,
                        "scan": true,
                        "load": false,
                        "ui": false,
                        "ui_host_param": false,
                        "meter_stream": false,
                        "automation": false,
                        "buffer_sample_rate_change": false,
                        "save_restore": false,
                        "offline_render": false,
                        "evidence": evidence_root.join("cubase").to_string(),
                    })
                } else {
                    complete_release_row(profile.name)
                }
            })
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let report = build_release_check_report(rows, &protocol, true, &options);

        assert!(!release_check_complete(&report));
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        assert_eq!(plan.version, 1);
        assert_eq!(plan.status, "failed");
        assert_eq!(
            plan.summary.action_count,
            report
                .checks
                .iter()
                .filter(|check| check.status != "ok")
                .count()
        );
        let daw_matrix = plan
            .actions
            .iter()
            .find(|action| action.check == "daw matrix")
            .expect("daw matrix action");
        assert_eq!(
            daw_matrix.evidence_path.as_deref(),
            Some(portable_report_path(&evidence_root).as_str())
        );

        let cubase = plan
            .actions
            .iter()
            .find(|action| action.check == "daw smoke: Cubase/Nuendo")
            .expect("cubase action");
        assert_eq!(cubase.priority, "required");
        assert_eq!(
            cubase.evidence_path.as_deref(),
            Some(portable_report_path(&evidence_root.join("cubase")).as_str())
        );
        assert!(cubase.commands.iter().any(|command| {
            command.contains("vesty daw-matrix --write-report --host cubase-nuendo")
        }));

        let platform = plan
            .actions
            .iter()
            .find(|action| action.check == "platform smoke artifacts")
            .expect("platform action");
        assert_eq!(platform.priority, "required");
        assert_eq!(
            platform.evidence_path.as_deref(),
            Some(portable_report_path(&release_evidence_dir.join("platform-smoke")).as_str())
        );
        assert!(
            platform
                .commands
                .iter()
                .any(|command| command.contains("vesty platform-smoke --write-report"))
        );
        let platform_commands = platform.commands.join("\n");
        assert!(platform_commands.contains("--platform macos"));
        assert!(platform_commands.contains("WebKit.framework loaded"));
        assert!(platform_commands.contains("--platform windows-x64"));
        assert!(platform_commands.contains("WebView2 runtime loaded"));
        assert!(platform_commands.contains("--platform linux-x11"));
        assert!(platform_commands.contains("WebKitGTK loaded; X11 display active"));
        assert!(!platform_commands.contains("system_webview=true"));
        assert!(!platform_commands.contains("vst3_validator=true"));

        let crate_package = plan
            .actions
            .iter()
            .find(|action| action.check == "crate package readiness")
            .expect("crate package readiness action");
        assert_eq!(crate_package.status, "failed");
        assert_eq!(crate_package.priority, "required");
        assert_eq!(
            crate_package.evidence_path.as_deref(),
            Some(
                portable_report_path(
                    &release_evidence_dir.join("crate-package/crate-package.json")
                )
                .as_str()
            )
        );
        assert!(
            crate_package
                .commands
                .iter()
                .any(|command| command.contains("vesty crate-package --out"))
        );
        assert!(
            crate_package
                .commands
                .iter()
                .any(|command| command.contains("vesty crate-package --check --out"))
        );

        let sdk = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 SDK header manifest")
            .expect("sdk action");
        assert_eq!(sdk.status, "skipped");
        assert_eq!(sdk.priority, "optional");
        assert!(
            sdk.commands
                .iter()
                .any(|command| command.contains("vesty vst3-sdk manifest"))
        );
        assert!(
            sdk.commands
                .iter()
                .any(|command| command.contains("vesty vst3-sdk manifest")
                    && command.contains("--check"))
        );

        let plan_path = root.join("release-action-plan.json");
        write_release_action_plan(&plan_path, &plan).unwrap();
        let decoded: ReleaseActionPlan =
            serde_json::from_str(&fs::read_to_string(plan_path).unwrap()).unwrap();
        assert_eq!(decoded, plan);
    }

    #[test]
    fn release_action_plan_uses_default_release_evidence_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                serde_json::json!({
                    "host": profile.name,
                    "scan": false,
                    "load": false,
                    "ui": false,
                    "ui_host_param": false,
                    "meter_stream": false,
                    "automation": false,
                    "buffer_sample_rate_change": false,
                    "save_restore": false,
                    "offline_render": false,
                    "evidence": Utf8PathBuf::from("target/daw-evidence").join(profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let report = build_release_check_report(rows, &protocol, true, &options);
        let plan = build_release_action_plan(&report, &protocol, None, None);

        let daw_matrix = plan
            .actions
            .iter()
            .find(|action| action.check == "daw matrix")
            .expect("daw matrix action");
        assert_eq!(
            daw_matrix.evidence_path.as_deref(),
            Some("target/daw-evidence")
        );

        let protocol_action = plan
            .actions
            .iter()
            .find(|action| action.check == "protocol snapshot")
            .expect("protocol snapshot action");
        assert_eq!(
            protocol_action.evidence_path.as_deref(),
            Some(portable_report_path(&protocol).as_str())
        );

        let ci_run_url = plan
            .actions
            .iter()
            .find(|action| action.check == "ci run url")
            .expect("ci run url action");
        assert_eq!(
            ci_run_url.evidence_path.as_deref(),
            Some("target/release-evidence/ci-run-url.txt")
        );
        assert!(ci_run_url.commands.iter().any(|command| command
            == "vesty release-check --write-evidence-template target/release-evidence"));

        let crate_package = plan
            .actions
            .iter()
            .find(|action| action.check == "crate package readiness")
            .expect("crate package readiness action");
        assert_eq!(
            crate_package.evidence_path.as_deref(),
            Some("target/release-evidence/crate-package/crate-package.json")
        );

        let generic_validator = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 validate reports")
            .expect("vst3 validate reports action");
        assert_eq!(
            generic_validator.evidence_path.as_deref(),
            Some("target/release-evidence/validator")
        );
        assert!(
            generic_validator
                .commands
                .iter()
                .any(|command| command.contains("vesty validate <bundle.vst3> --strict"))
        );
        assert_example_validator_matrix_commands(generic_validator);

        let validator = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 example validator coverage")
            .expect("vst3 example validator coverage action");
        assert_eq!(
            validator.evidence_path.as_deref(),
            Some("target/release-evidence/validator")
        );
        assert!(
            validator
                .commands
                .iter()
                .any(|command| command.contains("target/release-evidence/validator/"))
        );
        assert!(
            validator
                .commands
                .iter()
                .any(|command| { command.contains("vesty validate <bundle.vst3> --strict") })
        );
        assert_example_validator_matrix_commands(validator);

        let generic_static_validate = plan
            .actions
            .iter()
            .find(|action| action.check == "vst3 static validate reports")
            .expect("vst3 static validate reports action");
        assert_eq!(
            generic_static_validate.evidence_path.as_deref(),
            Some("target/release-evidence/package")
        );
        assert!(generic_static_validate.commands.iter().any(|command| {
            command.contains("vesty validate <bundle.vst3> --static-only --strict")
        }));
        assert_example_static_matrix_commands(generic_static_validate);

        let static_validate = plan
            .actions
            .iter()
            .find(|action| action.check == "ci example static validate coverage")
            .expect("ci example static validate coverage action");
        assert_eq!(
            static_validate.evidence_path.as_deref(),
            Some("target/release-evidence/package")
        );
        assert!(
            static_validate
                .commands
                .iter()
                .any(|command| command.contains("target/release-evidence/package/"))
        );
        assert!(static_validate.commands.iter().any(|command| {
            command.contains("vesty validate <bundle.vst3> --static-only --strict")
        }));
        assert_example_static_matrix_commands(static_validate);

        let signing = plan
            .actions
            .iter()
            .find(|action| action.check == "signed bundle evidence")
            .expect("signed bundle evidence action");
        assert_eq!(
            signing.evidence_path.as_deref(),
            Some(
                "target/release-evidence/signing-macos.log and target/release-evidence/signing-windows.log"
            )
        );

        let notarization = plan
            .actions
            .iter()
            .find(|action| action.check == "notarization log")
            .expect("notarization log action");
        assert_eq!(
            notarization.evidence_path.as_deref(),
            Some("target/release-evidence/notary.log")
        );
    }

    fn assert_example_validator_matrix_commands(action: &ReleaseActionItem) {
        let matrix_commands = action
            .commands
            .iter()
            .filter(|command| command.contains("<path-to-"))
            .collect::<Vec<_>>();
        assert_eq!(
            matrix_commands.len(),
            REQUIRED_EXAMPLE_BUNDLES.len() * REQUIRED_EXAMPLE_VALIDATE_PLATFORMS.len(),
            "{} should include one concrete validator command per example/platform",
            action.check
        );

        for platform in REQUIRED_EXAMPLE_VALIDATE_PLATFORMS {
            for bundle in REQUIRED_EXAMPLE_BUNDLES {
                let report =
                    format!("target/release-evidence/validator/{bundle}.{platform}.validate.json");
                let log =
                    format!("target/release-evidence/validator/{bundle}.{platform}.validator.log");
                let expected = matrix_commands
                    .iter()
                    .filter(|command| {
                        command.contains(&format!("vesty validate <path-to-{bundle}> --strict"))
                            && command.contains(&report)
                            && command.contains(&log)
                            && !command.contains("--static-only")
                    })
                    .count();
                assert_eq!(
                    expected, 1,
                    "{} should include exactly one strict validator command for {bundle} on {platform}",
                    action.check
                );
            }
        }
    }

    fn assert_example_static_matrix_commands(action: &ReleaseActionItem) {
        let matrix_commands = action
            .commands
            .iter()
            .filter(|command| command.contains("<path-to-"))
            .collect::<Vec<_>>();
        assert_eq!(
            matrix_commands.len(),
            REQUIRED_EXAMPLE_BUNDLES.len() * REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS.len(),
            "{} should include one concrete static validate command per example/platform",
            action.check
        );

        for platform in REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS {
            for bundle in REQUIRED_EXAMPLE_BUNDLES {
                let report = format!(
                    "target/release-evidence/package/{bundle}.{platform}.static-validate.json"
                );
                let expected = matrix_commands
                    .iter()
                    .filter(|command| {
                        command.contains(&format!(
                            "vesty validate <path-to-{bundle}> --static-only --strict"
                        )) && command.contains(&report)
                            && !command.contains("--validator-log")
                    })
                    .count();
                assert_eq!(
                    expected, 1,
                    "{} should include exactly one strict static validate command for {bundle} on {platform}",
                    action.check
                );
            }
        }
    }

    #[test]
    fn release_action_plan_daw_host_commands_are_accepted_by_writer() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
        let evidence = resolve_daw_evidence_paths(
            Some(evidence_root.clone()),
            Utf8PathBuf::from("ignored/reaper"),
            Utf8PathBuf::from("ignored/cubase"),
            Utf8PathBuf::from("ignored/bitwig"),
            Utf8PathBuf::from("ignored/ableton"),
            Utf8PathBuf::from("ignored/studio-one"),
        );
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                serde_json::json!({
                    "host": profile.name,
                    "scan": false,
                    "load": false,
                    "ui": false,
                    "ui_host_param": false,
                    "meter_stream": false,
                    "automation": false,
                    "buffer_sample_rate_change": false,
                    "save_restore": false,
                    "offline_render": false,
                    "evidence": daw_evidence_dir_for_host(&evidence, profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let report =
            build_release_check_report(rows, &protocol, true, &ReleaseEvidenceOptions::default());
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        let mut accepted_hosts = BTreeSet::new();
        for action in plan
            .actions
            .iter()
            .filter(|action| action.check.starts_with("daw smoke: "))
        {
            let command = action.commands.first().expect("daw smoke command");
            let host = command_arg(command, "--host").expect("host argument");
            write_daw_smoke_report(
                &evidence,
                DawSmokeReportInput {
                    host: Some(host.to_string()),
                    platform: Some("macos arm64 / test host version".to_string()),
                    scan: Some("scan=true".to_string()),
                    load: Some("load=true".to_string()),
                    ui: Some("ui=true".to_string()),
                    ui_host_param: Some("ui_host_param=true".to_string()),
                    meter_stream: Some("meter_flush sent=1".to_string()),
                    automation: Some("automation=true".to_string()),
                    buffer_sample_rate_change: Some("buffer_sample_rate_change=true".to_string()),
                    save_restore: Some("save_restore=true".to_string()),
                    offline_render: Some("offline_render=true".to_string()),
                },
            )
            .unwrap();
            accepted_hosts.insert(host.to_string());
        }

        assert_eq!(accepted_hosts.len(), vesty_core::host_profiles().len());
        assert!(accepted_hosts.contains("cubase-nuendo"));
        assert!(accepted_hosts.contains("ableton-live"));
        assert!(accepted_hosts.contains("studio-one"));
        assert!(daw_matrix_complete(&daw_matrix_rows(&evidence)));
    }

    #[test]
    fn release_action_plan_vesty_commands_parse_with_current_cli() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let protocol = root.join("vesty-protocol");
        let evidence_root = root.join("daw-evidence");
        let release_evidence_dir = root.join("release-evidence");
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| {
                serde_json::json!({
                    "host": profile.name,
                    "scan": false,
                    "load": false,
                    "ui": false,
                    "ui_host_param": false,
                    "meter_stream": false,
                    "automation": false,
                    "buffer_sample_rate_change": false,
                    "save_restore": false,
                    "offline_render": false,
                    "evidence": evidence_root.join(profile.id).to_string(),
                })
            })
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };
        let report = build_release_check_report(rows, &protocol, true, &options);
        let plan = build_release_action_plan(
            &report,
            &protocol,
            Some(&evidence_root),
            Some(&release_evidence_dir),
        );

        let mut parsed = 0;
        for action in &plan.actions {
            for command in &action.commands {
                let command = command.trim();
                if !release_action_command_starts_with_vesty(command) {
                    continue;
                }
                let argv = split_release_action_command(command).unwrap_or_else(|error| {
                    panic!(
                        "action `{}` command should split with production parser: {command}\nerror: {error}",
                        action.check
                    )
                });
                assert!(
                    Cli::try_parse_from(&argv).is_ok(),
                    "action `{}` command should parse with current CLI: {command}\nargv: {argv:?}",
                    action.check
                );
                parsed += 1;
            }
        }

        assert!(parsed >= 20, "expected broad action command coverage");
        assert!(plan.actions.iter().any(|action| {
            action.check == "notarization log"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("--notary-log <notarytool.log>"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "crate publish plan"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("publish-plan --check"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "npm package pack report"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("npm-pack --check"))
        }));
        assert!(plan.actions.iter().any(|action| {
            action.check == "dependency latest baseline"
                && action
                    .commands
                    .iter()
                    .any(|command| command.contains("dependency-baseline --latest --check"))
        }));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_incomplete_actions() {
        let mut missing_command = test_release_action_plan();
        missing_command.actions[0].commands.clear();

        let error = validate_release_action_plan_sidecar(&missing_command).unwrap_err();
        assert!(error.contains("has no suggested commands"));

        let mut empty_protocol = test_release_action_plan();
        empty_protocol.protocol_snapshot = " ".to_string();
        let error = validate_release_action_plan_sidecar(&empty_protocol).unwrap_err();
        assert!(error.contains("release action plan protocol snapshot must not be empty"));

        let mut long_value = test_release_action_plan();
        long_value.actions[0].value = "x".repeat(RELEASE_ACTION_TEXT_MAX_BYTES + 1);
        let error = validate_release_action_plan_sidecar(&long_value).unwrap_err();
        assert!(error.contains("value must be at most"));

        let mut bad_evidence_root = test_release_action_plan();
        bad_evidence_root.evidence_root = Some("target/daw-evidence\nbad".to_string());
        let error = validate_release_action_plan_sidecar(&bad_evidence_root).unwrap_err();
        assert!(error.contains("release action plan evidence root must not contain control"));

        let mut escaping_protocol = test_release_action_plan();
        escaping_protocol.protocol_snapshot = "target/vesty-protocol/..".to_string();
        let error = validate_release_action_plan_sidecar(&escaping_protocol).unwrap_err();
        assert!(error.contains("release action plan protocol snapshot"));
        assert!(error.contains("parent-directory components"));

        let mut escaping_evidence_root = test_release_action_plan();
        escaping_evidence_root.evidence_root = Some("target/daw-evidence/..".to_string());
        let error = validate_release_action_plan_sidecar(&escaping_evidence_root).unwrap_err();
        assert!(error.contains("release action plan evidence root"));
        assert!(error.contains("parent-directory components"));

        let mut bad_release_evidence_dir = test_release_action_plan();
        bad_release_evidence_dir.release_evidence_dir =
            Some("target/release-evidence\nbad".to_string());
        let error = validate_release_action_plan_sidecar(&bad_release_evidence_dir).unwrap_err();
        assert!(
            error.contains("release action plan release evidence dir must not contain control")
        );

        let mut escaping_release_evidence_dir = test_release_action_plan();
        escaping_release_evidence_dir.release_evidence_dir =
            Some("target/release-evidence/..".to_string());
        let error =
            validate_release_action_plan_sidecar(&escaping_release_evidence_dir).unwrap_err();
        assert!(error.contains("release action plan release evidence dir"));
        assert!(error.contains("parent-directory components"));

        let mut control_path = test_release_action_plan();
        control_path.actions[0].evidence_path = Some("target/daw-evidence/reaper\nbad".to_string());

        let error = validate_release_action_plan_sidecar(&control_path).unwrap_err();
        assert!(error.contains("evidence path must not contain control characters"));

        let mut escaping_action_path = test_release_action_plan();
        escaping_action_path.actions[0].evidence_path =
            Some("target/daw-evidence/reaper/../cubase".to_string());
        let error = validate_release_action_plan_sidecar(&escaping_action_path).unwrap_err();
        assert!(error.contains("release action `daw smoke: REAPER` evidence path"));
        assert!(error.contains("parent-directory components"));

        let mut wrong_daw_evidence_path = test_release_action_plan();
        wrong_daw_evidence_path.actions[0].evidence_path =
            Some("target/other-daw-evidence/reaper".to_string());
        let error = validate_release_action_plan_sidecar(&wrong_daw_evidence_path).unwrap_err();
        assert!(error.contains("evidence path"));
        assert!(error.contains("does not match expected `target/daw-evidence/reaper`"));

        let mut missing_evidence_path = test_release_action_plan();
        missing_evidence_path.actions[0].evidence_path = None;
        let error = validate_release_action_plan_sidecar(&missing_evidence_path).unwrap_err();
        assert!(error.contains("missing expected evidence path"));

        let mut wrong_release_evidence_path = test_release_action_plan();
        wrong_release_evidence_path.actions[1].evidence_path =
            Some("target/other-release-evidence/vst3-sdk".to_string());
        let error = validate_release_action_plan_sidecar(&wrong_release_evidence_path).unwrap_err();
        assert!(error.contains("does not match expected `target/release-evidence/vst3-sdk"));

        let mut protocol_action = test_release_action_plan();
        protocol_action.actions[0] = ReleaseActionItem {
            check: "protocol snapshot".to_string(),
            status: "failed".to_string(),
            priority: "required".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: Some("target/other-protocol".to_string()),
            commands: vec!["vesty export-types --out target/vesty-protocol".to_string()],
        };
        let error = validate_release_action_plan_sidecar(&protocol_action).unwrap_err();
        assert!(error.contains("does not match expected `target/vesty-protocol`"));

        let mut unsafe_unicode_hint = test_release_action_plan();
        unsafe_unicode_hint.actions[0].hint = Some("collect evidence\u{202e}hidden".to_string());
        let error = validate_release_action_plan_sidecar(&unsafe_unicode_hint).unwrap_err();
        assert!(error.contains("hint must not contain unsafe Unicode format characters"));

        let mut duplicate_action = test_release_action_plan();
        duplicate_action.actions[1].check = duplicate_action.actions[0].check.clone();
        duplicate_action.actions[1].evidence_path =
            duplicate_action.actions[0].evidence_path.clone();
        let error = validate_release_action_plan_sidecar(&duplicate_action).unwrap_err();
        assert!(error.contains("duplicate release action check"));

        let mut bad_pending_count = test_release_action_plan();
        bad_pending_count.summary.action_count += 1;
        bad_pending_count.actions.push(ReleaseActionItem {
            check: "manual follow-up".to_string(),
            status: "ok".to_string(),
            priority: "optional".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: None,
            commands: vec!["vesty release-check --strict".to_string()],
        });
        let error = validate_release_action_plan_sidecar(&bad_pending_count).unwrap_err();
        assert!(error.contains("action pending count mismatch"));

        let mut too_many_commands = test_release_action_plan();
        too_many_commands.actions[0].commands =
            vec!["vesty release-check --strict".to_string(); RELEASE_ACTION_MAX_COMMANDS + 1];
        let error = validate_release_action_plan_sidecar(&too_many_commands).unwrap_err();
        assert!(error.contains("too many suggested commands"));

        let mut absurd_summary = test_release_action_plan();
        absurd_summary.summary.ok = RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS;
        let error = validate_release_action_plan_sidecar(&absurd_summary).unwrap_err();
        assert!(error.contains("summary check count"));
        assert!(error.contains("exceeds maximum"));

        let mut missing_current_gate = test_release_action_plan();
        missing_current_gate.summary.ok -= 1;
        let error = validate_release_action_plan_sidecar(&missing_current_gate).unwrap_err();
        assert!(error.contains("summary check count must match current Vesty release gate"));

        let mut unknown_action = test_release_action_plan();
        unknown_action.actions[0].check = "manual follow-up".to_string();
        unknown_action.actions[0].evidence_path = None;
        let error = validate_release_action_plan_sidecar(&unknown_action).unwrap_err();
        assert!(error.contains("unknown release action check `manual follow-up`"));
    }

    #[test]
    fn release_action_plan_sidecar_accepts_signed_bundle_compound_evidence_path() {
        let mut plan = test_release_action_plan();
        plan.summary.ok = expected_release_check_names().len() - 1;
        plan.summary.failed = 1;
        plan.summary.skipped = 0;
        plan.summary.action_count = 1;
        plan.actions = vec![ReleaseActionItem {
            check: "signed bundle evidence".to_string(),
            status: "failed".to_string(),
            priority: "required".to_string(),
            value: "missing signed bundle evidence".to_string(),
            hint: None,
            evidence_path: Some(
                "target/release-evidence/signing-macos.log and target/release-evidence/signing-windows.log"
                    .to_string(),
            ),
            commands: vec![
                "vesty release-evidence collect-signing <signed-macos-bundle.vst3> --platform macos --dir target/release-evidence"
                    .to_string(),
            ],
        }];

        validate_release_action_plan_sidecar(&plan).unwrap();
    }

    #[test]
    fn release_action_plan_writer_rejects_invalid_plan() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let plan_path = root.join("release-action-plan.json");
        let mut plan = test_release_action_plan();
        plan.actions[0].commands.clear();

        let error = write_release_action_plan(&plan_path, &plan).unwrap_err();
        assert!(error.to_string().contains("invalid release action plan"));
        assert!(error.to_string().contains("has no suggested commands"));
        assert!(!plan_path.exists());
    }

    #[test]
    fn release_action_plan_sidecar_rejects_stale_vesty_commands() {
        let mut stale_command = test_release_action_plan();
        stale_command.actions[0].commands = vec![
            "  vesty release-evidence collect-notarization --notarytool-log notarytool.log --stapler-log stapler.log --dir target/release-evidence"
                .to_string(),
        ];

        let error = validate_release_action_plan_sidecar(&stale_command).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));
        assert!(error.contains("--notarytool-log"));

        let mut bare_vesty = test_release_action_plan();
        bare_vesty.actions[0].commands = vec!["  vesty  ".to_string()];
        let error = validate_release_action_plan_sidecar(&bare_vesty).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));

        let mut unicode_space = test_release_action_plan();
        unicode_space.actions[0].commands = vec![
            "vesty\u{00a0}release-evidence collect-notarization --notarytool-log notarytool.log"
                .to_string(),
        ];
        let error = validate_release_action_plan_sidecar(&unicode_space).unwrap_err();
        assert!(error.contains("does not parse with current CLI"));
        assert!(error.contains("--notarytool-log"));

        let mut unterminated_quote = test_release_action_plan();
        unterminated_quote.actions[0].commands =
            vec!["vesty daw-matrix --write-report --host reaper --ui \"ui=true".to_string()];
        let error = validate_release_action_plan_sidecar(&unterminated_quote).unwrap_err();
        assert!(error.contains("unterminated double quote"));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_failed_empty_plan() {
        let mut empty = test_release_action_plan();
        empty.summary.failed = 0;
        empty.summary.skipped = 0;
        empty.summary.ok = expected_release_check_names().len();
        empty.summary.action_count = 0;
        empty.actions.clear();

        let error = validate_release_action_plan_sidecar(&empty).unwrap_err();
        assert!(error.contains("failed release action plan must contain at least one action"));

        let mut empty_ok = test_release_action_plan();
        empty_ok.status = "ok".to_string();
        empty_ok.summary.ok = 0;
        empty_ok.summary.failed = 0;
        empty_ok.summary.skipped = 0;
        empty_ok.summary.action_count = 0;
        empty_ok.actions.clear();
        let error = validate_release_action_plan_sidecar(&empty_ok).unwrap_err();
        assert!(error.contains("summary must contain at least one check"));

        let mut skipped_only = test_release_action_plan();
        skipped_only.summary.ok = expected_release_check_names().len() - 1;
        skipped_only.summary.failed = 0;
        skipped_only.summary.skipped = 1;
        skipped_only.actions = vec![ReleaseActionItem {
            check: "vst3 SDK header manifest".to_string(),
            status: "skipped".to_string(),
            priority: "optional".to_string(),
            value: "not requested".to_string(),
            hint: Some("optional generated-headers audit".to_string()),
            evidence_path: Some(
                "target/release-evidence/vst3-sdk/vst3-sdk-headers.json".to_string(),
            ),
            commands: vec!["vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK".to_string()],
        }];
        skipped_only.summary.action_count = skipped_only.actions.len();

        let error = validate_release_action_plan_sidecar(&skipped_only).unwrap_err();
        assert!(
            error.contains("failed release action plan must contain at least one failed action")
        );

        let mut no_pending_actions = test_release_action_plan();
        no_pending_actions.summary.failed = 0;
        no_pending_actions.summary.skipped = 0;
        no_pending_actions.actions.clear();
        no_pending_actions.actions.push(ReleaseActionItem {
            check: "manual follow-up".to_string(),
            status: "ok".to_string(),
            priority: "optional".to_string(),
            value: "missing".to_string(),
            hint: None,
            evidence_path: None,
            commands: vec!["vesty release-check --strict".to_string()],
        });
        no_pending_actions.summary.action_count = no_pending_actions.actions.len();

        let error = validate_release_action_plan_sidecar(&no_pending_actions).unwrap_err();
        assert!(error.contains("action pending count mismatch"));
    }

    #[test]
    fn release_action_plan_sidecar_rejects_unknown_json_fields() {
        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["summary"]["pending"] = serde_json::json!(1);
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `pending`"));

        let mut plan = serde_json::to_value(test_release_action_plan()).unwrap();
        plan["actions"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<ReleaseActionPlan>(plan).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));
    }

    #[test]
    fn release_check_requires_release_artifacts_when_requested() {
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        let report = build_release_check_report(rows, Utf8Path::new("unused"), true, &options);

        assert!(!release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci doctor artifacts"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci release-check artifacts"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "crate publish plan" && check.status == "failed" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "crate package readiness"
                && check.status == "failed"
                && check.value.contains("required")
        }));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "npm package pack report" && check.status == "failed"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "dependency latest baseline" && check.status == "failed"
        }));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "signed bundle evidence" && check.status == "failed"
            })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "notarization log" && check.status == "failed" })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 validate reports" && check.status == "failed" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 static validate reports" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "protocol snapshot"
                && check.status == "failed"
                && check.value.contains("cannot skip protocol snapshot")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK header manifest" && check.status == "skipped"
        }));
    }

    #[test]
    fn publish_plan_release_check_validates_dependency_order() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("publish-plan.json");
        write_publish_plan_artifact(&valid);

        let check = publish_plan_release_check(Some(&valid), true);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 publishable crates"));
        assert!(check.value.contains("final crate: vesty"));

        let invalid = root.join("bad-publish-plan.json");
        let mut plan = test_publish_plan();
        plan.packages[1]
            .internal_dependencies
            .push("vesty".to_string());
        fs::write(&invalid, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

        let check = publish_plan_release_check(Some(&invalid), true);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("orders dependency vesty after dependent vesty-core")
        );

        let missing = publish_plan_release_check(None, false);
        assert_eq!(missing.status, "skipped");
        let required = publish_plan_release_check(None, true);
        assert_eq!(required.status, "failed");
    }

    #[test]
    fn publish_plan_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("publish-plan.json");
        write_publish_plan_artifact(&valid);

        run_publish_plan(&root, Some(&valid), true, "text").unwrap();
        run_publish_plan(&root, Some(&valid), true, "json").unwrap();

        let error = run_publish_plan(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));
    }

    #[test]
    fn publish_plan_report_rejects_unknown_json_fields() {
        let unknown_top_level = r#"{
          "packages": [],
          "skipped_private": [],
          "generated_by": "manual"
        }"#;
        let error = serde_json::from_str::<PublishPlan>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generated_by`"));

        let unknown_package_field = r#"{
          "packages": [{
            "order": 1,
            "level": 1,
            "name": "vesty-params",
            "version": "0.1.0",
            "manifest_path": "/workspace/crates/vesty-params/Cargo.toml",
            "internal_dependencies": [],
            "checksum": "hidden"
          }],
          "skipped_private": []
        }"#;
        let error = serde_json::from_str::<PublishPlan>(unknown_package_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));
    }

    #[test]
    fn crate_package_release_check_validates_packaged_and_deferred_entries() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("crate-package.json");
        let valid_publish_plan = root.join("publish-plan.json");
        write_crate_package_artifact(&valid);
        write_publish_plan_artifact(&valid_publish_plan);

        let check = crate_package_release_check(Some(&valid), Some(&valid_publish_plan), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 workspace crates"));
        assert!(check.value.contains("1 packageable now"));
        assert!(check.value.contains("2 deferred"));

        let missing = crate_package_release_check(None, None, false);
        assert_eq!(missing.status, "skipped");
        let required = crate_package_release_check(None, None, true);
        assert_eq!(required.status, "failed");

        let invalid = root.join("bad-crate-package.json");
        let mut report = test_crate_package_report();
        report.packages[0].status = "deferred".to_string();
        report.packages[0].reason = Some("not actually packageable".to_string());
        fs::write(&invalid, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = crate_package_release_check(Some(&invalid), Some(&valid_publish_plan), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("vesty-params"));
        assert!(check.value.contains("expected packaged"));
    }

    #[test]
    fn crate_package_release_check_rejects_mismatched_publish_plan_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let crate_package = root.join("crate-package.json");
        let publish_plan = root.join("publish-plan.json");
        write_crate_package_artifact(&crate_package);

        let mut mismatched_plan = test_publish_plan();
        mismatched_plan.packages[1].version = "0.2.0".to_string();
        mismatched_plan.packages[1].manifest_path =
            "/workspace/crates/renamed-core/Cargo.toml".to_string();
        write_publish_plan_artifact_with_plan(&publish_plan, &mismatched_plan);

        let check = crate_package_release_check(Some(&crate_package), Some(&publish_plan), true);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("does not match crate publish plan evidence")
        );
        assert!(check.value.contains("vesty-core"));
        assert!(check.value.contains("0.2.0"));
        assert!(check.value.contains("manifest path"));
    }

    #[test]
    fn crate_package_report_rejects_entries_out_of_sync_with_embedded_publish_plan() {
        let mut missing_entry = test_crate_package_report();
        missing_entry.packages.pop();
        let error = validate_crate_package_report(&missing_entry).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package report is out of sync with embedded publish plan")
        );
        assert!(
            error
                .to_string()
                .contains("embedded publish plan package vesty is missing")
        );

        let mut extra_entry = test_crate_package_report();
        extra_entry.packages.push(CratePackageEntry {
            name: "vesty-extra".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: "/workspace/crates/vesty-extra/Cargo.toml".to_string(),
            publish_order: 4,
            internal_dependencies: Vec::new(),
            status: "packaged".to_string(),
            reason: None,
        });
        let error = validate_crate_package_report(&extra_entry).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package entry vesty-extra is not present")
        );

        let mut drifted_entry = test_crate_package_report();
        drifted_entry.packages[1].manifest_path =
            "/workspace/crates/renamed-core/Cargo.toml".to_string();
        let error = validate_crate_package_report(&drifted_entry).unwrap_err();
        assert!(error.to_string().contains("manifest path"));
        assert!(error.to_string().contains("vesty-core"));
    }

    #[test]
    fn crate_package_report_rejects_unknown_json_fields() {
        let mut report = serde_json::to_value(test_crate_package_report()).unwrap();
        report["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<CratePackageReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut report = serde_json::to_value(test_crate_package_report()).unwrap();
        report["packages"][0]["checksum"] = serde_json::json!("hidden");
        let error = serde_json::from_value::<CratePackageReport>(report).unwrap_err();
        assert!(error.to_string().contains("unknown field `checksum`"));
    }

    #[test]
    fn crate_package_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("crate-package.json");
        write_crate_package_artifact(&valid);

        run_crate_package(&root, Some(&valid), true, "text").unwrap();
        run_crate_package(&root, Some(&valid), true, "json").unwrap();

        let error = run_crate_package(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));
    }

    #[test]
    fn npm_pack_release_check_validates_workspace_package_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("npm-pack.json");
        write_npm_pack_artifact(&valid);

        let check = npm_pack_release_check(Some(&valid), true);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("1 package"));
        assert!(check.value.contains("vesty-plugin-ui"));

        let missing = npm_pack_release_check(None, false);
        assert_eq!(missing.status, "skipped");
        let required = npm_pack_release_check(None, true);
        assert_eq!(required.status, "failed");

        let mut report = test_npm_pack_report();
        report[0].files.push(NpmPackFile {
            path: "src/index.ts".to_string(),
        });
        let invalid = root.join("bad-npm-pack.json");
        fs::write(&invalid, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = npm_pack_release_check(Some(&invalid), true);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("non-release file"));
        assert!(check.value.contains("src/index.ts"));
    }

    #[test]
    fn npm_pack_report_rejects_unknown_json_fields() {
        let unknown_entry = r#"[
          {
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [{ "path": "package.json" }, { "path": "dist/index.js" }],
            "scripts": { "prepack": "echo hidden" }
          }
        ]"#;
        let error = parse_npm_pack_report_text(unknown_entry).unwrap_err();
        assert!(error.to_string().contains("unknown field `scripts`"));

        let unknown_file = r#"[
          {
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [{ "path": "package.json", "mode": 420 }]
          }
        ]"#;
        let error = parse_npm_pack_report_text(unknown_file).unwrap_err();
        assert!(error.to_string().contains("unknown field `mode`"));
    }

    #[test]
    fn npm_pack_command_output_normalizes_external_metadata() {
        let command_output = r#"[
          {
            "id": "vesty-plugin-ui@0.1.0",
            "name": "vesty-plugin-ui",
            "version": "0.1.0",
            "size": 1024,
            "unpackedSize": 4096,
            "shasum": "abc",
            "integrity": "sha512-abc",
            "filename": "vesty-plugin-ui-0.1.0.tgz",
            "files": [
              { "path": "package.json", "size": 512, "mode": 420 },
              { "path": "dist/index.js", "size": 512, "mode": 420 }
            ],
            "entryCount": 2,
            "bundled": []
          }
        ]"#;

        let entries = parse_npm_pack_command_output(command_output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "vesty-plugin-ui");
        assert_eq!(entries[0].files.len(), 2);
        assert_eq!(entries[0].files[1].path, "dist/index.js");

        let normalized = serde_json::to_value(entries).unwrap();
        assert_eq!(normalized[0]["name"], "vesty-plugin-ui");
        assert_eq!(normalized[0]["files"][0]["path"], "package.json");
        assert!(normalized[0].get("id").is_none());
        assert!(normalized[0]["files"][0].get("mode").is_none());
    }

    #[test]
    fn publish_crate_and_npm_reports_reject_malformed_shape_fields() {
        let mut unsafe_skipped = test_publish_plan();
        unsafe_skipped
            .skipped_private
            .push("hidden\u{202e}package".to_string());
        let error = validate_publish_plan(&unsafe_skipped).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut duplicate_publish_dependency = test_publish_plan();
        duplicate_publish_dependency.packages[1]
            .internal_dependencies
            .push("vesty-params".to_string());
        let error = validate_publish_plan(&duplicate_publish_dependency).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate internal dependency `vesty-params`")
        );

        let mut too_many_publish_packages = test_publish_plan();
        while too_many_publish_packages.packages.len() <= PUBLISH_PLAN_MAX_PACKAGES {
            let index = too_many_publish_packages.packages.len() + 1;
            too_many_publish_packages.packages.push(PublishPlanPackage {
                order: index,
                level: index,
                name: format!("vesty-extra-{index}"),
                version: "0.1.0".to_string(),
                manifest_path: format!("/workspace/crates/vesty-extra-{index}/Cargo.toml"),
                internal_dependencies: Vec::new(),
            });
        }
        let error = validate_publish_plan(&too_many_publish_packages).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("publish plan has too many packages")
        );

        let mut unsafe_crate_reason = test_crate_package_report();
        unsafe_crate_reason.packages[0].reason = Some("hidden\u{202e}reason".to_string());
        let error = validate_crate_package_report(&unsafe_crate_reason).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut duplicate_crate_dependency = test_crate_package_report();
        duplicate_crate_dependency.packages[1]
            .internal_dependencies
            .push("vesty-params".to_string());
        let error = validate_crate_package_report(&duplicate_crate_dependency).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate internal dependency `vesty-params`")
        );

        let mut too_many_crate_packages = test_crate_package_report();
        while too_many_crate_packages.packages.len() <= CRATE_PACKAGE_MAX_PACKAGES {
            let index = too_many_crate_packages.packages.len() + 1;
            too_many_crate_packages.packages.push(CratePackageEntry {
                name: format!("vesty-extra-{index}"),
                version: "0.1.0".to_string(),
                manifest_path: format!("/workspace/crates/vesty-extra-{index}/Cargo.toml"),
                publish_order: index,
                internal_dependencies: Vec::new(),
                status: "packaged".to_string(),
                reason: None,
            });
        }
        let error = validate_crate_package_report(&too_many_crate_packages).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("crate package report has too many packages")
        );

        let sanitized = truncate_for_report("line one\nline two\u{202e}hidden", 128);
        assert_eq!(sanitized, "line one line two hidden");

        let mut control_npm_filename = test_npm_pack_report();
        control_npm_filename[0].filename = "vesty-plugin-ui\n0.1.0.tgz".to_string();
        let error = validate_npm_pack_entries(&control_npm_filename).unwrap_err();
        assert!(error.to_string().contains(
            "npm package `vesty-plugin-ui` filename must not contain control characters"
        ));

        let mut duplicate_npm_file = test_npm_pack_report();
        duplicate_npm_file[0].files.push(NpmPackFile {
            path: "package.json".to_string(),
        });
        let error = validate_npm_pack_entries(&duplicate_npm_file).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate packed path `package.json`")
        );

        let mut too_many_npm_files = test_npm_pack_report();
        while too_many_npm_files[0].files.len() <= NPM_PACK_MAX_FILES_PER_PACKAGE {
            let index = too_many_npm_files[0].files.len();
            too_many_npm_files[0].files.push(NpmPackFile {
                path: format!("dist/extra-{index}.js"),
            });
        }
        let error = validate_npm_pack_entries(&too_many_npm_files).unwrap_err();
        assert!(error.to_string().contains("has too many packed files"));
    }

    #[test]
    fn npm_pack_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let valid = root.join("npm-pack.json");
        write_npm_pack_artifact(&valid);

        run_npm_pack(&root, Some(&valid), true, "text").unwrap();
        run_npm_pack(&root, Some(&valid), true, "json").unwrap();

        let error = run_npm_pack(&root, None, true, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));

        let missing_workspace = root.join("missing-workspace");
        let error = run_npm_pack(
            &missing_workspace,
            Some(&root.join("out.json")),
            false,
            "text",
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("npm workspace directory does not exist")
        );
    }

    #[test]
    fn dependency_baseline_report_validates_current_workspace() {
        let report = dependency_baseline_report(&workspace_root()).unwrap();

        validate_dependency_baseline_report(&report).unwrap();
        assert_eq!(report.status, "ok");
        assert!(report.checks.iter().any(|check| {
            check.name == "cargo workspace external dependency baseline coverage"
                && check.status == "ok"
                && check
                    .actual
                    .as_deref()
                    .is_some_and(|actual| actual.contains("arc-swap") && actual.contains("tracing"))
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "cargo workspace dependency `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.55.1")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "Steinberg VST3 SDK baseline"
                && check.expected == "v3.8.0_build_66"
                && check.actual.as_deref() == Some("v3.8.0_build_66")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm lockfile `typescript` installed version"
                && check.expected == "7.0.2"
                && check.actual.as_deref() == Some("7.0.2")
        }));
    }

    #[test]
    fn dependency_baseline_coverage_rejects_unreviewed_external_workspace_dependency() {
        let manifest = toml::from_str::<toml::Value>(
            r#"
[workspace.dependencies]
arc-swap = "1.9.1"
vesty-core = { path = "crates/vesty-core", version = "0.1.0" }
unreviewed = "9.9.9"
"#,
        )
        .unwrap();

        let check = workspace_dependency_baseline_coverage_check(&manifest);

        assert_eq!(check.status, "failed");
        assert!(check.expected.contains("arc-swap"));
        assert!(!check.expected.contains("unreviewed"));
        assert!(check.actual.as_deref().unwrap().contains("unreviewed"));
        assert!(
            check
                .hint
                .as_deref()
                .unwrap()
                .contains("REQUIRED_RUST_BASELINE_DEPENDENCIES")
        );
    }

    #[derive(Default)]
    struct FakeLatestDependencyFetcher {
        crate_versions: BTreeMap<String, Result<String, String>>,
        npm_versions: BTreeMap<String, Result<String, String>>,
    }

    impl FakeLatestDependencyFetcher {
        fn current() -> Self {
            let mut fetcher = Self::default();
            for (name, version) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
                let version = rust_registry_latest_expected(name, version);
                fetcher
                    .crate_versions
                    .insert((*name).to_string(), Ok(version));
            }
            fetcher.npm_versions.insert(
                "typescript".to_string(),
                Ok(TYPESCRIPT_BASELINE_LOCK_VERSION.to_string()),
            );
            for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
                fetcher.npm_versions.insert(
                    dependency.dependency.to_string(),
                    Ok(dependency.expected_lock_version.to_string()),
                );
            }
            fetcher
        }
    }

    impl LatestDependencyFetcher for FakeLatestDependencyFetcher {
        fn latest_crate_version(&self, name: &str) -> Result<String, String> {
            self.crate_versions
                .get(name)
                .cloned()
                .unwrap_or_else(|| Err(format!("missing fake crate version for {name}")))
        }

        fn latest_npm_version(&self, name: &str) -> Result<String, String> {
            self.npm_versions
                .get(name)
                .cloned()
                .unwrap_or_else(|| Err(format!("missing fake npm version for {name}")))
        }
    }

    #[test]
    fn dependency_baseline_latest_report_validates_registry_versions() {
        let report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();

        validate_dependency_baseline_report(&report).unwrap();
        assert_eq!(report.status, "ok");
        assert!(report.checks.iter().any(|check| {
            check.name == "crates.io latest `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.55.1")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `typescript`"
                && check.expected == "7.0.2"
                && check.actual.as_deref() == Some("7.0.2")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `react`"
                && check.expected == "19.2.7"
                && check.actual.as_deref() == Some("19.2.7")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `@types/react`"
                && check.expected == "19.2.17"
                && check.actual.as_deref() == Some("19.2.17")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `vue`"
                && check.expected == "3.5.39"
                && check.actual.as_deref() == Some("3.5.39")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `svelte`"
                && check.expected == "5.56.5"
                && check.actual.as_deref() == Some("5.56.5")
        }));

        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline-latest.json"))
            .unwrap();
        write_dependency_baseline_report(&path, &report).unwrap();
        let evidence = validate_dependency_baseline_latest_report(&path).unwrap();
        assert_eq!(
            evidence.latest_checks,
            REQUIRED_RUST_BASELINE_DEPENDENCIES.len()
                + 1
                + REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES.len()
        );
        assert_eq!(
            dependency_baseline_latest_release_check(Some(&path), true).status,
            "ok"
        );
    }

    #[test]
    fn dependency_baseline_latest_report_fails_on_registry_drift_or_query_error() {
        let mut fetcher = FakeLatestDependencyFetcher::current();
        fetcher
            .crate_versions
            .insert("wry".to_string(), Ok("0.56.0".to_string()));
        fetcher.npm_versions.insert(
            "typescript".to_string(),
            Err("registry unavailable".to_string()),
        );

        let report = dependency_baseline_report_with_latest(&workspace_root(), &fetcher).unwrap();
        assert_eq!(report.status, "failed");
        assert!(validate_dependency_baseline_report(&report).is_err());
        assert!(report.checks.iter().any(|check| {
            check.name == "crates.io latest `wry`"
                && check.expected == "0.55.1"
                && check.actual.as_deref() == Some("0.56.0")
                && check.status == "failed"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm registry latest `typescript`"
                && check.actual.is_none()
                && check.status == "failed"
                && check
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("registry unavailable"))
        }));
    }

    #[test]
    fn dependency_baseline_report_rejects_inconsistent_statuses() {
        let mut forged_ok = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        let wry = forged_ok
            .checks
            .iter_mut()
            .find(|check| check.name == "crates.io latest `wry`")
            .unwrap();
        wry.actual = Some("0.56.0".to_string());
        wry.status = "ok".to_string();

        let error = validate_dependency_baseline_report(&forged_ok).unwrap_err();
        assert!(error.to_string().contains("status ok is inconsistent"));

        let mut forged_failed = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        forged_failed.checks[0].status = "failed".to_string();

        let error = validate_dependency_baseline_report(&forged_failed).unwrap_err();
        assert!(error.to_string().contains("status failed is inconsistent"));

        let mut unknown_status = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        unknown_status.checks[0].status = "partial".to_string();

        let error = validate_dependency_baseline_report(&unknown_status).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported dependency baseline check status")
        );

        let mut inconsistent_top_level = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        inconsistent_top_level.status = "failed".to_string();

        let error = validate_dependency_baseline_report(&inconsistent_top_level).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("report status `failed` is inconsistent")
        );
    }

    #[test]
    fn dependency_baseline_report_rejects_malformed_shape_fields() {
        let latest_report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        let baseline_report = dependency_baseline_report(&workspace_root()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&latest_report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error =
            serde_json::from_value::<DependencyBaselineReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&latest_report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error =
            serde_json::from_value::<DependencyBaselineReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = latest_report.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        let error = validate_dependency_baseline_report(&duplicate).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate dependency baseline check")
        );

        let mut unknown = latest_report.clone();
        unknown.checks.push(DependencyBaselineCheck {
            name: "manual extra dependency check".to_string(),
            kind: "manual".to_string(),
            path: "Cargo.toml".to_string(),
            expected: "ok".to_string(),
            actual: Some("ok".to_string()),
            status: "ok".to_string(),
            hint: None,
        });
        let error = validate_dependency_baseline_report(&unknown).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown dependency baseline check(s)")
        );
        assert!(
            error
                .to_string()
                .contains("manual:manual extra dependency check")
        );

        let mut missing = baseline_report.clone();
        missing
            .checks
            .retain(|check| check.name != "cargo workspace dependency `wry`");
        let error = validate_dependency_baseline_report(&missing).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("dependency baseline report missing required check(s)")
        );
        assert!(
            error
                .to_string()
                .contains("cargo:cargo workspace dependency `wry`")
        );

        let mut control_path = latest_report.clone();
        control_path.checks[0].path = "Cargo.toml\nforged".to_string();
        let error = validate_dependency_baseline_report(&control_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("path must not contain control characters")
        );

        let mut multiline_hint = latest_report.clone();
        multiline_hint.checks[0].hint = Some("line one\nline two\tok".to_string());
        validate_dependency_baseline_report(&multiline_hint).unwrap();

        let mut hint_nul = multiline_hint.clone();
        hint_nul.checks[0].hint = Some("line one\0line two".to_string());
        let error = validate_dependency_baseline_report(&hint_nul).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("hint must not contain control characters")
        );

        let mut hint_unsafe = multiline_hint.clone();
        hint_unsafe.checks[0].hint = Some("reviewed\u{202e}hidden".to_string());
        let error = validate_dependency_baseline_report(&hint_unsafe).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut hint_too_large = multiline_hint.clone();
        hint_too_large.checks[0].hint = Some("x".repeat(DEPENDENCY_BASELINE_HINT_MAX_BYTES + 1));
        let error = validate_dependency_baseline_report(&hint_too_large).unwrap_err();
        assert!(error.to_string().contains("hint must be at most"));

        let mut too_many = latest_report;
        while too_many.checks.len() <= DEPENDENCY_BASELINE_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(DependencyBaselineCheck {
                name: format!("extra dependency baseline check {index}"),
                kind: "extra-baseline".to_string(),
                path: format!("extra/{index}.toml"),
                expected: "0.1.0".to_string(),
                actual: Some("0.1.0".to_string()),
                status: "ok".to_string(),
                hint: None,
            });
        }
        let error = validate_dependency_baseline_report(&too_many).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("dependency baseline report has too many checks")
        );
    }

    #[test]
    fn dependency_latest_release_check_rejects_offline_baseline_report() {
        let temp = tempfile::tempdir().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline.json")).unwrap();
        let report = dependency_baseline_report(&workspace_root()).unwrap();
        write_dependency_baseline_report(&path, &report).unwrap();

        let check = dependency_baseline_latest_release_check(Some(&path), true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing latest registry checks"));
    }

    #[test]
    fn dependency_latest_release_check_requires_workspace_baseline_coverage() {
        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("dependency-baseline-latest.json"))
            .unwrap();
        let mut report = dependency_baseline_report_with_latest(
            &workspace_root(),
            &FakeLatestDependencyFetcher::current(),
        )
        .unwrap();
        report
            .checks
            .retain(|check| check.name != DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME);
        fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = dependency_baseline_latest_release_check(Some(&path), true);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing required check(s)"));
        assert!(
            check
                .value
                .contains(DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME)
        );
    }

    #[test]
    fn parses_exact_cargo_search_latest_version_line() {
        let output = r#"
wry = "0.55.1"    # Cross-platform WebView rendering library
wry-webkit = "0.1.0"
"#;
        assert_eq!(
            parse_cargo_search_latest_version("wry", output).as_deref(),
            Some("0.55.1")
        );
        assert!(parse_cargo_search_latest_version("missing", output).is_none());
    }

    #[test]
    fn parses_crates_io_latest_version_response() {
        let stable = r#"{"crate":{"max_stable_version":"0.55.1","max_version":"0.56.0-beta.1"}}"#;
        assert_eq!(
            parse_crates_io_latest_version(stable).as_deref(),
            Some("0.55.1")
        );

        let prerelease_only = r#"{"crate":{"max_stable_version":null,"max_version":"1.0.0-rc.1"}}"#;
        assert_eq!(
            parse_crates_io_latest_version(prerelease_only).as_deref(),
            Some("1.0.0-rc.1")
        );
        assert!(parse_crates_io_latest_version(r#"{"crate":{}}"#).is_none());
        assert!(parse_crates_io_latest_version("not json").is_none());
    }

    #[test]
    fn parses_cargo_info_version_line_for_registry_fallback() {
        let output = r#"
ts-rs #typescript #ts #bindings #ts-rs #wasm
generate typescript bindings from rust types
version: 12.0.1
license: MIT
"#;
        assert_eq!(parse_cargo_info_version(output).as_deref(), Some("12.0.1"));
        assert!(parse_cargo_info_version("license: MIT").is_none());
    }

    #[test]
    fn dependency_baseline_check_mode_validates_existing_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let report_path = root.join("dependency-baseline.json");
        let workspace = workspace_root();

        run_dependency_baseline(&workspace, Some(&report_path), false, false, "text").unwrap();
        run_dependency_baseline(&workspace, Some(&report_path), true, false, "json").unwrap();

        let error = run_dependency_baseline(&workspace, None, true, false, "text").unwrap_err();
        assert!(error.to_string().contains("--out <report>"));

        let mut report = read_dependency_baseline_report(&report_path).unwrap();
        report.checks.push(DependencyBaselineCheck {
            name: "extra stale check".to_string(),
            kind: "test".to_string(),
            path: "Cargo.toml".to_string(),
            expected: "ok".to_string(),
            actual: Some("ok".to_string()),
            status: "ok".to_string(),
            hint: None,
        });
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        let error = run_dependency_baseline(&workspace, Some(&report_path), true, false, "text")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown dependency baseline check")
        );
    }

    #[test]
    fn ci_release_check_artifacts_validate_local_invariants_across_os() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_ignore_action_plan_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let plan = ReleaseActionPlan {
            version: 1,
            status: "failed".to_string(),
            summary: ReleaseActionPlanSummary {
                ok: 1,
                failed: 1,
                skipped: 0,
                action_count: 1,
            },
            protocol_snapshot: "target/vesty-protocol".to_string(),
            evidence_root: None,
            release_evidence_dir: None,
            actions: vec![ReleaseActionItem {
                check: "daw smoke matrix".to_string(),
                status: "failed".to_string(),
                priority: "required".to_string(),
                value: "missing".to_string(),
                hint: None,
                evidence_path: None,
                commands: vec!["vesty daw-matrix --strict".to_string()],
            }],
        };
        fs::write(
            root.join("release-action-plan-Linux.json"),
            serde_json::to_string_pretty(&plan).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
    }

    #[test]
    fn ci_release_check_artifacts_accept_case_insensitive_report_filenames() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Release-Check-Linux.JSON"));
        write_ci_release_check_artifact(&root.join("RELEASE-CHECK-macOS.Json"));
        write_ci_release_check_artifact(&root.join("release-check-WINDOWS.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_infer_os_from_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Linux/release-check.json"));
        write_ci_release_check_artifact(&root.join("macOS/release-check.json"));
        write_ci_release_check_artifact(&root.join("Windows/release-check.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_infer_os_from_path_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("Linux/release-check.json"));
        write_ci_release_check_artifact(&root.join("macOS/release-check.json"));
        write_ci_release_check_artifact(&root.join("swing-state/release-check.json"));

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: Windows"));
        assert!(
            check
                .value
                .contains("swing-state/release-check.json: could not infer OS from artifact path")
        );
        assert!(!check.value.contains("duplicate OS reports: Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_os_label_mismatch_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let linux = root.join("release-check-Linux.json");
        let mut report: ReleaseCheckReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.os = Some("Windows".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("path indicates Linux"));
        assert!(check.value.contains("report os is Windows"));
    }

    #[test]
    fn ci_release_check_artifacts_allow_legacy_reports_without_os_label() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        for os in ["Linux", "macOS", "Windows"] {
            let path = root.join(format!("release-check-{os}.json"));
            write_ci_release_check_artifact(&path);
            let mut report: ReleaseCheckReport =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            report.os = None;
            fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        }

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_release_check_artifacts_preserve_crate_package_readiness_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == "crate package readiness")
            .expect("crate package readiness check");
        check.status = "failed".to_string();
        check.value = "crate package readiness failed: vesty-core cargo package failed".to_string();
        check.hint = Some("inspect vesty crate-package output".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_preserve_platform_smoke_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        let check = report
            .checks
            .iter_mut()
            .find(|check| check.name == "platform smoke artifacts")
            .expect("platform smoke artifacts check");
        check.status = "failed".to_string();
        check.value = "required evidence missing".to_string();
        check.hint = Some("collect macOS, Windows x64 and Linux X11 smoke reports".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_preserve_vst3_sdk_audit_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        for name in [
            "vst3 SDK header manifest",
            "vst3 SDK generated bindings plan",
            "vst3 SDK generated bindings surface",
            "vst3 SDK generated bindings scaffold",
            "vst3 SDK generated bindings ABI seed",
            "vst3 SDK generated bindings ABI layout",
            "vst3 SDK generated bindings interface skeleton",
        ] {
            let check = report
                .checks
                .iter_mut()
                .find(|check| check.name == name)
                .expect("VST3 SDK audit check");
            check.status = "failed".to_string();
            check.value = format!("{name}: optional audit artifact is invalid");
            check.hint = Some("regenerate optional VST3 SDK audit artifact".to_string());
        }
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 release-check report"));
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_missing_os_and_local_failures() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));
        let mut report = test_ci_release_check_report();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "vst3 binding baseline")
            .unwrap()
            .status = "failed".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("local invariant check"));
        assert!(check.value.contains("vst3 binding baseline"));

        fs::remove_file(root.join("release-check-macOS.json")).unwrap();
        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: macOS"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_inconsistent_or_unknown_statuses() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let mut ok_with_failure = test_ci_release_check_report();
        ok_with_failure.status = "ok".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&ok_with_failure).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("report status `ok` is inconsistent with failed checks")
        );

        let mut unknown_check_status = test_ci_release_check_report();
        unknown_check_status.checks[0].status = "pending".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&unknown_check_status).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unexpected check status"));
        assert!(check.value.contains("host profiles=pending"));

        let mut control_value = test_ci_release_check_report();
        control_value.checks[0].value = "5 release host profile(s) covered\nbad".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&control_value).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("value must not contain control characters")
        );

        let mut long_hint = test_ci_release_check_report();
        long_hint.checks[0].hint = Some("x".repeat(RELEASE_ACTION_TEXT_MAX_BYTES + 1));
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&long_hint).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("hint must be at most"));

        let mut invalid_run_url = test_ci_release_check_report();
        invalid_run_url.ci_run_url =
            Some("https://github.com/vesty-rs/vesty/actions/runs/not-a-number".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&invalid_run_url).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("invalid ci_run_url"));

        let mut control_run_url = test_ci_release_check_report();
        control_run_url.ci_run_url =
            Some("https://github.com/vesty-rs/vesty/actions/runs/123\nbad".to_string());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&control_run_url).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("ci run url must not contain control characters")
        );

        let mut empty_checks = test_ci_release_check_report();
        empty_checks.status = "ok".to_string();
        empty_checks.checks.clear();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&empty_checks).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("must contain at least one check"));

        let mut too_many_checks = test_ci_release_check_report();
        while too_many_checks.checks.len() <= RELEASE_CHECK_MAX_CHECKS {
            let index = too_many_checks.checks.len();
            too_many_checks.checks.push(ReleaseCheckItem {
                name: format!("extra skipped check {index}"),
                status: "skipped".to_string(),
                value: "not requested".to_string(),
                hint: None,
            });
        }
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&too_many_checks).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("too many checks"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_duplicate_or_forged_invariant_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-Windows.json"));

        let mut duplicate = test_ci_release_check_report();
        duplicate.checks.push(duplicate.checks[0].clone());
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&duplicate).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate check name"));
        assert!(check.value.contains("host profiles"));

        let mut forged_host_profiles = test_ci_release_check_report();
        forged_host_profiles
            .checks
            .iter_mut()
            .find(|check| check.name == "host profiles")
            .unwrap()
            .value = "1 release host profile covered".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_host_profiles).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("host profiles"));
        assert!(check.value.contains("inconsistent with daw_matrix"));

        let mut forged_binding_baseline = test_ci_release_check_report();
        forged_binding_baseline
            .checks
            .iter_mut()
            .find(|check| check.name == "vst3 binding baseline")
            .unwrap()
            .value = "Steinberg SDK v3.8.0_build_66".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_binding_baseline).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("vst3 binding baseline"));
        assert!(check.value.contains("does not match current baseline"));

        let mut forged_protocol = test_ci_release_check_report();
        forged_protocol
            .checks
            .iter_mut()
            .find(|check| check.name == "protocol snapshot")
            .unwrap()
            .value = "skipped in CI".to_string();
        fs::write(
            root.join("release-check-macOS.json"),
            serde_json::to_string_pretty(&forged_protocol).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("protocol snapshot"));
        assert!(check.value.contains("unexpected value"));
    }

    #[test]
    fn ci_release_check_artifacts_reject_mismatched_ci_run_url_when_expected() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&root).unwrap();
        write_ci_release_check_artifact(&root.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&root.join("release-check-macOS.json"));
        let mut report = test_ci_release_check_report();
        report.ci_run_url = Some("https://github.com/vesty-rs/other/actions/runs/999".to_string());
        fs::write(
            root.join("release-check-Windows.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();

        let check = ci_release_check_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890"),
        );

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("expected vesty-rs/vesty run 1234567890")
        );
        assert!(check.value.contains("got vesty-rs/other run 999"));
    }

    #[test]
    fn platform_smoke_release_check_requires_real_platform_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        assert_eq!(write_platform_smoke_templates(&root).unwrap(), 4);
        assert_eq!(write_platform_smoke_templates(&root).unwrap(), 0);
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("Platform Smoke Evidence"));
        assert!(readme.contains("Linux Wayland is experimental"));

        let pending_local = platform_smoke_release_check(Some(&root), false);
        assert_eq!(pending_local.status, "skipped");
        assert_eq!(pending_local.value, "not requested");

        let pending_required = platform_smoke_release_check(Some(&root), true);
        assert_eq!(pending_required.status, "failed");
        assert_eq!(pending_required.value, "required evidence missing");
        assert!(
            pending_required
                .hint
                .as_deref()
                .unwrap()
                .contains("no passing platform smoke reports found")
        );

        fs::remove_file(root.join("macos.json")).unwrap();
        fs::remove_file(root.join("windows-x64.json")).unwrap();
        fs::remove_file(root.join("linux-x11.json")).unwrap();
        write_platform_smoke_artifact(&root.join("macos.json"), "macos");

        let local = platform_smoke_release_check(Some(&root), false);
        assert_eq!(local.status, "ok");
        assert!(local.value.contains("macOS"));
        assert!(local.hint.as_deref().unwrap().contains("Windows x64"));

        let required = platform_smoke_release_check(Some(&root), true);
        assert_eq!(required.status, "failed");
        assert!(required.value.contains("Windows x64"));
        assert!(required.value.contains("Linux X11"));

        write_platform_smoke_artifact(&root.join("windows-x64.json"), "windows-x64");
        write_platform_smoke_artifact(&root.join("linux-x11.json"), "linux-x11");
        let complete = platform_smoke_release_check(Some(&root), true);
        assert_eq!(complete.status, "ok");
        assert!(complete.value.contains("3 platform smoke report"));
        assert!(complete.value.contains("Linux X11"));
    }

    #[test]
    fn platform_smoke_release_check_accepts_platform_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("macOS/platform-smoke.json"), "macos");
        write_platform_smoke_artifact(&root.join("Windows/platform-smoke.json"), "windows-x64");
        write_platform_smoke_artifact(&root.join("Linux-X11/platform-smoke.json"), "linux-x11");

        let check = platform_smoke_release_check(Some(&root), true);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 platform smoke report"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows x64"));
        assert!(check.value.contains("Linux X11"));
    }

    #[test]
    fn platform_smoke_release_check_rejects_path_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("Windows/platform-smoke.json"), "macos");

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("artifact path indicates windows-x64"));
        assert!(check.value.contains("report platform is macos"));
    }

    #[test]
    fn platform_smoke_release_check_rejects_ambiguous_path_platform_tokens() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("macos-windows/platform-smoke.json"), "macos");
        write_platform_smoke_artifact(
            &root.join("linux-x11-windows/platform-smoke.json"),
            "linux-x11",
        );

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("artifact path contains multiple platform tokens: macos, windows-x64")
        );
        assert!(
            check.value.contains(
                "artifact path contains multiple platform tokens: windows-x64, linux-x11"
            ),
            "{}",
            check.value
        );
    }

    #[test]
    fn platform_smoke_path_platform_inference_requires_linux_x11_token() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        write_platform_smoke_artifact(&root.join("linux-wayland/platform-smoke.json"), "macos");

        let check = platform_smoke_release_check(Some(&root), false);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
        assert!(!check.value.contains("artifact path indicates linux-x11"));
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_release_check_rejects_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&external).unwrap();
        write_platform_smoke_artifact(&external.join("macos.json"), "macos");
        unix_fs::symlink(&external, &root).unwrap();

        let check = platform_smoke_release_check(Some(&root), true);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke path must not be a symlink")
        );
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_templates_reject_symlink_root_dir() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &root).unwrap();

        let error = write_platform_smoke_templates(&root)
            .expect_err("platform smoke templates should reject symlink roots")
            .to_string();

        assert!(error.contains("template output directory must not be a symlink"));
        assert!(!external.join("README.md").exists());
    }

    #[test]
    fn platform_smoke_write_report_validates_and_writes_normalized_report() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let path =
            write_platform_smoke_report(&root, test_platform_smoke_report_input("darwin")).unwrap();

        assert_eq!(path.file_name(), Some("macos.json"));
        let report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(report.platform, "macos");
        assert_eq!(report.os.as_deref(), Some("macOS"));
        assert_eq!(report.host.as_deref(), Some("Vesty smoke host"));
        assert_eq!(report.checks.len(), REQUIRED_PLATFORM_SMOKE_CHECKS.len());

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_write_report_rejects_symlink_output_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-macos.json")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(&external, "do not overwrite\n").unwrap();
        unix_fs::symlink(&external, root.join("macos.json")).unwrap();

        let error = write_platform_smoke_report(&root, test_platform_smoke_report_input("macos"))
            .expect_err("platform smoke writer should reject symlink output")
            .to_string();

        assert!(error.contains("output file must not be a symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }

    #[cfg(unix)]
    #[test]
    fn platform_smoke_write_report_rejects_symlink_output_parent() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-platform-smoke")).unwrap();
        let parent_link = root.join("platform-parent");
        fs::create_dir(&external).unwrap();
        unix_fs::symlink(&external, &parent_link).unwrap();

        let error = write_platform_smoke_report(
            &parent_link.join("platform-smoke"),
            test_platform_smoke_report_input("macos"),
        )
        .expect_err("platform smoke writer should reject symlinked output parents")
        .to_string();

        assert!(error.contains("platform smoke report dir parent must not be a symlink"));
        assert!(!external.join("platform-smoke/macos.json").exists());
    }

    #[test]
    fn platform_smoke_write_report_rejects_invalid_or_zero_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let wayland =
            write_platform_smoke_report(&root, test_platform_smoke_report_input("linux-wayland"))
                .unwrap_err()
                .to_string();
        assert!(wayland.contains("unsupported platform"));

        let mut zero_meter = test_platform_smoke_report_input("linux-x11");
        zero_meter.meter_stream = Some("meter_flush sent=0".to_string());
        let error = write_platform_smoke_report(&root, zero_meter)
            .unwrap_err()
            .to_string();
        assert!(error.contains("meter stream"));
        assert!(!root.join("linux-x11.json").exists());
    }

    #[test]
    fn platform_smoke_requires_platform_specific_webview_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        for (platform, evidence) in [
            ("macos", "system_webview=true"),
            ("macos", "not WKWebView; WebKit.framework shim unavailable"),
            ("windows-x64", "WebKit.framework loaded"),
            ("windows-x64", "not WebView2; WebView2 disabled"),
            ("linux-x11", "WebKitGTK loaded"),
            (
                "linux-x11",
                "WebKitGTK loaded; Wayland compositor; X11 fallback not active",
            ),
            ("linux-x11", "WebKitGTK loaded; not X11"),
        ] {
            let mut input = test_platform_smoke_report_input(platform);
            input.system_webview = Some(evidence.to_string());
            let error = write_platform_smoke_report(&root, input)
                .unwrap_err()
                .to_string();
            assert!(error.contains("system WebView"), "{platform}: {error}");
        }
    }

    #[test]
    fn platform_smoke_requires_validator_identity_and_zero_fail_summary() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        for evidence in [
            "vst3_validator=true",
            "Steinberg validator passed",
            "custom checker passed 47 tests, 0 failed",
            "Steinberg validator passed 47 tests, 1 failed",
            "Steinberg validator passed 0 tests, 0 failed",
            "Steinberg validator passed 47 tests, 0 failed; validator timeout",
            "VST3 validator: passed=47 failed=0; validator crashed",
            "Steinberg validator passed 47 tests, 0 failed; validator error: log truncated",
        ] {
            let mut input = test_platform_smoke_report_input("macos");
            input.vst3_validator = Some(evidence.to_string());
            let error = write_platform_smoke_report(&root, input)
                .unwrap_err()
                .to_string();
            assert!(error.contains("VST3 validator"), "{evidence}: {error}");
            assert!(
                error.contains("accepted smoke evidence markers"),
                "{evidence}: {error}"
            );
        }
    }

    #[test]
    fn platform_smoke_accepts_alternate_system_webview_and_validator_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();

        let mut macos = test_platform_smoke_report_input("macos");
        macos.system_webview = Some("WKWebView created inside editor parent".to_string());
        macos.vst3_validator = Some("VST3 validator: passed=47 failed=0".to_string());
        let path = write_platform_smoke_report(&root, macos).unwrap();
        assert_eq!(path.file_name(), Some("macos.json"));

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "ok");
        assert!(check.value.contains("macOS"));
    }

    #[test]
    fn platform_smoke_rejects_placeholder_values_even_when_status_is_ok() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let mut report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "system_webview")
            .unwrap()
            .value = "replace with real system WebView evidence".to_string();
        fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("system WebView"));
        assert!(check.value.contains("missing positive evidence value"));
    }

    #[test]
    fn platform_smoke_rejects_contradictory_positive_values() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        for (name, value, expected_label) in [
            (
                "webview_attach",
                "webview_attach=true; attach failed",
                "WebView attach",
            ),
            (
                "meter_stream",
                "meter_flush sent=1; meter stream error",
                "meter stream",
            ),
            (
                "jsbridge_roundtrip",
                "jsbridge_roundtrip=true; roundtrip=false",
                "JSBridge roundtrip",
            ),
        ] {
            let mut candidate = report.clone();
            candidate
                .checks
                .iter_mut()
                .find(|check| check.name == name)
                .unwrap()
                .value = value.to_string();
            fs::write(&path, serde_json::to_string_pretty(&candidate).unwrap()).unwrap();

            let check = platform_smoke_release_check(Some(&root), false);
            assert_eq!(check.status, "failed", "{name}");
            assert!(
                check.value.contains(expected_label),
                "{name}: {}",
                check.value
            );
            assert!(
                check.value.contains("missing positive evidence value"),
                "{name}: {}",
                check.value
            );
        }
    }

    #[test]
    fn platform_smoke_rejects_os_metadata_platform_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let macos = root.join("macos.json");
        write_platform_smoke_artifact(&macos, "macos");
        let mut macos_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&macos).unwrap()).unwrap();
        macos_report.os = Some("Windows 11 x64".to_string());
        fs::write(&macos, serde_json::to_string_pretty(&macos_report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke os `Windows 11 x64` does not match platform `macos`")
        );

        fs::remove_file(&macos).unwrap();
        let linux = root.join("linux-x11.json");
        write_platform_smoke_artifact(&linux, "linux-x11");
        let mut linux_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        linux_report.os = Some("Linux Wayland session".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&linux_report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains(
            "platform smoke os `Linux Wayland session` does not match platform `linux-x11`"
        ));
    }

    #[test]
    fn platform_smoke_rejects_malformed_report_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let path = root.join("macos.json");
        write_platform_smoke_artifact(&path, "macos");
        let valid_report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&valid_report).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<PlatformSmokeReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&valid_report).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<PlatformSmokeReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut duplicate = valid_report.clone();
        duplicate.checks.push(PlatformSmokeCheck {
            name: "system-webview".to_string(),
            status: "ok".to_string(),
            value: "WKWebView created".to_string(),
            hint: None,
        });
        fs::write(&path, serde_json::to_string_pretty(&duplicate).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate platform smoke check"));
        assert!(check.value.contains("system_webview"));

        let mut unknown = valid_report.clone();
        unknown.checks.push(PlatformSmokeCheck {
            name: "extra-check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        fs::write(&path, serde_json::to_string_pretty(&unknown).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unknown platform smoke check(s)"));
        assert!(check.value.contains("extra_check"));

        let mut missing = valid_report.clone();
        missing
            .checks
            .retain(|check| check.name != "jsbridge_roundtrip");
        fs::write(&path, serde_json::to_string_pretty(&missing).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke report missing required check(s)")
        );
        assert!(check.value.contains("jsbridge_roundtrip"));

        let mut control_host = valid_report.clone();
        control_host.host = Some("Vesty smoke host\nforged".to_string());
        fs::write(&path, serde_json::to_string_pretty(&control_host).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke host must not contain control characters")
        );

        let mut unsafe_hint = valid_report.clone();
        unsafe_hint.host = Some("Vesty smoke host".to_string());
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        fs::write(&path, serde_json::to_string_pretty(&unsafe_hint).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut too_many = valid_report;
        too_many.checks[0].hint = None;
        while too_many.checks.len() <= PLATFORM_SMOKE_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(PlatformSmokeCheck {
                name: format!("extra_check_{index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        fs::write(&path, serde_json::to_string_pretty(&too_many).unwrap()).unwrap();
        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("platform smoke report has too many checks")
        );
    }

    #[test]
    fn platform_smoke_rejects_wayland_and_zero_meter_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("platform-smoke")).unwrap();
        fs::create_dir(&root).unwrap();
        let wayland = root.join("linux-wayland.json");
        write_platform_smoke_artifact(&wayland, "linux-wayland");

        let check = platform_smoke_release_check(Some(&root), true);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unsupported platform"));

        fs::remove_file(&wayland).unwrap();
        let linux = root.join("linux-x11.json");
        write_platform_smoke_artifact(&linux, "linux-x11");
        let mut report: PlatformSmokeReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report
            .checks
            .iter_mut()
            .find(|check| check.name == "meter_stream")
            .unwrap()
            .value = "meter_flush sent=0".to_string();
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = platform_smoke_release_check(Some(&root), false);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("meter stream"));
    }

    #[test]
    fn ci_run_url_requires_exact_github_actions_run_shape() {
        for url in [
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890/",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1",
            "https://github.com/vesty-rs/vesty/actions/runs/1234567890?check_suite_focus=true",
        ] {
            let check = ci_run_url_release_check(Some(url), true);
            assert_eq!(check.status, "ok", "{url}");
        }

        for url in [
            "https://github.com/vesty-rs/vesty/actions",
            "https://github.com/vesty-rs/vesty/actions/runs/",
            "https://github.com/vesty-rs/vesty/actions/runs/latest",
            "https://github.com/vesty-rs/vesty/actions/runs/123/jobs/456",
            "https://github.com/vesty-rs/vesty/actions/runs/123 456",
            "http://github.com/vesty-rs/vesty/actions/runs/123",
        ] {
            let check = ci_run_url_release_check(Some(url), true);
            assert_eq!(check.status, "failed", "{url}");
        }
    }

    #[test]
    fn ci_doctor_artifacts_reject_failed_required_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_check_status(&root.join("doctor-Linux.json"), "node", "missing");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("Linux/node status missing"));
    }

    #[test]
    fn ci_doctor_artifacts_require_sdk_headers_check() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        remove_doctor_check(&root.join("doctor-macOS.json"), "vst3 SDK headers");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("macOS/vst3 SDK headers missing"));
    }

    #[test]
    fn ci_doctor_artifacts_accept_linux_signing_policy_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_check_status(
            &root.join("doctor-Linux.json"),
            "signing: linux release policy",
            "unknown",
        );

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_infer_os_from_parent_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("Linux/doctor.json"), "Linux");
        write_doctor_artifact(&root.join("macOS/doctor.json"), "macOS");
        write_doctor_artifact(&root.join("Windows/doctor.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
        assert!(check.value.contains("3 doctor report"));
        assert!(check.value.contains("Linux"));
        assert!(check.value.contains("macOS"));
        assert!(check.value.contains("Windows"));
    }

    #[test]
    fn ci_doctor_artifacts_infer_os_from_path_tokens_not_substrings() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("Linux/doctor.json"), "Linux");
        write_doctor_artifact(&root.join("macOS/doctor.json"), "macOS");
        write_doctor_artifact(&root.join("swing-state/doctor.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("missing OS reports: Windows"));
        assert!(!check.value.contains("OS mismatch"));
        assert!(!check.value.contains("duplicate OS reports: Windows"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_mismatched_ci_run_url_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_ci_run_url(
            &root.join("doctor-Linux.json"),
            "https://github.com/vesty/vesty/actions/runs/99",
        );
        set_doctor_ci_run_url(
            &root.join("doctor-macOS.json"),
            "https://github.com/vesty/vesty/actions/runs/42/attempts/2",
        );
        set_doctor_ci_run_url(
            &root.join("doctor-Windows.json"),
            "https://github.com/vesty/vesty/actions/runs/42",
        );

        let check = ci_doctor_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty/vesty/actions/runs/42/attempts/1"),
        );

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("run URL mismatch"));
        assert!(check.value.contains("Linux expected vesty/vesty run 42"));
        assert!(!check.value.contains("macOS expected"));
        assert!(!check.value.contains("Windows expected"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_duplicate_os_reports() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir_all(root.join("linux-copy")).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("linux-copy/doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate OS reports: Linux"));
    }

    #[test]
    fn ci_doctor_artifacts_reject_os_label_mismatch_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        set_doctor_report_os(&root.join("doctor-Linux.json"), Some("macOS"));

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(check.value.contains("OS mismatch"));
        assert!(check.value.contains("doctor-Linux.json"));
        assert!(check.value.contains("path indicates Linux"));
        assert!(check.value.contains("report os is macOS"));
    }

    #[test]
    fn ci_doctor_artifacts_allow_legacy_reports_without_os_label() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        for file in [
            "doctor-Linux.json",
            "doctor-macOS.json",
            "doctor-Windows.json",
        ] {
            set_doctor_report_os(&root.join(file), None);
        }

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_allow_legacy_reports_without_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        write_doctor_artifact(&root.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let check = ci_doctor_artifacts_release_check(
            Some(&root),
            true,
            Some("https://github.com/vesty/vesty/actions/runs/42"),
        );

        assert_eq!(check.status, "ok");
    }

    #[test]
    fn ci_doctor_artifacts_reject_cross_os_checks() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.checks.push(DoctorCheck {
            name: "signing: codesign".to_string(),
            status: "ok".to_string(),
            value: "codesign from the wrong platform".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("Linux/signing: codesign unexpected for Linux doctor report")
        );
    }

    #[test]
    fn ci_doctor_artifacts_reject_legacy_cross_os_checks_from_path_os() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");

        let mut report: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();
        report.os = None;
        report.checks.push(DoctorCheck {
            name: "signing: signtool".to_string(),
            status: "ok".to_string(),
            value: "signtool from the wrong platform".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&report).unwrap()).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("Linux/signing: signtool unexpected for Linux doctor report")
        );
    }

    #[test]
    fn ci_doctor_artifacts_reject_malformed_report_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&root).unwrap();
        let linux = root.join("doctor-Linux.json");
        write_doctor_artifact(&linux, "Linux");
        write_doctor_artifact(&root.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&root.join("doctor-Windows.json"), "Windows");
        let valid_linux: DoctorReport =
            serde_json::from_str(&fs::read_to_string(&linux).unwrap()).unwrap();

        let mut unknown_top_level = serde_json::to_value(&valid_linux).unwrap();
        unknown_top_level["generatedBy"] = serde_json::json!("manual");
        let error = serde_json::from_value::<DoctorReport>(unknown_top_level).unwrap_err();
        assert!(error.to_string().contains("unknown field `generatedBy`"));

        let mut unknown_check_field = serde_json::to_value(&valid_linux).unwrap();
        unknown_check_field["checks"][0]["owner"] = serde_json::json!("release");
        let error = serde_json::from_value::<DoctorReport>(unknown_check_field).unwrap_err();
        assert!(error.to_string().contains("unknown field `owner`"));

        let mut invalid_url = valid_linux.clone();
        invalid_url.ci_run_url = Some("https://github.com/vesty/vesty/actions/runs/latest".into());
        fs::write(&linux, serde_json::to_string_pretty(&invalid_url).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("invalid doctor report ci_run_url"));

        let mut unknown = valid_linux.clone();
        unknown.checks.push(DoctorCheck {
            name: "manual extra doctor check".to_string(),
            status: "ok".to_string(),
            value: "extra=true".to_string(),
            hint: None,
        });
        fs::write(&linux, serde_json::to_string_pretty(&unknown).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unknown doctor check"));
        assert!(check.value.contains("manual extra doctor check"));

        let mut duplicate = valid_linux.clone();
        duplicate.checks.push(duplicate.checks[0].clone());
        fs::write(&linux, serde_json::to_string_pretty(&duplicate).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("duplicate doctor check"));

        let mut control_os = valid_linux.clone();
        control_os.os = Some("Linux\nforged".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&control_os).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("doctor report os must not contain control characters")
        );

        let mut unsafe_hint = valid_linux.clone();
        unsafe_hint.os = Some("Linux".to_string());
        unsafe_hint.checks[0].hint = Some("verified\u{202e}hidden".to_string());
        fs::write(&linux, serde_json::to_string_pretty(&unsafe_hint).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("must not contain unsafe Unicode format characters")
        );

        let mut unexpected_status = valid_linux.clone();
        unexpected_status.checks[0].hint = None;
        unexpected_status.checks[0].status = "passed".to_string();
        fs::write(
            &linux,
            serde_json::to_string_pretty(&unexpected_status).unwrap(),
        )
        .unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("unexpected status `passed`"));

        let mut too_many = valid_linux;
        while too_many.checks.len() <= DOCTOR_MAX_CHECKS {
            let index = too_many.checks.len();
            too_many.checks.push(DoctorCheck {
                name: format!("extra doctor check {index}"),
                status: "ok".to_string(),
                value: "extra=true".to_string(),
                hint: None,
            });
        }
        fs::write(&linux, serde_json::to_string_pretty(&too_many).unwrap()).unwrap();
        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);
        assert_eq!(check.status, "failed");
        assert!(check.value.contains("doctor report has too many checks"));
    }

    #[cfg(unix)]
    #[test]
    fn ci_doctor_artifacts_reject_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external = Utf8PathBuf::from_path_buf(temp.path().join("external-ci-doctor")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-doctor")).unwrap();
        fs::create_dir(&external).unwrap();
        write_doctor_artifact(&external.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&external.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&external.join("doctor-Windows.json"), "Windows");
        unix_fs::symlink(&external, &root).unwrap();

        let check = ci_doctor_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("doctor artifact path must not be a symlink")
        );
    }

    #[cfg(unix)]
    #[test]
    fn ci_release_check_artifacts_reject_symlink_root() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-ci-release-checks")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ci-release-checks")).unwrap();
        fs::create_dir(&external).unwrap();
        write_ci_release_check_artifact(&external.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&external.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&external.join("release-check-Windows.json"));
        unix_fs::symlink(&external, &root).unwrap();

        let check = ci_release_check_artifacts_release_check(Some(&root), true, None);

        assert_eq!(check.status, "failed");
        assert!(
            check
                .value
                .contains("release-check artifact path must not be a symlink")
        );
    }

    #[test]
    fn release_check_accepts_ci_signing_and_notarization_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let doctor_dir = root.join("ci-doctor");
        fs::create_dir(&doctor_dir).unwrap();
        write_doctor_artifact(&doctor_dir.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&doctor_dir.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&doctor_dir.join("doctor-Windows.json"), "Windows");
        let ci_release_check_dir = root.join("ci-release-checks");
        fs::create_dir(&ci_release_check_dir).unwrap();
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Windows.json"));
        let macos_signing_log = root.join("signing-macos.log");
        fs::write(&macos_signing_log, "codesign=pass\n").unwrap();
        let windows_signing_log = root.join("signing-windows.log");
        fs::write(&windows_signing_log, "signtool=pass\n").unwrap();
        let notary_log = root.join("notary.log");
        fs::write(
            &notary_log,
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        let validate_report = root.join("validate-report.json");
        write_validate_artifact(&validate_report, "ok", "passed");
        let mut validate_reports = vec![validate_report];
        validate_reports.extend(write_example_validate_matrix(&root.join("validator")));
        let static_reports = write_example_static_validate_matrix(&root.join("package"));
        let platform_smoke_dir = root.join("platform-smoke");
        write_platform_smoke_matrix(&platform_smoke_dir);
        let publish_plan_report = root.join("publish-plan.json");
        write_publish_plan_artifact(&publish_plan_report);
        let crate_package_report = root.join("crate-package.json");
        write_crate_package_artifact(&crate_package_report);
        let npm_pack_report = root.join("npm-pack.json");
        write_npm_pack_artifact(&npm_pack_report);
        let dependency_baseline_report = root.join("dependency-baseline-latest.json");
        write_dependency_baseline_latest_artifact(&dependency_baseline_report);
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let protocol = root.join("protocol");
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let options = ReleaseEvidenceOptions {
            ci_doctor_dir: Some(doctor_dir),
            ci_release_check_dir: Some(ci_release_check_dir),
            platform_smoke_dir: Some(platform_smoke_dir),
            ci_run_url: Some(
                "https://github.com/vesty-rs/vesty/actions/runs/1234567890".to_string(),
            ),
            validate_reports,
            static_validate_reports: static_reports,
            publish_plan_report: Some(publish_plan_report),
            crate_package_report: Some(crate_package_report),
            npm_pack_report: Some(npm_pack_report),
            dependency_baseline_report: Some(dependency_baseline_report),
            vst3_sdk_manifest: None,
            vst3_sdk_binding_plan: None,
            vst3_sdk_binding_surface: None,
            vst3_sdk_scaffold: None,
            vst3_sdk_abi_seed: None,
            vst3_sdk_abi: None,
            vst3_sdk_interface_skeleton: None,
            signed_bundle_evidence: vec![macos_signing_log, windows_signing_log],
            notarization_log: Some(notary_log),
            require_release_artifacts: true,
        };

        let report = build_release_check_report(rows, &protocol, false, &options);

        assert!(release_check_complete(&report));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci doctor artifacts"
                && check.status == "ok"
                && check.value.contains("3 doctor reports parsed")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "ci release-check artifacts"
                && check.status == "ok"
                && check.value.contains("3 release-check report")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "platform smoke artifacts"
                && check.status == "ok"
                && check.value.contains("macOS")
                && check.value.contains("Windows x64")
                && check.value.contains("Linux X11")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "signed bundle evidence"
                && check.status == "ok"
                && check.value.contains("macOS")
                && check.value.contains("Windows")
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "notarization log" && check.status == "ok" })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 validate reports" && check.status == "ok" })
        );
        assert!(
            report.checks.iter().any(|check| {
                check.name == "vst3 static validate reports" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "crate publish plan"
                && check.status == "ok"
                && check.value.contains("publishable crates")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "crate package readiness"
                && check.status == "ok"
                && check.value.contains("packageable now")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "npm package pack report"
                && check.status == "ok"
                && check.value.contains("vesty-plugin-ui")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "dependency latest baseline"
                && check.status == "ok"
                && check.value.contains("latest registry check")
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "ok"
        }));
    }

    #[test]
    fn release_evidence_dir_populates_standard_evidence_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        let doctor_dir = root.join("ci-doctor");
        fs::create_dir(&doctor_dir).unwrap();
        write_doctor_artifact(&doctor_dir.join("doctor-Linux.json"), "Linux");
        write_doctor_artifact(&doctor_dir.join("doctor-macOS.json"), "macOS");
        write_doctor_artifact(&doctor_dir.join("doctor-Windows.json"), "Windows");
        let ci_release_check_dir = root.join("ci-release-checks");
        fs::create_dir(&ci_release_check_dir).unwrap();
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Linux.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-macOS.json"));
        write_ci_release_check_artifact(&ci_release_check_dir.join("release-check-Windows.json"));
        let platform_smoke_dir = root.join("platform-smoke");
        write_platform_smoke_matrix(&platform_smoke_dir);
        write_validate_artifact(&root.join("validate-report.json"), "ok", "passed");
        let example_validate_reports =
            write_example_validate_matrix(&root.join("downloaded-artifacts/validator"));
        write_validate_artifact(&root.join("static-validate-report.json"), "ok", "skipped");
        let static_matrix_reports =
            write_example_static_validate_matrix(&root.join("downloaded-artifacts/package"));
        let publish_plan_report = root.join("publish-plan/publish-plan.json");
        write_publish_plan_artifact(&publish_plan_report);
        let crate_package_report = root.join("crate-package/crate-package.json");
        write_crate_package_artifact(&crate_package_report);
        let npm_pack_report = root.join("npm-pack/npm-pack.json");
        write_npm_pack_artifact(&npm_pack_report);
        let dependency_baseline_report =
            root.join("dependency-baseline/dependency-baseline-latest.json");
        write_dependency_baseline_latest_artifact(&dependency_baseline_report);
        let vst3_sdk_manifest = root.join("vst3-sdk/vst3-sdk-headers.json");
        write_test_vst3_sdk_manifest(&vst3_sdk_manifest, &[]);
        let vst3_sdk_binding_plan = root.join("vst3-sdk/generated-bindings-plan.json");
        write_test_vst3_sdk_binding_plan(
            &vst3_sdk_binding_plan,
            &[],
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let vst3_sdk_binding_surface = root.join("vst3-sdk/generated-bindings-surface.json");
        write_test_vst3_sdk_binding_surface(&vst3_sdk_binding_surface, &[]);
        let vst3_sdk_scaffold = root.join("vst3-sdk/generated.rs");
        write_test_vst3_sdk_scaffold(
            &vst3_sdk_scaffold,
            Utf8Path::new("target/vst3-sdk/generated.rs"),
        );
        let vst3_sdk_abi_seed = root.join("vst3-sdk/generated-abi-seed.rs");
        write_test_vst3_sdk_abi_seed(&vst3_sdk_abi_seed);
        let vst3_sdk_abi = root.join("vst3-sdk/generated-abi.rs");
        write_test_vst3_sdk_abi(&vst3_sdk_abi);
        let vst3_sdk_interface_skeleton = root.join("vst3-sdk/generated-interface-skeleton.rs");
        write_test_vst3_sdk_interface_skeleton(&vst3_sdk_interface_skeleton);
        fs::write(
            root.join("signing-macos.log"),
            "signed=true\ncodesign=pass\n",
        )
        .unwrap();
        fs::write(root.join("signing-windows.log"), "signtool=pass\n").unwrap();
        fs::write(
            root.join("notary.log"),
            "status: Accepted\nThe staple and validate action worked!\n",
        )
        .unwrap();
        let rows = vesty_core::host_profiles()
            .iter()
            .map(|profile| complete_release_row(profile.name))
            .collect::<Vec<_>>();
        let protocol = root.join("protocol");
        vesty_ipc::export_protocol_bindings(&protocol).unwrap();
        let mut options = ReleaseEvidenceOptions {
            require_release_artifacts: true,
            ..ReleaseEvidenceOptions::default()
        };

        apply_release_evidence_dir(&mut options, &root).unwrap();
        let report = build_release_check_report(rows, &protocol, false, &options);

        assert!(release_check_complete(&report));
        assert_eq!(
            options.ci_run_url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890")
        );
        assert_eq!(options.ci_doctor_dir.as_deref(), Some(doctor_dir.as_path()));
        assert_eq!(
            options.ci_release_check_dir.as_deref(),
            Some(ci_release_check_dir.as_path())
        );
        assert_eq!(
            options.platform_smoke_dir.as_deref(),
            Some(platform_smoke_dir.as_path())
        );
        assert_eq!(
            options.publish_plan_report.as_deref(),
            Some(publish_plan_report.as_path())
        );
        assert_eq!(
            options.crate_package_report.as_deref(),
            Some(crate_package_report.as_path())
        );
        assert_eq!(
            options.npm_pack_report.as_deref(),
            Some(npm_pack_report.as_path())
        );
        assert_eq!(
            options.dependency_baseline_report.as_deref(),
            Some(dependency_baseline_report.as_path())
        );
        assert_eq!(
            options.vst3_sdk_manifest.as_deref(),
            Some(vst3_sdk_manifest.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_plan.as_deref(),
            Some(vst3_sdk_binding_plan.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_surface.as_deref(),
            Some(vst3_sdk_binding_surface.as_path())
        );
        assert_eq!(
            options.vst3_sdk_scaffold.as_deref(),
            Some(vst3_sdk_scaffold.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi_seed.as_deref(),
            Some(vst3_sdk_abi_seed.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi.as_deref(),
            Some(vst3_sdk_abi.as_path())
        );
        assert_eq!(
            options.vst3_sdk_interface_skeleton.as_deref(),
            Some(vst3_sdk_interface_skeleton.as_path())
        );
        assert_eq!(options.validate_reports, {
            let mut expected = vec![root.join("validate-report.json")];
            expected.extend(example_validate_reports.clone());
            expected
        });
        assert!(
            options
                .static_validate_reports
                .contains(&root.join("static-validate-report.json"))
        );
        for path in static_matrix_reports {
            assert!(options.static_validate_reports.contains(&path));
        }
        assert_eq!(options.notarization_log, Some(root.join("notary.log")));
        assert!(
            report.checks.iter().any(|check| {
                check.name == "vst3 static validate reports" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "ci example static validate coverage" && check.status == "ok"
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "crate package readiness" && check.status == "ok" })
        );
        assert!(
            report.checks.iter().any(|check| {
                check.name == "dependency latest baseline" && check.status == "ok"
            })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 example validator coverage" && check.status == "ok"
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "vst3 SDK header manifest" && check.status == "ok" })
        );
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings plan" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings surface" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings scaffold" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings ABI seed" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings ABI layout" && check.status == "ok"
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "vst3 SDK generated bindings interface skeleton" && check.status == "ok"
        }));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_report_paths_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(root.join("publish-plan")).unwrap();
        fs::create_dir_all(root.join("crate-package")).unwrap();
        fs::create_dir_all(root.join("npm-pack")).unwrap();
        fs::create_dir_all(root.join("dependency-baseline")).unwrap();
        let publish_plan = root.join("publish-plan/publish-plan.json");
        let crate_package = root.join("crate-package/crate-package.json");
        let npm_pack = root.join("npm-pack/npm-pack.json");
        let dependency_baseline = root.join("dependency-baseline/dependency-baseline-latest.json");
        fs::write(&publish_plan, "{").unwrap();
        fs::write(&crate_package, r#"{ "packages": [] }"#).unwrap();
        fs::write(&npm_pack, r#"{ "not": "an npm pack array" }"#).unwrap();
        fs::write(&dependency_baseline, r#"{ "checks": [] }"#).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(
            options.publish_plan_report.as_deref(),
            Some(publish_plan.as_path())
        );
        assert_eq!(
            options.crate_package_report.as_deref(),
            Some(crate_package.as_path())
        );
        assert_eq!(options.npm_pack_report.as_deref(), Some(npm_pack.as_path()));
        assert_eq!(
            options.dependency_baseline_report.as_deref(),
            Some(dependency_baseline.as_path())
        );

        let publish_check =
            publish_plan_release_check(options.publish_plan_report.as_deref(), true);
        let crate_check = crate_package_release_check(
            options.crate_package_report.as_deref(),
            options.publish_plan_report.as_deref(),
            true,
        );
        let npm_check = npm_pack_release_check(options.npm_pack_report.as_deref(), true);
        let dependency_check = dependency_baseline_latest_release_check(
            options.dependency_baseline_report.as_deref(),
            true,
        );

        assert_eq!(publish_check.status, "failed");
        assert!(publish_check.value.contains("invalid publish plan JSON"));
        assert_eq!(crate_check.status, "failed");
        assert!(crate_check.value.contains("invalid crate package report"));
        assert_eq!(npm_check.status, "failed");
        assert!(npm_check.value.contains("invalid npm pack report JSON"));
        assert_eq!(dependency_check.status, "failed");
        assert!(!dependency_check.value.contains("required evidence missing"));
        assert!(dependency_check.value.contains("dependency"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_vst3_sdk_artifacts_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(root.join("vst3-sdk")).unwrap();
        let manifest = root.join("vst3-sdk/vst3-sdk-headers.json");
        let binding_plan = root.join("vst3-sdk/generated-bindings-plan.json");
        let binding_surface = root.join("vst3-sdk/generated-bindings-surface.json");
        let scaffold = root.join("vst3-sdk/generated.rs");
        let abi_seed = root.join("vst3-sdk/generated-abi-seed.rs");
        let abi = root.join("vst3-sdk/generated-abi.rs");
        let interface_skeleton = root.join("vst3-sdk/generated-interface-skeleton.rs");
        fs::write(&manifest, r#"{ "kind": "broken" }"#).unwrap();
        fs::write(&binding_plan, "{").unwrap();
        fs::write(
            &binding_surface,
            r#"{ "bindingsGenerated": true, "status": "generated" }"#,
        )
        .unwrap();
        fs::write(&scaffold, "pub const BINDINGS_GENERATED: bool = true;\n").unwrap();
        fs::write(
            &abi_seed,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            &abi,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        fs::write(
            &interface_skeleton,
            "pub const FULL_COM_BINDINGS_GENERATED: bool = true;\n",
        )
        .unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(
            options.vst3_sdk_manifest.as_deref(),
            Some(manifest.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_plan.as_deref(),
            Some(binding_plan.as_path())
        );
        assert_eq!(
            options.vst3_sdk_binding_surface.as_deref(),
            Some(binding_surface.as_path())
        );
        assert_eq!(
            options.vst3_sdk_scaffold.as_deref(),
            Some(scaffold.as_path())
        );
        assert_eq!(
            options.vst3_sdk_abi_seed.as_deref(),
            Some(abi_seed.as_path())
        );
        assert_eq!(options.vst3_sdk_abi.as_deref(), Some(abi.as_path()));
        assert_eq!(
            options.vst3_sdk_interface_skeleton.as_deref(),
            Some(interface_skeleton.as_path())
        );

        let manifest_check = vst3_sdk_manifest_release_check(options.vst3_sdk_manifest.as_deref());
        let plan_check =
            vst3_sdk_binding_plan_release_check(options.vst3_sdk_binding_plan.as_deref());
        let surface_check =
            vst3_sdk_binding_surface_release_check(options.vst3_sdk_binding_surface.as_deref());
        let scaffold_check =
            vst3_sdk_generated_scaffold_release_check(options.vst3_sdk_scaffold.as_deref());
        let abi_seed_check =
            vst3_sdk_generated_abi_seed_release_check(options.vst3_sdk_abi_seed.as_deref());
        let abi_check = vst3_sdk_generated_abi_release_check(options.vst3_sdk_abi.as_deref());
        let interface_check = vst3_sdk_generated_interface_skeleton_release_check(
            options.vst3_sdk_interface_skeleton.as_deref(),
        );

        assert_eq!(manifest_check.status, "failed");
        assert!(
            manifest_check
                .value
                .contains(&portable_report_path(&manifest))
        );
        assert!(
            manifest_check
                .value
                .contains("invalid VST3 SDK header manifest JSON"),
            "{}",
            manifest_check.value
        );
        assert_eq!(plan_check.status, "failed");
        assert!(
            plan_check
                .value
                .contains(&portable_report_path(&binding_plan))
        );
        assert!(
            plan_check
                .value
                .contains("invalid VST3 SDK generated bindings plan JSON"),
            "{}",
            plan_check.value
        );
        assert_eq!(surface_check.status, "failed");
        assert!(
            surface_check
                .value
                .contains(&portable_report_path(&binding_surface))
        );
        assert!(
            surface_check
                .value
                .contains("invalid VST3 SDK generated bindings surface JSON"),
            "{}",
            surface_check.value
        );
        assert_eq!(scaffold_check.status, "failed");
        assert!(
            scaffold_check
                .value
                .contains(&portable_report_path(&scaffold))
        );
        assert!(
            scaffold_check
                .value
                .contains("must not claim SDK bindings are generated"),
            "{}",
            scaffold_check.value
        );
        assert_eq!(abi_seed_check.status, "failed");
        assert!(
            abi_seed_check
                .value
                .contains(&portable_report_path(&abi_seed))
        );
        assert!(
            abi_seed_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            abi_seed_check.value
        );
        assert_eq!(abi_check.status, "failed");
        assert!(abi_check.value.contains(&portable_report_path(&abi)));
        assert!(
            abi_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            abi_check.value
        );
        assert_eq!(interface_check.status, "failed");
        assert!(
            interface_check
                .value
                .contains(&portable_report_path(&interface_skeleton))
        );
        assert!(
            interface_check
                .value
                .contains("must not claim full COM bindings are generated"),
            "{}",
            interface_check.value
        );
        for check in [
            manifest_check,
            plan_check,
            surface_check,
            scaffold_check,
            abi_seed_check,
            abi_check,
            interface_check,
        ] {
            assert!(!check.value.contains("not requested"), "{check:?}");
        }
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_validate_reports_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        let validate_report = root.join("validate-report.json");
        let static_validate_report = root.join("static-validate-report.json");
        fs::write(&validate_report, "{").unwrap();
        write_validate_artifact(&static_validate_report, "failed", "skipped");
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.validate_reports, vec![validate_report.clone()]);
        assert_eq!(
            options.static_validate_reports,
            vec![static_validate_report.clone()]
        );

        let validate_check = validate_reports_release_check(&options.validate_reports, true);
        let static_check =
            static_validate_reports_release_check(&options.static_validate_reports, false);

        assert_eq!(validate_check.status, "failed");
        assert!(validate_check.value.contains(validate_report.as_str()));
        assert!(!validate_check.value.contains("required evidence missing"));
        assert_eq!(static_check.status, "failed");
        assert!(
            static_check
                .value
                .contains("static bundle check status is failed")
        );
        assert!(!static_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let validator_dir = root.join("validator");
        let package_dir = root.join("package");
        fs::create_dir_all(&validator_dir).unwrap();
        fs::create_dir_all(&package_dir).unwrap();
        let validate_report = validator_dir.join("VestyGain.macos.validate.json");
        let static_validate_report = package_dir.join("VestyGain.linux-x64.static-validate.json");
        let release_check_sidecar = package_dir.join("static-validate-release-check.json");
        let notes = root.join("notes.json");
        fs::write(&validate_report, "{").unwrap();
        write_validate_artifact(&static_validate_report, "failed", "skipped");
        fs::write(&release_check_sidecar, r#"{"status":"failed","checks":[]}"#).unwrap();
        fs::write(&notes, r#"{ "note": true }"#).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.validate_reports, vec![validate_report.clone()]);
        assert_eq!(
            options.static_validate_reports,
            vec![static_validate_report.clone()]
        );
        assert!(!options.validate_reports.contains(&notes));
        assert!(!options.static_validate_reports.contains(&notes));
        assert!(
            !options
                .static_validate_reports
                .contains(&release_check_sidecar)
        );

        let validate_check = validate_reports_release_check(&options.validate_reports, true);
        let static_check =
            static_validate_reports_release_check(&options.static_validate_reports, false);

        assert_eq!(validate_check.status, "failed");
        assert!(validate_check.value.contains(validate_report.as_str()));
        assert!(
            validate_check
                .value
                .contains("invalid validate report JSON")
        );
        assert!(!validate_check.value.contains("required evidence missing"));
        assert_eq!(static_check.status, "failed");
        assert!(
            static_check
                .value
                .contains("static bundle check status is failed")
        );
        assert!(!static_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_keeps_invalid_standard_signing_and_notary_logs_for_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        let macos_signing = root.join("signing-macos.log");
        let windows_signing = root.join("signing-windows.log");
        let notary_log = root.join("notary.log");
        fs::write(&macos_signing, "codesign=pass\ninvalid signature\n").unwrap();
        fs::write(
            &windows_signing,
            "signtool verify /pa /v VestyGain.vst3\nNumber of errors: 1\n",
        )
        .unwrap();
        fs::write(&notary_log, "status: Rejected\n").unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert!(options.signed_bundle_evidence.contains(&macos_signing));
        assert!(options.signed_bundle_evidence.contains(&windows_signing));
        assert_eq!(options.notarization_log, Some(notary_log.clone()));

        let signing_check =
            signed_bundle_evidence_release_check(&options.signed_bundle_evidence, true);
        let notary_check =
            notarization_log_release_check(options.notarization_log.as_deref(), true);

        assert_eq!(signing_check.status, "failed");
        assert!(signing_check.value.contains("invalid signature"));
        assert!(signing_check.value.contains("number of errors: 1"));
        assert!(!signing_check.value.contains("required evidence missing"));
        assert_eq!(notary_check.status, "failed");
        assert!(notary_check.value.contains("status: rejected"));
        assert!(
            notary_check
                .value
                .contains("negative notarization evidence")
        );
        assert!(!notary_check.value.contains("required evidence missing"));
    }

    #[test]
    fn release_evidence_dir_accepts_matching_explicit_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/2\n",
        )
        .unwrap();
        let cli_url = "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1";
        let mut options = ReleaseEvidenceOptions {
            ci_run_url: Some(cli_url.to_string()),
            ..ReleaseEvidenceOptions::default()
        };

        apply_release_evidence_dir(&mut options, &root).unwrap();

        assert_eq!(options.ci_run_url.as_deref(), Some(cli_url));
    }

    #[test]
    fn release_evidence_dir_rejects_mismatched_explicit_ci_run_url() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ci-run-url.txt"),
            "ci_run_url=https://github.com/vesty-rs/other/actions/runs/1234567890\n",
        )
        .unwrap();
        let mut options = ReleaseEvidenceOptions {
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ..ReleaseEvidenceOptions::default()
        };

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("mismatched explicit release evidence URL should fail")
            .to_string();

        assert!(error.contains("refer to different GitHub Actions runs"));
        assert!(error.contains("vesty-rs/vesty, run 42"));
        assert!(error.contains("vesty-rs/other, run 1234567890"));
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_ci_run_url_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-ci-run-url.txt")).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &external,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        unix_fs::symlink(&external, root.join("ci-run-url.txt")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked ci-run-url.txt")
            .to_string();

        assert!(error.contains("CI run URL evidence must not be a symlink"));
        assert!(options.ci_run_url.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_root_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-release-evidence")).unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        fs::create_dir_all(&external).unwrap();
        write_publish_plan_artifact(&external.join("publish-plan.json"));
        unix_fs::symlink(&external, &root).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir root symlink should be rejected")
            .to_string();

        assert!(error.contains("release evidence dir must not be a symlink"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_standard_file_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-publish-plan.json")).unwrap();
        fs::create_dir_all(root.join("publish-plan")).unwrap();
        write_publish_plan_artifact(&external);
        unix_fs::symlink(&external, root.join("publish-plan/publish-plan.json")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked standard files")
            .to_string();

        assert!(error.contains("release evidence path must not be a symlink"));
        assert!(error.contains("publish-plan.json"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_standard_dir_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(temp.path().join("external-publish-plan")).unwrap();
        fs::create_dir_all(&external).unwrap();
        write_publish_plan_artifact(&external.join("publish-plan.json"));
        fs::create_dir_all(&root).unwrap();
        unix_fs::symlink(&external, root.join("publish-plan")).unwrap();
        let mut options = ReleaseEvidenceOptions::default();

        let error = apply_release_evidence_dir(&mut options, &root)
            .expect_err("release evidence dir must reject symlinked standard dirs")
            .to_string();

        assert!(error.contains("release evidence path must not be a symlink"));
        assert!(error.contains("publish-plan"));
        assert!(options.publish_plan_report.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn release_evidence_dir_rejects_validator_and_package_symlink_dirs() {
        for dir_name in ["validator", "package"] {
            let temp = tempfile::tempdir().unwrap();
            let root = Utf8PathBuf::from_path_buf(temp.path().join("release-evidence")).unwrap();
            let external =
                Utf8PathBuf::from_path_buf(temp.path().join(format!("external-{dir_name}")))
                    .unwrap();
            fs::create_dir_all(&root).unwrap();
            fs::create_dir_all(&external).unwrap();
            write_validate_artifact(
                &external.join(format!("{dir_name}.json")),
                "ok",
                if dir_name == "validator" {
                    "passed"
                } else {
                    "skipped"
                },
            );
            unix_fs::symlink(&external, root.join(dir_name)).unwrap();
            let mut options = ReleaseEvidenceOptions::default();

            let error = apply_release_evidence_dir(&mut options, &root)
                .expect_err("release evidence dir must reject symlinked matrix dirs")
                .to_string();

            assert!(error.contains("JSON artifact contains symlink"));
            assert!(error.contains(dir_name));
            assert!(options.validate_reports.is_empty());
            assert!(options.static_validate_reports.is_empty());
        }
    }

    #[test]
    fn ci_run_url_file_accepts_raw_url_and_named_key_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "# release evidence\n\
             note=https://github.com/vesty-rs/vesty/actions/runs/111\n\
             ci-run-url=pending\n\
             ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();

        let url = read_ci_run_url_file(&file).unwrap();

        assert_eq!(
            url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890")
        );

        fs::write(&file, "https://github.com/vesty-rs/vesty/actions/runs/42\n").unwrap();

        assert_eq!(
            read_ci_run_url_file(&file).unwrap().as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/42")
        );
    }

    #[test]
    fn ci_run_url_file_ignores_unrelated_key_value_lines() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "note=https://github.com/vesty-rs/vesty/actions/runs/111\n\
             ci_run_url=PENDING\n",
        )
        .unwrap();

        assert_eq!(read_ci_run_url_file(&file).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn ci_run_url_file_rejects_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let external = root.join("external-ci-run-url.txt");
        let file = root.join("ci-run-url.txt");
        fs::write(
            &external,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890\n",
        )
        .unwrap();
        unix_fs::symlink(&external, &file).unwrap();

        let error = read_ci_run_url_file(&file)
            .expect_err("CI run URL evidence symlink should be rejected")
            .to_string();

        assert!(error.contains("CI run URL evidence must not be a symlink"));
    }

    #[test]
    fn import_ci_run_url_evidence_accepts_matching_cli_and_file_urls() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        let file = root.join("ci-run-url.txt");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            &file,
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/2\n",
        )
        .unwrap();
        let options = ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: Some(
                "https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1".to_string(),
            ),
            ci_run_url_file: Some(file.clone()),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let url = import_ci_run_url_evidence(&options, &[], &mut items).unwrap();

        assert_eq!(
            url.as_deref(),
            Some("https://github.com/vesty-rs/vesty/actions/runs/1234567890/attempts/1")
        );
        assert_eq!(
            read_ci_run_url_file(&evidence.join("ci-run-url.txt"))
                .unwrap()
                .as_deref(),
            url.as_deref()
        );
        assert!(items.iter().any(|item| {
            item.name == "ci run url"
                && item.status == "imported"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_paths_equal(source, file.as_str()))
        }));
    }

    #[test]
    fn import_ci_run_url_evidence_rejects_mismatched_cli_and_file_urls() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(
            &file,
            "ci_run_url=https://github.com/vesty-rs/other/actions/runs/1234567890\n",
        )
        .unwrap();
        let options = ImportCiOptions {
            source: root.join("downloaded-artifacts"),
            dir: root.join("release-evidence"),
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ci_run_url_file: Some(file),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let error = import_ci_run_url_evidence(&options, &[], &mut items)
            .expect_err("mismatched explicit CI run URL sources should fail")
            .to_string();

        assert!(error.contains("refer to different GitHub Actions runs"));
        assert!(error.contains("vesty-rs/vesty, run 42"));
        assert!(error.contains("vesty-rs/other, run 1234567890"));
        assert!(items.is_empty());
    }

    #[test]
    fn import_ci_run_url_evidence_rejects_invalid_file_even_when_cli_url_is_valid() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let file = root.join("ci-run-url.txt");
        fs::write(&file, "ci_run_url=not-a-github-run-url\n").unwrap();
        let options = ImportCiOptions {
            source: root.join("downloaded-artifacts"),
            dir: root.join("release-evidence"),
            ci_run_url: Some("https://github.com/vesty-rs/vesty/actions/runs/42".to_string()),
            ci_run_url_file: Some(file),
            template: false,
            overwrite: false,
            format: "json".to_string(),
        };
        let mut items = Vec::new();

        let error = import_ci_run_url_evidence(&options, &[], &mut items)
            .expect_err("invalid CI run URL file should fail even with a valid CLI URL")
            .to_string();

        assert!(error.contains("--ci-run-url-file"));
        assert!(error.contains("is not a valid GitHub Actions run URL"));
        assert!(items.is_empty());
    }

    #[test]
    fn import_ci_reports_invalid_auto_discovered_ci_run_url_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
        let source = root.join("downloaded-artifacts");
        let evidence = root.join("release-evidence");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("ci-run-url.txt"), "ci_run_url=not-a-run-url\n").unwrap();
        fs::write(
            source.join("notes.txt"),
            "ci_run_url=https://github.com/vesty-rs/vesty/actions/runs/latest\n",
        )
        .unwrap();

        import_ci_release_evidence(ImportCiOptions {
            source,
            dir: evidence.clone(),
            ci_run_url: None,
            ci_run_url_file: None,
            template: false,
            overwrite: false,
            format: "json".to_string(),
        })
        .unwrap();

        assert!(!evidence.join("ci-run-url.txt").exists());
        let report: ImportCiReleaseEvidenceReport = serde_json::from_str(
            &fs::read_to_string(evidence.join("import-ci-report.json")).unwrap(),
        )
        .unwrap();
        assert!(report.items.iter().any(|item| {
            item.name == "ci run url"
                && item.status == "failed"
                && item.value.contains("invalid GitHub Actions run URL")
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "ci-run-url.txt"))
        }));
        assert!(report.items.iter().any(|item| {
            item.name == "text artifact"
                && item.status == "skipped"
                && item
                    .source
                    .as_deref()
                    .is_some_and(|source| release_report_path_ends_with(source, "notes.txt"))
        }));
    }


