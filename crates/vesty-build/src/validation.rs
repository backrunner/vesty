use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

use crate::{
    AssetManifest, BuildError, BundlePlatform, ModuleInfo, PARAMETER_MANIFEST_FILE, canonical_utf8,
    normalize_class_id, path_exists_no_symlink, read_bytes_file_no_symlink,
    read_parameter_manifest, read_text_file_no_symlink, real_directory_exists_no_symlink,
    require_real_directory, require_real_file, sanitize_bundle_name,
    validate_bundle_identifier_shape, validate_no_control_chars,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleValidationReport {
    pub bundle_dir: Utf8PathBuf,
    pub moduleinfo_path: Utf8PathBuf,
    pub binary_paths: Vec<Utf8PathBuf>,
    pub binary_export_checks: Vec<BinaryExportCheck>,
    pub parameter_manifest_path: Option<Utf8PathBuf>,
    pub asset_manifest_path: Option<Utf8PathBuf>,
    pub asset_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BinaryExportCheck {
    pub binary: String,
    pub platform: String,
    pub status: String,
    pub tool: Option<String>,
    pub required_symbols: Vec<String>,
    pub found_symbols: Vec<String>,
    pub missing_symbols: Vec<String>,
    pub error: Option<String>,
}

pub fn validate_vst3_bundle(bundle_dir: &Utf8Path) -> Result<BundleValidationReport, BuildError> {
    require_real_directory(bundle_dir)?;
    if bundle_dir.extension() != Some("vst3") {
        return Err(BuildError::InvalidBundle(format!(
            "bundle path must end with .vst3: {bundle_dir}"
        )));
    }

    let contents_dir = bundle_dir.join("Contents");
    require_real_directory(&contents_dir)?;

    let resources_dir = contents_dir.join("Resources");
    require_real_directory(&resources_dir)?;

    let moduleinfo_path = resources_dir.join("moduleinfo.json");
    let moduleinfo_text = read_text_file_no_symlink(&moduleinfo_path)?;
    let moduleinfo: ModuleInfo = serde_json::from_str(&moduleinfo_text)
        .map_err(|error| BuildError::InvalidBundle(format!("invalid moduleinfo.json: {error}")))?;
    validate_moduleinfo(&moduleinfo)?;

    let parameter_manifest_path = resources_dir.join(PARAMETER_MANIFEST_FILE);
    let parameter_manifest_path = if path_exists_no_symlink(&parameter_manifest_path)? {
        read_parameter_manifest(&parameter_manifest_path)?;
        Some(parameter_manifest_path)
    } else {
        None
    };

    let binary_paths = collect_bundle_binaries(&contents_dir)?;
    if binary_paths.is_empty() {
        return Err(BuildError::InvalidBundle(
            "no platform binary found under Contents/MacOS, Contents/x86_64-win or Contents/x86_64-linux".to_string(),
        ));
    }
    let plugin_name = sanitize_bundle_name(&moduleinfo.name);
    validate_platform_binary_names(&contents_dir, &plugin_name)?;
    validate_bundle_binary_formats(&contents_dir, &binary_paths)?;
    let binary_export_checks = validate_bundle_binary_exports(&contents_dir, &binary_paths)?;

    if contents_dir.join("MacOS").is_dir() {
        validate_macos_metadata(
            &contents_dir,
            &plugin_name,
            &moduleinfo.name,
            &moduleinfo.plugin_version,
        )?;
    }

    let ui_dir = resources_dir.join("ui");
    let manifest_path = resources_dir.join("assets.manifest.json");
    let (asset_manifest_path, asset_count) = if path_exists_no_symlink(&ui_dir)?
        || path_exists_no_symlink(&manifest_path)?
    {
        require_real_directory(&ui_dir)?;
        require_real_file(&manifest_path)?;
        let manifest_text = read_text_file_no_symlink(&manifest_path)?;
        let manifest: AssetManifest = serde_json::from_str(&manifest_text).map_err(|error| {
            BuildError::InvalidBundle(format!("invalid assets.manifest.json: {error}"))
        })?;
        validate_asset_manifest_files(&ui_dir, &manifest)?;
        (Some(manifest_path), manifest.files.len())
    } else {
        (None, 0)
    };

    Ok(BundleValidationReport {
        bundle_dir: bundle_dir.to_path_buf(),
        moduleinfo_path,
        binary_paths,
        binary_export_checks,
        parameter_manifest_path,
        asset_manifest_path,
        asset_count,
    })
}

pub(crate) fn validate_moduleinfo(moduleinfo: &ModuleInfo) -> Result<(), BuildError> {
    for (field, value) in [
        ("name", moduleinfo.name.as_str()),
        ("vendor", moduleinfo.vendor.as_str()),
        ("plugin_version", moduleinfo.plugin_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BuildError::InvalidBundle(format!(
                "moduleinfo.json {field} must not be empty"
            )));
        }
        validate_no_control_chars(&format!("moduleinfo.json {field}"), value)
            .map_err(BuildError::InvalidBundle)?;
    }

    if moduleinfo.classes.is_empty() {
        return Err(BuildError::InvalidBundle(
            "moduleinfo.json must contain at least one class".to_string(),
        ));
    }
    for (index, class) in moduleinfo.classes.iter().enumerate() {
        if class.name.trim().is_empty() {
            return Err(BuildError::InvalidBundle(format!(
                "moduleinfo.json classes[{index}].name must not be empty"
            )));
        }
        validate_no_control_chars(
            &format!("moduleinfo.json classes[{index}].name"),
            &class.name,
        )
        .map_err(BuildError::InvalidBundle)?;
        if class.category.trim().is_empty() {
            return Err(BuildError::InvalidBundle(format!(
                "moduleinfo.json classes[{index}].category must not be empty"
            )));
        }
        validate_no_control_chars(
            &format!("moduleinfo.json classes[{index}].category"),
            &class.category,
        )
        .map_err(BuildError::InvalidBundle)?;
        normalize_class_id(&class.cid).map_err(|error| {
            BuildError::InvalidBundle(format!(
                "invalid moduleinfo class cid for {}: {error}",
                class.name
            ))
        })?;
    }
    Ok(())
}

pub(crate) fn validate_platform_binary_names(
    contents_dir: &Utf8Path,
    plugin_name: &str,
) -> Result<(), BuildError> {
    for platform in [
        BundlePlatform::Macos,
        BundlePlatform::WindowsX64,
        BundlePlatform::LinuxX64,
    ] {
        let platform_dir = platform_binary_dir(contents_dir, platform);
        if !platform_dir.exists() {
            continue;
        }
        require_real_directory(&platform_dir)?;
        let expected = platform_binary_path(contents_dir, platform, plugin_name);
        if let Err(error) = require_real_file(&expected) {
            if matches!(error, BuildError::MissingFile(_)) {
                return Err(BuildError::InvalidBundle(format!(
                    "missing expected {} binary: {}",
                    platform_label(platform),
                    expected
                )));
            }
            return Err(BuildError::InvalidBundle(format!(
                "{} binary failed path validation: {} ({error})",
                platform_label(platform),
                expected
            )));
        }
    }
    Ok(())
}

pub(crate) fn platform_binary_dir(
    contents_dir: &Utf8Path,
    platform: BundlePlatform,
) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => contents_dir.join("MacOS"),
        BundlePlatform::WindowsX64 => contents_dir.join("x86_64-win"),
        BundlePlatform::LinuxX64 => contents_dir.join("x86_64-linux"),
    }
}

