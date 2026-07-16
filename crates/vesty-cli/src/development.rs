use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DevInstallMode {
    Copy,
    Symlink,
}

pub(super) fn parse_dev_install_mode(
    mode: &str,
) -> Result<DevInstallMode, Box<dyn std::error::Error>> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "copy" => Ok(DevInstallMode::Copy),
        "symlink" | "link" => Ok(DevInstallMode::Symlink),
        _ => Err(format!("unsupported install mode '{mode}'; expected copy or symlink").into()),
    }
}

pub(super) fn default_user_vst3_dir() -> Option<Utf8PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(Utf8PathBuf::from)
            .map(|home| home.join("Library/Audio/Plug-Ins/VST3"))
    } else if cfg!(target_os = "windows") {
        std::env::var("CommonProgramFiles")
            .ok()
            .map(Utf8PathBuf::from)
            .map(|root| root.join("VST3"))
    } else if cfg!(target_os = "linux") {
        std::env::var("HOME")
            .ok()
            .map(Utf8PathBuf::from)
            .map(|home| home.join(".vst3"))
    } else {
        None
    }
}

pub(super) fn install_dev_bundle_from_options(
    bundle_dir: &Utf8Path,
    vst3_dir: Option<Utf8PathBuf>,
    install_mode: &str,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let mode = parse_dev_install_mode(install_mode)?;
    let target_dir = match vst3_dir {
        Some(dir) => dir,
        None => default_user_vst3_dir()
            .ok_or("could not determine default user VST3 directory; pass --vst3-dir")?,
    };
    install_dev_bundle(bundle_dir, &target_dir, mode)
}

pub(super) fn install_dev_bundle(
    bundle_dir: &Utf8Path,
    vst3_dir: &Utf8Path,
    mode: DevInstallMode,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if bundle_dir.extension() != Some("vst3") {
        return Err(format!("dev install source must be a .vst3 directory: {bundle_dir}").into());
    }
    require_existing_directory_no_symlink("dev install source", bundle_dir)?;
    let name = bundle_dir
        .file_name()
        .ok_or("dev install source has no bundle file name")?;
    create_directory_no_parent_or_leaf_symlink("dev install directory", vst3_dir)?;
    let destination = vst3_dir.join(name);
    remove_existing_dev_bundle(&destination)?;
    match mode {
        DevInstallMode::Copy => copy_bundle_dir(bundle_dir, &destination)?,
        DevInstallMode::Symlink => symlink_bundle_dir(bundle_dir, &destination)?,
    }
    Ok(destination)
}

