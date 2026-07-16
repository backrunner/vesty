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
