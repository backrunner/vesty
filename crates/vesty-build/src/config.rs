use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{normalize_class_id, plugin_kind_category, read_text_file_no_symlink};

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

pub(crate) fn validate_required_config_field(name: &str, value: &str) -> Result<(), BuildError> {
    if value.trim().is_empty() {
        return Err(BuildError::InvalidConfig(format!(
            "{name} must not be empty"
        )));
    }
    validate_no_control_chars(name, value).map_err(BuildError::InvalidConfig)?;
    Ok(())
}

pub(crate) fn validate_optional_config_field(
    name: &str,
    value: &Option<String>,
) -> Result<(), BuildError> {
    if let Some(value) = value {
        validate_required_config_field(name, value)?;
    }
    Ok(())
}

pub(crate) fn validate_optional_config_text(
    name: &str,
    value: &Option<String>,
) -> Result<(), BuildError> {
    if let Some(value) = value {
        validate_no_control_chars(name, value).map_err(BuildError::InvalidConfig)?;
    }
    Ok(())
}

pub(crate) fn validate_no_control_chars(name: &str, value: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!("{name} must not contain control characters"));
    }
    Ok(())
}

pub(crate) fn validate_bundle_identifier(name: &str, value: &str) -> Result<(), BuildError> {
    validate_bundle_identifier_shape(name, value).map_err(BuildError::InvalidConfig)
}

pub(crate) fn validate_bundle_identifier_shape(name: &str, value: &str) -> Result<(), String> {
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

pub(crate) fn validate_ui_config(ui: &UiConfig) -> Result<(), BuildError> {
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

pub(crate) fn validate_dimension_pair(
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