pub(crate) fn platform_binary_path(
    contents_dir: &Utf8Path,
    platform: BundlePlatform,
    plugin_name: &str,
) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => contents_dir.join("MacOS").join(plugin_name),
        BundlePlatform::WindowsX64 => contents_dir
            .join("x86_64-win")
            .join(format!("{plugin_name}.vst3")),
        BundlePlatform::LinuxX64 => contents_dir
            .join("x86_64-linux")
            .join(format!("{plugin_name}.so")),
    }
}

pub(crate) fn platform_label(platform: BundlePlatform) -> &'static str {
    match platform {
        BundlePlatform::Macos => "macOS",
        BundlePlatform::WindowsX64 => "Windows x64",
        BundlePlatform::LinuxX64 => "Linux x64",
    }
}

pub(crate) fn validate_bundle_binary_formats(
    contents_dir: &Utf8Path,
    binary_paths: &[Utf8PathBuf],
) -> Result<(), BuildError> {
    for path in binary_paths {
        let Some(platform) = infer_bundle_binary_platform(contents_dir, path) else {
            continue;
        };
        validate_binary_format(path, platform)?;
    }
    Ok(())
}

pub(crate) fn infer_bundle_binary_platform(
    contents_dir: &Utf8Path,
    path: &Utf8Path,
) -> Option<BundlePlatform> {
    if path.starts_with(contents_dir.join("MacOS")) {
        Some(BundlePlatform::Macos)
    } else if path.starts_with(contents_dir.join("x86_64-win")) {
        Some(BundlePlatform::WindowsX64)
    } else if path.starts_with(contents_dir.join("x86_64-linux")) {
        Some(BundlePlatform::LinuxX64)
    } else {
        None
    }
}

