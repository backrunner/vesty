use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdkHeaderProbe {
    pub root: PathBuf,
    pub baseline: &'static str,
    pub present_headers: Vec<&'static str>,
    pub missing_headers: Vec<&'static str>,
    pub version_hint: Option<String>,
}

impl SdkHeaderProbe {
    pub fn ready_for_generated_headers(&self) -> bool {
        self.missing_headers.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SdkHeaderInputManifest {
    pub version: u32,
    pub generator: String,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub complete: bool,
    pub version_hint: Option<String>,
    pub headers: Vec<SdkHeaderInput>,
    pub missing_headers: Vec<String>,
}

impl SdkHeaderInputManifest {
    pub fn header(&self, path: &str) -> Option<&SdkHeaderInput> {
        self.headers.iter().find(|header| header.path == path)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SdkHeaderInput {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsPlan {
    pub version: u32,
    pub generator: String,
    pub status: String,
    pub bindings_generated: bool,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub active_backend: String,
    pub sdk_dir: String,
    pub bindings_module: String,
    pub header_manifest: SdkHeaderInputManifest,
    pub checks: Vec<GeneratedBindingsPlanCheck>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsPlanCheck {
    pub name: String,
    pub status: String,
    pub value: String,
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsSurface {
    pub version: u32,
    pub generator: String,
    pub status: String,
    pub bindings_generated: bool,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub active_backend: String,
    pub sdk_dir: String,
    pub header_manifest: SdkHeaderInputManifest,
    pub required_headers: Vec<String>,
    pub missing_headers: Vec<String>,
    pub missing_symbols: Vec<String>,
    pub symbols: Vec<GeneratedBindingsSurfaceSymbol>,
    pub blockers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsSurfaceSymbol {
    pub name: String,
    pub kind: String,
    pub header: String,
    pub purpose: String,
    pub header_present: bool,
    pub symbol_present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsScaffold {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsAbiSeed {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsAbi {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsInterfaceSkeleton {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Debug)]
pub enum SdkHeaderManifestError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidHeaderInput {
        path: PathBuf,
        reason: String,
    },
    Drift {
        differences: Vec<String>,
    },
}

impl std::fmt::Display for SdkHeaderManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SdkHeaderManifestError::Io { path, source } => {
                write!(formatter, "{}: {source}", path.display())
            }
            SdkHeaderManifestError::InvalidHeaderInput { path, reason } => {
                write!(formatter, "{}: {reason}", path.display())
            }
            SdkHeaderManifestError::Drift { differences } => {
                write!(
                    formatter,
                    "SDK header manifest drift: {}",
                    differences.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for SdkHeaderManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SdkHeaderManifestError::Io { source, .. } => Some(source),
            SdkHeaderManifestError::InvalidHeaderInput { .. }
            | SdkHeaderManifestError::Drift { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum GeneratedBindingsScaffoldError {
    Manifest(SdkHeaderManifestError),
    Blocked { blockers: Vec<String> },
}

impl std::fmt::Display for GeneratedBindingsScaffoldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneratedBindingsScaffoldError::Manifest(error) => write!(formatter, "{error}"),
            GeneratedBindingsScaffoldError::Blocked { blockers } => write!(
                formatter,
                "generated bindings scaffold is blocked: {}",
                blockers.join("; ")
            ),
        }
    }
}

impl std::error::Error for GeneratedBindingsScaffoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GeneratedBindingsScaffoldError::Manifest(error) => Some(error),
            GeneratedBindingsScaffoldError::Blocked { .. } => None,
        }
    }
}

impl From<SdkHeaderManifestError> for GeneratedBindingsScaffoldError {
    fn from(error: SdkHeaderManifestError) -> Self {
        GeneratedBindingsScaffoldError::Manifest(error)
    }
}
