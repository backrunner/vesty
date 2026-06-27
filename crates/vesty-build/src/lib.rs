use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;
use thiserror::Error;
use vesty_params::{
    ParamSpec, VST3_PARAM_ID_ALGORITHM, stable_vst3_param_id, validate_param_specs,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VestyConfig {
    pub plugin: PluginConfig,
    pub ui: Option<UiConfig>,
    pub package: Option<PackageConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginConfig {
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub kind: String,
    pub class_id: String,
    pub sidechain: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    pub dir: String,
    pub dev_url: Option<String>,
    pub build: Option<String>,
    pub dist: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageConfig {
    pub bundle_id: Option<String>,
    pub category: Option<String>,
    pub signing: Option<String>,
    pub parameter_manifest: Option<String>,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("failed to serialize json: {0}")]
    JsonSerialize(#[from] serde_json::Error),
    #[error("failed to read/write plist: {0}")]
    Plist(#[from] plist::Error),
    #[error("path is not valid utf-8")]
    NonUtf8Path,
    #[error("missing required file: {0}")]
    MissingFile(String),
    #[error("invalid VST3 bundle: {0}")]
    InvalidBundle(String),
    #[error("invalid Vesty config: {0}")]
    InvalidConfig(String),
    #[error("invalid parameter manifest: {0}")]
    InvalidParameterManifest(String),
    #[error("invalid parameter specs: {0}")]
    InvalidParameterSpecs(String),
    #[error("asset integrity check failed: {0}")]
    AssetIntegrity(String),
    #[error("asset path escapes root: {0}")]
    PathEscapesRoot(String),
    #[error("symlinked assets are not allowed: {0}")]
    SymlinkAsset(String),
}

pub fn read_config(path: &Utf8Path) -> Result<VestyConfig, BuildError> {
    let text = read_text_file_no_symlink(path)?;
    let config = toml::from_str(&text)?;
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &VestyConfig) -> Result<(), BuildError> {
    validate_required_config_field("[plugin].name", &config.plugin.name)?;
    validate_required_config_field("[plugin].vendor", &config.plugin.vendor)?;
    validate_required_config_field("[plugin].version", &config.plugin.version)?;
    validate_required_config_field("[plugin].kind", &config.plugin.kind)?;
    let plugin_category = plugin_kind_category(&config.plugin.kind)?;
    normalize_class_id(&config.plugin.class_id)?;
    if config.plugin.sidechain == Some(true) && plugin_category == "Instrument" {
        return Err(BuildError::InvalidConfig(
            "[plugin].sidechain is only supported for effect plugins".to_string(),
        ));
    }

    if let Some(package) = &config.package {
        if let Some(bundle_id) = &package.bundle_id {
            validate_required_config_field("[package].bundle_id", bundle_id)?;
            validate_bundle_identifier("[package].bundle_id", bundle_id)?;
        }
        validate_optional_config_text("[package].category", &package.category)?;
        validate_optional_config_field("[package].signing", &package.signing)?;
        validate_optional_config_field(
            "[package].parameter_manifest",
            &package.parameter_manifest,
        )?;
    }

    if let Some(ui) = &config.ui {
        validate_ui_config(ui)?;
    }

    Ok(())
}

fn validate_required_config_field(name: &str, value: &str) -> Result<(), BuildError> {
    if value.trim().is_empty() {
        return Err(BuildError::InvalidConfig(format!(
            "{name} must not be empty"
        )));
    }
    validate_no_control_chars(name, value).map_err(BuildError::InvalidConfig)?;
    Ok(())
}

fn validate_optional_config_field(name: &str, value: &Option<String>) -> Result<(), BuildError> {
    if let Some(value) = value {
        validate_required_config_field(name, value)?;
    }
    Ok(())
}

fn validate_optional_config_text(name: &str, value: &Option<String>) -> Result<(), BuildError> {
    if let Some(value) = value {
        validate_no_control_chars(name, value).map_err(BuildError::InvalidConfig)?;
    }
    Ok(())
}

fn validate_no_control_chars(name: &str, value: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!("{name} must not contain control characters"));
    }
    Ok(())
}

fn validate_bundle_identifier(name: &str, value: &str) -> Result<(), BuildError> {
    validate_bundle_identifier_shape(name, value).map_err(BuildError::InvalidConfig)
}

fn validate_bundle_identifier_shape(name: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    let mut segment_count = 0usize;
    for segment in value.split('.') {
        segment_count += 1;
        if segment.is_empty() {
            return Err(format!(
                "{name} must not contain empty dot-separated segments"
            ));
        }
        if segment.starts_with('-') || segment.ends_with('-') {
            return Err(format!("{name} segments must not start or end with '-'"));
        }
        if !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(format!(
                "{name} may only contain ASCII letters, numbers, '-' and '.'"
            ));
        }
    }
    if segment_count < 2 {
        return Err(format!(
            "{name} should use reverse-DNS style with at least one '.'"
        ));
    }
    Ok(())
}

fn validate_ui_config(ui: &UiConfig) -> Result<(), BuildError> {
    validate_required_config_field("[ui].dir", &ui.dir)?;
    validate_optional_config_field("[ui].dev_url", &ui.dev_url)?;
    validate_optional_config_field("[ui].build", &ui.build)?;
    validate_optional_config_field("[ui].dist", &ui.dist)?;
    validate_dimension_pair("[ui].width", ui.width, "[ui].height", ui.height)?;
    validate_dimension_pair(
        "[ui].min_width",
        ui.min_width,
        "[ui].min_height",
        ui.min_height,
    )?;

    if let (Some(width), Some(min_width)) = (ui.width, ui.min_width)
        && min_width > width
    {
        return Err(BuildError::InvalidConfig(format!(
            "[ui].min_width must be <= [ui].width ({min_width} > {width})"
        )));
    }
    if let (Some(height), Some(min_height)) = (ui.height, ui.min_height)
        && min_height > height
    {
        return Err(BuildError::InvalidConfig(format!(
            "[ui].min_height must be <= [ui].height ({min_height} > {height})"
        )));
    }

    Ok(())
}

fn validate_dimension_pair(
    first_name: &str,
    first: Option<u32>,
    second_name: &str,
    second: Option<u32>,
) -> Result<(), BuildError> {
    match (first, second) {
        (Some(0), _) => Err(BuildError::InvalidConfig(format!(
            "{first_name} must be greater than 0"
        ))),
        (_, Some(0)) => Err(BuildError::InvalidConfig(format!(
            "{second_name} must be greater than 0"
        ))),
        (Some(_), None) => Err(BuildError::InvalidConfig(format!(
            "{second_name} must be set when {first_name} is set"
        ))),
        (None, Some(_)) => Err(BuildError::InvalidConfig(format!(
            "{first_name} must be set when {second_name} is set"
        ))),
        _ => Ok(()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetManifest {
    pub version: u32,
    pub root: String,
    pub entry: String,
    pub files: Vec<AssetFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetFile {
    pub path: String,
    pub mime: String,
    pub sha256: String,
    pub size: u64,
}

impl AssetManifest {
    pub fn from_dir(root: &Utf8Path, entry: impl Into<String>) -> Result<Self, BuildError> {
        let root = require_real_directory(root)?;
        let mut files = Vec::new();
        collect_assets(&root, &root, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let entry = entry.into().replace('\\', "/");
        if !is_safe_manifest_path(&entry) {
            return Err(BuildError::InvalidBundle(format!(
                "asset manifest entry is not safe for custom protocol: {entry}"
            )));
        }
        if !files.iter().any(|file| file.path == entry) {
            return Err(BuildError::MissingFile(entry));
        }
        Ok(Self {
            version: 1,
            root: root.to_string(),
            entry,
            files,
        })
    }
}

fn collect_assets(
    root: &Utf8Path,
    current: &Utf8Path,
    files: &mut Vec<AssetFile>,
) -> Result<(), BuildError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| BuildError::NonUtf8Path)?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SymlinkAsset(path.to_string()));
        }
        if metadata.is_dir() {
            collect_assets(root, &path, files)?;
            continue;
        }
        let canonical = canonical_utf8(&path)?;
        if !canonical.starts_with(root) {
            return Err(BuildError::PathEscapesRoot(path.to_string()));
        }
        let relative = canonical
            .strip_prefix(root)
            .map_err(|_| BuildError::NonUtf8Path)?;
        let relative_path = relative.as_str().replace('\\', "/");
        if !is_safe_manifest_path(&relative_path) {
            return Err(BuildError::InvalidBundle(format!(
                "asset path is not safe for custom protocol: {relative_path}"
            )));
        }
        let bytes = fs::read(&canonical)?;
        let sha256 = sha256_hex(&bytes);
        let mime = mime_guess::from_path(&canonical)
            .first_or_octet_stream()
            .to_string();
        files.push(AssetFile {
            path: relative_path,
            mime,
            sha256,
            size: bytes.len() as u64,
        });
    }
    Ok(())
}

fn canonical_utf8(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
    Utf8PathBuf::from_path_buf(path.canonicalize()?).map_err(|_| BuildError::NonUtf8Path)
}

fn require_real_directory(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| BuildError::MissingFile(path.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    if !metadata.is_dir() {
        return Err(BuildError::MissingFile(path.to_string()));
    }
    canonical_utf8(path)
}

fn real_directory_exists_no_symlink(path: &Utf8Path) -> Result<bool, BuildError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(BuildError::Io(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    Ok(metadata.is_dir())
}

fn existing_directory_no_parent_or_leaf_symlink(path: &Utf8Path) -> Result<bool, BuildError> {
    reject_existing_output_parent_symlink(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(BuildError::Io(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    if !metadata.is_dir() {
        return Err(BuildError::MissingFile(path.to_string()));
    }
    Ok(true)
}

fn create_directory_no_parent_or_leaf_symlink(path: &Utf8Path) -> Result<(), BuildError> {
    if existing_directory_no_parent_or_leaf_symlink(path)? {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    require_real_directory(path)?;
    Ok(())
}

fn remove_existing_output_directory(path: &Utf8Path) -> Result<(), BuildError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BuildError::Io(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    if !metadata.is_dir() {
        return Err(BuildError::MissingFile(path.to_string()));
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

fn reject_existing_output_file_symlink(path: &Utf8Path) -> Result<(), BuildError> {
    reject_existing_output_parent_symlink(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BuildError::Io(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    if metadata.is_dir() {
        return Err(BuildError::MissingFile(path.to_string()));
    }
    Ok(())
}

fn write_text_file_no_symlink(path: &Utf8Path, text: &str) -> Result<(), BuildError> {
    write_bytes_file_no_symlink(path, text.as_bytes())
}

fn write_bytes_file_no_symlink(path: &Utf8Path, bytes: &[u8]) -> Result<(), BuildError> {
    reject_existing_output_file_symlink(path)?;
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        create_directory_no_parent_or_leaf_symlink(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn copy_file_no_symlink(source: &Utf8Path, destination: &Utf8Path) -> Result<(), BuildError> {
    let source = require_real_file(source)?;
    reject_existing_output_file_symlink(destination)?;
    if let Some(parent) = destination.parent()
        && !parent.as_str().is_empty()
    {
        create_directory_no_parent_or_leaf_symlink(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn reject_existing_output_parent_symlink(path: &Utf8Path) -> Result<(), BuildError> {
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
            Err(error) => return Err(BuildError::Io(error)),
        };
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SymlinkAsset(ancestor.to_string()));
        }
        if !metadata.is_dir() {
            return Err(BuildError::MissingFile(ancestor.to_string()));
        }
    }
    Ok(())
}

fn path_exists_no_symlink(path: &Utf8Path) -> Result<bool, BuildError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(BuildError::Io(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    Ok(true)
}

fn require_real_file(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| BuildError::MissingFile(path.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(BuildError::SymlinkAsset(path.to_string()));
    }
    if !metadata.is_file() {
        return Err(BuildError::MissingFile(path.to_string()));
    }
    canonical_utf8(path)
}

fn read_text_file_no_symlink(path: &Utf8Path) -> Result<String, BuildError> {
    let canonical = require_real_file(path)?;
    fs::read_to_string(canonical).map_err(BuildError::Io)
}

fn read_bytes_file_no_symlink(path: &Utf8Path) -> Result<Vec<u8>, BuildError> {
    let canonical = require_real_file(path)?;
    fs::read(canonical).map_err(BuildError::Io)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundlePlatform {
    Macos,
    WindowsX64,
    LinuxX64,
}

pub fn binary_relative_path(platform: BundlePlatform, plugin_name: &str) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => Utf8PathBuf::from(format!("Contents/MacOS/{plugin_name}")),
        BundlePlatform::WindowsX64 => {
            Utf8PathBuf::from(format!("Contents/x86_64-win/{plugin_name}.vst3"))
        }
        BundlePlatform::LinuxX64 => {
            Utf8PathBuf::from(format!("Contents/x86_64-linux/{plugin_name}.so"))
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackageOptions {
    pub project_dir: Utf8PathBuf,
    pub output_dir: Utf8PathBuf,
    pub platform: BundlePlatform,
    pub binary_path: Utf8PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageReport {
    pub bundle_dir: Utf8PathBuf,
    pub binary_path: Utf8PathBuf,
    pub moduleinfo_path: Utf8PathBuf,
    pub parameter_manifest_path: Option<Utf8PathBuf>,
    pub asset_manifest_path: Option<Utf8PathBuf>,
    pub copied_assets: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModuleInfo {
    pub version: u32,
    pub name: String,
    pub vendor: String,
    pub plugin_version: String,
    pub classes: Vec<ModuleClassInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModuleClassInfo {
    pub name: String,
    pub cid: String,
    pub category: String,
}

pub const PARAMETER_MANIFEST_FILE: &str = "parameters.manifest.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterManifest {
    pub version: u32,
    pub id_algorithm: String,
    pub parameters: Vec<ParameterManifestEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterManifestEntry {
    pub id: String,
    pub vst3_param_id: u32,
    pub spec: ParamSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterSpecDocument {
    pub version: u32,
    pub parameters: Vec<ParamSpec>,
}

impl ParameterManifest {
    pub fn from_param_specs(specs: Vec<ParamSpec>) -> Result<Self, BuildError> {
        validate_param_specs(&specs)
            .map_err(|error| BuildError::InvalidParameterManifest(error.to_string()))?;
        let manifest = Self {
            version: 1,
            id_algorithm: VST3_PARAM_ID_ALGORITHM.to_string(),
            parameters: specs
                .into_iter()
                .map(|spec| ParameterManifestEntry {
                    id: spec.id.clone(),
                    vst3_param_id: stable_vst3_param_id(&spec.id),
                    spec,
                })
                .collect(),
        };
        validate_parameter_manifest(&manifest)?;
        Ok(manifest)
    }
}

impl ParameterSpecDocument {
    pub fn new(parameters: Vec<ParamSpec>) -> Result<Self, BuildError> {
        validate_param_specs(&parameters)
            .map_err(|error| BuildError::InvalidParameterSpecs(error.to_string()))?;
        Ok(Self {
            version: 1,
            parameters,
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ParameterSpecInput {
    Array(Vec<ParamSpec>),
    Document(ParameterSpecDocument),
}

pub fn parameter_manifest_from_specs_json(text: &str) -> Result<ParameterManifest, BuildError> {
    let input = serde_json::from_str::<ParameterSpecInput>(text)
        .map_err(|error| BuildError::InvalidParameterSpecs(error.to_string()))?;
    let document = match input {
        ParameterSpecInput::Array(parameters) => ParameterSpecDocument::new(parameters)?,
        ParameterSpecInput::Document(document) => {
            if document.version != 1 {
                return Err(BuildError::InvalidParameterSpecs(format!(
                    "unsupported parameter specs version {}",
                    document.version
                )));
            }
            validate_param_specs(&document.parameters)
                .map_err(|error| BuildError::InvalidParameterSpecs(error.to_string()))?;
            document
        }
    };
    ParameterManifest::from_param_specs(document.parameters)
}

pub fn read_parameter_specs(path: &Utf8Path) -> Result<ParameterSpecDocument, BuildError> {
    let text = read_text_file_no_symlink(path)?;
    let input = serde_json::from_str::<ParameterSpecInput>(&text)
        .map_err(|error| BuildError::InvalidParameterSpecs(error.to_string()))?;
    match input {
        ParameterSpecInput::Array(parameters) => ParameterSpecDocument::new(parameters),
        ParameterSpecInput::Document(document) => {
            if document.version != 1 {
                return Err(BuildError::InvalidParameterSpecs(format!(
                    "unsupported parameter specs version {}",
                    document.version
                )));
            }
            validate_param_specs(&document.parameters)
                .map_err(|error| BuildError::InvalidParameterSpecs(error.to_string()))?;
            Ok(document)
        }
    }
}

pub fn read_parameter_manifest(path: &Utf8Path) -> Result<ParameterManifest, BuildError> {
    let text = read_text_file_no_symlink(path)?;
    let manifest = serde_json::from_str::<ParameterManifest>(&text)
        .map_err(|error| BuildError::InvalidParameterManifest(error.to_string()))?;
    validate_parameter_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_parameter_manifest(manifest: &ParameterManifest) -> Result<(), BuildError> {
    if manifest.version != 1 {
        return Err(BuildError::InvalidParameterManifest(format!(
            "unsupported parameter manifest version {}",
            manifest.version
        )));
    }
    if manifest.id_algorithm != VST3_PARAM_ID_ALGORITHM {
        return Err(BuildError::InvalidParameterManifest(format!(
            "parameter manifest idAlgorithm must be {VST3_PARAM_ID_ALGORITHM}"
        )));
    }

    let specs = manifest
        .parameters
        .iter()
        .map(|entry| entry.spec.clone())
        .collect::<Vec<_>>();
    validate_param_specs(&specs)
        .map_err(|error| BuildError::InvalidParameterManifest(error.to_string()))?;

    let mut seen_host_ids = BTreeSet::new();
    for (index, entry) in manifest.parameters.iter().enumerate() {
        if entry.id != entry.spec.id {
            return Err(BuildError::InvalidParameterManifest(format!(
                "parameters[{index}].id must match parameters[{index}].spec.id"
            )));
        }
        let expected = stable_vst3_param_id(&entry.spec.id);
        if entry.vst3_param_id != expected {
            return Err(BuildError::InvalidParameterManifest(format!(
                "parameters[{index}].vst3ParamId for {} must be {expected}",
                entry.spec.id
            )));
        }
        if !seen_host_ids.insert(entry.vst3_param_id) {
            return Err(BuildError::InvalidParameterManifest(format!(
                "duplicate VST3 ParamID {} in parameter manifest",
                entry.vst3_param_id
            )));
        }
    }
    Ok(())
}

pub fn package_vst3(
    config: &VestyConfig,
    options: &PackageOptions,
) -> Result<PackageReport, BuildError> {
    validate_config(config)?;

    let binary_source = require_real_file(&options.binary_path)?;
    validate_binary_format(&binary_source, options.platform)?;

    let plugin_name = sanitize_bundle_name(&config.plugin.name);
    let bundle_dir = options.output_dir.join(format!("{plugin_name}.vst3"));
    let contents_dir = bundle_dir.join("Contents");
    let resources_dir = contents_dir.join("Resources");
    create_directory_no_parent_or_leaf_symlink(&resources_dir)?;

    let binary_dest = bundle_dir.join(binary_relative_path(options.platform, &plugin_name));
    if let Some(parent) = binary_dest.parent() {
        create_directory_no_parent_or_leaf_symlink(parent)?;
    }
    copy_file_no_symlink(&binary_source, &binary_dest)?;

    if options.platform == BundlePlatform::Macos {
        write_macos_plist(config, &contents_dir, &plugin_name)?;
        write_text_file_no_symlink(&contents_dir.join("PkgInfo"), "BNDL????")?;
    }

    let moduleinfo = ModuleInfo {
        version: 1,
        name: config.plugin.name.clone(),
        vendor: config.plugin.vendor.clone(),
        plugin_version: config.plugin.version.clone(),
        classes: vec![ModuleClassInfo {
            name: config.plugin.name.clone(),
            cid: normalize_class_id(&config.plugin.class_id)?,
            category: module_category(config)?,
        }],
    };
    let moduleinfo_path = resources_dir.join("moduleinfo.json");
    write_text_file_no_symlink(&moduleinfo_path, &serde_json_pretty(&moduleinfo)?)?;

    let parameter_manifest_path = configured_parameter_manifest_path(config, &options.project_dir)
        .map(|source_path| {
            let manifest = read_parameter_manifest(&source_path)?;
            let manifest_path = resources_dir.join(PARAMETER_MANIFEST_FILE);
            write_text_file_no_symlink(&manifest_path, &serde_json_pretty(&manifest)?)?;
            Ok::<Utf8PathBuf, BuildError>(manifest_path)
        })
        .transpose()?;

    let mut copied_assets = 0;
    let mut asset_manifest_path = None;
    if let Some(ui) = &config.ui
        && let Some(dist) = &ui.dist
    {
        let ui_dist = options.project_dir.join(&ui.dir).join(dist);
        if real_directory_exists_no_symlink(&ui_dist)? {
            let ui_dest = resources_dir.join("ui");
            remove_existing_output_directory(&ui_dest)?;
            copied_assets = copy_dir_recursive(&ui_dist, &ui_dest)?;
            let manifest = AssetManifest::from_dir(&ui_dest, "index.html")?;
            let manifest_path = resources_dir.join("assets.manifest.json");
            write_text_file_no_symlink(&manifest_path, &serde_json_pretty(&manifest)?)?;
            asset_manifest_path = Some(manifest_path);
        }
    }

    Ok(PackageReport {
        bundle_dir,
        binary_path: binary_dest,
        moduleinfo_path,
        parameter_manifest_path,
        asset_manifest_path,
        copied_assets,
    })
}

fn configured_parameter_manifest_path(
    config: &VestyConfig,
    project_dir: &Utf8Path,
) -> Option<Utf8PathBuf> {
    let path = config
        .package
        .as_ref()
        .and_then(|package| package.parameter_manifest.as_deref())
        .map(str::trim)
        .filter(|path| !path.is_empty())?;
    Some(resolve_project_path(project_dir, path))
}

fn resolve_project_path(project_dir: &Utf8Path, path: &str) -> Utf8PathBuf {
    let path = Utf8Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_dir.join(path)
    }
}

fn module_category(config: &VestyConfig) -> Result<String, BuildError> {
    if let Some(category) = config
        .package
        .as_ref()
        .and_then(|package| package.category.as_deref())
        .map(str::trim)
        .filter(|category| !category.is_empty())
    {
        return Ok(category.to_string());
    }

    plugin_kind_category(&config.plugin.kind).map(str::to_string)
}

fn plugin_kind_category(kind: &str) -> Result<&'static str, BuildError> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "effect" | "fx" | "audio-effect" | "audio_effect" => Ok("Fx"),
        "instrument" => Ok("Instrument"),
        _ => Err(BuildError::InvalidConfig(format!(
            "[plugin].kind must be one of effect, fx, audio-effect, audio_effect or instrument: {kind}"
        ))),
    }
}

pub fn normalize_class_id(class_id: &str) -> Result<String, BuildError> {
    let compact = class_id
        .trim()
        .chars()
        .filter(|char| *char != '-')
        .collect::<String>();
    if compact.len() != 32 || !compact.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(BuildError::InvalidConfig(format!(
            "plugin.class_id must be a 16-byte hex UUID/FUID: {class_id}"
        )));
    }

    let compact = compact.to_ascii_lowercase();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &compact[0..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..32]
    ))
}

fn copy_dir_recursive(source: &Utf8Path, destination: &Utf8Path) -> Result<usize, BuildError> {
    let root = require_real_directory(source)?;
    copy_dir_recursive_inner(&root, source, destination)
}

fn copy_dir_recursive_inner(
    root: &Utf8Path,
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<usize, BuildError> {
    create_directory_no_parent_or_leaf_symlink(destination)?;
    let mut copied = 0;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path =
            Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| BuildError::NonUtf8Path)?;
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            return Err(BuildError::SymlinkAsset(source_path.to_string()));
        }
        let canonical = canonical_utf8(&source_path)?;
        if !canonical.starts_with(root) {
            return Err(BuildError::PathEscapesRoot(source_path.to_string()));
        }
        let file_name = source_path.file_name().ok_or(BuildError::NonUtf8Path)?;
        let destination_path = destination.join(file_name);
        if metadata.is_dir() {
            copied += copy_dir_recursive_inner(root, &source_path, &destination_path)?;
        } else {
            copy_file_no_symlink(&canonical, &destination_path)?;
            copied += 1;
        }
    }
    Ok(copied)
}

fn write_macos_plist(
    config: &VestyConfig,
    contents_dir: &Utf8Path,
    executable: &str,
) -> Result<(), BuildError> {
    let bundle_id = config
        .package
        .as_ref()
        .and_then(|package| package.bundle_id.clone())
        .unwrap_or_else(|| fallback_bundle_id(executable));

    let mut dict = plist::Dictionary::new();
    dict.insert("CFBundleDevelopmentRegion".into(), "en".into());
    dict.insert("CFBundleExecutable".into(), executable.into());
    dict.insert("CFBundleIdentifier".into(), bundle_id.into());
    dict.insert("CFBundleName".into(), config.plugin.name.clone().into());
    dict.insert("CFBundlePackageType".into(), "BNDL".into());
    dict.insert(
        "CFBundleShortVersionString".into(),
        config.plugin.version.clone().into(),
    );
    dict.insert(
        "CFBundleVersion".into(),
        config.plugin.version.clone().into(),
    );
    let mut bytes = Vec::new();
    plist::to_writer_xml(&mut bytes, &dict)?;
    write_bytes_file_no_symlink(&contents_dir.join("Info.plist"), &bytes)?;
    Ok(())
}

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

fn validate_moduleinfo(moduleinfo: &ModuleInfo) -> Result<(), BuildError> {
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

fn validate_platform_binary_names(
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

fn platform_binary_dir(contents_dir: &Utf8Path, platform: BundlePlatform) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => contents_dir.join("MacOS"),
        BundlePlatform::WindowsX64 => contents_dir.join("x86_64-win"),
        BundlePlatform::LinuxX64 => contents_dir.join("x86_64-linux"),
    }
}

fn platform_binary_path(
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

fn platform_label(platform: BundlePlatform) -> &'static str {
    match platform {
        BundlePlatform::Macos => "macOS",
        BundlePlatform::WindowsX64 => "Windows x64",
        BundlePlatform::LinuxX64 => "Linux x64",
    }
}

fn validate_bundle_binary_formats(
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

fn infer_bundle_binary_platform(
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

fn validate_binary_format(path: &Utf8Path, platform: BundlePlatform) -> Result<(), BuildError> {
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

fn binary_format_matches(bytes: &[u8], platform: BundlePlatform) -> bool {
    match platform {
        BundlePlatform::Macos => macos_binary_format_matches(bytes),
        BundlePlatform::WindowsX64 => windows_x64_binary_format_matches(bytes),
        BundlePlatform::LinuxX64 => linux_x64_binary_format_matches(bytes),
    }
}

fn macos_binary_format_matches(bytes: &[u8]) -> bool {
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

fn windows_x64_binary_format_matches(bytes: &[u8]) -> bool {
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

fn linux_x64_binary_format_matches(bytes: &[u8]) -> bool {
    if bytes.len() < 20 || !bytes.starts_with(b"\x7fELF") || bytes[4] != 2 {
        return false;
    }
    match bytes[5] {
        1 => u16::from_le_bytes([bytes[18], bytes[19]]) == 0x3e,
        2 => u16::from_be_bytes([bytes[18], bytes[19]]) == 0x3e,
        _ => false,
    }
}

fn validate_bundle_binary_exports(
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

fn inspect_binary_exports(path: &Utf8Path, platform: BundlePlatform) -> BinaryExportCheck {
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

fn binary_export_check_from_output(
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

fn export_output_contains_symbol(output: &str, symbol: &str) -> bool {
    output.lines().any(|line| {
        line.split(|char: char| {
            !(char.is_ascii_alphanumeric() || matches!(char, '_' | '@' | '$' | '.'))
        })
        .any(|token| token == symbol)
    })
}

fn output_message(label: &str, bytes: &[u8]) -> String {
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

fn export_symbol_tools(
    platform: BundlePlatform,
) -> &'static [vesty_vst3_sys::BinaryExportInspectionToolPlan] {
    vesty_vst3_sys::binary_export_inspection_tools(platform_slug(platform))
        .expect("all bundle platforms have VST3 binary export inspection tools")
}

fn required_export_symbols(platform: BundlePlatform) -> &'static [&'static str] {
    vesty_vst3_sys::required_binary_export_tool_symbols(platform_slug(platform))
        .expect("all bundle platforms have required VST3 binary export symbols")
}

fn platform_slug(platform: BundlePlatform) -> &'static str {
    match platform {
        BundlePlatform::Macos => "macos",
        BundlePlatform::WindowsX64 => "windows-x64",
        BundlePlatform::LinuxX64 => "linux-x64",
    }
}

fn validate_macos_metadata(
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

fn required_plist_string<'a>(
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

fn is_safe_bundle_executable_name(executable: &str) -> bool {
    !executable.contains('/')
        && !executable.contains('\\')
        && executable != "."
        && executable != ".."
        && !executable.trim().is_empty()
}

fn collect_bundle_binaries(contents_dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>, BuildError> {
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

fn collect_files_matching(
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

fn validate_asset_manifest_files(
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

fn is_safe_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(is_safe_manifest_segment)
}

fn is_safe_manifest_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'%' | b'?' | b'#' | b':'))
}

fn validate_asset_manifest_mime(path: &str, mime: &str) -> Result<(), BuildError> {
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

fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sanitize_bundle_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-')
        .collect();
    if sanitized.is_empty() {
        "VestyPlugin".to_string()
    } else {
        sanitized
    }
}

fn fallback_bundle_id(executable: &str) -> String {
    let mut segment = String::new();
    for byte in executable.bytes() {
        let byte = byte.to_ascii_lowercase();
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            segment.push(byte as char);
        } else if byte == b'_' {
            segment.push('-');
        }
    }
    let segment = segment.trim_matches('-');
    if segment.is_empty() {
        "dev.vesty.plugin".to_string()
    } else {
        format!("dev.vesty.{segment}")
    }
}

fn serde_json_pretty<T: Serialize>(value: &T) -> Result<String, BuildError> {
    serde_json::to_string_pretty(value).map_err(BuildError::JsonSerialize)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .map(|path| path.strip_prefix(&bundle_dir).unwrap().as_str().to_string())
            .collect::<Vec<_>>();
        binary_paths.sort();

        assert_eq!(
            binary_paths,
            vec![
                "Contents/MacOS/Gain",
                "Contents/x86_64-linux/Gain.so",
                "Contents/x86_64-win/Gain.vst3",
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
        let error =
            serde_json::from_value::<ParameterManifest>(unknown_manifest_field).unwrap_err();
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
            packaged_path
                .strip_prefix(&report.bundle_dir)
                .unwrap()
                .as_str(),
            "Contents/Resources/parameters.manifest.json"
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
        let mut manifest =
            ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
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
        let path =
            Utf8PathBuf::from_path_buf(dir.path().join("instrument-sidechain.toml")).unwrap();
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

            let error =
                package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

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

            let error =
                package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

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

            let error =
                package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

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
            let error =
                package_fixture_with_config_result(BundlePlatform::Macos, config).unwrap_err();

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
        let mut moduleinfo = serde_json::from_str::<ModuleInfo>(
            &fs::read_to_string(&report.moduleinfo_path).unwrap(),
        )
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
        let mut moduleinfo = serde_json::from_str::<ModuleInfo>(
            &fs::read_to_string(&report.moduleinfo_path).unwrap(),
        )
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
            report
                .binary_path
                .strip_prefix(&report.bundle_dir)
                .unwrap()
                .as_str(),
            binary_relative
        );
        assert!(report.moduleinfo_path.is_file());
        assert_eq!(
            report
                .moduleinfo_path
                .strip_prefix(&report.bundle_dir)
                .unwrap()
                .as_str(),
            "Contents/Resources/moduleinfo.json"
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
                Utf8PathBuf::from_path_buf(dir.path().join(format!("external-{label}.txt")))
                    .unwrap();
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
            Utf8PathBuf::from_path_buf(dir.path().join("external-parameters.manifest.json"))
                .unwrap();
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
            Utf8PathBuf::from_path_buf(dir.path().join("external-parameters.manifest.json"))
                .unwrap();
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

    fn test_parameter_manifest() -> ParameterManifest {
        ParameterManifest::from_param_specs(vec![vesty_params::ParamSpec::float(
            "gain", "Gain", 0.0, 1.0, 0.5,
        )])
        .unwrap()
    }
}