pub(crate) fn validate_binary_format(
    path: &Utf8Path,
    platform: BundlePlatform,
) -> Result<(), BuildError> {
    let bytes = read_bytes_file_no_symlink(path)?;
    if binary_format_matches(&bytes, platform) {
        return Ok(());
    }

    Err(BuildError::InvalidBundle(format!(
        "{} binary has unexpected file format: {}",
        platform_label(platform),
        path
    )))
}

pub(crate) fn binary_format_matches(bytes: &[u8], platform: BundlePlatform) -> bool {
    match platform {
        BundlePlatform::Macos => macos_binary_format_matches(bytes),
        BundlePlatform::WindowsX64 => windows_x64_binary_format_matches(bytes),
        BundlePlatform::LinuxX64 => linux_x64_binary_format_matches(bytes),
    }
}

pub(crate) fn macos_binary_format_matches(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(..4),
        Some(
            [0xfe, 0xed, 0xfa, 0xcf]
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
                | [0xca, 0xfe, 0xba, 0xbf]
                | [0xbf, 0xba, 0xfe, 0xca]
        )
    )
}

pub(crate) fn windows_x64_binary_format_matches(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"MZ") || bytes.len() < 0x40 {
        return false;
    }
    let pe_offset =
        u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
    let Some(header) = bytes.get(pe_offset..pe_offset.saturating_add(6)) else {
        return false;
    };
    header.starts_with(b"PE\0\0") && u16::from_le_bytes([header[4], header[5]]) == 0x8664
}

pub(crate) fn linux_x64_binary_format_matches(bytes: &[u8]) -> bool {
    if bytes.len() < 20 || !bytes.starts_with(b"\x7fELF") || bytes[4] != 2 {
        return false;
    }
    match bytes[5] {
        1 => u16::from_le_bytes([bytes[18], bytes[19]]) == 0x3e,
        2 => u16::from_be_bytes([bytes[18], bytes[19]]) == 0x3e,
        _ => false,
    }
}

pub(crate) fn validate_bundle_binary_exports(
    contents_dir: &Utf8Path,
    binary_paths: &[Utf8PathBuf],
) -> Result<Vec<BinaryExportCheck>, BuildError> {
    let mut checks = Vec::new();
    for path in binary_paths {
        let Some(platform) = infer_bundle_binary_platform(contents_dir, path) else {
            continue;
        };
        let check = inspect_binary_exports(path, platform);
        if check.status == "failed" {
            return Err(BuildError::InvalidBundle(format!(
                "{} binary is missing required VST3 export symbols: {} ({})",
                platform_label(platform),
                check.missing_symbols.join(", "),
                path
            )));
        }
        checks.push(check);
    }
    Ok(checks)
}

