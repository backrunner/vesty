use super::*;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use vesty_params::{VST3_PARAM_ID_ALGORITHM, stable_vst3_param_id};

#[test]
fn builds_asset_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("index.html"), "<main></main>").unwrap();
    let manifest = AssetManifest::from_dir(&root, "index.html").unwrap();
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].path, "index.html");
    assert_eq!(manifest.files[0].mime, "text/html");
}

#[test]
fn asset_manifest_rejects_unknown_json_fields() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("index.html"), "<main></main>").unwrap();
    let manifest = AssetManifest::from_dir(&root, "index.html").unwrap();

    let mut unknown_manifest_field = serde_json::to_value(&manifest).unwrap();
    unknown_manifest_field["generatedBy"] = serde_json::json!("forged");
    let error = serde_json::from_value::<AssetManifest>(unknown_manifest_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `generatedBy`"));

    let mut unknown_file_field = serde_json::to_value(&manifest).unwrap();
    unknown_file_field["files"][0]["mode"] = serde_json::json!("0755");
    let error = serde_json::from_value::<AssetManifest>(unknown_file_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `mode`"));
}

#[test]
fn asset_manifest_requires_entry() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("main.js"), "console.log(1)").unwrap();
    let error = AssetManifest::from_dir(&root, "index.html").unwrap_err();
    assert!(matches!(error, BuildError::MissingFile(_)));
}

#[cfg(unix)]
#[test]
fn asset_manifest_rejects_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("index.html"), "<main></main>").unwrap();
    std::os::unix::fs::symlink(root.join("index.html"), root.join("link.html")).unwrap();
    let error = AssetManifest::from_dir(&root, "index.html").unwrap_err();
    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn asset_manifest_rejects_symlinked_root() {
    let dir = tempfile::tempdir().unwrap();
    let external_root = Utf8PathBuf::from_path_buf(dir.path().join("external-dist")).unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir(&external_root).unwrap();
    fs::write(external_root.join("index.html"), "<main></main>").unwrap();
    std::os::unix::fs::symlink(&external_root, &root).unwrap();

    let error = AssetManifest::from_dir(&root, "index.html").unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[test]
fn asset_manifest_rejects_url_ambiguous_dist_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().join("dist")).unwrap();
    fs::create_dir_all(root.join("assets/%2e%2e")).unwrap();
    fs::write(root.join("index.html"), "<main></main>").unwrap();
    fs::write(root.join("assets/%2e%2e/app.js"), "console.log(1)").unwrap();

    let error = AssetManifest::from_dir(&root, "index.html").unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
}

#[test]
fn maps_bundle_paths() {
    assert_eq!(
        binary_relative_path(BundlePlatform::Macos, "Gain").as_str(),
        "Contents/MacOS/Gain"
    );
    assert_eq!(
        binary_relative_path(BundlePlatform::WindowsX64, "Gain").as_str(),
        "Contents/x86_64-win/Gain.vst3"
    );
    assert_eq!(
        binary_relative_path(BundlePlatform::LinuxX64, "Gain").as_str(),
        "Contents/x86_64-linux/Gain.so"
    );
}

#[test]
fn packages_vst3_bundle() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);

    assert_common_bundle_files(&report, "Contents/MacOS/Gain");
    assert!(report.bundle_dir.join("Contents/Info.plist").is_file());
    assert!(report.bundle_dir.join("Contents/PkgInfo").is_file());
}

#[test]
fn packages_windows_vst3_bundle() {
    let (_dir, report) = package_fixture(BundlePlatform::WindowsX64);

    assert_common_bundle_files(&report, "Contents/x86_64-win/Gain.vst3");
    assert!(!report.bundle_dir.join("Contents/Info.plist").exists());
    assert!(!report.bundle_dir.join("Contents/PkgInfo").exists());
}

#[test]
fn packages_linux_vst3_bundle() {
    let (_dir, report) = package_fixture(BundlePlatform::LinuxX64);

    assert_common_bundle_files(&report, "Contents/x86_64-linux/Gain.so");
    assert!(!report.bundle_dir.join("Contents/Info.plist").exists());
    assert!(!report.bundle_dir.join("Contents/PkgInfo").exists());
}

#[test]
fn packages_and_validates_merged_multi_platform_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    fs::write(ui_dist.join("main.js"), "console.log('vesty')").unwrap();

    let config = test_config();
    let bundle_dir = out.join("Gain.vst3");
    let mut reports = Vec::new();
    for (platform, binary_name) in [
        (BundlePlatform::Macos, "Gain-macos"),
        (BundlePlatform::WindowsX64, "Gain-windows"),
        (BundlePlatform::LinuxX64, "Gain-linux"),
    ] {
        let binary = project.join("target/release").join(binary_name);
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, test_binary_bytes(platform)).unwrap();
        reports.push(
            package_vst3(
                &config,
                &PackageOptions {
                    project_dir: project.clone(),
                    output_dir: out.clone(),
                    platform,
                    binary_path: binary,
                },
            )
            .unwrap(),
        );
    }

    assert!(reports.iter().all(|report| report.bundle_dir == bundle_dir));
    let validation = validate_vst3_bundle(&bundle_dir).unwrap();
    let mut binary_paths = validation
        .binary_paths
        .iter()
        .map(|path| path.strip_prefix(&bundle_dir).unwrap().to_path_buf())
        .collect::<Vec<_>>();
    binary_paths.sort();

    assert_eq!(
        binary_paths,
        vec![
            Utf8PathBuf::from("Contents/MacOS/Gain"),
            Utf8PathBuf::from("Contents/x86_64-linux/Gain.so"),
            Utf8PathBuf::from("Contents/x86_64-win/Gain.vst3"),
        ]
    );
    assert_eq!(validation.asset_count, 2);
}

#[test]
fn moduleinfo_uses_package_category_when_present() {
    let mut config = test_config();
    config.plugin.kind = "effect".to_string();
    config.package.as_mut().unwrap().category = Some("Fx|Analyzer".to_string());
    let (_dir, report) = package_fixture_with_config(BundlePlatform::Macos, config);

    let text = fs::read_to_string(report.moduleinfo_path).unwrap();
    let moduleinfo: ModuleInfo = serde_json::from_str(&text).unwrap();

    assert_eq!(moduleinfo.classes[0].category, "Fx|Analyzer");
}

#[test]
fn moduleinfo_uses_plugin_kind_when_package_category_is_empty() {
    let mut config = test_config();
    config.plugin.kind = "Fx".to_string();
    config.package.as_mut().unwrap().category = Some(" ".to_string());
    let (_dir, report) = package_fixture_with_config(BundlePlatform::Macos, config);

    let text = fs::read_to_string(report.moduleinfo_path).unwrap();
    let moduleinfo: ModuleInfo = serde_json::from_str(&text).unwrap();

    assert_eq!(moduleinfo.classes[0].category, "Fx");
}

