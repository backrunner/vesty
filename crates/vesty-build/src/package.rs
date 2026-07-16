use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

use crate::{
    AssetManifest, BuildError, BundlePlatform, ModuleClassInfo, ModuleInfo,
    PARAMETER_MANIFEST_FILE, PackageOptions, PackageReport, VestyConfig, binary_relative_path,
    canonical_utf8, copy_file_no_symlink, create_directory_no_parent_or_leaf_symlink,
    fallback_bundle_id, read_parameter_manifest, real_directory_exists_no_symlink,
    remove_existing_output_directory, require_real_directory, require_real_file,
    sanitize_bundle_name, serde_json_pretty, validate_binary_format, validate_config,
    write_bytes_file_no_symlink, write_text_file_no_symlink,
};

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

pub(crate) fn configured_parameter_manifest_path(
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

pub(crate) fn resolve_project_path(project_dir: &Utf8Path, path: &str) -> Utf8PathBuf {
    let path = Utf8Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_dir.join(path)
    }
}

pub(crate) fn module_category(config: &VestyConfig) -> Result<String, BuildError> {
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

pub(crate) fn plugin_kind_category(kind: &str) -> Result<&'static str, BuildError> {
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

pub(crate) fn copy_dir_recursive(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<usize, BuildError> {
    let root = require_real_directory(source)?;
    copy_dir_recursive_inner(&root, source, destination)
}

pub(crate) fn copy_dir_recursive_inner(
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

pub(crate) fn write_macos_plist(
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