pub(crate) fn inspect_binary_exports(
    path: &Utf8Path,
    platform: BundlePlatform,
) -> BinaryExportCheck {
    let required_symbols = required_export_symbols(platform)
        .iter()
        .map(|symbol| (*symbol).to_string())
        .collect::<Vec<_>>();
    let mut attempts = Vec::new();

    for tool in export_symbol_tools(platform) {
        match Command::new(tool.program)
            .args(tool.args)
            .arg(path.as_std_path())
            .output()
        {
            Ok(output) if output.status.success() => {
                let mut text = String::from_utf8_lossy(&output.stdout).to_string();
                if !output.stderr.is_empty() {
                    text.push('\n');
                    text.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                return binary_export_check_from_output(path, platform, tool.display(), &text);
            }
            Ok(output) => {
                attempts.push(format!(
                    "{} exited with {}{}{}",
                    tool.display(),
                    output
                        .status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "signal".to_string()),
                    output_message(" stdout", &output.stdout),
                    output_message(" stderr", &output.stderr),
                ));
            }
            Err(error) => {
                attempts.push(format!("{}: {error}", tool.display()));
            }
        }
    }

    BinaryExportCheck {
        binary: path.to_string(),
        platform: platform_slug(platform).to_string(),
        status: "skipped".to_string(),
        tool: None,
        required_symbols,
        found_symbols: Vec::new(),
        missing_symbols: Vec::new(),
        error: Some(format!(
            "no usable export-symbol inspection tool for {}; attempted: {}",
            platform_label(platform),
            attempts.join("; ")
        )),
    }
}

pub(crate) fn binary_export_check_from_output(
    path: &Utf8Path,
    platform: BundlePlatform,
    tool: String,
    output: &str,
) -> BinaryExportCheck {
    let required_symbols = required_export_symbols(platform);
    let found_symbols = required_symbols
        .iter()
        .copied()
        .filter(|symbol| export_output_contains_symbol(output, symbol))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let missing_symbols = required_symbols
        .iter()
        .copied()
        .filter(|symbol| !found_symbols.iter().any(|found| found == symbol))
        .map(str::to_string)
        .collect::<Vec<_>>();
    BinaryExportCheck {
        binary: path.to_string(),
        platform: platform_slug(platform).to_string(),
        status: if missing_symbols.is_empty() {
            "ok".to_string()
        } else {
            "failed".to_string()
        },
        tool: Some(tool),
        required_symbols: required_symbols
            .iter()
            .map(|symbol| (*symbol).to_string())
            .collect(),
        found_symbols,
        missing_symbols,
        error: None,
    }
}

pub(crate) fn export_output_contains_symbol(output: &str, symbol: &str) -> bool {
    output.lines().any(|line| {
        line.split(|char: char| {
            !(char.is_ascii_alphanumeric() || matches!(char, '_' | '@' | '$' | '.'))
        })
        .any(|token| token == symbol)
    })
}

pub(crate) fn output_message(label: &str, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let text = String::from_utf8_lossy(bytes);
    let compact = text.trim().replace('\n', " ");
    if compact.is_empty() {
        String::new()
    } else {
        let snippet = compact.chars().take(240).collect::<String>();
        format!("{label}: {snippet}")
    }
}

pub(crate) fn export_symbol_tools(
    platform: BundlePlatform,
) -> &'static [vesty_vst3_sys::BinaryExportInspectionToolPlan] {
    vesty_vst3_sys::binary_export_inspection_tools(platform_slug(platform))
        .expect("all bundle platforms have VST3 binary export inspection tools")
}

pub(crate) fn required_export_symbols(platform: BundlePlatform) -> &'static [&'static str] {
    vesty_vst3_sys::required_binary_export_tool_symbols(platform_slug(platform))
        .expect("all bundle platforms have required VST3 binary export symbols")
}