pub(super) fn remove_existing_dev_bundle(
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub(super) fn copy_bundle_dir(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    create_directory_no_parent_or_leaf_symlink("dev install bundle directory", destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "bundle path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("dev install refuses symlink in bundle: {source_path}").into());
        }
        let destination_path = destination.join(
            source_path
                .file_name()
                .ok_or("bundle entry has no file name")?,
        );
        if metadata.is_dir() {
            copy_bundle_dir(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            reject_existing_path_symlink("dev install bundle file", &destination_path)?;
            reject_existing_output_parent_symlink("dev install bundle file", &destination_path)?;
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn symlink_bundle_dir(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::os::unix::fs::symlink(source, destination)?;
    Ok(())
}

#[cfg(windows)]
pub(super) fn symlink_bundle_dir(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::os::windows::fs::symlink_dir(source, destination)?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn symlink_bundle_dir(
    _source: &Utf8Path,
    _destination: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("symlink install mode is unsupported on this platform".into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CommandSpec {
    pub(super) program: String,
    pub(super) args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NotarizationCredentials {
    KeychainProfile(String),
    AppleId {
        apple_id: String,
        team_id: String,
        password: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NotarizationPlan {
    pub(super) archive: Utf8PathBuf,
    pub(super) commands: Vec<CommandSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NotarizeOptions {
    pub(super) bundle: Utf8PathBuf,
    pub(super) keychain_profile: Option<String>,
    pub(super) apple_id: Option<String>,
    pub(super) team_id: Option<String>,
    pub(super) password: Option<String>,
    pub(super) archive: Option<Utf8PathBuf>,
    pub(super) wait: bool,
    pub(super) staple: bool,
}

pub(super) fn bundle_signing_command(
    platform: BundlePlatform,
    bundle_dir: &Utf8Path,
    binary_path: &Utf8Path,
    identity: &str,
) -> Result<CommandSpec, Box<dyn std::error::Error>> {
    let identity = identity.trim();
    if identity.is_empty() {
        return Err("package signing identity cannot be empty".into());
    }

    match platform {
        BundlePlatform::Macos => {
            let program = find_command("codesign", &[Utf8PathBuf::from("/usr/bin/codesign")])
                .map(|path| path.to_string())
                .unwrap_or_else(|| "codesign".to_string());
            Ok(CommandSpec {
                program,
                args: vec![
                    "--force".to_string(),
                    "--deep".to_string(),
                    "--options".to_string(),
                    "runtime".to_string(),
                    "--timestamp".to_string(),
                    "--sign".to_string(),
                    identity.to_string(),
                    portable_report_path(bundle_dir),
                ],
            })
        }
        BundlePlatform::WindowsX64 => {
            let program = find_command("signtool.exe", &windows_signtool_candidates())
                .map(|path| path.to_string())
                .unwrap_or_else(|| "signtool.exe".to_string());
            Ok(CommandSpec {
                program,
                args: vec![
                    "sign".to_string(),
                    "/fd".to_string(),
                    "SHA256".to_string(),
                    "/td".to_string(),
                    "SHA256".to_string(),
                    "/tr".to_string(),
                    "http://timestamp.digicert.com".to_string(),
                    "/n".to_string(),
                    identity.to_string(),
                    portable_report_path(binary_path),
                ],
            })
        }
        BundlePlatform::LinuxX64 => Err(
            "Linux VST3 bundle signing is release-channel specific; sign distro packages or installers outside `vesty package`"
                .into(),
        ),
    }
}

pub(super) fn signing_verification_command(
    platform: BundlePlatform,
    bundle_dir: &Utf8Path,
    binary_path: Option<&Utf8Path>,
    tool: Option<&Utf8Path>,
) -> Result<CommandSpec, Box<dyn std::error::Error>> {
    if bundle_dir.extension() != Some("vst3") {
        return Err(
            format!("signing verification target must be a .vst3 bundle: {bundle_dir}").into(),
        );
    }
    require_existing_directory_no_symlink("signing verification target", bundle_dir)?;

    match platform {
        BundlePlatform::Macos => {
            if let Some(binary_path) = binary_path {
                return Err(format!(
                    "macOS signing verification checks the .vst3 bundle; --binary is only supported for windows-x64: {binary_path}"
                )
                .into());
            }
            let program = signing_verification_program(
                tool,
                "codesign",
                &[Utf8PathBuf::from("/usr/bin/codesign")],
            )?;
            Ok(CommandSpec {
                program,
                args: vec![
                    "--verify".to_string(),
                    "--deep".to_string(),
                    "--strict".to_string(),
                    "--verbose=2".to_string(),
                    portable_report_path(bundle_dir),
                ],
            })
        }
        BundlePlatform::WindowsX64 => {
            let binary_path = match binary_path {
                Some(path) => path.to_path_buf(),
                None => infer_windows_bundle_binary(bundle_dir)?,
            };
            require_windows_signing_binary_in_bundle(bundle_dir, &binary_path)?;
            let program =
                signing_verification_program(tool, "signtool.exe", &windows_signtool_candidates())?;
            Ok(CommandSpec {
                program,
                args: vec![
                    "verify".to_string(),
                    "/pa".to_string(),
                    "/v".to_string(),
                    portable_report_path(&binary_path),
                ],
            })
        }
        BundlePlatform::LinuxX64 => Err(
            "Linux VST3 signing evidence is release-channel specific; collect distro/package signing evidence outside `vesty release-evidence collect-signing`"
                .into(),
        ),
    }
}

pub(super) fn signing_verification_program(
    tool: Option<&Utf8Path>,
    default_binary: &str,
    candidates: &[Utf8PathBuf],
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(tool) = tool {
        reject_existing_output_parent_symlink("signing verification tool", tool)?;
        reject_existing_path_symlink("signing verification tool", tool)?;
        return Ok(tool.to_string());
    }
    Ok(find_command(default_binary, candidates)
        .map(|path| path.to_string())
        .unwrap_or_else(|| default_binary.to_string()))
}

pub(super) fn require_windows_signing_binary_in_bundle(
    bundle_dir: &Utf8Path,
    binary_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    require_existing_file_no_symlink("Windows signing verification binary", binary_path)?;
    if !binary_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
    {
        return Err(format!(
            "Windows signing verification binary must be a .vst3 file: {binary_path}"
        )
        .into());
    }

    let platform_dir = bundle_dir.join("Contents/x86_64-win");
    require_existing_directory_no_symlink("Windows signing payload directory", &platform_dir)?;
    let platform_dir = canonicalize_utf8(&platform_dir)?;
    let binary_path = canonicalize_utf8(binary_path)?;
    if !binary_path.starts_with(&platform_dir) {
        return Err(format!(
            "Windows signing verification binary must be inside bundle Contents/x86_64-win: {binary_path}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn infer_windows_bundle_binary(
    bundle_dir: &Utf8Path,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let platform_dir = bundle_dir.join("Contents/x86_64-win");
    if !existing_directory_no_symlink("Windows signing payload directory", &platform_dir)? {
        return Err(format!(
            "Windows signing verification needs Contents/x86_64-win in bundle or explicit --binary: {bundle_dir}"
        )
        .into());
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(&platform_dir)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "Windows signing binary path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("Windows signing binary must not be a symlink: {path}").into());
        }
        if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
        {
            candidates.push(path);
        }
    }
    candidates.sort();
    match candidates.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!("no Windows .vst3 platform binary found under {platform_dir}").into()),
        _ => Err(format!(
            "multiple Windows .vst3 platform binaries found under {platform_dir}; pass --binary"
        )
        .into()),
    }
}

pub(super) fn run_signing_command(command: &CommandSpec) -> Result<(), Box<dyn std::error::Error>> {
    run_command_spec(command, "signing")
}

pub(super) fn run_command_spec(
    command: &CommandSpec,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} command failed: {} {}",
            command.program,
            command.args.join(" ")
        )
        .into())
    }
}

pub(super) fn run_command_spec_capture(
    command: &CommandSpec,
    label: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .output()?;
    if output.status.success() {
        Ok(output)
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{label} command failed: {} {}\nstdout:\n{}\nstderr:\n{}",
            command.program,
            command.args.join(" "),
            stdout.trim_end(),
            stderr.trim_end()
        )
        .into())
    }
}

pub(super) fn captured_command_log(command: &CommandSpec, output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut text = format!(
        "command={} {}\nstatus={}\n",
        command.program,
        command.args.join(" "),
        output.status
    );
    if !stdout.is_empty() {
        text.push_str("\n[stdout]\n");
        text.push_str(&stdout);
        if !stdout.ends_with('\n') {
            text.push('\n');
        }
    }
    if !stderr.is_empty() {
        text.push_str("\n[stderr]\n");
        text.push_str(&stderr);
        if !stderr.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}

pub(super) fn run_notarize(options: NotarizeOptions) -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(target_os = "macos") {
        return Err("notarization requires macOS with xcrun notarytool".into());
    }
    if !options.bundle.is_dir() || options.bundle.extension() != Some("vst3") {
        return Err(format!(
            "notarization bundle must be a .vst3 directory: {}",
            options.bundle
        )
        .into());
    }
    let credentials = notarization_credentials(
        options.keychain_profile.as_deref(),
        options.apple_id.as_deref(),
        options.team_id.as_deref(),
        options.password.as_deref(),
    )?;
    let plan = notarization_plan(
        &options.bundle,
        options.archive.as_deref(),
        &credentials,
        options.wait,
        options.staple,
    )?;
    for command in &plan.commands {
        run_command_spec(command, "notarization")?;
    }
    println!("notarization archive: {}", plan.archive);
    if options.staple {
        println!("stapled: {}", options.bundle);
    }
    Ok(())
}

pub(super) fn notarization_credentials(
    keychain_profile: Option<&str>,
    apple_id: Option<&str>,
    team_id: Option<&str>,
    password: Option<&str>,
) -> Result<NotarizationCredentials, Box<dyn std::error::Error>> {
    let keychain_profile = keychain_profile
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let apple_id = apple_id.map(str::trim).filter(|value| !value.is_empty());
    let team_id = team_id.map(str::trim).filter(|value| !value.is_empty());
    let password = password.map(str::trim).filter(|value| !value.is_empty());

    if let Some(profile) = keychain_profile {
        if apple_id.is_some() || team_id.is_some() || password.is_some() {
            return Err(
                "use either --keychain-profile or --apple-id/--team-id/--password, not both".into(),
            );
        }
        return Ok(NotarizationCredentials::KeychainProfile(
            profile.to_string(),
        ));
    }

    match (apple_id, team_id, password) {
        (Some(apple_id), Some(team_id), Some(password)) => Ok(NotarizationCredentials::AppleId {
            apple_id: apple_id.to_string(),
            team_id: team_id.to_string(),
            password: password.to_string(),
        }),
        _ => {
            Err("provide --keychain-profile or all of --apple-id, --team-id and --password".into())
        }
    }
}

pub(super) fn notarization_plan(
    bundle: &Utf8Path,
    archive: Option<&Utf8Path>,
    credentials: &NotarizationCredentials,
    wait: bool,
    staple: bool,
) -> Result<NotarizationPlan, Box<dyn std::error::Error>> {
    if !wait && staple {
        return Err("--no-wait cannot be used with stapling; pass --no-staple".into());
    }

    let archive = archive
        .map(Utf8Path::to_path_buf)
        .unwrap_or_else(|| Utf8PathBuf::from(format!("{bundle}.zip")));
    let ditto = find_command("ditto", &[Utf8PathBuf::from("/usr/bin/ditto")])
        .map(|path| path.to_string())
        .unwrap_or_else(|| "ditto".to_string());
    let xcrun = find_command("xcrun", &[Utf8PathBuf::from("/usr/bin/xcrun")])
        .map(|path| path.to_string())
        .unwrap_or_else(|| "xcrun".to_string());

    let mut commands = Vec::new();
    commands.push(CommandSpec {
        program: ditto,
        args: vec![
            "-c".to_string(),
            "-k".to_string(),
            "--keepParent".to_string(),
            bundle.to_string(),
            archive.to_string(),
        ],
    });

    let mut submit_args = vec![
        "notarytool".to_string(),
        "submit".to_string(),
        archive.to_string(),
    ];
    if wait {
        submit_args.push("--wait".to_string());
    }
    match credentials {
        NotarizationCredentials::KeychainProfile(profile) => {
            submit_args.extend(["--keychain-profile".to_string(), profile.clone()]);
        }
        NotarizationCredentials::AppleId {
            apple_id,
            team_id,
            password,
        } => {
            submit_args.extend([
                "--apple-id".to_string(),
                apple_id.clone(),
                "--team-id".to_string(),
                team_id.clone(),
                "--password".to_string(),
                password.clone(),
            ]);
        }
    }
    commands.push(CommandSpec {
        program: xcrun.clone(),
        args: submit_args,
    });

    if staple {
        commands.push(CommandSpec {
            program: xcrun,
            args: vec![
                "stapler".to_string(),
                "staple".to_string(),
                bundle.to_string(),
            ],
        });
    }

    Ok(NotarizationPlan { archive, commands })
}

#[derive(Clone, Debug)]
pub(super) struct DevOptions {
    pub(super) config_path: Utf8PathBuf,
    pub(super) release: bool,
    pub(super) no_ui: bool,
    pub(super) ui_command: Option<String>,
    pub(super) install_dev: bool,
    pub(super) binary: Option<Utf8PathBuf>,
    pub(super) platform: Option<String>,
    pub(super) output_dir: Utf8PathBuf,
    pub(super) vst3_dir: Option<Utf8PathBuf>,
    pub(super) install_mode: String,
}

pub(super) fn run_dev(options: DevOptions) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = options
        .config_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| Utf8PathBuf::from("."));
    let config = read_config(&options.config_path)?;

    println!("dev: {}", config.plugin.name);
    let mut cargo = Command::new("cargo");
    cargo.current_dir(&project_dir).arg("build");
    if options.release {
        cargo.arg("--release");
    }
    let status = cargo.status()?;
    if !status.success() {
        return Err("cargo build failed".into());
    }

    if options.install_dev {
        let binary =
            resolve_dev_binary_path(options.binary.as_ref(), &project_dir, options.release)?;
        let platform = resolve_bundle_platform(options.platform.as_deref())?;
        let report = package_vst3(
            &config,
            &PackageOptions {
                project_dir: project_dir.clone(),
                output_dir: options.output_dir,
                platform,
                binary_path: binary,
            },
        )?;
        let installed = install_dev_bundle_from_options(
            &report.bundle_dir,
            options.vst3_dir,
            &options.install_mode,
        )?;
        println!("dev bundle: {}", report.bundle_dir);
        println!("installed dev bundle: {installed}");
    }

    let Some(ui) = config.ui else {
        println!("ui: none");
        return Ok(());
    };
    if options.no_ui {
        println!("ui: skipped");
        return Ok(());
    }

    let ui_dir = project_dir.join(&ui.dir);
    if !ui_dir.is_dir() {
        return Err(format!("ui directory does not exist: {ui_dir}").into());
    }
    if let Some(dev_url) = &ui.dev_url {
        println!("ui dev url: {dev_url}");
    }

    let command = options.ui_command.or_else(|| {
        ui_dir
            .join("package.json")
            .is_file()
            .then(|| "npm run dev".to_string())
    });
    let Some(command) = command else {
        println!("ui dev server: no package.json found in {ui_dir}");
        return Ok(());
    };

    println!("ui dev server: {command}");
    run_shell(&command, Some(&ui_dir))
}

pub(super) fn resolve_dev_binary_path(
    explicit: Option<&Utf8PathBuf>,
    project_dir: &Utf8Path,
    release: bool,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if let Some(binary) = explicit {
        return Ok(binary.clone());
    }

    let metadata = cargo_metadata_json(project_dir)?;
    let manifest_path = canonical_manifest_path(project_dir)?;
    let target_name = cdylib_target_name_from_metadata(&metadata, manifest_path.as_deref())?;
    let target_dir = metadata
        .get("target_directory")
        .and_then(serde_json::Value::as_str)
        .ok_or("cargo metadata did not report target_directory")?;
    let profile = if release { "release" } else { "debug" };
    Ok(Utf8PathBuf::from(target_dir)
        .join(profile)
        .join(cdylib_filename(target_name)))
}

pub(super) fn cargo_metadata_json(
    project_dir: &Utf8Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .current_dir(project_dir)
        .args(["metadata", "--no-deps", "--format-version=1"])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {}", stderr.trim()).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub(super) fn canonical_manifest_path(
    project_dir: &Utf8Path,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let manifest_path = project_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let canonical = fs::canonicalize(&manifest_path)?;
    Ok(Some(Utf8PathBuf::from_path_buf(canonical).map_err(
        |_| format!("Cargo manifest path is not UTF-8: {manifest_path}"),
    )?))
}

pub(super) fn cdylib_target_name_from_metadata<'a>(
    metadata: &'a serde_json::Value,
    manifest_path: Option<&Utf8Path>,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not report packages")?;
    let package = cdylib_package_from_metadata(metadata, packages, manifest_path)?;
    let targets = package
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata package did not report targets")?;
    targets
        .iter()
        .find(|target| {
            target
                .get("kind")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|kind| kind.iter().any(|value| value.as_str() == Some("cdylib")))
        })
        .and_then(|target| target.get("name").and_then(serde_json::Value::as_str))
        .ok_or_else(|| "cargo metadata root package has no cdylib target".into())
}

pub(super) fn cdylib_package_from_metadata<'a>(
    metadata: &'a serde_json::Value,
    packages: &'a [serde_json::Value],
    manifest_path: Option<&Utf8Path>,
) -> Result<&'a serde_json::Value, Box<dyn std::error::Error>> {
    if let Some(manifest_path) = manifest_path
        && let Some(package) = packages.iter().find(|package| {
            package
                .get("manifest_path")
                .and_then(serde_json::Value::as_str)
                == Some(manifest_path.as_str())
        })
    {
        return Ok(package);
    }

    if let Some(root_package) = metadata
        .get("root_package")
        .and_then(serde_json::Value::as_str)
        && let Some(package) = packages.iter().find(|package| {
            package.get("id").and_then(serde_json::Value::as_str) == Some(root_package)
        })
    {
        return Ok(package);
    }

    if let Some(member) = metadata
        .get("workspace_default_members")
        .and_then(serde_json::Value::as_array)
        .and_then(|members| (members.len() == 1).then(|| members[0].as_str()).flatten())
        && let Some(package) = packages
            .iter()
            .find(|package| package.get("id").and_then(serde_json::Value::as_str) == Some(member))
    {
        return Ok(package);
    }

    Err("cargo metadata could not identify the plugin package; pass --binary explicitly".into())
}