#[test]
fn moduleinfo_maps_effect_kind_to_vst3_fx_category_when_package_category_is_empty() {
    let mut config = test_config();
    config.plugin.kind = "effect".to_string();
    config.package.as_mut().unwrap().category = None;
    let (_dir, report) = package_fixture_with_config(BundlePlatform::Macos, config);

    let text = fs::read_to_string(report.moduleinfo_path).unwrap();
    let moduleinfo: ModuleInfo = serde_json::from_str(&text).unwrap();

    assert_eq!(moduleinfo.classes[0].category, "Fx");
}

#[test]
fn moduleinfo_rejects_unknown_json_fields() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    let moduleinfo: ModuleInfo =
        serde_json::from_str(&fs::read_to_string(&report.moduleinfo_path).unwrap()).unwrap();

    let mut unknown_moduleinfo_field = serde_json::to_value(&moduleinfo).unwrap();
    unknown_moduleinfo_field["generatedBy"] = serde_json::json!("forged");
    let error = serde_json::from_value::<ModuleInfo>(unknown_moduleinfo_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `generatedBy`"));

    let mut unknown_class_field = serde_json::to_value(&moduleinfo).unwrap();
    unknown_class_field["classes"][0]["owner"] = serde_json::json!("release");
    let error = serde_json::from_value::<ModuleInfo>(unknown_class_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `owner`"));
}

#[test]
fn parameter_manifest_from_param_specs_records_stable_host_ids() {
    let manifest = ParameterManifest::from_param_specs(vec![
        vesty_params::ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
        vesty_params::ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 0),
    ])
    .unwrap();

    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.id_algorithm, VST3_PARAM_ID_ALGORITHM);
    assert_eq!(manifest.parameters[0].id, "gain");
    assert_eq!(
        manifest.parameters[0].vst3_param_id,
        stable_vst3_param_id("gain")
    );
    assert!(!manifest.parameters[0].spec.flags.program_change);
    validate_parameter_manifest(&manifest).unwrap();
}

#[test]
fn parameter_manifest_rejects_unknown_json_fields() {
    let manifest = ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", -60.0, 12.0, 0.0,
    )])
    .unwrap();

    let mut unknown_manifest_field = serde_json::to_value(&manifest).unwrap();
    unknown_manifest_field["generatedBy"] = serde_json::json!("forged");
    let error = serde_json::from_value::<ParameterManifest>(unknown_manifest_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `generatedBy`"));

    let mut unknown_entry_field = serde_json::to_value(&manifest).unwrap();
    unknown_entry_field["parameters"][0]["checksum"] = serde_json::json!("forged");
    let error = serde_json::from_value::<ParameterManifest>(unknown_entry_field).unwrap_err();
    assert!(error.to_string().contains("unknown field `checksum`"));
}

#[test]
fn parameter_manifest_from_specs_json_accepts_array_and_document() {
    let specs = vec![
        vesty_params::ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
        vesty_params::ParamSpec::bool("bypass", "Bypass", false).as_bypass(),
    ];
    let array_json = serde_json::to_string(&specs).unwrap();
    let array_manifest = parameter_manifest_from_specs_json(&array_json).unwrap();

    let document_json = serde_json::to_string(&ParameterSpecDocument {
        version: 1,
        parameters: specs,
    })
    .unwrap();
    let document_manifest = parameter_manifest_from_specs_json(&document_json).unwrap();

    assert_eq!(array_manifest, document_manifest);
    assert_eq!(document_manifest.parameters.len(), 2);
    assert_eq!(
        document_manifest.parameters[1].vst3_param_id,
        stable_vst3_param_id("bypass")
    );
    assert!(document_manifest.parameters[1].spec.flags.bypass);
    assert!(!document_manifest.parameters[1].spec.flags.program_change);
}

#[test]
fn parameter_manifest_from_specs_json_rejects_invalid_documents() {
    let unsupported_version = r#"{"version":2,"parameters":[]}"#;
    let error = parameter_manifest_from_specs_json(unsupported_version).unwrap_err();
    assert!(matches!(error, BuildError::InvalidParameterSpecs(_)));
    assert!(error.to_string().contains("version 2"));

    let duplicate = serde_json::to_string(&vec![
        vesty_params::ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5),
        vesty_params::ParamSpec::float("gain", "Gain Copy", 0.0, 1.0, 0.5),
    ])
    .unwrap();
    let error = parameter_manifest_from_specs_json(&duplicate).unwrap_err();
    assert!(error.to_string().contains("duplicate parameter id"));
}

#[test]
fn packages_and_validates_parameter_manifest_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    fs::write(ui_dist.join("main.js"), "console.log('vesty')").unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let source_manifest = project.join("target/vesty-parameters.json");
    let manifest = ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", 0.0, 1.0, 0.5,
    )])
    .unwrap();
    fs::write(&source_manifest, serde_json_pretty(&manifest).unwrap()).unwrap();

    let mut config = test_config();
    config.package.as_mut().unwrap().parameter_manifest =
        Some("target/vesty-parameters.json".to_string());
    let report = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap();

    let packaged_path = report.parameter_manifest_path.as_ref().unwrap();
    assert_eq!(
        packaged_path.strip_prefix(&report.bundle_dir).unwrap(),
        Utf8Path::new("Contents/Resources/parameters.manifest.json")
    );
    let packaged = read_parameter_manifest(packaged_path).unwrap();
    assert_eq!(packaged, manifest);

    let validation = validate_vst3_bundle(&report.bundle_dir).unwrap();
    assert_eq!(
        validation.parameter_manifest_path,
        report.parameter_manifest_path
    );
}

#[test]
fn validation_rejects_tampered_parameter_manifest_host_ids() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    let manifest_path = report
        .bundle_dir
        .join("Contents/Resources/parameters.manifest.json");
    let mut manifest = ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", 0.0, 1.0, 0.5,
    )])
    .unwrap();
    manifest.parameters[0].vst3_param_id = manifest.parameters[0].vst3_param_id.wrapping_add(1);
    fs::write(&manifest_path, serde_json_pretty(&manifest).unwrap()).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidParameterManifest(_)));
    assert!(error.to_string().contains("vst3ParamId"));
}

#[test]
fn normalizes_class_id_to_uuid_form() {
    assert_eq!(
        normalize_class_id("56455354494741494E30303030303031").unwrap(),
        "56455354-4947-4149-4e30-303030303031"
    );
    assert_eq!(
        normalize_class_id("56455354-4947-4149-4E30-303030303031").unwrap(),
        "56455354-4947-4149-4e30-303030303031"
    );
    assert!(normalize_class_id("not-a-class-id").is_err());
}