pub(crate) fn platform_slug(platform: BundlePlatform) -> &'static str {
    match platform {
        BundlePlatform::Macos => "macos",
        BundlePlatform::WindowsX64 => "windows-x64",
        BundlePlatform::LinuxX64 => "linux-x64",
    }
}

pub(crate) fn validate_macos_metadata(
    contents_dir: &Utf8Path,
    expected_executable: &str,
    expected_bundle_name: &str,
    expected_version: &str,
) -> Result<(), BuildError> {
    let info_plist_path = contents_dir.join("Info.plist");
    let info_plist_path = require_real_file(&info_plist_path)?;

    let pkg_info_path = contents_dir.join("PkgInfo");
    let pkg_info_path = require_real_file(&pkg_info_path)?;

    let plist = plist::Value::from_file(&info_plist_path)
        .map_err(|error| BuildError::InvalidBundle(format!("invalid Info.plist: {error}")))?;
    let dict = plist.as_dictionary().ok_or_else(|| {
        BuildError::InvalidBundle("Info.plist must contain a dictionary".to_string())
    })?;

    let package_type = required_plist_string(dict, "CFBundlePackageType")?;
    if package_type != "BNDL" {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundlePackageType must be BNDL, got {package_type}"
        )));
    }

    let executable = required_plist_string(dict, "CFBundleExecutable")?;
    if !is_safe_bundle_executable_name(executable) {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundleExecutable must be a single file name, got {executable}"
        )));
    }
    let executable_path = contents_dir.join("MacOS").join(executable);
    require_real_file(&executable_path)?;
    if executable != expected_executable {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundleExecutable must match moduleinfo binary name {expected_executable}, got {executable}"
        )));
    }

    let bundle_name = required_plist_string(dict, "CFBundleName")?;
    if bundle_name != expected_bundle_name {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundleName must match moduleinfo name {expected_bundle_name}, got {bundle_name}"
        )));
    }

    let short_version = required_plist_string(dict, "CFBundleShortVersionString")?;
    if short_version != expected_version {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundleShortVersionString must match moduleinfo plugin_version {expected_version}, got {short_version}"
        )));
    }
    let bundle_version = required_plist_string(dict, "CFBundleVersion")?;
    if bundle_version != expected_version {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist CFBundleVersion must match moduleinfo plugin_version {expected_version}, got {bundle_version}"
        )));
    }

    let bundle_id = required_plist_string(dict, "CFBundleIdentifier")?;
    validate_bundle_identifier_shape("Info.plist CFBundleIdentifier", bundle_id)
        .map_err(BuildError::InvalidBundle)?;

    let pkg_info = fs::read(&pkg_info_path)?;
    if pkg_info != b"BNDL????" {
        return Err(BuildError::InvalidBundle(
            "PkgInfo must be exactly BNDL????".to_string(),
        ));
    }

    Ok(())
}

pub(crate) fn required_plist_string<'a>(
    dict: &'a plist::Dictionary,
    key: &str,
) -> Result<&'a str, BuildError> {
    let value = dict
        .get(key)
        .and_then(plist::Value::as_string)
        .ok_or_else(|| BuildError::InvalidBundle(format!("Info.plist missing {key} string")))?;
    if value.trim().is_empty() {
        return Err(BuildError::InvalidBundle(format!(
            "Info.plist {key} must not be empty"
        )));
    }
    Ok(value)
}

pub(crate) fn is_safe_bundle_executable_name(executable: &str) -> bool {
    !executable.contains('/')
        && !executable.contains('\\')
        && executable != "."
        && executable != ".."
        && !executable.trim().is_empty()
}