pub(super) fn cdylib_filename(target_name: &str) -> String {
    let stem = target_name.replace('-', "_");
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

pub(super) fn command_version(command: &str) -> String {
    Command::new(command)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|| "not found".to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct DoctorReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) ci_run_url: Option<String>,
    pub(super) checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct DoctorCheck {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) value: String,
    pub(super) hint: Option<String>,
}

pub(super) const DOCTOR_MAX_CHECKS: usize = 128;
pub(super) const DOCTOR_DAW_HOSTS: [&str; 5] = [
    "REAPER",
    "Cubase/Nuendo",
    "Bitwig Studio",
    "Ableton Live",
    "Studio One",
];

pub(super) fn known_doctor_check_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for os in ["Linux", "macOS", "Windows"] {
        names.extend(doctor_allowed_check_names_for_os(os));
    }
    names.insert("signing: release signing".to_string());
    names
}

pub(super) fn doctor_allowed_check_names_for_os(os: &str) -> BTreeSet<String> {
    required_doctor_check_names(os)
        .into_iter()
        .map(str::to_string)
        .chain(DOCTOR_DAW_HOSTS.map(|host| format!("daw install: {host}")))
        .collect()
}

pub(super) fn unexpected_doctor_checks_for_os(os: &str, report: &DoctorReport) -> Vec<String> {
    let allowed = doctor_allowed_check_names_for_os(os);
    let mut unexpected = report
        .checks
        .iter()
        .map(|check| check.name.trim())
        .filter(|name| !allowed.contains(*name))
        .map(str::to_string)
        .collect::<Vec<_>>();
    unexpected.sort();
    unexpected.dedup();
    unexpected
}