#[test]
fn read_config_rejects_unknown_future_scope_fields_and_tables() {
    for (name, text, expected) in [
        (
            "top-level-bus",
            r#"[plugin]
name = "Gain"
vendor = "Vesty"
version = "0.1.0"
kind = "Fx"
class_id = "01234567-89ab-cdef-0123-456789abcdef"

[bus]
inputs = ["main"]
outputs = ["main"]

[package]
bundle_id = "dev.vesty.gain"
category = "Fx"
"#,
            "bus",
        ),
        (
            "ui-wayland",
            r#"[plugin]
name = "Gain"
vendor = "Vesty"
version = "0.1.0"
kind = "Fx"
class_id = "01234567-89ab-cdef-0123-456789abcdef"

[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420
experimental_wayland = true

[package]
bundle_id = "dev.vesty.gain"
category = "Fx"
"#,
            "experimental_wayland",
        ),
        (
            "package-installer",
            r#"[plugin]
name = "Gain"
vendor = "Vesty"
version = "0.1.0"
kind = "Fx"
class_id = "01234567-89ab-cdef-0123-456789abcdef"

[package]
bundle_id = "dev.vesty.gain"
category = "Fx"
installer = "pkg"
"#,
            "installer",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join(format!("{name}.toml"))).unwrap();
        fs::write(&path, text).unwrap();

        let error = read_config(&path).unwrap_err();

        assert!(matches!(error, BuildError::Toml(_)));
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?} in {error}"
        );
    }
}

#[test]
fn read_config_accepts_effect_sidechain_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("sidechain.toml")).unwrap();
    fs::write(
        &path,
        r#"[plugin]
name = "Gain"
vendor = "Vesty"
version = "0.1.0"
kind = "Fx"
class_id = "01234567-89ab-cdef-0123-456789abcdef"
sidechain = true

[package]
bundle_id = "dev.vesty.gain"
category = "Fx"
"#,
    )
    .unwrap();

    let config = read_config(&path).unwrap();

    assert_eq!(config.plugin.sidechain, Some(true));
}

#[test]
fn read_config_rejects_instrument_sidechain_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("instrument-sidechain.toml")).unwrap();
    fs::write(
        &path,
        r#"[plugin]
name = "Synth"
vendor = "Vesty"
version = "0.1.0"
kind = "Instrument"
class_id = "01234567-89ab-cdef-0123-456789abcdef"
sidechain = true
"#,
    )
    .unwrap();

    let error = read_config(&path).unwrap_err();

    assert!(matches!(error, BuildError::InvalidConfig(_)));
    assert!(error.to_string().contains("[plugin].sidechain"));
}

#[test]
fn read_config_accepts_current_schema_for_examples() {
    let workspace_root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for path in [
        workspace_root.join("examples/gain/vesty.toml"),
        workspace_root.join("examples/midi-synth/vesty.toml"),
        workspace_root.join("examples/web-ui-param-demo/vesty.toml"),
    ] {
        read_config(&path).unwrap_or_else(|error| panic!("failed to parse {path}: {error}"));
    }
}

#[test]
fn package_rejects_invalid_config_class_id() {
    let mut config = test_config();
    config.plugin.class_id = "not-a-class-id".to_string();

    let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

    assert!(matches!(error, BuildError::InvalidConfig(_)));
}

#[test]
fn package_rejects_unsupported_plugin_kind() {
    let mut config = test_config();
    config.plugin.kind = "surround-generator".to_string();

    let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

    assert!(matches!(error, BuildError::InvalidConfig(_)));
    assert!(error.to_string().contains("[plugin].kind"));
}

#[test]
fn package_rejects_empty_plugin_metadata() {
    for field in ["name", "vendor", "version", "kind"] {
        let mut config = test_config();
        match field {
            "name" => config.plugin.name = " ".to_string(),
            "vendor" => config.plugin.vendor = " ".to_string(),
            "version" => config.plugin.version = " ".to_string(),
            "kind" => config.plugin.kind = " ".to_string(),
            _ => unreachable!(),
        }

        let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

        assert!(matches!(error, BuildError::InvalidConfig(_)));
        assert!(error.to_string().contains(field));
    }
}

#[test]
fn package_rejects_control_chars_in_config_metadata() {
    for field in [
        "name",
        "vendor",
        "version",
        "category",
        "signing",
        "parameter_manifest",
    ] {
        let mut config = test_config();
        match field {
            "name" => config.plugin.name = "Gain\nBad".to_string(),
            "vendor" => config.plugin.vendor = "Vesty\nBad".to_string(),
            "version" => config.plugin.version = "0.1.0\nBad".to_string(),
            "category" => {
                config.package.as_mut().unwrap().category = Some("Fx\nAnalyzer".to_string());
            }
            "signing" => {
                config.package.as_mut().unwrap().signing = Some("Developer\nID".to_string());
            }
            "parameter_manifest" => {
                config.package.as_mut().unwrap().parameter_manifest =
                    Some("target\nparams.json".to_string());
            }
            _ => unreachable!(),
        }

        let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

        assert!(matches!(error, BuildError::InvalidConfig(_)));
        assert!(
            error.to_string().contains("control characters"),
            "expected control character error for {field}: {error}"
        );
    }
}

#[test]
fn package_rejects_empty_bundle_id_when_present() {
    let mut config = test_config();
    config.package.as_mut().unwrap().bundle_id = Some(" ".to_string());

    let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

    assert!(matches!(error, BuildError::InvalidConfig(_)));
}

#[test]
fn package_rejects_invalid_bundle_id_when_present() {
    for bundle_id in [
        "com example.plugin",
        "com/example/plugin",
        "com..example",
        "com.example.",
        "com.example_plugin.demo",
        "com.-example.demo",
    ] {
        let mut config = test_config();
        config.package.as_mut().unwrap().bundle_id = Some(bundle_id.to_string());

        let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

        assert!(matches!(error, BuildError::InvalidConfig(_)));
        assert!(
            error.to_string().contains("[package].bundle_id"),
            "expected bundle id error for {bundle_id}: {error}"
        );
    }
}

#[test]
fn macos_plist_fallback_bundle_id_uses_valid_identifier_shape() {
    let mut config = test_config();
    config.plugin.name = "My_Plugin".to_string();
    config.package = None;
    let (_dir, report) = package_fixture_with_config(BundlePlatform::Macos, config);

    let plist = plist::Value::from_file(report.bundle_dir.join("Contents/Info.plist"))
        .unwrap()
        .into_dictionary()
        .unwrap();

    assert_eq!(
        plist
            .get("CFBundleIdentifier")
            .and_then(plist::Value::as_string),
        Some("dev.vesty.my-plugin")
    );
}

#[test]
fn package_rejects_empty_signing_identity_when_present() {
    let mut config = test_config();
    config.package.as_mut().unwrap().signing = Some(" ".to_string());

    let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

    assert!(matches!(error, BuildError::InvalidConfig(_)));
    assert!(error.to_string().contains("[package].signing"));
}