pub(crate) fn collect_bundle_binaries(
    contents_dir: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, BuildError> {
    let mut binaries = Vec::new();
    collect_files_matching(&contents_dir.join("MacOS"), &mut binaries, |_| true)?;
    collect_files_matching(&contents_dir.join("x86_64-win"), &mut binaries, |path| {
        path.extension() == Some("vst3")
    })?;
    collect_files_matching(&contents_dir.join("x86_64-linux"), &mut binaries, |path| {
        path.extension() == Some("so")
    })?;
    binaries.sort();
    Ok(binaries)
}

pub(crate) fn collect_files_matching(
    dir: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
    predicate: impl Fn(&Utf8Path) -> bool + Copy,
) -> Result<(), BuildError> {
    if !real_directory_exists_no_symlink(dir)? {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| BuildError::NonUtf8Path)?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SymlinkAsset(path.to_string()));
        }
        if metadata.is_dir() {
            collect_files_matching(&path, files, predicate)?;
        } else if predicate(&path) {
            files.push(path);
        }
    }
    Ok(())
}

pub(crate) fn validate_asset_manifest_files(
    ui_dir: &Utf8Path,
    manifest: &AssetManifest,
) -> Result<(), BuildError> {
    let ui_root = require_real_directory(ui_dir)?;
    if manifest.version != 1 {
        return Err(BuildError::InvalidBundle(format!(
            "unsupported asset manifest version {}",
            manifest.version
        )));
    }
    if manifest.root.trim().is_empty() || manifest.root.chars().any(char::is_control) {
        return Err(BuildError::InvalidBundle(
            "asset manifest root is invalid".to_string(),
        ));
    }
    if !is_safe_manifest_path(&manifest.entry)
        || !manifest
            .files
            .iter()
            .any(|file| file.path == manifest.entry)
    {
        return Err(BuildError::InvalidBundle(
            "asset manifest entry is missing from files".to_string(),
        ));
    }

    let mut seen_paths = BTreeSet::new();
    for file in &manifest.files {
        if !is_safe_manifest_path(&file.path) {
            return Err(BuildError::PathEscapesRoot(file.path.clone()));
        }
        if !seen_paths.insert(file.path.as_str()) {
            return Err(BuildError::InvalidBundle(format!(
                "asset manifest contains duplicate path: {}",
                file.path
            )));
        }
        validate_asset_manifest_mime(&file.path, &file.mime)?;
        if !is_valid_sha256_hex(&file.sha256) {
            return Err(BuildError::AssetIntegrity(format!(
                "{} sha256 must be a 64-byte hex digest",
                file.path
            )));
        }
        let path = ui_root.join(&file.path);
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| BuildError::MissingFile(path.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SymlinkAsset(path.to_string()));
        }
        if !metadata.is_file() {
            return Err(BuildError::MissingFile(path.to_string()));
        }
        let canonical = canonical_utf8(&path)?;
        if !canonical.starts_with(&ui_root) {
            return Err(BuildError::PathEscapesRoot(file.path.clone()));
        }
        let bytes = fs::read(&canonical)?;
        if bytes.len() as u64 != file.size {
            return Err(BuildError::AssetIntegrity(format!(
                "{} size mismatch: manifest {}, actual {}",
                file.path,
                file.size,
                bytes.len()
            )));
        }
        let actual_sha256 = sha256_hex(&bytes);
        if !actual_sha256.eq_ignore_ascii_case(&file.sha256) {
            return Err(BuildError::AssetIntegrity(format!(
                "{} sha256 mismatch",
                file.path
            )));
        }
    }
    Ok(())
}

pub(crate) fn is_safe_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(is_safe_manifest_segment)
}

pub(crate) fn is_safe_manifest_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'%' | b'?' | b'#' | b':'))
}

pub(crate) fn validate_asset_manifest_mime(path: &str, mime: &str) -> Result<(), BuildError> {
    if mime.trim().is_empty() {
        return Err(BuildError::InvalidBundle(format!(
            "asset manifest mime must not be empty for {path}"
        )));
    }
    if mime.trim() != mime {
        return Err(BuildError::InvalidBundle(format!(
            "asset manifest mime must not have leading or trailing whitespace for {path}"
        )));
    }
    if mime.chars().any(char::is_control) {
        return Err(BuildError::InvalidBundle(format!(
            "asset manifest mime must not contain control characters for {path}"
        )));
    }
    Ok(())
}

pub(crate) fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