pub(super) fn validate_doctor_report(
    report: &DoctorReport,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(os) = &report.os {
        validate_release_action_text("doctor report os", os)?;
    }
    if let Some(ci_run_url) = &report.ci_run_url {
        validate_release_action_text("doctor report ci run url", ci_run_url)?;
        if !is_github_actions_run_url(ci_run_url) {
            return Err(format!("invalid doctor report ci_run_url `{ci_run_url}`").into());
        }
    }
    if report.checks.is_empty() {
        return Err("doctor report must contain at least one check".into());
    }
    if report.checks.len() > DOCTOR_MAX_CHECKS {
        return Err(format!(
            "doctor report has too many checks: {} exceeds maximum {DOCTOR_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen = BTreeSet::new();
    let known_checks = known_doctor_check_names();
    for check in &report.checks {
        validate_release_action_text("doctor check name", &check.name)?;
        validate_release_action_text(&format!("doctor `{}` status", check.name), &check.status)?;
        validate_release_action_text(&format!("doctor `{}` value", check.name), &check.value)?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("doctor `{}` hint", check.name), hint)?;
        }
        match check.status.as_str() {
            "ok" | "missing" | "skipped" | "unknown" | "unsupported" => {}
            other => {
                return Err(format!(
                    "doctor check `{}` has unexpected status `{other}`",
                    check.name
                )
                .into());
            }
        }
        let normalized = check.name.trim();
        if !seen.insert(normalized.to_string()) {
            return Err(format!("duplicate doctor check `{normalized}`").into());
        }
        if !known_checks.contains(normalized) {
            return Err(format!("unknown doctor check `{normalized}`").into());
        }
    }
    Ok(())
}

pub(super) fn run_doctor(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let report = doctor_report();
    validate_doctor_report(&report)?;
    match format {
        OutputFormat::Text => {
            for check in &report.checks {
                println!("{}: {}", check.name, check.value);
            }
        }
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }
    Ok(())
}

pub(super) fn doctor_report() -> DoctorReport {
    let mut checks = vec![
        command_check("rustc", None),
        command_check("cargo", None),
        command_check(
            "node",
            Some("install Node.js before building Web UI templates"),
        ),
        command_check(
            "npm",
            Some("install npm or provide an explicit UI build command"),
        ),
        binding_baseline_doctor_check(),
        sdk_headers_doctor_check(),
        validator_doctor_check(),
        system_webview_check(),
    ];
    checks.extend(signing_doctor_checks());
    checks.extend(daw_install_checks());
    DoctorReport {
        os: Some(doctor_os_label().to_string()),
        ci_run_url: github_actions_run_url_from_env(),
        checks,
    }
}

pub(super) fn doctor_os_label() -> &'static str {
    if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "unknown"
    }
}