#[test]
fn package_rejects_invalid_ui_config() {
    fn assert_invalid_ui_config(config: VestyConfig, expected: &str) {
        let error = package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

        assert!(matches!(error, BuildError::InvalidConfig(_)));
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?} in {error}"
        );
    }

    let mut config = test_config();
    config.ui.as_mut().unwrap().dir = " ".to_string();
    assert_invalid_ui_config(config, "[ui].dir");

    let mut config = test_config();
    config.ui.as_mut().unwrap().dev_url = Some(" ".to_string());
    assert_invalid_ui_config(config, "[ui].dev_url");

    let mut config = test_config();
    config.ui.as_mut().unwrap().build = Some(" ".to_string());
    assert_invalid_ui_config(config, "[ui].build");

    let mut config = test_config();
    config.ui.as_mut().unwrap().dist = Some(" ".to_string());
    assert_invalid_ui_config(config, "[ui].dist");

    let mut config = test_config();
    config.ui.as_mut().unwrap().width = Some(900);
    assert_invalid_ui_config(config, "[ui].height");

    let mut config = test_config();
    config.ui.as_mut().unwrap().height = Some(560);
    assert_invalid_ui_config(config, "[ui].width");

    let mut config = test_config();
    config.ui.as_mut().unwrap().width = Some(0);
    config.ui.as_mut().unwrap().height = Some(560);
    assert_invalid_ui_config(config, "greater than 0");

    let mut config = test_config();
    config.ui.as_mut().unwrap().width = Some(900);
    config.ui.as_mut().unwrap().height = Some(560);
    config.ui.as_mut().unwrap().min_width = Some(901);
    config.ui.as_mut().unwrap().min_height = Some(420);
    assert_invalid_ui_config(config, "[ui].min_width must be <=");

    let mut config = test_config();
    config.ui.as_mut().unwrap().width = Some(900);
    config.ui.as_mut().unwrap().height = Some(560);
    config.ui.as_mut().unwrap().min_width = Some(640);
    config.ui.as_mut().unwrap().min_height = Some(561);
    assert_invalid_ui_config(config, "[ui].min_height must be <=");
}

#[test]
fn validates_packaged_vst3_bundle() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);

    let validation = validate_vst3_bundle(&report.bundle_dir).unwrap();
    assert_eq!(validation.bundle_dir, report.bundle_dir);
    assert_eq!(validation.moduleinfo_path, report.moduleinfo_path);
    assert_eq!(validation.binary_paths, vec![report.binary_path]);
    assert_eq!(validation.asset_manifest_path, report.asset_manifest_path);
    assert_eq!(validation.asset_count, 2);
}

#[test]
fn macos_validation_rejects_tampered_executable_plist() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    write_test_info_plist(&report, "BNDL", "MissingBinary", "dev.vesty.gain");

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::MissingFile(_)));
}

#[test]
fn macos_validation_rejects_executable_plist_mismatch_with_moduleinfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    let extra_binary = report.bundle_dir.join("Contents/MacOS/Other");
    fs::copy(&report.binary_path, &extra_binary).unwrap();
    write_test_info_plist(&report, "BNDL", "Other", "dev.vesty.gain");

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("CFBundleExecutable"));
    assert!(error.to_string().contains("moduleinfo binary name"));
}

#[test]
fn macos_validation_rejects_bundle_name_plist_mismatch_with_moduleinfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    write_test_info_plist_with_name(&report, "BNDL", "Gain", "dev.vesty.gain", "Other");

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("CFBundleName"));
    assert!(error.to_string().contains("moduleinfo name"));
}

#[test]
fn macos_validation_rejects_version_plist_mismatch_with_moduleinfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    write_test_info_plist_with_metadata(
        &report,
        TestInfoPlist {
            package_type: "BNDL",
            executable: "Gain",
            bundle_id: "dev.vesty.gain",
            bundle_name: "Gain",
            short_version: "9.9.9",
            bundle_version: "0.1.0",
        },
    );

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("CFBundleShortVersionString"));

    write_test_info_plist_with_metadata(
        &report,
        TestInfoPlist {
            package_type: "BNDL",
            executable: "Gain",
            bundle_id: "dev.vesty.gain",
            bundle_name: "Gain",
            short_version: "0.1.0",
            bundle_version: "9.9.9",
        },
    );

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("CFBundleVersion"));
}

#[test]
fn macos_validation_rejects_invalid_package_type_plist() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    write_test_info_plist(&report, "APPL", "Gain", "dev.vesty.gain");

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
}

#[test]
fn macos_validation_rejects_invalid_bundle_identifier_plist() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    write_test_info_plist(&report, "BNDL", "Gain", "dev_vesty_gain");

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("CFBundleIdentifier"));
}

#[test]
fn macos_validation_rejects_bad_pkginfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    fs::write(report.bundle_dir.join("Contents/PkgInfo"), "APPL????").unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
}

#[test]
fn macos_validation_rejects_missing_pkginfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    fs::remove_file(report.bundle_dir.join("Contents/PkgInfo")).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::MissingFile(_)));
}

#[test]
fn validation_rejects_tampered_asset_manifest_entries() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    fs::write(
        report.bundle_dir.join("Contents/Resources/ui/main.js"),
        "console.log('tampered')",
    )
    .unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();
    assert!(matches!(error, BuildError::AssetIntegrity(_)));
}

#[test]
fn binary_export_output_parsing_accepts_required_platform_symbols() {
    let path = Utf8Path::new("/tmp/VestyGain.vst3/Contents/MacOS/VestyGain");

    let macos = binary_export_check_from_output(
        path,
        BundlePlatform::Macos,
        "nm -gU".to_string(),
        "0000000000000000 T _GetPluginFactory\n\
         0000000000000000 T _bundleEntry\n\
         0000000000000000 T _bundleExit\n\
         0000000000000000 T _BundleEntry\n\
         0000000000000000 T _BundleExit\n",
    );

    assert_eq!(macos.status, "ok");
    assert!(macos.missing_symbols.is_empty());
    assert_eq!(macos.required_symbols.len(), 5);

    let linux = binary_export_check_from_output(
        Utf8Path::new("/tmp/VestyGain.vst3/Contents/x86_64-linux/VestyGain.so"),
        BundlePlatform::LinuxX64,
        "nm -D --defined-only".to_string(),
        "0000000000000000 T GetPluginFactory\n\
         0000000000000000 T ModuleEntry\n\
         0000000000000000 T ModuleExit\n",
    );

    assert_eq!(linux.status, "ok");
    assert!(linux.found_symbols.contains(&"ModuleEntry".to_string()));
}

#[test]
fn binary_export_output_parsing_reports_missing_required_symbols() {
    let check = binary_export_check_from_output(
        Utf8Path::new("/tmp/VestyGain.vst3/Contents/x86_64-win/VestyGain.vst3"),
        BundlePlatform::WindowsX64,
        "llvm-objdump -p".to_string(),
        "Export Table:\nGetPluginFactory\n",
    );

    assert_eq!(check.status, "failed");
    assert_eq!(
        check.missing_symbols,
        vec!["InitDll".to_string(), "ExitDll".to_string()]
    );
}

