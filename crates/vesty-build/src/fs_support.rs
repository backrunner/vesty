use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

use crate::BuildError;

pub(crate) fn canonical_utf8(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
    Utf8PathBuf::from_path_buf(path.canonicalize()?).map_err(|_| BuildError::NonUtf8Path)
}

pub(crate) fn require_real_directory(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
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

pub(crate) fn real_directory_exists_no_symlink(path: &Utf8Path) -> Result<bool, BuildError> {
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

pub(crate) fn existing_directory_no_parent_or_leaf_symlink(
    path: &Utf8Path,
) -> Result<bool, BuildError> {
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

pub(crate) fn create_directory_no_parent_or_leaf_symlink(
    path: &Utf8Path,
) -> Result<(), BuildError> {
    if existing_directory_no_parent_or_leaf_symlink(path)? {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    require_real_directory(path)?;
    Ok(())
}

pub(crate) fn remove_existing_output_directory(path: &Utf8Path) -> Result<(), BuildError> {
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

pub(crate) fn reject_existing_output_file_symlink(path: &Utf8Path) -> Result<(), BuildError> {
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

pub(crate) fn write_text_file_no_symlink(path: &Utf8Path, text: &str) -> Result<(), BuildError> {
    write_bytes_file_no_symlink(path, text.as_bytes())
}

pub(crate) fn write_bytes_file_no_symlink(path: &Utf8Path, bytes: &[u8]) -> Result<(), BuildError> {
    reject_existing_output_file_symlink(path)?;
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        create_directory_no_parent_or_leaf_symlink(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

pub(crate) fn copy_file_no_symlink(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<(), BuildError> {
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

pub(crate) fn reject_existing_output_parent_symlink(path: &Utf8Path) -> Result<(), BuildError> {
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

pub(crate) fn path_exists_no_symlink(path: &Utf8Path) -> Result<bool, BuildError> {
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

pub(crate) fn require_real_file(path: &Utf8Path) -> Result<Utf8PathBuf, BuildError> {
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

pub(crate) fn read_text_file_no_symlink(path: &Utf8Path) -> Result<String, BuildError> {
    let canonical = require_real_file(path)?;
    fs::read_to_string(canonical).map_err(BuildError::Io)
}

pub(crate) fn read_bytes_file_no_symlink(path: &Utf8Path) -> Result<Vec<u8>, BuildError> {
    let canonical = require_real_file(path)?;
    fs::read(canonical).map_err(BuildError::Io)
}