pub(super) fn github_actions_run_url_from_env() -> Option<String> {
    let server = std::env::var("GITHUB_SERVER_URL").ok()?;
    let repo = std::env::var("GITHUB_REPOSITORY").ok()?;
    let run_id = std::env::var("GITHUB_RUN_ID").ok()?;
    if server.trim_end_matches('/') != "https://github.com"
        || repo.split('/').count() != 2
        || repo.split('/').any(str::is_empty)
        || !run_id.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let mut url = format!(
        "{}/{}/actions/runs/{}",
        server.trim_end_matches('/'),
        repo,
        run_id
    );
    if let Ok(attempt) = std::env::var("GITHUB_RUN_ATTEMPT")
        && !attempt.is_empty()
        && attempt.chars().all(|ch| ch.is_ascii_digit())
    {
        url.push_str("/attempts/");
        url.push_str(&attempt);
    }
    Some(url)
}

pub(super) fn command_check(command: &str, missing_hint: Option<&str>) -> DoctorCheck {
    let value = command_version(command);
    let found = value != "not found";
    DoctorCheck {
        name: command.to_string(),
        status: if found { "ok" } else { "missing" }.to_string(),
        value,
        hint: (!found)
            .then(|| missing_hint.unwrap_or("install this command and make sure it is on PATH"))
            .map(str::to_string),
    }
}

pub(super) fn command_presence_check(
    name: &str,
    binary: &str,
    candidates: &[Utf8PathBuf],
    missing_hint: &str,
) -> DoctorCheck {
    match find_command(binary, candidates) {
        Some(path) => DoctorCheck {
            name: name.to_string(),
            status: "ok".to_string(),
            value: path.to_string(),
            hint: None,
        },
        None => DoctorCheck {
            name: name.to_string(),
            status: "missing".to_string(),
            value: format!("{binary} not found"),
            hint: Some(missing_hint.to_string()),
        },
    }
}

pub(super) fn binding_baseline_doctor_check() -> DoctorCheck {
    DoctorCheck {
        name: "vst3 binding baseline".to_string(),
        status: "ok".to_string(),
        value: binding_baseline_value(),
        hint: binding_baseline_hint(),
    }
}

pub(super) fn binding_baseline_value() -> String {
    let baseline = vesty_vst3_sys::BINDING_BASELINE;
    format!(
        "Steinberg SDK {}; upstream vst3 crate {}; backend {}",
        baseline.steinberg_sdk,
        baseline.upstream_vst3_crate,
        binding_backend_text(baseline.backend)
    )
}

pub(super) fn binding_backend_text(backend: vesty_vst3_sys::BindingBackend) -> &'static str {
    match backend {
        vesty_vst3_sys::BindingBackend::UpstreamVst3Crate => "upstream-vst3",
        vesty_vst3_sys::BindingBackend::GeneratedHeadersReserved => "generated-headers-reserved",
        vesty_vst3_sys::BindingBackend::MetadataOnly => "metadata-only",
    }
}

pub(super) fn binding_baseline_hint() -> Option<String> {
    (vesty_vst3_sys::BINDING_BASELINE.backend
        == vesty_vst3_sys::BindingBackend::GeneratedHeadersReserved)
        .then(|| {
            format!(
                "set {} to the official VST3 SDK checkout before generating header bindings",
                vesty_vst3_sys::VST3_SDK_DIR_ENV
            )
        })
}

pub(super) fn sdk_headers_doctor_check() -> DoctorCheck {
    match vesty_vst3_sys::probe_sdk_headers_from_env() {
        Ok(probe) if probe.ready_for_generated_headers() => DoctorCheck {
            name: "vst3 SDK headers".to_string(),
            status: "ok".to_string(),
            value: sdk_header_probe_value(&probe),
            hint: None,
        },
        Ok(probe) => DoctorCheck {
            name: "vst3 SDK headers".to_string(),
            status: "missing".to_string(),
            value: sdk_header_probe_value(&probe),
            hint: Some(format!(
                "point {} at an official Steinberg VST3 SDK {} checkout before enabling generated header bindings",
                vesty_vst3_sys::VST3_SDK_DIR_ENV,
                vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
            )),
        },
        Err(vesty_vst3_sys::SdkHeaderProbeError::MissingEnv) => DoctorCheck {
            name: "vst3 SDK headers".to_string(),
            status: "skipped".to_string(),
            value: format!(
                "{} is not set; upstream vst3 crate backend is active",
                vesty_vst3_sys::VST3_SDK_DIR_ENV
            ),
            hint: Some(format!(
                "set {} to the official Steinberg VST3 SDK checkout when generating Vesty-owned header bindings",
                vesty_vst3_sys::VST3_SDK_DIR_ENV
            )),
        },
    }
}