#[test]
fn binary_export_validation_uses_vst3_sys_required_symbol_plan() {
    for (platform, slug, path) in [
        (
            BundlePlatform::Macos,
            "macos",
            "/tmp/VestyGain.vst3/Contents/MacOS/VestyGain",
        ),
        (
            BundlePlatform::WindowsX64,
            "windows-x64",
            "/tmp/VestyGain.vst3/Contents/x86_64-win/VestyGain.vst3",
        ),
        (
            BundlePlatform::LinuxX64,
            "linux-x64",
            "/tmp/VestyGain.vst3/Contents/x86_64-linux/VestyGain.so",
        ),
    ] {
        let expected = vesty_vst3_sys::required_binary_export_tool_symbols(slug).unwrap();
        assert_eq!(required_export_symbols(platform), expected);

        let output = expected
            .iter()
            .map(|symbol| format!("0000000000000000 T {symbol}\n"))
            .collect::<String>();
        let check = binary_export_check_from_output(
            Utf8Path::new(path),
            platform,
            "test-symbol-tool".to_string(),
            &output,
        );

        assert_eq!(check.status, "ok");
        assert_eq!(
            check.required_symbols,
            expected
                .iter()
                .map(|symbol| (*symbol).to_string())
                .collect::<Vec<_>>()
        );
        assert!(check.missing_symbols.is_empty());
    }
}

#[test]
fn binary_export_validation_uses_vst3_sys_inspection_tool_plan() {
    for (platform, slug) in [
        (BundlePlatform::Macos, "macos"),
        (BundlePlatform::WindowsX64, "windows-x64"),
        (BundlePlatform::LinuxX64, "linux-x64"),
    ] {
        let expected = vesty_vst3_sys::binary_export_inspection_tools(slug).unwrap();
        let actual = export_symbol_tools(platform);

        assert_eq!(actual, expected);
        assert_eq!(
            actual
                .iter()
                .map(vesty_vst3_sys::BinaryExportInspectionToolPlan::display)
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(vesty_vst3_sys::BinaryExportInspectionToolPlan::display)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn validation_rejects_malformed_asset_manifest_metadata() {
    let mutators: [fn(&mut AssetManifest); 13] = [
        |manifest: &mut AssetManifest| {
            manifest.version = 2;
        },
        |manifest: &mut AssetManifest| {
            manifest.root.clear();
        },
        |manifest: &mut AssetManifest| {
            manifest.root = "ui\nroot".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files.push(manifest.files[0].clone());
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].mime.clear();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].mime = "text/html\nx-bad: 1".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].sha256 = "not-a-sha".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.entry = "../index.html".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].path = "../index.html".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].path = "index.html?cache=1".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].path = "index.html#fragment".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].path = "C:index.html".to_string();
        },
        |manifest: &mut AssetManifest| {
            manifest.files[0].path = "assets/%2e%2e/index.html".to_string();
        },
    ];
    for mutate in mutators {
        let (_dir, report) = package_fixture(BundlePlatform::Macos);
        let manifest_path = report.asset_manifest_path.as_ref().unwrap();
        let mut manifest =
            serde_json::from_str::<AssetManifest>(&fs::read_to_string(manifest_path).unwrap())
                .unwrap();
        mutate(&mut manifest);
        fs::write(manifest_path, serde_json_pretty(&manifest).unwrap()).unwrap();

        let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();
        assert!(matches!(
            error,
            BuildError::InvalidBundle(_)
                | BuildError::AssetIntegrity(_)
                | BuildError::PathEscapesRoot(_)
        ));
    }
}

#[test]
fn validation_rejects_bundle_without_platform_binary() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    fs::remove_file(&report.binary_path).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();
    assert!(matches!(error, BuildError::InvalidBundle(_)));
}

#[test]
fn validation_rejects_invalid_moduleinfo_class_id() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    let mut moduleinfo =
        serde_json::from_str::<ModuleInfo>(&fs::read_to_string(&report.moduleinfo_path).unwrap())
            .unwrap();
    moduleinfo.classes[0].cid = "not-a-class-id".to_string();
    fs::write(
        &report.moduleinfo_path,
        serde_json_pretty(&moduleinfo).unwrap(),
    )
    .unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
}

#[test]
fn validation_rejects_empty_moduleinfo_metadata_fields() {
    for field in ["name", "vendor", "plugin_version"] {
        let (_dir, report) = package_fixture(BundlePlatform::Macos);
        let mut moduleinfo = serde_json::from_str::<ModuleInfo>(
            &fs::read_to_string(&report.moduleinfo_path).unwrap(),
        )
        .unwrap();
        match field {
            "name" => moduleinfo.name = " ".to_string(),
            "vendor" => moduleinfo.vendor = " ".to_string(),
            "plugin_version" => moduleinfo.plugin_version = " ".to_string(),
            _ => unreachable!(),
        }
        fs::write(
            &report.moduleinfo_path,
            serde_json_pretty(&moduleinfo).unwrap(),
        )
        .unwrap();

        let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

        assert!(
            error.to_string().contains(field),
            "expected field {field} in error: {error}"
        );
    }
}

#[test]
fn validation_rejects_control_chars_in_moduleinfo_metadata_fields() {
    for field in ["name", "vendor", "plugin_version", "class_name", "category"] {
        let (_dir, report) = package_fixture(BundlePlatform::Macos);
        let mut moduleinfo = serde_json::from_str::<ModuleInfo>(
            &fs::read_to_string(&report.moduleinfo_path).unwrap(),
        )
        .unwrap();
        match field {
            "name" => moduleinfo.name = "Gain\nBad".to_string(),
            "vendor" => moduleinfo.vendor = "Vesty\nBad".to_string(),
            "plugin_version" => moduleinfo.plugin_version = "0.1.0\nBad".to_string(),
            "class_name" => moduleinfo.classes[0].name = "Gain\nBad".to_string(),
            "category" => moduleinfo.classes[0].category = "Fx\nBad".to_string(),
            _ => unreachable!(),
        }
        fs::write(
            &report.moduleinfo_path,
            serde_json_pretty(&moduleinfo).unwrap(),
        )
        .unwrap();

        let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

        assert!(matches!(error, BuildError::InvalidBundle(_)));
        assert!(
            error.to_string().contains("control characters"),
            "expected control character error for {field}: {error}"
        );
    }
}

