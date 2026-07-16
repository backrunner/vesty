use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{borrow::Cow, collections::BTreeMap, path::PathBuf};

#[cfg(feature = "wry-backend")]
pub(crate) fn asset_response(
    root: &std::path::Path,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
    request_path: &str,
) -> wry::http::Response<Cow<'static, [u8]>> {
    let mime = asset_request_key(request_path)
        .ok()
        .and_then(|key| manifest.get(&key))
        .map(|entry| entry.mime.as_str())
        .unwrap_or_else(|| mime_for_path(request_path));
    match read_asset(root, request_path, manifest) {
        Ok(bytes) => {
            asset_http_response(200, mime, Cow::Owned(bytes), mime.starts_with("text/html"))
        }
        Err(_) => asset_not_found_response(),
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn asset_not_found_response() -> wry::http::Response<Cow<'static, [u8]>> {
    asset_http_response(
        404,
        "text/plain; charset=utf-8",
        Cow::Borrowed(&b"not found"[..]),
        false,
    )
}

#[cfg(feature = "wry-backend")]
pub(crate) fn asset_http_response(
    status: u16,
    content_type: &str,
    body: Cow<'static, [u8]>,
    include_csp: bool,
) -> wry::http::Response<Cow<'static, [u8]>> {
    let mut response = wry::http::Response::new(body);
    *response.status_mut() = wry::http::StatusCode::from_u16(status)
        .unwrap_or(wry::http::StatusCode::INTERNAL_SERVER_ERROR);
    let content_type = wry::http::HeaderValue::from_str(content_type)
        .unwrap_or_else(|_| wry::http::HeaderValue::from_static("application/octet-stream"));
    response.headers_mut().insert(
        wry::http::HeaderName::from_static("content-type"),
        content_type,
    );
    response.headers_mut().insert(
        wry::http::HeaderName::from_static("x-content-type-options"),
        wry::http::HeaderValue::from_static("nosniff"),
    );
    if include_csp {
        response.headers_mut().insert(
            wry::http::HeaderName::from_static("content-security-policy"),
            wry::http::HeaderValue::from_static(release_asset_csp()),
        );
    }
    response
}

#[cfg(feature = "wry-backend")]
pub(crate) fn release_asset_csp() -> &'static str {
    "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; media-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'"
}

#[cfg(feature = "wry-backend")]
pub(crate) fn release_navigation_allowed(url: String) -> bool {
    url == "about:blank" || release_asset_url_allowed(&url)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn release_ipc_allowed(url: &str) -> bool {
    release_asset_url_allowed(url)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn release_asset_url_allowed(url: &str) -> bool {
    if url.is_empty()
        || url.trim() != url
        || url
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return false;
    }

    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, ""), |(authority, path)| (authority, path));
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return false;
    }
    let allowed_origin =
        scheme.eq_ignore_ascii_case("vesty") && authority.eq_ignore_ascii_case("assets");
    if !allowed_origin {
        return false;
    }
    if path.is_empty() {
        return true;
    }
    if path == "/" {
        return true;
    }
    is_safe_runtime_manifest_path(path)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn safe_asset_path(
    root: &std::path::Path,
    request_path: &str,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
) -> std::io::Result<PathBuf> {
    let key = asset_request_key(request_path)?;
    if !manifest.contains_key(&key) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path is not listed in manifest",
        ));
    }
    let root = root.canonicalize()?;
    let relative = key.split('/').collect::<PathBuf>();
    let unresolved = root.join(relative);
    let metadata = std::fs::symlink_metadata(&unresolved)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path is not a regular file",
        ));
    }
    let candidate = unresolved.canonicalize()?;
    if candidate.starts_with(&root) {
        Ok(candidate)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path escapes root",
        ))
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn read_asset(
    root: &std::path::Path,
    request_path: &str,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
) -> std::io::Result<Vec<u8>> {
    let key = asset_request_key(request_path)?;
    let path = safe_asset_path(root, request_path, manifest)?;
    let bytes = std::fs::read(path)?;
    if let Some(entry) = manifest.get(&key) {
        verify_manifest_entry(entry, &bytes)?;
    }
    Ok(bytes)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn verify_manifest_entry(
    entry: &RuntimeAssetEntry,
    bytes: &[u8],
) -> std::io::Result<()> {
    if bytes.len() as u64 != entry.size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset size does not match manifest",
        ));
    }
    if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "asset sha256 is not a valid hex digest",
        ));
    }

    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset sha256 does not match manifest",
        ));
    }
    Ok(())
}

#[cfg(feature = "wry-backend")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeAssetManifest {
    version: u32,
    root: String,
    entry: String,
    files: Vec<RuntimeAssetFile>,
}

#[cfg(feature = "wry-backend")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeAssetFile {
    path: String,
    mime: String,
    sha256: String,
    size: u64,
}