pub(super) fn sdk_header_probe_value(probe: &vesty_vst3_sys::SdkHeaderProbe) -> String {
    let version_hint = probe
        .version_hint
        .as_deref()
        .unwrap_or("version marker not found");
    if probe.ready_for_generated_headers() {
        format!(
            "{}; {} required headers present; baseline {}; {}",
            probe.root.display(),
            probe.present_headers.len(),
            probe.baseline,
            version_hint
        )
    } else {
        format!(
            "{}; missing {}/{} required headers: {}; baseline {}; {}",
            probe.root.display(),
            probe.missing_headers.len(),
            vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS.len(),
            probe.missing_headers.join(", "),
            probe.baseline,
            version_hint
        )
    }
}

pub(super) fn validator_doctor_check() -> DoctorCheck {
    match discover_validator(None) {
        Some(path) => DoctorCheck {
            name: "vst3 validator".to_string(),
            status: "ok".to_string(),
            value: path.to_string(),
            hint: None,
        },
        None => DoctorCheck {
            name: "vst3 validator".to_string(),
            status: "missing".to_string(),
            value: "not found (set VST3_VALIDATOR or pass --validator)".to_string(),
            hint: Some(
                "build Steinberg VST3 SDK validator and expose it through VST3_VALIDATOR or PATH"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn signing_doctor_checks() -> Vec<DoctorCheck> {
    if cfg!(target_os = "macos") {
        vec![
            command_presence_check(
                "signing: codesign",
                "codesign",
                &[Utf8PathBuf::from("/usr/bin/codesign")],
                "install Xcode Command Line Tools before signing release bundles",
            ),
            macos_notarytool_check(),
        ]
    } else if cfg!(target_os = "windows") {
        vec![command_presence_check(
            "signing: signtool",
            "signtool.exe",
            &windows_signtool_candidates(),
            "install Windows SDK signtool.exe before signing release bundles",
        )]
    } else if cfg!(target_os = "linux") {
        vec![DoctorCheck {
            name: "signing: linux release policy".to_string(),
            status: "unknown".to_string(),
            value: "no standard VST3 bundle signing tool required by Vesty".to_string(),
            hint: Some(
                "sign distro packages or installers outside Vesty when your release channel requires it"
                    .to_string(),
            ),
        }]
    } else {
        vec![DoctorCheck {
            name: "signing: release signing".to_string(),
            status: "unsupported".to_string(),
            value: "unsupported platform".to_string(),
            hint: Some("Vesty MVP release packaging targets macOS, Windows and Linux".to_string()),
        }]
    }
}

pub(super) fn macos_notarytool_check() -> DoctorCheck {
    let xcrun = find_command("xcrun", &[Utf8PathBuf::from("/usr/bin/xcrun")]);
    let Some(xcrun) = xcrun else {
        return DoctorCheck {
            name: "signing: notarytool".to_string(),
            status: "missing".to_string(),
            value: "xcrun not found".to_string(),
            hint: Some(
                "install Xcode Command Line Tools; notarization requires xcrun notarytool"
                    .to_string(),
            ),
        };
    };

    let output = Command::new(xcrun.as_std_path())
        .args(["--find", "notarytool"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            DoctorCheck {
                name: "signing: notarytool".to_string(),
                status: "ok".to_string(),
                value: if value.is_empty() {
                    "notarytool found".to_string()
                } else {
                    value
                },
                hint: None,
            }
        }
        _ => DoctorCheck {
            name: "signing: notarytool".to_string(),
            status: "missing".to_string(),
            value: "notarytool not found via xcrun".to_string(),
            hint: Some(
                "install a recent Xcode or Command Line Tools before notarizing macOS bundles"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn system_webview_check() -> DoctorCheck {
    if cfg!(target_os = "macos") {
        let found = std::path::Path::new("/System/Library/Frameworks/WebKit.framework").exists();
        DoctorCheck {
            name: "system webview".to_string(),
            status: if found { "ok" } else { "missing" }.to_string(),
            value: if found {
                "ok (macOS WebKit.framework)".to_string()
            } else {
                "missing WebKit.framework".to_string()
            },
            hint: (!found).then(|| "install or repair macOS WebKit.framework".to_string()),
        }
    } else if cfg!(target_os = "windows") {
        let found = std::env::var_os("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER").is_some()
            || std::path::Path::new(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application")
                .exists()
            || std::path::Path::new(r"C:\Program Files\Microsoft\EdgeWebView\Application").exists();
        DoctorCheck {
            name: "system webview".to_string(),
            status: if found { "ok" } else { "unknown" }.to_string(),
            value: if found {
                "ok (WebView2 runtime detected)".to_string()
            } else {
                "unknown (install Microsoft Edge WebView2 Runtime if host UI fails)".to_string()
            },
            hint: (!found).then(|| "install Microsoft Edge WebView2 Evergreen Runtime".to_string()),
        }
    } else if cfg!(target_os = "linux") {
        let webkit = pkg_config_version("webkit2gtk-4.1")
            .or_else(|| pkg_config_version("webkit2gtk-4.0"))
            .unwrap_or_else(|| "not found".to_string());
        let x11 = pkg_config_version("x11").unwrap_or_else(|| "not found".to_string());
        let found = webkit != "not found" && x11 != "not found";
        DoctorCheck {
            name: "system webview".to_string(),
            status: if found { "ok" } else { "missing" }.to_string(),
            value: format!("WebKitGTK {webkit}; X11 {x11}; Wayland experimental"),
            hint: (!found).then(|| {
                "install WebKitGTK 4.1/4.0, GTK3 and X11 development packages".to_string()
            }),
        }
    } else {
        DoctorCheck {
            name: "system webview".to_string(),
            status: "unsupported".to_string(),
            value: "unsupported platform".to_string(),
            hint: Some("Vesty MVP supports macOS, Windows and Linux X11".to_string()),
        }
    }
}

pub(super) fn daw_install_checks() -> Vec<DoctorCheck> {
    DOCTOR_DAW_HOSTS
        .into_iter()
        .map(|host| {
            let candidates = match host {
                "REAPER" => daw_reaper_candidates(),
                "Cubase/Nuendo" => daw_cubase_nuendo_candidates(),
                "Bitwig Studio" => daw_bitwig_candidates(),
                "Ableton Live" => daw_ableton_candidates(),
                "Studio One" => daw_studio_one_candidates(),
                _ => Vec::new(),
            };
            daw_install_check(host, candidates)
        })
        .collect()
}

pub(super) fn daw_install_check(host: &str, candidates: Vec<Utf8PathBuf>) -> DoctorCheck {
    match first_existing_path(&candidates) {
        Some(path) => DoctorCheck {
            name: format!("daw install: {host}"),
            status: "ok".to_string(),
            value: path.to_string(),
            hint: Some(
                "installation detected; real DAW smoke evidence is still required".to_string(),
            ),
        },
        None => DoctorCheck {
            name: format!("daw install: {host}"),
            status: "missing".to_string(),
            value: "not found in common install paths".to_string(),
            hint: Some(
                "optional for development; required only when collecting the release DAW matrix"
                    .to_string(),
            ),
        },
    }
}

pub(super) fn first_existing_path(candidates: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

pub(super) fn daw_reaper_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.extend(mac_app_candidates(["REAPER"]));
    } else if cfg!(target_os = "windows") {
        candidates.extend(windows_program_candidates([
            "REAPER (x64)\\reaper.exe",
            "REAPER\\reaper.exe",
        ]));
    } else if cfg!(target_os = "linux") {
        candidates.extend([
            Utf8PathBuf::from("/usr/bin/reaper"),
            Utf8PathBuf::from("/usr/local/bin/reaper"),
            Utf8PathBuf::from("/opt/REAPER/reaper"),
        ]);
    }
    candidates
}

pub(super) fn daw_cubase_nuendo_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.extend(mac_app_prefix_candidates(["Cubase", "Nuendo"]));
    } else if cfg!(target_os = "windows") {
        candidates.extend(windows_program_prefix_candidates([
            ("Steinberg", "Cubase"),
            ("Steinberg", "Nuendo"),
        ]));
    }
    candidates
}

pub(super) fn daw_bitwig_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.extend(mac_app_candidates(["Bitwig Studio"]));
    } else if cfg!(target_os = "windows") {
        candidates.extend(windows_program_candidates([
            "Bitwig Studio\\Bitwig Studio.exe",
        ]));
    } else if cfg!(target_os = "linux") {
        candidates.extend([
            Utf8PathBuf::from("/usr/bin/bitwig-studio"),
            Utf8PathBuf::from("/usr/local/bin/bitwig-studio"),
            Utf8PathBuf::from("/opt/bitwig-studio/bitwig-studio"),
            Utf8PathBuf::from("/opt/Bitwig Studio/bitwig-studio"),
        ]);
    }
    candidates
}

pub(super) fn daw_ableton_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.extend(mac_app_prefix_candidates(["Ableton Live"]));
    } else if cfg!(target_os = "windows") {
        candidates.extend(windows_program_prefix_candidates([("Ableton", "Live")]));
    }
    candidates
}

pub(super) fn daw_studio_one_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.extend(mac_app_prefix_candidates(["Studio One"]));
    } else if cfg!(target_os = "windows") {
        candidates.extend(windows_program_candidates([
            "PreSonus\\Studio One 6\\Studio One.exe",
        ]));
        candidates.extend(windows_program_prefix_candidates([(
            "PreSonus",
            "Studio One",
        )]));
    }
    candidates
}