#[test]
fn validation_rejects_empty_moduleinfo_class_fields() {
    for field in ["name", "category"] {
        let (_dir, report) = package_fixture(BundlePlatform::Macos);
        let mut moduleinfo = serde_json::from_str::<ModuleInfo>(
            &fs::read_to_string(&report.moduleinfo_path).unwrap(),
        )
        .unwrap();
        match field {
            "name" => moduleinfo.classes[0].name = " ".to_string(),
            "category" => moduleinfo.classes[0].category = " ".to_string(),
            _ => unreachable!(),
        }
        fs::write(
            &report.moduleinfo_path,
            serde_json_pretty(&moduleinfo).unwrap(),
        )
        .unwrap();

        let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

        assert!(
            error.to_string().contains(field),
            "expected field {field} in error: {error}"
        );
    }
}

#[test]
fn validation_rejects_macos_binary_name_mismatched_with_moduleinfo() {
    let (_dir, report) = package_fixture(BundlePlatform::Macos);
    let mut moduleinfo =
        serde_json::from_str::<ModuleInfo>(&fs::read_to_string(&report.moduleinfo_path).unwrap())
            .unwrap();
    moduleinfo.name = "Other".to_string();
    fs::write(
        &report.moduleinfo_path,
        serde_json_pretty(&moduleinfo).unwrap(),
    )
    .unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("missing expected macOS binary"));
}

#[test]
fn validation_rejects_misnamed_windows_binary() {
    let (_dir, report) = package_fixture(BundlePlatform::WindowsX64);
    let wrong_binary = report.bundle_dir.join("Contents/x86_64-win/WrongName.vst3");
    fs::rename(&report.binary_path, wrong_binary).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(
        error
            .to_string()
            .contains("missing expected Windows x64 binary")
    );
}

#[test]
fn validation_rejects_misnamed_linux_binary() {
    let (_dir, report) = package_fixture(BundlePlatform::LinuxX64);
    let wrong_binary = report.bundle_dir.join("Contents/x86_64-linux/WrongName.so");
    fs::rename(&report.binary_path, wrong_binary).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(
        error
            .to_string()
            .contains("missing expected Linux x64 binary")
    );
}

#[test]
fn package_rejects_binary_format_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let error = package_vst3(
        &test_config(),
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::WindowsX64,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("Windows x64 binary"));
    assert!(error.to_string().contains("unexpected file format"));
}

#[test]
fn validation_rejects_wrong_platform_binary_format() {
    let (_dir, report) = package_fixture(BundlePlatform::LinuxX64);
    fs::write(
        &report.binary_path,
        test_binary_bytes(BundlePlatform::WindowsX64),
    )
    .unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("Linux x64 binary"));
    assert!(error.to_string().contains("unexpected file format"));
}

#[test]
fn validation_rejects_non_x64_windows_binary() {
    let (_dir, report) = package_fixture(BundlePlatform::WindowsX64);
    let mut bytes = test_binary_bytes(BundlePlatform::WindowsX64);
    bytes[0x44..0x46].copy_from_slice(&(0xaa64u16).to_le_bytes());
    fs::write(&report.binary_path, bytes).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("Windows x64 binary"));
    assert!(error.to_string().contains("unexpected file format"));
}

#[test]
fn validation_rejects_non_x64_linux_binary() {
    let (_dir, report) = package_fixture(BundlePlatform::LinuxX64);
    let mut bytes = test_binary_bytes(BundlePlatform::LinuxX64);
    bytes[18..20].copy_from_slice(&(0xb7u16).to_le_bytes());
    fs::write(&report.binary_path, bytes).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::InvalidBundle(_)));
    assert!(error.to_string().contains("Linux x64 binary"));
    assert!(error.to_string().contains("unexpected file format"));
}

fn package_fixture(platform: BundlePlatform) -> (tempfile::TempDir, PackageReport) {
    package_fixture_with_config(platform, test_config())
}

fn package_fixture_with_config(
    platform: BundlePlatform,
    config: VestyConfig,
) -> (tempfile::TempDir, PackageReport) {
    let (dir, report) = package_fixture_with_config_result(platform, config).unwrap();
    (dir, report)
}

fn package_fixture_with_config_result(
    platform: BundlePlatform,
    config: VestyConfig,
) -> Result<(tempfile::TempDir, PackageReport), BuildError> {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    fs::write(ui_dist.join("main.js"), "console.log('vesty')").unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(platform)).unwrap();

    let report = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform,
            binary_path: binary,
        },
    )?;

    Ok((dir, report))
}

fn test_binary_bytes(platform: BundlePlatform) -> Vec<u8> {
    match platform {
        BundlePlatform::Macos => {
            let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
            bytes.extend_from_slice(b"vesty-test-binary");
            bytes
        }
        BundlePlatform::WindowsX64 => {
            let mut bytes = vec![0; 0x80];
            bytes[0..2].copy_from_slice(b"MZ");
            bytes[0x3c..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
            bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
            bytes[0x44..0x46].copy_from_slice(&(0x8664u16).to_le_bytes());
            bytes.extend_from_slice(b"vesty-test-binary");
            bytes
        }
        BundlePlatform::LinuxX64 => {
            let mut bytes = vec![0; 20];
            bytes[0..4].copy_from_slice(b"\x7fELF");
            bytes[4] = 2;
            bytes[5] = 1;
            bytes[18..20].copy_from_slice(&(0x3eu16).to_le_bytes());
            bytes
        }
    }
}

fn test_config() -> VestyConfig {
    VestyConfig {
        plugin: PluginConfig {
            name: "Gain".to_string(),
            vendor: "Vesty".to_string(),
            version: "0.1.0".to_string(),
            kind: "Fx".to_string(),
            class_id: "01234567-89ab-cdef-0123-456789abcdef".to_string(),
            sidechain: None,
        },
        ui: Some(UiConfig {
            dir: "ui".to_string(),
            dev_url: None,
            build: None,
            dist: Some("dist".to_string()),
            width: None,
            height: None,
            min_width: None,
            min_height: None,
        }),
        package: Some(PackageConfig {
            bundle_id: Some("dev.vesty.gain".to_string()),
            category: Some("Fx".to_string()),
            signing: None,
            parameter_manifest: None,
        }),
    }
}

fn assert_common_bundle_files(report: &PackageReport, binary_relative: &str) {
    assert!(report.binary_path.is_file());
    assert_eq!(
        report.binary_path.strip_prefix(&report.bundle_dir).unwrap(),
        Utf8Path::new(binary_relative)
    );
    assert!(report.moduleinfo_path.is_file());
    assert_eq!(
        report
            .moduleinfo_path
            .strip_prefix(&report.bundle_dir)
            .unwrap(),
        Utf8Path::new("Contents/Resources/moduleinfo.json")
    );
    assert!(
        report
            .bundle_dir
            .join("Contents/Resources/ui/index.html")
            .is_file()
    );
    assert!(
        report
            .bundle_dir
            .join("Contents/Resources/ui/main.js")
            .is_file()
    );
    assert!(report.asset_manifest_path.as_ref().unwrap().is_file());
    assert_eq!(report.copied_assets, 2);
}

fn write_test_info_plist(
    report: &PackageReport,
    package_type: &str,
    executable: &str,
    bundle_id: &str,
) {
    write_test_info_plist_with_name(report, package_type, executable, bundle_id, "Gain");
}

fn write_test_info_plist_with_name(
    report: &PackageReport,
    package_type: &str,
    executable: &str,
    bundle_id: &str,
    bundle_name: &str,
) {
    write_test_info_plist_with_metadata(
        report,
        TestInfoPlist {
            package_type,
            executable,
            bundle_id,
            bundle_name,
            short_version: "0.1.0",
            bundle_version: "0.1.0",
        },
    );
}

struct TestInfoPlist<'a> {
    package_type: &'a str,
    executable: &'a str,
    bundle_id: &'a str,
    bundle_name: &'a str,
    short_version: &'a str,
    bundle_version: &'a str,
}