#[cfg(feature = "wry-backend")]
#[derive(Debug)]
pub(crate) struct RuntimeAssetEntry {
    mime: String,
    sha256: String,
    size: u64,
}

#[cfg(feature = "wry-backend")]
pub(crate) fn load_asset_manifest(
    root: &std::path::Path,
) -> std::io::Result<BTreeMap<String, RuntimeAssetEntry>> {
    let root_metadata = std::fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "UI asset root is not a regular directory",
        ));
    }
    let candidates = [
        root.parent()
            .map(|parent| parent.join("assets.manifest.json")),
        Some(root.join("assets.manifest.json")),
    ];
    let mut manifest_path = None;
    for path in candidates.into_iter().flatten() {
        if let Some(path) = manifest_candidate_file(path)? {
            manifest_path = Some(path);
            break;
        }
    }
    let manifest_path = manifest_path.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "assets.manifest.json was not found next to or inside the UI assets directory",
        )
    })?;
    let text = std::fs::read_to_string(&manifest_path)?;
    let manifest = serde_json::from_str::<RuntimeAssetManifest>(&text).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "invalid asset manifest {}: {error}",
                manifest_path.display()
            ),
        )
    })?;
    runtime_asset_entries(manifest)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn manifest_candidate_file(path: PathBuf) -> std::io::Result<Option<PathBuf>> {
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "assets.manifest.json must not be a symlink",
        ));
    }
    Ok(metadata.is_file().then_some(path))
}

#[cfg(feature = "wry-backend")]
pub(crate) fn runtime_asset_entries(
    manifest: RuntimeAssetManifest,
) -> std::io::Result<BTreeMap<String, RuntimeAssetEntry>> {
    if manifest.version != 1 {
        return Err(invalid_manifest(format!(
            "unsupported asset manifest version {}",
            manifest.version
        )));
    }
    if manifest.root.trim().is_empty() || manifest.root.chars().any(char::is_control) {
        return Err(invalid_manifest("asset manifest root is invalid"));
    }
    if manifest.files.is_empty() {
        return Err(invalid_manifest("asset manifest files must not be empty"));
    }

    let entry = normalize_runtime_manifest_path(&manifest.entry)?;
    let mut entries = BTreeMap::new();
    for file in manifest.files {
        let path = normalize_runtime_manifest_path(&file.path)?;
        let mime = validate_runtime_manifest_mime(&path, &file.mime)?;
        validate_runtime_manifest_sha(&path, &file.sha256)?;
        let previous = entries.insert(
            path.clone(),
            RuntimeAssetEntry {
                mime,
                sha256: file.sha256,
                size: file.size,
            },
        );
        if previous.is_some() {
            return Err(invalid_manifest(format!(
                "asset manifest contains duplicate path: {path}"
            )));
        }
    }

    if !entries.contains_key(&entry) {
        return Err(invalid_manifest(format!(
            "asset manifest entry is missing from files: {entry}"
        )));
    }
    Ok(entries)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn normalize_runtime_manifest_path(path: &str) -> std::io::Result<String> {
    if !is_safe_runtime_manifest_path(path) {
        return Err(invalid_manifest(format!(
            "asset manifest path is not safe: {path}"
        )));
    }
    Ok(path.to_string())
}

#[cfg(feature = "wry-backend")]
pub(crate) fn is_safe_runtime_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(is_safe_runtime_manifest_segment)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn is_safe_runtime_manifest_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'%' | b'?' | b'#' | b':'))
}

#[cfg(feature = "wry-backend")]
pub(crate) fn validate_runtime_manifest_mime(path: &str, mime: &str) -> std::io::Result<String> {
    let trimmed = mime.trim();
    if trimmed.is_empty() {
        return Err(invalid_manifest(format!("asset {path} has an empty mime")));
    }
    if trimmed != mime {
        return Err(invalid_manifest(format!(
            "asset {path} mime must not have leading or trailing whitespace"
        )));
    }
    wry::http::HeaderValue::from_str(mime).map_err(|_| {
        invalid_manifest(format!(
            "asset {path} mime is not a valid HTTP header value"
        ))
    })?;
    Ok(mime.to_string())
}

#[cfg(feature = "wry-backend")]
pub(crate) fn validate_runtime_manifest_sha(path: &str, sha256: &str) -> std::io::Result<()> {
    if sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid_manifest(format!(
            "asset {path} sha256 is not a 64-byte hex digest"
        )))
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn invalid_manifest(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

#[cfg(feature = "wry-backend")]
pub(crate) fn asset_request_key(request_path: &str) -> std::io::Result<String> {
    let key = request_path.trim_start_matches('/');
    if is_safe_runtime_manifest_path(key) {
        Ok(key.to_string())
    } else {
        Err(invalid_manifest(format!(
            "asset request path is not safe: {request_path}"
        )))
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn mime_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}