pub(super) fn mac_app_candidates<const N: usize>(names: [&str; N]) -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    for root in mac_application_roots() {
        for name in names {
            candidates.push(root.join(format!("{name}.app")));
        }
    }
    candidates
}

pub(super) fn mac_app_prefix_candidates<const N: usize>(prefixes: [&str; N]) -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    for root in mac_application_roots() {
        if let Ok(entries) = fs::read_dir(root.as_std_path()) {
            for entry in entries.flatten() {
                let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                    continue;
                };
                let Some(name) = path.file_name() else {
                    continue;
                };
                if name.ends_with(".app") && prefixes.iter().any(|prefix| name.starts_with(prefix))
                {
                    candidates.push(path);
                }
            }
        }
    }
    candidates
}

pub(super) fn mac_application_roots() -> Vec<Utf8PathBuf> {
    let mut roots = vec![Utf8PathBuf::from("/Applications")];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(Utf8PathBuf::from(home).join("Applications"));
    }
    roots
}

pub(super) fn windows_program_candidates<const N: usize>(
    relative_paths: [&str; N],
) -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    for root in windows_program_roots() {
        for relative in relative_paths {
            candidates.push(root.join(relative));
        }
    }
    candidates
}

pub(super) fn windows_program_prefix_candidates<const N: usize>(
    vendor_prefixes: [(&str, &str); N],
) -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    for root in windows_program_roots() {
        for (vendor, prefix) in vendor_prefixes {
            let vendor_dir = root.join(vendor);
            if let Ok(entries) = fs::read_dir(vendor_dir.as_std_path()) {
                for entry in entries.flatten() {
                    let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                        continue;
                    };
                    let Some(name) = path.file_name() else {
                        continue;
                    };
                    if name.starts_with(prefix) {
                        candidates.push(path);
                    }
                }
            }
        }
    }
    candidates
}

pub(super) fn windows_program_roots() -> Vec<Utf8PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
        .into_iter()
        .filter_map(|var| std::env::var(var).ok())
        .map(Utf8PathBuf::from)
        .collect()
}

