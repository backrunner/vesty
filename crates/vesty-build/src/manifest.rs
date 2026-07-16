use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use vesty_params::{
    ParamSpec, VST3_PARAM_ID_ALGORITHM, stable_vst3_param_id, validate_param_specs,
};

use crate::{BuildError, BundlePlatform, read_text_file_no_symlink};

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
