use std::collections::BTreeMap;
use std::path::Path;

use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SdkHeaderProbeError {
    MissingEnv,
}

pub fn probe_sdk_headers_from_env() -> Result<SdkHeaderProbe, SdkHeaderProbeError> {
    let root = std::env::var_os(VST3_SDK_DIR_ENV).ok_or(SdkHeaderProbeError::MissingEnv)?;
    Ok(probe_sdk_headers(root))
}

pub fn probe_sdk_headers(root: impl AsRef<Path>) -> SdkHeaderProbe {
    let root = root.as_ref().to_path_buf();
    let mut present_headers = Vec::new();
    let mut missing_headers = Vec::new();
    for header in REQUIRED_GENERATED_HEADER_INPUTS {
        if path_is_regular_file_no_symlink(&root.join(header)) {
            present_headers.push(*header);
        } else {
            missing_headers.push(*header);
        }
    }

    SdkHeaderProbe {
        version_hint: sdk_version_hint(&root),
        root,
        baseline: STEINBERG_VST3_SDK_BASELINE,
        present_headers,
        missing_headers,
    }
}

pub fn sdk_header_input_manifest(
    root: impl AsRef<Path>,
) -> Result<SdkHeaderInputManifest, SdkHeaderManifestError> {
    let root = root.as_ref();
    let probe = probe_sdk_headers(root);
    let mut headers = Vec::new();
    let mut missing_headers = Vec::new();

    for relative in REQUIRED_GENERATED_HEADER_INPUTS {
        let path = root.join(relative);
        let Some((metadata, bytes)) = sdk_header_bytes_no_symlink(&path)? else {
            missing_headers.push((*relative).to_string());
            continue;
        };
        headers.push(SdkHeaderInput {
            path: (*relative).to_string(),
            size: metadata.len(),
            sha256: sha256_hex(&bytes),
        });
    }

    Ok(SdkHeaderInputManifest {
        version: SDK_HEADER_MANIFEST_VERSION,
        generator: SDK_HEADER_MANIFEST_GENERATOR.to_string(),
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        complete: missing_headers.is_empty(),
        version_hint: probe.version_hint,
        headers,
        missing_headers,
    })
}

pub(crate) fn sdk_header_bytes_no_symlink(
    path: &Path,
) -> Result<Option<(std::fs::Metadata, Vec<u8>)>, SdkHeaderManifestError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(SdkHeaderManifestError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(SdkHeaderManifestError::InvalidHeaderInput {
            path: path.to_path_buf(),
            reason: "header input must be a regular file, not a symlink".to_string(),
        });
    }
    if !file_type.is_file() {
        return Err(SdkHeaderManifestError::InvalidHeaderInput {
            path: path.to_path_buf(),
            reason: "header input must be a regular file".to_string(),
        });
    }
    let bytes = std::fs::read(path).map_err(|source| SdkHeaderManifestError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some((metadata, bytes)))
}

pub(crate) fn path_is_regular_file_no_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        let file_type = metadata.file_type();
        file_type.is_file() && !file_type.is_symlink()
    })
}

pub fn check_sdk_header_input_manifest(
    root: impl AsRef<Path>,
    expected: &SdkHeaderInputManifest,
) -> Result<SdkHeaderInputManifest, SdkHeaderManifestError> {
    let actual = sdk_header_input_manifest(root)?;
    let differences = sdk_header_manifest_differences(expected, &actual);
    if differences.is_empty() {
        Ok(actual)
    } else {
        Err(SdkHeaderManifestError::Drift { differences })
    }
}

pub fn sdk_header_manifest_differences(
    expected: &SdkHeaderInputManifest,
    actual: &SdkHeaderInputManifest,
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
    if expected.complete != actual.complete {
        differences.push(format!(
            "complete expected {} actual {}",
            expected.complete, actual.complete
        ));
    }
    if expected.version_hint != actual.version_hint {
        differences.push(format!(
            "version hint expected {:?} actual {:?}",
            expected.version_hint, actual.version_hint
        ));
    }
    if expected.missing_headers != actual.missing_headers {
        differences.push(format!(
            "missing headers expected {:?} actual {:?}",
            expected.missing_headers, actual.missing_headers
        ));
    }

    let expected_headers = expected
        .headers
        .iter()
        .map(|header| (header.path.as_str(), header))
        .collect::<BTreeMap<_, _>>();
    let actual_headers = actual
        .headers
        .iter()
        .map(|header| (header.path.as_str(), header))
        .collect::<BTreeMap<_, _>>();

    for path in expected_headers.keys() {
        if !actual_headers.contains_key(path) {
            differences.push(format!("header missing from actual manifest: {path}"));
        }
    }
    for path in actual_headers.keys() {
        if !expected_headers.contains_key(path) {
            differences.push(format!("unexpected header in actual manifest: {path}"));
        }
    }
    for (path, expected_header) in expected_headers {
        let Some(actual_header) = actual_headers.get(path) else {
            continue;
        };
        if expected_header.size != actual_header.size {
            differences.push(format!(
                "{path} size expected {} actual {}",
                expected_header.size, actual_header.size
            ));
        }
        if expected_header.sha256 != actual_header.sha256 {
            differences.push(format!("{path} sha256 mismatch"));
        }
    }

    differences
}