pub(super) fn windows_signtool_candidates() -> Vec<Utf8PathBuf> {
    let mut candidates = Vec::new();
    for root in windows_program_roots() {
        let kit_bin = root.join("Windows Kits/10/bin");
        candidates.extend([
            kit_bin.join("x64/signtool.exe"),
            kit_bin.join("x86/signtool.exe"),
            kit_bin.join("arm64/signtool.exe"),
        ]);
        if let Ok(entries) = fs::read_dir(kit_bin.as_std_path()) {
            for entry in entries.flatten() {
                let Ok(version_dir) = Utf8PathBuf::from_path_buf(entry.path()) else {
                    continue;
                };
                candidates.extend([
                    version_dir.join("x64/signtool.exe"),
                    version_dir.join("x86/signtool.exe"),
                    version_dir.join("arm64/signtool.exe"),
                ]);
            }
        }
    }
    candidates
}

pub(super) fn discover_validator(explicit: Option<Utf8PathBuf>) -> Option<Utf8PathBuf> {
    if let Some(path) = explicit {
        return Some(path);
    }
    if let Ok(path) = std::env::var("VST3_VALIDATOR") {
        let path = Utf8PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    for name in validator_binary_names() {
        if let Some(path) = find_in_path(name) {
            return Some(path);
        }
    }
    validator_candidates()
        .into_iter()
        .find(|path| path.is_file())
}

pub(super) fn validator_binary_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["validator.exe", "validator"]
    } else {
        &["validator"]
    }
}

pub(super) fn validator_candidates() -> Vec<Utf8PathBuf> {
    let mut roots = Vec::new();
    if let Ok(sdk) = std::env::var("VST3_SDK") {
        roots.push(Utf8PathBuf::from(sdk));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = Utf8PathBuf::from(home);
        roots.push(home.join("VST_SDK/VST3_SDK"));
        roots.push(home.join("SDKs/VST_SDK/VST3_SDK"));
        roots.push(home.join("Developer/VST3_SDK"));
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let profile = Utf8PathBuf::from(profile);
        roots.push(profile.join("VST_SDK/VST3_SDK"));
        roots.push(profile.join("source/VST3_SDK"));
    }

    let mut candidates = Vec::new();
    if let Ok(current_dir) = std::env::current_dir()
        && let Ok(current_dir) = Utf8PathBuf::from_path_buf(current_dir)
    {
        for name in validator_binary_names() {
            candidates.push(current_dir.join(format!(
                "target/steinberg/vst3sdk-build-xcode/bin/Release/{name}"
            )));
            candidates.push(
                current_dir.join(format!("target/steinberg/vst3sdk-build/bin/Release/{name}")),
            );
            candidates.push(current_dir.join(format!("target/steinberg/vst3sdk-build/bin/{name}")));
        }
    }
    for root in roots {
        for name in validator_binary_names() {
            candidates.push(root.join(format!("build/bin/{name}")));
            candidates.push(root.join(format!("build/bin/Debug/{name}")));
            candidates.push(root.join(format!("build/bin/Release/{name}")));
            candidates.push(root.join(format!("bin/{name}")));
        }
    }
    candidates
}

pub(super) fn find_command(binary: &str, candidates: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
    find_in_path(binary).or_else(|| candidates.iter().find(|path| path.is_file()).cloned())
}

pub(super) fn find_in_path(binary: &str) -> Option<Utf8PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(binary);
        if candidate.is_file()
            && let Ok(path) = Utf8PathBuf::from_path_buf(candidate)
        {
            return Some(path);
        }
    }
    None
}

pub(super) fn pkg_config_version(package: &str) -> Option<String> {
    let output = Command::new("pkg-config")
        .args(["--modversion", package])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

pub(super) fn build_configured_ui_assets(
    project_dir: &Utf8Path,
    ui: &UiConfig,
) -> Result<Option<AssetManifest>, Box<dyn std::error::Error>> {
    let ui_dir = project_dir.join(&ui.dir);
    if let Some(build) = &ui.build {
        run_ui_build_command(build, &ui_dir)?;
    }

    let Some(dist) = &ui.dist else {
        return Ok(None);
    };

    let root = ui_dir.join(dist);
    if !root.is_dir() {
        return Err(format!(
            "ui dist directory does not exist: {root}; check [ui].dist or run [ui].build"
        )
        .into());
    }
    let manifest = AssetManifest::from_dir(&root, "index.html").map_err(|error| {
        format!("failed to create UI asset manifest from [ui].dist {root}: {error}")
    })?;
    Ok(Some(manifest))
}

pub(super) fn run_ui_build_command(
    command: &str,
    ui_dir: &Utf8PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if command.trim().is_empty() {
        return Err("[ui].build must not be empty".into());
    }
    if !ui_dir.is_dir() {
        return Err(format!("ui directory does not exist for [ui].build: {ui_dir}").into());
    }
    run_shell(command, Some(ui_dir))
        .map_err(|error| format!("[ui].build failed in {ui_dir}: `{command}`: {error}").into())
}

pub(super) fn run_shell(
    command: &str,
    cwd: Option<&Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if command.trim().is_empty() {
        return Err("shell command must not be empty".into());
    }
    let cwd_text = cwd.map(ToString::to_string);
    let mut shell = if cfg!(target_os = "windows") {
        let mut command_runner = Command::new("cmd");
        command_runner.args(["/C", command]);
        command_runner
    } else {
        let mut command_runner = Command::new("sh");
        command_runner.args(["-lc", command]);
        command_runner
    };
    if let Some(cwd) = cwd {
        if !cwd.is_dir() {
            return Err(format!("command working directory does not exist: {cwd}").into());
        }
        shell.current_dir(cwd);
    }
    let status = shell.status().map_err(|error| {
        let cwd_suffix = cwd_text
            .as_deref()
            .map(|cwd| format!(" in {cwd}"))
            .unwrap_or_default();
        format!("failed to run command{cwd_suffix}: `{command}`: {error}")
    })?;
    if status.success() {
        Ok(())
    } else {
        let cwd_suffix = cwd_text
            .as_deref()
            .map(|cwd| format!(" in {cwd}"))
            .unwrap_or_default();
        Err(format!("command failed with status {status}{cwd_suffix}: `{command}`").into())
    }
}