fn write_test_info_plist_with_metadata(report: &PackageReport, metadata: TestInfoPlist<'_>) {
    let mut dict = plist::Dictionary::new();
    dict.insert("CFBundleExecutable".into(), metadata.executable.into());
    dict.insert("CFBundleIdentifier".into(), metadata.bundle_id.into());
    dict.insert("CFBundleName".into(), metadata.bundle_name.into());
    dict.insert("CFBundlePackageType".into(), metadata.package_type.into());
    dict.insert(
        "CFBundleShortVersionString".into(),
        metadata.short_version.into(),
    );
    dict.insert("CFBundleVersion".into(), metadata.bundle_version.into());
    plist::to_file_xml(report.bundle_dir.join("Contents/Info.plist"), &dict).unwrap();
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_ui_assets() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    std::os::unix::fs::symlink(ui_dist.join("index.html"), ui_dist.join("link.html")).unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let config = VestyConfig {
        plugin: PluginConfig {
            name: "Gain".to_string(),
            vendor: "Vesty".to_string(),
            version: "0.1.0".to_string(),
            kind: "Fx".to_string(),
            class_id: "01234567-89ab-cdef-0123-456789abcdef".to_string(),
            sidechain: None,
        },
        ui: Some(UiConfig {
            dir: "ui".to_string(),
            dev_url: None,
            build: None,
            dist: Some("dist".to_string()),
            width: None,
            height: None,
            min_width: None,
            min_height: None,
        }),
        package: None,
    };

    let error = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();
    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_ui_dist_root() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let external_dist = Utf8PathBuf::from_path_buf(dir.path().join("external-dist")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(ui_dist.parent().unwrap()).unwrap();
    fs::create_dir(&external_dist).unwrap();
    fs::write(external_dist.join("index.html"), "<main></main>").unwrap();
    std::os::unix::fs::symlink(&external_dist, &ui_dist).unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let error = package_vst3(
        &test_config(),
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_output_dir() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let external_out = Utf8PathBuf::from_path_buf(dir.path().join("external-out")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    fs::write(ui_dist.join("main.js"), "console.log('vesty')").unwrap();
    fs::create_dir(&external_out).unwrap();
    std::os::unix::fs::symlink(&external_out, &out).unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let error = package_vst3(
        &test_config(),
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
    assert!(!external_out.join("Gain.vst3").exists());
}

#[cfg(unix)]
#[test]
fn package_rejects_existing_symlinked_ui_output_dir() {
    let (dir, report) = package_fixture(BundlePlatform::Macos);
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let external_ui = Utf8PathBuf::from_path_buf(dir.path().join("external-ui")).unwrap();
    let ui_dest = report.bundle_dir.join("Contents/Resources/ui");
    fs::remove_dir_all(&ui_dest).unwrap();
    fs::create_dir(&external_ui).unwrap();
    fs::write(external_ui.join("keep.txt"), "do not remove\n").unwrap();
    std::os::unix::fs::symlink(&external_ui, &ui_dest).unwrap();

    let binary = project.join("target/release/Gain");
    let error = package_vst3(
        &test_config(),
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
    assert_eq!(
        fs::read_to_string(external_ui.join("keep.txt")).unwrap(),
        "do not remove\n"
    );
}

#[cfg(unix)]
#[test]
fn package_rejects_existing_symlinked_output_files() {
    for (label, relative_path) in [
        ("binary", "Contents/MacOS/Gain"),
        ("info-plist", "Contents/Info.plist"),
        ("pkg-info", "Contents/PkgInfo"),
        ("moduleinfo", "Contents/Resources/moduleinfo.json"),
        ("asset-manifest", "Contents/Resources/assets.manifest.json"),
    ] {
        let (dir, report) = package_fixture(BundlePlatform::Macos);
        let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
        let external =
            Utf8PathBuf::from_path_buf(dir.path().join(format!("external-{label}.txt"))).unwrap();
        let target = report.bundle_dir.join(relative_path);
        fs::write(&external, "do not overwrite\n").unwrap();
        fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(&external, &target).unwrap();

        let binary = project.join("target/release/Gain");
        let error = package_vst3(
            &test_config(),
            &PackageOptions {
                project_dir: project,
                output_dir: out,
                platform: BundlePlatform::Macos,
                binary_path: binary,
            },
        )
        .unwrap_err();

        assert!(
            matches!(error, BuildError::SymlinkAsset(_)),
            "expected symlink rejection for {relative_path}, got {error}"
        );
        assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
    }
}

#[cfg(unix)]
#[test]
fn package_rejects_existing_symlinked_parameter_manifest_output() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();
    fs::write(ui_dist.join("main.js"), "console.log('vesty')").unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let source_manifest = project.join("target/vesty-parameters.json");
    let manifest = ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", 0.0, 1.0, 0.5,
    )])
    .unwrap();
    fs::create_dir_all(source_manifest.parent().unwrap()).unwrap();
    fs::write(&source_manifest, serde_json_pretty(&manifest).unwrap()).unwrap();

    let mut config = test_config();
    config.package.as_mut().unwrap().parameter_manifest =
        Some("target/vesty-parameters.json".to_string());
    let report = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project.clone(),
            output_dir: out.clone(),
            platform: BundlePlatform::Macos,
            binary_path: binary.clone(),
        },
    )
    .unwrap();

    let external = Utf8PathBuf::from_path_buf(dir.path().join("external-params.json")).unwrap();
    let target = report.parameter_manifest_path.as_ref().unwrap();
    fs::write(&external, "do not overwrite\n").unwrap();
    fs::remove_file(target).unwrap();
    std::os::unix::fs::symlink(&external, target).unwrap();

    let error = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
    assert_eq!(fs::read_to_string(&external).unwrap(), "do not overwrite\n");
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_ui_asset_manifest_file() {
    let (dir, report) =
        package_fixture_with_config_result(BundlePlatform::Macos, test_config()).unwrap();
    let resources_dir = report.bundle_dir.join("Contents/Resources");
    let manifest_path = resources_dir.join("assets.manifest.json");
    let external_manifest =
        Utf8PathBuf::from_path_buf(dir.path().join("external-assets.manifest.json")).unwrap();
    fs::rename(&manifest_path, &external_manifest).unwrap();
    std::os::unix::fs::symlink(&external_manifest, &manifest_path).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_ui_asset_root() {
    let (dir, report) =
        package_fixture_with_config_result(BundlePlatform::Macos, test_config()).unwrap();
    let ui_dir = report.bundle_dir.join("Contents/Resources/ui");
    let external_ui = Utf8PathBuf::from_path_buf(dir.path().join("external-ui")).unwrap();
    fs::rename(&ui_dir, &external_ui).unwrap();
    std::os::unix::fs::symlink(&external_ui, &ui_dir).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn read_config_rejects_symlinked_file() {
    let dir = tempfile::tempdir().unwrap();
    let external_config =
        Utf8PathBuf::from_path_buf(dir.path().join("external-vesty.toml")).unwrap();
    let config_path = Utf8PathBuf::from_path_buf(dir.path().join("vesty.toml")).unwrap();
    fs::write(
        &external_config,
        r#"[plugin]
name = "Gain"
vendor = "Vesty"
version = "0.1.0"
kind = "Fx"
class_id = "01234567-89ab-cdef-0123-456789abcdef"
"#,
    )
    .unwrap();
    std::os::unix::fs::symlink(&external_config, &config_path).unwrap();

    let error = read_config(&config_path).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn read_parameter_specs_rejects_symlinked_file() {
    let dir = tempfile::tempdir().unwrap();
    let external_specs =
        Utf8PathBuf::from_path_buf(dir.path().join("external-params.specs.json")).unwrap();
    let specs_path = Utf8PathBuf::from_path_buf(dir.path().join("params.specs.json")).unwrap();
    let specs = ParameterSpecDocument::new(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", 0.0, 1.0, 0.5,
    )])
    .unwrap();
    fs::write(&external_specs, serde_json_pretty(&specs).unwrap()).unwrap();
    std::os::unix::fs::symlink(&external_specs, &specs_path).unwrap();

    let error = read_parameter_specs(&specs_path).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn read_parameter_manifest_rejects_symlinked_file() {
    let dir = tempfile::tempdir().unwrap();
    let external_manifest =
        Utf8PathBuf::from_path_buf(dir.path().join("external-parameters.manifest.json")).unwrap();
    let manifest_path =
        Utf8PathBuf::from_path_buf(dir.path().join("parameters.manifest.json")).unwrap();
    fs::write(
        &external_manifest,
        serde_json_pretty(&test_parameter_manifest()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink(&external_manifest, &manifest_path).unwrap();

    let error = read_parameter_manifest(&manifest_path).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_binary_input() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let external_binary = Utf8PathBuf::from_path_buf(dir.path().join("external-Gain")).unwrap();
    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&external_binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();
    std::os::unix::fs::symlink(&external_binary, &binary).unwrap();

    let error = package_vst3(
        &test_config(),
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_configured_parameter_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
    let out = Utf8PathBuf::from_path_buf(dir.path().join("out")).unwrap();
    let ui_dist = project.join("ui/dist");
    fs::create_dir_all(&ui_dist).unwrap();
    fs::write(ui_dist.join("index.html"), "<main></main>").unwrap();

    let binary = project.join("target/release/Gain");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, test_binary_bytes(BundlePlatform::Macos)).unwrap();

    let manifest_path = project.join("target/vesty-parameters.json");
    let external_manifest =
        Utf8PathBuf::from_path_buf(dir.path().join("external-vesty-parameters.json")).unwrap();
    fs::write(
        &external_manifest,
        serde_json_pretty(&test_parameter_manifest()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink(&external_manifest, &manifest_path).unwrap();

    let mut config = test_config();
    config.package.as_mut().unwrap().parameter_manifest =
        Some("target/vesty-parameters.json".to_string());

    let error = package_vst3(
        &config,
        &PackageOptions {
            project_dir: project,
            output_dir: out,
            platform: BundlePlatform::Macos,
            binary_path: binary,
        },
    )
    .unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_bundle_root() {
    let (dir, report) = package_fixture(BundlePlatform::Macos);
    let bundle_link = Utf8PathBuf::from_path_buf(dir.path().join("GainLink.vst3")).unwrap();
    std::os::unix::fs::symlink(&report.bundle_dir, &bundle_link).unwrap();

    let error = validate_vst3_bundle(&bundle_link).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_moduleinfo_file() {
    let (dir, report) = package_fixture(BundlePlatform::Macos);
    let external_moduleinfo =
        Utf8PathBuf::from_path_buf(dir.path().join("external-moduleinfo.json")).unwrap();
    fs::rename(&report.moduleinfo_path, &external_moduleinfo).unwrap();
    std::os::unix::fs::symlink(&external_moduleinfo, &report.moduleinfo_path).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_packaged_parameter_manifest() {
    let (dir, report) = package_fixture(BundlePlatform::Macos);
    let manifest_path = report
        .bundle_dir
        .join("Contents/Resources")
        .join(PARAMETER_MANIFEST_FILE);
    let external_manifest =
        Utf8PathBuf::from_path_buf(dir.path().join("external-parameters.manifest.json")).unwrap();
    fs::write(
        &external_manifest,
        serde_json_pretty(&test_parameter_manifest()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink(&external_manifest, &manifest_path).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_macos_metadata_files() {
    for file_name in ["Info.plist", "PkgInfo"] {
        let (dir, report) = package_fixture(BundlePlatform::Macos);
        let path = report.bundle_dir.join("Contents").join(file_name);
        let external = Utf8PathBuf::from_path_buf(
            dir.path()
                .join(format!("external-{}", file_name.replace('.', "-"))),
        )
        .unwrap();
        fs::rename(&path, &external).unwrap();
        std::os::unix::fs::symlink(&external, &path).unwrap();

        let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

        assert!(
            matches!(error, BuildError::SymlinkAsset(_)),
            "expected symlink rejection for {file_name}, got {error}"
        );
    }
}

#[cfg(unix)]
#[test]
fn validate_rejects_symlinked_platform_binary_dir() {
    let (dir, report) = package_fixture(BundlePlatform::Macos);
    let macos_dir = report.bundle_dir.join("Contents/MacOS");
    let external_macos = Utf8PathBuf::from_path_buf(dir.path().join("external-MacOS")).unwrap();
    fs::rename(&macos_dir, &external_macos).unwrap();
    std::os::unix::fs::symlink(&external_macos, &macos_dir).unwrap();

    let error = validate_vst3_bundle(&report.bundle_dir).unwrap_err();

    assert!(matches!(error, BuildError::SymlinkAsset(_)));
}

#[cfg(unix)]
fn test_parameter_manifest() -> ParameterManifest {
    ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
        "gain", "Gain", 0.0, 1.0, 0.5,
    )])
    .unwrap()
}
