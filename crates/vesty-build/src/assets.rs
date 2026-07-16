use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{
    BuildError, canonical_utf8, is_safe_manifest_path, require_real_directory, sha256_hex,
};

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

pub(crate) fn collect_assets(
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
