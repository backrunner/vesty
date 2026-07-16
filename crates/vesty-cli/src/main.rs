use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::{Command, Output};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use vesty_build::{
    AssetManifest, BinaryExportCheck, BundlePlatform, BundleValidationReport,
    PARAMETER_MANIFEST_FILE, PackageOptions, ParameterManifest, UiConfig, package_vst3,
    parameter_manifest_from_specs_json, read_config, read_parameter_manifest, validate_vst3_bundle,
};

#[derive(Parser)]
#[command(name = "vesty")]
#[command(about = "Vesty VST3 plugin framework tooling")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    New {
        name: String,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        ui: Option<String>,
        #[arg(long)]
        template: Option<String>,
        #[arg(long)]
        vesty_path: Option<Utf8PathBuf>,
        #[arg(long)]
        plugin_ui_path: Option<Utf8PathBuf>,
    },
    Templates {
        #[arg(long, default_value = "text")]
        format: String,
    },
    Dev {
        #[arg(long, default_value = "vesty.toml")]
        config: Utf8PathBuf,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        no_ui: bool,
        #[arg(long)]
        ui_command: Option<String>,
        #[arg(long)]
        install_dev: bool,
        #[arg(long)]
        binary: Option<Utf8PathBuf>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long, default_value = "target/vesty-dev")]
        out: Utf8PathBuf,
        #[arg(long)]
        vst3_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "copy")]
        install_mode: String,
    },
    Build {
        #[arg(long, default_value = "vesty.toml")]
        config: Utf8PathBuf,
        #[arg(long)]
        debug: bool,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        no_ui: bool,
    },
    Package {
        #[arg(long, default_value = "vesty.toml")]
        config: Utf8PathBuf,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        binary: Utf8PathBuf,
        #[arg(long, default_value = "target/vesty")]
        out: Utf8PathBuf,
        #[arg(long)]
        install_dev: bool,
        #[arg(long)]
        vst3_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "copy")]
        install_mode: String,
    },
    Notarize {
        bundle: Utf8PathBuf,
        #[arg(long)]
        keychain_profile: Option<String>,
        #[arg(long)]
        apple_id: Option<String>,
        #[arg(long)]
        team_id: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        archive: Option<Utf8PathBuf>,
        #[arg(long)]
        no_wait: bool,
        #[arg(long)]
        no_staple: bool,
    },
    Validate {
        bundle: Utf8PathBuf,
        #[arg(long)]
        validator: Option<Utf8PathBuf>,
        #[arg(long)]
        static_only: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long)]
        report: Option<Utf8PathBuf>,
        #[arg(long)]
        validator_log: Option<Utf8PathBuf>,
    },
    #[command(name = "param-manifest", alias = "parameter-manifest")]
    ParamManifest {
        #[arg(long)]
        specs: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    DawMatrix {
        #[arg(long, default_value = "target/daw-evidence/reaper")]
        reaper_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/cubase")]
        cubase_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/bitwig")]
        bitwig_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/ableton")]
        ableton_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/studio-one")]
        studio_one_evidence: Utf8PathBuf,
        #[arg(long)]
        evidence_root: Option<Utf8PathBuf>,
        #[arg(long, default_value = "markdown")]
        format: String,
        #[arg(long)]
        write_template: bool,
        #[arg(long)]
        write_report: bool,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        scan: Option<String>,
        #[arg(long)]
        load: Option<String>,
        #[arg(long)]
        ui: Option<String>,
        #[arg(long)]
        ui_host_param: Option<String>,
        #[arg(long)]
        meter_stream: Option<String>,
        #[arg(long)]
        automation: Option<String>,
        #[arg(long)]
        buffer_sample_rate_change: Option<String>,
        #[arg(long)]
        save_restore: Option<String>,
        #[arg(long)]
        offline_render: Option<String>,
        #[arg(long)]
        strict: bool,
    },
    HostQuirks {
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    #[command(name = "release-evidence")]
    ReleaseEvidence {
        #[command(subcommand)]
        command: ReleaseEvidenceCommand,
    },
    ReleaseCheck {
        #[arg(long, default_value = "target/daw-evidence/reaper")]
        reaper_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/cubase")]
        cubase_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/bitwig")]
        bitwig_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/ableton")]
        ableton_evidence: Utf8PathBuf,
        #[arg(long, default_value = "target/daw-evidence/studio-one")]
        studio_one_evidence: Utf8PathBuf,
        #[arg(long)]
        evidence_root: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vesty-protocol")]
        protocol_snapshot: Utf8PathBuf,
        #[arg(long)]
        skip_protocol: bool,
        #[arg(long, default_value = "markdown")]
        format: String,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        ci_doctor_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        ci_release_check_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        platform_smoke_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        ci_run_url: Option<String>,
        #[arg(long)]
        release_evidence_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        validate_report: Vec<Utf8PathBuf>,
        #[arg(long)]
        static_validate_report: Vec<Utf8PathBuf>,
        #[arg(long)]
        publish_plan_report: Option<Utf8PathBuf>,
        #[arg(long)]
        crate_package_report: Option<Utf8PathBuf>,
        #[arg(long)]
        npm_pack_report: Option<Utf8PathBuf>,
        #[arg(long)]
        dependency_baseline_report: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_manifest: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_binding_plan: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_binding_surface: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_scaffold: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_abi_seed: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_abi: Option<Utf8PathBuf>,
        #[arg(long)]
        vst3_sdk_interface_skeleton: Option<Utf8PathBuf>,
        #[arg(long)]
        signed_bundle_evidence: Vec<Utf8PathBuf>,
        #[arg(long)]
        notarization_log: Option<Utf8PathBuf>,
        #[arg(long)]
        require_release_artifacts: bool,
        #[arg(long)]
        write_evidence_template: Option<Utf8PathBuf>,
        #[arg(long)]
        report: Option<Utf8PathBuf>,
        #[arg(long)]
        plan: Option<Utf8PathBuf>,
    },
    #[command(name = "platform-smoke")]
    PlatformSmoke {
        #[arg(long, default_value = "target/platform-smoke")]
        dir: Utf8PathBuf,
        #[arg(long)]
        write_template: bool,
        #[arg(long)]
        write_report: bool,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        system_webview: Option<String>,
        #[arg(long)]
        vst3_validator: Option<String>,
        #[arg(long)]
        vst3_example_scan: Option<String>,
        #[arg(long)]
        webview_attach: Option<String>,
        #[arg(long)]
        webview_resize: Option<String>,
        #[arg(long)]
        asset_protocol: Option<String>,
        #[arg(long)]
        jsbridge_roundtrip: Option<String>,
        #[arg(long)]
        meter_stream: Option<String>,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    #[command(name = "smoke-host")]
    SmokeHost {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long)]
        bridge_trace: Option<Utf8PathBuf>,
        #[arg(long)]
        meter_log: Option<Utf8PathBuf>,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "publish-plan", alias = "release-order")]
    PublishPlan {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "crate-package")]
    CratePackage {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "npm-pack")]
    NpmPack {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "dependency-baseline")]
    DependencyBaseline {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        latest: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    ExportTypes {
        #[arg(long, default_value = "target/vesty-protocol")]
        out: Utf8PathBuf,
        #[arg(long)]
        check: bool,
    },
    #[command(name = "vst3-sdk")]
    Vst3Sdk {
        #[command(subcommand)]
        command: Vst3SdkCommand,
    },
    Doctor {
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
enum ReleaseEvidenceCommand {
    #[command(name = "collect-local")]
    CollectLocal {
        #[arg(long, default_value = ".")]
        workspace: Utf8PathBuf,
        #[arg(long, default_value = "target/release-evidence")]
        dir: Utf8PathBuf,
        #[arg(long, default_value = "target/vesty-protocol")]
        protocol_snapshot: Utf8PathBuf,
        #[arg(long)]
        no_template: bool,
        #[arg(long)]
        no_protocol: bool,
        #[arg(long)]
        no_publish_plan: bool,
        #[arg(long)]
        crate_package: bool,
        #[arg(long)]
        no_npm_pack: bool,
        #[arg(long)]
        dependency_baseline_latest: bool,
        #[arg(long)]
        vst3_sdk_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vst3-sdk/generated.rs")]
        vst3_sdk_bindings_module: Utf8PathBuf,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "import-ci")]
    ImportCi {
        #[arg(long)]
        source: Utf8PathBuf,
        #[arg(long, default_value = "target/release-evidence")]
        dir: Utf8PathBuf,
        #[arg(long)]
        ci_run_url: Option<String>,
        #[arg(long)]
        ci_run_url_file: Option<Utf8PathBuf>,
        #[arg(long)]
        no_template: bool,
        #[arg(long)]
        overwrite: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "collect-signing")]
    CollectSigning {
        bundle: Utf8PathBuf,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        binary: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/release-evidence")]
        dir: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        tool: Option<Utf8PathBuf>,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "collect-notarization")]
    CollectNotarization {
        #[arg(long)]
        notary_log: Utf8PathBuf,
        #[arg(long)]
        stapler_log: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/release-evidence")]
        dir: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
enum Vst3SdkCommand {
    #[command(name = "manifest")]
    Manifest {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "binding-plan")]
    BindingPlan {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vst3-sdk/generated.rs")]
        bindings_module: Utf8PathBuf,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "binding-surface")]
    BindingSurface {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "emit-scaffold")]
    EmitScaffold {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vst3-sdk/generated.rs")]
        out: Utf8PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "emit-abi-seed")]
    EmitAbiSeed {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vst3-sdk/generated-abi-seed.rs")]
        out: Utf8PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "emit-abi")]
    EmitAbi {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(long, default_value = "target/vst3-sdk/generated-abi.rs")]
        out: Utf8PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    #[command(name = "emit-interface-skeleton")]
    EmitInterfaceSkeleton {
        #[arg(long)]
        sdk_dir: Option<Utf8PathBuf>,
        #[arg(
            long,
            default_value = "target/vst3-sdk/generated-interface-skeleton.rs"
        )]
        out: Utf8PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

const CLI_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() {
    let worker = thread::Builder::new()
        .name("vesty-cli".to_string())
        .stack_size(CLI_STACK_SIZE)
        .spawn(cli_main);
    let result = match worker {
        Ok(worker) => match worker.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        },
        Err(error) => Err(format!("failed to start CLI worker: {error}")),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn cli_main() -> Result<(), String> {
    let cli = Cli::parse();
    run(cli).map_err(|error| error.to_string())
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::New {
            name,
            kind,
            ui,
            template,
            vesty_path,
            plugin_ui_path,
        } => {
            create_project(
                &name,
                kind.as_deref(),
                ui.as_deref(),
                template.as_deref(),
                vesty_path.as_deref(),
                plugin_ui_path.as_deref(),
            )?;
            println!("created Vesty project '{name}'");
        }
        Commands::Templates { format } => {
            print_template_gallery(&format)?;
        }
        Commands::Dev {
            config,
            release,
            no_ui,
            ui_command,
            install_dev,
            binary,
            platform,
            out,
            vst3_dir,
            install_mode,
        } => {
            run_dev(DevOptions {
                config_path: config,
                release,
                no_ui,
                ui_command,
                install_dev,
                binary,
                platform,
                output_dir: out,
                vst3_dir,
                install_mode,
            })?;
        }
        Commands::Build {
            config,
            debug,
            release,
            no_ui,
        } => {
            let release_mode = build_release_mode(debug, release)?;
            let project_dir = config
                .parent()
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| Utf8PathBuf::from("."));
            let config = read_config(&config)?;
            println!(
                "building {} {} ({})",
                config.plugin.name,
                config.plugin.version,
                if release_mode { "release" } else { "debug" }
            );
            if no_ui {
                println!("ui: skipped");
            } else if let Some(ui) = config.ui {
                let manifest = build_configured_ui_assets(&project_dir, &ui)?;
                if let Some(manifest) = manifest {
                    println!("{}", serde_json::to_string_pretty(&manifest)?);
                }
            }
            let mut cargo = Command::new("cargo");
            cargo.current_dir(&project_dir).arg("build");
            if release_mode {
                cargo.arg("--release");
            }
            let status = cargo.status()?;
            if !status.success() {
                return Err("cargo build failed".into());
            }
        }
        Commands::Package {
            config,
            platform,
            binary,
            out,
            install_dev,
            vst3_dir,
            install_mode,
        } => {
            let project_dir = config
                .parent()
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| Utf8PathBuf::from("."));
            let config = read_config(&config)?;
            let platform = resolve_bundle_platform(platform.as_deref())?;
            let report = package_vst3(
                &config,
                &PackageOptions {
                    project_dir,
                    output_dir: out,
                    platform,
                    binary_path: binary,
                },
            )?;
            if let Some(signing) = config
                .package
                .as_ref()
                .and_then(|package| package.signing.as_deref())
                .filter(|signing| !signing.trim().is_empty())
            {
                let command = bundle_signing_command(
                    platform,
                    &report.bundle_dir,
                    &report.binary_path,
                    signing,
                )?;
                run_signing_command(&command)?;
                println!("signed: {}", report.bundle_dir);
            }
            println!("bundle: {}", report.bundle_dir);
            println!("binary: {}", report.binary_path);
            println!("moduleinfo: {}", report.moduleinfo_path);
            if let Some(manifest) = report.parameter_manifest_path {
                println!("parameter manifest: {manifest}");
            }
            if let Some(manifest) = report.asset_manifest_path {
                println!("asset manifest: {manifest}");
            }
            if install_dev {
                let installed =
                    install_dev_bundle_from_options(&report.bundle_dir, vst3_dir, &install_mode)?;
                println!("installed dev bundle: {installed}");
            }
        }
        Commands::Notarize {
            bundle,
            keychain_profile,
            apple_id,
            team_id,
            password,
            archive,
            no_wait,
            no_staple,
        } => {
            run_notarize(NotarizeOptions {
                bundle,
                keychain_profile,
                apple_id,
                team_id,
                password,
                archive,
                wait: !no_wait,
                staple: !no_staple,
            })?;
        }
        Commands::Validate {
            bundle,
            validator,
            static_only,
            strict,
            format,
            report,
            validator_log,
        } => {
            run_validate(
                bundle,
                validator,
                static_only,
                strict,
                &format,
                report,
                validator_log,
            )?;
        }
        Commands::ParamManifest {
            specs,
            out,
            check,
            format,
        } => {
            run_param_manifest(specs, out, check, &format)?;
        }
        Commands::DawMatrix {
            reaper_evidence,
            cubase_evidence,
            bitwig_evidence,
            ableton_evidence,
            studio_one_evidence,
            evidence_root,
            format,
            write_template,
            write_report,
            host,
            platform,
            scan,
            load,
            ui,
            ui_host_param,
            meter_stream,
            automation,
            buffer_sample_rate_change,
            save_restore,
            offline_render,
            strict,
        } => {
            let evidence = resolve_daw_evidence_paths(
                evidence_root,
                reaper_evidence,
                cubase_evidence,
                bitwig_evidence,
                ableton_evidence,
                studio_one_evidence,
            );
            if write_template {
                let created = write_daw_evidence_templates(&evidence)?;
                eprintln!("daw evidence template files created: {created}");
            }
            if write_report {
                let path = write_daw_smoke_report(
                    &evidence,
                    DawSmokeReportInput {
                        host,
                        platform,
                        scan,
                        load,
                        ui,
                        ui_host_param,
                        meter_stream,
                        automation,
                        buffer_sample_rate_change,
                        save_restore,
                        offline_render,
                    },
                )?;
                eprintln!("daw smoke evidence written: {path}");
            }
            let rows = daw_matrix_rows(&evidence);
            print_daw_matrix(&rows, &format)?;
            if strict && !daw_matrix_complete(&rows) {
                return Err(
                    "DAW matrix is incomplete; collect all host smoke evidence before release"
                        .into(),
                );
            }
        }
        Commands::HostQuirks { host, format } => {
            print_host_quirks(host.as_deref(), &format)?;
        }
        Commands::ReleaseEvidence { command } => match command {
            ReleaseEvidenceCommand::CollectLocal {
                workspace,
                dir,
                protocol_snapshot,
                no_template,
                no_protocol,
                no_publish_plan,
                crate_package,
                no_npm_pack,
                dependency_baseline_latest,
                vst3_sdk_dir,
                vst3_sdk_bindings_module,
                format,
            } => {
                collect_local_release_evidence(
                    &workspace,
                    &dir,
                    &protocol_snapshot,
                    CollectLocalOptions {
                        template: !no_template,
                        protocol: !no_protocol,
                        publish_plan: !no_publish_plan,
                        crate_package,
                        npm_pack: !no_npm_pack,
                        dependency_baseline_latest,
                        vst3_sdk_dir,
                        vst3_sdk_bindings_module,
                    },
                    &format,
                )?;
            }
            ReleaseEvidenceCommand::ImportCi {
                source,
                dir,
                ci_run_url,
                ci_run_url_file,
                no_template,
                overwrite,
                format,
            } => {
                import_ci_release_evidence(ImportCiOptions {
                    source,
                    dir,
                    ci_run_url,
                    ci_run_url_file,
                    template: !no_template,
                    overwrite,
                    format,
                })?;
            }
            ReleaseEvidenceCommand::CollectSigning {
                bundle,
                platform,
                binary,
                dir,
                out,
                tool,
                format,
            } => {
                collect_signing_release_evidence(CollectSigningOptions {
                    bundle,
                    platform,
                    binary,
                    dir,
                    out,
                    tool,
                    format,
                })?;
            }
            ReleaseEvidenceCommand::CollectNotarization {
                notary_log,
                stapler_log,
                dir,
                out,
                format,
            } => {
                collect_notarization_release_evidence(CollectNotarizationOptions {
                    notary_log,
                    stapler_log,
                    dir,
                    out,
                    format,
                })?;
            }
        },
        Commands::ReleaseCheck {
            reaper_evidence,
            cubase_evidence,
            bitwig_evidence,
            ableton_evidence,
            studio_one_evidence,
            evidence_root,
            protocol_snapshot,
            skip_protocol,
            format,
            strict,
            ci_doctor_dir,
            ci_release_check_dir,
            platform_smoke_dir,
            ci_run_url,
            release_evidence_dir,
            validate_report,
            static_validate_report,
            publish_plan_report,
            crate_package_report,
            npm_pack_report,
            dependency_baseline_report,
            vst3_sdk_manifest,
            vst3_sdk_binding_plan,
            vst3_sdk_binding_surface,
            vst3_sdk_scaffold,
            vst3_sdk_abi_seed,
            vst3_sdk_abi,
            vst3_sdk_interface_skeleton,
            signed_bundle_evidence,
            notarization_log,
            require_release_artifacts,
            write_evidence_template,
            report: report_path,
            plan,
        } => {
            if let Some(template_dir) = write_evidence_template.as_ref() {
                let created = write_release_evidence_templates(template_dir)?;
                eprintln!("release evidence template files created: {created}");
            }
            let evidence_root_for_plan = evidence_root.clone();
            let evidence = resolve_daw_evidence_paths(
                evidence_root,
                reaper_evidence,
                cubase_evidence,
                bitwig_evidence,
                ableton_evidence,
                studio_one_evidence,
            );
            let rows = daw_matrix_rows(&evidence);
            let mut release_evidence = ReleaseEvidenceOptions {
                ci_doctor_dir,
                ci_release_check_dir,
                platform_smoke_dir,
                ci_run_url,
                validate_reports: validate_report,
                static_validate_reports: static_validate_report,
                publish_plan_report,
                crate_package_report,
                npm_pack_report,
                dependency_baseline_report,
                vst3_sdk_manifest,
                vst3_sdk_binding_plan,
                vst3_sdk_binding_surface,
                vst3_sdk_scaffold,
                vst3_sdk_abi_seed,
                vst3_sdk_abi,
                vst3_sdk_interface_skeleton,
                signed_bundle_evidence,
                notarization_log,
                require_release_artifacts,
            };
            if let Some(dir) = release_evidence_dir.as_deref() {
                apply_release_evidence_dir(&mut release_evidence, dir)?;
            }
            let release_report = build_release_check_report(
                rows,
                &protocol_snapshot,
                skip_protocol,
                &release_evidence,
            );
            if let Some(plan_path) = plan.as_deref() {
                let action_plan = build_release_action_plan(
                    &release_report,
                    &protocol_snapshot,
                    evidence_root_for_plan.as_deref(),
                    release_evidence_dir.as_deref(),
                );
                write_release_action_plan(plan_path, &action_plan)?;
                eprintln!("release action plan written: {plan_path}");
            }
            print_release_check_report(&release_report, &format, report_path.as_deref())?;
            if strict && !release_check_complete(&release_report) {
                return Err("release check failed; collect missing evidence before release".into());
            }
        }
        Commands::PlatformSmoke {
            dir,
            write_template,
            write_report,
            platform,
            os,
            host,
            system_webview,
            vst3_validator,
            vst3_example_scan,
            webview_attach,
            webview_resize,
            asset_protocol,
            jsbridge_roundtrip,
            meter_stream,
            check,
            strict,
            format,
        } => {
            if write_template {
                let created = write_platform_smoke_templates(&dir)?;
                eprintln!("platform smoke template files created: {created}");
            }
            if write_report {
                let path = write_platform_smoke_report(
                    &dir,
                    PlatformSmokeReportInput {
                        platform,
                        os,
                        host,
                        system_webview,
                        vst3_validator,
                        vst3_example_scan,
                        webview_attach,
                        webview_resize,
                        asset_protocol,
                        jsbridge_roundtrip,
                        meter_stream,
                    },
                )?;
                eprintln!("platform smoke report written: {path}");
            }
            if check || (!write_template && !write_report) {
                let report = platform_smoke_release_check(Some(&dir), strict);
                print_single_release_check_item(&report, &format)?;
                if strict && report.status != "ok" {
                    return Err("platform smoke evidence is incomplete".into());
                }
            }
        }
        Commands::SmokeHost {
            workspace,
            bridge_trace,
            meter_log,
            out,
            check,
            strict,
            format,
        } => {
            run_smoke_host(SmokeHostOptions {
                workspace,
                bridge_trace,
                meter_log,
                out,
                check,
                strict,
                format,
            })?;
        }
        Commands::PublishPlan {
            workspace,
            out,
            check,
            format,
        } => {
            run_publish_plan(&workspace, out.as_deref(), check, &format)?;
        }
        Commands::CratePackage {
            workspace,
            out,
            check,
            format,
        } => {
            run_crate_package(&workspace, out.as_deref(), check, &format)?;
        }
        Commands::NpmPack {
            workspace,
            out,
            check,
            format,
        } => {
            run_npm_pack(&workspace, out.as_deref(), check, &format)?;
        }
        Commands::DependencyBaseline {
            workspace,
            out,
            check,
            latest,
            format,
        } => {
            run_dependency_baseline(&workspace, out.as_deref(), check, latest, &format)?;
        }
        Commands::ExportTypes { out, check } => {
            if check {
                check_protocol_export(&out)?;
                println!("protocol export matches: {out}");
            } else {
                let report = vesty_ipc::export_protocol_bindings(&out)?;
                println!("typescript: {}", report.typescript_dir.display());
                println!("json schema: {}", report.json_schema_dir.display());
                println!("typescript files: {}", report.typescript_files);
                println!("json schema files: {}", report.json_schema_files);
            }
        }
        Commands::Vst3Sdk { command } => match command {
            Vst3SdkCommand::Manifest {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_manifest(sdk_dir, out, check, &format)?;
            }
            Vst3SdkCommand::BindingPlan {
                sdk_dir,
                bindings_module,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_binding_plan(sdk_dir, bindings_module, out, check, &format)?;
            }
            Vst3SdkCommand::BindingSurface {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_binding_surface(sdk_dir, out, check, &format)?;
            }
            Vst3SdkCommand::EmitScaffold {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_emit_scaffold(sdk_dir, out, check, &format)?;
            }
            Vst3SdkCommand::EmitAbiSeed {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_emit_abi_seed(sdk_dir, out, check, &format)?;
            }
            Vst3SdkCommand::EmitAbi {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_emit_abi(sdk_dir, out, check, &format)?;
            }
            Vst3SdkCommand::EmitInterfaceSkeleton {
                sdk_dir,
                out,
                check,
                format,
            } => {
                run_vst3_sdk_emit_interface_skeleton(sdk_dir, out, check, &format)?;
            }
        },
        Commands::Doctor { format } => {
            run_doctor(&format)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

fn parse_output_format(format: &str) -> Result<OutputFormat, Box<dyn std::error::Error>> {
    match format.trim().to_ascii_lowercase().as_str() {
        "text" | "plain" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!("unsupported output format '{format}'; expected text or json").into()),
    }
}

fn run_vst3_sdk_manifest(
    sdk_dir: Option<Utf8PathBuf>,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let manifest = vesty_vst3_sys::sdk_header_input_manifest(&sdk_dir)?;
    validate_vst3_sdk_header_manifest_shape(&manifest)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <sdk-header-manifest.json>")?;
        let text = read_text_file_no_symlink("VST3 SDK header manifest check input", out_path)?;
        let expected: vesty_vst3_sys::SdkHeaderInputManifest = serde_json::from_str(&text)?;
        vesty_vst3_sys::check_sdk_header_input_manifest(&sdk_dir, &expected)?;
        print_vst3_sdk_manifest_report(&sdk_dir, Some(out_path), &manifest, true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&manifest)? + "\n"))?;
        print_vst3_sdk_manifest_report(&sdk_dir, Some(out_path), &manifest, false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        print_vst3_sdk_manifest_report(&sdk_dir, None, &manifest, false, format)?;
    }

    Ok(())
}

fn run_vst3_sdk_binding_plan(
    sdk_dir: Option<Utf8PathBuf>,
    bindings_module: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let plan = vesty_vst3_sys::generated_bindings_plan(&sdk_dir, &bindings_module)?;
    validate_vst3_sdk_binding_plan_shape(&plan)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <generated-bindings-plan.json>")?;
        let text =
            read_text_file_no_symlink("VST3 SDK generated bindings plan check input", out_path)?;
        let expected: vesty_vst3_sys::GeneratedBindingsPlan = serde_json::from_str(&text)?;
        let mut differences = Vec::new();
        if expected != plan {
            differences = generated_bindings_plan_differences(&expected, &plan);
        }
        if !differences.is_empty() {
            return Err(format!(
                "VST3 SDK generated bindings plan drift: {}",
                differences.join("; ")
            )
            .into());
        }
        print_vst3_sdk_binding_plan_report(&plan, Some(out_path), true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&plan)? + "\n"))?;
        print_vst3_sdk_binding_plan_report(&plan, Some(out_path), false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        print_vst3_sdk_binding_plan_report(&plan, None, false, format)?;
    }

    Ok(())
}

fn run_vst3_sdk_binding_surface(
    sdk_dir: Option<Utf8PathBuf>,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let surface = vesty_vst3_sys::generated_bindings_surface(&sdk_dir)?;
    validate_vst3_sdk_binding_surface_shape(&surface)?;

    if check {
        let out_path = out
            .as_deref()
            .ok_or("--check requires --out <generated-bindings-surface.json>")?;
        let text =
            read_text_file_no_symlink("VST3 SDK generated bindings surface check input", out_path)?;
        let expected: vesty_vst3_sys::GeneratedBindingsSurface = serde_json::from_str(&text)?;
        let differences =
            vesty_vst3_sys::generated_bindings_surface_differences(&expected, &surface);
        if !differences.is_empty() {
            return Err(format!(
                "VST3 SDK generated bindings surface drift: {}",
                differences.join("; ")
            )
            .into());
        }
        print_vst3_sdk_binding_surface_report(&surface, Some(out_path), true, format)?;
        return Ok(());
    }

    if let Some(out_path) = out.as_deref() {
        write_text_file(out_path, &(serde_json::to_string_pretty(&surface)? + "\n"))?;
        print_vst3_sdk_binding_surface_report(&surface, Some(out_path), false, format)?;
    } else if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&surface)?);
    } else {
        print_vst3_sdk_binding_surface_report(&surface, None, false, format)?;
    }

    Ok(())
}

fn run_vst3_sdk_emit_scaffold(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let scaffold = vesty_vst3_sys::generated_bindings_scaffold(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings scaffold check input", &out)?;
        if expected != scaffold.module {
            return Err(format!("VST3 SDK generated bindings scaffold drift: {out}").into());
        }
        print_vst3_sdk_emit_scaffold_report(&scaffold, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &scaffold.module)?;
    print_vst3_sdk_emit_scaffold_report(&scaffold, &out, false, format)?;
    Ok(())
}

fn run_vst3_sdk_emit_abi_seed(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let seed = vesty_vst3_sys::generated_bindings_abi_seed(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings ABI seed check input", &out)?;
        if expected != seed.module {
            return Err(format!("VST3 SDK generated bindings ABI seed drift: {out}").into());
        }
        print_vst3_sdk_emit_abi_seed_report(&seed, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &seed.module)?;
    print_vst3_sdk_emit_abi_seed_report(&seed, &out, false, format)?;
    Ok(())
}

fn run_vst3_sdk_emit_abi(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let abi = vesty_vst3_sys::generated_bindings_abi(&sdk_dir, &out)?;

    if check {
        let expected =
            read_text_file_no_symlink("VST3 SDK generated bindings ABI layout check input", &out)?;
        if expected != abi.module {
            return Err(format!("VST3 SDK generated bindings ABI layout drift: {out}").into());
        }
        print_vst3_sdk_emit_abi_report(&abi, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &abi.module)?;
    print_vst3_sdk_emit_abi_report(&abi, &out, false, format)?;
    Ok(())
}

fn run_vst3_sdk_emit_interface_skeleton(
    sdk_dir: Option<Utf8PathBuf>,
    out: Utf8PathBuf,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let sdk_dir = resolve_vst3_sdk_dir(sdk_dir)?;
    let skeleton = vesty_vst3_sys::generated_bindings_interface_skeleton(&sdk_dir, &out)?;

    if check {
        let expected = read_text_file_no_symlink(
            "VST3 SDK generated bindings interface skeleton check input",
            &out,
        )?;
        if expected != skeleton.module {
            return Err(
                format!("VST3 SDK generated bindings interface skeleton drift: {out}").into(),
            );
        }
        print_vst3_sdk_emit_interface_skeleton_report(&skeleton, &out, true, format)?;
        return Ok(());
    }

    write_text_file(&out, &skeleton.module)?;
    print_vst3_sdk_emit_interface_skeleton_report(&skeleton, &out, false, format)?;
    Ok(())
}

fn generated_bindings_plan_differences(
    expected: &vesty_vst3_sys::GeneratedBindingsPlan,
    actual: &vesty_vst3_sys::GeneratedBindingsPlan,
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
    if expected.status != actual.status {
        differences.push(format!(
            "status expected {} actual {}",
            expected.status, actual.status
        ));
    }
    if expected.bindings_generated != actual.bindings_generated {
        differences.push(format!(
            "bindings_generated expected {} actual {}",
            expected.bindings_generated, actual.bindings_generated
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
    if expected.active_backend != actual.active_backend {
        differences.push(format!(
            "active backend expected {} actual {}",
            expected.active_backend, actual.active_backend
        ));
    }
    if expected.sdk_dir != actual.sdk_dir {
        differences.push(format!(
            "sdk dir expected {} actual {}",
            expected.sdk_dir, actual.sdk_dir
        ));
    }
    if expected.bindings_module != actual.bindings_module {
        differences.push(format!(
            "bindings module expected {} actual {}",
            expected.bindings_module, actual.bindings_module
        ));
    }
    differences.extend(vesty_vst3_sys::sdk_header_manifest_differences(
        &expected.header_manifest,
        &actual.header_manifest,
    ));
    if expected.blockers != actual.blockers {
        differences.push(format!(
            "blockers expected {:?} actual {:?}",
            expected.blockers, actual.blockers
        ));
    }
    if expected.checks != actual.checks {
        differences.push("checks changed".to_string());
    }
    if expected.next_steps != actual.next_steps {
        differences.push("next steps changed".to_string());
    }
    differences
}

fn resolve_vst3_sdk_dir(
    sdk_dir: Option<Utf8PathBuf>,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    resolve_vst3_sdk_dir_from_env_value(sdk_dir, std::env::var_os(vesty_vst3_sys::VST3_SDK_DIR_ENV))
}

fn resolve_vst3_sdk_dir_from_env_value(
    sdk_dir: Option<Utf8PathBuf>,
    env_value: Option<std::ffi::OsString>,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if let Some(sdk_dir) = sdk_dir {
        return Ok(sdk_dir);
    }
    let raw = env_value.ok_or_else(|| {
        format!(
            "pass --sdk-dir <path> or set {}",
            vesty_vst3_sys::VST3_SDK_DIR_ENV
        )
    })?;
    Utf8PathBuf::from_path_buf(std::path::PathBuf::from(raw)).map_err(|path| {
        format!(
            "{} is not valid UTF-8: {}",
            vesty_vst3_sys::VST3_SDK_DIR_ENV,
            path.display()
        )
        .into()
    })
}

fn print_vst3_sdk_manifest_report(
    sdk_dir: &Utf8Path,
    out: Option<&Utf8Path>,
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_header_manifest_shape(manifest)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(manifest)?);
        }
        OutputFormat::Text => {
            let status = if checked {
                "matches"
            } else if manifest.complete {
                "complete"
            } else {
                "incomplete"
            };
            println!("VST3 SDK header manifest: {status}");
            println!("sdk dir: {sdk_dir}");
            if let Some(out) = out {
                println!("manifest: {out}");
            }
            println!("baseline: {}", manifest.steinberg_sdk_baseline);
            println!("generator: {}", manifest.generator);
            println!("headers: {}", manifest.headers.len());
            println!("missing headers: {}", manifest.missing_headers.len());
            if !manifest.missing_headers.is_empty() {
                println!("missing: {}", manifest.missing_headers.join(", "));
            }
            if let Some(version_hint) = manifest.version_hint.as_deref() {
                println!("version hint: {version_hint}");
            }
        }
    }
    Ok(())
}

fn print_vst3_sdk_binding_plan_report(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
    out: Option<&Utf8Path>,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_plan_shape(plan)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(plan)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings plan: {}",
                if checked {
                    "matches"
                } else {
                    plan.status.as_str()
                }
            );
            println!("sdk dir: {}", plan.sdk_dir);
            println!("bindings module: {}", plan.bindings_module);
            if let Some(out) = out {
                println!("plan: {out}");
            }
            println!("active backend: {}", plan.active_backend);
            println!("bindings generated: {}", plan.bindings_generated);
            println!("headers: {}", plan.header_manifest.headers.len());
            println!(
                "missing headers: {}",
                plan.header_manifest.missing_headers.len()
            );
            if !plan.blockers.is_empty() {
                println!("blockers: {}", plan.blockers.join("; "));
            }
        }
    }
    Ok(())
}

fn print_vst3_sdk_binding_surface_report(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
    out: Option<&Utf8Path>,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_surface_shape(surface)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(surface)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings surface: {}",
                if checked {
                    "matches"
                } else {
                    surface.status.as_str()
                }
            );
            println!("sdk dir: {}", surface.sdk_dir);
            if let Some(out) = out {
                println!("surface: {out}");
            }
            println!("active backend: {}", surface.active_backend);
            println!("bindings generated: {}", surface.bindings_generated);
            println!("required headers: {}", surface.required_headers.len());
            println!("symbols: {}", surface.symbols.len());
            println!(
                "missing headers: {}",
                surface.header_manifest.missing_headers.len()
            );
            if !surface.missing_headers.is_empty() {
                println!(
                    "missing surface headers: {}",
                    surface.missing_headers.join("; ")
                );
            }
            println!("missing symbols: {}", surface.missing_symbols.len());
            if !surface.missing_symbols.is_empty() {
                println!(
                    "missing surface symbols: {}",
                    surface.missing_symbols.join("; ")
                );
            }
            if !surface.blockers.is_empty() {
                println!("blockers: {}", surface.blockers.join("; "));
            }
        }
    }
    Ok(())
}

fn print_vst3_sdk_emit_scaffold_report(
    scaffold: &vesty_vst3_sys::GeneratedBindingsScaffold,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR,
                "bindingsGenerated": false,
                "planStatus": scaffold.plan.status,
                "headerCount": scaffold.plan.header_manifest.headers.len(),
                "activeBackend": scaffold.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings scaffold: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR
            );
            println!("bindings generated: false");
            println!("plan status: {}", scaffold.plan.status);
            println!("active backend: {}", scaffold.plan.active_backend);
            println!("headers: {}", scaffold.plan.header_manifest.headers.len());
        }
    }
    Ok(())
}

fn print_vst3_sdk_emit_abi_seed_report(
    seed: &vesty_vst3_sys::GeneratedBindingsAbiSeed,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR,
                "abiSeedGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": seed.plan.status,
                "surfaceStatus": seed.surface.status,
                "headerCount": seed.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": seed.surface.symbols.len(),
                "activeBackend": seed.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings ABI seed: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR
            );
            println!("ABI seed generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", seed.plan.status);
            println!("surface status: {}", seed.surface.status);
            println!("active backend: {}", seed.plan.active_backend);
            println!("headers: {}", seed.plan.header_manifest.headers.len());
            println!("surface symbols: {}", seed.surface.symbols.len());
        }
    }
    Ok(())
}

fn print_vst3_sdk_emit_abi_report(
    abi: &vesty_vst3_sys::GeneratedBindingsAbi,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR,
                "abiLayoutGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": abi.plan.status,
                "surfaceStatus": abi.surface.status,
                "headerCount": abi.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": abi.surface.symbols.len(),
                "activeBackend": abi.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings ABI layout: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR
            );
            println!("ABI layout generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", abi.plan.status);
            println!("surface status: {}", abi.surface.status);
            println!("active backend: {}", abi.plan.active_backend);
            println!("headers: {}", abi.plan.header_manifest.headers.len());
            println!("surface symbols: {}", abi.surface.symbols.len());
        }
    }
    Ok(())
}

fn print_vst3_sdk_emit_interface_skeleton_report(
    skeleton: &vesty_vst3_sys::GeneratedBindingsInterfaceSkeleton,
    out: &Utf8Path,
    checked: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let report = serde_json::json!({
                "status": if checked { "matches" } else { "written" },
                "path": out.as_str(),
                "generator": vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR,
                "interfaceSkeletonGenerated": true,
                "bindingsGenerated": false,
                "fullComBindingsGenerated": false,
                "planStatus": skeleton.plan.status,
                "surfaceStatus": skeleton.surface.status,
                "headerCount": skeleton.plan.header_manifest.headers.len(),
                "surfaceSymbolCount": skeleton.surface.symbols.len(),
                "interfaceCount": skeleton
                    .surface
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == "interface")
                    .count(),
                "activeBackend": skeleton.plan.active_backend,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!(
                "VST3 SDK generated bindings interface skeleton: {}",
                if checked { "matches" } else { "written" }
            );
            println!("module: {out}");
            println!(
                "generator: {}",
                vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR
            );
            println!("interface skeleton generated: true");
            println!("bindings generated: false");
            println!("full COM bindings generated: false");
            println!("plan status: {}", skeleton.plan.status);
            println!("surface status: {}", skeleton.surface.status);
            println!("active backend: {}", skeleton.plan.active_backend);
            println!("headers: {}", skeleton.plan.header_manifest.headers.len());
            println!("surface symbols: {}", skeleton.surface.symbols.len());
            println!(
                "interfaces: {}",
                skeleton
                    .surface
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == "interface")
                    .count()
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CollectLocalOptions {
    template: bool,
    protocol: bool,
    publish_plan: bool,
    crate_package: bool,
    npm_pack: bool,
    dependency_baseline_latest: bool,
    vst3_sdk_dir: Option<Utf8PathBuf>,
    vst3_sdk_bindings_module: Utf8PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LocalReleaseEvidenceReport {
    evidence_dir: String,
    workspace: String,
    protocol_snapshot: Option<String>,
    items: Vec<LocalReleaseEvidenceItem>,
    external_evidence_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LocalReleaseEvidenceItem {
    name: String,
    status: String,
    path: Option<String>,
    value: String,
}

const RELEASE_EVIDENCE_REPORT_MAX_ITEMS: usize = 1024;

fn collect_local_release_evidence(
    workspace: &Utf8Path,
    dir: &Utf8Path,
    protocol_snapshot: &Utf8Path,
    options: CollectLocalOptions,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", dir)?;
    let mut items = Vec::new();

    if options.template {
        let created = write_release_evidence_templates(&dir.to_path_buf())?;
        items.push(LocalReleaseEvidenceItem {
            name: "release evidence template".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(dir)),
            value: format!("{created} template file(s) created; existing files preserved"),
        });
    }

    if options.protocol {
        let report = vesty_ipc::export_protocol_bindings(protocol_snapshot)?;
        check_protocol_export(protocol_snapshot)?;
        items.push(LocalReleaseEvidenceItem {
            name: "protocol snapshot".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(protocol_snapshot)),
            value: format!(
                "{} TypeScript file(s), {} JSON schema file(s)",
                report.typescript_files, report.json_schema_files
            ),
        });
    }

    if options.publish_plan {
        let path = dir.join("publish-plan/publish-plan.json");
        let metadata = cargo_metadata_json(workspace)?;
        let plan = workspace_publish_plan(&metadata)?;
        write_publish_plan_report(&path, &plan)?;
        let evidence = validate_publish_plan_report(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "crate publish plan".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} publishable crates; {} private skipped; final crate: {}",
                evidence.package_count, evidence.skipped_private_count, evidence.final_package
            ),
        });
    }

    if options.crate_package {
        let path = dir.join("crate-package/crate-package.json");
        let report = crate_package_report(workspace, true)?;
        write_crate_package_report(&path, &report)?;
        let evidence = validate_crate_package_report_path(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "crate package readiness".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} packageable now; {} deferred until internal dependencies publish",
                evidence.packaged_count, evidence.deferred_count
            ),
        });
    }

    if options.npm_pack {
        let path = dir.join("npm-pack/npm-pack.json");
        let raw_report = npm_pack_dry_run_json(workspace)?;
        let entries = parse_npm_pack_report_text(&raw_report)?;
        let evidence = validate_npm_pack_entries(&entries)?;
        write_text_file(&path, &(serde_json::to_string_pretty(&entries)? + "\n"))?;
        items.push(LocalReleaseEvidenceItem {
            name: "npm package pack report".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} package(s), {} file(s): {}",
                evidence.package_count,
                evidence.total_files,
                evidence.packages.join(", ")
            ),
        });
    }

    if options.dependency_baseline_latest {
        let path = dir.join("dependency-baseline/dependency-baseline-latest.json");
        let report =
            dependency_baseline_report_with_latest(workspace, &CommandLatestDependencyFetcher)?;
        write_dependency_baseline_report(&path, &report)?;
        let evidence = validate_dependency_baseline_latest_report(&path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "dependency latest baseline".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&path)),
            value: format!(
                "{} baseline check(s), {} latest registry check(s)",
                evidence.baseline_checks, evidence.latest_checks
            ),
        });
    }

    if let Some(sdk_dir) = options.vst3_sdk_dir.as_deref() {
        let manifest_path = dir.join("vst3-sdk/vst3-sdk-headers.json");
        let manifest = vesty_vst3_sys::sdk_header_input_manifest(sdk_dir)?;
        write_text_file(
            &manifest_path,
            &(serde_json::to_string_pretty(&manifest)? + "\n"),
        )?;
        let manifest_evidence = validate_vst3_sdk_header_manifest(&manifest_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&manifest_path)),
            value: format!(
                "{} required header(s); Steinberg SDK {}; upstream vst3 crate {}",
                manifest_evidence.header_count,
                manifest_evidence.baseline,
                manifest_evidence.upstream_vst3_crate
            ),
        });

        let binding_plan_path = dir.join("vst3-sdk/generated-bindings-plan.json");
        let binding_plan =
            vesty_vst3_sys::generated_bindings_plan(sdk_dir, &options.vst3_sdk_bindings_module)?;
        write_text_file(
            &binding_plan_path,
            &(serde_json::to_string_pretty(&binding_plan)? + "\n"),
        )?;
        let plan_evidence = validate_vst3_sdk_binding_plan(&binding_plan_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&binding_plan_path)),
            value: format!(
                "{}; {} required header(s); active backend {}; module {}",
                plan_evidence.status,
                plan_evidence.header_count,
                plan_evidence.active_backend,
                plan_evidence.bindings_module
            ),
        });

        let binding_surface_path = dir.join("vst3-sdk/generated-bindings-surface.json");
        let binding_surface = vesty_vst3_sys::generated_bindings_surface(sdk_dir)?;
        write_text_file(
            &binding_surface_path,
            &(serde_json::to_string_pretty(&binding_surface)? + "\n"),
        )?;
        let surface_evidence = validate_vst3_sdk_binding_surface(&binding_surface_path)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&binding_surface_path)),
            value: format!(
                "{}; {} required header(s); {} symbol(s); active backend {}; bindings generated false",
                surface_evidence.status,
                surface_evidence.header_count,
                surface_evidence.symbol_count,
                surface_evidence.active_backend
            ),
        });

        let scaffold_path = dir.join("vst3-sdk/generated.rs");
        let scaffold = vesty_vst3_sys::generated_bindings_scaffold(
            sdk_dir,
            &options.vst3_sdk_bindings_module,
        )?;
        write_text_file(&scaffold_path, &scaffold.module)?;
        validate_vst3_sdk_generated_bindings_scaffold_text(&scaffold.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings scaffold".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&scaffold_path)),
            value: format!(
                "metadata scaffold; {} required header(s); active backend {}; bindings generated false",
                scaffold.plan.header_manifest.headers.len(),
                scaffold.plan.active_backend
            ),
        });

        let abi_seed_path = dir.join("vst3-sdk/generated-abi-seed.rs");
        let abi_seed = vesty_vst3_sys::generated_bindings_abi_seed(sdk_dir, &abi_seed_path)?;
        write_text_file(&abi_seed_path, &abi_seed.module)?;
        validate_vst3_sdk_generated_bindings_abi_seed_text(&abi_seed.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings ABI seed".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&abi_seed_path)),
            value: format!(
                "ABI seed aliases/constants; {} required header(s); {} symbol(s); active backend {}; full COM bindings generated false",
                abi_seed.plan.header_manifest.headers.len(),
                abi_seed.surface.symbols.len(),
                abi_seed.plan.active_backend
            ),
        });

        let abi_path = dir.join("vst3-sdk/generated-abi.rs");
        let abi = vesty_vst3_sys::generated_bindings_abi(sdk_dir, &abi_path)?;
        write_text_file(&abi_path, &abi.module)?;
        validate_vst3_sdk_generated_bindings_abi_text(&abi.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings ABI layout".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&abi_path)),
            value: format!(
                "ABI layout module; {} required header(s); {} symbol(s); active backend {}; layout fingerprints present; full COM bindings generated false",
                abi.plan.header_manifest.headers.len(),
                abi.surface.symbols.len(),
                abi.plan.active_backend
            ),
        });

        let interface_skeleton_path = dir.join("vst3-sdk/generated-interface-skeleton.rs");
        let interface_skeleton = vesty_vst3_sys::generated_bindings_interface_skeleton(
            sdk_dir,
            &interface_skeleton_path,
        )?;
        write_text_file(&interface_skeleton_path, &interface_skeleton.module)?;
        validate_vst3_sdk_generated_bindings_interface_skeleton_text(&interface_skeleton.module)?;
        items.push(LocalReleaseEvidenceItem {
            name: "vst3 SDK generated bindings interface skeleton".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&interface_skeleton_path)),
            value: format!(
                "interface/vtable skeleton module; {} required header(s); {} symbol(s); active backend {}; full COM bindings generated false",
                interface_skeleton.plan.header_manifest.headers.len(),
                interface_skeleton.surface.symbols.len(),
                interface_skeleton.plan.active_backend
            ),
        });
    }

    let report = LocalReleaseEvidenceReport {
        evidence_dir: portable_report_path(dir),
        workspace: portable_report_path(workspace),
        protocol_snapshot: options
            .protocol
            .then(|| portable_report_path(protocol_snapshot)),
        items,
        external_evidence_note: "collect-local only gathers local protocol/publish/npm, opt-in dependency latest review and explicitly requested VST3 SDK audit evidence; DAW matrix, platform smoke, validator, CI, signing and notarization evidence must come from real external runs".to_string(),
    };
    validate_local_release_evidence_report_shape(&report)?;
    let report_path = dir.join("local-collect-report.json");
    write_text_file(
        &report_path,
        &(serde_json::to_string_pretty(&report)? + "\n"),
    )?;
    print_local_release_evidence_report(&report, format, &report_path)
}

fn print_local_release_evidence_report(
    report: &LocalReleaseEvidenceReport,
    format: OutputFormat,
    report_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_local_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputFormat::Text => {
            println!("local release evidence: {}", report.evidence_dir);
            for item in &report.items {
                let path = item
                    .path
                    .as_deref()
                    .map(|path| format!(" ({path})"))
                    .unwrap_or_default();
                println!("- {}: {}{} - {}", item.name, item.status, path, item.value);
            }
            println!("- report: {report_path}");
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct ImportCiOptions {
    source: Utf8PathBuf,
    dir: Utf8PathBuf,
    ci_run_url: Option<String>,
    ci_run_url_file: Option<Utf8PathBuf>,
    template: bool,
    overwrite: bool,
    format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ImportCiReleaseEvidenceReport {
    evidence_dir: String,
    source: String,
    items: Vec<ImportCiReleaseEvidenceItem>,
    external_evidence_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ImportCiReleaseEvidenceItem {
    name: String,
    status: String,
    source: Option<String>,
    path: Option<String>,
    value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImportWriteOutcome {
    Imported,
    SkippedExisting,
}

impl ImportWriteOutcome {
    const fn status(self) -> &'static str {
        match self {
            ImportWriteOutcome::Imported => "imported",
            ImportWriteOutcome::SkippedExisting => "skipped",
        }
    }

    const fn value(self) -> &'static str {
        match self {
            ImportWriteOutcome::Imported => "copied into release evidence",
            ImportWriteOutcome::SkippedExisting => {
                "destination exists; pass --overwrite to replace"
            }
        }
    }
}

fn import_ci_release_evidence(options: ImportCiOptions) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    require_existing_directory_no_symlink("CI artifact source", &options.source)?;
    reject_existing_path_symlink("release evidence dir", &options.dir)?;
    reject_existing_output_parent_symlink("release evidence dir", &options.dir)?;
    validate_import_ci_source_and_dir(&options.source, &options.dir)?;
    fs::create_dir_all(&options.dir)?;
    require_existing_directory_no_symlink("release evidence dir", &options.dir)?;

    let mut items = Vec::new();
    if options.template {
        let created = write_release_evidence_templates(&options.dir)?;
        items.push(import_ci_item(
            "release evidence template",
            "ok",
            None,
            Some(options.dir.as_path()),
            format!("{created} template file(s) created; existing files preserved"),
        ));
    }

    let text_files = collect_evidence_text_files_recursive(&options.source)?;
    let ci_run_url = import_ci_run_url_evidence(&options, &text_files, &mut items)?;
    let protocol_snapshot_dirs = import_protocol_snapshot_evidence(&options, &mut items)?;

    for path in collect_json_files_recursive(&options.source)? {
        if path_is_under_any(&path, &protocol_snapshot_dirs) {
            continue;
        }
        import_ci_json_artifact(&path, &options, ci_run_url.as_deref(), &mut items)?;
    }
    for path in collect_rust_files_recursive(&options.source)? {
        import_ci_rust_artifact(&path, &options, &mut items)?;
    }
    for path in text_files {
        import_ci_text_artifact(&path, &options, &mut items)?;
    }
    for path in collect_vst3_bundle_dirs_recursive(&options.source)? {
        import_ci_signed_bundle_artifact(&path, &options, &mut items)?;
    }

    let report = ImportCiReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        source: portable_report_path(&options.source),
        items,
        external_evidence_note: "import-ci only copies artifacts whose content is recognized by the release evidence parsers; it does not create DAW, platform smoke, validator-passed, signing or notarization passes by itself".to_string(),
    };
    validate_import_ci_release_evidence_report_shape(&report)?;
    let report_path = options.dir.join("import-ci-report.json");
    write_text_file(
        &report_path,
        &(serde_json::to_string_pretty(&report)? + "\n"),
    )?;
    print_import_ci_release_evidence_report(&report, format, &report_path)
}

fn validate_import_ci_source_and_dir(
    source: &Utf8Path,
    dir: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = canonicalize_utf8(source)?;
    let dir = canonicalize_existing_or_parent(dir)?;
    if source == dir {
        return Err("CI artifact source and release evidence dir must be different".into());
    }
    if dir.starts_with(&source) {
        return Err(format!(
            "release evidence dir must not be inside CI artifact source: {dir} is under {source}"
        )
        .into());
    }
    if source.starts_with(&dir) {
        return Err(format!(
            "CI artifact source must not be inside release evidence dir: {source} is under {dir}"
        )
        .into());
    }
    Ok(())
}

fn canonicalize_existing_or_parent(
    path: &Utf8Path,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let absolute = absolute_utf8_path(path)?;
    if absolute.exists() {
        return canonicalize_utf8(&absolute);
    }

    let mut cursor = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        if cursor.exists() {
            let mut canonical = canonicalize_utf8(cursor)?;
            for part in suffix.iter().rev() {
                canonical.push(part);
            }
            return Ok(canonical);
        }
        let Some(file_name) = cursor.file_name() else {
            return Err(format!("cannot resolve parent for path: {path}").into());
        };
        suffix.push(file_name.to_string());
        cursor = cursor
            .parent()
            .ok_or_else(|| format!("cannot resolve parent for path: {path}"))?;
    }
}

fn absolute_utf8_path(path: &Utf8Path) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| "current directory is not valid utf-8")?;
    Ok(cwd.join(path))
}

fn canonicalize_utf8(path: &Utf8Path) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    Utf8PathBuf::from_path_buf(path.canonicalize()?)
        .map_err(|_| format!("path is not valid utf-8 after canonicalization: {path}").into())
}

fn import_ci_run_url_evidence(
    options: &ImportCiOptions,
    text_files: &[Utf8PathBuf],
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let discovered = match (
        options.ci_run_url.as_deref(),
        options.ci_run_url_file.as_deref(),
    ) {
        (Some(url), Some(path)) => {
            let cli_url = url.trim();
            let file_url = read_required_ci_run_url_file(path)?;
            validate_import_ci_explicit_run_urls_match(cli_url, &file_url, path)?;
            Some(DiscoveredCiRunUrl {
                url: cli_url.to_string(),
                source: Some(path.to_path_buf()),
                had_failed_candidate: false,
            })
        }
        (Some(url), None) => Some(DiscoveredCiRunUrl {
            url: url.trim().to_string(),
            source: None,
            had_failed_candidate: false,
        }),
        (None, Some(path)) => Some(DiscoveredCiRunUrl {
            url: read_required_ci_run_url_file(path)?,
            source: Some(path.to_path_buf()),
            had_failed_candidate: false,
        }),
        (None, None) => auto_discover_ci_run_url_evidence(text_files, items)?,
    };

    let Some(discovered) = discovered else {
        items.push(import_ci_item(
            "ci run url",
            "skipped",
            None,
            None,
            "no GitHub Actions run URL found; pass --ci-run-url or --ci-run-url-file",
        ));
        return Ok(None);
    };
    if discovered.url.is_empty() {
        if discovered.had_failed_candidate {
            return Ok(None);
        }
        items.push(import_ci_item(
            "ci run url",
            "skipped",
            None,
            None,
            "no GitHub Actions run URL found; pass --ci-run-url or --ci-run-url-file",
        ));
        return Ok(None);
    }
    if !is_github_actions_run_url(&discovered.url) {
        return Err(format!("invalid GitHub Actions run URL: {}", discovered.url).into());
    }

    let destination = options.dir.join("ci-run-url.txt");
    let outcome = import_write_ci_run_url_file(&destination, &discovered.url, options.overwrite)?;
    items.push(import_ci_item(
        "ci run url",
        outcome.status(),
        discovered.source.as_deref(),
        Some(destination.as_path()),
        if matches!(outcome, ImportWriteOutcome::Imported) {
            discovered.url.clone()
        } else {
            outcome.value().to_string()
        },
    ));
    Ok(Some(discovered.url))
}

#[derive(Clone, Debug)]
struct DiscoveredCiRunUrl {
    url: String,
    source: Option<Utf8PathBuf>,
    had_failed_candidate: bool,
}

fn auto_discover_ci_run_url_evidence(
    text_files: &[Utf8PathBuf],
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Option<DiscoveredCiRunUrl>, Box<dyn std::error::Error>> {
    let mut had_failed_candidate = false;
    for path in text_files {
        let Some(url) = read_ci_run_url_file(path)? else {
            continue;
        };
        if is_github_actions_run_url(&url) {
            return Ok(Some(DiscoveredCiRunUrl {
                url,
                source: Some(path.clone()),
                had_failed_candidate,
            }));
        }
        if ci_run_url_evidence_path(path) {
            had_failed_candidate = true;
            items.push(import_ci_item(
                "ci run url",
                "failed",
                Some(path),
                None,
                format!("invalid GitHub Actions run URL in CI run URL evidence: {url}"),
            ));
        }
    }
    Ok(had_failed_candidate.then_some(DiscoveredCiRunUrl {
        url: String::new(),
        source: None,
        had_failed_candidate: true,
    }))
}

fn ci_run_url_evidence_path(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "ci-run-url.txt" | "ci_run_url.txt" | "ci-run-url.log" | "ci_run_url.log"
        )
    })
}

fn read_required_ci_run_url_file(path: &Utf8Path) -> Result<String, Box<dyn std::error::Error>> {
    read_ci_run_url_file(path)?.ok_or_else(|| {
        format!("CI run URL file did not contain a valid GitHub Actions run URL: {path}").into()
    })
}

fn validate_import_ci_explicit_run_urls_match(
    cli_url: &str,
    file_url: &str,
    file_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_explicit_ci_run_urls_match(
        cli_url,
        "--ci-run-url",
        file_url,
        &format!("--ci-run-url-file {file_path}"),
    )
}

fn validate_explicit_ci_run_urls_match(
    left_url: &str,
    left_label: &str,
    right_url: &str,
    right_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let left_url = left_url.trim();
    let right_url = right_url.trim();
    let Some(left_run) = github_actions_run_key(left_url) else {
        return Err(
            format!("{left_label} is not a valid GitHub Actions run URL: {left_url}").into(),
        );
    };
    let Some(right_run) = github_actions_run_key(right_url) else {
        return Err(
            format!("{right_label} is not a valid GitHub Actions run URL: {right_url}").into(),
        );
    };
    if left_run == right_run {
        return Ok(());
    }
    Err(format!(
        "{left_label} ({}/{}, run {}) and {right_label} ({}/{}, run {}) refer to different GitHub Actions runs",
        left_run.owner,
        left_run.repo,
        left_run.run_id,
        right_run.owner,
        right_run.repo,
        right_run.run_id,
    )
    .into())
}

fn import_protocol_snapshot_evidence(
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let candidates = collect_protocol_snapshot_dirs_recursive(&options.source)?;
    if candidates.is_empty() {
        items.push(import_ci_item(
            "protocol snapshot",
            "skipped",
            None,
            None,
            "no protocol snapshot directory found",
        ));
        return Ok(Vec::new());
    }

    let destination = options.dir.join("vesty-protocol");
    for candidate in &candidates {
        match check_protocol_export(candidate) {
            Ok(()) => {
                let outcome = import_copy_dir_contents(candidate, &destination, options.overwrite)?;
                items.push(import_ci_item(
                    "protocol snapshot",
                    outcome.status(),
                    Some(candidate.as_path()),
                    Some(destination.as_path()),
                    outcome.value(),
                ));
                return Ok(candidates);
            }
            Err(error) => items.push(import_ci_item(
                "protocol snapshot",
                "failed",
                Some(candidate.as_path()),
                None,
                error.to_string(),
            )),
        }
    }
    Ok(candidates)
}

fn import_ci_json_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("CI JSON artifact", path)?;

    if let Ok(report) = serde_json::from_str::<ReleaseCheckReport>(&text) {
        return import_ci_release_check_artifact(path, report, options, expected_ci_run_url, items);
    }
    if let Ok(plan) = serde_json::from_str::<ReleaseActionPlan>(&text) {
        return import_ci_release_action_plan_artifact(path, &plan, options, items);
    }
    if let Ok(report) = serde_json::from_str::<PlatformSmokeReport>(&text) {
        return import_ci_platform_smoke_artifact(path, report, options, items);
    }
    if let Ok(surface) = serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsSurface>(&text) {
        return import_ci_vst3_sdk_binding_surface_artifact(path, &surface, options, items);
    }
    if let Ok(plan) = serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsPlan>(&text) {
        return import_ci_vst3_sdk_binding_plan_artifact(path, &plan, options, items);
    }
    if let Ok(manifest) = serde_json::from_str::<vesty_vst3_sys::SdkHeaderInputManifest>(&text) {
        return import_ci_vst3_sdk_manifest_artifact(path, &manifest, options, items);
    }
    if let Ok(report) = serde_json::from_str::<DoctorReport>(&text) {
        return import_ci_doctor_artifact(path, report, options, expected_ci_run_url, items);
    }
    if let Ok(report) = serde_json::from_str::<ValidateReport>(&text) {
        return import_ci_validate_artifact(path, report, options, items);
    }
    if serde_json::from_str::<PublishPlan>(&text).is_ok() {
        return import_ci_publish_plan_artifact(path, options, items);
    }
    if serde_json::from_str::<CratePackageReport>(&text).is_ok() {
        return import_ci_crate_package_artifact(path, options, items);
    }
    if serde_json::from_str::<DependencyBaselineReport>(&text).is_ok() {
        return import_ci_dependency_baseline_artifact(path, options, items);
    }
    if parse_npm_pack_report_text(&text).is_ok() {
        return import_ci_npm_pack_artifact(path, options, items);
    }
    if let Some((name, reason)) = recognized_json_artifact_name_from_path(path) {
        let parse_error = serde_json::from_str::<serde_json::Value>(&text)
            .err()
            .map(|error| format!("invalid JSON: {error}"))
            .unwrap_or_else(|| {
                "JSON did not match the expected release evidence schema".to_string()
            });
        items.push(import_ci_item(
            name,
            "failed",
            Some(path),
            None,
            format!("{reason}; {parse_error}"),
        ));
        return Ok(());
    }
    items.push(import_ci_item(
        "json artifact",
        "skipped",
        Some(path),
        None,
        "unrecognized JSON artifact",
    ));
    Ok(())
}

fn recognized_json_artifact_name_from_path(path: &Utf8Path) -> Option<(&'static str, String)> {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name()?.to_ascii_lowercase();
    let stem_lower = path.file_stem().unwrap_or("").to_ascii_lowercase();

    let recognized = if file_lower.starts_with("doctor-")
        || path_lower.contains("/ci-doctor/")
        || path_lower.contains("/doctor-")
    {
        Some(("ci doctor artifact", "path looks like a CI doctor artifact"))
    } else if file_lower.starts_with("release-check-")
        || path_lower.contains("/ci-release-checks/")
        || path_lower.contains("/release-check-")
    {
        Some((
            "ci release-check artifact",
            "path looks like a CI release-check artifact",
        ))
    } else if file_lower.starts_with("release-action-plan-")
        || path_lower.contains("/release-action-plan-")
    {
        Some((
            "release action plan sidecar",
            "path looks like a release action plan sidecar",
        ))
    } else if file_lower.contains("platform-smoke") || path_lower.contains("/platform-smoke/") {
        Some((
            "platform smoke artifact",
            "path looks like a platform smoke artifact",
        ))
    } else if file_lower.contains("static-validate")
        || path_lower.contains("/static-validate")
        || path_lower.contains("/package/")
    {
        Some((
            "vst3 static validate report",
            "path looks like a VST3 static validate report",
        ))
    } else if file_lower.contains("validate") || path_lower.contains("/validator/") {
        Some((
            "vst3 validate report",
            "path looks like a VST3 validate report",
        ))
    } else if file_lower == "publish-plan.json" || path_lower.contains("/publish-plan/") {
        Some(("crate publish plan", "path looks like a crate publish plan"))
    } else if file_lower == "crate-package.json" || path_lower.contains("/crate-package/") {
        Some((
            "crate package readiness",
            "path looks like a crate package readiness report",
        ))
    } else if file_lower == "npm-pack.json" || path_lower.contains("/npm-pack/") {
        Some((
            "npm package pack report",
            "path looks like an npm package pack report",
        ))
    } else if file_lower == "dependency-baseline-latest.json"
        || path_lower.contains("/dependency-baseline/")
    {
        Some((
            "dependency latest baseline",
            "path looks like a dependency latest baseline report",
        ))
    } else if file_lower == "vst3-sdk-headers.json" || stem_lower.contains("vst3-sdk-headers") {
        Some((
            "vst3 SDK header manifest",
            "path looks like a VST3 SDK header manifest",
        ))
    } else if file_lower == "generated-bindings-plan.json"
        || stem_lower.contains("generated-bindings-plan")
    {
        Some((
            "vst3 SDK generated bindings plan",
            "path looks like a VST3 SDK generated bindings plan",
        ))
    } else if file_lower == "generated-bindings-surface.json"
        || stem_lower.contains("generated-bindings-surface")
    {
        Some((
            "vst3 SDK generated bindings surface",
            "path looks like a VST3 SDK generated bindings surface",
        ))
    } else if path_lower.contains("/vst3-sdk/")
        || path_lower.contains("/vesty-vst3-sdk/")
        || stem_lower.contains("vst3-sdk")
    {
        Some((
            "vst3 SDK artifact",
            "path looks like a VST3 SDK release artifact",
        ))
    } else {
        None
    }?;

    Some((recognized.0, recognized.1.to_string()))
}

fn import_ci_rust_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("CI Rust artifact", path)?;
    if is_vst3_sdk_generated_bindings_abi_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_abi_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings ABI layout",
                    path,
                    &options.dir.join("vst3-sdk/generated-abi.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings ABI layout",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if is_vst3_sdk_generated_bindings_abi_seed_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_abi_seed_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings ABI seed",
                    path,
                    &options.dir.join("vst3-sdk/generated-abi-seed.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings ABI seed",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if is_vst3_sdk_generated_bindings_interface_skeleton_candidate(path, &text) {
        match validate_vst3_sdk_generated_bindings_interface_skeleton_text(&text) {
            Ok(()) => {
                return import_copy_file_item(
                    "vst3 SDK generated bindings interface skeleton",
                    path,
                    &options.dir.join("vst3-sdk/generated-interface-skeleton.rs"),
                    options.overwrite,
                    items,
                );
            }
            Err(error) => {
                items.push(import_ci_item(
                    "vst3 SDK generated bindings interface skeleton",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
        }
    }
    if !is_vst3_sdk_generated_bindings_scaffold_candidate(path, &text) {
        return Ok(());
    }

    match validate_vst3_sdk_generated_bindings_scaffold_text(&text) {
        Ok(()) => import_copy_file_item(
            "vst3 SDK generated bindings scaffold",
            path,
            &options.dir.join("vst3-sdk/generated.rs"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings scaffold",
                "failed",
                Some(path),
                None,
                error,
            ));
            Ok(())
        }
    }
}

fn is_vst3_sdk_generated_bindings_scaffold_candidate(path: &Utf8Path, text: &str) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

fn is_vst3_sdk_generated_bindings_abi_candidate(path: &Utf8Path, text: &str) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-abi.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

fn is_vst3_sdk_generated_bindings_abi_seed_candidate(path: &Utf8Path, text: &str) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-abi-seed.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

fn is_vst3_sdk_generated_bindings_interface_skeleton_candidate(
    path: &Utf8Path,
    text: &str,
) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-interface-skeleton.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

fn validate_vst3_sdk_generated_bindings_abi_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR) {
        errors.push(format!(
            "missing ABI layout generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"abi-layout\";") {
        errors.push("missing ABI layout status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("ABI layout plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("ABI layout surface status must be ready-for-binding-emitter".to_string());
    }
    if !text.contains("pub const ABI_LAYOUT_GENERATED: bool = true;") {
        errors.push("ABI layout must explicitly mark ABI_LAYOUT_GENERATED true".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("ABI layout must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI layout must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors.push("ABI layout must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI layout must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push("ABI layout must be generated from a complete header manifest".to_string());
    }
    for required in [
        "#[repr(C)]",
        "pub struct TUID",
        "pub struct FUnknownVTable",
        "pub struct FUnknown",
        "pub struct ViewRect",
        "pub struct ProgramListInfo",
        "pub struct UnitInfo",
        "pub struct NoteExpressionValueDescription",
        "pub struct NoteExpressionTypeInfo",
        "pub struct PhysicalUIMap",
        "pub struct PhysicalUIMapList",
        "pub struct AbiLayoutRecord",
        "pub struct AbiFieldOffset",
        "pub type FUnknownQueryInterface = unsafe extern \"system\" fn(",
        "pub type ParamID = u32;",
        "pub type ParamValue = f64;",
        "pub type TChar = u16;",
        "pub type String128 = [TChar; STRING128_CODE_UNITS];",
        "pub type UnitID = int32;",
        "pub type ProgramListID = int32;",
        "pub type NoteExpressionTypeID = uint32;",
        "pub type NoteExpressionValue = f64;",
        "pub type PhysicalUITypeID = uint32;",
        "pub type Sample32 = f32;",
        "pub type Sample64 = f64;",
        "pub const ABI_LAYOUT_TYPES: &[&str] = &[",
        "pub const STRING128_CODE_UNITS: usize = 128;",
        "pub const PROGRAM_LIST_INFO_FIELD_COUNT: usize = 3;",
        "pub const UNIT_INFO_FIELD_COUNT: usize = 4;",
        "pub const NOTE_EXPRESSION_TYPE_INFO_FIELD_COUNT: usize = 8;",
        "pub const PHYSICAL_UI_MAP_FIELD_COUNT: usize = 2;",
        "pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &[",
        "pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &[",
        "type_name: \"ProgramListInfo\", size: std::mem::size_of::<ProgramListInfo>()",
        "type_name: \"NoteExpressionTypeInfo\", size: std::mem::size_of::<NoteExpressionTypeInfo>()",
        "owner: \"ProgramListInfo\", field: \"programCount\", offset: std::mem::offset_of!(ProgramListInfo, programCount)",
        "owner: \"NoteExpressionTypeInfo\", field: \"valueDesc\", offset: std::mem::offset_of!(NoteExpressionTypeInfo, valueDesc)",
        "owner: \"PhysicalUIMapList\", field: \"map\", offset: std::mem::offset_of!(PhysicalUIMapList, map)",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kRootUnitId: UnitID = 0;",
        "pub const kNoParentUnitId: UnitID = -1;",
        "pub const kNoProgramListId: ProgramListID = -1;",
        "pub const kPlatformTypeHWND: PlatformType = \"HWND\";",
        "pub const kPlatformTypeNSView: PlatformType = \"NSView\";",
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing ABI layout item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in [
        "FUnknown",
        "IPlugView",
        "IMidiMapping",
        "ProcessData",
        "UnitInfo",
        "ProgramListInfo",
        "NoteExpressionTypeInfo",
        "PhysicalUIMap",
    ] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings ABI layout: {}",
            errors.join("; ")
        ))
    }
}

fn validate_vst3_sdk_generated_bindings_abi_seed_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR) {
        errors.push(format!(
            "missing ABI seed generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"abi-seed\";") {
        errors.push("missing ABI seed status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("ABI seed plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("ABI seed surface status must be ready-for-binding-emitter".to_string());
    }
    if !text.contains("pub const ABI_SEED_GENERATED: bool = true;") {
        errors.push("ABI seed must explicitly mark ABI_SEED_GENERATED true".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("ABI seed must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI seed must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors.push("ABI seed must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI seed must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push("ABI seed must be generated from a complete header manifest".to_string());
    }
    for required in [
        "pub type TResult = i32;",
        "pub type ParamID = u32;",
        "pub type ParamValue = f64;",
        "pub type TChar = u16;",
        "pub type TUID = [std::os::raw::c_char; 16];",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kPlatformTypeHWND: PlatformType = \"HWND\";",
        "pub const kPlatformTypeNSView: PlatformType = \"NSView\";",
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing ABI seed item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in ["IPlugView", "IMidiMapping"] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings ABI seed: {}",
            errors.join("; ")
        ))
    }
}

fn validate_vst3_sdk_generated_bindings_interface_skeleton_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR) {
        errors.push(format!(
            "missing interface skeleton generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"interface-skeleton\";") {
        errors.push("missing interface skeleton status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors
            .push("interface skeleton plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push(
            "interface skeleton surface status must be ready-for-binding-emitter".to_string(),
        );
    }
    if !text.contains("pub const INTERFACE_SKELETON_GENERATED: bool = true;") {
        errors.push(
            "interface skeleton must explicitly mark INTERFACE_SKELETON_GENERATED true".to_string(),
        );
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("interface skeleton must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("interface skeleton must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors
            .push("interface skeleton must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("interface skeleton must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push(
            "interface skeleton must be generated from a complete header manifest".to_string(),
        );
    }
    for required in [
        "#[repr(C)]",
        "pub struct FUnknownVTable",
        "pub struct FUnknown",
        "pub struct IPlugViewVTable",
        "pub struct IPlugView",
        "pub struct IEditControllerVTable",
        "pub struct IMidiMappingVTable",
        "pub struct INoteExpressionControllerVTable",
        "pub struct INoteExpressionPhysicalUIMappingVTable",
        "pub struct IUnitInfoVTable",
        "pub struct IProgramListDataVTable",
        "pub struct InterfaceMethod",
        "pub struct InterfaceVTableSlot",
        "pub struct InterfaceCallbackType",
        "pub struct InterfaceVTableFieldOffset",
        "pub struct InterfaceId",
        "pub struct QueryInterfaceEntry",
        "pub struct ComObjectInterface",
        "pub struct ComObjectIdentityPlan",
        "pub struct ComObjectQueryInterfaceDispatchEntry",
        "pub struct FactoryExportPlan",
        "pub struct FactoryClassPlan",
        "pub struct ModuleExportPlan",
        "pub struct BinaryExportSymbolPlan",
        "pub struct BinaryExportInspectionToolPlan",
        "pub slot: usize,",
        "pub signature: &'static str,",
        "pub local_slot: usize,",
        "pub global_slot: usize,",
        "pub callback_type: &'static str,",
        "pub object: &'static str,",
        "pub iid_const: &'static str,",
        "pub uid_words: [u32; 4],",
        "pub inherits_funknown: bool,",
        "pub required: bool,",
        "pub root_interface: &'static str,",
        "pub root_iid_const: &'static str,",
        "pub funknown_identity: &'static str,",
        "pub unknown_iid_result: &'static str,",
        "pub returns_same_identity: bool,",
        "pub add_ref_on_success: bool,",
        "pub factory_object: &'static str,",
        "pub factory_interface: &'static str,",
        "pub factory_iid_const: &'static str,",
        "pub class_count: usize,",
        "pub class_kind: &'static str,",
        "pub class_index: usize,",
        "pub category: &'static str,",
        "pub name_source: &'static str,",
        "pub cid_source: &'static str,",
        "pub cid_policy: &'static str,",
        "pub create_instance_root_iid_const: &'static str,",
        "pub unknown_cid_result: &'static str,",
        "pub construction_failure_result: &'static str,",
        "pub requested_iid_dispatch: &'static str,",
        "pub symbol: &'static str,",
        "pub platforms: &'static str,",
        "pub generated_callable: bool,",
        "pub binary_format: &'static str,",
        "pub tool_symbol: &'static str,",
        "pub inspection_tool: &'static str,",
        "pub verified_by_generated_bindings: bool,",
        "pub program: &'static str,",
        "pub args: &'static [&'static str],",
        "pub const INTERFACE_METHOD_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_SLOT_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_FIELD_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_COUNT: usize = ",
        "pub const INTERFACE_CALLBACK_TYPE_COUNT: usize = ",
        "pub const INTERFACE_ID_COUNT: usize = ",
        "pub const QUERY_INTERFACE_ENTRY_COUNT: usize = ",
        "pub const COM_OBJECT_COUNT: usize = ",
        "pub const COM_OBJECT_INTERFACE_COUNT: usize = ",
        "pub const COM_OBJECT_IDENTITY_PLAN_COUNT: usize = ",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRY_COUNT: usize = ",
        "pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;",
        "pub const FACTORY_CLASS_PLAN_COUNT: usize = ",
        "pub const MODULE_EXPORT_PLAN_COUNT: usize = ",
        "pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = ",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = ",
        "pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";",
        "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";",
        "pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";",
        "pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";",
        "pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";",
        "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";",
        "pub const INTERFACE_ID_SCOPE: &str = \"upstream-vst3-interface-iid-audit\";",
        "pub const QUERY_INTERFACE_ENTRY_SCOPE: &str = \"query-interface-dispatch-plan-audit\";",
        "pub const QUERY_INTERFACE_IID_LOOKUP_SCOPE: &str = \"pure-iid-dispatch-lookup-seed-audit\";",
        "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";",
        "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";",
        "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";",
        "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";",
        "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";",
        "pub const BINARY_EXPORT_SYMBOL_PLAN_SCOPE: &str = \"vesty-binary-export-symbol-plan-audit\";",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";",
        "pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;",
        "pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;",
        "pub const fn iid_from_words(a: u32, b: u32, c: u32, d: u32) -> TUID",
        "pub const FUNKNOWN_IID: TUID = iid_from_words(0x00000000, 0x00000000, 0xC0000000, 0x00000046);",
        "pub const IAUDIOPROCESSOR_IID: TUID = iid_from_words(0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D);",
        "pub const IEDITCONTROLLER_IID: TUID = iid_from_words(0xDCD7BBE3, 0x7742448D, 0xA874AACC, 0x979C759E);",
        "pub const IMIDIMAPPING_IID: TUID = iid_from_words(0xDF0FF9F7, 0x49B74669, 0xB63AB732, 0x7ADBF5E5);",
        "pub const IUNITINFO_IID: TUID = iid_from_words(0x3D4BD6B5, 0x913A4FD2, 0xA886E768, 0xA5EB92C1);",
        "pub const IPROGRAMLISTDATA_IID: TUID = iid_from_words(0x8683B01F, 0x7B354F70, 0xA2651DEC, 0x353AF4FF);",
        "pub const INOTEEXPRESSIONCONTROLLER_IID: TUID = iid_from_words(0xB7F8F859, 0x41234872, 0x91169581, 0x4F3721A3);",
        "pub fn interface_id_for_iid(iid: &TUID) -> Option<&'static InterfaceId>",
        "pub fn query_interface_entry_by_interface(interface: &str) -> Option<&'static QueryInterfaceEntry>",
        "pub fn query_interface_entry_for_iid(iid: &TUID) -> Option<&'static QueryInterfaceEntry>",
        "pub const INTERFACE_IDS: &[InterfaceId] = &[",
        "pub const QUERY_INTERFACE_ENTRIES: &[QueryInterfaceEntry] = &[",
        "pub const COM_OBJECTS: &[&str] = &[",
        "pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &[",
        "pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &[",
        "pub fn com_object_query_interface_dispatch_by_interface(",
        "pub fn com_object_query_interface_dispatch_for_iid(",
        "pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = ",
        "pub const VESTYPROCESSOR_FACTORY_CLASS_PLAN: FactoryClassPlan = ",
        "pub const VESTYCONTROLLER_FACTORY_CLASS_PLAN: FactoryClassPlan = ",
        "pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &[",
        "pub const GETPLUGINFACTORY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const WINDOWS_INITDLL_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const LINUX_MODULEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &[",
        "pub const WINDOWS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const MACOS_BUNDLEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const LINUX_MODULEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[",
        "pub fn binary_export_inspection_tools(",
        "pub fn binary_export_symbol_plan_by_platform_and_symbol(",
        "pub fn required_binary_export_symbol_count(platform: &str) -> usize",
        "pub fn first_missing_binary_export_symbol(",
        "pub fn binary_export_required_symbols_present(",
        "pub const VESTYPROCESSOR_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYCONTROLLER_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYPLUGVIEW_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYFACTORY_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYPROCESSOR_IDENTITY_PLAN: ComObjectIdentityPlan = ",
        "pub const VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH: &[ComObjectQueryInterfaceDispatchEntry] = &[",
        "pub const INTERFACE_METHODS: &[InterfaceMethod] = &[",
        "pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub fn interface_vtable_slot_by_interface_and_method(",
        "pub fn interface_vtable_slot_by_interface_and_global_slot(",
        "pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub fn interface_vtable_field_offset_by_interface_and_field(",
        "pub const IAUDIOPROCESSOR_METHODS: &[InterfaceMethod] = &[",
        "pub const IAUDIOPROCESSOR_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub const IAUDIOPROCESSOR_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const IAUDIOPROCESSOR_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub const IEDITCONTROLLER_METHODS: &[InterfaceMethod] = &[",
        "pub const IUNITINFO_METHODS: &[InterfaceMethod] = &[",
        "pub const IUNITINFO_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub const IUNITINFO_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const IUNITINFO_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub const IPROGRAMLISTDATA_METHODS: &[InterfaceMethod] = &[",
        "pub const INOTEEXPRESSIONCONTROLLER_METHODS: &[InterfaceMethod] = &[",
        "pub type IAudioProcessorProcess = unsafe extern \"system\" fn(",
        "pub process: IAudioProcessorProcess,",
        "offset: std::mem::offset_of!(IAudioProcessorVTable, process)",
        "local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\"",
        "pub type IUnitInfoGetProgramListInfo = unsafe extern \"system\" fn(",
        "pub getProgramListInfo: IUnitInfoGetProgramListInfo,",
        "offset: std::mem::offset_of!(IUnitInfoVTable, getProgramListInfo)",
        "local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\"",
        "pub type IProgramListDataGetProgramData = unsafe extern \"system\" fn(",
        "pub getProgramData: IProgramListDataGetProgramData,",
        "offset: std::mem::offset_of!(IProgramListDataVTable, getProgramData)",
        "local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\"",
        "pub type INoteExpressionControllerGetNoteExpressionInfo = unsafe extern \"system\" fn(",
        "pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,",
        "offset: std::mem::offset_of!(INoteExpressionControllerVTable, getNoteExpressionInfo)",
        "local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\"",
        "pub const INTERFACE_SKELETON_TYPES: &[&str] = &[",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kNoInterface: TResult",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing interface skeleton item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    if !text.contains(&format!(
        "pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = {};",
        vesty_vst3_sys::binary_export_symbol_plans().len()
    )) {
        errors.push("binary export symbol plan count does not match vesty-vst3-sys".to_string());
    }
    if !text.contains(&format!(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = {};",
        vesty_vst3_sys::binary_export_inspection_tool_plans().len()
    )) {
        errors.push(
            "binary export inspection tool plan count does not match vesty-vst3-sys".to_string(),
        );
    }
    if let Some(symbol_array) = rust_array_body(
        text,
        "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[",
    ) {
        let actual_count = count_text_occurrences(symbol_array, "BinaryExportSymbolPlan {");
        let expected_count = vesty_vst3_sys::binary_export_symbol_plans().len();
        if actual_count != expected_count {
            errors.push(format!(
                "binary export symbol plan array contains {actual_count} record(s), expected {expected_count}"
            ));
        }
        for plan in vesty_vst3_sys::binary_export_symbol_plans() {
            let expected = format!(
                "BinaryExportSymbolPlan {{ platform: {}, binary_format: {}, symbol: {}, tool_symbol: {}, inspection_tool: {}",
                rust_string_literal(plan.platform),
                rust_string_literal(plan.binary_format),
                rust_string_literal(plan.symbol),
                rust_string_literal(plan.tool_symbol),
                rust_string_literal(plan.inspection_tool),
            );
            match count_text_occurrences(symbol_array, &expected) {
                1 => {}
                0 => errors.push(format!(
                    "missing vesty-vst3-sys binary export symbol plan `{}/{}`",
                    plan.platform, plan.tool_symbol
                )),
                count => errors.push(format!(
                    "vesty-vst3-sys binary export symbol plan `{}/{}` appears {count} time(s), expected exactly once",
                    plan.platform, plan.tool_symbol
                )),
            }
        }
    } else {
        errors.push("missing binary export symbol plan array body".to_string());
    }
    if let Some(tool_array) = rust_array_body(
        text,
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[",
    ) {
        let actual_count = count_text_occurrences(tool_array, "BinaryExportInspectionToolPlan {");
        let expected_count = vesty_vst3_sys::binary_export_inspection_tool_plans().len();
        if actual_count != expected_count {
            errors.push(format!(
                "binary export inspection tool plan array contains {actual_count} record(s), expected {expected_count}"
            ));
        }
        for tool in vesty_vst3_sys::binary_export_inspection_tool_plans() {
            let args = tool
                .args
                .iter()
                .map(|arg| rust_string_literal(arg))
                .collect::<Vec<_>>()
                .join(", ");
            let expected = format!(
                "BinaryExportInspectionToolPlan {{ platform: {}, program: {}, args: &[{}] }}",
                rust_string_literal(tool.platform),
                rust_string_literal(tool.program),
                args
            );
            match count_text_occurrences(tool_array, &expected) {
                1 => {}
                0 => errors.push(format!(
                    "missing vesty-vst3-sys binary export inspection tool plan `{}/{}`",
                    tool.platform, tool.program
                )),
                count => errors.push(format!(
                    "vesty-vst3-sys binary export inspection tool plan `{}/{}` appears {count} time(s), expected exactly once",
                    tool.platform, tool.program
                )),
            }
        }
    } else {
        errors.push("missing binary export inspection tool plan array body".to_string());
    }
    for symbol in [
        "FUnknown",
        "IPlugView",
        "IEditController",
        "IMidiMapping",
        "INoteExpressionController",
        "IUnitInfo",
        "IProgramListData",
    ] {
        if !text.contains(symbol) {
            errors.push(format!("missing interface skeleton symbol `{symbol}`"));
        }
    }
    for required_metadata in [
        "slot: 0, interface: \"IAudioProcessor\", name: \"setBusArrangements\"",
        "slot: 6, interface: \"IAudioProcessor\", name: \"process\"",
        "local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\"",
        "local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\"",
        "local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\"",
        "local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\"",
        "name: \"process\", purpose: \"realtime audio/MIDI/process callback\", realtime: true, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\"",
        "name: \"getParameterInfo\", purpose: \"return host parameter metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IEditController, param_index: int32, info: *mut ParameterInfo) -> TResult\"",
        "name: \"getMidiControllerAssignment\", purpose: \"map MIDI controller to host parameter id\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IMidiMapping, bus_index: int32, channel: int16, midi_controller_number: CtrlNumber, id: *mut ParamID) -> TResult\"",
        "name: \"getNoteExpressionInfo\", purpose: \"return Note Expression type metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\"",
        "name: \"getPhysicalUIMapping\", purpose: \"return physical UI mapping for Note Expression\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionPhysicalUIMapping, bus_index: int32, channel: int16, list: *mut PhysicalUIMapList) -> TResult\"",
        "name: \"getProgramListInfo\", purpose: \"return program-list metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\"",
        "name: \"getProgramData\", purpose: \"save program data to stream\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\"",
        "InterfaceId { interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", uid_words: [0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D], source: \"upstream-vst3-0.3.0/src/bindings.rs\" }",
        "QueryInterfaceEntry { interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", inherits_funknown: true, implementation: \"planned-dispatch-entry-no-callable-glue\" }",
        "InterfaceId { interface: \"IUnitInfo\", iid_const: \"IUNITINFO_IID\", uid_words: [0x3D4BD6B5, 0x913A4FD2, 0xA886E768, 0xA5EB92C1], source: \"upstream-vst3-0.3.0/src/bindings.rs\" }",
        "QueryInterfaceEntry { interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", inherits_funknown: true, implementation: \"planned-dispatch-entry-no-callable-glue\" }",
        "ComObjectInterface { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyProcessor\", interface: \"IProcessContextRequirements\", iid_const: \"IPROCESSCONTEXTREQUIREMENTS_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IEditController\", iid_const: \"IEDITCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IUnitInfo\", iid_const: \"IUNITINFO_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"INoteExpressionController\", iid_const: \"INOTEEXPRESSIONCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyPlugView\", interface: \"IPlugView\", iid_const: \"IPLUGVIEW_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectIdentityPlan { object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }",
        "ComObjectIdentityPlan { object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"FUnknown\", iid_const: \"FUNKNOWN_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", root_interface: \"IEditController\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", root_interface: \"IPluginFactory\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "FactoryExportPlan { factory_object: \"VestyFactory\", factory_interface: \"IPluginFactory\", factory_iid_const: \"IPLUGINFACTORY_IID\", class_count: 2, count_classes_result: \"2\", get_factory_info_source: \"PluginInfo vendor/url/email + kUnicode\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "FactoryClassPlan { class_kind: \"processor\", class_index: 0, class_object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", category: \"Audio Module Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id\", cid_policy: \"processor-cid-is-plugin-class-id\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyProcessor\", create_instance_root_interface: \"IComponent\", create_instance_root_iid_const: \"ICOMPONENT_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "FactoryClassPlan { class_kind: \"controller\", class_index: 1, class_object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", category: \"Component Controller Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id[15].wrapping_add(1)\", cid_policy: \"controller-cid-last-byte-wrapping-add-1\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyController\", create_instance_root_interface: \"IEditController\", create_instance_root_iid_const: \"IEDITCONTROLLER_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "ModuleExportPlan { symbol: \"GetPluginFactory\", platforms: \"windows,macos,linux\", signature: \"extern \\\"system\\\" fn() -> *mut IPluginFactory\", purpose: \"return VST3 plugin factory pointer\", implementation: \"vesty_vst3::create_plugin_factory::<Plugin>()\", return_policy: \"returns owned COM factory pointer for host discovery\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"InitDll\", platforms: \"windows\", signature: \"extern \\\"system\\\" fn() -> bool\", purpose: \"Windows VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"bundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS VST3 bundle initialization entry\", implementation: \"vesty_vst3::set_macos_bundle_ref(bundle_ref); return true\", return_policy: \"bundle resources path is captured when possible\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"BundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS compatibility initialization alias\", implementation: \"delegate to bundleEntry(bundle_ref)\", return_policy: \"keeps uppercase host lookup compatibility\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"ModuleEntry\", platforms: \"linux\", signature: \"extern \\\"system\\\" fn(library_handle: *mut c_void) -> bool\", purpose: \"Linux VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"GetPluginFactory\", tool_symbol: \"GetPluginFactory\", inspection_tool: \"dumpbin /exports or llvm-objdump -p\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"macos\", binary_format: \"Mach-O\", symbol: \"bundleEntry\", tool_symbol: \"_bundleEntry\", inspection_tool: \"nm -gU or llvm-nm -gU\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"linux-x64\", binary_format: \"ELF\", symbol: \"ModuleEntry\", tool_symbol: \"ModuleEntry\", inspection_tool: \"nm -D --defined-only or llvm-nm -D --defined-only\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
    ] {
        if !text.contains(required_metadata) {
            errors.push(format!(
                "missing interface skeleton metadata `{required_metadata}`"
            ));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings interface skeleton: {}",
            errors.join("; ")
        ))
    }
}

fn rust_array_body<'a>(text: &'a str, declaration: &str) -> Option<&'a str> {
    let after_declaration = text.get(text.find(declaration)? + declaration.len()..)?;
    let end = after_declaration.find("\n];")?;
    Some(&after_declaration[..end])
}

fn count_text_occurrences(text: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    text.match_indices(needle).count()
}

fn validate_vst3_sdk_generated_bindings_scaffold_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR) {
        errors.push(format!(
            "missing scaffold generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"metadata-scaffold\";") {
        errors.push("missing metadata scaffold status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("scaffold plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("scaffold surface status must be ready-for-binding-emitter".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("scaffold must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("scaffold must keep `BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push(
            "metadata scaffold must be generated from a complete header manifest".to_string(),
        );
    }
    if !text.contains("pub const SURFACE_SYMBOL_COUNT: usize = ") {
        errors.push("missing surface symbol count".to_string());
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in ["IPlugView", "IMidiMapping"] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings scaffold: {}",
            errors.join("; ")
        ))
    }
}

fn import_ci_release_check_artifact(
    path: &Utf8Path,
    report: ReleaseCheckReport,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = artifact_os_from_path(path) else {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            "could not infer OS from artifact path",
        ));
        return Ok(());
    };
    if let Err(error) = validate_ci_release_check_report(&report) {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }
    if let Err(error) =
        validate_import_ci_run_match(report.ci_run_url.as_deref(), expected_ci_run_url)
    {
        items.push(import_ci_item(
            "ci release-check artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join(format!("ci-release-checks/release-check-{os}.json"));
    import_copy_file_item(
        "ci release-check artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

fn import_ci_release_action_plan_artifact(
    path: &Utf8Path,
    plan: &ReleaseActionPlan,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = artifact_os_from_path(path) else {
        items.push(import_ci_item(
            "release action plan sidecar",
            "skipped",
            Some(path),
            None,
            "could not infer OS from action plan path; not a per-OS CI sidecar",
        ));
        return Ok(());
    };
    if let Err(error) = validate_release_action_plan_sidecar(plan) {
        items.push(import_ci_item(
            "release action plan sidecar",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join(format!("ci-release-checks/release-action-plan-{os}.json"));
    import_copy_file_item(
        "release action plan sidecar",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

fn validate_release_action_plan_sidecar(plan: &ReleaseActionPlan) -> Result<(), String> {
    if plan.version != 1 {
        return Err(format!(
            "unsupported release action plan version {}",
            plan.version
        ));
    }
    if !matches!(plan.status.as_str(), "ok" | "failed") {
        return Err(format!(
            "unexpected release action plan status `{}`",
            plan.status
        ));
    }
    validate_release_action_text(
        "release action plan protocol snapshot",
        &plan.protocol_snapshot,
    )?;
    validate_release_action_safe_path(
        "release action plan protocol snapshot",
        &plan.protocol_snapshot,
    )?;
    if let Some(evidence_root) = &plan.evidence_root {
        validate_release_action_text("release action plan evidence root", evidence_root)?;
        validate_release_action_safe_path("release action plan evidence root", evidence_root)?;
    }
    if let Some(release_evidence_dir) = &plan.release_evidence_dir {
        validate_release_action_text(
            "release action plan release evidence dir",
            release_evidence_dir,
        )?;
        validate_release_action_safe_path(
            "release action plan release evidence dir",
            release_evidence_dir,
        )?;
    }
    if plan.summary.action_count != plan.actions.len() {
        return Err(format!(
            "action count mismatch: summary={} actual={}",
            plan.summary.action_count,
            plan.actions.len()
        ));
    }

    let failed = plan
        .actions
        .iter()
        .filter(|action| action.status == "failed")
        .count();
    let skipped = plan
        .actions
        .iter()
        .filter(|action| action.status == "skipped")
        .count();
    if plan.summary.failed != failed {
        return Err(format!(
            "failed count mismatch: summary={} actions={failed}",
            plan.summary.failed
        ));
    }
    if plan.summary.skipped != skipped {
        return Err(format!(
            "skipped count mismatch: summary={} actions={skipped}",
            plan.summary.skipped
        ));
    }
    let pending = failed + skipped;
    if plan.summary.action_count != pending {
        return Err(format!(
            "action pending count mismatch: action_count={} failed+skipped={pending}",
            plan.summary.action_count
        ));
    }
    let summary_checks = plan.summary.ok + plan.summary.failed + plan.summary.skipped;
    if summary_checks == 0 {
        return Err("release action plan summary must contain at least one check".to_string());
    }
    if summary_checks > RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS {
        return Err(format!(
            "release action plan summary check count {summary_checks} exceeds maximum {RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS}"
        ));
    }
    validate_release_action_plan_summary_check_count(summary_checks)?;
    if plan.status == "ok" && plan.summary.failed != 0 {
        return Err("ok release action plan must not contain failed actions".to_string());
    }
    if plan.status == "failed" && plan.actions.is_empty() {
        return Err("failed release action plan must contain at least one action".to_string());
    }
    if plan.status == "failed" && plan.summary.failed == 0 {
        return Err(
            "failed release action plan must contain at least one failed action".to_string(),
        );
    }

    let expected_checks = expected_release_check_names();
    let mut seen_actions = BTreeSet::new();
    for action in &plan.actions {
        validate_release_action_item(action)?;
        validate_release_action_item_evidence_path(plan, action)?;
        if !seen_actions.insert(action.check.as_str()) {
            return Err(format!("duplicate release action check `{}`", action.check));
        }
        if !expected_checks.contains(action.check.as_str()) {
            return Err(format!(
                "unknown release action check `{}`; action plan sidecar must use the current Vesty release gate",
                action.check
            ));
        }
    }
    Ok(())
}

const RELEASE_ACTION_PLAN_MAX_SUMMARY_CHECKS: usize = 128;
const RELEASE_ACTION_MAX_COMMANDS: usize = 16;
const RELEASE_ACTION_TEXT_MAX_BYTES: usize = 8 * 1024;
const RELEASE_CHECK_MAX_CHECKS: usize = 128;

fn validate_release_action_plan_summary_check_count(summary_checks: usize) -> Result<(), String> {
    let expected = expected_release_check_names().len();
    if summary_checks == expected {
        return Ok(());
    }

    Err(format!(
        "release action plan summary check count must match current Vesty release gate: summary={summary_checks} expected={expected}"
    ))
}

fn validate_release_action_item(action: &ReleaseActionItem) -> Result<(), String> {
    validate_release_action_text("check name", &action.check)?;
    validate_release_action_text(
        &format!("release action `{}` value", action.check),
        &action.value,
    )?;
    if let Some(hint) = &action.hint {
        validate_release_action_text(&format!("release action `{}` hint", action.check), hint)?;
    }
    if let Some(evidence_path) = &action.evidence_path {
        validate_release_action_text(
            &format!("release action `{}` evidence path", action.check),
            evidence_path,
        )?;
        validate_release_action_safe_path(
            &format!("release action `{}` evidence path", action.check),
            evidence_path,
        )?;
    }
    if action.commands.is_empty() {
        return Err(format!(
            "release action `{}` has no suggested commands",
            action.check
        ));
    }
    if action.commands.len() > RELEASE_ACTION_MAX_COMMANDS {
        return Err(format!(
            "release action `{}` has too many suggested commands: {} exceeds maximum {RELEASE_ACTION_MAX_COMMANDS}",
            action.check,
            action.commands.len()
        ));
    }
    for (index, command) in action.commands.iter().enumerate() {
        let label = format!("release action `{}` command {index}", action.check);
        validate_release_action_text(&label, command)?;
        validate_release_action_command_syntax(&label, command)?;
    }
    match (action.status.as_str(), action.priority.as_str()) {
        ("failed", "required") | ("skipped", "optional") => Ok(()),
        ("failed", other) | ("skipped", other) => Err(format!(
            "release action `{}` status {} has invalid priority `{other}`",
            action.check, action.status
        )),
        (other, _) => Err(format!(
            "release action `{}` has unexpected status `{other}`",
            action.check
        )),
    }
}

fn validate_release_action_item_evidence_path(
    plan: &ReleaseActionPlan,
    action: &ReleaseActionItem,
) -> Result<(), String> {
    let Some(expected) = expected_release_action_evidence_path(plan, &action.check) else {
        return Ok(());
    };
    match action.evidence_path.as_deref() {
        Some(actual) if release_report_paths_equal(actual, &expected) => Ok(()),
        Some(actual) => Err(format!(
            "release action `{}` evidence path `{actual}` does not match expected `{expected}`",
            action.check
        )),
        None => Err(format!(
            "release action `{}` is missing expected evidence path `{expected}`",
            action.check
        )),
    }
}

fn validate_release_action_safe_path(label: &str, path: &str) -> Result<(), String> {
    validate_release_report_root_path(label, path).map_err(|error| error.to_string())
}

fn expected_release_action_evidence_path(plan: &ReleaseActionPlan, check: &str) -> Option<String> {
    if let Some(host) = check.strip_prefix("daw smoke: ") {
        let evidence_root = plan
            .evidence_root
            .as_deref()
            .unwrap_or("target/daw-evidence");
        let host_dir = release_action_host_dir_name(host)?;
        return Some(format!("{evidence_root}/{host_dir}"));
    }
    if check == "daw matrix" {
        return Some(
            plan.evidence_root
                .clone()
                .unwrap_or_else(|| "target/daw-evidence".to_string()),
        );
    }
    if check == "protocol snapshot" {
        return Some(plan.protocol_snapshot.clone());
    }

    let release_evidence_dir = plan
        .release_evidence_dir
        .as_deref()
        .unwrap_or("target/release-evidence");
    let relative = match check {
        "ci run url" => "ci-run-url.txt",
        "ci doctor artifacts" => "ci-doctor",
        "ci release-check artifacts" => "ci-release-checks",
        "platform smoke artifacts" => "platform-smoke",
        "vst3 validate reports" | "vst3 example validator coverage" => "validator",
        "vst3 static validate reports" | "ci example static validate coverage" => "package",
        "crate publish plan" => "publish-plan/publish-plan.json",
        "crate package readiness" => "crate-package/crate-package.json",
        "npm package pack report" => "npm-pack/npm-pack.json",
        "dependency latest baseline" => "dependency-baseline/dependency-baseline-latest.json",
        "vst3 SDK header manifest" => "vst3-sdk/vst3-sdk-headers.json",
        "vst3 SDK generated bindings plan" => "vst3-sdk/generated-bindings-plan.json",
        "vst3 SDK generated bindings surface" => "vst3-sdk/generated-bindings-surface.json",
        "vst3 SDK generated bindings scaffold" => "vst3-sdk/generated.rs",
        "vst3 SDK generated bindings ABI seed" => "vst3-sdk/generated-abi-seed.rs",
        "vst3 SDK generated bindings ABI layout" => "vst3-sdk/generated-abi.rs",
        "vst3 SDK generated bindings interface skeleton" => {
            "vst3-sdk/generated-interface-skeleton.rs"
        }
        "signed bundle evidence" => {
            return Some(format!(
                "{release_evidence_dir}/signing-macos.log and {release_evidence_dir}/signing-windows.log"
            ));
        }
        "notarization log" => "notary.log",
        _ => return None,
    };
    Some(format!("{release_evidence_dir}/{relative}"))
}

fn release_action_host_dir_name(host: &str) -> Option<&'static str> {
    match host {
        "REAPER" => Some("reaper"),
        "Cubase/Nuendo" => Some("cubase"),
        "Bitwig Studio" => Some("bitwig"),
        "Ableton Live" => Some("ableton"),
        "Studio One" => Some("studio-one"),
        _ => None,
    }
}

fn validate_release_action_command_syntax(label: &str, command: &str) -> Result<(), String> {
    let command = command.trim();
    if !release_action_command_starts_with_vesty(command) {
        return Ok(());
    }
    let argv = split_release_action_command(command)?;
    Cli::try_parse_from(&argv)
        .map(|_| ())
        .map_err(|error| format!("{label} does not parse with current CLI: {error}"))
}

fn release_action_command_starts_with_vesty(command: &str) -> bool {
    let Some(remainder) = command.strip_prefix("vesty") else {
        return false;
    };
    remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace)
}

fn split_release_action_command(command: &str) -> Result<Vec<String>, String> {
    let mut args = vec!["vesty".to_string()];
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut chars = command.trim().chars().peekable();

    for _ in 0.."vesty".len() {
        chars.next();
    }

    for ch in chars {
        match ch {
            '"' => in_double_quotes = !in_double_quotes,
            '#' if !in_double_quotes => break,
            ch if ch.is_whitespace() && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_double_quotes {
        return Err("release action command contains an unterminated double quote".to_string());
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

fn validate_release_action_text(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > RELEASE_ACTION_TEXT_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {RELEASE_ACTION_TEXT_MAX_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} must not contain control characters"));
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!(
            "{label} must not contain unsafe Unicode format characters"
        ));
    }
    Ok(())
}

fn sanitize_release_report_text(value: impl Into<String>) -> String {
    let value = value.into();
    let mut out = String::new();
    let mut previous_space = false;
    for raw in value.chars() {
        let ch = if raw.is_control() || is_release_action_unsafe_format_char(raw) {
            ' '
        } else {
            raw
        };
        if ch.is_whitespace() {
            if !out.is_empty() && !previous_space {
                if out.len() + 1 > RELEASE_ACTION_TEXT_MAX_BYTES {
                    break;
                }
                out.push(' ');
                previous_space = true;
            }
            continue;
        }
        if out.len() + ch.len_utf8() > RELEASE_ACTION_TEXT_MAX_BYTES {
            break;
        }
        out.push(ch);
        previous_space = false;
    }

    let trimmed = out.trim();
    if trimmed.is_empty() {
        "<empty>".to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_release_action_unsafe_format_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF
    )
}

fn import_ci_doctor_artifact(
    path: &Utf8Path,
    report: DoctorReport,
    options: &ImportCiOptions,
    expected_ci_run_url: Option<&str>,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = validate_doctor_report(&report) {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }
    let os = match import_doctor_artifact_os(path, &report) {
        Ok(os) => os,
        Err(error) => {
            items.push(import_ci_item(
                "ci doctor artifact",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
    };
    let missing = missing_doctor_checks(os, &report);
    if !missing.is_empty() {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            format!("missing checks: {}", missing.join("; ")),
        ));
        return Ok(());
    }
    if let Err(error) =
        validate_import_ci_run_match(report.ci_run_url.as_deref(), expected_ci_run_url)
    {
        items.push(import_ci_item(
            "ci doctor artifact",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options.dir.join(format!("ci-doctor/doctor-{os}.json"));
    import_copy_file_item(
        "ci doctor artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

fn import_ci_validate_artifact(
    path: &Utf8Path,
    report: ValidateReport,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (name, destination) = if validate_release_validate_report(&report).is_ok() {
        if let Err(error) = validate_report_artifact_path_platform(path, &report) {
            items.push(import_ci_item(
                "vst3 validate report",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
        (
            "vst3 validate report",
            options
                .dir
                .join("validator")
                .join(validate_report_import_filename(&report, "validate")),
        )
    } else if validate_static_validate_report(&report).is_ok() {
        if let Err(error) = validate_report_artifact_path_platform(path, &report) {
            items.push(import_ci_item(
                "vst3 static validate report",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
        (
            "vst3 static validate report",
            options
                .dir
                .join("package")
                .join(validate_report_import_filename(&report, "static-validate")),
        )
    } else {
        let release_error = validate_release_validate_report(&report)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "not a validator-passed report".to_string());
        let static_error = validate_static_validate_report(&report)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "not a static validate report".to_string());
        items.push(import_ci_item(
            "vst3 validate report",
            "failed",
            Some(path),
            None,
            format!("{release_error}; {static_error}"),
        ));
        return Ok(());
    };

    import_copy_file_item(name, path, &destination, options.overwrite, items)
}

fn import_ci_publish_plan_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_publish_plan_report(path) {
        Ok(_) => import_copy_file_item(
            "crate publish plan",
            path,
            &options.dir.join("publish-plan/publish-plan.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "crate publish plan",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_crate_package_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_crate_package_report_path(path) {
        Ok(_) => import_copy_file_item(
            "crate package readiness",
            path,
            &options.dir.join("crate-package/crate-package.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "crate package readiness",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_npm_pack_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_npm_pack_report(path) {
        Ok(_) => import_copy_file_item(
            "npm package pack report",
            path,
            &options.dir.join("npm-pack/npm-pack.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "npm package pack report",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_dependency_baseline_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_dependency_baseline_latest_report(path) {
        Ok(_) => import_copy_file_item(
            "dependency latest baseline",
            path,
            &options
                .dir
                .join("dependency-baseline/dependency-baseline-latest.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "dependency latest baseline",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_vst3_sdk_manifest_artifact(
    path: &Utf8Path,
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_header_manifest_content(manifest) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK header manifest",
            path,
            &options.dir.join("vst3-sdk/vst3-sdk-headers.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK header manifest",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_vst3_sdk_binding_plan_artifact(
    path: &Utf8Path,
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_binding_plan_content(plan) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK generated bindings plan",
            path,
            &options.dir.join("vst3-sdk/generated-bindings-plan.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings plan",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_vst3_sdk_binding_surface_artifact(
    path: &Utf8Path,
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match validate_vst3_sdk_binding_surface_content(surface) {
        Ok(_) => import_copy_file_item(
            "vst3 SDK generated bindings surface",
            path,
            &options.dir.join("vst3-sdk/generated-bindings-surface.json"),
            options.overwrite,
            items,
        ),
        Err(error) => {
            items.push(import_ci_item(
                "vst3 SDK generated bindings surface",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            Ok(())
        }
    }
}

fn import_ci_platform_smoke_artifact(
    path: &Utf8Path,
    report: PlatformSmokeReport,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if platform_smoke_report_is_pending_template(&report) {
        items.push(import_ci_item(
            "platform smoke artifact",
            "skipped",
            Some(path),
            None,
            "pending platform smoke template",
        ));
        return Ok(());
    }
    let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
        items.push(import_ci_item(
            "platform smoke artifact",
            "failed",
            Some(path),
            None,
            format!("unsupported platform `{}`", report.platform),
        ));
        return Ok(());
    };
    match platform_smoke_platform_from_artifact_path(path) {
        Ok(Some(path_platform)) if path_platform != platform => {
            items.push(import_ci_item(
                "platform smoke artifact",
                "failed",
                Some(path),
                None,
                format!(
                    "artifact path indicates {path_platform}, but report platform is {platform}"
                ),
            ));
            return Ok(());
        }
        Ok(_) => {}
        Err(error) => {
            items.push(import_ci_item(
                "platform smoke artifact",
                "failed",
                Some(path),
                None,
                error,
            ));
            return Ok(());
        }
    }
    if let Err(error) = validate_platform_smoke_report(&report) {
        items.push(import_ci_item(
            "platform smoke artifact",
            "failed",
            Some(path),
            None,
            error.to_string(),
        ));
        return Ok(());
    }

    let destination = options.dir.join(format!("platform-smoke/{platform}.json"));
    import_copy_file_item(
        "platform smoke artifact",
        path,
        &destination,
        options.overwrite,
        items,
    )
}

fn import_ci_text_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if read_ci_run_url_file(path)?.is_some_and(|url| is_github_actions_run_url(&url)) {
        return Ok(());
    }

    match signing_evidence_platforms(path) {
        Ok(platforms) => {
            if let Err(error) = validate_signing_artifact_path_platform(path, &platforms) {
                items.push(import_ci_item(
                    "signed bundle evidence",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
            let destination = signing_import_destination(&options.dir, path, &platforms);
            return import_copy_file_item(
                "signed bundle evidence",
                path,
                &destination,
                options.overwrite,
                items,
            );
        }
        Err(error) if signing_evidence_import_failure_is_actionable(path, &error.to_string()) => {
            items.push(import_ci_item(
                "signed bundle evidence",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
        Err(_) => {}
    }

    match notarization_evidence(path) {
        Ok(evidence) => {
            if let Err(error) = validate_notarization_artifact_path_platform(path) {
                items.push(import_ci_item(
                    "notarization log",
                    "failed",
                    Some(path),
                    None,
                    error,
                ));
                return Ok(());
            }
            if evidence.accepted && evidence.stapled {
                return import_copy_file_item(
                    "notarization log",
                    path,
                    &options.dir.join("notary.log"),
                    options.overwrite,
                    items,
                );
            }
            items.push(import_ci_item(
                "notarization log",
                "failed",
                Some(path),
                None,
                "notarization evidence must include both accepted notarytool output and stapler success",
            ));
            return Ok(());
        }
        Err(error)
            if notarization_evidence_import_failure_is_actionable(path, &error.to_string()) =>
        {
            items.push(import_ci_item(
                "notarization log",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
        Err(_) => {}
    }

    items.push(import_ci_item(
        "text artifact",
        "skipped",
        Some(path),
        None,
        "unrecognized text artifact",
    ));
    Ok(())
}

fn signing_evidence_import_failure_is_actionable(path: &Utf8Path, error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("negative signing evidence")
        || release_artifact_file_name_contains_any(
            path,
            &["codesign", "signtool", "signed", "signing", "signature"],
        )
}

fn notarization_evidence_import_failure_is_actionable(path: &Utf8Path, error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("negative notarization evidence")
        || release_artifact_file_name_contains_any(
            path,
            &["notary", "notarytool", "notarization", "stapler", "staple"],
        )
}

fn validate_notarization_artifact_path_platform(path: &Utf8Path) -> Result<(), String> {
    let Some(path_platform) = notarization_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    if path_platform == "macOS" {
        Ok(())
    } else {
        Err(format!(
            "artifact path indicates {path_platform}, but notarization evidence is macOS-only"
        ))
    }
}

fn notarization_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    match notarization_file_name_platform(path)? {
        Some(platform) => Ok(Some(platform)),
        None => notarization_parent_dir_platform(path),
    }
}

fn notarization_file_name_platform(path: &Utf8Path) -> Result<Option<&'static str>, String> {
    let tokens = file_name_tokens(path);
    platform_label_from_tokens(&tokens, "notarization evidence file name")
}

fn notarization_parent_dir_platform(path: &Utf8Path) -> Result<Option<&'static str>, String> {
    let Some(parent) = path.parent().and_then(Utf8Path::file_name) else {
        return Ok(None);
    };
    let tokens = path_component_tokens(parent);
    platform_label_from_tokens(&tokens, "notarization evidence parent directory")
}

fn platform_label_from_tokens(
    tokens: &[String],
    source: &str,
) -> Result<Option<&'static str>, String> {
    let mut found = Vec::new();
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        found.push("macOS");
    }
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "windows" | "windowsx64" | "win32" | "win64"))
    {
        found.push("Windows");
    }
    if tokens.iter().any(|token| token == "linux") {
        found.push("Linux");
    }
    match found.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "{source} contains multiple platform labels: {}",
            found.join(", ")
        )),
    }
}

fn release_artifact_file_name_contains_any(path: &Utf8Path, needles: &[&str]) -> bool {
    let name = path.file_name().unwrap_or("").to_ascii_lowercase();
    needles.iter().any(|needle| name.contains(needle))
}

fn validate_signing_artifact_path_platform(
    path: &Utf8Path,
    platforms: &BTreeSet<SigningEvidencePlatform>,
) -> Result<(), String> {
    let Some(path_platform) = signing_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    if platforms.contains(&path_platform) {
        Ok(())
    } else {
        Err(format!(
            "artifact path indicates {}, but signing evidence platform is {}",
            path_platform.label(),
            format_signing_platform_set(platforms)
        ))
    }
}

fn signing_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<SigningEvidencePlatform>, String> {
    match signing_file_name_platform(path)? {
        Some(platform) => Ok(Some(platform)),
        None => Ok(signing_parent_dir_platform(path)),
    }
}

fn signing_file_name_platform(path: &Utf8Path) -> Result<Option<SigningEvidencePlatform>, String> {
    let tokens = file_name_tokens(path);
    let mut found = [
        (
            SigningEvidencePlatform::Macos,
            &["macos", "mac", "darwin", "osx"][..],
        ),
        (
            SigningEvidencePlatform::Windows,
            &["windows", "windowsx64", "win64"][..],
        ),
    ]
    .into_iter()
    .filter_map(|(platform, aliases)| {
        tokens
            .iter()
            .any(|token| aliases.contains(&token.as_str()))
            .then_some(platform)
    })
    .collect::<Vec<_>>();
    found.sort_unstable();
    found.dedup();
    match found.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "signing evidence file name contains multiple platform labels: {}",
            format_signing_platform_list(&found)
        )),
    }
}

fn signing_parent_dir_platform(path: &Utf8Path) -> Option<SigningEvidencePlatform> {
    let parent = path.parent()?.file_name()?;
    let tokens = path_component_tokens(parent);
    match tokens
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["macos"] | ["mac"] | ["darwin"] | ["osx"] => Some(SigningEvidencePlatform::Macos),
        ["windows"] | ["windows", "x64"] | ["windowsx64"] | ["win64"] => {
            Some(SigningEvidencePlatform::Windows)
        }
        _ => None,
    }
}

fn format_signing_platform_set(platforms: &BTreeSet<SigningEvidencePlatform>) -> String {
    if platforms.is_empty() {
        "unknown".to_string()
    } else {
        format_signing_platform_list(&platforms.iter().copied().collect::<Vec<_>>())
    }
}

fn format_signing_platform_list(platforms: &[SigningEvidencePlatform]) -> String {
    platforms
        .iter()
        .map(|platform| platform.label())
        .collect::<Vec<_>>()
        .join(", ")
}

fn import_ci_signed_bundle_artifact(
    path: &Utf8Path,
    options: &ImportCiOptions,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let platforms = match signing_evidence_platforms(path) {
        Ok(platforms) => platforms,
        Err(error) => {
            items.push(import_ci_item(
                "signed bundle evidence",
                "failed",
                Some(path),
                None,
                error.to_string(),
            ));
            return Ok(());
        }
    };
    if let Err(error) = validate_signing_artifact_path_platform(path, &platforms) {
        items.push(import_ci_item(
            "signed bundle evidence",
            "failed",
            Some(path),
            None,
            error,
        ));
        return Ok(());
    }

    let destination = options
        .dir
        .join("signed-bundles")
        .join(path.file_name().unwrap_or("signed.vst3"));
    match import_copy_dir_contents(path, &destination, options.overwrite)? {
        ImportWriteOutcome::Imported => items.push(import_ci_item(
            "signed bundle evidence",
            "imported",
            Some(path),
            Some(destination.as_path()),
            "copied signed macOS .vst3 bundle",
        )),
        ImportWriteOutcome::SkippedExisting => items.push(import_ci_item(
            "signed bundle evidence",
            "skipped",
            Some(path),
            Some(destination.as_path()),
            ImportWriteOutcome::SkippedExisting.value(),
        )),
    }
    Ok(())
}

fn import_copy_file_item(
    name: &str,
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
    items: &mut Vec<ImportCiReleaseEvidenceItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = import_copy_file(source, destination, overwrite)?;
    items.push(import_ci_item(
        name,
        outcome.status(),
        Some(source),
        Some(destination),
        outcome.value(),
    ));
    Ok(())
}

fn import_copy_file(
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("CI artifact import refuses symlink source: {source}").into());
    }
    if !metadata.is_file() {
        return Err(format!("CI artifact import source is not a file: {source}").into());
    }
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    if let Some(parent) = destination.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(ImportWriteOutcome::Imported)
}

fn import_write_text_file(
    destination: &Utf8Path,
    text: &str,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    import_write_bytes_file(destination, text.as_bytes(), overwrite)
}

fn import_write_ci_run_url_file(
    destination: &Utf8Path,
    url: &str,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite && read_ci_run_url_file(destination)?.is_some() {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    import_write_text_file(destination, &format!("ci_run_url={url}\n"), true)
}

fn import_write_bytes_file(
    destination: &Utf8Path,
    bytes: &[u8],
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    if let Some(parent) = destination.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, bytes)?;
    Ok(ImportWriteOutcome::Imported)
}

fn import_copy_dir_contents(
    source: &Utf8Path,
    destination: &Utf8Path,
    overwrite: bool,
) -> Result<ImportWriteOutcome, Box<dyn std::error::Error>> {
    let destination_exists = path_exists_no_follow(destination)?;
    if destination_exists && !overwrite {
        return Ok(ImportWriteOutcome::SkippedExisting);
    }
    if destination_exists {
        remove_existing_path(destination)?;
    }
    reject_existing_output_parent_symlink("CI artifact import destination", destination)?;
    fs::create_dir_all(destination)?;
    for (relative, bytes) in collect_relative_files(source)? {
        let target = destination.join(&relative);
        import_write_bytes_file(&target, &bytes, true)?;
    }
    Ok(ImportWriteOutcome::Imported)
}

fn remove_existing_path(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn path_exists_no_follow(path: &Utf8Path) -> Result<bool, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn import_ci_item(
    name: &str,
    status: &str,
    source: Option<&Utf8Path>,
    path: Option<&Utf8Path>,
    value: impl Into<String>,
) -> ImportCiReleaseEvidenceItem {
    ImportCiReleaseEvidenceItem {
        name: name.to_string(),
        status: status.to_string(),
        source: source.map(portable_report_path),
        path: path.map(portable_report_path),
        value: sanitize_release_report_text(value),
    }
}

fn validate_local_release_evidence_report_shape(
    report: &LocalReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("local release evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("local release evidence dir", &report.evidence_dir)?;
    validate_release_action_text("local release workspace", &report.workspace)?;
    if let Some(protocol_snapshot) = &report.protocol_snapshot {
        validate_release_action_text("local release protocol snapshot", protocol_snapshot)?;
        validate_release_report_root_path("local release protocol snapshot", protocol_snapshot)?;
    }
    validate_release_action_text(
        "local release external evidence note",
        &report.external_evidence_note,
    )?;
    validate_release_evidence_item_count(
        "local release evidence report",
        report.items.len(),
        true,
    )?;
    for item in &report.items {
        validate_local_release_evidence_item_shape(
            "local release evidence item",
            item,
            &["ok", "skipped", "failed"],
        )?;
        validate_local_release_evidence_item_paths(report, item)?;
    }
    validate_local_release_evidence_protocol_consistency(report)?;
    Ok(())
}

fn validate_import_ci_release_evidence_report_shape(
    report: &ImportCiReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("import-ci evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("import-ci evidence dir", &report.evidence_dir)?;
    validate_release_action_text("import-ci source", &report.source)?;
    validate_release_report_root_path("import-ci source", &report.source)?;
    validate_release_action_text(
        "import-ci external evidence note",
        &report.external_evidence_note,
    )?;
    validate_release_evidence_item_count("import-ci report", report.items.len(), false)?;
    for item in &report.items {
        validate_import_ci_release_evidence_item_shape(item)?;
        validate_import_ci_release_evidence_item_paths(report, item)?;
    }
    validate_import_ci_release_evidence_success_output_uniqueness(report)?;
    Ok(())
}

fn validate_collected_release_evidence_report_shape(
    report: &CollectedReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("collected release evidence dir", &report.evidence_dir)?;
    validate_release_report_root_path("collected release evidence dir", &report.evidence_dir)?;
    validate_release_action_text("collected release evidence kind", &report.kind)?;
    validate_release_action_text("collected release evidence output", &report.output)?;
    validate_release_report_path_under(
        "collected release evidence output",
        &report.output,
        "collected release evidence dir",
        &report.evidence_dir,
    )?;
    if !matches!(report.kind.as_str(), "signing" | "notarization") {
        return Err(format!(
            "unsupported collected release evidence kind `{}`",
            report.kind
        )
        .into());
    }
    if let Some(expected) = expected_collected_release_evidence_output_path(report)?
        && !release_report_paths_equal(&report.output, &expected)
    {
        return Err(format!(
            "collected release evidence output `{}` does not match expected `{expected}` for kind `{}`",
            report.output, report.kind
        )
        .into());
    }
    validate_release_evidence_item_count(
        "collected release evidence report",
        report.items.len(),
        true,
    )?;
    for item in &report.items {
        validate_local_release_evidence_item_shape(
            "collected release evidence item",
            item,
            &["ok"],
        )?;
        if let Some(path) = &item.path {
            validate_release_report_path_under(
                &format!("collected release evidence item `{}` path", item.name),
                path,
                "collected release evidence dir",
                &report.evidence_dir,
            )?;
            if !release_report_paths_equal(path, &report.output) {
                return Err(format!(
                    "collected release evidence item `{}` path `{path}` must match report output `{}`",
                    item.name, report.output
                )
                .into());
            }
        }
    }
    Ok(())
}

fn expected_collected_release_evidence_output_path(
    report: &CollectedReleaseEvidenceReport,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let relative = match report.kind.as_str() {
        "notarization" => Some("notary.log"),
        "signing" => {
            let mut expected = BTreeSet::new();
            for item in &report.items {
                match item.name.as_str() {
                    "macOS signing verification" => {
                        expected.insert("signing-macos.log");
                    }
                    "Windows signing verification" => {
                        expected.insert("signing-windows.log");
                    }
                    _ => {}
                }
            }
            if expected.len() > 1 {
                return Err(
                    "collected signing report must not mix macOS and Windows output items".into(),
                );
            }
            expected.into_iter().next()
        }
        _ => None,
    };
    Ok(relative.map(|relative| format!("{}/{}", report.evidence_dir, relative)))
}

fn validate_local_release_evidence_item_paths(
    report: &LocalReleaseEvidenceReport,
    item: &LocalReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "protocol snapshot" {
        if let Some(path) = &item.path {
            validate_release_report_root_path(
                &format!("local release evidence item `{}` path", item.name),
                path,
            )?;
        }
        return Ok(());
    }
    if let Some(path) = &item.path {
        validate_release_report_path_under(
            &format!("local release evidence item `{}` path", item.name),
            path,
            "local release evidence dir",
            &report.evidence_dir,
        )?;
        validate_release_evidence_template_item_path(
            &format!("local release evidence item `{}` path", item.name),
            &item.name,
            path,
            &report.evidence_dir,
        )?;
        if let Some(expected) = expected_local_release_evidence_item_path(report, item)
            && !release_report_paths_equal(path, &expected)
        {
            return Err(format!(
                "local release evidence item `{}` path `{path}` does not match expected `{expected}`",
                item.name
            )
            .into());
        }
    }
    Ok(())
}

fn expected_local_release_evidence_item_path(
    report: &LocalReleaseEvidenceReport,
    item: &LocalReleaseEvidenceItem,
) -> Option<String> {
    let relative = fixed_release_evidence_item_relative_path(item.name.as_str())?;
    Some(format!("{}/{}", report.evidence_dir, relative))
}

fn validate_local_release_evidence_protocol_consistency(
    report: &LocalReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol_items = report
        .items
        .iter()
        .filter(|item| item.name == "protocol snapshot")
        .collect::<Vec<_>>();
    match report.protocol_snapshot.as_deref() {
        Some(protocol_snapshot) => {
            if protocol_items.len() != 1 {
                return Err(format!(
                    "local release protocol snapshot `{protocol_snapshot}` must match exactly one protocol snapshot item"
                )
                .into());
            }
            let item = protocol_items[0];
            if item.status != "ok"
                || !item
                    .path
                    .as_deref()
                    .is_some_and(|path| release_report_paths_equal(path, protocol_snapshot))
            {
                return Err(format!(
                    "local release protocol snapshot `{protocol_snapshot}` does not match protocol snapshot item status/path"
                )
                .into());
            }
        }
        None if !protocol_items.is_empty() => {
            return Err(
                "local release protocol snapshot item present but top-level protocol_snapshot is missing"
                    .into(),
            );
        }
        None => {}
    }
    Ok(())
}

fn validate_release_evidence_item_count(
    label: &str,
    count: usize,
    require_nonempty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if require_nonempty && count == 0 {
        return Err(format!("{label} has no items").into());
    }
    if count > RELEASE_EVIDENCE_REPORT_MAX_ITEMS {
        return Err(format!(
            "{label} has too many items: {count} exceeds maximum {RELEASE_EVIDENCE_REPORT_MAX_ITEMS}"
        )
        .into());
    }
    Ok(())
}

fn validate_local_release_evidence_item_shape(
    label: &str,
    item: &LocalReleaseEvidenceItem,
    allowed_statuses: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text(&format!("{label} name"), &item.name)?;
    validate_release_action_text(&format!("{label} status"), &item.status)?;
    if !allowed_statuses.contains(&item.status.as_str()) {
        return Err(format!(
            "{label} `{}` has unsupported status `{}`",
            item.name, item.status
        )
        .into());
    }
    if let Some(path) = &item.path {
        validate_release_action_text(&format!("{label} `{}` path", item.name), path)?;
    }
    validate_release_action_text(&format!("{label} `{}` value", item.name), &item.value)?;
    validate_release_evidence_template_item_status(label, &item.name, &item.status)?;
    if item.status == "ok" && item.path.is_none() {
        return Err(format!(
            "{label} `{}` status ok must include an evidence path",
            item.name
        )
        .into());
    }
    Ok(())
}

fn validate_import_ci_release_evidence_item_shape(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("import-ci item name", &item.name)?;
    validate_release_action_text(&format!("import-ci `{}` status", item.name), &item.status)?;
    if !matches!(
        item.status.as_str(),
        "ok" | "imported" | "skipped" | "failed"
    ) {
        return Err(format!(
            "import-ci item `{}` has unsupported status `{}`",
            item.name, item.status
        )
        .into());
    }
    if let Some(source) = &item.source {
        validate_release_action_text(&format!("import-ci `{}` source", item.name), source)?;
    }
    if let Some(path) = &item.path {
        validate_release_action_text(&format!("import-ci `{}` path", item.name), path)?;
    }
    validate_release_action_text(&format!("import-ci `{}` value", item.name), &item.value)?;
    validate_release_evidence_template_item_status("import-ci item", &item.name, &item.status)?;
    validate_import_ci_release_evidence_item_source_semantics(item)?;
    validate_import_ci_release_evidence_item_value(item)?;
    match item.status.as_str() {
        "ok" => {
            if item.path.is_none() {
                return Err(format!(
                    "import-ci item `{}` status ok must include an output path",
                    item.name
                )
                .into());
            }
        }
        "imported" => {
            if item.path.is_none() {
                return Err(format!(
                    "import-ci item `{}` status imported must include an output path",
                    item.name
                )
                .into());
            }
        }
        "failed" => {
            if item.path.is_some() {
                return Err(format!(
                    "import-ci item `{}` status failed must not include an output path",
                    item.name
                )
                .into());
            }
        }
        "skipped"
            if item.path.is_some()
                && !item
                    .value
                    .contains(ImportWriteOutcome::SkippedExisting.value()) =>
        {
            return Err(format!(
                "import-ci item `{}` status skipped may include an output path only when the destination already exists",
                item.name
            )
            .into());
        }
        "skipped" => {}
        _ => {}
    }
    Ok(())
}

fn validate_release_evidence_template_item_status(
    label: &str,
    item_name: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if item_name == "release evidence template" && status != "ok" {
        return Err(format!("{label} `release evidence template` status must be ok").into());
    }
    Ok(())
}

fn validate_import_ci_release_evidence_item_source_semantics(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "release evidence template" && item.source.is_some() {
        return Err("import-ci item `release evidence template` must not include a source".into());
    }
    Ok(())
}

fn validate_import_ci_release_evidence_item_value(
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if item.name == "ci run url"
        && matches!(item.status.as_str(), "ok" | "imported")
        && !is_github_actions_run_url(&item.value)
    {
        return Err(format!(
            "import-ci item `ci run url` status {} must contain a GitHub Actions run URL value",
            item.status
        )
        .into());
    }
    Ok(())
}

fn validate_import_ci_release_evidence_item_paths(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(source) = &item.source {
        if import_ci_item_source_may_be_external(item) {
            validate_release_report_root_path(
                &format!("import-ci `{}` source", item.name),
                source,
            )?;
        } else {
            validate_release_report_path_under(
                &format!("import-ci `{}` source", item.name),
                source,
                "import-ci source",
                &report.source,
            )?;
        }
    }
    if let Some(path) = &item.path {
        validate_release_report_path_under(
            &format!("import-ci `{}` path", item.name),
            path,
            "import-ci evidence dir",
            &report.evidence_dir,
        )?;
        validate_release_evidence_template_item_path(
            &format!("import-ci `{}` path", item.name),
            &item.name,
            path,
            &report.evidence_dir,
        )?;
        if let Some(expected) = expected_import_ci_release_evidence_item_path(report, item)
            && !release_report_paths_equal(path, &expected)
        {
            return Err(format!(
                "import-ci item `{}` path `{path}` does not match expected `{expected}`",
                item.name
            )
            .into());
        }
        validate_import_ci_release_evidence_item_dynamic_path(report, item, path)?;
    }
    Ok(())
}

fn expected_import_ci_release_evidence_item_path(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
) -> Option<String> {
    let relative = fixed_release_evidence_item_relative_path(item.name.as_str())?;
    Some(format!("{}/{}", report.evidence_dir, relative))
}

fn fixed_release_evidence_item_relative_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "ci run url" => "ci-run-url.txt",
        "protocol snapshot" => "vesty-protocol",
        "crate publish plan" => "publish-plan/publish-plan.json",
        "crate package readiness" => "crate-package/crate-package.json",
        "npm package pack report" => "npm-pack/npm-pack.json",
        "dependency latest baseline" => "dependency-baseline/dependency-baseline-latest.json",
        "vst3 SDK header manifest" => "vst3-sdk/vst3-sdk-headers.json",
        "vst3 SDK generated bindings plan" => "vst3-sdk/generated-bindings-plan.json",
        "vst3 SDK generated bindings surface" => "vst3-sdk/generated-bindings-surface.json",
        "vst3 SDK generated bindings scaffold" => "vst3-sdk/generated.rs",
        "vst3 SDK generated bindings ABI seed" => "vst3-sdk/generated-abi-seed.rs",
        "vst3 SDK generated bindings ABI layout" => "vst3-sdk/generated-abi.rs",
        "vst3 SDK generated bindings interface skeleton" => {
            "vst3-sdk/generated-interface-skeleton.rs"
        }
        "notarization log" => "notary.log",
        _ => return None,
    })
}

fn validate_release_evidence_template_item_path(
    label: &str,
    item_name: &str,
    path: &str,
    evidence_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if item_name == "release evidence template" && !release_report_paths_equal(path, evidence_dir) {
        return Err(
            format!("{label} `{path}` must match release evidence dir `{evidence_dir}`").into(),
        );
    }
    Ok(())
}

fn import_ci_item_source_may_be_external(item: &ImportCiReleaseEvidenceItem) -> bool {
    item.name == "ci run url"
}

fn validate_import_ci_release_evidence_item_dynamic_path(
    report: &ImportCiReleaseEvidenceReport,
    item: &ImportCiReleaseEvidenceItem,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(expectation) = import_ci_dynamic_output_expectation(item.name.as_str()) else {
        return Ok(());
    };

    let evidence_root =
        lexical_release_report_root_path(&report.evidence_dir).map_err(|error| {
            format!(
                "import-ci evidence dir `{}` is not a safe report path: {error}",
                report.evidence_dir
            )
        })?;
    let item_path = lexical_release_report_path(path).map_err(|error| {
        format!(
            "import-ci `{}` path `{path}` is not a safe report path: {error}",
            item.name
        )
    })?;
    let relative = item_path.strip_prefix(&evidence_root).ok_or_else(|| {
        format!(
            "import-ci `{}` path `{path}` must be under import-ci evidence dir `{}`",
            item.name, report.evidence_dir
        )
    })?;
    if import_ci_relative_output_path_is_allowed(item.name.as_str(), relative) {
        Ok(())
    } else {
        Err(format!(
            "import-ci `{}` path `{path}` must be {expectation} under `{}`",
            item.name, report.evidence_dir
        )
        .into())
    }
}

fn import_ci_dynamic_output_expectation(item_name: &str) -> Option<&'static str> {
    match item_name {
        "ci doctor artifact" => Some("`ci-doctor/doctor-<OS>.json`"),
        "ci release-check artifact" => Some("`ci-release-checks/release-check-<OS>.json`"),
        "release action plan sidecar" => Some("`ci-release-checks/release-action-plan-<OS>.json`"),
        "platform smoke artifact" => Some("`platform-smoke/<platform>.json`"),
        "vst3 validate report" => Some("`validator/<safe-bundle>.<platform>.validate.json`"),
        "vst3 static validate report" => {
            Some("`package/<safe-bundle>.<platform>.static-validate.json`")
        }
        "signed bundle evidence" => Some(
            "`signing-macos.log`, `signing-windows.log`, `signing/<safe-name>.log`, or `signed-bundles/<safe-bundle>.vst3`",
        ),
        _ => None,
    }
}

fn import_ci_relative_output_path_is_allowed(item_name: &str, relative: &[String]) -> bool {
    match item_name {
        "ci doctor artifact" => ci_doctor_relative_output_path_is_allowed(relative),
        "ci release-check artifact" => ci_release_check_relative_output_path_is_allowed(relative),
        "release action plan sidecar" => {
            release_action_plan_sidecar_relative_output_path_is_allowed(relative)
        }
        "platform smoke artifact" => platform_smoke_relative_output_path_is_allowed(relative),
        "vst3 validate report" => {
            validate_report_relative_output_path_is_allowed(relative, "validator", "validate.json")
        }
        "vst3 static validate report" => validate_report_relative_output_path_is_allowed(
            relative,
            "package",
            "static-validate.json",
        ),
        "signed bundle evidence" => {
            signed_bundle_evidence_relative_output_path_is_allowed(relative)
        }
        _ => false,
    }
}

fn ci_doctor_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "ci-doctor" && matches!(
        file.as_str(),
        "doctor-Linux.json" | "doctor-macOS.json" | "doctor-Windows.json"
    ))
}

fn ci_release_check_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "ci-release-checks" && matches!(
        file.as_str(),
        "release-check-Linux.json" | "release-check-macOS.json" | "release-check-Windows.json"
    ))
}

fn release_action_plan_sidecar_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "ci-release-checks" && matches!(
        file.as_str(),
        "release-action-plan-Linux.json"
            | "release-action-plan-macOS.json"
            | "release-action-plan-Windows.json"
    ))
}

fn platform_smoke_relative_output_path_is_allowed(relative: &[String]) -> bool {
    matches!(relative, [dir, file] if dir == "platform-smoke" && matches!(
        file.as_str(),
        "macos.json" | "windows-x64.json" | "linux-x11.json"
    ))
}

fn validate_report_relative_output_path_is_allowed(
    relative: &[String],
    directory: &str,
    suffix: &str,
) -> bool {
    match relative {
        [dir, file] if dir == directory => {
            file.ends_with(suffix) && safe_evidence_filename_part(file) == *file
        }
        _ => false,
    }
}

fn signed_bundle_evidence_relative_output_path_is_allowed(relative: &[String]) -> bool {
    match relative {
        [file] if matches!(file.as_str(), "signing-macos.log" | "signing-windows.log") => true,
        [dir, file] if dir == "signing" => {
            file.ends_with(".log") && safe_evidence_filename_part(file) == *file
        }
        [dir, bundle] if dir == "signed-bundles" => {
            bundle.ends_with(".vst3") && safe_evidence_filename_part(bundle) == *bundle
        }
        _ => false,
    }
}

fn validate_import_ci_release_evidence_success_output_uniqueness(
    report: &ImportCiReleaseEvidenceReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen_paths = BTreeMap::<String, &str>::new();
    for item in &report.items {
        if !matches!(item.status.as_str(), "ok" | "imported") {
            continue;
        }
        let Some(path) = item.path.as_deref() else {
            continue;
        };
        let key = lexical_release_report_path(path)
            .map(|path| format!("{}|{}", path.prefix, path.components.join("/")))
            .map_err(|error| {
                format!(
                    "import-ci item `{}` output path `{path}` is not a safe report path: {error}",
                    item.name
                )
            })?;
        if let Some(previous_name) = seen_paths.insert(key, item.name.as_str()) {
            return Err(format!(
                "import-ci item `{}` output path `{path}` duplicates successful item `{previous_name}`",
                item.name
            )
            .into());
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LexicalReleaseReportPath {
    prefix: String,
    components: Vec<String>,
}

impl LexicalReleaseReportPath {
    fn starts_with(&self, root: &Self) -> bool {
        self.prefix == root.prefix && self.components.starts_with(&root.components)
    }

    fn strip_prefix(&self, root: &Self) -> Option<&[String]> {
        self.starts_with(root)
            .then(|| &self.components[root.components.len()..])
    }
}

fn validate_release_report_path_under(
    child_label: &str,
    child: &str,
    root_label: &str,
    root: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let child_path = lexical_release_report_path(child)
        .map_err(|error| format!("{child_label} `{child}` is not a safe report path: {error}"))?;
    let root_path = lexical_release_report_root_path(root)
        .map_err(|error| format!("{root_label} `{root}` is not a safe report path: {error}"))?;
    if !child_path.starts_with(&root_path) {
        return Err(format!("{child_label} `{child}` must be under {root_label} `{root}`").into());
    }
    Ok(())
}

fn validate_release_report_root_path(
    label: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    lexical_release_report_root_path(path)
        .map(|_| ())
        .map_err(|error| format!("{label} `{path}` is not a safe report path: {error}").into())
}

fn lexical_release_report_root_path(path: &str) -> Result<LexicalReleaseReportPath, String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized
        .split('/')
        .any(|component| component.trim() == "..")
    {
        return Err("root path must not contain parent-directory components".to_string());
    }
    lexical_release_report_path(path)
}

fn lexical_release_report_path(path: &str) -> Result<LexicalReleaseReportPath, String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("path is empty".to_string());
    }

    let mut prefix = String::new();
    let mut rest = normalized.as_str();
    if rest.starts_with('/') {
        prefix.push('/');
        rest = rest.trim_start_matches('/');
    } else if let Some(drive) = windows_drive_prefix(rest) {
        prefix.push_str(&drive.to_ascii_lowercase());
        rest = rest[drive.len()..].trim_start_matches('/');
    }

    let absolute = !prefix.is_empty();
    let mut components = Vec::new();
    for component in rest.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            if components.last().is_some_and(|last| last != "..") {
                components.pop();
            } else if absolute {
                return Err("path escapes an absolute root".to_string());
            } else {
                components.push(component.to_string());
            }
            continue;
        }
        components.push(component.to_string());
    }

    Ok(LexicalReleaseReportPath { prefix, components })
}

fn windows_drive_prefix(path: &str) -> Option<&str> {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic() {
        Some(&path[..2])
    } else {
        None
    }
}

fn path_is_under_any(path: &Utf8Path, roots: &[Utf8PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

fn print_import_ci_release_evidence_report(
    report: &ImportCiReleaseEvidenceReport,
    format: OutputFormat,
    report_path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_import_ci_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("CI release evidence import: {}", report.evidence_dir);
            for item in &report.items {
                let source = item
                    .source
                    .as_deref()
                    .map(|source| format!(" from {source}"))
                    .unwrap_or_default();
                let path = item
                    .path
                    .as_deref()
                    .map(|path| format!(" -> {path}"))
                    .unwrap_or_default();
                println!(
                    "- {}: {}{}{} - {}",
                    item.name, item.status, source, path, item.value
                );
            }
            println!("- report: {report_path}");
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}

fn collect_protocol_snapshot_dirs_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut dirs = Vec::new();
    collect_protocol_snapshot_dirs_recursive_inner(root, &mut dirs)?;
    dirs.sort();
    Ok(dirs)
}

fn collect_protocol_snapshot_dirs_recursive_inner(
    current: &Utf8Path,
    dirs: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(current)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("protocol artifact contains symlink: {current}").into());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    if current.join("typescript").is_dir() && current.join("json-schema").is_dir() {
        dirs.push(current.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "protocol artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("protocol artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_protocol_snapshot_dirs_recursive_inner(&path, dirs)?;
        }
    }
    Ok(())
}

fn artifact_os_from_path(path: &Utf8Path) -> Option<&'static str> {
    doctor_artifact_os(path).or_else(|| os_from_artifact_path_tokens(&artifact_path_tokens(path)))
}

fn artifact_path_tokens(path: &Utf8Path) -> Vec<String> {
    path.as_str()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn os_from_artifact_path_tokens(tokens: &[String]) -> Option<&'static str> {
    if tokens.iter().any(|token| token == "linux") {
        Some("Linux")
    } else if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        Some("macOS")
    } else if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "windows" | "windowsx64" | "win" | "win32" | "win64"
        )
    }) {
        Some("Windows")
    } else {
        None
    }
}

fn import_doctor_artifact_os(
    path: &Utf8Path,
    report: &DoctorReport,
) -> Result<&'static str, String> {
    let file_os = artifact_os_from_path(path);
    let report_os = report.os.as_deref().and_then(normalize_doctor_os_label);
    match (file_os, report_os) {
        (Some(file_os), Some(report_os)) if file_os == report_os => Ok(file_os),
        (Some(file_os), Some(report_os)) => Err(format!(
            "artifact path indicates {file_os}, but report os is {report_os}"
        )),
        (Some(file_os), None) => Ok(file_os),
        (None, Some(report_os)) => Ok(report_os),
        (None, None) => Err("could not infer OS from artifact path or report".to_string()),
    }
}

fn validate_import_ci_run_match(
    actual_url: Option<&str>,
    expected_url: Option<&str>,
) -> Result<(), String> {
    let (Some(actual_url), Some(expected_url)) = (actual_url, expected_url) else {
        return Ok(());
    };
    let Some(actual) = github_actions_run_key(actual_url) else {
        return Err(format!("artifact has invalid ci_run_url `{actual_url}`"));
    };
    let Some(expected) = github_actions_run_key(expected_url) else {
        return Err(format!("expected CI run URL is invalid `{expected_url}`"));
    };
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "artifact is from {}/{} run {}, expected {}/{} run {}",
            actual.owner,
            actual.repo,
            actual.run_id,
            expected.owner,
            expected.repo,
            expected.run_id
        ))
    }
}

fn validate_report_artifact_path_platform(
    path: &Utf8Path,
    report: &ValidateReport,
) -> Result<(), String> {
    let Some(path_platform) = validate_report_platform_from_artifact_path(path)? else {
        return Ok(());
    };
    let platforms = validate_report_platforms(report);
    if platforms.contains(path_platform) {
        Ok(())
    } else {
        let report_platforms = if platforms.is_empty() {
            "unknown".to_string()
        } else {
            format_platform_set(&platforms)
        };
        Err(format!(
            "artifact path indicates {path_platform}, but report platform is {report_platforms}"
        ))
    }
}

fn validate_report_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    match validate_report_file_name_platform(path, &["linux-x64", "macos", "windows-x64"]) {
        Ok(Some(platform)) => Ok(Some(platform)),
        Ok(None) => Ok(validate_report_parent_dir_platform(path)),
        Err(error) => Err(error.to_string()),
    }
}

fn validate_report_parent_dir_platform(path: &Utf8Path) -> Option<&'static str> {
    let parent = path.parent()?.file_name()?;
    let tokens = path_component_tokens(parent);
    match tokens
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["linux", "x64"] | ["linuxx64"] => Some("linux-x64"),
        ["macos"] | ["mac"] | ["darwin"] | ["osx"] => Some("macos"),
        ["windows"] | ["windows", "x64"] | ["windowsx64"] | ["win64"] => Some("windows-x64"),
        _ => None,
    }
}

fn path_component_tokens(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn validate_report_import_filename(report: &ValidateReport, kind: &str) -> String {
    let bundle = safe_evidence_filename_part(&validate_report_bundle_name(report));
    let mut platforms = validate_report_platforms(report)
        .into_iter()
        .collect::<Vec<_>>();
    platforms.sort_unstable();
    let platform = if platforms.is_empty() {
        "unknown".to_string()
    } else {
        platforms.join("+")
    };
    format!(
        "{bundle}.{platform}.{kind}.json",
        platform = safe_evidence_filename_part(&platform)
    )
}

fn signing_import_destination(
    dir: &Utf8Path,
    source: &Utf8Path,
    platforms: &BTreeSet<SigningEvidencePlatform>,
) -> Utf8PathBuf {
    if platforms.contains(&SigningEvidencePlatform::Macos)
        && !platforms.contains(&SigningEvidencePlatform::Windows)
    {
        dir.join("signing-macos.log")
    } else if platforms.contains(&SigningEvidencePlatform::Windows)
        && !platforms.contains(&SigningEvidencePlatform::Macos)
    {
        dir.join("signing-windows.log")
    } else {
        dir.join("signing").join(format!(
            "{}.log",
            safe_evidence_filename_part(source.file_stem().unwrap_or("signing"))
        ))
    }
}

fn safe_evidence_filename_part(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            ch
        } else {
            '-'
        };
        if mapped == '-' {
            if !last_dash {
                out.push(mapped);
            }
            last_dash = true;
        } else {
            out.push(mapped);
            last_dash = false;
        }
    }
    let trimmed = out.trim_matches(['-', '.', '_']).to_string();
    if trimmed.is_empty() {
        "artifact".to_string()
    } else {
        trimmed
    }
}

#[derive(Clone, Debug)]
struct CollectSigningOptions {
    bundle: Utf8PathBuf,
    platform: Option<String>,
    binary: Option<Utf8PathBuf>,
    dir: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    tool: Option<Utf8PathBuf>,
    format: String,
}

#[derive(Clone, Debug)]
struct CollectNotarizationOptions {
    notary_log: Utf8PathBuf,
    stapler_log: Option<Utf8PathBuf>,
    dir: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CollectedReleaseEvidenceReport {
    evidence_dir: String,
    kind: String,
    output: String,
    items: Vec<LocalReleaseEvidenceItem>,
}

fn collect_signing_release_evidence(
    options: CollectSigningOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", &options.dir)?;
    let platform = match options.platform.as_deref() {
        Some(platform) => parse_bundle_platform(platform)?,
        None => infer_signing_bundle_platform(&options.bundle)?,
    };
    let output_path = options
        .out
        .unwrap_or_else(|| default_signing_evidence_path(&options.dir, platform));
    let command = signing_verification_command(
        platform,
        &options.bundle,
        options.binary.as_deref(),
        options.tool.as_deref(),
    )?;
    let output = run_command_spec_capture(&command, "signing verification")?;
    let text = captured_command_log(&command, &output);
    let platforms = signing_evidence_platforms_from_text(&text)?;
    let expected = signing_platform_for_bundle_platform(platform)?;
    if !platforms.contains(&expected) {
        return Err(format!(
            "signing verification output did not prove {} coverage",
            expected.label()
        )
        .into());
    }

    let report = CollectedReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        kind: "signing".to_string(),
        output: portable_report_path(&output_path),
        items: vec![LocalReleaseEvidenceItem {
            name: format!("{} signing verification", expected.label()),
            status: "ok".to_string(),
            path: Some(portable_report_path(&output_path)),
            value: format!("{} {}", command.program, command.args.join(" ")),
        }],
    };
    validate_collected_release_evidence_report_shape(&report)?;
    write_text_file(&output_path, &text)?;
    print_collected_release_evidence_report(&report, format)
}

fn collect_notarization_release_evidence(
    options: CollectNotarizationOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    create_directory_no_parent_or_leaf_symlink("release evidence dir", &options.dir)?;
    let output_path = options
        .out
        .unwrap_or_else(|| options.dir.join("notary.log"));
    let mut text = String::new();
    text.push_str(&format!("notary_log={}\n", options.notary_log));
    text.push_str("\n[notarytool]\n");
    text.push_str(&read_text_file_no_symlink(
        "notarytool log",
        &options.notary_log,
    )?);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(stapler_log) = options.stapler_log.as_ref() {
        text.push_str(&format!("\nstapler_log={stapler_log}\n"));
        text.push_str("\n[stapler]\n");
        text.push_str(&read_text_file_no_symlink("stapler log", stapler_log)?);
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    let evidence = notarization_evidence_from_text(&text)?;
    if !evidence.accepted || !evidence.stapled {
        let mut missing = Vec::new();
        if !evidence.accepted {
            missing.push("accepted notarytool result");
        }
        if !evidence.stapled {
            missing.push("stapler success");
        }
        return Err(format!(
            "notarization collection is incomplete: missing {}",
            missing.join(", ")
        )
        .into());
    }

    let report = CollectedReleaseEvidenceReport {
        evidence_dir: portable_report_path(&options.dir),
        kind: "notarization".to_string(),
        output: portable_report_path(&output_path),
        items: vec![LocalReleaseEvidenceItem {
            name: "macOS notarization".to_string(),
            status: "ok".to_string(),
            path: Some(portable_report_path(&output_path)),
            value: "accepted notarytool result and stapler success".to_string(),
        }],
    };
    validate_collected_release_evidence_report_shape(&report)?;
    write_text_file(&output_path, &text)?;
    print_collected_release_evidence_report(&report, format)
}

fn print_collected_release_evidence_report(
    report: &CollectedReleaseEvidenceReport,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_collected_release_evidence_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("{} release evidence: {}", report.kind, report.output);
            for item in &report.items {
                println!("- {}: {} - {}", item.name, item.status, item.value);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PublishPlan {
    packages: Vec<PublishPlanPackage>,
    skipped_private: Vec<String>,
}

const PUBLISH_PLAN_MAX_PACKAGES: usize = 128;
const PUBLISH_PLAN_MAX_DEPENDENCIES: usize = 128;
const PUBLISH_PLAN_MAX_SKIPPED_PRIVATE: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PublishPlanPackage {
    order: usize,
    level: usize,
    name: String,
    version: String,
    manifest_path: String,
    internal_dependencies: Vec<String>,
}

fn run_publish_plan(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty publish-plan --check` requires `--out <report>`".into());
        };
        let plan = read_publish_plan_report(report_path)?;
        validate_publish_plan(&plan)?;
        print_publish_plan(&plan, format, Some(report_path))?;
        return Ok(());
    }

    let metadata = cargo_metadata_json(workspace)?;
    let plan = workspace_publish_plan(&metadata)?;
    if let Some(path) = out {
        write_publish_plan_report(path, &plan)?;
    }
    print_publish_plan(&plan, format, out)
}

const CRATE_PACKAGE_REPORT_VERSION: u32 = 1;
const CRATE_PACKAGE_REPORT_GENERATOR: &str = "vesty-cli.crate-package.v1";
const CRATE_PACKAGE_MAX_PACKAGES: usize = 128;
const CRATE_PACKAGE_MAX_DEPENDENCIES: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CratePackageReport {
    version: u32,
    generator: String,
    status: String,
    publish_plan: PublishPlan,
    packages: Vec<CratePackageEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CratePackageEntry {
    name: String,
    version: String,
    manifest_path: String,
    publish_order: usize,
    internal_dependencies: Vec<String>,
    status: String,
    reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CratePackageEvidence {
    package_count: usize,
    packaged_count: usize,
    deferred_count: usize,
}

fn run_crate_package(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty crate-package --check` requires `--out <report>`".into());
        };
        let report = read_crate_package_report(report_path)?;
        let evidence = validate_crate_package_report(&report)?;
        print_crate_package_report(&report, &evidence, format, Some(report_path))?;
        return Ok(());
    }

    let report = crate_package_report(workspace, true)?;
    let evidence = validate_crate_package_report(&report)?;
    if let Some(path) = out {
        write_crate_package_report(path, &report)?;
    }
    print_crate_package_report(&report, &evidence, format, out)?;
    if report.status == "ok" {
        Ok(())
    } else {
        Err("crate package readiness failed; inspect failed package entries".into())
    }
}

fn crate_package_report(
    workspace: &Utf8Path,
    run_package: bool,
) -> Result<CratePackageReport, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata_json(workspace)?;
    let plan = workspace_publish_plan(&metadata)?;
    validate_publish_plan(&plan)?;

    let mut entries = Vec::new();
    for package in &plan.packages {
        if package.internal_dependencies.is_empty() {
            let (status, reason) = if run_package {
                match cargo_package_workspace_crate(workspace, &package.name) {
                    Ok(()) => ("packaged".to_string(), None),
                    Err(error) => ("failed".to_string(), Some(error)),
                }
            } else {
                ("packaged".to_string(), None)
            };
            entries.push(CratePackageEntry {
                name: package.name.clone(),
                version: package.version.clone(),
                manifest_path: package.manifest_path.clone(),
                publish_order: package.order,
                internal_dependencies: Vec::new(),
                status,
                reason,
            });
        } else {
            let dependencies = package.internal_dependencies.clone();
            entries.push(CratePackageEntry {
                name: package.name.clone(),
                version: package.version.clone(),
                manifest_path: package.manifest_path.clone(),
                publish_order: package.order,
                internal_dependencies: dependencies.clone(),
                status: "deferred".to_string(),
                reason: Some(format!(
                    "requires published internal dependencies: {}",
                    dependencies.join(", ")
                )),
            });
        }
    }

    let status = if entries.iter().any(|entry| entry.status == "failed") {
        "failed"
    } else {
        "ok"
    };
    Ok(CratePackageReport {
        version: CRATE_PACKAGE_REPORT_VERSION,
        generator: CRATE_PACKAGE_REPORT_GENERATOR.to_string(),
        status: status.to_string(),
        publish_plan: plan,
        packages: entries,
    })
}

fn cargo_package_workspace_crate(workspace: &Utf8Path, package: &str) -> Result<(), String> {
    let output = Command::new("cargo")
        .current_dir(workspace)
        .args(["package", "-p", package, "--allow-dirty", "--no-verify"])
        .output()
        .map_err(|error| format!("failed to run cargo package for {package}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo package failed for {package}: status {}; {}",
        output.status,
        captured_output_summary(&output)
    ))
}

fn captured_output_summary(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut parts = Vec::new();
    if !stdout.trim().is_empty() {
        parts.push(format!(
            "stdout: {}",
            truncate_for_report(stdout.trim(), 2_000)
        ));
    }
    if !stderr.trim().is_empty() {
        parts.push(format!(
            "stderr: {}",
            truncate_for_report(stderr.trim(), 4_000)
        ));
    }
    if parts.is_empty() {
        "no stdout/stderr captured".to_string()
    } else {
        parts.join("; ")
    }
}

fn truncate_for_report(text: &str, max_chars: usize) -> String {
    let mut sanitized = String::new();
    let mut previous_space = false;
    let mut truncated = false;
    let mut char_count = 0;
    for raw in text.chars() {
        let ch = if raw.is_control() || is_release_action_unsafe_format_char(raw) {
            ' '
        } else {
            raw
        };
        if ch.is_whitespace() {
            if !sanitized.is_empty() && !previous_space {
                if char_count >= max_chars {
                    truncated = true;
                    break;
                }
                sanitized.push(' ');
                previous_space = true;
                char_count += 1;
            }
            continue;
        }
        if char_count >= max_chars {
            truncated = true;
            break;
        }
        sanitized.push(ch);
        previous_space = false;
        char_count += 1;
    }

    let mut sanitized = sanitized.trim().to_string();
    if truncated {
        sanitized.push_str("...");
    }
    sanitized
}

fn write_crate_package_report(
    path: &Utf8Path,
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;
    write_text_file(path, &(serde_json::to_string_pretty(report)? + "\n"))
}

fn read_crate_package_report(
    path: &Utf8Path,
) -> Result<CratePackageReport, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("crate package report", path)?;
    serde_json::from_str::<CratePackageReport>(&text)
        .map_err(|error| format!("invalid crate package report JSON: {error}").into())
}

fn validate_crate_package_report_path(
    path: &Utf8Path,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    let report = read_crate_package_report(path)?;
    validate_crate_package_report(&report)
}

fn validate_crate_package_report_path_with_publish_plan(
    path: &Utf8Path,
    publish_plan_path: Option<&Utf8Path>,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    let report = read_crate_package_report(path)?;
    let evidence = validate_crate_package_report(&report)?;
    if let Some(publish_plan_path) = publish_plan_path {
        let plan = read_publish_plan_report(publish_plan_path)?;
        validate_publish_plan(&plan)?;
        validate_crate_package_publish_plan_identity(&report.publish_plan, &plan).map_err(
            |error| {
                format!("crate package report does not match crate publish plan evidence: {error}")
            },
        )?;
    }
    Ok(evidence)
}

fn validate_crate_package_report(
    report: &CratePackageReport,
) -> Result<CratePackageEvidence, Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;

    if report.version != CRATE_PACKAGE_REPORT_VERSION {
        return Err(format!(
            "unsupported crate package report version {}; expected {}",
            report.version, CRATE_PACKAGE_REPORT_VERSION
        )
        .into());
    }
    if report.generator != CRATE_PACKAGE_REPORT_GENERATOR {
        return Err(format!(
            "unsupported crate package report generator {}; expected {}",
            report.generator, CRATE_PACKAGE_REPORT_GENERATOR
        )
        .into());
    }
    if report.packages.is_empty() {
        return Err("crate package report has no packages".into());
    }
    validate_publish_plan(&report.publish_plan)
        .map_err(|error| format!("crate package report publish plan is invalid: {error}"))?;
    validate_crate_package_report_entries_match_publish_plan(report)?;

    let mut seen_names = BTreeSet::<&str>::new();
    let mut seen_orders = BTreeSet::<usize>::new();
    let mut packaged_count = 0;
    let mut deferred_count = 0;
    let mut errors = Vec::new();
    let mut order_by_name = BTreeMap::<&str, usize>::new();

    for (index, package) in report.packages.iter().enumerate() {
        if package.name.trim().is_empty() {
            errors.push(format!("package at index {index} has empty name"));
        }
        if package.version.trim().is_empty() {
            errors.push(format!("package {} has empty version", package.name));
        }
        if package.manifest_path.trim().is_empty() {
            errors.push(format!("package {} has empty manifest path", package.name));
        }
        if package.publish_order != index + 1 {
            errors.push(format!(
                "package {} has publishOrder {}, expected {}",
                package.name,
                package.publish_order,
                index + 1
            ));
        }
        if !seen_names.insert(package.name.as_str()) {
            errors.push(format!("duplicate package {}", package.name));
        }
        if !seen_orders.insert(package.publish_order) {
            errors.push(format!("duplicate publishOrder {}", package.publish_order));
        }
        order_by_name.insert(package.name.as_str(), package.publish_order);

        match (
            package.internal_dependencies.is_empty(),
            package.status.as_str(),
        ) {
            (true, "packaged") => {
                packaged_count += 1;
                if package.reason.is_some() {
                    errors.push(format!(
                        "package {} is packaged but has a reason",
                        package.name
                    ));
                }
            }
            (true, "failed") => errors.push(format!(
                "package {} failed packaging: {}",
                package.name,
                package.reason.as_deref().unwrap_or("<missing reason>")
            )),
            (true, other) => errors.push(format!(
                "package {} has no internal dependencies but status is {other}, expected packaged",
                package.name
            )),
            (false, "deferred") => {
                deferred_count += 1;
                if package
                    .reason
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
                {
                    errors.push(format!("deferred package {} has no reason", package.name));
                }
            }
            (false, other) => errors.push(format!(
                "package {} has internal dependencies but status is {other}, expected deferred",
                package.name
            )),
        }
    }

    for package in &report.packages {
        let Some(package_order) = order_by_name.get(package.name.as_str()).copied() else {
            continue;
        };
        for dependency in &package.internal_dependencies {
            match order_by_name.get(dependency.as_str()).copied() {
                Some(dependency_order) if dependency_order < package_order => {}
                Some(_) => errors.push(format!(
                    "package {} depends on {} but the dependency is not earlier in publish order",
                    package.name, dependency
                )),
                None => errors.push(format!(
                    "package {} depends on unknown package {}",
                    package.name, dependency
                )),
            }
        }
    }

    if report.status != "ok" {
        errors.push(format!("report status is {}", report.status));
    }
    if packaged_count == 0 {
        errors.push("crate package report has no immediately packageable crates".to_string());
    }

    if !errors.is_empty() {
        return Err(format!("crate package readiness failed: {}", errors.join("; ")).into());
    }

    Ok(CratePackageEvidence {
        package_count: report.packages.len(),
        packaged_count,
        deferred_count,
    })
}

fn validate_crate_package_report_entries_match_publish_plan(
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    if report.packages.len() != report.publish_plan.packages.len() {
        errors.push(format!(
            "crate package report has {} package entries but embedded publish plan has {}",
            report.packages.len(),
            report.publish_plan.packages.len()
        ));
    }

    for (index, expected) in report.publish_plan.packages.iter().enumerate() {
        let Some(actual) = report.packages.get(index) else {
            errors.push(format!(
                "embedded publish plan package {} is missing from crate package entries",
                expected.name
            ));
            continue;
        };
        if actual.name != expected.name {
            errors.push(format!(
                "package entry at index {index} is {}, expected {} from embedded publish plan",
                actual.name, expected.name
            ));
        }
        if actual.version != expected.version {
            errors.push(format!(
                "package {} has version {}, expected {} from embedded publish plan",
                actual.name, actual.version, expected.version
            ));
        }
        if actual.manifest_path != expected.manifest_path {
            errors.push(format!(
                "package {} has manifest path {}, expected {} from embedded publish plan",
                actual.name, actual.manifest_path, expected.manifest_path
            ));
        }
        if actual.publish_order != expected.order {
            errors.push(format!(
                "package {} has publishOrder {}, expected {} from embedded publish plan",
                actual.name, actual.publish_order, expected.order
            ));
        }
        if actual.internal_dependencies != expected.internal_dependencies {
            errors.push(format!(
                "package {} has internal dependencies {:?}, expected {:?} from embedded publish plan",
                actual.name, actual.internal_dependencies, expected.internal_dependencies
            ));
        }
    }

    for actual in report
        .packages
        .iter()
        .skip(report.publish_plan.packages.len())
    {
        errors.push(format!(
            "crate package entry {} is not present in embedded publish plan",
            actual.name
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "crate package report is out of sync with embedded publish plan: {}",
            errors.join("; ")
        )
        .into())
    }
}

fn validate_crate_package_publish_plan_identity(
    embedded: &PublishPlan,
    external: &PublishPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    if embedded.packages.len() != external.packages.len() {
        errors.push(format!(
            "embedded publish plan has {} packages but external publish plan has {}",
            embedded.packages.len(),
            external.packages.len()
        ));
    }
    if embedded.skipped_private != external.skipped_private {
        errors.push(format!(
            "embedded publish plan skipped_private {:?}, expected {:?}",
            embedded.skipped_private, external.skipped_private
        ));
    }

    for (index, expected) in external.packages.iter().enumerate() {
        let Some(actual) = embedded.packages.get(index) else {
            errors.push(format!(
                "external publish plan package {} is missing from embedded publish plan",
                expected.name
            ));
            continue;
        };
        if actual.order != expected.order {
            errors.push(format!(
                "publish plan package {} has order {}, expected {}",
                actual.name, actual.order, expected.order
            ));
        }
        if actual.level != expected.level {
            errors.push(format!(
                "publish plan package {} has level {}, expected {}",
                actual.name, actual.level, expected.level
            ));
        }
        if actual.name != expected.name {
            errors.push(format!(
                "publish plan package at index {index} is {}, expected {}",
                actual.name, expected.name
            ));
        }
        if actual.version != expected.version {
            errors.push(format!(
                "publish plan package {} has version {}, expected {}",
                actual.name, actual.version, expected.version
            ));
        }
        if actual.manifest_path != expected.manifest_path {
            errors.push(format!(
                "publish plan package {} has manifest path {}, expected {}",
                actual.name, actual.manifest_path, expected.manifest_path
            ));
        }
        if actual.internal_dependencies != expected.internal_dependencies {
            errors.push(format!(
                "publish plan package {} has internal dependencies {:?}, expected {:?}",
                actual.name, actual.internal_dependencies, expected.internal_dependencies
            ));
        }
    }

    for actual in embedded.packages.iter().skip(external.packages.len()) {
        errors.push(format!(
            "embedded publish plan package {} is not present in external publish plan",
            actual.name
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; ").into())
    }
}

fn validate_crate_package_report_shape(
    report: &CratePackageReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("crate package report generator", &report.generator)?;
    validate_release_action_text("crate package report status", &report.status)?;
    validate_publish_plan_shape(&report.publish_plan)?;
    if report.packages.is_empty() {
        return Err("crate package report has no packages".into());
    }
    if report.packages.len() > CRATE_PACKAGE_MAX_PACKAGES {
        return Err(format!(
            "crate package report has too many packages: {} exceeds maximum {CRATE_PACKAGE_MAX_PACKAGES}",
            report.packages.len()
        )
        .into());
    }

    for package in &report.packages {
        validate_release_action_text("crate package name", &package.name)?;
        validate_release_action_text(
            &format!("crate package `{}` version", package.name),
            &package.version,
        )?;
        validate_release_action_text(
            &format!("crate package `{}` manifest path", package.name),
            &package.manifest_path,
        )?;
        validate_release_action_text(
            &format!("crate package `{}` status", package.name),
            &package.status,
        )?;
        if let Some(reason) = &package.reason {
            validate_release_action_text(
                &format!("crate package `{}` reason", package.name),
                reason,
            )?;
        }
        if package.internal_dependencies.len() > CRATE_PACKAGE_MAX_DEPENDENCIES {
            return Err(format!(
                "crate package `{}` has too many internal dependencies: {} exceeds maximum {CRATE_PACKAGE_MAX_DEPENDENCIES}",
                package.name,
                package.internal_dependencies.len()
            )
            .into());
        }
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &package.internal_dependencies {
            validate_release_action_text(
                &format!("crate package `{}` internal dependency", package.name),
                dependency,
            )?;
            if !seen_dependencies.insert(dependency.as_str()) {
                return Err(format!(
                    "crate package `{}` has duplicate internal dependency `{dependency}`",
                    package.name
                )
                .into());
            }
        }
    }
    Ok(())
}

fn print_crate_package_report(
    report: &CratePackageReport,
    evidence: &CratePackageEvidence,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_crate_package_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!(
                "crate package readiness: {} ({} packageable now, {} deferred)",
                report.status, evidence.packaged_count, evidence.deferred_count
            );
            for package in &report.packages {
                let deps = if package.internal_dependencies.is_empty() {
                    "-".to_string()
                } else {
                    package.internal_dependencies.join(", ")
                };
                let reason = package
                    .reason
                    .as_deref()
                    .map(|reason| format!("; {reason}"))
                    .unwrap_or_default();
                println!(
                    "{:>2}. [{}] {} {} (deps: {}){}",
                    package.publish_order,
                    package.status,
                    package.name,
                    package.version,
                    deps,
                    reason
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

fn run_npm_pack(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if check {
        let Some(report_path) = out else {
            return Err("`vesty npm-pack --check` requires `--out <report>`".into());
        };
        let entries = read_npm_pack_report(report_path)?;
        let evidence = validate_npm_pack_entries(&entries)?;
        print_npm_pack_report(&entries, &evidence, format, Some(report_path))?;
        return Ok(());
    }

    let raw_report = npm_pack_dry_run_json(workspace)?;
    let entries = parse_npm_pack_command_output(&raw_report)?;
    let evidence = validate_npm_pack_entries(&entries)?;
    let report_text = serde_json::to_string_pretty(&entries)? + "\n";
    if let Some(path) = out {
        write_text_file(path, &report_text)?;
    }
    print_npm_pack_report(&entries, &evidence, format, out)
}

const DEPENDENCY_BASELINE_REPORT_VERSION: u32 = 1;
const DEPENDENCY_BASELINE_REPORT_GENERATOR: &str = "vesty-cli.dependency-baseline.v1";
const DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME: &str =
    "cargo workspace external dependency baseline coverage";
const DEPENDENCY_BASELINE_MAX_CHECKS: usize = 256;
const DEPENDENCY_BASELINE_HINT_MAX_BYTES: usize = 64 * 1024;
const TYPESCRIPT_BASELINE_RANGE: &str = "^7.0.2";
const TYPESCRIPT_BASELINE_LOCK_VERSION: &str = "7.0.2";
const REQUIRED_JS_BASELINE_PACKAGES: &[&str] = &["plugin-ui"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct JsLatestBaselineDependency {
    workspace_package: &'static str,
    dependency: &'static str,
    node_package_path: &'static str,
    expected_range: &'static str,
    expected_lock_version: &'static str,
}

const REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES: &[JsLatestBaselineDependency] = &[
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "react",
        node_package_path: "node_modules/react",
        expected_range: "latest",
        expected_lock_version: "19.2.7",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "@types/react",
        node_package_path: "node_modules/@types/react",
        expected_range: "latest",
        expected_lock_version: "19.2.17",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "vue",
        node_package_path: "node_modules/vue",
        expected_range: "latest",
        expected_lock_version: "3.5.39",
    },
    JsLatestBaselineDependency {
        workspace_package: "plugin-ui",
        dependency: "svelte",
        node_package_path: "node_modules/svelte",
        expected_range: "latest",
        expected_lock_version: "5.56.5",
    },
];
const REQUIRED_RUST_BASELINE_DEPENDENCIES: &[(&str, &str)] = &[
    ("arc-swap", "1.9.2"),
    ("atomic_float", "1.1.0"),
    ("camino", "1.2.4"),
    ("cargo_metadata", "0.23.1"),
    ("wry", "0.55.1"),
    ("vst3", vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE),
    ("raw-window-handle", "0.6.2"),
    ("rtrb", "0.3.4"),
    ("serde", "1.0.228"),
    ("serde_json", "1.0.150"),
    ("ts-rs", "12.0.1"),
    ("clap", "4.6.2"),
    ("mime_guess", "2.0.5"),
    ("plist", "1.10.0"),
    ("proc-macro-crate", "3.5.0"),
    ("proc-macro2", "1.0.106"),
    ("quote", "1.0.46"),
    ("schemars", "1.2.1"),
    ("syn", "2.0.119"),
    ("toml", "1.1.3"),
    ("sha2", "0.11.0"),
    ("tempfile", "3.27.0"),
    ("thiserror", "2.0.18"),
    ("tracing", "0.1.44"),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DependencyBaselineReport {
    version: u32,
    generator: String,
    status: String,
    checks: Vec<DependencyBaselineCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DependencyBaselineCheck {
    name: String,
    kind: String,
    path: String,
    expected: String,
    actual: Option<String>,
    status: String,
    hint: Option<String>,
}

fn run_dependency_baseline(
    workspace: &Utf8Path,
    out: Option<&Utf8Path>,
    check: bool,
    latest: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let current = if latest {
        dependency_baseline_report_with_latest(workspace, &CommandLatestDependencyFetcher)?
    } else {
        dependency_baseline_report(workspace)?
    };

    if check {
        let Some(report_path) = out else {
            return Err("`vesty dependency-baseline --check` requires `--out <report>`".into());
        };
        let expected = read_dependency_baseline_report(report_path)?;
        validate_dependency_baseline_report(&expected)?;
        if expected != current {
            return Err(
                "dependency baseline report drift; regenerate with `vesty dependency-baseline --out <report>`"
                    .into(),
            );
        }
        print_dependency_baseline_report(&current, format, Some(report_path))?;
        return Ok(());
    }

    if let Some(path) = out {
        write_dependency_baseline_report(path, &current)?;
    }
    print_dependency_baseline_report(&current, format, out)?;
    validate_dependency_baseline_report(&current)
}

fn dependency_baseline_report(
    workspace: &Utf8Path,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    dependency_baseline_report_with_optional_latest(workspace, None)
}

fn dependency_baseline_report_with_latest(
    workspace: &Utf8Path,
    latest: &dyn LatestDependencyFetcher,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    dependency_baseline_report_with_optional_latest(workspace, Some(latest))
}

fn dependency_baseline_report_with_optional_latest(
    workspace: &Utf8Path,
    latest: Option<&dyn LatestDependencyFetcher>,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    let mut checks = Vec::new();
    let cargo_manifest = read_toml_file(&workspace.join("Cargo.toml"))?;
    checks.push(workspace_dependency_baseline_coverage_check(
        &cargo_manifest,
    ));
    for (name, expected) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        checks.push(dependency_baseline_check(
            &format!("cargo workspace dependency `{name}`"),
            "cargo",
            "Cargo.toml",
            expected,
            workspace_dependency_version(&cargo_manifest, name),
            Some("update the workspace dependency baseline only after registry/latest-deps review"),
        ));
    }
    checks.push(dependency_baseline_check(
        "Steinberg VST3 SDK baseline",
        "vst3-sdk",
        "crates/vesty-vst3-sys/src/lib.rs",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE,
        Some(vesty_vst3_sys::BINDING_BASELINE.steinberg_sdk.to_string()),
        Some("Steinberg SDK remains the VST3 source of truth; keep generated-header audit docs in sync"),
    ));
    checks.push(dependency_baseline_check(
        "upstream `vst3` crate binding baseline",
        "vst3-sdk",
        "crates/vesty-vst3-sys/src/lib.rs",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE,
        Some(vesty_vst3_sys::BINDING_BASELINE.upstream_vst3_crate.to_string()),
        Some("if upstream VST3 bindings change, update vesty-vst3-sys baseline and adapter tests together"),
    ));

    for package in REQUIRED_JS_BASELINE_PACKAGES {
        let path = format!("packages/{package}/package.json");
        let package_json = read_json_file(&workspace.join(&path))?;
        checks.push(dependency_baseline_check(
            "npm package `vesty-plugin-ui` TypeScript devDependency",
            "npm",
            &path,
            TYPESCRIPT_BASELINE_RANGE,
            json_dependency_version(&package_json, "devDependencies", "typescript"),
            Some(
                "run `npm install --workspaces typescript@latest --save-dev` after registry review",
            ),
        ));
    }

    let lock_path = "package-lock.json";
    let package_lock = read_json_file(&workspace.join(lock_path))?;
    checks.push(dependency_baseline_check(
        "npm lockfile `typescript` installed version",
        "npm-lock",
        lock_path,
        TYPESCRIPT_BASELINE_LOCK_VERSION,
        package_lock_node_package_version(&package_lock, "node_modules/typescript"),
        Some("run `npm install` after changing JS package dependency ranges"),
    ));
    for package in REQUIRED_JS_BASELINE_PACKAGES {
        checks.push(dependency_baseline_check(
            "npm lockfile `vesty-plugin-ui` TypeScript devDependency",
            "npm-lock",
            lock_path,
            TYPESCRIPT_BASELINE_RANGE,
            package_lock_workspace_dev_dependency(
                &package_lock,
                &format!("packages/{package}"),
                "typescript",
            ),
            Some("package-lock workspace entries must match package.json dependency ranges"),
        ));
    }
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        let package_path = format!("packages/{}", dependency.workspace_package);
        let package_json_path = format!("{package_path}/package.json");
        let package_json = read_json_file(&workspace.join(&package_json_path))?;
        checks.push(dependency_baseline_check(
            &format!(
                "npm package `vesty-plugin-ui` {} devDependency range",
                dependency.dependency
            ),
            "npm",
            &package_json_path,
            dependency.expected_range,
            json_dependency_version(&package_json, "devDependencies", dependency.dependency),
            Some("framework adapter devDependencies should stay on `latest` until a compatibility floor is intentionally pinned"),
        ));
        checks.push(dependency_baseline_check(
            &format!("npm lockfile `{}` installed version", dependency.dependency),
            "npm-lock",
            lock_path,
            dependency.expected_lock_version,
            package_lock_node_package_version(&package_lock, dependency.node_package_path),
            Some("run `npm install --workspaces` after reviewing the latest framework adapter dependency"),
        ));
    }

    if let Some(latest) = latest {
        append_dependency_latest_checks(&mut checks, latest);
    }

    let complete = checks.iter().all(|check| check.status == "ok");
    Ok(DependencyBaselineReport {
        version: DEPENDENCY_BASELINE_REPORT_VERSION,
        generator: DEPENDENCY_BASELINE_REPORT_GENERATOR.to_string(),
        status: if complete { "ok" } else { "failed" }.to_string(),
        checks,
    })
}

trait LatestDependencyFetcher {
    fn latest_crate_version(&self, name: &str) -> Result<String, String>;
    fn latest_npm_version(&self, name: &str) -> Result<String, String>;
}

struct CommandLatestDependencyFetcher;

impl LatestDependencyFetcher for CommandLatestDependencyFetcher {
    fn latest_crate_version(&self, name: &str) -> Result<String, String> {
        latest_crate_version_from_crates_io(name)
    }

    fn latest_npm_version(&self, name: &str) -> Result<String, String> {
        latest_npm_version_from_npm_view(name)
    }
}

fn append_dependency_latest_checks(
    checks: &mut Vec<DependencyBaselineCheck>,
    latest: &dyn LatestDependencyFetcher,
) {
    for (name, expected) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        let registry_expected = rust_registry_latest_expected(name, expected);
        checks.push(dependency_latest_check(
            &format!("crates.io latest `{name}`"),
            "cargo-registry",
            "crates.io",
            &registry_expected,
            latest.latest_crate_version(name),
            "review crates.io release notes, then update workspace dependency and baseline together",
        ));
    }
    checks.push(dependency_latest_check(
        "npm registry latest `typescript`",
        "npm-registry",
        "registry.npmjs.org",
        TYPESCRIPT_BASELINE_LOCK_VERSION,
        latest.latest_npm_version("typescript"),
        "review TypeScript release notes, then run `npm install --workspaces typescript@latest --save-dev`",
    ));
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        checks.push(dependency_latest_check(
            &format!("npm registry latest `{}`", dependency.dependency),
            "npm-registry",
            "registry.npmjs.org",
            dependency.expected_lock_version,
            latest.latest_npm_version(dependency.dependency),
            "review framework adapter dependency release notes, then run `npm install --workspaces`",
        ));
    }
}

fn dependency_baseline_check_key(kind: &str, name: &str) -> String {
    format!("{}:{}", kind.trim(), name.trim())
}

fn expected_dependency_baseline_check_keys(include_latest: bool) -> BTreeSet<String> {
    let mut expected = BTreeSet::new();
    expected.insert(dependency_baseline_check_key(
        "cargo-baseline",
        DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME,
    ));
    for (name, _) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
        expected.insert(dependency_baseline_check_key(
            "cargo",
            &format!("cargo workspace dependency `{name}`"),
        ));
    }
    expected.insert(dependency_baseline_check_key(
        "vst3-sdk",
        "Steinberg VST3 SDK baseline",
    ));
    expected.insert(dependency_baseline_check_key(
        "vst3-sdk",
        "upstream `vst3` crate binding baseline",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm",
        "npm package `vesty-plugin-ui` TypeScript devDependency",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm-lock",
        "npm lockfile `typescript` installed version",
    ));
    expected.insert(dependency_baseline_check_key(
        "npm-lock",
        "npm lockfile `vesty-plugin-ui` TypeScript devDependency",
    ));
    for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
        expected.insert(dependency_baseline_check_key(
            "npm",
            &format!(
                "npm package `vesty-plugin-ui` {} devDependency range",
                dependency.dependency
            ),
        ));
        expected.insert(dependency_baseline_check_key(
            "npm-lock",
            &format!("npm lockfile `{}` installed version", dependency.dependency),
        ));
    }

    if include_latest {
        for (name, _) in REQUIRED_RUST_BASELINE_DEPENDENCIES {
            expected.insert(dependency_baseline_check_key(
                "cargo-registry",
                &format!("crates.io latest `{name}`"),
            ));
        }
        expected.insert(dependency_baseline_check_key(
            "npm-registry",
            "npm registry latest `typescript`",
        ));
        for dependency in REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES {
            expected.insert(dependency_baseline_check_key(
                "npm-registry",
                &format!("npm registry latest `{}`", dependency.dependency),
            ));
        }
    }

    expected
}

fn dependency_baseline_check(
    name: &str,
    kind: &str,
    path: &str,
    expected: &str,
    actual: Option<String>,
    hint: Option<&str>,
) -> DependencyBaselineCheck {
    let status = if actual.as_deref() == Some(expected) {
        "ok"
    } else {
        "failed"
    };
    DependencyBaselineCheck {
        name: name.to_string(),
        kind: kind.to_string(),
        path: path.to_string(),
        expected: expected.to_string(),
        actual,
        status: status.to_string(),
        hint: (status != "ok").then(|| {
            hint.unwrap_or("restore the dependency baseline")
                .to_string()
        }),
    }
}

fn rust_registry_latest_expected(name: &str, manifest_expected: &str) -> String {
    match name {
        // Cargo dependency requirements ignore SemVer build metadata, but crates.io
        // exposes toml's current release with the spec marker in the version string.
        "toml" => "1.1.3+spec-1.1.0".to_string(),
        _ => manifest_expected.to_string(),
    }
}

fn workspace_dependency_baseline_coverage_check(manifest: &toml::Value) -> DependencyBaselineCheck {
    let expected = REQUIRED_RUST_BASELINE_DEPENDENCIES
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect::<BTreeSet<_>>();
    let actual = workspace_external_dependency_names(manifest);
    dependency_baseline_check(
        DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME,
        "cargo-baseline",
        "Cargo.toml",
        &format_dependency_name_set(&expected),
        Some(format_dependency_name_set(&actual)),
        Some(
            "add every external [workspace.dependencies] entry to REQUIRED_RUST_BASELINE_DEPENDENCIES after latest-deps review",
        ),
    )
}

fn workspace_external_dependency_names(manifest: &toml::Value) -> BTreeSet<String> {
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter(|(_, dependency)| !workspace_dependency_is_internal_path(dependency))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn workspace_dependency_is_internal_path(dependency: &toml::Value) -> bool {
    dependency
        .as_table()
        .is_some_and(|table| table.contains_key("path"))
}

fn format_dependency_name_set(names: &BTreeSet<String>) -> String {
    names.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn dependency_latest_check(
    name: &str,
    kind: &str,
    path: &str,
    expected: &str,
    actual: Result<String, String>,
    hint: &str,
) -> DependencyBaselineCheck {
    match actual {
        Ok(actual) => {
            dependency_baseline_check(name, kind, path, expected, Some(actual), Some(hint))
        }
        Err(error) => DependencyBaselineCheck {
            name: name.to_string(),
            kind: kind.to_string(),
            path: path.to_string(),
            expected: expected.to_string(),
            actual: None,
            status: "failed".to_string(),
            hint: Some(format!("{hint}; latest registry query failed: {error}")),
        },
    }
}

fn latest_crate_version_from_cargo_search(name: &str) -> Result<String, String> {
    let search_result = latest_crate_version_from_cargo_search_only(name);
    if search_result.is_ok() {
        return search_result;
    }
    let search_error = search_result
        .err()
        .unwrap_or_else(|| "cargo search failed".to_string());
    latest_crate_version_from_cargo_info(name)
        .map_err(|info_error| format!("{search_error}; cargo info fallback failed: {info_error}"))
}

fn latest_crate_version_from_crates_io(name: &str) -> Result<String, String> {
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(format!("invalid crates.io crate name `{name}`"));
    }

    let url = format!("https://crates.io/api/v1/crates/{name}");
    let user_agent = format!(
        "User-Agent: vesty-cli/{} (https://github.com/backrunner/vesty)",
        env!("CARGO_PKG_VERSION")
    );
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--retry-delay",
            "1",
            "--retry-all-errors",
            "--max-time",
            "30",
            "--header",
            &user_agent,
            &url,
        ])
        .output();

    let api_result = match output {
        Ok(output) if output.status.success() => {
            let body = String::from_utf8_lossy(&output.stdout);
            parse_crates_io_latest_version(&body)
                .ok_or_else(|| format!("crates.io response did not include a version for `{name}`"))
        }
        Ok(output) => Err(format!(
            "crates.io API failed with status {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => Err(format!("failed to run curl for crates.io API: {error}")),
    };

    if api_result.is_ok() {
        return api_result;
    }
    let api_error = api_result
        .err()
        .unwrap_or_else(|| "crates.io API query failed".to_string());
    latest_crate_version_from_cargo_search(name)
        .map_err(|cargo_error| format!("{api_error}; cargo fallback failed: {cargo_error}"))
}

fn parse_crates_io_latest_version(body: &str) -> Option<String> {
    let response: serde_json::Value = serde_json::from_str(body).ok()?;
    let crate_data = response.get("crate")?;
    crate_data
        .get("max_stable_version")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            crate_data
                .get("max_version")
                .and_then(serde_json::Value::as_str)
        })
        .filter(|version| !version.is_empty())
        .map(str::to_string)
}

fn latest_crate_version_from_cargo_search_only(name: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["search", name, "--limit", "1"])
        .output()
        .map_err(|error| format!("failed to run cargo search: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "cargo search failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    parse_cargo_search_latest_version(name, &stdout)
        .ok_or_else(|| format!("cargo search did not return an exact `{name}` version line"))
}

fn parse_cargo_search_latest_version(name: &str, stdout: &str) -> Option<String> {
    let prefix = format!("{name} = \"");
    stdout.lines().map(str::trim).find_map(|line| {
        line.strip_prefix(&prefix)?
            .split('"')
            .next()
            .map(str::to_string)
    })
}

fn latest_crate_version_from_cargo_info(name: &str) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["info", name])
        .output()
        .map_err(|error| format!("failed to run cargo info: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "cargo info failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    parse_cargo_info_version(&stdout)
        .ok_or_else(|| format!("cargo info did not return a `version:` line for `{name}`"))
}

fn parse_cargo_info_version(stdout: &str) -> Option<String> {
    stdout.lines().map(str::trim).find_map(|line| {
        line.strip_prefix("version:")
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .map(str::to_string)
    })
}

fn latest_npm_version_from_npm_view(name: &str) -> Result<String, String> {
    let output = Command::new("npm")
        .args(["view", name, "version"])
        .output()
        .map_err(|error| format!("failed to run npm view: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "npm view failed with status {}; stderr: {}",
            output.status,
            stderr.trim()
        ));
    }
    let version = stdout.trim();
    if version.is_empty() {
        Err(format!("npm view did not return a version for `{name}`"))
    } else {
        Ok(version.to_string())
    }
}

fn read_toml_file(path: &Utf8Path) -> Result<toml::Value, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("TOML input", path)?;
    toml::from_str::<toml::Value>(&text)
        .map_err(|error| format!("invalid TOML in {path}: {error}").into())
}

fn read_json_file(path: &Utf8Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("JSON input", path)?;
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("invalid JSON in {path}: {error}").into())
}

fn workspace_dependency_version(manifest: &toml::Value, name: &str) -> Option<String> {
    let dependency = manifest.get("workspace")?.get("dependencies")?.get(name)?;
    match dependency {
        toml::Value::String(version) => Some(version.clone()),
        toml::Value::Table(table) => table
            .get("version")
            .and_then(toml::Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn json_dependency_version(
    package_json: &serde_json::Value,
    section: &str,
    name: &str,
) -> Option<String> {
    package_json
        .get(section)?
        .get(name)?
        .as_str()
        .map(str::to_string)
}

fn package_lock_node_package_version(
    package_lock: &serde_json::Value,
    package_path: &str,
) -> Option<String> {
    package_lock
        .get("packages")?
        .get(package_path)?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

fn package_lock_workspace_dev_dependency(
    package_lock: &serde_json::Value,
    workspace_path: &str,
    name: &str,
) -> Option<String> {
    package_lock
        .get("packages")?
        .get(workspace_path)?
        .get("devDependencies")?
        .get(name)?
        .as_str()
        .map(str::to_string)
}

fn validate_dependency_baseline_report(
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;

    if report.version != DEPENDENCY_BASELINE_REPORT_VERSION {
        return Err(format!(
            "unsupported dependency baseline report version {}; expected {}",
            report.version, DEPENDENCY_BASELINE_REPORT_VERSION
        )
        .into());
    }
    if report.generator != DEPENDENCY_BASELINE_REPORT_GENERATOR {
        return Err(format!(
            "unsupported dependency baseline report generator {}; expected {}",
            report.generator, DEPENDENCY_BASELINE_REPORT_GENERATOR
        )
        .into());
    }
    if !matches!(report.status.as_str(), "ok" | "failed") {
        return Err(format!(
            "unsupported dependency baseline report status `{}`",
            report.status
        )
        .into());
    }

    let mut failed = Vec::new();
    for check in &report.checks {
        if !matches!(check.status.as_str(), "ok" | "failed") {
            return Err(format!(
                "unsupported dependency baseline check status `{}` for {}",
                check.status, check.name
            )
            .into());
        }

        let matches_expected = check.actual.as_deref() == Some(check.expected.as_str());
        match (check.status.as_str(), matches_expected) {
            ("ok", false) => {
                return Err(format!(
                    "dependency baseline check `{}` status ok is inconsistent with expected {}, actual {}",
                    check.name,
                    check.expected,
                    check.actual.as_deref().unwrap_or("<missing>")
                )
                .into());
            }
            ("failed", true) => {
                return Err(format!(
                    "dependency baseline check `{}` status failed is inconsistent with matching expected/actual {}",
                    check.name, check.expected
                )
                .into());
            }
            ("failed", false) => failed.push(format_dependency_baseline_failure(check)),
            ("ok", true) => {}
            _ => unreachable!("dependency baseline check status validated"),
        }
    }

    let expected_report_status = if failed.is_empty() { "ok" } else { "failed" };
    if report.status != expected_report_status {
        return Err(format!(
            "dependency baseline report status `{}` is inconsistent with {} failed check(s)",
            report.status,
            failed.len()
        )
        .into());
    }

    if !failed.is_empty() {
        return Err(format!("dependency baseline check failed: {}", failed.join("; ")).into());
    }
    Ok(())
}

fn validate_dependency_baseline_report_shape(
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("dependency baseline report generator", &report.generator)?;
    validate_release_action_text("dependency baseline report status", &report.status)?;
    if report.checks.is_empty() {
        return Err("dependency baseline report has no checks".into());
    }
    if report.checks.len() > DEPENDENCY_BASELINE_MAX_CHECKS {
        return Err(format!(
            "dependency baseline report has too many checks: {} exceeds maximum {DEPENDENCY_BASELINE_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("dependency baseline check name", &check.name)?;
        validate_release_action_text(
            &format!("dependency baseline `{}` kind", check.name),
            &check.kind,
        )?;
        validate_release_action_text(
            &format!("dependency baseline `{}` path", check.name),
            &check.path,
        )?;
        validate_release_action_text(
            &format!("dependency baseline `{}` expected", check.name),
            &check.expected,
        )?;
        if let Some(actual) = &check.actual {
            validate_release_action_text(
                &format!("dependency baseline `{}` actual", check.name),
                actual,
            )?;
        }
        validate_release_action_text(
            &format!("dependency baseline `{}` status", check.name),
            &check.status,
        )?;
        if let Some(hint) = &check.hint {
            validate_dependency_baseline_hint_text(
                &format!("dependency baseline `{}` hint", check.name),
                hint,
            )?;
        }
        let key = dependency_baseline_check_key(&check.kind, &check.name);
        if !seen_checks.insert(key.clone()) {
            return Err(format!("duplicate dependency baseline check `{key}`").into());
        }
    }
    let include_latest = seen_checks
        .iter()
        .any(|key| key.starts_with("cargo-registry:") || key.starts_with("npm-registry:"));
    let expected_checks = expected_dependency_baseline_check_keys(include_latest);
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown dependency baseline check(s): {}",
            unknown_checks.join(", ")
        )
        .into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "dependency baseline report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

fn validate_dependency_baseline_hint_text(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > DEPENDENCY_BASELINE_HINT_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {DEPENDENCY_BASELINE_HINT_MAX_BYTES} bytes"
        ));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(format!(
            "{label} must not contain control characters other than tab/newline"
        ));
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!(
            "{label} must not contain unsafe Unicode format characters"
        ));
    }
    Ok(())
}

fn format_dependency_baseline_failure(check: &DependencyBaselineCheck) -> String {
    format!(
        "{} expected {}, actual {}",
        check.name,
        check.expected,
        check.actual.as_deref().unwrap_or("<missing>")
    )
}

fn write_dependency_baseline_report(
    path: &Utf8Path,
    report: &DependencyBaselineReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;
    write_text_file(path, &(serde_json::to_string_pretty(report)? + "\n"))
}

fn read_dependency_baseline_report(
    path: &Utf8Path,
) -> Result<DependencyBaselineReport, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("dependency baseline report", path)?;
    serde_json::from_str::<DependencyBaselineReport>(&text)
        .map_err(|error| format!("invalid dependency baseline report JSON: {error}").into())
}

fn print_dependency_baseline_report(
    report: &DependencyBaselineReport,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_dependency_baseline_report_shape(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("dependency baseline: {}", report.status);
            for check in &report.checks {
                println!(
                    "- [{}] {}: expected {}, actual {}",
                    check.status,
                    check.name,
                    check.expected,
                    check.actual.as_deref().unwrap_or("<missing>")
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

fn print_npm_pack_report(
    entries: &[NpmPackEntry],
    evidence: &NpmPackEvidence,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_npm_pack_entries_shape(entries)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(entries)?);
        }
        OutputFormat::Text => {
            println!(
                "npm pack dry-run: {} package(s), {} file(s)",
                evidence.package_count, evidence.total_files
            );
            println!("packages: {}", evidence.packages.join(", "));
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

fn npm_pack_dry_run_json(workspace: &Utf8Path) -> Result<String, Box<dyn std::error::Error>> {
    if !workspace.is_dir() {
        return Err(format!("npm workspace directory does not exist: {workspace}").into());
    }
    let output = Command::new("npm")
        .args(["pack", "--workspaces", "--dry-run", "--json"])
        .current_dir(workspace)
        .output()
        .map_err(|error| format!("failed to run npm pack in {workspace}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "npm pack dry-run failed in {workspace}: status {}; stdout: {}; stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        )
        .into());
    }
    Ok(stdout.into_owned())
}

fn print_publish_plan(
    plan: &PublishPlan,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(plan)?);
        }
        OutputFormat::Text => {
            println!("publishable workspace crates: {}", plan.packages.len());
            for package in &plan.packages {
                let deps = if package.internal_dependencies.is_empty() {
                    "-".to_string()
                } else {
                    package.internal_dependencies.join(", ")
                };
                println!(
                    "{:>2}. {} {} (level {}, deps: {})",
                    package.order, package.name, package.version, package.level, deps
                );
            }
            if !plan.skipped_private.is_empty() {
                println!(
                    "private workspace packages skipped: {}",
                    plan.skipped_private.join(", ")
                );
            }
            if let Some(path) = out {
                println!("report: {path}");
            }
        }
    }
    Ok(())
}

fn write_publish_plan_report(
    path: &Utf8Path,
    plan: &PublishPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;
    let text = serde_json::to_string_pretty(plan)? + "\n";
    write_text_file(path, &text)
}

fn read_publish_plan_report(path: &Utf8Path) -> Result<PublishPlan, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("publish plan report", path)?;
    serde_json::from_str::<PublishPlan>(&text)
        .map_err(|error| format!("invalid publish plan JSON: {error}").into())
}

fn workspace_publish_plan(
    metadata: &serde_json::Value,
) -> Result<PublishPlan, Box<dyn std::error::Error>> {
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not report packages")?;
    let workspace_member_ids = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata did not report workspace_members")?;

    let mut package_by_id = BTreeMap::<String, &serde_json::Value>::new();
    for package in packages {
        let id = package_string_field(package, "id")?.to_string();
        package_by_id.insert(id, package);
    }

    let mut workspace_packages = Vec::new();
    for member in workspace_member_ids {
        let member_id = member
            .as_str()
            .ok_or("cargo metadata workspace member id is not a string")?;
        let package = package_by_id.get(member_id).ok_or_else(|| {
            format!("workspace member package is missing from metadata: {member_id}")
        })?;
        workspace_packages.push(*package);
    }

    let mut workspace_index = BTreeMap::<String, usize>::new();
    let mut publishable_names = BTreeSet::<String>::new();
    let mut private_names = BTreeSet::<String>::new();
    let mut skipped_private = Vec::new();

    for (index, package) in workspace_packages.iter().enumerate() {
        let name = package_string_field(package, "name")?.to_string();
        workspace_index.insert(name.clone(), index);
        if package_is_publishable(package) {
            publishable_names.insert(name);
        } else {
            private_names.insert(name.clone());
            skipped_private.push(name);
        }
    }

    let mut dependencies = BTreeMap::<String, Vec<String>>::new();
    let mut blockers = Vec::new();
    for package in &workspace_packages {
        let name = package_string_field(package, "name")?.to_string();
        if !publishable_names.contains(&name) {
            continue;
        }
        let mut internal = Vec::new();
        for dependency in package
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("cargo metadata package did not report dependencies: {name}"))?
        {
            if dependency.get("kind").and_then(serde_json::Value::as_str) == Some("dev") {
                continue;
            }
            let dependency_name = dependency_string_field(dependency, "name")?.to_string();
            if publishable_names.contains(&dependency_name) {
                internal.push(dependency_name);
            } else if private_names.contains(&dependency_name) {
                blockers.push(format!(
                    "{name} depends on private workspace package {dependency_name}"
                ));
            }
        }
        internal.sort_by_key(|name| workspace_index.get(name).copied().unwrap_or(usize::MAX));
        internal.dedup();
        dependencies.insert(name, internal);
    }

    if !blockers.is_empty() {
        return Err(format!(
            "publish plan has private workspace dependency blockers: {}",
            blockers.join("; ")
        )
        .into());
    }

    let mut levels = BTreeMap::<String, usize>::new();
    let mut visiting = BTreeSet::<String>::new();
    for name in &publishable_names {
        publish_level(name, &dependencies, &mut levels, &mut visiting)?;
    }

    let mut package_rows = Vec::new();
    for package in workspace_packages {
        let name = package_string_field(package, "name")?.to_string();
        if !publishable_names.contains(&name) {
            continue;
        }
        let level = *levels
            .get(&name)
            .ok_or_else(|| format!("publish level missing for package {name}"))?;
        package_rows.push(PublishPlanPackage {
            order: 0,
            level,
            name: name.clone(),
            version: package_string_field(package, "version")?.to_string(),
            manifest_path: package_string_field(package, "manifest_path")?.to_string(),
            internal_dependencies: dependencies.get(&name).cloned().unwrap_or_default(),
        });
    }
    package_rows.sort_by_key(|package| {
        (
            package.level,
            workspace_index
                .get(&package.name)
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
    for (index, package) in package_rows.iter_mut().enumerate() {
        package.order = index + 1;
    }

    Ok(PublishPlan {
        packages: package_rows,
        skipped_private,
    })
}

fn publish_level(
    name: &str,
    dependencies: &BTreeMap<String, Vec<String>>,
    levels: &mut BTreeMap<String, usize>,
    visiting: &mut BTreeSet<String>,
) -> Result<usize, Box<dyn std::error::Error>> {
    if let Some(level) = levels.get(name) {
        return Ok(*level);
    }
    if !visiting.insert(name.to_string()) {
        return Err(format!("workspace publish dependency cycle includes {name}").into());
    }

    let mut level = 1;
    for dependency in dependencies
        .get(name)
        .ok_or_else(|| format!("publish dependencies missing for package {name}"))?
    {
        level = level.max(publish_level(dependency, dependencies, levels, visiting)? + 1);
    }
    visiting.remove(name);
    levels.insert(name.to_string(), level);
    Ok(level)
}

fn package_is_publishable(package: &serde_json::Value) -> bool {
    !matches!(
        package.get("publish").and_then(serde_json::Value::as_array),
        Some(registries) if registries.is_empty()
    )
}

fn package_string_field<'a>(
    package: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    package
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("cargo metadata package field is not a string: {field}").into())
}

fn dependency_string_field<'a>(
    dependency: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    dependency
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("cargo metadata dependency field is not a string: {field}").into())
}

fn parse_bundle_platform(platform: &str) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    match platform.trim().to_ascii_lowercase().as_str() {
        "macos" | "darwin" => Ok(BundlePlatform::Macos),
        "windows-x64" | "windows" | "win64" => Ok(BundlePlatform::WindowsX64),
        "linux-x64" | "linux" => Ok(BundlePlatform::LinuxX64),
        _ => Err(format!("unsupported platform '{platform}'").into()),
    }
}

fn infer_signing_bundle_platform(
    bundle: &Utf8Path,
) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    if bundle.extension() != Some("vst3") {
        return Err(format!("signing evidence source must be a .vst3 directory: {bundle}").into());
    }
    require_existing_directory_no_symlink("signing evidence source", bundle)?;

    let has_macos = existing_directory_no_symlink(
        "macOS signing payload directory",
        &bundle.join("Contents/MacOS"),
    )? || existing_directory_no_symlink(
        "macOS signing payload directory",
        &bundle.join("Contents/_CodeSignature"),
    )?;
    let has_windows = existing_directory_no_symlink(
        "Windows signing payload directory",
        &bundle.join("Contents/x86_64-win"),
    )?;
    match (has_macos, has_windows) {
        (true, false) => Ok(BundlePlatform::Macos),
        (false, true) => Ok(BundlePlatform::WindowsX64),
        (true, true) => Err(
            "bundle contains both macOS and Windows payloads; pass --platform explicitly".into(),
        ),
        (false, false) => match current_bundle_platform() {
            Some(BundlePlatform::Macos) => Ok(BundlePlatform::Macos),
            Some(BundlePlatform::WindowsX64) => Ok(BundlePlatform::WindowsX64),
            _ => Err(
                "could not infer signing platform from bundle contents; pass --platform macos or windows-x64"
                    .into(),
            ),
        },
    }
}

fn signing_platform_for_bundle_platform(
    platform: BundlePlatform,
) -> Result<SigningEvidencePlatform, Box<dyn std::error::Error>> {
    match platform {
        BundlePlatform::Macos => Ok(SigningEvidencePlatform::Macos),
        BundlePlatform::WindowsX64 => Ok(SigningEvidencePlatform::Windows),
        BundlePlatform::LinuxX64 => Err(
            "Linux VST3 signing evidence is release-channel specific; collect distro/package signing evidence outside `vesty release-evidence collect-signing`"
                .into(),
        ),
    }
}

fn default_signing_evidence_path(dir: &Utf8Path, platform: BundlePlatform) -> Utf8PathBuf {
    match platform {
        BundlePlatform::Macos => dir.join("signing-macos.log"),
        BundlePlatform::WindowsX64 => dir.join("signing-windows.log"),
        BundlePlatform::LinuxX64 => dir.join("signing-linux.log"),
    }
}

fn current_bundle_platform() -> Option<BundlePlatform> {
    if cfg!(target_os = "macos") {
        Some(BundlePlatform::Macos)
    } else if cfg!(target_os = "windows") {
        Some(BundlePlatform::WindowsX64)
    } else if cfg!(target_os = "linux") {
        Some(BundlePlatform::LinuxX64)
    } else {
        None
    }
}

fn resolve_bundle_platform(
    platform: Option<&str>,
) -> Result<BundlePlatform, Box<dyn std::error::Error>> {
    match platform {
        Some(platform) => parse_bundle_platform(platform),
        None => current_bundle_platform()
            .ok_or_else(|| "unsupported host OS; pass --platform explicitly".into()),
    }
}

fn build_release_mode(debug: bool, release: bool) -> Result<bool, Box<dyn std::error::Error>> {
    if debug && release {
        return Err("use only one of --debug or --release".into());
    }
    Ok(!debug)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ValidateReport {
    bundle: String,
    static_check: StaticBundleCheck,
    validator: ValidatorCheck,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StaticBundleCheck {
    status: String,
    moduleinfo: Option<String>,
    binaries: Vec<String>,
    #[serde(default)]
    binary_exports: Vec<BinaryExportCheck>,
    #[serde(default)]
    parameter_manifest: Option<String>,
    asset_manifest: Option<String>,
    asset_count: usize,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ValidatorCheck {
    status: String,
    path: Option<String>,
    exit_code: Option<i32>,
    tests_passed: Option<u32>,
    tests_failed: Option<u32>,
    stdout: Option<String>,
    stderr: Option<String>,
    reason: Option<String>,
    error: Option<String>,
}

impl StaticBundleCheck {
    fn passed(report: &BundleValidationReport) -> Self {
        Self {
            status: "ok".to_string(),
            moduleinfo: Some(report.moduleinfo_path.to_string()),
            binaries: report
                .binary_paths
                .iter()
                .map(ToString::to_string)
                .collect(),
            binary_exports: report.binary_export_checks.clone(),
            parameter_manifest: report
                .parameter_manifest_path
                .as_ref()
                .map(ToString::to_string),
            asset_manifest: report.asset_manifest_path.as_ref().map(ToString::to_string),
            asset_count: report.asset_count,
            error: None,
        }
    }

    fn failed(error: impl ToString) -> Self {
        Self {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(error.to_string()),
        }
    }
}

impl ValidatorCheck {
    fn not_run(reason: impl ToString) -> Self {
        Self {
            status: "not_run".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some(reason.to_string()),
            error: None,
        }
    }

    fn skipped(reason: impl ToString) -> Self {
        Self {
            status: "skipped".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some(reason.to_string()),
            error: None,
        }
    }

    fn not_found() -> Self {
        Self {
            status: "not_found".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some("pass --validator or set VST3_VALIDATOR".to_string()),
            error: None,
        }
    }

    fn process_error(path: &Utf8Path, error: impl ToString) -> Self {
        Self {
            status: "failed".to_string(),
            path: Some(path.to_string()),
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: None,
            error: Some(error.to_string()),
        }
    }
}

fn run_validate(
    bundle: Utf8PathBuf,
    validator: Option<Utf8PathBuf>,
    static_only: bool,
    strict: bool,
    format: &str,
    report_path: Option<Utf8PathBuf>,
    validator_log_path: Option<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    if format == OutputFormat::Text {
        println!("validating bundle: {bundle}");
    }

    let bundle_report = match validate_vst3_bundle(&bundle) {
        Ok(report) => report,
        Err(error) => {
            if format == OutputFormat::Json {
                let report = ValidateReport {
                    bundle: bundle.to_string(),
                    static_check: StaticBundleCheck::failed(error.to_string()),
                    validator: ValidatorCheck::not_run("static bundle validation failed"),
                };
                write_validate_report(report_path.as_deref(), &report)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let report = ValidateReport {
                    bundle: bundle.to_string(),
                    static_check: StaticBundleCheck::failed(error.to_string()),
                    validator: ValidatorCheck::not_run("static bundle validation failed"),
                };
                write_validate_report(report_path.as_deref(), &report)?;
            }
            return Err(Box::new(error));
        }
    };

    let static_check = StaticBundleCheck::passed(&bundle_report);
    if format == OutputFormat::Text {
        println!(
            "bundle structure: ok (binaries: {}, binary export checks: {}, ui assets: {})",
            bundle_report.binary_paths.len(),
            bundle_report.binary_export_checks.len(),
            bundle_report.asset_count
        );
    }

    if static_only {
        let validator_check = ValidatorCheck::skipped("--static-only");
        let strict_error = strict
            .then(|| strict_static_bundle_check_error(&static_check))
            .flatten();
        let report = ValidateReport {
            bundle: bundle.to_string(),
            static_check,
            validator: validator_check,
        };
        print_validate_report(format, &report, report_path.as_deref())?;
        if let Some(error) = strict_error {
            return Err(error.into());
        }
        return Ok(());
    }

    let Some(validator) = discover_validator(validator) else {
        let report = ValidateReport {
            bundle: bundle.to_string(),
            static_check,
            validator: ValidatorCheck::not_found(),
        };
        print_validate_report(format, &report, report_path.as_deref())?;
        return Err("validator not configured; pass --validator or set VST3_VALIDATOR".into());
    };

    if format == OutputFormat::Text {
        println!("validator: {validator}");
    }

    let validator_check = match Command::new(validator.as_std_path()).arg(&bundle).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let summary =
                validator_test_summary(&stdout).or_else(|| validator_test_summary(&stderr));
            if format == OutputFormat::Text {
                print_child_output(&stdout, &stderr);
            }
            write_validator_log(
                validator_log_path.as_deref(),
                &validator,
                &bundle,
                &stdout,
                &stderr,
            )?;
            ValidatorCheck {
                status: if output.status.success() {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
                path: Some(validator.to_string()),
                exit_code: output.status.code(),
                tests_passed: summary.map(|(passed, _)| passed),
                tests_failed: summary.map(|(_, failed)| failed),
                stdout: (!stdout.is_empty()).then_some(stdout),
                stderr: (!stderr.is_empty()).then_some(stderr),
                reason: None,
                error: None,
            }
        }
        Err(error) => {
            if let Some(path) = validator_log_path.as_deref() {
                write_text_file(
                    path,
                    &format!("validator={validator}\nbundle={bundle}\nerror={error}\n"),
                )?;
            }
            ValidatorCheck::process_error(&validator, error)
        }
    };

    let validator_passed = validator_check.status == "passed";
    let process_error = validator_check.error.clone();
    let report = ValidateReport {
        bundle: bundle.to_string(),
        static_check,
        validator: validator_check,
    };
    print_validate_report(format, &report, report_path.as_deref())?;
    if !validator_passed {
        if let Some(error) = process_error {
            return Err(format!("VST3 validator failed to run: {error}").into());
        }
        return Err("VST3 validator failed".into());
    }
    if let Some(error) = strict
        .then(|| strict_static_bundle_check_error(&report.static_check))
        .flatten()
    {
        return Err(error.into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ParamManifestReport {
    status: String,
    specs: String,
    manifest: Option<String>,
    parameters: usize,
    id_algorithm: String,
    check: bool,
}

fn run_param_manifest(
    specs: Utf8PathBuf,
    out: Option<Utf8PathBuf>,
    check: bool,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(format)?;
    let specs_text = read_text_file_no_symlink("parameter specs", &specs)?;
    let manifest = parameter_manifest_from_specs_json(&specs_text)?;
    let out_path = out.as_deref();

    if check {
        let Some(out_path) = out_path else {
            return Err("--check requires --out <manifest.json>".into());
        };
        let existing = read_parameter_manifest(out_path)?;
        if existing != manifest {
            return Err(format!(
                "parameter manifest is out of date: {out_path}; regenerate from {specs}"
            )
            .into());
        }
        print_param_manifest_report(
            format,
            &manifest,
            &ParamManifestReport {
                status: "ok".to_string(),
                specs: specs.to_string(),
                manifest: Some(out_path.to_string()),
                parameters: manifest.parameters.len(),
                id_algorithm: manifest.id_algorithm.clone(),
                check: true,
            },
        )?;
        return Ok(());
    }

    let report = ParamManifestReport {
        status: "generated".to_string(),
        specs: specs.to_string(),
        manifest: out_path.map(ToString::to_string),
        parameters: manifest.parameters.len(),
        id_algorithm: manifest.id_algorithm.clone(),
        check: false,
    };
    if let Some(out_path) = out_path {
        write_parameter_manifest(out_path, &manifest)?;
        print_param_manifest_report(format, &manifest, &report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    }
    Ok(())
}

fn write_parameter_manifest(
    path: &Utf8Path,
    manifest: &ParameterManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    write_text_file(path, &(serde_json::to_string_pretty(manifest)? + "\n"))
}

fn print_param_manifest_report(
    format: OutputFormat,
    manifest: &ParameterManifest,
    report: &ParamManifestReport,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Text => match report.status.as_str() {
            "ok" => println!(
                "parameter manifest: ok ({} params, {})",
                report.parameters, report.id_algorithm
            ),
            _ => {
                if let Some(path) = &report.manifest {
                    println!(
                        "parameter manifest: {path} ({} params, {})",
                        report.parameters, report.id_algorithm
                    );
                } else {
                    println!("{}", serde_json::to_string_pretty(manifest)?);
                }
            }
        },
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
    }
    Ok(())
}

fn print_validate_report(
    format: OutputFormat,
    report: &ValidateReport,
    report_path: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_shape(report)?;
    write_validate_report(report_path, report)?;
    match format {
        OutputFormat::Text => match report.validator.status.as_str() {
            "skipped" => println!(
                "validator: skipped ({})",
                report.validator.reason.as_deref().unwrap_or("requested")
            ),
            "not_found" => println!(
                "validator: not found ({})",
                report
                    .validator
                    .reason
                    .as_deref()
                    .unwrap_or("configure validator")
            ),
            "failed" if report.validator.error.is_some() => println!(
                "validator: failed to run ({})",
                report.validator.error.as_deref().unwrap_or("")
            ),
            _ => {}
        },
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
    }
    Ok(())
}

fn write_validate_report(
    path: Option<&Utf8Path>,
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else {
        return Ok(());
    };
    validate_validate_report_shape(report)?;
    let text = serde_json::to_string_pretty(report)?;
    write_text_file(path, &(text + "\n"))
}

fn write_validator_log(
    path: Option<&Utf8Path>,
    validator: &Utf8Path,
    bundle: &Utf8Path,
    stdout: &str,
    stderr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut text = format!("validator={validator}\nbundle={bundle}\n");
    if !stdout.is_empty() {
        text.push_str("\n[stdout]\n");
        text.push_str(stdout);
        if !stdout.ends_with('\n') {
            text.push('\n');
        }
    }
    if !stderr.is_empty() {
        text.push_str("\n[stderr]\n");
        text.push_str(stderr);
        if !stderr.ends_with('\n') {
            text.push('\n');
        }
    }
    write_text_file(path, &text)
}

fn write_text_file(path: &Utf8Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    reject_existing_path_symlink("output file", path)?;
    reject_existing_output_parent_symlink("output file", path)?;
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn reject_existing_output_parent_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
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
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(format!("{label} parent must not be a symlink: {ancestor}").into());
        }
        if !metadata.is_dir() {
            return Err(format!("{label} parent must be a directory: {ancestor}").into());
        }
    }
    Ok(())
}

fn print_child_output(stdout: &str, stderr: &str) {
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
}

fn validator_test_summary(text: &str) -> Option<(u32, u32)> {
    let mut passed = None;
    let mut failed = None;
    for line in text.lines() {
        let line = line.to_ascii_lowercase();
        if passed.is_none() {
            passed = extract_count_near_any_marker(&line, &["tests passed", "test passed"]);
        }
        if failed.is_none() {
            failed = extract_count_near_any_marker(&line, &["tests failed", "test failed"]);
        }
        if let (Some(passed), Some(failed)) = (passed, failed) {
            return Some((passed, failed));
        }
    }
    None
}

fn extract_count_near_any_marker(line: &str, markers: &[&str]) -> Option<u32> {
    markers
        .iter()
        .find_map(|marker| extract_count_near_marker(line, marker))
}

fn extract_count_near_marker(line: &str, marker: &str) -> Option<u32> {
    let index = line.find(marker)?;
    last_number_before(&line[..index]).or_else(|| first_number_after(&line[index + marker.len()..]))
}

fn last_number_before(text: &str) -> Option<u32> {
    let end = text.rfind(|char: char| char.is_ascii_digit())?;
    let start = text[..=end]
        .rfind(|char: char| !char.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    text[start..=end].parse().ok()
}

fn first_number_after(text: &str) -> Option<u32> {
    let start = text.find(|char: char| char.is_ascii_digit())?;
    let end = text[start..]
        .find(|char: char| !char.is_ascii_digit())
        .map(|index| start + index)
        .unwrap_or(text.len());
    text[start..end].parse().ok()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DirectoryDiff {
    missing: Vec<String>,
    changed: Vec<String>,
    extra: Vec<String>,
}

impl DirectoryDiff {
    const MAX_REPORTED_PATHS_PER_KIND: usize = 12;

    fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.changed.is_empty() && self.extra.is_empty()
    }

    fn summary(&self) -> String {
        format!(
            "{} missing, {} changed, {} extra",
            self.missing.len(),
            self.changed.len(),
            self.extra.len()
        )
    }

    fn summary_with_paths(&self) -> String {
        let details = [
            Self::format_paths("missing", &self.missing),
            Self::format_paths("changed", &self.changed),
            Self::format_paths("extra", &self.extra),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if details.is_empty() {
            self.summary()
        } else {
            format!("{} ({})", self.summary(), details.join("; "))
        }
    }

    fn format_paths(kind: &str, paths: &[String]) -> Option<String> {
        if paths.is_empty() {
            return None;
        }

        let shown = paths
            .iter()
            .take(Self::MAX_REPORTED_PATHS_PER_KIND)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if paths.len() > Self::MAX_REPORTED_PATHS_PER_KIND {
            Some(format!(
                "{kind}: {shown}, ... (+{} more)",
                paths.len() - Self::MAX_REPORTED_PATHS_PER_KIND
            ))
        } else {
            Some(format!("{kind}: {shown}"))
        }
    }
}

fn check_protocol_export(out: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    if !out.is_dir() {
        return Err(format!("protocol snapshot directory does not exist: {out}").into());
    }

    let temp_dir = unique_temp_dir("vesty-protocol-check")?;
    let result = (|| -> Result<DirectoryDiff, Box<dyn std::error::Error>> {
        vesty_ipc::export_protocol_bindings(&temp_dir)?;
        diff_directories(out, &temp_dir)
    })();
    let _ = fs::remove_dir_all(&temp_dir);

    let diff = result?;
    if diff.is_empty() {
        return Ok(());
    }

    for missing in &diff.missing {
        eprintln!("protocol missing: {missing}");
    }
    for changed in &diff.changed {
        eprintln!("protocol changed: {changed}");
    }
    for extra in &diff.extra {
        eprintln!("protocol extra: {extra}");
    }
    Err(format!(
        "protocol export drift detected: {}",
        diff.summary_with_paths()
    )
    .into())
}

fn unique_temp_dir(prefix: &str) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir(&path)?;
    Utf8PathBuf::from_path_buf(path)
        .map_err(|_| "temporary protocol export path is not valid utf-8".into())
}

fn diff_directories(
    expected_dir: &Utf8Path,
    actual_dir: &Utf8Path,
) -> Result<DirectoryDiff, Box<dyn std::error::Error>> {
    let expected = collect_relative_files(expected_dir)?;
    let actual = collect_relative_files(actual_dir)?;
    let mut diff = DirectoryDiff::default();

    for path in expected.keys() {
        if !actual.contains_key(path) {
            diff.missing.push(path.clone());
        }
    }
    for (path, actual_bytes) in &actual {
        match expected.get(path) {
            Some(expected_bytes) if expected_bytes != actual_bytes => {
                diff.changed.push(path.clone())
            }
            Some(_) => {}
            None => diff.extra.push(path.clone()),
        }
    }
    Ok(diff)
}

fn collect_relative_files(
    root: &Utf8Path,
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut files = BTreeMap::new();
    collect_relative_files_inner(root, root, &mut files)?;
    Ok(files)
}

fn collect_relative_files_inner(
    root: &Utf8Path,
    current: &Utf8Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "protocol export path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("protocol export contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_relative_files_inner(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "protocol export path escaped root")?
                .as_str()
                .replace('\\', "/");
            files.insert(relative, fs::read(&path)?);
        }
    }
    Ok(())
}

struct DawEvidencePaths {
    reaper: Utf8PathBuf,
    cubase: Utf8PathBuf,
    bitwig: Utf8PathBuf,
    ableton: Utf8PathBuf,
    studio_one: Utf8PathBuf,
}

impl DawEvidencePaths {
    fn from_root(root: &Utf8Path) -> Self {
        Self {
            reaper: root.join("reaper"),
            cubase: root.join("cubase"),
            bitwig: root.join("bitwig"),
            ableton: root.join("ableton"),
            studio_one: root.join("studio-one"),
        }
    }
}

fn resolve_daw_evidence_paths(
    evidence_root: Option<Utf8PathBuf>,
    reaper: Utf8PathBuf,
    cubase: Utf8PathBuf,
    bitwig: Utf8PathBuf,
    ableton: Utf8PathBuf,
    studio_one: Utf8PathBuf,
) -> DawEvidencePaths {
    evidence_root
        .as_deref()
        .map(DawEvidencePaths::from_root)
        .unwrap_or(DawEvidencePaths {
            reaper,
            cubase,
            bitwig,
            ableton,
            studio_one,
        })
}

#[derive(Clone, Debug, Default)]
struct DawSmokeReportInput {
    host: Option<String>,
    platform: Option<String>,
    scan: Option<String>,
    load: Option<String>,
    ui: Option<String>,
    ui_host_param: Option<String>,
    meter_stream: Option<String>,
    automation: Option<String>,
    buffer_sample_rate_change: Option<String>,
    save_restore: Option<String>,
    offline_render: Option<String>,
}

const DAW_SMOKE_MARKER_MAX_BYTES: usize = 256 * 1024;

fn write_daw_smoke_report(
    evidence: &DawEvidencePaths,
    input: DawSmokeReportInput,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let host_raw = input
        .host
        .as_deref()
        .ok_or("DAW smoke report requires `--host <reaper|cubase|bitwig|ableton|studio-one>`")?;
    validate_release_action_text("DAW smoke report host", host_raw)?;
    let profile = vesty_core::find_host_profile(host_raw)
        .ok_or_else(|| format!("unknown host profile '{host_raw}'"))?;
    let dir = daw_evidence_dir_for_host(evidence, profile.id).to_path_buf();
    create_directory_no_parent_or_leaf_symlink("DAW evidence directory", &dir)?;

    let platform = required_daw_platform_marker(profile, input.platform)?;
    let scan = required_daw_marker("scan", input.scan)?;
    let load = required_daw_marker("load", input.load)?;
    let ui = required_daw_marker("ui", input.ui)?;
    let ui_host_param = required_daw_marker("ui-host-param", input.ui_host_param)?;
    let meter_stream = required_daw_marker("meter-stream", input.meter_stream)?;
    let automation = required_daw_marker("automation", input.automation)?;
    let buffer_sample_rate_change =
        required_daw_marker("buffer-sample-rate-change", input.buffer_sample_rate_change)?;
    let save_restore = required_daw_marker("save-restore", input.save_restore)?;
    let offline_render = required_daw_marker("offline-render", input.offline_render)?;

    validate_daw_smoke_report_markers(DawSmokeReportMarkers {
        profile,
        evidence_dir: &dir,
        scan: &scan,
        load: &load,
        ui: &ui,
        ui_host_param: &ui_host_param,
        meter_stream: &meter_stream,
        automation: &automation,
        buffer_sample_rate_change: &buffer_sample_rate_change,
        save_restore: &save_restore,
        offline_render: &offline_render,
    })?;

    write_text_file(&dir.join("platform.txt"), &(platform + "\n"))?;
    write_text_file(&dir.join("scan-smoke.log"), &(scan + "\n"))?;
    write_text_file(&dir.join("load-smoke.log"), &(load + "\n"))?;
    write_text_file(&dir.join("ui-smoke.log"), &(ui + "\n"))?;
    write_text_file(&dir.join("ui-host-smoke.log"), &(ui_host_param + "\n"))?;
    write_text_file(&dir.join("meter-stream.log"), &(meter_stream + "\n"))?;
    write_text_file(&dir.join("automation-smoke.log"), &(automation + "\n"))?;
    write_text_file(
        &dir.join("buffer-sample-rate.log"),
        &(buffer_sample_rate_change + "\n"),
    )?;
    write_text_file(&dir.join("restore-smoke.log"), &(save_restore + "\n"))?;
    write_text_file(&dir.join("offline-render.log"), &(offline_render + "\n"))?;

    let row = collect_daw_evidence_for_profile(profile, &dir);
    if !daw_row_complete(&row) {
        let missing = daw_missing_checks(&row);
        return Err(format!(
            "written DAW smoke evidence did not pass parser for {}: missing {}",
            profile.name,
            missing.join(", ")
        )
        .into());
    }
    Ok(dir)
}

struct DawSmokeReportMarkers<'a> {
    profile: &'a vesty_core::HostProfile,
    evidence_dir: &'a Utf8Path,
    scan: &'a str,
    load: &'a str,
    ui: &'a str,
    ui_host_param: &'a str,
    meter_stream: &'a str,
    automation: &'a str,
    buffer_sample_rate_change: &'a str,
    save_restore: &'a str,
    offline_render: &'a str,
}

fn validate_daw_smoke_report_markers(
    markers: DawSmokeReportMarkers<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let checks = [
        (
            "scan",
            daw_marker_matches_for_profile(markers.profile, markers.scan, generic_scan_ok),
        ),
        (
            "load",
            daw_marker_matches_for_profile(markers.profile, markers.load, generic_load_ok),
        ),
        (
            "ui",
            daw_marker_matches_for_profile(markers.profile, markers.ui, generic_ui_ok),
        ),
        (
            "ui-host-param",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.ui_host_param,
                generic_ui_host_ok,
            ),
        ),
        (
            "meter-stream",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.meter_stream,
                meter_stream_delivered,
            ),
        ),
        (
            "automation",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.automation,
                generic_automation_ok,
            ),
        ),
        (
            "buffer-sample-rate-change",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.buffer_sample_rate_change,
                generic_buffer_sample_rate_change_ok,
            ),
        ),
        (
            "save-restore",
            daw_marker_matches_for_profile(
                markers.profile,
                markers.save_restore,
                generic_restore_ok,
            ),
        ),
        (
            "offline-render",
            daw_marker_matches_for_profile(markers.profile, markers.offline_render, |text| {
                generic_offline_render_ok(text)
                    || render_file_evidence_ok(text, markers.evidence_dir)
            }),
        ),
    ];

    let missing = checks
        .iter()
        .filter_map(|(name, ok)| (!ok).then_some(*name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "DAW smoke report markers did not pass parser: {}",
            missing.join(", ")
        )
        .into())
    }
}

fn daw_evidence_dir_for_host<'a>(evidence: &'a DawEvidencePaths, host_id: &str) -> &'a Utf8PathBuf {
    match host_id {
        "reaper" => &evidence.reaper,
        "cubase-nuendo" => &evidence.cubase,
        "bitwig" => &evidence.bitwig,
        "ableton-live" => &evidence.ableton,
        "studio-one" => &evidence.studio_one,
        _ => &evidence.reaper,
    }
}

fn required_daw_marker(
    name: &str,
    value: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err(format!("DAW smoke report requires `--{name}`").into());
    };
    validate_daw_smoke_marker_text(name, &value)?;
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || lower == "pending"
        || lower == "false"
        || lower.starts_with("pending ")
        || lower.contains("manual platform pending")
        || lower.contains("replace with real")
        || daw_marker_has_missing_assignment(trimmed)
        || daw_marker_has_negative_evidence(trimmed)
    {
        return Err(format!("DAW smoke report `{name}` is missing positive evidence").into());
    }
    if name == "meter-stream" && !meter_stream_delivered(trimmed) {
        return Err(format!("DAW smoke report `{name}` is missing nonzero meter evidence").into());
    }
    Ok(trimmed.to_string())
}

fn required_daw_platform_marker(
    profile: &vesty_core::HostProfile,
    value: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err("DAW smoke report requires `--platform`".into());
    };
    validate_daw_smoke_marker_text("platform", &value)?;
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || lower == "pending"
        || lower.contains("manual platform pending")
        || lower.contains("replace with real")
        || lower.contains("wayland")
        || daw_marker_has_negative_evidence(trimmed)
    {
        return Err(
            "DAW smoke report `platform` is missing supported host/platform evidence".into(),
        );
    }
    let Some(platform) = daw_smoke_platform_from_text(trimmed) else {
        return Err(format!(
            "DAW smoke report `platform` must mention a supported platform for {}: {}",
            profile.name,
            profile.platforms.join(", ")
        )
        .into());
    };
    if !profile.platforms.contains(&platform) {
        return Err(format!(
            "DAW smoke report `platform` `{platform}` is not supported by {} profile (expected one of {})",
            profile.name,
            profile.platforms.join(", ")
        )
        .into());
    }
    Ok(trimmed.to_string())
}

fn daw_smoke_platform_from_text(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("macos")
        || lower.contains("mac os")
        || lower.contains("darwin")
        || lower.contains("osx")
    {
        Some("macos")
    } else if lower.contains("windows") || lower.contains("win64") || lower.contains("win32") {
        Some("windows")
    } else if lower.contains("linux") && lower.contains("x11") {
        Some("linux-x11")
    } else if lower.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

fn validate_daw_smoke_marker_text(
    name: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let label = format!("DAW smoke report `{name}`");
    if name == "platform" {
        validate_release_action_text(&label, value)?;
        return Ok(());
    }
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty").into());
    }
    if value.len() > DAW_SMOKE_MARKER_MAX_BYTES {
        return Err(format!("{label} must be at most {DAW_SMOKE_MARKER_MAX_BYTES} bytes").into());
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(
            format!("{label} must not contain control characters other than tab/newline").into(),
        );
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!("{label} must not contain unsafe Unicode format characters").into());
    }
    Ok(())
}

fn daw_marker_has_negative_evidence(text: &str) -> bool {
    text.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        if split_marker_assignment(&lower)
            .is_some_and(|(key, _)| key.trim().replace('-', "_") == "render_file")
        {
            return false;
        }
        [
            "not found",
            "not installed",
            "unavailable",
            "failed",
            "failure",
            "error",
            "timeout",
            "timed out",
            "crashed",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    })
}

fn daw_marker_has_missing_assignment(text: &str) -> bool {
    text.lines().any(|line| {
        line.split(';').any(|fragment| {
            let normalized = fragment.trim().to_ascii_lowercase().replace('-', "_");
            let Some((_, raw_value)) = split_marker_assignment(&normalized) else {
                return false;
            };
            let value = raw_value
                .trim()
                .trim_start_matches(['`', '"', '\''])
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .find(|token| !token.is_empty())
                .unwrap_or_default();
            matches!(value, "pending" | "false")
        })
    })
}

fn write_daw_evidence_templates(
    evidence: &DawEvidencePaths,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut created = 0;
    created += write_host_evidence_template("REAPER", &evidence.reaper)?;
    created += write_host_evidence_template("Cubase/Nuendo", &evidence.cubase)?;
    created += write_host_evidence_template("Bitwig Studio", &evidence.bitwig)?;
    created += write_host_evidence_template("Ableton Live", &evidence.ableton)?;
    created += write_host_evidence_template("Studio One", &evidence.studio_one)?;
    Ok(created)
}

fn write_host_evidence_template(
    host: &str,
    dir: &Utf8PathBuf,
) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &host_evidence_readme(host))?;
    created += write_template_file(dir.join("platform.txt"), "manual platform pending\n")?;
    created += write_template_file(dir.join("scan-smoke.log"), "scan=pending\n")?;
    created += write_template_file(dir.join("load-smoke.log"), "load=pending\n")?;
    created += write_template_file(dir.join("ui-smoke.log"), "ui=pending\n")?;
    created += write_template_file(dir.join("ui-host-smoke.log"), "ui_host_param=pending\n")?;
    created += write_template_file(dir.join("meter-stream.log"), "meter_flush sent=0\n")?;
    created += write_template_file(dir.join("automation-smoke.log"), "automation=pending\n")?;
    created += write_template_file(
        dir.join("buffer-sample-rate.log"),
        "buffer_sample_rate_change=pending\n",
    )?;
    created += write_template_file(dir.join("restore-smoke.log"), "save_restore=pending\n")?;
    created += write_template_file(dir.join("offline-render.log"), "offline_render=pending\n")?;
    Ok(created)
}

fn host_evidence_readme(host: &str) -> String {
    let profile = vesty_core::find_host_profile(host);
    let mut text = String::new();
    text.push_str(&format!("# {host} Vesty Smoke Evidence\n\n"));
    text.push_str(
        "Fill these files after a real host smoke run. Leave `pending` values untouched until the evidence exists.\n\n",
    );
    text.push_str(
        "Templates, pending values and `vesty doctor` install detection do not count as pass evidence.\n\n",
    );

    if let Some(profile) = profile {
        text.push_str("## Host Profile\n\n");
        text.push_str(&format!("- Host id: `{}`\n", profile.id));
        text.push_str(&format!("- Platforms: {}\n", profile.platforms.join(", ")));
        text.push_str(&format!(
            "- Required smoke checks: {}\n",
            profile.required_smoke_checks.join(", ")
        ));
        if !profile.notes.is_empty() {
            text.push_str("- Notes:\n");
            for note in profile.notes {
                text.push_str(&format!("  - {note}\n"));
            }
        }
        text.push('\n');

        if !profile.quirks.is_empty() {
            text.push_str("## Host Quirks\n\n");
            text.push_str("| Area | Severity | Summary | Mitigation |\n");
            text.push_str("| --- | --- | --- | --- |\n");
            for quirk in profile.quirks {
                text.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    host_quirk_area_text(quirk.area),
                    host_quirk_severity_text(quirk.severity),
                    markdown_cell(quirk.summary),
                    markdown_cell(quirk.mitigation)
                ));
            }
            text.push('\n');
        }
    } else {
        text.push_str("## Expected Checks\n\n");
        text.push_str(&format!(
            "- Required smoke checks: {}\n\n",
            vesty_core::RELEASE_SMOKE_CHECKS.join(", ")
        ));
    }

    text.push_str("## Evidence Files\n\n");
    text.push_str("- `platform.txt`: exact OS, architecture and host version.\n");
    text.push_str("- `scan-smoke.log`: plugin scan result for all Vesty examples.\n");
    text.push_str("- `load-smoke.log`: instance creation/load result.\n");
    text.push_str("- `ui-smoke.log`: editor open/close result.\n");
    text.push_str("- `ui-host-smoke.log`: UI parameter edit observed by the host.\n");
    text.push_str(
        "- `meter-stream.log`: nonzero `meter.main` stream or `meter_flush sent=N` evidence.\n",
    );
    text.push_str("- `automation-smoke.log`: host automation record/playback result.\n");
    text.push_str("- `buffer-sample-rate.log`: buffer size and sample-rate change result.\n");
    text.push_str("- `restore-smoke.log`: save, close and restore result.\n");
    text.push_str("- `offline-render.log`: offline render result or `render_file=/absolute/path.wav` marker.\n");
    text.push_str("\n## Accepted Pass Markers\n\n");
    text.push_str(
        "Use these exact markers when the corresponding real smoke check has passed:\n\n",
    );
    text.push_str("```text\n");
    text.push_str("scan=true\n");
    text.push_str("load=true\n");
    text.push_str("ui=true\n");
    text.push_str("ui_host_param=true\n");
    text.push_str("meter_flush sent=1\n");
    text.push_str("automation=true\n");
    text.push_str("buffer_sample_rate_change=true\n");
    text.push_str("save_restore=true\n");
    text.push_str("offline_render=true\n");
    text.push_str("render_file=/absolute/path/to/rendered.wav\n");
    text.push_str("```\n\n");
    text.push_str("After filling evidence, run:\n\n");
    text.push_str("```bash\n");
    text.push_str("vesty daw-matrix --evidence-root target/daw-evidence --format markdown\n");
    text.push_str("vesty daw-matrix --evidence-root target/daw-evidence --strict\n");
    text.push_str("```\n");
    text
}

fn write_release_evidence_templates(
    dir: &Utf8PathBuf,
) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    create_template_dir(&dir.join("ci-doctor"))?;
    create_template_dir(&dir.join("ci-release-checks"))?;
    create_template_dir(&dir.join("platform-smoke"))?;
    create_template_dir(&dir.join("validator"))?;
    create_template_dir(&dir.join("package"))?;
    create_template_dir(&dir.join("publish-plan"))?;
    create_template_dir(&dir.join("crate-package"))?;
    create_template_dir(&dir.join("npm-pack"))?;
    create_template_dir(&dir.join("dependency-baseline"))?;
    create_template_dir(&dir.join("vst3-sdk"))?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &release_evidence_readme())?;
    created += write_template_file(
        dir.join("ci-run-url.txt"),
        "ci_run_url=pending\n# pass the real URL with --ci-run-url\n",
    )?;
    let validate_report_template = pending_validate_report_template()?;
    created += write_template_file(dir.join("validate-report.json"), &validate_report_template)?;
    let static_validate_report_template = pending_static_validate_report_template()?;
    created += write_template_file(
        dir.join("static-validate-report.json"),
        &static_validate_report_template,
    )?;
    created += write_template_file(dir.join("ci-doctor/README.md"), &ci_doctor_readme())?;
    created += write_template_file(
        dir.join("ci-release-checks/README.md"),
        &ci_release_checks_readme(),
    )?;
    created += write_template_file(
        dir.join("platform-smoke/README.md"),
        &platform_smoke_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("publish-plan/README.md"),
        &publish_plan_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("crate-package/README.md"),
        &crate_package_evidence_readme(),
    )?;
    created += write_template_file(dir.join("npm-pack/README.md"), &npm_pack_evidence_readme())?;
    created += write_template_file(
        dir.join("dependency-baseline/README.md"),
        &dependency_baseline_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("vst3-sdk/README.md"),
        &vst3_sdk_manifest_evidence_readme(),
    )?;
    created += write_template_file(
        dir.join("signing-macos.log"),
        "signed=pending\ncodesign verify=pending\n",
    )?;
    created += write_template_file(
        dir.join("signing-windows.log"),
        "signed=pending\nsigntool verify=pending\n",
    )?;
    created += write_template_file(
        dir.join("notary.log"),
        "notarization=pending\nstapled=pending\n",
    )?;
    Ok(created)
}

fn release_evidence_readme() -> String {
    [
        "# Vesty Release Artifact Evidence",
        "",
        "Fill these files after real release verification. Leave `pending` values untouched until the evidence exists.",
        "",
        "Templates and pending values do not count as pass evidence.",
        "",
        "Local evidence helper:",
        "",
        "```bash",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol",
        "",
        "# Optional explicit online dependency latest review:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --dependency-baseline-latest",
        "",
        "# Optional crate package readiness smoke:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --crate-package",
        "",
        "# Optional VST3 SDK audit evidence when an official SDK checkout is available:",
        "vesty release-evidence collect-local \\",
        "  --dir target/release-evidence \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --vst3-sdk-dir /path/to/VST_SDK \\",
        "  --vst3-sdk-bindings-module target/vst3-sdk/generated.rs",
        "```",
        "",
        "`collect-local` writes the template, exports/checks the protocol snapshot, and generates crate publish-plan plus npm pack dry-run reports from real local commands. With `--crate-package`, it also runs `vesty crate-package` and writes `crate-package/crate-package.json`; this proves currently packageable leaf crates and records deferred crates that require already published internal dependencies. With `--dependency-baseline-latest`, it also runs the explicit online crates.io/npm latest dependency review and writes `dependency-baseline/dependency-baseline-latest.json`. With `--vst3-sdk-dir <official-vst3sdk>`, it also writes `vst3-sdk/vst3-sdk-headers.json`, `vst3-sdk/generated-bindings-plan.json`, `vst3-sdk/generated-bindings-surface.json`, metadata-only `vst3-sdk/generated.rs`, ABI seed `vst3-sdk/generated-abi-seed.rs`, ABI layout `vst3-sdk/generated-abi.rs`, and interface/vtable skeleton plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata, pure IID/dispatch lookup helpers, and pure binary export required-symbol / inspection-tool helpers in `vst3-sdk/generated-interface-skeleton.rs` as optional generated-headers audit evidence. The binding plan and surface must keep `bindingsGenerated: false`; the scaffold must keep `BINDINGS_GENERATED = false`; the ABI seed, ABI layout and interface skeleton must keep `FULL_COM_BINDINGS_GENERATED = false`; the ABI layout must keep `ABI_LAYOUT_GENERATED = true` plus layout size/alignment and field-offset fingerprints; the interface skeleton must keep `INTERFACE_SKELETON_GENERATED = true` and preserve audit-only method order/signature metadata plus vtable local slot/field/callback type alias/field layout seeds, offset fingerprints, interface IID records, queryInterface planned dispatch entries, pure lookup helpers, Vesty COM object interface exposure records, object FUnknown identity plans, per-object queryInterface dispatch records, factory export plan records, processor/controller factory class plan records, platform module export plan records, binary export symbol plan records, binary export inspection tool plan records, and binary export required-symbol lookup/missing-symbol helpers. These are readiness/surface/drift/interface/ABI/COM identity/dispatch/exposure-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan artifacts, not proof that full SDK 3.8 bindings are generated. It does not inspect plugin binaries and does not create DAW, platform smoke, validator-passed, CI, signing or notarization evidence.",
        "",
        "CI artifact import helper:",
        "",
        "```bash",
        "vesty release-evidence import-ci \\",
        "  --source target/downloaded-artifacts \\",
        "  --dir target/release-evidence \\",
        "  --ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>",
        "```",
        "",
        "`import-ci` scans an already-downloaded GitHub Actions artifact directory, content-validates recognized artifacts, and copies only accepted evidence into the standard release evidence layout. It can import crate package readiness into `crate-package/crate-package.json`, the explicit latest dependency review into `dependency-baseline/dependency-baseline-latest.json`, and valid per-OS release action plan sidecars into `ci-release-checks/release-action-plan-<OS>.json`; an offline `dependency-baseline.json` drift report is not accepted as release latest evidence. Invalid action plan sidecars are recorded as failed and not copied, and valid sidecars remain checklist metadata rather than release-check pass evidence. For optional VST3 SDK artifacts it can import valid header manifest, generated-bindings plan, generated-bindings surface, metadata-only `generated.rs` scaffold, `generated-abi-seed.rs` ABI seed, `generated-abi.rs` ABI layout, and `generated-interface-skeleton.rs` interface/vtable skeleton plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata and pure IID/dispatch lookup helper artifacts into `vst3-sdk/`; release-check validates these SDK audit files when they are present, but they remain drift/audit metadata rather than proof that full SDK 3.8 bindings or final release readiness exist. It writes `import-ci-report.json`, preserves existing files by default, and supports `--overwrite` for a deliberate re-import. It does not synthesize CI, DAW, validator, signing or notarization passes.",
        "",
        "Signing and notarization evidence helpers:",
        "",
        "```bash",
        "vesty release-evidence collect-signing target/vesty/MyPlugin.vst3 \\",
        "  --platform macos \\",
        "  --dir target/release-evidence",
        "vesty release-evidence collect-signing target/vesty/MyPlugin.vst3 \\",
        "  --platform windows-x64 \\",
        "  --dir target/release-evidence",
        "vesty release-evidence collect-notarization \\",
        "  --notary-log target/notarytool.log \\",
        "  --stapler-log target/stapler.log \\",
        "  --dir target/release-evidence",
        "```",
        "",
        "`collect-signing` runs real `codesign --verify` or `signtool verify` and writes the captured log only after the evidence parser accepts the platform coverage. `collect-notarization` combines real notarytool and stapler logs and writes `notary.log` only when both acceptance and stapler success are present. Linux signing remains release-channel specific and is not produced by these helpers.",
        "",
        "Suggested strict gate:",
        "",
        "```bash",
        "vesty daw-matrix --write-template --evidence-root target/daw-evidence",
        "vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke",
        "vesty export-types --out target/vesty-protocol --check",
        "vesty validate target/vesty/MyPlugin.vst3 --strict --format json --report target/release-evidence/validator/MyPlugin.macos.validate.json --validator-log target/release-evidence/validator/MyPlugin.macos.validator.log",
        "vesty release-check --strict --require-release-artifacts \\",
        "  --protocol-snapshot target/vesty-protocol \\",
        "  --evidence-root target/daw-evidence \\",
        "  --release-evidence-dir target/release-evidence \\",
        "  --plan target/release-evidence/release-action-plan.json \\",
        "  --report target/release-evidence/release-check.json",
        "```",
        "",
        "`--plan` writes a machine-readable action plan derived from the current release-check result. It lists failed/skipped checks, evidence paths and suggested commands, but it is not pass evidence and does not change the release gate result.",
        "",
        "Do not pass `--skip-protocol` to the final `--require-release-artifacts` gate; protocol drift must be checked for release evidence.",
        "",
        "Optional CI static packaging smoke:",
        "",
        "```bash",
        "vesty validate target/vesty/MyPlugin.vst3 --static-only --strict --format json --report target/release-evidence/package/MyPlugin.macos.static-validate.json",
        "vesty release-check --static-validate-report target/release-evidence/package/MyPlugin.macos.static-validate.json",
        "```",
        "",
        "Evidence files:",
        "",
        "- `ci-run-url.txt`: paste the real GitHub Actions run URL, then pass it through `--ci-run-url`.",
        "- `import-ci-report.json`: written by `vesty release-evidence import-ci`; records imported, skipped and failed artifacts from a real downloaded CI artifact directory. This report is audit metadata, not pass evidence by itself.",
        "- `ci-release-checks/`: optional conventional directory for downloaded per-OS `release-check-*` artifacts from CI. `--require-release-artifacts` requires Linux, macOS and Windows snapshots whose local invariant checks passed; OS coverage may be inferred from the `release-check*.json` file name or from parent directories such as `Linux/release-check.json`, `macOS/release-check.json` and `Windows/release-check.json`. When `ci-run-url.txt` is present, each snapshot must carry the same `ci_run_url` repo/run id. Valid `release-action-plan-<OS>.json` sidecars may also be preserved here by `import-ci` for human follow-up, but `--ci-release-check-dir` only reads `release-check*.json` and never treats action plans as pass evidence. These snapshots may be generated with `--skip-protocol` on matrix runners, but the final strict release gate must run the protocol snapshot check and must not pass `--skip-protocol`. These snapshots do not replace DAW, validator, signing or notarization evidence.",
        "- `platform-smoke/`: place platform smoke JSON reports from macOS, Windows x64 and Linux X11 real host/platform runs. `--require-release-artifacts` requires all three platforms and checks system WebView, VST3 validator, VST3 example scan, WebView attach/resize, manifest asset protocol, JSBridge roundtrip and nonzero meter stream evidence. Linux Wayland remains experimental and does not satisfy the Linux X11 gate.",
        "- `publish-plan/publish-plan.json`: place the downloaded `vesty-publish-plan` artifact from CI, or generate it with `vesty publish-plan --out publish-plan/publish-plan.json`. The report must preserve dependency-safe crate order.",
        "- `crate-package/crate-package.json`: crate package readiness report from CI, or generate it with `vesty crate-package --out crate-package/crate-package.json`. It is required by `release-check --require-release-artifacts` and must show zero-internal-dependency crates as `packaged` and internal-dependent crates as `deferred` until their workspace dependencies are published.",
        "- `npm-pack/npm-pack.json`: place the downloaded npm pack dry-run artifact from CI, or generate it with `vesty npm-pack --out npm-pack/npm-pack.json`. The report must include `vesty-plugin-ui`, and each packed file must stay inside `dist/**` or `package.json`.",
        "- `dependency-baseline/dependency-baseline-latest.json`: place the downloaded latest dependency review artifact from CI, or generate it with `vesty dependency-baseline --latest --out dependency-baseline/dependency-baseline-latest.json`. The report must include the `cargo workspace external dependency baseline coverage` check plus crates.io/npm registry latest checks. `dependency-baseline.json` without registry latest checks is useful drift evidence, but it does not satisfy the final `--require-release-artifacts` gate.",
        "- `vst3-sdk/vst3-sdk-headers.json`: optional official Steinberg VST3 SDK header input manifest generated by `vesty vst3-sdk manifest`. When present, `release-check` verifies manifest version, generator, SDK/crate baselines, required header set, `missing_headers`, SHA-256 shape and completeness. This is audit evidence for the reserved generated-headers backend; absence stays `skipped` while the upstream `vst3` crate backend is active.",
        "- `vst3-sdk/generated-bindings-plan.json`: optional generated-bindings readiness plan generated by `vesty vst3-sdk binding-plan`. When present, `release-check` verifies the locked SDK header manifest, `.rs` output module path, active backend baseline and reserved binding emitter check. This does not claim full SDK 3.8 bindings are generated; `bindingsGenerated` must remain false until the emitter exists.",
        "- `vst3-sdk/generated-bindings-surface.json`: optional generated-bindings symbol surface generated by `vesty vst3-sdk binding-surface`. When present, `release-check` verifies the locked SDK header manifest, required symbol/header surface, active backend baseline and audit notes. This locks the expected generated-headers surface only; `bindingsGenerated` must remain false and no C++ AST parsing, ABI verification or Rust bindings generation is implied.",
        "- `vst3-sdk/generated.rs`: optional metadata-only scaffold generated by `vesty vst3-sdk emit-scaffold` and copied by `import-ci` when valid. It fixes generator/header/baseline metadata for drift checks and must keep `BINDINGS_GENERATED = false`; when present, `release-check` validates its scaffold markers, but it is not proof that full SDK bindings are generated.",
        "- `vst3-sdk/generated-abi-seed.rs`: optional ABI seed generated by `vesty vst3-sdk emit-abi-seed` and copied by `import-ci` when valid. It fixes basic VST3 ABI aliases/constants such as `TResult`, `ParamID`, `ParamValue`, `TChar`, `TUID`, result constants and platform type strings, while keeping `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; when present, `release-check` validates its ABI seed markers, but it is not proof that full SDK bindings are generated.",
        "- `vst3-sdk/generated-abi.rs`: optional foundational ABI layout module generated by `vesty vst3-sdk emit-abi` and copied by `import-ci` when valid. It fixes deterministic `repr(C)` layouts such as `TUID`, `FUnknownVTable`, `FUnknown`, `ViewRect`, `ProgramListInfo`, `UnitInfo`, `NoteExpressionValueDescription`, `NoteExpressionTypeInfo`, `PhysicalUIMap` and `PhysicalUIMapList`, plus basic aliases/constants and layout size/alignment/field-offset fingerprints, while keeping `ABI_LAYOUT_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; when present, `release-check` validates its ABI layout markers, but it is not proof that full SDK bindings or complete ABI verification exist.",
        "- `vst3-sdk/generated-interface-skeleton.rs`: optional interface/vtable skeleton module generated by `vesty vst3-sdk emit-interface-skeleton` and copied by `import-ci` when valid. It fixes deterministic `repr(C)` placeholders plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata for discovered VST3 interfaces such as `IPlugViewVTable`, `IEditControllerVTable`, `IMidiMappingVTable`, `IUnitInfoVTable`, `IProgramListDataVTable` and `INoteExpressionControllerVTable`, while keeping `INTERFACE_SKELETON_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`; its method order entries are audit-only per-interface order numbers, its vtable slot seeds only fix future emitter local slot, field, callback type alias names, signature intent, repr(C) callback field layout and field offset fingerprints, its IID/queryInterface entries fix upstream IID words, a future interface dispatch plan and pure lookup helpers such as `interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()`, its `COM_OBJECT_INTERFACES` entries only fix the current Vesty adapter object-to-interface exposure plan, its `COM_OBJECT_IDENTITY_PLANS` / `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` entries only fix per-object FUnknown identity and dispatch behavior, its `FACTORY_EXPORT_PLAN` / `FACTORY_CLASS_PLANS` entries only fix the current `VestyFactory` class-count, category, CID derivation and `createInstance` dispatch/error policy, its `MODULE_EXPORT_PLANS` entries only fix the current `export_vst3!` platform entry symbol plan, and its `BINARY_EXPORT_SYMBOL_PLANS` / `BINARY_EXPORT_INSPECTION_TOOL_PLANS` entries plus `binary_export_symbol_plan_by_platform_and_symbol()` / `binary_export_inspection_tools()` / `required_binary_export_symbol_count()` / `first_missing_binary_export_symbol()` / `binary_export_required_symbols_present()` helpers fix expected per-platform export names/tool spellings, inspection tool order and pure required-symbol checks. When present, `release-check` validates its interface skeleton markers and metadata, but it does not emit callable `queryInterface` glue, factory glue, generated factory exports, generated module exports, binary inspection tooling, Steinberg method implementations or full COM bindings.",
        "- `validator/`: recommended directory for validator-passed release reports and logs. `release-check --plan` writes suggested `vesty validate --strict` commands to this directory, and `release-check --release-evidence-dir` auto-discovers valid validator-passed JSON reports recursively.",
        "- `validate-report.json`: legacy/single-plugin pending template; replace it with `vesty validate --strict --report` output from a validator-passed run when you are collecting one report manually. Framework release matrices should prefer `validator/<bundle>.<platform>.validate.json` plus matching validator logs.",
        "- Vesty framework release gates expect Steinberg validator-passed reports for `VestyGain.vst3`, `VestyWebUIDemo.vst3` and `VestyMIDISynth.vst3` on `linux-x64`, `macos` and `windows-x64`. Per-OS validator jobs may provide one platform at a time; `--require-release-artifacts` requires the full 3x3 validator matrix. Each example/platform report must include `static_check.parameter_manifest` pointing at that bundle's `Contents/Resources/parameters.manifest.json`; `VestyWebUIDemo.vst3` must also include `static_check.asset_manifest` pointing at `Contents/Resources/assets.manifest.json` and a nonzero `asset_count`. In the final strict gate, each example/platform report must also include an `ok` `static_check.binary_exports` entry for the matching platform, proving `GetPluginFactory` plus platform entry/exit exports were observed by `vesty validate`; generate release validator reports with `vesty validate --strict` so missing/skipped export-symbol evidence fails at collection time.",
        "- `package/`: recommended directory for CI package/static validate reports. `release-check --plan` writes suggested `vesty validate --static-only --strict` commands to this directory, and `release-check --release-evidence-dir` auto-discovers valid static-only JSON reports recursively.",
        "- `static-validate-report.json`: legacy/single-plugin pending template; replace it with `vesty validate --static-only --strict --report` output from CI packaging smoke when you are collecting one report manually. Framework release matrices should prefer `package/<bundle>.<platform>.static-validate.json`. This does not replace validator-passed release evidence.",
        "- Vesty framework release gates expect CI static validate reports for `VestyGain.vst3`, `VestyWebUIDemo.vst3` and `VestyMIDISynth.vst3` on `linux-x64`, `macos` and `windows-x64`. Per-OS package jobs may provide one platform at a time; `--require-release-artifacts` requires the full 3x3 matrix. Each example/platform report must include `static_check.parameter_manifest` pointing at that bundle's `Contents/Resources/parameters.manifest.json`; `VestyWebUIDemo.vst3` must also include UI asset manifest evidence pointing at `Contents/Resources/assets.manifest.json`. In the final strict gate, each example/platform static report must also include an `ok` `static_check.binary_exports` entry for the matching platform.",
        "- `ci-doctor/`: optional conventional directory for downloaded `doctor-Linux.json`, `doctor-macOS.json` and `doctor-Windows.json` artifacts. OS coverage may also be inferred from parent directories such as `Linux/doctor.json`, `macOS/doctor.json` and `Windows/doctor.json`. Each artifact must include toolchain, Node/npm, VST3 binding baseline, validator, system WebView and signing/notarization preflight checks. The template README alone does not count.",
        "- `signing-macos.log`: paste `codesign --verify --deep --strict --verbose=2` or equivalent positive signing evidence. Additional nested `.log`/`.txt` files and signed macOS `.vst3` bundles are auto-discovered by content.",
        "- `signing-windows.log`: paste `signtool verify /pa /v` or equivalent positive signing evidence. Additional nested `.log`/`.txt` files are auto-discovered by content.",
        "- `--require-release-artifacts` requires both macOS codesign and Windows signtool coverage. Generic `signed=true` / `signature=ok` markers are rejected because they do not prove either platform by themselves.",
        "- `notary.log`: paste accepted `xcrun notarytool submit --wait` and stapler output. Additional nested `.log`/`.txt` files are auto-discovered by content.",
        "- `--require-release-artifacts` requires both accepted notarytool output and stapler success evidence. Generic `notarization=pass` / `notary=ok` markers are rejected because they do not prove notarytool accepted status.",
        "",
        "Accepted release artifact markers:",
        "",
        "Marker lines are parsed as exact `key=value` or `key: value` pairs; pending, false and instructional text do not count.",
        "Positive signing/notarization markers do not override explicit failure evidence in the same log: invalid signatures, nonzero signtool error counts, rejected/invalid notary statuses and stapler failures are rejected.",
        "",
        "```text",
        "codesign=pass",
        "signtool=pass",
        "signtool ... Number of errors: 0",
        "notarytool=pass",
        "stapled=true",
        "status: Accepted",
        "The staple and validate action worked!",
        "```",
        "",
        "DAW smoke evidence is kept separately under `target/daw-evidence/<host>` by convention; generate it with `vesty daw-matrix --write-template --evidence-root target/daw-evidence` and pass the same root to `vesty release-check --evidence-root target/daw-evidence`.",
        "",
    ]
    .join("\n")
}

fn vst3_sdk_manifest_evidence_readme() -> String {
    [
        "# VST3 SDK Generated Bindings Evidence",
        "",
        "Place `vst3-sdk-headers.json` here when auditing the official Steinberg VST3 SDK header inputs reserved for generated bindings. Place `generated-bindings-plan.json` and `generated-bindings-surface.json` here when auditing generated-bindings readiness and expected symbol surface without claiming generated bindings are complete. `generated.rs`, `generated-abi-seed.rs`, `generated-abi.rs` and `generated-interface-skeleton.rs` are optional drift/audit modules; `release-check` validates them when present, but they are not proof that complete SDK bindings or final release readiness exist.",
        "",
        "Expected header manifest commands:",
        "",
        "```bash",
        "vesty vst3-sdk manifest \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/vst3-sdk-headers.json",
        "vesty vst3-sdk manifest \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/vst3-sdk-headers.json \\",
        "  --check",
        "```",
        "",
        "Expected generated-bindings plan commands:",
        "",
        "```bash",
        "vesty vst3-sdk binding-plan \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --bindings-module target/vst3-sdk/generated.rs \\",
        "  --out vst3-sdk/generated-bindings-plan.json",
        "vesty vst3-sdk binding-plan \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --bindings-module target/vst3-sdk/generated.rs \\",
        "  --out vst3-sdk/generated-bindings-plan.json \\",
        "  --check",
        "vesty vst3-sdk binding-surface \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/generated-bindings-surface.json",
        "vesty vst3-sdk binding-surface \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out vst3-sdk/generated-bindings-surface.json \\",
        "  --check",
        "vesty vst3-sdk emit-scaffold \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated.rs",
        "vesty vst3-sdk emit-scaffold \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated.rs \\",
        "  --check",
        "vesty vst3-sdk emit-abi-seed \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi-seed.rs",
        "vesty vst3-sdk emit-abi-seed \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi-seed.rs \\",
        "  --check",
        "vesty vst3-sdk emit-abi \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi.rs",
        "vesty vst3-sdk emit-abi \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-abi.rs \\",
        "  --check",
        "vesty vst3-sdk emit-interface-skeleton \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-interface-skeleton.rs",
        "vesty vst3-sdk emit-interface-skeleton \\",
        "  --sdk-dir /path/to/VST_SDK \\",
        "  --out target/vst3-sdk/generated-interface-skeleton.rs \\",
        "  --check",
        "```",
        "",
        "`release-check` treats the manifest, plan, surface and generated Rust audit modules as optional because the active MVP backend still uses the upstream `vst3` crate. If any JSON file or `.rs` audit file exists in this directory, it must be complete and match the expected Vesty generator, baselines, markers and required header/symbol/interface surface. The generated-bindings plan and surface must report `bindingsGenerated: false`; the surface locks expected symbols for a future emitter, but does not parse C++ ASTs, verify ABI, or generate Rust bindings. `emit-scaffold` writes a deterministic metadata-only Rust module at the planned `.rs` path so CI can check output drift, and `import-ci` can copy that module to `vst3-sdk/generated.rs` after validating its scaffold markers. `emit-abi-seed` writes a deterministic ABI seed module with basic aliases/constants and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-abi-seed.rs` after validation. `emit-abi` writes a deterministic foundational ABI layout module with `repr(C)` `TUID`, `FUnknownVTable`, `FUnknown`, `ViewRect`, `ProgramListInfo`, `UnitInfo`, `NoteExpressionValueDescription`, `NoteExpressionTypeInfo`, `PhysicalUIMap`, `PhysicalUIMapList`, aliases/constants, `ABI_LAYOUT_RECORDS` size/alignment fingerprints, `ABI_FIELD_OFFSETS` field-offset fingerprints, `ABI_LAYOUT_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-abi.rs` after validation. `emit-interface-skeleton` writes a deterministic interface/vtable skeleton module with `repr(C)` interface placeholders, method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan audit metadata, pure IID/dispatch lookup helpers and pure binary export required-symbol / inspection-tool helpers, including `QUERY_INTERFACE_IID_LOOKUP_SCOPE`, `interface_id_for_iid()`, `query_interface_entry_by_interface()`, `query_interface_entry_for_iid()`, `com_object_query_interface_dispatch_by_interface()`, `com_object_query_interface_dispatch_for_iid()`, `binary_export_symbol_plan_by_platform_and_symbol()`, `binary_export_inspection_tools()`, `required_binary_export_symbol_count()`, `first_missing_binary_export_symbol()`, `binary_export_required_symbols_present()`, `COM_OBJECT_INTERFACES`, `COM_OBJECT_IDENTITY_PLANS`, `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`, `FACTORY_EXPORT_PLAN`, `FACTORY_CLASS_PLANS`, `MODULE_EXPORT_PLANS`, `BINARY_EXPORT_SYMBOL_PLANS`, `BINARY_EXPORT_INSPECTION_TOOL_PLANS`, `BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED = true`, `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`, `INTERFACE_SKELETON_GENERATED = true`, `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`, and `import-ci` can copy it to `vst3-sdk/generated-interface-skeleton.rs` after validation. None of these modules generate callable Steinberg VST3 COM/API method implementations, generated factory exports, generated module exports, binary inspection tooling or full COM bindings.",
        "",
        "This README is only a placeholder. It does not count as VST3 SDK header manifest, generated bindings plan, generated bindings surface, generated scaffold, ABI seed, ABI layout, or interface skeleton evidence.",
        "",
    ]
    .join("\n")
}

fn publish_plan_evidence_readme() -> String {
    [
        "# Publish Plan Evidence",
        "",
        "Place the `publish-plan.json` file from the `vesty-publish-plan` CI artifact here.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty publish-plan --out publish-plan/publish-plan.json",
        "```",
        "",
        "Use `vesty publish-plan --check --out publish-plan/publish-plan.json` to verify an existing report.",
        "",
        "This README is only a placeholder. It does not count as publish-plan evidence.",
        "",
    ]
    .join("\n")
}

fn crate_package_evidence_readme() -> String {
    [
        "# Crate Package Readiness Evidence",
        "",
        "Place the `crate-package.json` file from the `vesty-crate-package` CI artifact here.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty crate-package --out crate-package/crate-package.json",
        "```",
        "",
        "Use `vesty crate-package --check --out crate-package/crate-package.json` to verify an existing report.",
        "",
        "`vesty crate-package` runs `cargo package -p <crate> --allow-dirty --no-verify` for workspace crates that have no internal dependencies. Crates that still depend on unpublished Vesty workspace crates are recorded as `deferred` until those dependencies are published in dependency order.",
        "",
        "This README is only a placeholder. It does not count as crate package readiness evidence.",
        "",
    ]
    .join("\n")
}

fn npm_pack_evidence_readme() -> String {
    [
        "# NPM Pack Evidence",
        "",
        "Place the npm pack dry-run JSON artifact here after building the JS packages.",
        "",
        "Expected command:",
        "",
        "```bash",
        "vesty npm-pack --out npm-pack/npm-pack.json",
        "```",
        "",
        "`vesty npm-pack` runs `npm pack --workspaces --dry-run --json` and then applies the same validation used by `release-check`: the report must include `vesty-plugin-ui`, with packed files limited to `dist/**` and `package.json`.",
        "",
        "This README is only a placeholder. It does not count as npm pack evidence.",
        "",
    ]
    .join("\n")
}

fn dependency_baseline_evidence_readme() -> String {
    [
        "# Dependency Baseline Evidence",
        "",
        "Place `dependency-baseline-latest.json` here after an explicit online dependency latest review.",
        "",
        "Expected commands:",
        "",
        "```bash",
        "vesty dependency-baseline --latest \\",
        "  --out dependency-baseline/dependency-baseline-latest.json",
        "vesty dependency-baseline --latest \\",
        "  --check \\",
        "  --out dependency-baseline/dependency-baseline-latest.json",
        "```",
        "",
        "`dependency-baseline-latest.json` must include the `cargo workspace external dependency baseline coverage` check plus crates.io and npm registry latest checks. A plain `dependency-baseline.json` generated without `--latest` is useful CI drift evidence, but it does not prove release-time latest dependency review.",
        "",
        "This README is only a placeholder. It does not count as dependency latest baseline evidence.",
        "",
    ]
    .join("\n")
}

fn ci_doctor_readme() -> String {
    [
        "# CI Doctor Artifacts",
        "",
        "Place downloaded GitHub Actions doctor artifacts here after a real CI run.",
        "",
        "Recommended flat files:",
        "",
        "- `doctor-Linux.json`",
        "- `doctor-macOS.json`",
        "- `doctor-Windows.json`",
        "",
        "Directory-grouped downloads are also accepted because OS coverage is inferred from the full artifact path:",
        "",
        "- `Linux/doctor.json`",
        "- `macOS/doctor.json`",
        "- `Windows/doctor.json`",
        "",
        "Each artifact should be generated with `vesty doctor --format json` and include `vst3 binding baseline`, toolchain, Node/npm, validator, system WebView and signing/notarization preflight checks.",
        "",
        "This README is only a placeholder. It does not count as CI doctor evidence.",
        "",
    ]
    .join("\n")
}

fn ci_release_checks_readme() -> String {
    [
        "# CI Release-Check Artifacts",
        "",
        "Place downloaded per-OS `release-check-*` GitHub Actions artifacts here after a real CI run.",
        "",
        "Recommended flat files:",
        "",
        "- `release-check-Linux.json`",
        "- `release-check-macOS.json`",
        "- `release-check-Windows.json`",
        "",
        "Directory-grouped downloads are also accepted because OS coverage is inferred from the full artifact path:",
        "",
        "- `Linux/release-check.json`",
        "- `macOS/release-check.json`",
        "- `Windows/release-check.json`",
        "",
        "Optional checklist sidecars preserved by `release-evidence import-ci`:",
        "",
        "- `release-action-plan-Linux.json`",
        "- `release-action-plan-macOS.json`",
        "- `release-action-plan-Windows.json`",
        "",
        "Each artifact should be generated by `vesty release-check --skip-protocol --ci-run-url <run-url> --format json --report ...` on its own runner. The gate checks that local invariants such as host profile coverage and the VST3 binding baseline passed on all three OSes, and that `ci_run_url` matches `ci-run-url.txt` when an expected run URL is provided.",
        "",
        "Action plan sidecars are content-validated checklist metadata for human follow-up. `--ci-release-check-dir` only reads `release-check*.json` case-insensitively; it ignores `release-action-plan-*.json` and never treats them as pass evidence.",
        "",
        "`--skip-protocol` is only for these per-OS CI snapshots; the final `vesty release-check --strict --require-release-artifacts` command must check the protocol snapshot and will fail if `--skip-protocol` is used.",
        "",
        "These snapshots do not replace DAW smoke, validator, signing or notarization evidence; those are checked by separate release gates.",
        "",
        "This README is only a placeholder. It does not count as CI release-check evidence.",
        "",
    ]
    .join("\n")
}

fn platform_smoke_evidence_readme() -> String {
    [
        "# Platform Smoke Evidence",
        "",
        "Place real platform smoke JSON reports here after running Vesty on each release platform.",
        "",
        "Expected coverage:",
        "",
        "- `macos`",
        "- `windows-x64`",
        "- `linux-x11`",
        "",
        "Each report must prove system WebView availability, Steinberg validator execution, VST3 example scan, WebView attach/resize, manifest-backed asset protocol, JSBridge roundtrip and a nonzero meter stream.",
        "",
        "System WebView evidence is platform-specific: macOS must mention WebKit.framework or WKWebView, Windows x64 must mention WebView2, and Linux X11 must mention both WebKitGTK and active X11 evidence without Wayland/fallback wording. Steinberg validator evidence must identify Steinberg or VST3 validator output and include passed tests with zero failures.",
        "",
        "Generate starter templates with:",
        "",
        "```bash",
        "vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke",
        "```",
        "",
        "After a real host/platform run, write a normalized report from explicit evidence markers:",
        "",
        "```bash",
        "vesty platform-smoke --write-report --dir target/release-evidence/platform-smoke \\",
        "  --platform macos \\",
        "  --host \"REAPER 7.73\" \\",
        "  --system-webview \"WebKit.framework loaded\" \\",
        "  --vst3-validator \"Steinberg validator passed 47 tests, 0 failed\" \\",
        "  --vst3-example-scan \"VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\" \\",
        "  --webview-attach \"webview_attach=true\" \\",
        "  --webview-resize \"webview_resize=true width=640 height=420\" \\",
        "  --asset-protocol \"asset_protocol=true assets.manifest.json served\" \\",
        "  --jsbridge-roundtrip \"jsbridge_roundtrip=true readyAck reply\" \\",
        "  --meter-stream \"meter_flush sent=3\"",
        "```",
        "",
        "Then run:",
        "",
        "```bash",
        "vesty platform-smoke --dir target/release-evidence/platform-smoke --strict",
        "```",
        "",
        "This README and pending JSON values do not count as platform smoke evidence. Linux Wayland is experimental and does not satisfy the Linux X11 release gate.",
        "",
    ]
    .join("\n")
}

fn pending_validate_report_template() -> Result<String, serde_json::Error> {
    let report = ValidateReport {
        bundle: "replace-with-bundle.vst3".to_string(),
        static_check: StaticBundleCheck {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(
                "pending: replace with `vesty validate --strict --report` output".to_string(),
            ),
        },
        validator: ValidatorCheck {
            status: "not_run".to_string(),
            path: None,
            exit_code: None,
            tests_passed: None,
            tests_failed: None,
            stdout: None,
            stderr: None,
            reason: Some("pending real Steinberg validator run".to_string()),
            error: None,
        },
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

fn pending_static_validate_report_template() -> Result<String, serde_json::Error> {
    let report = ValidateReport {
        bundle: "replace-with-bundle.vst3".to_string(),
        static_check: StaticBundleCheck {
            status: "failed".to_string(),
            moduleinfo: None,
            binaries: Vec::new(),
            binary_exports: Vec::new(),
            parameter_manifest: None,
            asset_manifest: None,
            asset_count: 0,
            error: Some(
                "pending: replace with `vesty validate --static-only --strict --report` output"
                    .to_string(),
            ),
        },
        validator: ValidatorCheck::skipped("--static-only pending template"),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn existing_directory_no_parent_or_leaf_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    reject_existing_output_parent_symlink(label, path)?;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    if !metadata.is_dir() {
        return Err(format!("{label} must be a directory: {path}").into());
    }
    Ok(true)
}

fn create_directory_no_parent_or_leaf_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if existing_directory_no_parent_or_leaf_symlink(label, path)? {
        return Ok(());
    }
    fs::create_dir_all(path)?;
    if existing_directory_no_parent_or_leaf_symlink(label, path)? {
        Ok(())
    } else {
        Err(format!("{label} was not created: {path}").into())
    }
}

fn create_template_dir(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(
                    format!("template output directory must not be a symlink: {path}").into(),
                );
            }
            if !metadata.is_dir() {
                return Err(
                    format!("template output directory must be a directory: {path}").into(),
                );
            }
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    create_directory_no_parent_or_leaf_symlink("template output directory", path)
}

fn write_template_file(
    path: Utf8PathBuf,
    contents: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!("template output file must not be a symlink: {path}").into());
            }
            return Ok(0);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    write_text_file(&path, contents)?;
    Ok(1)
}

fn write_platform_smoke_templates(dir: &Utf8PathBuf) -> Result<usize, Box<dyn std::error::Error>> {
    create_template_dir(dir)?;
    let mut created = 0;
    created += write_template_file(dir.join("README.md"), &platform_smoke_evidence_readme())?;
    for (platform, label) in REQUIRED_PLATFORM_SMOKE_PLATFORMS {
        let report = pending_platform_smoke_report(platform, label)?;
        created += write_template_file(dir.join(format!("{platform}.json")), &report)?;
    }
    Ok(created)
}

fn pending_platform_smoke_report(platform: &str, label: &str) -> Result<String, serde_json::Error> {
    let report = PlatformSmokeReport {
        platform: platform.to_string(),
        os: Some(label.to_string()),
        host: Some("pending real host/platform run".to_string()),
        checks: REQUIRED_PLATFORM_SMOKE_CHECKS
            .into_iter()
            .map(|(name, label)| PlatformSmokeCheck {
                name: name.to_string(),
                status: "pending".to_string(),
                value: format!("replace with real {label} evidence"),
                hint: Some("pending template values do not count as pass evidence".to_string()),
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

fn print_single_release_check_item(
    item: &ReleaseCheckItem,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(item)?),
        "markdown" | "md" => {
            println!("| Check | Status | Value | Hint |");
            println!("| --- | --- | --- | --- |");
            println!(
                "| {} | {} | {} | {} |",
                item.name,
                item.status,
                markdown_cell(&item.value),
                markdown_cell(item.hint.as_deref().unwrap_or(""))
            );
        }
        "text" | "plain" => {
            println!("{}: {} ({})", item.name, item.status, item.value);
            if let Some(hint) = item.hint.as_deref() {
                println!("hint: {hint}");
            }
        }
        _ => return Err(format!("unsupported platform smoke format '{format}'").into()),
    }
    Ok(())
}

fn print_daw_matrix(
    rows: &[serde_json::Value],
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(rows)?),
        "markdown" | "md" => {
            println!(
                "| Host | Platform | Scan | Load | UI | UI->Host | Meter Stream | Automation | Buffer/Sample Rate | Save/Restore | Offline Render | Evidence |"
            );
            println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
            for row in rows {
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    row["host"].as_str().unwrap_or(""),
                    daw_platform_text(row),
                    status_text(&row["scan"]),
                    status_text(&row["load"]),
                    status_text(&row["ui"]),
                    status_text(&row["ui_host_param"]),
                    status_text(&row["meter_stream"]),
                    status_text(&row["automation"]),
                    status_text(&row["buffer_sample_rate_change"]),
                    status_text(&row["save_restore"]),
                    status_text(&row["offline_render"]),
                    row["evidence"].as_str().unwrap_or("")
                );
            }
        }
        _ => return Err(format!("unsupported matrix format '{format}'").into()),
    }
    Ok(())
}

fn daw_platform_text(row: &serde_json::Value) -> String {
    let platform = row["platform"].as_str().unwrap_or("").trim();
    match row["platform_supported"].as_bool() {
        Some(true) => platform.to_string(),
        Some(false) if platform.is_empty() => "missing (unsupported)".to_string(),
        Some(false) => format!("{platform} (unsupported)"),
        None if platform.is_empty() => "unknown".to_string(),
        None => format!("{platform} (unknown)"),
    }
}

fn daw_matrix_complete(rows: &[serde_json::Value]) -> bool {
    !rows.is_empty() && rows.iter().all(daw_row_complete)
}

fn daw_row_complete(row: &serde_json::Value) -> bool {
    if row["platform_supported"].as_bool() != Some(true) {
        return false;
    }
    [
        "scan",
        "load",
        "ui",
        "ui_host_param",
        "meter_stream",
        "automation",
        "buffer_sample_rate_change",
        "save_restore",
        "offline_render",
    ]
    .iter()
    .all(|key| row[*key].as_bool() == Some(true))
}

fn daw_missing_checks(row: &serde_json::Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if row["platform_supported"].as_bool() != Some(true) {
        missing.push("platform");
    }
    missing.extend(
        [
            "scan",
            "load",
            "ui",
            "ui_host_param",
            "meter_stream",
            "automation",
            "buffer_sample_rate_change",
            "save_restore",
            "offline_render",
        ]
        .into_iter()
        .filter(|key| row[*key].as_bool() != Some(true)),
    );
    missing
}

fn status_text(value: &serde_json::Value) -> &'static str {
    match value.as_bool() {
        Some(true) => "pass",
        Some(false) => "missing",
        None => "unknown",
    }
}

fn daw_matrix_rows(evidence: &DawEvidencePaths) -> Vec<serde_json::Value> {
    vec![
        collect_daw_evidence_for_host("reaper", &evidence.reaper),
        collect_daw_evidence_for_host("cubase-nuendo", &evidence.cubase),
        collect_daw_evidence_for_host("bitwig", &evidence.bitwig),
        collect_daw_evidence_for_host("ableton-live", &evidence.ableton),
        collect_daw_evidence_for_host("studio-one", &evidence.studio_one),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DawEvidenceDirStatus {
    Present,
    Missing,
    Blocked,
}

fn daw_evidence_dir_status(dir: &Utf8Path) -> DawEvidenceDirStatus {
    match existing_directory_no_parent_or_leaf_symlink("DAW evidence directory", dir) {
        Ok(true) => DawEvidenceDirStatus::Present,
        Ok(false) => DawEvidenceDirStatus::Missing,
        Err(_) => DawEvidenceDirStatus::Blocked,
    }
}

fn collect_daw_evidence_for_profile(
    profile: &vesty_core::HostProfile,
    dir: &Utf8PathBuf,
) -> serde_json::Value {
    if profile.id == "reaper" {
        collect_reaper_evidence_for_profile(profile, dir)
    } else {
        collect_generic_daw_evidence_for_profile(profile, dir)
    }
}

fn collect_daw_evidence_for_host(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return missing_daw_row(host, dir);
    };
    collect_daw_evidence_for_profile(profile, dir)
}

fn print_host_quirks(host: Option<&str>, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = selected_host_profiles(host)?;
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&profiles)?);
        }
        "markdown" | "md" => {
            println!(
                "| Host | Platforms | Required Smoke | Area | Severity | Summary | Mitigation |"
            );
            println!("| --- | --- | --- | --- | --- | --- | --- |");
            for profile in &profiles {
                let platforms = profile.platforms.join(", ");
                let required = profile.required_smoke_checks.join(", ");
                for quirk in profile.quirks {
                    println!(
                        "| {} | {} | {} | {} | {} | {} | {} |",
                        profile.name,
                        platforms,
                        required,
                        host_quirk_area_text(quirk.area),
                        host_quirk_severity_text(quirk.severity),
                        quirk.summary,
                        quirk.mitigation
                    );
                }
            }
            println!();
            println!(
                "Install detection and host quirk notes do not prove compatibility; release still requires real DAW smoke evidence."
            );
        }
        _ => return Err(format!("unsupported host quirk format '{format}'").into()),
    }
    Ok(())
}

fn host_quirk_area_text(area: vesty_core::HostQuirkArea) -> &'static str {
    match area {
        vesty_core::HostQuirkArea::Scanning => "scanning",
        vesty_core::HostQuirkArea::Editor => "editor",
        vesty_core::HostQuirkArea::Automation => "automation",
        vesty_core::HostQuirkArea::State => "state",
        vesty_core::HostQuirkArea::Render => "render",
        vesty_core::HostQuirkArea::Meter => "meter",
        vesty_core::HostQuirkArea::Platform => "platform",
        vesty_core::HostQuirkArea::Packaging => "packaging",
    }
}

fn host_quirk_severity_text(severity: vesty_core::HostQuirkSeverity) -> &'static str {
    match severity {
        vesty_core::HostQuirkSeverity::Info => "info",
        vesty_core::HostQuirkSeverity::Warning => "warning",
        vesty_core::HostQuirkSeverity::Required => "required",
    }
}

fn selected_host_profiles(
    host: Option<&str>,
) -> Result<Vec<&'static vesty_core::HostProfile>, Box<dyn std::error::Error>> {
    match host {
        Some(host) => vesty_core::find_host_profile(host)
            .map(|profile| vec![profile])
            .ok_or_else(|| format!("unknown host profile '{host}'").into()),
        None => Ok(vesty_core::host_profiles().iter().collect()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReleaseCheckReport {
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ci_run_url: Option<String>,
    checks: Vec<ReleaseCheckItem>,
    daw_matrix: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReleaseCheckItem {
    name: String,
    status: String,
    value: String,
    hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReleaseActionPlan {
    version: u32,
    status: String,
    summary: ReleaseActionPlanSummary,
    protocol_snapshot: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release_evidence_dir: Option<String>,
    actions: Vec<ReleaseActionItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReleaseActionPlanSummary {
    ok: usize,
    failed: usize,
    skipped: usize,
    action_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReleaseActionItem {
    check: String,
    status: String,
    priority: String,
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence_path: Option<String>,
    commands: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReleaseEvidenceOptions {
    ci_doctor_dir: Option<Utf8PathBuf>,
    ci_release_check_dir: Option<Utf8PathBuf>,
    platform_smoke_dir: Option<Utf8PathBuf>,
    ci_run_url: Option<String>,
    validate_reports: Vec<Utf8PathBuf>,
    static_validate_reports: Vec<Utf8PathBuf>,
    publish_plan_report: Option<Utf8PathBuf>,
    crate_package_report: Option<Utf8PathBuf>,
    npm_pack_report: Option<Utf8PathBuf>,
    dependency_baseline_report: Option<Utf8PathBuf>,
    vst3_sdk_manifest: Option<Utf8PathBuf>,
    vst3_sdk_binding_plan: Option<Utf8PathBuf>,
    vst3_sdk_binding_surface: Option<Utf8PathBuf>,
    vst3_sdk_scaffold: Option<Utf8PathBuf>,
    vst3_sdk_abi_seed: Option<Utf8PathBuf>,
    vst3_sdk_abi: Option<Utf8PathBuf>,
    vst3_sdk_interface_skeleton: Option<Utf8PathBuf>,
    signed_bundle_evidence: Vec<Utf8PathBuf>,
    notarization_log: Option<Utf8PathBuf>,
    require_release_artifacts: bool,
}

fn build_release_check_report(
    rows: Vec<serde_json::Value>,
    protocol_snapshot: &Utf8Path,
    skip_protocol: bool,
    release_evidence: &ReleaseEvidenceOptions,
) -> ReleaseCheckReport {
    let mut checks = Vec::new();
    checks.push(host_profile_release_check(&rows));
    checks.push(daw_matrix_release_check(&rows));
    checks.extend(rows.iter().map(host_row_release_check));
    checks.push(protocol_release_check(
        protocol_snapshot,
        skip_protocol,
        release_evidence.require_release_artifacts,
    ));
    checks.push(binding_baseline_release_check());
    checks.push(ci_run_url_release_check(
        release_evidence.ci_run_url.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(ci_doctor_artifacts_release_check(
        release_evidence.ci_doctor_dir.as_deref(),
        release_evidence.require_release_artifacts,
        release_evidence.ci_run_url.as_deref(),
    ));
    checks.push(ci_release_check_artifacts_release_check(
        release_evidence.ci_release_check_dir.as_deref(),
        release_evidence.require_release_artifacts,
        release_evidence.ci_run_url.as_deref(),
    ));
    checks.push(platform_smoke_release_check(
        release_evidence.platform_smoke_dir.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(validate_reports_release_check(
        &release_evidence.validate_reports,
        release_evidence.require_release_artifacts,
    ));
    checks.push(example_validate_coverage_release_check(
        &release_evidence.validate_reports,
        release_evidence.require_release_artifacts,
    ));
    checks.push(static_validate_reports_release_check(
        &release_evidence.static_validate_reports,
        release_evidence.require_release_artifacts,
    ));
    checks.push(example_static_validate_coverage_release_check(
        &release_evidence.static_validate_reports,
        release_evidence.require_release_artifacts,
    ));
    checks.push(publish_plan_release_check(
        release_evidence.publish_plan_report.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(crate_package_release_check(
        release_evidence.crate_package_report.as_deref(),
        release_evidence.publish_plan_report.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(npm_pack_release_check(
        release_evidence.npm_pack_report.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(dependency_baseline_latest_release_check(
        release_evidence.dependency_baseline_report.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    checks.push(vst3_sdk_manifest_release_check(
        release_evidence.vst3_sdk_manifest.as_deref(),
    ));
    checks.push(vst3_sdk_binding_plan_release_check(
        release_evidence.vst3_sdk_binding_plan.as_deref(),
    ));
    checks.push(vst3_sdk_binding_surface_release_check(
        release_evidence.vst3_sdk_binding_surface.as_deref(),
    ));
    checks.push(vst3_sdk_generated_scaffold_release_check(
        release_evidence.vst3_sdk_scaffold.as_deref(),
    ));
    checks.push(vst3_sdk_generated_abi_seed_release_check(
        release_evidence.vst3_sdk_abi_seed.as_deref(),
    ));
    checks.push(vst3_sdk_generated_abi_release_check(
        release_evidence.vst3_sdk_abi.as_deref(),
    ));
    checks.push(vst3_sdk_generated_interface_skeleton_release_check(
        release_evidence.vst3_sdk_interface_skeleton.as_deref(),
    ));
    checks.push(signed_bundle_evidence_release_check(
        &release_evidence.signed_bundle_evidence,
        release_evidence.require_release_artifacts,
    ));
    checks.push(notarization_log_release_check(
        release_evidence.notarization_log.as_deref(),
        release_evidence.require_release_artifacts,
    ));
    let complete = checks
        .iter()
        .all(|check| matches!(check.status.as_str(), "ok" | "skipped"));
    ReleaseCheckReport {
        status: if complete { "ok" } else { "failed" }.to_string(),
        os: current_release_check_os_label().map(str::to_string),
        ci_run_url: release_evidence.ci_run_url.clone(),
        checks,
        daw_matrix: rows,
    }
}

fn current_release_check_os_label() -> Option<&'static str> {
    match doctor_os_label() {
        "Linux" => Some("Linux"),
        "macOS" => Some("macOS"),
        "Windows" => Some("Windows"),
        _ => None,
    }
}

fn release_check_complete(report: &ReleaseCheckReport) -> bool {
    report.status == "ok"
}

fn build_release_action_plan(
    report: &ReleaseCheckReport,
    protocol_snapshot: &Utf8Path,
    evidence_root: Option<&Utf8Path>,
    release_evidence_dir: Option<&Utf8Path>,
) -> ReleaseActionPlan {
    let ok = report
        .checks
        .iter()
        .filter(|check| check.status == "ok")
        .count();
    let failed = report
        .checks
        .iter()
        .filter(|check| check.status == "failed")
        .count();
    let skipped = report
        .checks
        .iter()
        .filter(|check| check.status == "skipped")
        .count();
    let actions = report
        .checks
        .iter()
        .filter(|check| check.status != "ok")
        .map(|check| {
            release_action_for_check(
                check,
                report,
                protocol_snapshot,
                evidence_root,
                release_evidence_dir,
            )
        })
        .collect::<Vec<_>>();

    ReleaseActionPlan {
        version: 1,
        status: report.status.clone(),
        summary: ReleaseActionPlanSummary {
            ok,
            failed,
            skipped,
            action_count: actions.len(),
        },
        protocol_snapshot: portable_report_path(protocol_snapshot),
        evidence_root: evidence_root.map(portable_report_path),
        release_evidence_dir: release_evidence_dir.map(portable_report_path),
        actions,
    }
}

fn release_action_for_check(
    check: &ReleaseCheckItem,
    report: &ReleaseCheckReport,
    protocol_snapshot: &Utf8Path,
    evidence_root: Option<&Utf8Path>,
    release_evidence_dir: Option<&Utf8Path>,
) -> ReleaseActionItem {
    let priority = if check.status == "failed" {
        "required"
    } else {
        "optional"
    };
    let evidence_path = release_action_evidence_path(
        check,
        report,
        protocol_snapshot,
        evidence_root,
        release_evidence_dir,
    );
    let commands = release_action_commands(
        check,
        report,
        protocol_snapshot,
        evidence_root,
        release_evidence_dir,
    );
    ReleaseActionItem {
        check: check.name.clone(),
        status: check.status.clone(),
        priority: priority.to_string(),
        value: check.value.clone(),
        hint: check.hint.clone(),
        evidence_path,
        commands,
    }
}

fn release_action_evidence_path(
    check: &ReleaseCheckItem,
    report: &ReleaseCheckReport,
    protocol_snapshot: &Utf8Path,
    evidence_root: Option<&Utf8Path>,
    release_evidence_dir: Option<&Utf8Path>,
) -> Option<String> {
    if let Some(host) = check.name.strip_prefix("daw smoke: ") {
        return report
            .daw_matrix
            .iter()
            .find(|row| row["host"].as_str() == Some(host))
            .and_then(|row| row["evidence"].as_str())
            .and_then(normalize_report_path);
    }
    if check.name == "daw matrix" {
        return Some(portable_report_path(
            evidence_root.unwrap_or_else(|| Utf8Path::new("target/daw-evidence")),
        ));
    }
    if check.name == "protocol snapshot" {
        return Some(portable_report_path(protocol_snapshot));
    }

    let base = release_evidence_dir.unwrap_or_else(|| Utf8Path::new("target/release-evidence"));
    let relative = match check.name.as_str() {
        "ci run url" => "ci-run-url.txt",
        "ci doctor artifacts" => "ci-doctor",
        "ci release-check artifacts" => "ci-release-checks",
        "platform smoke artifacts" => "platform-smoke",
        "vst3 validate reports" | "vst3 example validator coverage" => "validator",
        "vst3 static validate reports" | "ci example static validate coverage" => "package",
        "crate publish plan" => "publish-plan/publish-plan.json",
        "crate package readiness" => "crate-package/crate-package.json",
        "npm package pack report" => "npm-pack/npm-pack.json",
        "dependency latest baseline" => "dependency-baseline/dependency-baseline-latest.json",
        "vst3 SDK header manifest" => "vst3-sdk/vst3-sdk-headers.json",
        "vst3 SDK generated bindings plan" => "vst3-sdk/generated-bindings-plan.json",
        "vst3 SDK generated bindings surface" => "vst3-sdk/generated-bindings-surface.json",
        "vst3 SDK generated bindings scaffold" => "vst3-sdk/generated.rs",
        "vst3 SDK generated bindings ABI seed" => "vst3-sdk/generated-abi-seed.rs",
        "vst3 SDK generated bindings ABI layout" => "vst3-sdk/generated-abi.rs",
        "vst3 SDK generated bindings interface skeleton" => {
            "vst3-sdk/generated-interface-skeleton.rs"
        }
        "signed bundle evidence" => {
            return Some(format!(
                "{}/signing-macos.log and {}/signing-windows.log",
                base, base
            ));
        }
        "notarization log" => "notary.log",
        _ => return None,
    };
    Some(portable_report_path(&base.join(relative)))
}

fn release_action_commands(
    check: &ReleaseCheckItem,
    report: &ReleaseCheckReport,
    protocol_snapshot: &Utf8Path,
    evidence_root: Option<&Utf8Path>,
    release_evidence_dir: Option<&Utf8Path>,
) -> Vec<String> {
    let evidence_root_text = evidence_root
        .map(portable_report_path)
        .unwrap_or_else(|| "target/daw-evidence".to_string());
    let release_evidence_text = release_evidence_dir
        .map(portable_report_path)
        .unwrap_or_else(|| "target/release-evidence".to_string());

    if check.name == "daw matrix" {
        return vec![
            format!("vesty daw-matrix --write-template --evidence-root {evidence_root_text}"),
            format!("vesty daw-matrix --evidence-root {evidence_root_text} --format markdown"),
            format!("vesty daw-matrix --evidence-root {evidence_root_text} --strict"),
        ];
    }
    if let Some(host) = check.name.strip_prefix("daw smoke: ") {
        let host_arg = host.to_ascii_lowercase().replace(['/', ' '], "-");
        let evidence_path = report
            .daw_matrix
            .iter()
            .find(|row| row["host"].as_str() == Some(host))
            .and_then(|row| row["evidence"].as_str())
            .unwrap_or("target/daw-evidence/<host>");
        return vec![format!(
            "vesty daw-matrix --write-report --host {host_arg} --platform \"<os arch / {host} version>\" --scan \"scan=true\" --load \"load=true\" --ui \"ui=true\" --ui-host-param \"ui_host_param=true\" --meter-stream \"meter_flush sent=1\" --automation \"automation=true\" --buffer-sample-rate-change \"buffer_sample_rate_change=true\" --save-restore \"save_restore=true\" --offline-render \"offline_render=true\" # writes {evidence_path}"
        )];
    }

    match check.name.as_str() {
        "protocol snapshot" => vec![
            format!("vesty export-types --out {protocol_snapshot}"),
            format!("vesty export-types --out {protocol_snapshot} --check"),
        ],
        "ci run url" => vec![
            format!("vesty release-check --write-evidence-template {release_evidence_text}"),
            format!(
                "printf '%s\\n' 'ci_run_url=https://github.com/<org>/<repo>/actions/runs/<id>' > {release_evidence_text}/ci-run-url.txt"
            ),
        ],
        "ci doctor artifacts" => vec![
            "download Linux, macOS and Windows doctor JSON snapshots from the same GitHub Actions run".to_string(),
            format!("vesty release-evidence import-ci --source target/downloaded-artifacts --dir {release_evidence_text} --ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>"),
        ],
        "ci release-check artifacts" => vec![
            "download release-check-Linux.json, release-check-macOS.json and release-check-Windows.json from the same GitHub Actions run".to_string(),
            format!("vesty release-evidence import-ci --source target/downloaded-artifacts --dir {release_evidence_text} --ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>"),
        ],
        "platform smoke artifacts" => vec![
            format!("vesty platform-smoke --write-template --dir {release_evidence_text}/platform-smoke"),
            format!("vesty platform-smoke --write-report --dir {release_evidence_text}/platform-smoke --platform macos --system-webview \"WebKit.framework loaded\" --vst3-validator \"Steinberg validator passed 47 tests, 0 failed\" --vst3-example-scan \"VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\" --webview-attach \"webview_attach=true\" --webview-resize \"webview_resize=true width=640 height=420\" --asset-protocol \"asset_protocol=true assets.manifest.json served\" --jsbridge-roundtrip \"jsbridge_roundtrip=true readyAck reply\" --meter-stream \"meter_flush sent=1\""),
            format!("vesty platform-smoke --write-report --dir {release_evidence_text}/platform-smoke --platform windows-x64 --system-webview \"WebView2 runtime loaded\" --vst3-validator \"Steinberg validator passed 47 tests, 0 failed\" --vst3-example-scan \"VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\" --webview-attach \"webview_attach=true\" --webview-resize \"webview_resize=true width=640 height=420\" --asset-protocol \"asset_protocol=true assets.manifest.json served\" --jsbridge-roundtrip \"jsbridge_roundtrip=true readyAck reply\" --meter-stream \"meter_flush sent=1\""),
            format!("vesty platform-smoke --write-report --dir {release_evidence_text}/platform-smoke --platform linux-x11 --system-webview \"WebKitGTK loaded; X11 display active\" --vst3-validator \"Steinberg validator passed 47 tests, 0 failed\" --vst3-example-scan \"VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3\" --webview-attach \"webview_attach=true\" --webview-resize \"webview_resize=true width=640 height=420\" --asset-protocol \"asset_protocol=true assets.manifest.json served\" --jsbridge-roundtrip \"jsbridge_roundtrip=true readyAck reply\" --meter-stream \"meter_flush sent=1\""),
            format!("vesty platform-smoke --dir {release_evidence_text}/platform-smoke --strict"),
        ],
        "vst3 validate reports" | "vst3 example validator coverage" => vec![
            format!("vesty validate <bundle.vst3> --strict --format json --report {release_evidence_text}/validator/<bundle>.<platform>.validate.json --validator-log {release_evidence_text}/validator/<bundle>.<platform>.validator.log"),
            "collect validator-passed reports for VestyGain.vst3, VestyWebUIDemo.vst3 and VestyMIDISynth.vst3 on macOS, Windows x64 and Linux x64".to_string(),
        ]
        .into_iter()
        .chain(example_validator_matrix_commands(&release_evidence_text))
        .collect(),
        "vst3 static validate reports" | "ci example static validate coverage" => vec![
            format!("vesty validate <bundle.vst3> --static-only --strict --format json --report {release_evidence_text}/package/<bundle>.<platform>.static-validate.json"),
            "collect static validate reports for VestyGain.vst3, VestyWebUIDemo.vst3 and VestyMIDISynth.vst3 on macOS, Windows x64 and Linux x64".to_string(),
        ]
        .into_iter()
        .chain(example_static_validate_matrix_commands(&release_evidence_text))
        .collect(),
        "crate publish plan" => vec![
            format!(
                "vesty publish-plan --out {release_evidence_text}/publish-plan/publish-plan.json"
            ),
            format!(
                "vesty publish-plan --check --out {release_evidence_text}/publish-plan/publish-plan.json"
            ),
        ],
        "crate package readiness" => vec![
            format!(
                "vesty crate-package --out {release_evidence_text}/crate-package/crate-package.json"
            ),
            format!(
                "vesty crate-package --check --out {release_evidence_text}/crate-package/crate-package.json"
            ),
        ],
        "npm package pack report" => vec![
            "npm run build".to_string(),
            format!("vesty npm-pack --out {release_evidence_text}/npm-pack/npm-pack.json"),
            format!("vesty npm-pack --check --out {release_evidence_text}/npm-pack/npm-pack.json"),
        ],
        "dependency latest baseline" => vec![
            format!(
                "vesty dependency-baseline --latest --out {release_evidence_text}/dependency-baseline/dependency-baseline-latest.json"
            ),
            format!(
                "vesty dependency-baseline --latest --check --out {release_evidence_text}/dependency-baseline/dependency-baseline-latest.json"
            ),
        ],
        "vst3 SDK header manifest" => vec![
            format!(
                "vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/vst3-sdk-headers.json"
            ),
            format!(
                "vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/vst3-sdk-headers.json --check"
            ),
        ],
        "vst3 SDK generated bindings plan" => vec![
            format!(
                "vesty vst3-sdk binding-plan --sdk-dir /path/to/VST_SDK --bindings-module target/vst3-sdk/generated.rs --out {release_evidence_text}/vst3-sdk/generated-bindings-plan.json"
            ),
            format!(
                "vesty vst3-sdk binding-plan --sdk-dir /path/to/VST_SDK --bindings-module target/vst3-sdk/generated.rs --out {release_evidence_text}/vst3-sdk/generated-bindings-plan.json --check"
            ),
        ],
        "vst3 SDK generated bindings surface" => vec![
            format!(
                "vesty vst3-sdk binding-surface --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-bindings-surface.json"
            ),
            format!(
                "vesty vst3-sdk binding-surface --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-bindings-surface.json --check"
            ),
        ],
        "vst3 SDK generated bindings scaffold" => vec![
            format!(
                "vesty vst3-sdk emit-scaffold --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated.rs"
            ),
            format!(
                "vesty vst3-sdk emit-scaffold --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated.rs --check"
            ),
        ],
        "vst3 SDK generated bindings ABI seed" => vec![
            format!(
                "vesty vst3-sdk emit-abi-seed --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-abi-seed.rs"
            ),
            format!(
                "vesty vst3-sdk emit-abi-seed --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-abi-seed.rs --check"
            ),
        ],
        "vst3 SDK generated bindings ABI layout" => vec![
            format!(
                "vesty vst3-sdk emit-abi --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-abi.rs"
            ),
            format!(
                "vesty vst3-sdk emit-abi --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-abi.rs --check"
            ),
        ],
        "vst3 SDK generated bindings interface skeleton" => vec![
            format!(
                "vesty vst3-sdk emit-interface-skeleton --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-interface-skeleton.rs"
            ),
            format!(
                "vesty vst3-sdk emit-interface-skeleton --sdk-dir /path/to/VST_SDK --out {release_evidence_text}/vst3-sdk/generated-interface-skeleton.rs --check"
            ),
        ],
        "signed bundle evidence" => vec![
            format!("vesty release-evidence collect-signing <signed-macos-bundle.vst3> --platform macos --dir {release_evidence_text}"),
            format!("vesty release-evidence collect-signing <signed-windows-bundle.vst3> --platform windows-x64 --dir {release_evidence_text}"),
        ],
        "notarization log" => vec![format!(
            "vesty release-evidence collect-notarization --notary-log <notarytool.log> --stapler-log <stapler.log> --dir {release_evidence_text}"
        )],
        _ => check
            .hint
            .as_ref()
            .map(|hint| vec![hint.clone()])
            .unwrap_or_default(),
    }
}

fn example_validator_matrix_commands(release_evidence_dir: &str) -> Vec<String> {
    REQUIRED_EXAMPLE_VALIDATE_PLATFORMS
        .iter()
        .flat_map(|platform| {
            REQUIRED_EXAMPLE_BUNDLES.iter().map(move |bundle| {
                format!(
                    "vesty validate <path-to-{bundle}> --strict --format json --report {release_evidence_dir}/validator/{bundle}.{platform}.validate.json --validator-log {release_evidence_dir}/validator/{bundle}.{platform}.validator.log"
                )
            })
        })
        .collect()
}

fn example_static_validate_matrix_commands(release_evidence_dir: &str) -> Vec<String> {
    REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS
        .iter()
        .flat_map(|platform| {
            REQUIRED_EXAMPLE_BUNDLES.iter().map(move |bundle| {
                format!(
                    "vesty validate <path-to-{bundle}> --static-only --strict --format json --report {release_evidence_dir}/package/{bundle}.{platform}.static-validate.json"
                )
            })
        })
        .collect()
}

fn write_release_action_plan(
    path: &Utf8Path,
    plan: &ReleaseActionPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = validate_release_action_plan_sidecar(plan) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid release action plan: {error}"),
        )
        .into());
    }
    write_text_file(path, &(serde_json::to_string_pretty(plan)? + "\n"))
}

fn apply_release_evidence_dir(
    options: &mut ReleaseEvidenceOptions,
    dir: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    require_existing_directory_no_symlink("release evidence dir", dir)?;

    let ci_run_url_file = dir.join("ci-run-url.txt");
    if let Some(file_url) = read_ci_run_url_file(&ci_run_url_file)? {
        if let Some(cli_url) = options.ci_run_url.as_deref() {
            validate_explicit_ci_run_urls_match(
                cli_url,
                "--ci-run-url",
                &file_url,
                &format!("release evidence {ci_run_url_file}"),
            )?;
        } else {
            options.ci_run_url = Some(file_url);
        }
    }
    if options.ci_doctor_dir.is_none()
        && let Some(ci_doctor) = release_evidence_candidate_dir(dir, "ci-doctor")?
        && !collect_json_files_recursive(&ci_doctor)?.is_empty()
    {
        options.ci_doctor_dir = Some(ci_doctor);
    }
    if options.ci_release_check_dir.is_none()
        && let Some(ci_release_checks) = release_evidence_candidate_dir(dir, "ci-release-checks")?
        && !collect_json_files_recursive(&ci_release_checks)?.is_empty()
    {
        options.ci_release_check_dir = Some(ci_release_checks);
    }
    if options.platform_smoke_dir.is_none()
        && let Some(platform_smoke) = release_evidence_candidate_dir(dir, "platform-smoke")?
        && platform_smoke_dir_has_non_pending_or_invalid_reports(&platform_smoke)?
    {
        options.platform_smoke_dir = Some(platform_smoke);
    }
    if options.publish_plan_report.is_none() {
        for relative in ["publish-plan/publish-plan.json", "publish-plan.json"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.publish_plan_report = Some(candidate);
                break;
            }
        }
    }
    if options.crate_package_report.is_none() {
        for relative in ["crate-package/crate-package.json", "crate-package.json"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.crate_package_report = Some(candidate);
                break;
            }
        }
    }
    if options.npm_pack_report.is_none() {
        for relative in ["npm-pack/npm-pack.json", "npm-pack.json"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.npm_pack_report = Some(candidate);
                break;
            }
        }
    }
    if options.dependency_baseline_report.is_none() {
        for relative in [
            "dependency-baseline/dependency-baseline-latest.json",
            "dependency-baseline-latest.json",
        ] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.dependency_baseline_report = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_manifest.is_none() {
        for relative in ["vst3-sdk/vst3-sdk-headers.json", "vst3-sdk-headers.json"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_manifest = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_binding_plan.is_none() {
        for relative in [
            "vst3-sdk/generated-bindings-plan.json",
            "generated-bindings-plan.json",
        ] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_binding_plan = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_binding_surface.is_none() {
        for relative in [
            "vst3-sdk/generated-bindings-surface.json",
            "generated-bindings-surface.json",
        ] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_binding_surface = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_scaffold.is_none() {
        for relative in ["vst3-sdk/generated.rs", "generated.rs"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_scaffold = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_abi_seed.is_none() {
        for relative in ["vst3-sdk/generated-abi-seed.rs", "generated-abi-seed.rs"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_abi_seed = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_abi.is_none() {
        for relative in ["vst3-sdk/generated-abi.rs", "generated-abi.rs"] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_abi = Some(candidate);
                break;
            }
        }
    }
    if options.vst3_sdk_interface_skeleton.is_none() {
        for relative in [
            "vst3-sdk/generated-interface-skeleton.rs",
            "generated-interface-skeleton.rs",
        ] {
            if let Some(candidate) = release_evidence_candidate_file(dir, relative)? {
                options.vst3_sdk_interface_skeleton = Some(candidate);
                break;
            }
        }
    }
    if let Some(validate_report) = release_evidence_candidate_file(dir, "validate-report.json")? {
        push_standard_release_validate_report(&mut options.validate_reports, validate_report);
    }
    if let Some(static_validate_report) =
        release_evidence_candidate_file(dir, "static-validate-report.json")?
    {
        push_standard_static_validate_report(
            &mut options.static_validate_reports,
            static_validate_report,
        );
    }
    apply_validate_reports_from_evidence_dir(options, dir)?;
    if let Some(macos_signing) = release_evidence_candidate_file(dir, "signing-macos.log")? {
        push_standard_signing_evidence(&mut options.signed_bundle_evidence, macos_signing);
    }
    if let Some(windows_signing) = release_evidence_candidate_file(dir, "signing-windows.log")? {
        push_standard_signing_evidence(&mut options.signed_bundle_evidence, windows_signing);
    }
    if options.notarization_log.is_none()
        && let Some(notary) = release_evidence_candidate_file(dir, "notary.log")?
        && !notarization_evidence_is_pending_template(&notary)
    {
        options.notarization_log = Some(notary);
    }
    apply_signing_and_notarization_from_evidence_dir(options, dir)?;
    Ok(())
}

fn release_evidence_candidate_file(
    dir: &Utf8Path,
    relative: &str,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    release_evidence_candidate_path(dir, relative, ReleaseEvidenceCandidateKind::File)
}

fn release_evidence_candidate_dir(
    dir: &Utf8Path,
    relative: &str,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    release_evidence_candidate_path(dir, relative, ReleaseEvidenceCandidateKind::Directory)
}

fn require_existing_directory_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("{label} does not exist or is not a directory: {path}").into());
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    if !metadata.is_dir() {
        return Err(format!("{label} does not exist or is not a directory: {path}").into());
    }
    Ok(())
}

fn existing_directory_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    Ok(metadata.is_dir())
}

fn reject_existing_path_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    Ok(())
}

fn require_existing_file_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<std::fs::Metadata, Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("{label} does not exist or is not a file: {path}").into());
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    if !metadata.is_file() {
        return Err(format!("{label} does not exist or is not a file: {path}").into());
    }
    Ok(metadata)
}

fn read_text_file_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<String, Box<dyn std::error::Error>> {
    require_existing_file_no_symlink(label, path)?;
    Ok(fs::read_to_string(path)?)
}

fn require_existing_file_or_directory_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<std::fs::Metadata, Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("{label} does not exist: {path}").into());
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} must not be a symlink: {path}").into());
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(format!("{label} must be a file or directory: {path}").into());
    }
    Ok(metadata)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReleaseEvidenceCandidateKind {
    File,
    Directory,
}

fn release_evidence_candidate_path(
    dir: &Utf8Path,
    relative: &str,
    kind: ReleaseEvidenceCandidateKind,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut path = dir.to_path_buf();
    let mut parts = relative
        .split('/')
        .filter(|part| !part.is_empty())
        .peekable();
    while let Some(part) = parts.next() {
        if part == "." || part == ".." || part.contains('\\') {
            return Err(format!("invalid release evidence relative path: {relative}").into());
        }
        path.push(part);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(format!("release evidence path must not be a symlink: {path}").into());
        }
        let is_leaf = parts.peek().is_none();
        if is_leaf {
            let matches_kind = match kind {
                ReleaseEvidenceCandidateKind::File => metadata.is_file(),
                ReleaseEvidenceCandidateKind::Directory => metadata.is_dir(),
            };
            return Ok(matches_kind.then_some(path));
        }
        if !metadata.is_dir() {
            return Ok(None);
        }
    }
    Ok(None)
}

fn platform_smoke_dir_has_non_pending_or_invalid_reports(
    dir: &Utf8Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    match collect_platform_smoke_reports(dir) {
        Ok(reports) => Ok(reports
            .iter()
            .any(|(_, report)| !platform_smoke_report_is_pending_template(report))),
        Err(_) => Ok(!collect_json_files_recursive(dir)?.is_empty()),
    }
}

fn apply_validate_reports_from_evidence_dir(
    options: &mut ReleaseEvidenceOptions,
    dir: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for path in collect_json_files_recursive(dir)? {
        if options
            .validate_reports
            .iter()
            .any(|existing| existing == &path)
            || options
                .static_validate_reports
                .iter()
                .any(|existing| existing == &path)
        {
            continue;
        }

        match read_validate_report(&path) {
            Ok(report) => {
                if validate_report_is_pending_template(&report) {
                    continue;
                } else if validate_release_validate_report(&report).is_ok() {
                    push_existing_unique(&mut options.validate_reports, path);
                } else if validate_static_validate_report(&report).is_ok()
                    || validate_report_path_prefers_static(&path)
                {
                    push_existing_unique(&mut options.static_validate_reports, path);
                } else if validate_report_path_prefers_release(&path) {
                    push_existing_unique(&mut options.validate_reports, path);
                }
            }
            Err(_) => {
                let looks_like_validate_report = validate_report_path_prefers_static(&path)
                    || validate_report_path_prefers_release(&path);
                if looks_like_validate_report
                    && !json_file_matches_non_validate_release_schema(&path)
                {
                    if validate_report_path_prefers_static(&path) {
                        push_existing_unique(&mut options.static_validate_reports, path);
                    } else {
                        push_existing_unique(&mut options.validate_reports, path);
                    }
                }
            }
        }
    }
    Ok(())
}

fn json_file_matches_non_validate_release_schema(path: &Utf8Path) -> bool {
    let Ok(text) = read_text_file_no_symlink("release evidence JSON artifact", path) else {
        return false;
    };
    serde_json::from_str::<ReleaseCheckReport>(&text).is_ok()
        || serde_json::from_str::<ReleaseActionPlan>(&text).is_ok()
        || serde_json::from_str::<PlatformSmokeReport>(&text).is_ok()
        || serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsSurface>(&text).is_ok()
        || serde_json::from_str::<vesty_vst3_sys::GeneratedBindingsPlan>(&text).is_ok()
        || serde_json::from_str::<vesty_vst3_sys::SdkHeaderInputManifest>(&text).is_ok()
        || serde_json::from_str::<DoctorReport>(&text).is_ok()
        || serde_json::from_str::<PublishPlan>(&text).is_ok()
        || serde_json::from_str::<CratePackageReport>(&text).is_ok()
        || serde_json::from_str::<DependencyBaselineReport>(&text).is_ok()
        || parse_npm_pack_report_text(&text).is_ok()
        || json_value_looks_like_non_validate_release_artifact(path, &text)
}

fn json_value_looks_like_non_validate_release_artifact(path: &Utf8Path, text: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };

    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name().unwrap_or_default().to_ascii_lowercase();
    if file_lower.contains("release-check")
        || path_lower.contains("/ci-release-checks/")
        || path_lower.contains("/release-check-")
    {
        return object
            .get("status")
            .is_some_and(serde_json::Value::is_string)
            && object
                .get("checks")
                .is_some_and(serde_json::Value::is_array);
    }

    if file_lower.contains("release-action-plan") || path_lower.contains("/release-action-plan-") {
        return object
            .get("status")
            .is_some_and(serde_json::Value::is_string)
            && object
                .get("actions")
                .is_some_and(serde_json::Value::is_array);
    }

    false
}

fn validate_report_path_prefers_static(path: &Utf8Path) -> bool {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name().unwrap_or_default().to_ascii_lowercase();
    path_lower.contains("/package/")
        || path_lower.contains("/static-validate/")
        || file_lower.contains("static-validate")
}

fn validate_report_path_prefers_release(path: &Utf8Path) -> bool {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name().unwrap_or_default().to_ascii_lowercase();
    path_lower.contains("/validator/")
        || file_lower.contains(".validate.")
        || (file_lower.contains("validate") && !file_lower.contains("static-validate"))
}

fn apply_signing_and_notarization_from_evidence_dir(
    options: &mut ReleaseEvidenceOptions,
    dir: &Utf8Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for path in collect_evidence_text_files_recursive(dir)? {
        if validate_signing_evidence(&path).is_ok() {
            push_existing_unique(&mut options.signed_bundle_evidence, path.clone());
        }
        if options.notarization_log.is_none() && validate_notarization_evidence(&path).is_ok() {
            options.notarization_log = Some(path);
        }
    }

    for path in collect_vst3_bundle_dirs_recursive(dir)? {
        if validate_signing_evidence(&path).is_ok() {
            push_existing_unique(&mut options.signed_bundle_evidence, path);
        }
    }

    Ok(())
}

fn push_existing_unique(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    if path.exists() && !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn push_standard_signing_evidence(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    if !signing_evidence_is_pending_template(&path) {
        push_existing_unique(paths, path);
    }
}

fn push_standard_release_validate_report(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    match read_validate_report(&path) {
        Ok(report) if validate_report_is_pending_template(&report) => {}
        Ok(_) | Err(_) => push_existing_unique(paths, path),
    }
}

fn push_standard_static_validate_report(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    match read_validate_report(&path) {
        Ok(report) if validate_report_is_pending_template(&report) => {}
        Ok(_) | Err(_) => push_existing_unique(paths, path),
    }
}

fn validate_report_is_pending_template(report: &ValidateReport) -> bool {
    report.bundle == "replace-with-bundle.vst3"
        && report.static_check.status == "failed"
        && report
            .static_check
            .error
            .as_deref()
            .is_some_and(|error| error.to_ascii_lowercase().contains("pending:"))
        && matches!(report.validator.status.as_str(), "not_run" | "skipped")
        && report
            .validator
            .reason
            .as_deref()
            .is_some_and(|reason| reason.to_ascii_lowercase().contains("pending"))
}

fn signing_evidence_is_pending_template(path: &Utf8Path) -> bool {
    read_text_file_no_symlink("signing evidence path", path)
        .is_ok_and(|text| signing_evidence_is_pending_template_text(&text))
}

fn signing_evidence_is_pending_template_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("signed=pending")
        && (lower.contains("codesign verify=pending") || lower.contains("signtool verify=pending"))
}

fn notarization_evidence_is_pending_template(path: &Utf8Path) -> bool {
    read_text_file_no_symlink("notarization evidence", path)
        .is_ok_and(|text| notarization_evidence_is_pending_template_text(&text))
}

fn notarization_evidence_is_pending_template_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("notarization=pending") && lower.contains("stapled=pending")
}

fn read_ci_run_url_file(path: &Utf8Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("CI run URL evidence must not be a symlink: {path}").into());
    }
    if !metadata.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let value = if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if key != "ci_run_url" && key != "ci-run-url" {
                continue;
            }
            value.trim()
        } else {
            line
        };
        if value.is_empty() || value.eq_ignore_ascii_case("pending") {
            continue;
        }
        return Ok(Some(value.to_string()));
    }
    Ok(None)
}

fn host_profile_release_check(rows: &[serde_json::Value]) -> ReleaseCheckItem {
    let diff = daw_matrix_host_set_diff(rows);
    let issues = daw_matrix_host_set_issues(&diff);
    if issues.is_empty() {
        ReleaseCheckItem {
            name: "host profiles".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} release host profiles covered: {}",
                vesty_core::host_profiles().len(),
                release_host_profile_names().join(", ")
            ),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "host profiles".to_string(),
            status: "failed".to_string(),
            value: issues.join("; "),
            hint: Some("keep daw-matrix rows aligned with vesty-core host profiles".to_string()),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DawMatrixHostSetDiff {
    missing: Vec<String>,
    duplicate: Vec<String>,
    unknown: Vec<String>,
    non_canonical: Vec<String>,
}

fn release_host_profile_names() -> Vec<&'static str> {
    vesty_core::host_profiles()
        .iter()
        .map(|profile| profile.name)
        .collect()
}

fn daw_matrix_host_set_diff(rows: &[serde_json::Value]) -> DawMatrixHostSetDiff {
    let mut seen_profile_ids = BTreeSet::new();
    let mut duplicate = Vec::new();
    let mut unknown = Vec::new();
    let mut non_canonical = Vec::new();
    for row in rows {
        let Some(host) = row.get("host").and_then(serde_json::Value::as_str) else {
            unknown.push("<missing host>".to_string());
            continue;
        };
        let Some(profile) = vesty_core::find_host_profile(host) else {
            unknown.push(host.to_string());
            continue;
        };
        if host != profile.name {
            non_canonical.push(format!("{host} -> {}", profile.name));
        }
        if !seen_profile_ids.insert(profile.id) {
            duplicate.push(profile.name.to_string());
        }
    }
    let missing = vesty_core::host_profiles()
        .iter()
        .filter(|profile| !seen_profile_ids.contains(profile.id))
        .map(|profile| profile.name.to_string())
        .collect();
    DawMatrixHostSetDiff {
        missing,
        duplicate,
        unknown,
        non_canonical,
    }
}

fn daw_matrix_host_set_issues(diff: &DawMatrixHostSetDiff) -> Vec<String> {
    let mut issues = Vec::new();
    if !diff.missing.is_empty() {
        issues.push(format!("missing profile rows: {}", diff.missing.join(", ")));
    }
    if !diff.duplicate.is_empty() {
        issues.push(format!(
            "duplicate profile rows: {}",
            diff.duplicate.join(", ")
        ));
    }
    if !diff.unknown.is_empty() {
        issues.push(format!("unknown profile rows: {}", diff.unknown.join(", ")));
    }
    if !diff.non_canonical.is_empty() {
        issues.push(format!(
            "non-canonical profile rows: {}",
            diff.non_canonical.join(", ")
        ));
    }
    issues
}

fn daw_matrix_release_check(rows: &[serde_json::Value]) -> ReleaseCheckItem {
    if daw_matrix_complete(rows) {
        ReleaseCheckItem {
            name: "daw matrix".to_string(),
            status: "ok".to_string(),
            value: "all required host smoke checks pass".to_string(),
            hint: None,
        }
    } else {
        let missing_hosts = rows
            .iter()
            .filter(|row| !missing_smoke_checks(row).is_empty())
            .filter_map(|row| row["host"].as_str())
            .collect::<Vec<_>>();
        ReleaseCheckItem {
            name: "daw matrix".to_string(),
            status: "failed".to_string(),
            value: format!("incomplete hosts: {}", missing_hosts.join(", ")),
            hint: Some(
                "run `vesty daw-matrix --write-template` and collect real host evidence"
                    .to_string(),
            ),
        }
    }
}

fn host_row_release_check(row: &serde_json::Value) -> ReleaseCheckItem {
    let host = row["host"].as_str().unwrap_or("unknown host");
    let missing = missing_smoke_checks(row);
    if missing.is_empty() {
        ReleaseCheckItem {
            name: format!("daw smoke: {host}"),
            status: "ok".to_string(),
            value: "all smoke checks pass".to_string(),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: format!("daw smoke: {host}"),
            status: "failed".to_string(),
            value: format!("missing: {}", missing.join(", ")),
            hint: row["evidence"].as_str().map(|evidence| {
                format!("collect evidence in `{evidence}`; install detection is not enough")
            }),
        }
    }
}

fn missing_smoke_checks(row: &serde_json::Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if row["platform_supported"].as_bool() != Some(true) {
        missing.push("platform");
    }
    missing.extend(
        vesty_core::RELEASE_SMOKE_CHECKS
            .iter()
            .copied()
            .filter(|key| row[*key].as_bool() != Some(true)),
    );
    missing
}

fn protocol_release_check(
    protocol_snapshot: &Utf8Path,
    skip_protocol: bool,
    required: bool,
) -> ReleaseCheckItem {
    if skip_protocol {
        if required {
            return ReleaseCheckItem {
                name: "protocol snapshot".to_string(),
                status: "failed".to_string(),
                value: "cannot skip protocol snapshot when --require-release-artifacts is set"
                    .to_string(),
                hint: Some(format!(
                    "run `vesty export-types --out {protocol_snapshot} --check` in final release evidence"
                )),
            };
        }
        return ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "skipped".to_string(),
            value: "skipped by --skip-protocol".to_string(),
            hint: Some("release CI should normally run protocol drift check".to_string()),
        };
    }
    match check_protocol_export(protocol_snapshot) {
        Ok(()) => ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "ok".to_string(),
            value: protocol_snapshot.to_string(),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "protocol snapshot".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(format!(
                "run `vesty export-types --out {protocol_snapshot}` and commit/update the snapshot"
            )),
        },
    }
}

fn binding_baseline_release_check() -> ReleaseCheckItem {
    ReleaseCheckItem {
        name: "vst3 binding baseline".to_string(),
        status: "ok".to_string(),
        value: binding_baseline_value(),
        hint: binding_baseline_hint(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PublishPlanEvidence {
    package_count: usize,
    skipped_private_count: usize,
    final_package: String,
}

fn publish_plan_release_check(path: Option<&Utf8Path>, required: bool) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "crate publish plan",
            required,
            "pass `--publish-plan-report <path>` from `vesty publish-plan --out <path>`, or include `publish-plan/publish-plan.json` in release evidence",
        );
    };

    match validate_publish_plan_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "crate publish plan".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} publishable crates; {} private skipped; final crate: {}",
                evidence.package_count, evidence.skipped_private_count, evidence.final_package
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "crate publish plan".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some("regenerate with `vesty publish-plan --out publish-plan.json`".to_string()),
        },
    }
}

fn validate_publish_plan_report(
    path: &Utf8Path,
) -> Result<PublishPlanEvidence, Box<dyn std::error::Error>> {
    let plan = read_publish_plan_report(path)?;
    validate_publish_plan(&plan)?;
    let final_package = plan
        .packages
        .iter()
        .max_by_key(|package| package.order)
        .map(|package| package.name.clone())
        .ok_or("publish plan has no packages")?;
    Ok(PublishPlanEvidence {
        package_count: plan.packages.len(),
        skipped_private_count: plan.skipped_private.len(),
        final_package,
    })
}

fn validate_publish_plan(plan: &PublishPlan) -> Result<(), Box<dyn std::error::Error>> {
    validate_publish_plan_shape(plan)?;

    if plan.packages.is_empty() {
        return Err("publish plan has no publishable packages".into());
    }

    let mut order_by_name = BTreeMap::<&str, usize>::new();
    let mut level_by_name = BTreeMap::<&str, usize>::new();
    let mut seen_orders = BTreeSet::<usize>::new();
    for (index, package) in plan.packages.iter().enumerate() {
        if package.name.trim().is_empty() {
            return Err(format!("publish plan package at index {index} has empty name").into());
        }
        if package.version.trim().is_empty() {
            return Err(format!("publish plan package {} has empty version", package.name).into());
        }
        if package.manifest_path.trim().is_empty() {
            return Err(format!(
                "publish plan package {} has empty manifest path",
                package.name
            )
            .into());
        }
        if package.order != index + 1 {
            return Err(format!(
                "publish plan package {} has order {}, expected {}",
                package.name,
                package.order,
                index + 1
            )
            .into());
        }
        if package.level == 0 {
            return Err(format!("publish plan package {} has zero level", package.name).into());
        }
        if order_by_name
            .insert(package.name.as_str(), package.order)
            .is_some()
        {
            return Err(format!("publish plan contains duplicate package {}", package.name).into());
        }
        if !seen_orders.insert(package.order) {
            return Err(format!("publish plan contains duplicate order {}", package.order).into());
        }
        level_by_name.insert(package.name.as_str(), package.level);
    }

    for package in &plan.packages {
        for dependency in &package.internal_dependencies {
            let Some(dependency_order) = order_by_name.get(dependency.as_str()) else {
                return Err(format!(
                    "publish plan package {} depends on unknown package {}",
                    package.name, dependency
                )
                .into());
            };
            if *dependency_order >= package.order {
                return Err(format!(
                    "publish plan orders dependency {} after dependent {}",
                    dependency, package.name
                )
                .into());
            }
            if level_by_name.get(dependency.as_str()).copied().unwrap_or(0) >= package.level {
                return Err(format!(
                    "publish plan level for dependency {} is not lower than {}",
                    dependency, package.name
                )
                .into());
            }
        }
    }

    Ok(())
}

fn validate_publish_plan_shape(plan: &PublishPlan) -> Result<(), Box<dyn std::error::Error>> {
    if plan.packages.is_empty() {
        return Err("publish plan has no publishable packages".into());
    }
    if plan.packages.len() > PUBLISH_PLAN_MAX_PACKAGES {
        return Err(format!(
            "publish plan has too many packages: {} exceeds maximum {PUBLISH_PLAN_MAX_PACKAGES}",
            plan.packages.len()
        )
        .into());
    }
    if plan.skipped_private.len() > PUBLISH_PLAN_MAX_SKIPPED_PRIVATE {
        return Err(format!(
            "publish plan has too many skipped private packages: {} exceeds maximum {PUBLISH_PLAN_MAX_SKIPPED_PRIVATE}",
            plan.skipped_private.len()
        )
        .into());
    }

    let mut seen_skipped_private = BTreeSet::new();
    for package in &plan.skipped_private {
        validate_release_action_text("publish plan skipped private package", package)?;
        if !seen_skipped_private.insert(package.as_str()) {
            return Err(format!(
                "publish plan contains duplicate skipped private package `{package}`"
            )
            .into());
        }
    }

    for package in &plan.packages {
        validate_release_action_text("publish plan package name", &package.name)?;
        validate_release_action_text(
            &format!("publish plan package `{}` version", package.name),
            &package.version,
        )?;
        validate_release_action_text(
            &format!("publish plan package `{}` manifest path", package.name),
            &package.manifest_path,
        )?;
        if package.internal_dependencies.len() > PUBLISH_PLAN_MAX_DEPENDENCIES {
            return Err(format!(
                "publish plan package `{}` has too many internal dependencies: {} exceeds maximum {PUBLISH_PLAN_MAX_DEPENDENCIES}",
                package.name,
                package.internal_dependencies.len()
            )
            .into());
        }
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &package.internal_dependencies {
            validate_release_action_text(
                &format!(
                    "publish plan package `{}` internal dependency",
                    package.name
                ),
                dependency,
            )?;
            if !seen_dependencies.insert(dependency.as_str()) {
                return Err(format!(
                    "publish plan package `{}` has duplicate internal dependency `{dependency}`",
                    package.name
                )
                .into());
            }
        }
    }
    Ok(())
}

fn crate_package_release_check(
    path: Option<&Utf8Path>,
    publish_plan_path: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "crate package readiness",
            required,
            "pass `--crate-package-report <path>` from `vesty crate-package --out <path>`, or include `crate-package/crate-package.json` in release evidence",
        );
    };

    match validate_crate_package_report_path_with_publish_plan(path, publish_plan_path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "crate package readiness".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} workspace crates; {} packageable now; {} deferred until internal dependencies publish",
                evidence.package_count, evidence.packaged_count, evidence.deferred_count
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "crate package readiness".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty crate-package --out crate-package.json`".to_string(),
            ),
        },
    }
}

const REQUIRED_NPM_PACKAGES: [&str; 1] = ["vesty-plugin-ui"];
const NPM_PACK_MAX_PACKAGES: usize = 16;
const NPM_PACK_MAX_FILES_PER_PACKAGE: usize = 512;
const NPM_PACK_MAX_TOTAL_FILES: usize = 2048;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NpmPackEntry {
    name: String,
    version: String,
    filename: String,
    files: Vec<NpmPackFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NpmPackFile {
    path: String,
}

#[derive(Debug, Deserialize)]
struct NpmPackCommandEntry {
    name: String,
    version: String,
    filename: String,
    files: Vec<NpmPackCommandFile>,
}

#[derive(Debug, Deserialize)]
struct NpmPackCommandFile {
    path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NpmPackEvidence {
    package_count: usize,
    total_files: usize,
    packages: Vec<String>,
}

fn npm_pack_release_check(path: Option<&Utf8Path>, required: bool) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "npm package pack report",
            required,
            "pass `--npm-pack-report <path>` from `vesty npm-pack --out <path>`, or include `npm-pack/npm-pack.json` in release evidence",
        );
    };

    match validate_npm_pack_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "npm package pack report".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} package(s), {} file(s): {}",
                evidence.package_count,
                evidence.total_files,
                evidence.packages.join(", ")
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "npm package pack report".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty npm-pack --out npm-pack.json` after building JS packages"
                    .to_string(),
            ),
        },
    }
}

fn validate_npm_pack_report(
    path: &Utf8Path,
) -> Result<NpmPackEvidence, Box<dyn std::error::Error>> {
    let entries = read_npm_pack_report(path)?;
    validate_npm_pack_entries(&entries)
}

fn read_npm_pack_report(path: &Utf8Path) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("npm pack report", path)?;
    parse_npm_pack_report_text(&text)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DependencyBaselineEvidence {
    baseline_checks: usize,
    latest_checks: usize,
}

fn dependency_baseline_latest_release_check(
    path: Option<&Utf8Path>,
    required: bool,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "dependency latest baseline",
            required,
            "pass `--dependency-baseline-report <path>` from `vesty dependency-baseline --latest --out <path>`, or include `dependency-baseline/dependency-baseline-latest.json` in release evidence",
        );
    };

    match validate_dependency_baseline_latest_report(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "dependency latest baseline".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} baseline check(s), {} latest registry check(s)",
                evidence.baseline_checks, evidence.latest_checks
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "dependency latest baseline".to_string(),
            status: "failed".to_string(),
            value: error.to_string(),
            hint: Some(
                "regenerate with `vesty dependency-baseline --latest --out dependency-baseline-latest.json` after registry review"
                    .to_string(),
            ),
        },
    }
}

fn validate_dependency_baseline_latest_report(
    path: &Utf8Path,
) -> Result<DependencyBaselineEvidence, Box<dyn std::error::Error>> {
    let report = read_dependency_baseline_report(path)?;
    validate_dependency_baseline_report(&report)?;

    let expected_latest = expected_dependency_latest_check_names();
    let has_workspace_baseline_coverage = report.checks.iter().any(|check| {
        check.name == DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME && check.kind == "cargo-baseline"
    });
    if !has_workspace_baseline_coverage {
        return Err(format!(
            "dependency baseline report is missing required baseline check: {DEPENDENCY_BASELINE_COVERAGE_CHECK_NAME}"
        )
        .into());
    }

    let actual_latest = report
        .checks
        .iter()
        .filter(|check| matches!(check.kind.as_str(), "cargo-registry" | "npm-registry"))
        .map(|check| check.name.clone())
        .collect::<BTreeSet<_>>();

    let missing = expected_latest
        .iter()
        .filter(|name| !actual_latest.contains(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "dependency baseline report is missing latest registry checks: {}",
            missing.join(", ")
        )
        .into());
    }

    let unexpected = actual_latest
        .iter()
        .filter(|name| !expected_latest.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(format!(
            "dependency baseline report contains unexpected latest registry checks: {}",
            unexpected.join(", ")
        )
        .into());
    }

    Ok(DependencyBaselineEvidence {
        baseline_checks: report.checks.len().saturating_sub(actual_latest.len()),
        latest_checks: actual_latest.len(),
    })
}

fn expected_dependency_latest_check_names() -> BTreeSet<String> {
    let mut expected = REQUIRED_RUST_BASELINE_DEPENDENCIES
        .iter()
        .map(|(name, _)| format!("crates.io latest `{name}`"))
        .collect::<BTreeSet<_>>();
    expected.insert("npm registry latest `typescript`".to_string());
    expected.extend(
        REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES
            .iter()
            .map(|dependency| format!("npm registry latest `{}`", dependency.dependency)),
    );
    expected
}

fn parse_npm_pack_report_text(text: &str) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    serde_json::from_str::<Vec<NpmPackEntry>>(text)
        .map_err(|error| format!("invalid npm pack report JSON: {error}").into())
}

fn parse_npm_pack_command_output(
    text: &str,
) -> Result<Vec<NpmPackEntry>, Box<dyn std::error::Error>> {
    let entries = serde_json::from_str::<Vec<NpmPackCommandEntry>>(text)
        .map_err(|error| format!("invalid npm pack command JSON: {error}"))?;
    Ok(entries
        .into_iter()
        .map(|entry| NpmPackEntry {
            name: entry.name,
            version: entry.version,
            filename: entry.filename,
            files: entry
                .files
                .into_iter()
                .map(|file| NpmPackFile { path: file.path })
                .collect(),
        })
        .collect())
}

fn validate_npm_pack_entries(
    entries: &[NpmPackEntry],
) -> Result<NpmPackEvidence, Box<dyn std::error::Error>> {
    validate_npm_pack_entries_shape(entries)?;

    if entries.is_empty() {
        return Err("npm pack report has no packages".into());
    }

    let required = REQUIRED_NPM_PACKAGES.into_iter().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::<String>::new();
    let mut total_files = 0;
    for entry in entries {
        if !required.contains(entry.name.as_str()) {
            return Err(format!("unexpected npm package {}", entry.name).into());
        }
        if !seen.insert(entry.name.clone()) {
            return Err(format!("duplicate npm package {}", entry.name).into());
        }
        if entry.version.trim().is_empty() {
            return Err(format!("npm package {} has empty version", entry.name).into());
        }
        if !entry.filename.ends_with(".tgz") {
            return Err(
                format!("npm package {} filename does not end with .tgz", entry.name).into(),
            );
        }
        if entry.files.is_empty() {
            return Err(format!("npm package {} has no packed files", entry.name).into());
        }

        let mut has_package_json = false;
        let mut has_dist = false;
        for file in &entry.files {
            validate_npm_pack_file_path(&entry.name, &file.path)?;
            has_package_json |= file.path == "package.json";
            has_dist |= file.path.starts_with("dist/");
            total_files += 1;
        }
        if !has_package_json {
            return Err(format!("npm package {} is missing package.json", entry.name).into());
        }
        if !has_dist {
            return Err(format!("npm package {} has no dist files", entry.name).into());
        }
    }

    let missing = required
        .iter()
        .filter(|package| !seen.contains(**package))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("npm pack report missing packages: {}", missing.join(", ")).into());
    }

    let packages = REQUIRED_NPM_PACKAGES
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    Ok(NpmPackEvidence {
        package_count: entries.len(),
        total_files,
        packages,
    })
}

fn validate_npm_pack_entries_shape(
    entries: &[NpmPackEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    if entries.is_empty() {
        return Err("npm pack report has no packages".into());
    }
    if entries.len() > NPM_PACK_MAX_PACKAGES {
        return Err(format!(
            "npm pack report has too many packages: {} exceeds maximum {NPM_PACK_MAX_PACKAGES}",
            entries.len()
        )
        .into());
    }

    let mut total_files = 0usize;
    for entry in entries {
        validate_release_action_text("npm package name", &entry.name)?;
        validate_release_action_text(
            &format!("npm package `{}` version", entry.name),
            &entry.version,
        )?;
        validate_release_action_text(
            &format!("npm package `{}` filename", entry.name),
            &entry.filename,
        )?;
        if entry.files.is_empty() {
            return Err(format!("npm package {} has no packed files", entry.name).into());
        }
        if entry.files.len() > NPM_PACK_MAX_FILES_PER_PACKAGE {
            return Err(format!(
                "npm package {} has too many packed files: {} exceeds maximum {NPM_PACK_MAX_FILES_PER_PACKAGE}",
                entry.name,
                entry.files.len()
            )
            .into());
        }
        total_files += entry.files.len();
        if total_files > NPM_PACK_MAX_TOTAL_FILES {
            return Err(format!(
                "npm pack report has too many packed files: {total_files} exceeds maximum {NPM_PACK_MAX_TOTAL_FILES}"
            )
            .into());
        }

        let mut seen_paths = BTreeSet::new();
        for file in &entry.files {
            validate_release_action_text(
                &format!("npm package `{}` packed path", entry.name),
                &file.path,
            )?;
            if !seen_paths.insert(file.path.as_str()) {
                return Err(format!(
                    "npm package {} has duplicate packed path `{}`",
                    entry.name, file.path
                )
                .into());
            }
        }
    }
    Ok(())
}

fn validate_npm_pack_file_path(
    package: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.trim() != path || path.is_empty() {
        return Err(format!("npm package {package} has invalid packed path `{path}`").into());
    }
    if path.contains('\\') || path.starts_with('/') || path == ".." || path.contains("../") {
        return Err(format!("npm package {package} has unsafe packed path `{path}`").into());
    }
    if path == "package.json" || path.starts_with("dist/") {
        Ok(())
    } else {
        Err(format!(
            "npm package {package} includes non-release file `{path}`; expected package.json or dist/**"
        )
        .into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Vst3SdkHeaderManifestEvidence {
    header_count: usize,
    baseline: String,
    upstream_vst3_crate: String,
    version_hint: Option<String>,
}

fn vst3_sdk_manifest_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out vst3-sdk-headers.json` when auditing generated-header inputs"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_header_manifest(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} required header(s); Steinberg SDK {}; upstream vst3 crate {}{}",
                evidence.header_count,
                evidence.baseline,
                evidence.upstream_vst3_crate,
                evidence
                    .version_hint
                    .as_deref()
                    .map(|hint| format!("; {hint}"))
                    .unwrap_or_default()
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK header manifest".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate from the official Steinberg VST3 SDK checkout with `vesty vst3-sdk manifest`"
                    .to_string(),
            ),
        },
    }
}

fn validate_vst3_sdk_header_manifest(
    path: &Utf8Path,
) -> Result<Vst3SdkHeaderManifestEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK header manifest", path)?;
    let manifest: vesty_vst3_sys::SdkHeaderInputManifest = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK header manifest JSON: {error}"))?;
    validate_vst3_sdk_header_manifest_content(&manifest)
}

const VST3_SDK_MAX_HEADERS: usize = 128;
const VST3_SDK_MAX_PLAN_CHECKS: usize = 32;
const VST3_SDK_MAX_TEXT_LIST_ITEMS: usize = 128;
const VST3_SDK_MAX_SURFACE_SYMBOLS: usize = 512;

fn expected_vst3_sdk_binding_plan_check_names() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "sdk header inputs",
        "bindings module path",
        "binding emitter",
    ])
}

fn expected_vst3_sdk_binding_surface_symbol_names() -> BTreeSet<String> {
    expected_vst3_sdk_binding_surface_symbol_specs()
        .keys()
        .cloned()
        .collect()
}

fn expected_vst3_sdk_binding_surface_symbol_specs()
-> BTreeMap<String, vesty_vst3_sys::GeneratedBindingsSurfaceSymbolSpec> {
    vesty_vst3_sys::generated_bindings_surface_symbol_specs()
        .into_iter()
        .map(|symbol| (symbol.name.to_string(), symbol))
        .collect()
}

fn validate_vst3_sdk_header_manifest_shape(
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK manifest generator", &manifest.generator)?;
    validate_release_action_text(
        "VST3 SDK manifest Steinberg SDK baseline",
        &manifest.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK manifest upstream vst3 crate baseline",
        &manifest.upstream_vst3_crate_baseline,
    )?;
    if let Some(version_hint) = &manifest.version_hint {
        validate_release_action_text("VST3 SDK manifest version hint", version_hint)?;
    }
    if manifest.headers.len() > VST3_SDK_MAX_HEADERS {
        return Err(format!(
            "VST3 SDK manifest has too many headers: {} exceeds maximum {VST3_SDK_MAX_HEADERS}",
            manifest.headers.len()
        )
        .into());
    }
    if manifest.missing_headers.len() > VST3_SDK_MAX_HEADERS {
        return Err(format!(
            "VST3 SDK manifest has too many missing headers: {} exceeds maximum {VST3_SDK_MAX_HEADERS}",
            manifest.missing_headers.len()
        )
        .into());
    }

    let mut seen_headers = BTreeSet::new();
    for header in &manifest.headers {
        validate_vst3_sdk_header_path_text("VST3 SDK manifest header path", &header.path)?;
        validate_release_action_text("VST3 SDK manifest header sha256", &header.sha256)?;
        if !seen_headers.insert(header.path.as_str()) {
            return Err(format!("duplicate VST3 SDK manifest header `{}`", header.path).into());
        }
    }
    validate_vst3_sdk_text_list(
        "VST3 SDK manifest missing header",
        &manifest.missing_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    Ok(())
}

fn validate_vst3_sdk_binding_plan_shape(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK binding plan generator", &plan.generator)?;
    validate_release_action_text("VST3 SDK binding plan status", &plan.status)?;
    validate_release_action_text(
        "VST3 SDK binding plan Steinberg SDK baseline",
        &plan.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding plan upstream vst3 crate baseline",
        &plan.upstream_vst3_crate_baseline,
    )?;
    validate_release_action_text("VST3 SDK binding plan active backend", &plan.active_backend)?;
    validate_release_action_text("VST3 SDK binding plan sdk dir", &plan.sdk_dir)?;
    validate_release_action_text("VST3 SDK binding plan module", &plan.bindings_module)?;
    validate_vst3_sdk_header_manifest_shape(&plan.header_manifest)?;
    if plan.checks.is_empty() {
        return Err("VST3 SDK binding plan has no checks".into());
    }
    if plan.checks.len() > VST3_SDK_MAX_PLAN_CHECKS {
        return Err(format!(
            "VST3 SDK binding plan has too many checks: {} exceeds maximum {VST3_SDK_MAX_PLAN_CHECKS}",
            plan.checks.len()
        )
        .into());
    }
    let mut seen_checks = BTreeSet::new();
    for check in &plan.checks {
        validate_release_action_text("VST3 SDK binding plan check name", &check.name)?;
        validate_release_action_text(
            &format!("VST3 SDK binding plan `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(
            &format!("VST3 SDK binding plan `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(
                &format!("VST3 SDK binding plan `{}` hint", check.name),
                hint,
            )?;
        }
        if !seen_checks.insert(check.name.as_str()) {
            return Err(format!("duplicate VST3 SDK binding plan check `{}`", check.name).into());
        }
    }
    let expected_checks = expected_vst3_sdk_binding_plan_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .copied()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown VST3 SDK binding plan check(s): {}",
            unknown_checks.join(", ")
        )
        .into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .copied()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "VST3 SDK binding plan missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    validate_vst3_sdk_text_list(
        "VST3 SDK binding plan blocker",
        &plan.blockers,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding plan next step",
        &plan.next_steps,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        false,
    )?;
    Ok(())
}

fn validate_vst3_sdk_binding_surface_shape(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("VST3 SDK binding surface generator", &surface.generator)?;
    validate_release_action_text("VST3 SDK binding surface status", &surface.status)?;
    validate_release_action_text(
        "VST3 SDK binding surface Steinberg SDK baseline",
        &surface.steinberg_sdk_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding surface upstream vst3 crate baseline",
        &surface.upstream_vst3_crate_baseline,
    )?;
    validate_release_action_text(
        "VST3 SDK binding surface active backend",
        &surface.active_backend,
    )?;
    validate_release_action_text("VST3 SDK binding surface sdk dir", &surface.sdk_dir)?;
    validate_vst3_sdk_header_manifest_shape(&surface.header_manifest)?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface required header",
        &surface.required_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface missing header",
        &surface.missing_headers,
        VST3_SDK_MAX_HEADERS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface missing symbol",
        &surface.missing_symbols,
        VST3_SDK_MAX_SURFACE_SYMBOLS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface blocker",
        &surface.blockers,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        true,
    )?;
    validate_vst3_sdk_text_list(
        "VST3 SDK binding surface note",
        &surface.notes,
        VST3_SDK_MAX_TEXT_LIST_ITEMS,
        false,
    )?;
    if surface.symbols.is_empty() {
        return Err("VST3 SDK binding surface has no symbols".into());
    }
    if surface.symbols.len() > VST3_SDK_MAX_SURFACE_SYMBOLS {
        return Err(format!(
            "VST3 SDK binding surface has too many symbols: {} exceeds maximum {VST3_SDK_MAX_SURFACE_SYMBOLS}",
            surface.symbols.len()
        )
        .into());
    }
    let mut seen_symbols = BTreeSet::new();
    for symbol in &surface.symbols {
        validate_release_action_text("VST3 SDK binding surface symbol name", &symbol.name)?;
        validate_release_action_text(
            &format!("VST3 SDK binding surface `{}` kind", symbol.name),
            &symbol.kind,
        )?;
        validate_vst3_sdk_header_path_text(
            &format!("VST3 SDK binding surface `{}` header", symbol.name),
            &symbol.header,
        )?;
        validate_release_action_text(
            &format!("VST3 SDK binding surface `{}` purpose", symbol.name),
            &symbol.purpose,
        )?;
        let key = (&symbol.name, &symbol.kind, &symbol.header);
        if !seen_symbols.insert(key) {
            return Err(format!(
                "duplicate VST3 SDK binding surface symbol `{}` kind `{}` header `{}`",
                symbol.name, symbol.kind, symbol.header
            )
            .into());
        }
    }
    Ok(())
}

fn validate_vst3_sdk_text_list(
    label: &str,
    values: &[String],
    max_items: usize,
    allow_empty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !allow_empty && values.is_empty() {
        return Err(format!("{label} list must not be empty").into());
    }
    if values.len() > max_items {
        return Err(format!(
            "{label} list has too many items: {} exceeds maximum {max_items}",
            values.len()
        )
        .into());
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_release_action_text(label, value)?;
        if !seen.insert(value.as_str()) {
            return Err(format!("duplicate {label} `{value}`").into());
        }
    }
    Ok(())
}

fn validate_vst3_sdk_header_path_text(label: &str, value: &str) -> Result<(), String> {
    validate_release_action_text(label, value)?;
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(format!("{label} must be a relative normalized header path"));
    }
    Ok(())
}

fn validate_vst3_sdk_header_manifest_content(
    manifest: &vesty_vst3_sys::SdkHeaderInputManifest,
) -> Result<Vst3SdkHeaderManifestEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_header_manifest_shape(manifest)?;
    if manifest.version != vesty_vst3_sys::SDK_HEADER_MANIFEST_VERSION {
        return Err(format!(
            "manifest version is {}, expected {}",
            manifest.version,
            vesty_vst3_sys::SDK_HEADER_MANIFEST_VERSION
        )
        .into());
    }
    if manifest.generator != vesty_vst3_sys::SDK_HEADER_MANIFEST_GENERATOR {
        return Err(format!(
            "manifest generator is `{}`, expected `{}`",
            manifest.generator,
            vesty_vst3_sys::SDK_HEADER_MANIFEST_GENERATOR
        )
        .into());
    }
    if manifest.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        return Err(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            manifest.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        )
        .into());
    }
    if manifest.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        return Err(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            manifest.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        )
        .into());
    }

    let required = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut duplicate_headers = Vec::new();
    let mut unexpected_headers = Vec::new();
    let mut invalid_headers = Vec::new();
    for header in &manifest.headers {
        let path = header.path.as_str();
        if !required.contains(path) {
            unexpected_headers.push(path.to_string());
        }
        if !seen.insert(path) {
            duplicate_headers.push(path.to_string());
        }
        if header.size == 0 {
            invalid_headers.push(format!("{path} has zero size"));
        }
        if !is_lowercase_sha256_hex(&header.sha256) {
            invalid_headers.push(format!("{path} has invalid sha256"));
        }
    }

    let expected_missing = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .filter(|path| !seen.contains(**path))
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    let unexpected_missing = manifest
        .missing_headers
        .iter()
        .filter(|path| !required.contains(path.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let mut errors = Vec::new();
    if !duplicate_headers.is_empty() {
        errors.push(format!(
            "duplicate header entries: {}",
            duplicate_headers.join(", ")
        ));
    }
    if !unexpected_headers.is_empty() {
        errors.push(format!(
            "unexpected header entries: {}",
            unexpected_headers.join(", ")
        ));
    }
    if !unexpected_missing.is_empty() {
        errors.push(format!(
            "unexpected missing header entries: {}",
            unexpected_missing.join(", ")
        ));
    }
    if manifest.missing_headers != expected_missing {
        errors.push(format!(
            "missing_headers is {:?}, expected {:?}",
            manifest.missing_headers, expected_missing
        ));
    }
    if manifest.complete != expected_missing.is_empty() {
        errors.push(format!(
            "complete is {}, expected {}",
            manifest.complete,
            expected_missing.is_empty()
        ));
    }
    errors.extend(invalid_headers);
    if !manifest.complete {
        errors.push(format!(
            "manifest is incomplete; missing headers: {}",
            manifest.missing_headers.join(", ")
        ));
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkHeaderManifestEvidence {
        header_count: manifest.headers.len(),
        baseline: manifest.steinberg_sdk_baseline.clone(),
        upstream_vst3_crate: manifest.upstream_vst3_crate_baseline.clone(),
        version_hint: manifest.version_hint.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Vst3SdkBindingPlanEvidence {
    header_count: usize,
    status: String,
    active_backend: String,
    bindings_module: String,
}

fn vst3_sdk_binding_plan_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --out generated-bindings-plan.json` when auditing generated-header readiness"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_binding_plan(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{}; {} required header(s); active backend {}; module {}",
                evidence.status,
                evidence.header_count,
                evidence.active_backend,
                evidence.bindings_module
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings plan".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate with `vesty vst3-sdk binding-plan` from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

fn validate_vst3_sdk_binding_plan(
    path: &Utf8Path,
) -> Result<Vst3SdkBindingPlanEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK generated bindings plan", path)?;
    let plan: vesty_vst3_sys::GeneratedBindingsPlan = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK generated bindings plan JSON: {error}"))?;
    validate_vst3_sdk_binding_plan_content(&plan)
}

fn validate_vst3_sdk_binding_plan_content(
    plan: &vesty_vst3_sys::GeneratedBindingsPlan,
) -> Result<Vst3SdkBindingPlanEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_plan_shape(plan)?;
    let mut errors = Vec::new();
    if plan.version != vesty_vst3_sys::GENERATED_BINDINGS_PLAN_VERSION {
        errors.push(format!(
            "plan version is {}, expected {}",
            plan.version,
            vesty_vst3_sys::GENERATED_BINDINGS_PLAN_VERSION
        ));
    }
    if plan.generator != vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR {
        errors.push(format!(
            "plan generator is `{}`, expected `{}`",
            plan.generator,
            vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
        ));
    }
    if plan.bindings_generated {
        errors
            .push("generated bindings plan must not claim bindings are generated yet".to_string());
    }
    if plan.status != "ready-for-binding-generator" {
        errors.push(format!(
            "plan status is `{}`, expected `ready-for-binding-generator`",
            plan.status
        ));
    }
    if !plan.blockers.is_empty() {
        errors.push(format!("plan has blockers: {}", plan.blockers.join("; ")));
    }
    if plan.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        errors.push(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            plan.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        ));
    }
    if plan.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        errors.push(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            plan.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        ));
    }
    let expected_backend =
        vesty_vst3_sys::binding_backend_name(vesty_vst3_sys::BINDING_BASELINE.backend);
    if plan.active_backend != expected_backend {
        errors.push(format!(
            "active backend is `{}`, expected `{expected_backend}`",
            plan.active_backend
        ));
    }
    if !plan.bindings_module.ends_with(".rs") {
        errors.push(format!(
            "bindings module `{}` must point at a Rust .rs file",
            plan.bindings_module
        ));
    }
    if plan.sdk_dir.trim().is_empty() {
        errors.push("sdk dir is empty".to_string());
    }
    if let Err(error) = validate_vst3_sdk_header_manifest_content(&plan.header_manifest) {
        errors.push(format!("embedded header manifest invalid: {error}"));
    }
    if !plan.header_manifest.complete {
        errors.push("embedded header manifest is incomplete".to_string());
    }
    if !plan.checks.iter().any(|check| {
        check.name == "binding emitter"
            && check.status == "reserved"
            && check.value.contains("not enabled yet")
    }) {
        errors.push("plan must include reserved binding emitter check".to_string());
    }
    if plan.next_steps.is_empty() {
        errors.push("plan must include next steps".to_string());
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkBindingPlanEvidence {
        header_count: plan.header_manifest.headers.len(),
        status: plan.status.clone(),
        active_backend: plan.active_backend.clone(),
        bindings_module: plan.bindings_module.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Vst3SdkBindingSurfaceEvidence {
    header_count: usize,
    symbol_count: usize,
    status: String,
    active_backend: String,
}

fn vst3_sdk_binding_surface_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(
                "optional: generate with `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out generated-bindings-surface.json` when auditing the reserved generated binding symbol surface"
                    .to_string(),
            ),
        };
    };

    match validate_vst3_sdk_binding_surface(path) {
        Ok(evidence) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{}; {} required header(s); {} symbol(s); active backend {}; bindings generated false",
                evidence.status,
                evidence.header_count,
                evidence.symbol_count,
                evidence.active_backend
            ),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: "vst3 SDK generated bindings surface".to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate with `vesty vst3-sdk binding-surface` from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

fn validate_vst3_sdk_binding_surface(
    path: &Utf8Path,
) -> Result<Vst3SdkBindingSurfaceEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("VST3 SDK generated bindings surface", path)?;
    let surface: vesty_vst3_sys::GeneratedBindingsSurface = serde_json::from_str(&text)
        .map_err(|error| format!("invalid VST3 SDK generated bindings surface JSON: {error}"))?;
    validate_vst3_sdk_binding_surface_content(&surface)
}

fn validate_vst3_sdk_binding_surface_content(
    surface: &vesty_vst3_sys::GeneratedBindingsSurface,
) -> Result<Vst3SdkBindingSurfaceEvidence, Box<dyn std::error::Error>> {
    validate_vst3_sdk_binding_surface_shape(surface)?;
    let mut errors = Vec::new();
    if surface.version != vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_VERSION {
        errors.push(format!(
            "surface version is {}, expected {}",
            surface.version,
            vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_VERSION
        ));
    }
    if surface.generator != vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR {
        errors.push(format!(
            "surface generator is `{}`, expected `{}`",
            surface.generator,
            vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
        ));
    }
    if surface.bindings_generated {
        errors.push(
            "generated bindings surface must not claim bindings are generated yet".to_string(),
        );
    }
    if surface.status != "ready-for-binding-emitter" {
        errors.push(format!(
            "surface status is `{}`, expected `ready-for-binding-emitter`",
            surface.status
        ));
    }
    if !surface.blockers.is_empty() {
        errors.push(format!(
            "surface has blockers: {}",
            surface.blockers.join("; ")
        ));
    }
    if surface.steinberg_sdk_baseline != vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE {
        errors.push(format!(
            "Steinberg SDK baseline is `{}`, expected `{}`",
            surface.steinberg_sdk_baseline,
            vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
        ));
    }
    if surface.upstream_vst3_crate_baseline != vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE {
        errors.push(format!(
            "upstream vst3 crate baseline is `{}`, expected `{}`",
            surface.upstream_vst3_crate_baseline,
            vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
        ));
    }
    let expected_backend =
        vesty_vst3_sys::binding_backend_name(vesty_vst3_sys::BINDING_BASELINE.backend);
    if surface.active_backend != expected_backend {
        errors.push(format!(
            "active backend is `{}`, expected `{expected_backend}`",
            surface.active_backend
        ));
    }
    if surface.sdk_dir.trim().is_empty() {
        errors.push("sdk dir is empty".to_string());
    }
    if let Err(error) = validate_vst3_sdk_header_manifest_content(&surface.header_manifest) {
        errors.push(format!("embedded header manifest invalid: {error}"));
    }
    if !surface.header_manifest.complete {
        errors.push("embedded header manifest is incomplete".to_string());
    }

    let required = vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .map(|header| (*header).to_string())
        .collect::<Vec<_>>();
    let required_set = required.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if surface.required_headers != required {
        errors.push(
            "required_headers does not match the locked generated-header input set".to_string(),
        );
    }
    if !surface.missing_headers.is_empty() {
        errors.push(format!(
            "surface missing headers: {}",
            surface.missing_headers.join("; ")
        ));
    }
    let expected_missing_symbols = surface
        .symbols
        .iter()
        .filter(|symbol| symbol.header_present && !symbol.symbol_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if surface.missing_symbols != expected_missing_symbols {
        errors.push(format!(
            "surface missing_symbols does not match absent symbol tokens: expected {:?}, actual {:?}",
            expected_missing_symbols, surface.missing_symbols
        ));
    }
    if !surface.missing_symbols.is_empty() {
        errors.push(format!(
            "surface missing symbols: {}",
            surface.missing_symbols.join("; ")
        ));
    }
    if surface.symbols.is_empty() {
        errors.push("surface must include binding symbols".to_string());
    }
    let mut seen_symbols = BTreeSet::new();
    for symbol in &surface.symbols {
        if symbol.name.trim().is_empty() {
            errors.push("surface symbol has empty name".to_string());
        }
        if symbol.kind.trim().is_empty() {
            errors.push(format!("surface symbol `{}` has empty kind", symbol.name));
        }
        if !matches!(symbol.kind.as_str(), "interface" | "type" | "constant") {
            errors.push(format!(
                "surface symbol `{}` has unsupported kind `{}`",
                symbol.name, symbol.kind
            ));
        }
        if symbol.header.trim().is_empty() {
            errors.push(format!("surface symbol `{}` has empty header", symbol.name));
        } else if !required_set.contains(symbol.header.as_str()) {
            errors.push(format!(
                "surface symbol `{}` references header outside required set `{}`",
                symbol.name, symbol.header
            ));
        }
        if symbol.purpose.trim().is_empty() {
            errors.push(format!(
                "surface symbol `{}` has empty purpose",
                symbol.name
            ));
        }
        if !symbol.header_present {
            errors.push(format!(
                "surface symbol `{}` references missing header `{}`",
                symbol.name, symbol.header
            ));
        }
        if !symbol.symbol_present {
            errors.push(format!(
                "surface symbol `{}` is absent from header `{}`",
                symbol.name, symbol.header
            ));
        }
        if !seen_symbols.insert(symbol.name.as_str()) {
            errors.push(format!("duplicate surface symbol `{}`", symbol.name));
        }
    }
    let expected_symbol_names = expected_vst3_sdk_binding_surface_symbol_names();
    let actual_symbol_names = seen_symbols
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    let unknown_symbols = actual_symbol_names
        .difference(&expected_symbol_names)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_symbols.is_empty() {
        errors.push(format!(
            "surface contains unknown symbol(s): {}",
            unknown_symbols.join(", ")
        ));
    }
    let missing_symbols = expected_symbol_names
        .difference(&actual_symbol_names)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_symbols.is_empty() {
        errors.push(format!(
            "surface missing expected symbol(s): {}",
            missing_symbols.join(", ")
        ));
    }
    let expected_symbol_specs = expected_vst3_sdk_binding_surface_symbol_specs();
    for symbol in &surface.symbols {
        let Some(expected) = expected_symbol_specs.get(&symbol.name) else {
            continue;
        };
        if symbol.kind != expected.kind {
            errors.push(format!(
                "surface symbol `{}` kind is `{}`, expected `{}`",
                symbol.name, symbol.kind, expected.kind
            ));
        }
        if symbol.header != expected.header {
            errors.push(format!(
                "surface symbol `{}` header is `{}`, expected `{}`",
                symbol.name, symbol.header, expected.header
            ));
        }
        if symbol.purpose != expected.purpose {
            errors.push(format!(
                "surface symbol `{}` purpose is `{}`, expected `{}`",
                symbol.name, symbol.purpose, expected.purpose
            ));
        }
    }
    for required_symbol in [
        "IPlugView",
        "IAudioProcessor",
        "IEditController",
        "IParameterChanges",
        "IEventList",
        "IMidiMapping",
        "IBStream",
    ] {
        if !seen_symbols.contains(required_symbol) {
            errors.push(format!(
                "surface missing required symbol `{required_symbol}`"
            ));
        }
    }
    if surface.notes.is_empty() {
        errors.push("surface must include audit notes".to_string());
    } else if !surface
        .notes
        .iter()
        .any(|note| note.contains("does not") || note.contains("false"))
    {
        errors.push("surface notes must clarify that bindings are not generated yet".to_string());
    }
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }

    Ok(Vst3SdkBindingSurfaceEvidence {
        header_count: surface.header_manifest.headers.len(),
        symbol_count: surface.symbols.len(),
        status: surface.status.clone(),
        active_backend: surface.active_backend.clone(),
    })
}

fn vst3_sdk_generated_scaffold_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings scaffold",
        "optional: generate with `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out generated.rs` when auditing generated-header scaffold drift",
        "metadata scaffold; bindings generated false",
        validate_vst3_sdk_generated_bindings_scaffold_text,
    )
}

fn vst3_sdk_generated_abi_seed_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings ABI seed",
        "optional: generate with `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out generated-abi-seed.rs` when auditing foundational ABI aliases/constants",
        "ABI seed aliases/constants; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_abi_seed_text,
    )
}

fn vst3_sdk_generated_abi_release_check(path: Option<&Utf8Path>) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings ABI layout",
        "optional: generate with `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out generated-abi.rs` when auditing foundational ABI layout fingerprints",
        "ABI layout fingerprints present; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_abi_text,
    )
}

fn vst3_sdk_generated_interface_skeleton_release_check(
    path: Option<&Utf8Path>,
) -> ReleaseCheckItem {
    optional_vst3_sdk_rust_artifact_release_check(
        path,
        "vst3 SDK generated bindings interface skeleton",
        "optional: generate with `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out generated-interface-skeleton.rs` when auditing interface/vtable skeleton metadata",
        "interface/vtable skeleton metadata present; bindings generated false; full COM bindings generated false",
        validate_vst3_sdk_generated_bindings_interface_skeleton_text,
    )
}

fn optional_vst3_sdk_rust_artifact_release_check(
    path: Option<&Utf8Path>,
    name: &str,
    missing_hint: &str,
    ok_value: &str,
    validate: fn(&str) -> Result<(), String>,
) -> ReleaseCheckItem {
    let Some(path) = path else {
        return ReleaseCheckItem {
            name: name.to_string(),
            status: "skipped".to_string(),
            value: "not requested".to_string(),
            hint: Some(missing_hint.to_string()),
        };
    };

    match read_text_file_no_symlink(name, path).and_then(|text| {
        validate(&text)
            .map(|_| ())
            .map_err(|error| -> Box<dyn std::error::Error> { error.into() })
    }) {
        Ok(()) => ReleaseCheckItem {
            name: name.to_string(),
            status: "ok".to_string(),
            value: ok_value.to_string(),
            hint: None,
        },
        Err(error) => ReleaseCheckItem {
            name: name.to_string(),
            status: "failed".to_string(),
            value: format!("{}: {error}", portable_report_path(path)),
            hint: Some(
                "regenerate this optional VST3 SDK audit artifact from the official Steinberg VST3 SDK checkout"
                    .to_string(),
            ),
        },
    }
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn ci_run_url_release_check(url: Option<&str>, required: bool) -> ReleaseCheckItem {
    let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
        return optional_release_check_missing(
            "ci run url",
            required,
            "pass `--ci-run-url https://github.com/<org>/<repo>/actions/runs/<id>` for release evidence",
        );
    };

    if is_github_actions_run_url(url) {
        ReleaseCheckItem {
            name: "ci run url".to_string(),
            status: "ok".to_string(),
            value: url.to_string(),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "ci run url".to_string(),
            status: "failed".to_string(),
            value: url.to_string(),
            hint: Some(
                "expected a GitHub Actions run URL, not a local log or project page".to_string(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GithubActionsRunKey {
    owner: String,
    repo: String,
    run_id: String,
}

fn is_github_actions_run_url(url: &str) -> bool {
    github_actions_run_key(url).is_some()
}

fn github_actions_run_key(url: &str) -> Option<GithubActionsRunKey> {
    if url.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = url.strip_prefix("https://github.com/")?;
    let path = rest.split(['?', '#']).next().unwrap_or_default();
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 5
        || segments[0].is_empty()
        || segments[1].is_empty()
        || segments[2] != "actions"
        || segments[3] != "runs"
        || segments[4].is_empty()
        || !segments[4].chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let valid_shape = matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _] | [_, _, "actions", "runs", _, ""]
    ) || matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _, "attempts", attempt]
            if attempt.chars().all(|ch| ch.is_ascii_digit())
    ) || matches!(
        segments.as_slice(),
        [_, _, "actions", "runs", _, "attempts", attempt, ""]
            if attempt.chars().all(|ch| ch.is_ascii_digit())
    );

    valid_shape.then(|| GithubActionsRunKey {
        owner: segments[0].to_string(),
        repo: segments[1].to_string(),
        run_id: segments[4].to_string(),
    })
}

fn ci_doctor_artifacts_release_check(
    dir: Option<&Utf8Path>,
    required: bool,
    ci_run_url: Option<&str>,
) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "ci doctor artifacts",
            required,
            "download/upload Linux, macOS and Windows doctor JSON artifacts and pass `--ci-doctor-dir <dir>`",
        );
    };

    let reports = match collect_doctor_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "ci doctor artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some("expected parseable doctor JSON artifacts from CI".to_string()),
            };
        }
    };

    let mut invalid_reports = Vec::new();
    let mut duplicate_os = Vec::new();
    let mut seen_os = BTreeSet::new();
    let valid_reports = reports
        .iter()
        .filter_map(|artifact| {
            if let Err(error) = validate_doctor_report(&artifact.report) {
                invalid_reports.push(format!("{}: {error}", artifact.path));
                return None;
            }
            if let Some(os) = artifact.os
                && !seen_os.insert(os)
            {
                duplicate_os.push(os);
                return None;
            }
            Some(artifact.clone())
        })
        .collect::<Vec<_>>();

    let coverage = DoctorArtifactCoverage::from_reports(&valid_reports, ci_run_url);
    let missing_os = coverage.missing_os();
    let os_mismatches = coverage.os_mismatches();
    let missing_checks = coverage.missing_checks();
    let run_mismatches = coverage.run_mismatches();
    if missing_os.is_empty()
        && invalid_reports.is_empty()
        && duplicate_os.is_empty()
        && os_mismatches.is_empty()
        && missing_checks.is_empty()
        && run_mismatches.is_empty()
    {
        ReleaseCheckItem {
            name: "ci doctor artifacts".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} doctor reports parsed ({})",
                reports.len(),
                coverage.present_os().join(", ")
            ),
            hint: None,
        }
    } else {
        let mut value = Vec::new();
        if !missing_os.is_empty() {
            value.push(format!("missing OS reports: {}", missing_os.join(", ")));
        }
        if !invalid_reports.is_empty() {
            value.push(format!("invalid reports: {}", invalid_reports.join("; ")));
        }
        if !duplicate_os.is_empty() {
            duplicate_os.sort_unstable();
            duplicate_os.dedup();
            value.push(format!("duplicate OS reports: {}", duplicate_os.join(", ")));
        }
        if !os_mismatches.is_empty() {
            value.push(format!("OS mismatch: {}", os_mismatches.join("; ")));
        }
        if !missing_checks.is_empty() {
            value.push(format!("missing checks: {}", missing_checks.join("; ")));
        }
        if !run_mismatches.is_empty() {
            value.push(format!("run URL mismatch: {}", run_mismatches.join("; ")));
        }
        ReleaseCheckItem {
            name: "ci doctor artifacts".to_string(),
            status: "failed".to_string(),
            value: value.join("; "),
            hint: Some(
                "CI should upload Linux, macOS and Windows doctor JSON snapshots from the same GitHub Actions run"
                    .to_string(),
            ),
        }
    }
}

fn ci_release_check_artifacts_release_check(
    dir: Option<&Utf8Path>,
    required: bool,
    ci_run_url: Option<&str>,
) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "ci release-check artifacts",
            required,
            "pass `--ci-release-check-dir <dir>` containing Linux, macOS and Windows release-check*.json artifacts from CI",
        );
    };

    let reports = match collect_ci_release_check_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "ci release-check artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some("expected parseable release-check JSON artifacts from CI".to_string()),
            };
        }
    };

    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    let mut failures = Vec::new();
    let expected_run = ci_run_url.and_then(github_actions_run_key);
    for artifact in &reports {
        let Some(os) = artifact.os else {
            failures.push(format!(
                "{}: could not infer OS from artifact path",
                portable_report_path(&artifact.path)
            ));
            continue;
        };
        if !seen.insert(os) {
            duplicates.push(os);
        }
        if let Err(error) = validate_ci_release_check_report(&artifact.report) {
            failures.push(format!("{os} {}: {error}", artifact.path));
        }
        if let Err(error) = validate_ci_release_check_report_os_matches_path(artifact) {
            failures.push(error);
        }
        if let Some(expected) = expected_run.as_ref() {
            match artifact.report.ci_run_url.as_deref() {
                Some(actual_url) => match github_actions_run_key(actual_url) {
                    Some(actual) if actual == *expected => {}
                    Some(actual) => failures.push(format!(
                        "{os} {}: expected {}/{} run {}, got {}/{} run {}",
                        artifact.path,
                        expected.owner,
                        expected.repo,
                        expected.run_id,
                        actual.owner,
                        actual.repo,
                        actual.run_id
                    )),
                    None => failures.push(format!(
                        "{os} {}: invalid ci_run_url `{actual_url}`",
                        artifact.path
                    )),
                },
                None => failures.push(format!("{}: missing ci_run_url", artifact.path)),
            }
        }
    }

    let missing_os = ["Linux", "macOS", "Windows"]
        .into_iter()
        .filter(|os| !seen.contains(os))
        .collect::<Vec<_>>();

    if missing_os.is_empty() && duplicates.is_empty() && failures.is_empty() {
        ReleaseCheckItem {
            name: "ci release-check artifacts".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} release-check report(s) parsed ({})",
                reports.len(),
                seen.into_iter().collect::<Vec<_>>().join(", ")
            ),
            hint: None,
        }
    } else {
        let mut value = Vec::new();
        if !missing_os.is_empty() {
            value.push(format!("missing OS reports: {}", missing_os.join(", ")));
        }
        if !duplicates.is_empty() {
            duplicates.sort_unstable();
            duplicates.dedup();
            value.push(format!("duplicate OS reports: {}", duplicates.join(", ")));
        }
        if !failures.is_empty() {
            value.push(format!("invalid reports: {}", failures.join("; ")));
        }
        ReleaseCheckItem {
            name: "ci release-check artifacts".to_string(),
            status: "failed".to_string(),
            value: value.join("; "),
            hint: Some(
                "CI should upload Linux, macOS and Windows release-check*.json snapshots; external evidence gaps may remain failed, but local invariant checks must pass"
                    .to_string(),
            ),
        }
    }
}

const REQUIRED_PLATFORM_SMOKE_PLATFORMS: [(&str, &str); 3] = [
    ("macos", "macOS"),
    ("windows-x64", "Windows x64"),
    ("linux-x11", "Linux X11"),
];

const REQUIRED_PLATFORM_SMOKE_CHECKS: [(&str, &str); 8] = [
    ("system_webview", "system WebView"),
    ("vst3_validator", "VST3 validator"),
    ("vst3_example_scan", "VST3 example scan"),
    ("webview_attach", "WebView attach"),
    ("webview_resize", "WebView resize"),
    ("asset_protocol", "asset protocol"),
    ("jsbridge_roundtrip", "JSBridge roundtrip"),
    ("meter_stream", "meter stream"),
];
const PLATFORM_SMOKE_MAX_CHECKS: usize = 32;

fn expected_platform_smoke_check_names() -> BTreeSet<String> {
    REQUIRED_PLATFORM_SMOKE_CHECKS
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PlatformSmokeReport {
    platform: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    checks: Vec<PlatformSmokeCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PlatformSmokeCheck {
    name: String,
    status: String,
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct PlatformSmokeReportInput {
    platform: Option<String>,
    os: Option<String>,
    host: Option<String>,
    system_webview: Option<String>,
    vst3_validator: Option<String>,
    vst3_example_scan: Option<String>,
    webview_attach: Option<String>,
    webview_resize: Option<String>,
    asset_protocol: Option<String>,
    jsbridge_roundtrip: Option<String>,
    meter_stream: Option<String>,
}

fn write_platform_smoke_report(
    dir: &Utf8Path,
    input: PlatformSmokeReportInput,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let platform_raw = input
        .platform
        .as_deref()
        .ok_or("platform smoke report requires `--platform <macos|windows-x64|linux-x11>`")?;
    let platform = normalize_platform_smoke_platform(platform_raw)
        .ok_or_else(|| format!("unsupported platform `{platform_raw}`"))?;
    let platform_label = REQUIRED_PLATFORM_SMOKE_PLATFORMS
        .iter()
        .find_map(|(key, label)| (*key == platform).then_some(*label))
        .unwrap_or(platform);

    let report = PlatformSmokeReport {
        platform: platform.to_string(),
        os: input.os.or_else(|| Some(platform_label.to_string())),
        host: input.host,
        checks: vec![
            platform_smoke_check_from_value("system_webview", input.system_webview)?,
            platform_smoke_check_from_value("vst3_validator", input.vst3_validator)?,
            platform_smoke_check_from_value("vst3_example_scan", input.vst3_example_scan)?,
            platform_smoke_check_from_value("webview_attach", input.webview_attach)?,
            platform_smoke_check_from_value("webview_resize", input.webview_resize)?,
            platform_smoke_check_from_value("asset_protocol", input.asset_protocol)?,
            platform_smoke_check_from_value("jsbridge_roundtrip", input.jsbridge_roundtrip)?,
            platform_smoke_check_from_value("meter_stream", input.meter_stream)?,
        ],
    };
    validate_platform_smoke_report(&report)?;

    create_directory_no_parent_or_leaf_symlink("platform smoke report dir", dir)?;
    let path = dir.join(format!("{platform}.json"));
    write_text_file(&path, &(serde_json::to_string_pretty(&report)? + "\n"))?;
    Ok(path)
}

fn platform_smoke_check_from_value(
    name: &str,
    value: Option<String>,
) -> Result<PlatformSmokeCheck, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Err(format!(
            "platform smoke report requires `--{}`",
            name.replace('_', "-")
        )
        .into());
    };
    Ok(PlatformSmokeCheck {
        name: name.to_string(),
        status: "ok".to_string(),
        value,
        hint: None,
    })
}

fn platform_smoke_release_check(dir: Option<&Utf8Path>, required: bool) -> ReleaseCheckItem {
    let Some(dir) = dir else {
        return optional_release_check_missing(
            "platform smoke artifacts",
            required,
            "pass `--platform-smoke-dir <dir>` containing macOS, Windows x64 and Linux X11 platform smoke JSON reports",
        );
    };

    let reports = match collect_platform_smoke_reports(dir) {
        Ok(reports) => reports,
        Err(error) => {
            return ReleaseCheckItem {
                name: "platform smoke artifacts".to_string(),
                status: "failed".to_string(),
                value: error.to_string(),
                hint: Some(
                    "expected parseable `vesty platform-smoke --format json` reports".to_string(),
                ),
            };
        }
    };

    let mut platforms = BTreeSet::new();
    let mut failures = Vec::new();
    for (path, report) in &reports {
        if let Err(error) = validate_platform_smoke_report_shape(report) {
            failures.push(format!("{path}: {error}"));
            continue;
        }
        if platform_smoke_report_is_pending_template(report) {
            continue;
        }
        let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
            failures.push(format!(
                "{path}: unsupported platform `{}`",
                report.platform
            ));
            continue;
        };
        match platform_smoke_platform_from_artifact_path(path) {
            Ok(Some(path_platform)) if path_platform != platform => {
                failures.push(format!(
                    "{path}: artifact path indicates {path_platform}, but report platform is {platform}"
                ));
                continue;
            }
            Ok(_) => {}
            Err(error) => {
                failures.push(format!("{path}: {error}"));
                continue;
            }
        }
        if !platforms.insert(platform) {
            failures.push(format!(
                "{path}: duplicate platform smoke report for {platform}"
            ));
        }
        if let Err(error) = validate_platform_smoke_report(report) {
            failures.push(format!("{path}: {error}"));
        }
    }

    if !failures.is_empty() {
        return ReleaseCheckItem {
            name: "platform smoke artifacts".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "collect real platform smoke with system WebView, validator, editor attach/resize, JSBridge and meter evidence"
                    .to_string(),
            ),
        };
    }

    if platforms.is_empty() {
        return optional_release_check_missing(
            "platform smoke artifacts",
            required,
            "no passing platform smoke reports found",
        );
    }

    let missing = REQUIRED_PLATFORM_SMOKE_PLATFORMS
        .iter()
        .filter_map(|(platform, label)| (!platforms.contains(platform)).then_some(*label))
        .collect::<Vec<_>>();
    if required && !missing.is_empty() {
        return ReleaseCheckItem {
            name: "platform smoke artifacts".to_string(),
            status: "failed".to_string(),
            value: format!("missing platform smoke reports: {}", missing.join(", ")),
            hint: Some(
                "run platform smoke on macOS, Windows x64 and Linux X11; Wayland remains experimental"
                    .to_string(),
            ),
        };
    }

    let labels = platforms
        .iter()
        .filter_map(|platform| {
            REQUIRED_PLATFORM_SMOKE_PLATFORMS
                .iter()
                .find_map(|(key, label)| (*key == *platform).then_some(*label))
        })
        .collect::<Vec<_>>();
    ReleaseCheckItem {
        name: "platform smoke artifacts".to_string(),
        status: "ok".to_string(),
        value: format!(
            "{} platform smoke report(s): {}",
            platforms.len(),
            labels.join(", ")
        ),
        hint: (!missing.is_empty()).then(|| {
            format!(
                "full release coverage still needs platform smoke for {}",
                missing.join(", ")
            )
        }),
    }
}

fn collect_platform_smoke_reports(
    root: &Utf8Path,
) -> Result<Vec<(Utf8PathBuf, PlatformSmokeReport)>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("platform smoke path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
    };
    if files.is_empty() {
        return Err(format!("no JSON platform smoke artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<PlatformSmokeReport>(&text)
                .map_err(|error| format!("invalid platform smoke JSON: {error}"))?;
            Ok((path, report))
        })
        .collect()
}

fn validate_platform_smoke_report(
    report: &PlatformSmokeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_platform_smoke_report_shape(report)?;
    let Some(platform) = normalize_platform_smoke_platform(&report.platform) else {
        return Err(format!("unsupported platform `{}`", report.platform).into());
    };
    validate_platform_smoke_os_matches_platform(report, platform)?;

    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for (key, label) in REQUIRED_PLATFORM_SMOKE_CHECKS {
        let Some(check) = report
            .checks
            .iter()
            .find(|check| normalize_platform_smoke_check_name(&check.name) == key)
        else {
            missing.push(label);
            continue;
        };
        if let Err(error) = validate_platform_smoke_check(platform, key, check) {
            failed.push(format!("{label}: {error}"));
        }
    }

    if !missing.is_empty() || !failed.is_empty() {
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("missing checks: {}", missing.join(", ")));
        }
        if !failed.is_empty() {
            parts.push(format!("failed checks: {}", failed.join("; ")));
        }
        return Err(parts.join("; ").into());
    }
    Ok(())
}

fn validate_platform_smoke_os_matches_platform(
    report: &PlatformSmokeReport,
    platform: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(os) = report.os.as_deref() else {
        return Ok(());
    };
    let tokens = artifact_path_tokens(Utf8Path::new(os));
    let matches_platform = match platform {
        "macos" => tokens
            .iter()
            .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx")),
        "windows-x64" => tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "windows" | "windowsx64" | "win" | "win32" | "win64"
            )
        }),
        "linux-x11" => {
            tokens.iter().any(|token| token == "linux")
                && tokens.iter().any(|token| token == "x11")
                && !tokens.iter().any(|token| token == "wayland")
        }
        _ => false,
    };
    if matches_platform {
        Ok(())
    } else {
        Err(format!("platform smoke os `{os}` does not match platform `{platform}`").into())
    }
}

fn validate_platform_smoke_report_shape(
    report: &PlatformSmokeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("platform smoke platform", &report.platform)?;
    if let Some(os) = &report.os {
        validate_release_action_text("platform smoke os", os)?;
    }
    if let Some(host) = &report.host {
        validate_release_action_text("platform smoke host", host)?;
    }
    if report.checks.is_empty() {
        return Err("platform smoke report has no checks".into());
    }
    if report.checks.len() > PLATFORM_SMOKE_MAX_CHECKS {
        return Err(format!(
            "platform smoke report has too many checks: {} exceeds maximum {PLATFORM_SMOKE_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("platform smoke check name", &check.name)?;
        validate_release_action_text(
            &format!("platform smoke `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(
            &format!("platform smoke `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("platform smoke `{}` hint", check.name), hint)?;
        }

        let normalized = normalize_platform_smoke_check_name(&check.name);
        if normalized.is_empty() {
            return Err(format!(
                "platform smoke check `{}` does not normalize to a stable check name",
                check.name
            )
            .into());
        }
        if !seen_checks.insert(normalized.clone()) {
            return Err(format!("duplicate platform smoke check `{normalized}`").into());
        }
    }
    let expected_checks = expected_platform_smoke_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!(
            "unknown platform smoke check(s): {}",
            unknown_checks.join(", ")
        )
        .into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "platform smoke report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

fn platform_smoke_report_is_pending_template(report: &PlatformSmokeReport) -> bool {
    !report.checks.is_empty()
        && report.checks.iter().all(|check| {
            check.status.trim().eq_ignore_ascii_case("pending")
                || check
                    .value
                    .to_ascii_lowercase()
                    .contains("replace with real")
        })
}

fn validate_platform_smoke_check(
    platform: &str,
    key: &str,
    check: &PlatformSmokeCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    if check.status.trim().eq_ignore_ascii_case("pending")
        || check.status.trim().eq_ignore_ascii_case("skipped")
        || !check.status.trim().eq_ignore_ascii_case("ok")
    {
        return Err(format!("status is {}", check.status).into());
    }
    let value = check.value.trim();
    let lower_value = value.to_ascii_lowercase();
    if value.is_empty()
        || value.eq_ignore_ascii_case("pending")
        || value.eq_ignore_ascii_case("false")
        || lower_value.starts_with("pending ")
        || lower_value.contains("replace with real")
        || platform_smoke_value_has_contradiction(key, value)
    {
        return Err("missing positive evidence value".into());
    }

    let ok = match key {
        "system_webview" => platform_system_webview_evidence_ok(platform, value),
        "vst3_validator" => platform_vst3_validator_evidence_ok(value),
        "vst3_example_scan" => generic_scan_ok(value),
        "webview_attach" => explicit_truthy_marker(value, &["webview_attach", "attach"]),
        "webview_resize" => explicit_truthy_marker(value, &["webview_resize", "resize"]),
        "asset_protocol" => explicit_truthy_marker(
            value,
            &["asset_protocol", "asset_manifest", "custom_protocol"],
        ),
        "jsbridge_roundtrip" => {
            explicit_truthy_marker(
                value,
                &["jsbridge_roundtrip", "bridge_roundtrip", "roundtrip"],
            ) || bridge_trace_relayed_param_gesture(value)
                || (value.contains("readyAck") && value.contains("reply"))
        }
        "meter_stream" => meter_stream_delivered(value),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("value does not match accepted smoke evidence markers".into())
    }
}

fn platform_smoke_value_has_contradiction(key: &str, value: &str) -> bool {
    if daw_marker_has_missing_assignment(value) {
        return true;
    }
    if key == "vst3_validator" {
        return false;
    }
    daw_marker_has_negative_evidence(value)
}

fn platform_system_webview_evidence_ok(platform: &str, value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.contains("not found")
        || lower.contains("missing")
        || lower.contains("unavailable")
        || lower.contains("not installed")
        || lower.contains("failed")
        || lower.contains("error")
        || explicit_truthy_marker(value, &["system_webview", "webview"])
        || platform_system_webview_has_negative_platform_evidence(platform, &lower)
    {
        return false;
    }

    match platform {
        "macos" => lower.contains("webkit.framework") || lower.contains("wkwebview"),
        "windows-x64" => lower.contains("webview2"),
        "linux-x11" => lower.contains("webkitgtk") && lower.contains("x11"),
        _ => false,
    }
}

fn platform_system_webview_has_negative_platform_evidence(platform: &str, lower: &str) -> bool {
    match platform {
        "macos" => [
            "not webkit.framework",
            "no webkit.framework",
            "without webkit.framework",
            "not wkwebview",
            "no wkwebview",
            "without wkwebview",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        "windows-x64" => [
            "not webview2",
            "no webview2",
            "without webview2",
            "webview2 disabled",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        "linux-x11" => [
            "wayland",
            "not x11",
            "no x11",
            "without x11",
            "x11 disabled",
            "x11 unavailable",
            "not webkitgtk",
            "no webkitgtk",
            "without webkitgtk",
            "webkitgtk disabled",
            "experimental",
            "fallback",
        ]
        .iter()
        .any(|needle| lower.contains(needle)),
        _ => true,
    }
}

fn platform_vst3_validator_evidence_ok(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if vst3_validator_has_runtime_failure(&lower)
        || explicit_truthy_marker(value, &["validator", "vst3_validator"])
    {
        return false;
    }

    let has_validator_marker = lower.contains("steinberg") || lower.contains("vst3 validator");
    let Some((passed, failed)) = smoke_validator_test_summary(value) else {
        return false;
    };
    has_validator_marker && passed > 0 && failed == 0
}

fn vst3_validator_has_runtime_failure(lower: &str) -> bool {
    [
        "not found",
        "missing",
        "unavailable",
        "not installed",
        "failed to run",
        "validator error",
        "vst3 validator error",
        "validator timeout",
        "validator timed out",
        "vst3 validator timeout",
        "vst3 validator timed out",
        "validator crashed",
        "vst3 validator crashed",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn smoke_validator_test_summary(text: &str) -> Option<(u32, u32)> {
    let mut passed = None;
    let mut failed = None;
    for line in text.lines() {
        let line = line.to_ascii_lowercase();
        if passed.is_none() {
            passed = extract_count_after_any_marker(&line, &["passed=", "passed:"]).or_else(|| {
                extract_count_near_any_marker(
                    &line,
                    &[
                        "tests passed",
                        "test passed",
                        "passed tests",
                        "passed test",
                        "passed",
                    ],
                )
            });
        }
        if failed.is_none() {
            failed = extract_count_after_any_marker(&line, &["failed=", "failed:"]).or_else(|| {
                extract_count_near_any_marker(
                    &line,
                    &[
                        "tests failed",
                        "test failed",
                        "failed tests",
                        "failed test",
                        "failed",
                    ],
                )
            });
        }
        if let (Some(passed), Some(failed)) = (passed, failed) {
            return Some((passed, failed));
        }
    }
    None
}

fn extract_count_after_any_marker(line: &str, markers: &[&str]) -> Option<u32> {
    markers
        .iter()
        .find_map(|marker| extract_count_after_marker(line, marker))
}

fn extract_count_after_marker(line: &str, marker: &str) -> Option<u32> {
    let index = line.find(marker)?;
    first_number_after(&line[index + marker.len()..])
}

fn normalize_platform_smoke_platform(value: &str) -> Option<&'static str> {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "macos" | "darwin" | "osx" => Some("macos"),
        "windowsx64" | "windows" | "win64" => Some("windows-x64"),
        value if value.contains("linux") && value.contains("x11") => Some("linux-x11"),
        _ => None,
    }
}

fn platform_smoke_platform_from_artifact_path(
    path: &Utf8Path,
) -> Result<Option<&'static str>, String> {
    let tokens = artifact_path_tokens(path);
    let mut platforms = Vec::new();
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "macos" | "mac" | "darwin" | "osx"))
    {
        platforms.push("macos");
    }
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "windows" | "windowsx64" | "win" | "win32" | "win64"
        )
    }) {
        platforms.push("windows-x64");
    }
    if tokens.iter().any(|token| token == "linux") && tokens.iter().any(|token| token == "x11") {
        platforms.push("linux-x11");
    }
    match platforms.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "artifact path contains multiple platform tokens: {}",
            platforms.join(", ")
        )),
    }
}

fn normalize_platform_smoke_check_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

const SMOKE_HOST_GENERATOR: &str = "vesty-cli.smoke-host.v1";
const SMOKE_HOST_EXAMPLES: [(&str, &str, bool); 3] = [
    ("gain", "Vesty Gain", false),
    ("midi-synth", "Vesty MIDI Synth", false),
    ("web-ui-param-demo", "Vesty Web UI Demo", true),
];
const SMOKE_HOST_MAX_CHECKS: usize = 64;

fn expected_smoke_host_check_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.insert(normalize_smoke_host_check_name("workspace manifest"));
    for (example, _, has_ui) in SMOKE_HOST_EXAMPLES {
        names.insert(normalize_smoke_host_check_name(&format!(
            "{example} config"
        )));
        names.insert(normalize_smoke_host_check_name(&format!(
            "{example} parameter sidecar"
        )));
        if has_ui {
            names.insert(normalize_smoke_host_check_name(&format!(
                "{example} UI assets"
            )));
        }
    }
    names.insert(normalize_smoke_host_check_name("JSBridge trace"));
    names.insert(normalize_smoke_host_check_name("meter stream"));
    names
}

#[derive(Clone, Debug)]
struct SmokeHostOptions {
    workspace: Utf8PathBuf,
    bridge_trace: Option<Utf8PathBuf>,
    meter_log: Option<Utf8PathBuf>,
    out: Option<Utf8PathBuf>,
    check: bool,
    strict: bool,
    format: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SmokeHostReport {
    version: u32,
    generator: String,
    workspace: String,
    status: String,
    checks: Vec<SmokeHostCheck>,
    external_evidence_note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SmokeHostCheck {
    name: String,
    status: String,
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

fn run_smoke_host(options: SmokeHostOptions) -> Result<(), Box<dyn std::error::Error>> {
    let format = parse_output_format(&options.format)?;
    let report = build_smoke_host_report(
        &options.workspace,
        options.bridge_trace.as_deref(),
        options.meter_log.as_deref(),
    );
    validate_smoke_host_report(&report)?;

    if options.check {
        let out = options
            .out
            .as_deref()
            .ok_or("--check requires --out <smoke-host.json>")?;
        let text = read_text_file_no_symlink("smoke-host report", out)?;
        let expected: SmokeHostReport = serde_json::from_str(&text)?;
        validate_smoke_host_report(&expected)?;
        if expected != report {
            return Err(format!("smoke-host report is out of date: {out}").into());
        }
    } else if let Some(out) = options.out.as_deref() {
        write_text_file(out, &(serde_json::to_string_pretty(&report)? + "\n"))?;
    }

    print_smoke_host_report(&report, format, options.out.as_deref())?;
    if options.strict && !smoke_host_report_all_ok(&report) {
        return Err("smoke-host checks are incomplete; fix failed/skipped local checks".into());
    }
    Ok(())
}

fn build_smoke_host_report(
    workspace: &Utf8Path,
    bridge_trace: Option<&Utf8Path>,
    meter_log: Option<&Utf8Path>,
) -> SmokeHostReport {
    let workspace_root = canonicalize_utf8_or_original(workspace);
    let mut checks = vec![smoke_host_workspace_check(&workspace_root)];
    for (example, expected_name, has_ui) in SMOKE_HOST_EXAMPLES {
        checks.extend(smoke_host_example_checks(
            &workspace_root,
            example,
            expected_name,
            has_ui,
        ));
    }
    checks.push(smoke_host_bridge_trace_check(bridge_trace));
    checks.push(smoke_host_meter_log_check(meter_log));

    let status = if checks.iter().any(|check| check.status == "failed") {
        "failed"
    } else if checks.iter().any(|check| check.status == "skipped") {
        "partial"
    } else {
        "ok"
    };

    SmokeHostReport {
        version: 1,
        generator: SMOKE_HOST_GENERATOR.to_string(),
        workspace: workspace_root.to_string(),
        status: status.to_string(),
        checks,
        external_evidence_note: "Vesty smoke-host is a local headless framework self-check. It does not load plugin binaries and does not replace real DAW, platform WebView, Steinberg validator, signing or notarization evidence.".to_string(),
    }
}

fn canonicalize_utf8_or_original(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize()
        .ok()
        .and_then(|path| Utf8PathBuf::from_path_buf(path).ok())
        .unwrap_or_else(|| path.to_path_buf())
}

fn smoke_host_workspace_check(workspace: &Utf8Path) -> SmokeHostCheck {
    let manifest = workspace.join("Cargo.toml");
    if manifest.is_file() {
        smoke_host_ok(
            "workspace manifest",
            format!("workspace Cargo.toml found at {manifest}"),
        )
    } else {
        smoke_host_failed(
            "workspace manifest",
            format!("missing workspace Cargo.toml at {manifest}"),
            "run smoke-host from the Vesty workspace root or pass --workspace <path>",
        )
    }
}

fn smoke_host_example_checks(
    workspace: &Utf8Path,
    example: &str,
    expected_name: &str,
    has_ui: bool,
) -> Vec<SmokeHostCheck> {
    let example_dir = workspace.join("examples").join(example);
    let config_path = example_dir.join("vesty.toml");
    let specs_path = example_dir.join("params.specs.json");
    let manifest_path = example_dir.join("vesty-parameters.json");
    let mut checks = Vec::new();

    let config = match read_config(&config_path) {
        Ok(config) => {
            let mut failures = Vec::new();
            if config.plugin.name != expected_name {
                failures.push(format!(
                    "expected plugin name `{expected_name}`, got `{}`",
                    config.plugin.name
                ));
            }
            if config
                .package
                .as_ref()
                .and_then(|package| package.parameter_manifest.as_deref())
                != Some("vesty-parameters.json")
            {
                failures.push(
                    "missing [package].parameter_manifest = \"vesty-parameters.json\"".to_string(),
                );
            }
            if has_ui && config.ui.is_none() {
                failures.push("missing [ui] section".to_string());
            }
            if failures.is_empty() {
                checks.push(smoke_host_ok(
                    format!("{example} config"),
                    format!(
                        "{} v{} ({})",
                        config.plugin.name, config.plugin.version, config.plugin.kind
                    ),
                ));
            } else {
                checks.push(smoke_host_failed(
                    format!("{example} config"),
                    failures.join("; "),
                    "fix example vesty.toml metadata",
                ));
            }
            Some(config)
        }
        Err(error) => {
            checks.push(smoke_host_failed(
                format!("{example} config"),
                format!("{config_path}: {error}"),
                "add a valid vesty.toml for the example",
            ));
            None
        }
    };

    match smoke_host_parameter_sidecar_value(&specs_path, &manifest_path) {
        Ok(value) => checks.push(smoke_host_ok(format!("{example} parameter sidecar"), value)),
        Err(error) => checks.push(smoke_host_failed(
            format!("{example} parameter sidecar"),
            error,
            "run `vesty param-manifest --specs params.specs.json --out vesty-parameters.json` in the example",
        )),
    }

    if has_ui {
        let ui_check = config
            .as_ref()
            .and_then(|config| config.ui.as_ref())
            .map(|ui| smoke_host_ui_assets_check(&example_dir, example, ui))
            .unwrap_or_else(|| {
                smoke_host_failed(
                    format!("{example} UI assets"),
                    "missing UI config".to_string(),
                    "restore the example [ui] section",
                )
            });
        checks.push(ui_check);
    }

    checks
}

fn smoke_host_parameter_sidecar_value(
    specs_path: &Utf8Path,
    manifest_path: &Utf8Path,
) -> Result<String, String> {
    let specs_text = read_text_file_no_symlink("parameter specs", specs_path)
        .map_err(|error| format!("failed to read {specs_path}: {error}"))?;
    let expected = parameter_manifest_from_specs_json(&specs_text)
        .map_err(|error| format!("invalid parameter specs {specs_path}: {error}"))?;
    let actual = read_parameter_manifest(manifest_path)
        .map_err(|error| format!("invalid parameter manifest {manifest_path}: {error}"))?;
    if actual != expected {
        return Err(format!(
            "parameter manifest is out of date: {manifest_path}; regenerate from {specs_path}"
        ));
    }
    Ok(format!(
        "{} parameter(s), idAlgorithm {}",
        actual.parameters.len(),
        actual.id_algorithm
    ))
}

fn smoke_host_ui_assets_check(
    example_dir: &Utf8Path,
    example: &str,
    ui: &UiConfig,
) -> SmokeHostCheck {
    let dist = ui.dist.as_deref().unwrap_or("dist");
    let root = example_dir.join(&ui.dir).join(dist);
    match AssetManifest::from_dir(&root, "index.html") {
        Ok(manifest) => smoke_host_ok(
            format!("{example} UI assets"),
            format!("{} file(s), entry {}", manifest.files.len(), manifest.entry),
        ),
        Err(error) => smoke_host_failed(
            format!("{example} UI assets"),
            format!("{}: {error}", portable_report_path(&root)),
            "run the example UI build command before smoke-host",
        ),
    }
}

fn smoke_host_bridge_trace_check(path: Option<&Utf8Path>) -> SmokeHostCheck {
    let Some(path) = path else {
        return smoke_host_skipped(
            "JSBridge trace",
            "no --bridge-trace evidence supplied",
            "pass a bridge trace containing readyAck/reply or param gesture packets",
        );
    };
    match read_text_file_no_symlink("bridge trace", path) {
        Ok(text)
            if daw_marker_positive(&text)
                && (bridge_trace_relayed_param_gesture(&text)
                    || (text.contains("readyAck") && text.contains("reply"))) =>
        {
            smoke_host_ok(
                "JSBridge trace",
                format!("accepted bridge evidence from {path}"),
            )
        }
        Ok(_) => smoke_host_failed(
            "JSBridge trace",
            format!("{path}: no accepted bridge roundtrip or param gesture markers"),
            "capture bridge.hello/readyAck/reply or param begin/perform/end trace",
        ),
        Err(error) => smoke_host_failed(
            "JSBridge trace",
            format!("failed to read {path}: {error}"),
            "pass a readable bridge trace file",
        ),
    }
}

fn smoke_host_meter_log_check(path: Option<&Utf8Path>) -> SmokeHostCheck {
    let Some(path) = path else {
        return smoke_host_skipped(
            "meter stream",
            "no --meter-log evidence supplied",
            "pass a meter log or bridge trace containing a nonzero meter.main frame",
        );
    };
    match read_text_file_no_symlink("meter log", path) {
        Ok(text) if daw_marker_matches(&text, meter_stream_delivered) => smoke_host_ok(
            "meter stream",
            format!("accepted meter evidence from {path}"),
        ),
        Ok(_) => smoke_host_failed(
            "meter stream",
            format!("{path}: no accepted nonzero meter stream markers"),
            "capture meter_flush sent>0, meter lane packets, or meter.main peaks/rms",
        ),
        Err(error) => smoke_host_failed(
            "meter stream",
            format!("failed to read {path}: {error}"),
            "pass a readable meter log file",
        ),
    }
}

fn smoke_host_ok(name: impl Into<String>, value: impl Into<String>) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "ok".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: None,
    }
}

fn smoke_host_skipped(
    name: impl Into<String>,
    value: impl Into<String>,
    hint: impl Into<String>,
) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "skipped".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: Some(smoke_host_sanitize_report_text(hint)),
    }
}

fn smoke_host_failed(
    name: impl Into<String>,
    value: impl Into<String>,
    hint: impl Into<String>,
) -> SmokeHostCheck {
    SmokeHostCheck {
        name: smoke_host_sanitize_report_text(name),
        status: "failed".to_string(),
        value: smoke_host_sanitize_report_text(value),
        hint: Some(smoke_host_sanitize_report_text(hint)),
    }
}

fn smoke_host_sanitize_report_text(value: impl Into<String>) -> String {
    sanitize_release_report_text(value)
}

fn validate_smoke_host_report(report: &SmokeHostReport) -> Result<(), Box<dyn std::error::Error>> {
    validate_smoke_host_report_shape(report)?;

    let mut errors = Vec::new();
    if report.version != 1 {
        errors.push(format!(
            "unsupported smoke-host report version {}",
            report.version
        ));
    }
    if report.generator != SMOKE_HOST_GENERATOR {
        errors.push(format!(
            "unexpected smoke-host generator `{}`",
            report.generator
        ));
    }
    if !matches!(report.status.as_str(), "ok" | "partial" | "failed") {
        errors.push(format!(
            "invalid smoke-host report status `{}`",
            report.status
        ));
    }
    for check in &report.checks {
        if !matches!(check.status.as_str(), "ok" | "skipped" | "failed") {
            errors.push(format!(
                "smoke-host check `{}` has invalid status `{}`",
                check.name, check.status
            ));
        }
    }
    let expected_status = if report.checks.iter().any(|check| check.status == "failed") {
        "failed"
    } else if report.checks.iter().any(|check| check.status == "skipped") {
        "partial"
    } else {
        "ok"
    };
    if report.status != expected_status {
        errors.push(format!(
            "smoke-host report status must be `{expected_status}` for its checks"
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("invalid smoke-host report: {}", errors.join("; ")).into())
    }
}

fn validate_smoke_host_report_shape(
    report: &SmokeHostReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_action_text("smoke-host generator", &report.generator)?;
    validate_release_action_text("smoke-host workspace", &report.workspace)?;
    validate_release_action_text("smoke-host status", &report.status)?;
    validate_release_action_text(
        "smoke-host external evidence note",
        &report.external_evidence_note,
    )?;
    if report.checks.is_empty() {
        return Err("smoke-host report has no checks".into());
    }
    if report.checks.len() > SMOKE_HOST_MAX_CHECKS {
        return Err(format!(
            "smoke-host report has too many checks: {} exceeds maximum {SMOKE_HOST_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut seen_checks = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("smoke-host check name", &check.name)?;
        validate_release_action_text(
            &format!("smoke-host `{}` status", check.name),
            &check.status,
        )?;
        validate_release_action_text(&format!("smoke-host `{}` value", check.name), &check.value)?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("smoke-host `{}` hint", check.name), hint)?;
        }
        let normalized = normalize_smoke_host_check_name(&check.name);
        if normalized.is_empty() {
            return Err(format!(
                "smoke-host check `{}` does not normalize to a stable check name",
                check.name
            )
            .into());
        }
        if !seen_checks.insert(normalized.clone()) {
            return Err(format!("duplicate smoke-host check `{normalized}`").into());
        }
    }
    let expected_checks = expected_smoke_host_check_names();
    let unknown_checks = seen_checks
        .difference(&expected_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_checks.is_empty() {
        return Err(format!("unknown smoke-host check(s): {}", unknown_checks.join(", ")).into());
    }
    let missing_checks = expected_checks
        .difference(&seen_checks)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_checks.is_empty() {
        return Err(format!(
            "smoke-host report missing required check(s): {}",
            missing_checks.join(", ")
        )
        .into());
    }
    Ok(())
}

fn normalize_smoke_host_check_name(value: &str) -> String {
    normalize_platform_smoke_check_name(value)
}

fn smoke_host_report_all_ok(report: &SmokeHostReport) -> bool {
    report.status == "ok" && report.checks.iter().all(|check| check.status == "ok")
}

fn print_smoke_host_report(
    report: &SmokeHostReport,
    format: OutputFormat,
    out: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_smoke_host_report(report)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            println!("smoke-host: {}", report.status);
            println!("workspace: {}", report.workspace);
            for check in &report.checks {
                println!("- {}: {} - {}", check.name, check.status, check.value);
                if let Some(hint) = &check.hint {
                    println!("  hint: {hint}");
                }
            }
            if let Some(out) = out {
                println!("report: {out}");
            }
            println!("{}", report.external_evidence_note);
        }
    }
    Ok(())
}

fn validate_reports_release_check(paths: &[Utf8PathBuf], required: bool) -> ReleaseCheckItem {
    if paths.is_empty() {
        return optional_release_check_missing(
            "vst3 validate reports",
            required,
            "pass one or more `--validate-report <path>` files generated by `vesty validate --strict --report <path>`",
        );
    }

    let mut failures = Vec::new();
    let mut bundles = Vec::new();
    let mut platforms = Vec::new();
    for path in paths {
        match read_validate_report(path) {
            Ok(report) => {
                if let Err(error) = validate_release_validate_report(&report) {
                    failures.push(format!("{path}: {error}"));
                } else {
                    collect_validate_report_platforms(&report, &mut platforms);
                    bundles.push(report.bundle);
                }
            }
            Err(error) => failures.push(format!("{path}: {error}")),
        }
    }

    if failures.is_empty() {
        ReleaseCheckItem {
            name: "vst3 validate reports".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} validate report(s): {}; platforms: {}",
                paths.len(),
                bundles.join(", "),
                format_platform_coverage(&mut platforms)
            ),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "vst3 validate reports".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "run `vesty validate <bundle.vst3> --strict --report <path>` with Steinberg validator configured"
                    .to_string(),
            ),
        }
    }
}

fn static_validate_reports_release_check(
    paths: &[Utf8PathBuf],
    required: bool,
) -> ReleaseCheckItem {
    if paths.is_empty() {
        return optional_release_check_missing(
            "vst3 static validate reports",
            required,
            "pass one or more `--static-validate-report <path>` files generated by `vesty validate --static-only --strict --report <path>`",
        );
    }

    let mut failures = Vec::new();
    let mut bundles = Vec::new();
    let mut platforms = Vec::new();
    for path in paths {
        match read_validate_report(path) {
            Ok(report) => {
                if let Err(error) = validate_static_validate_report(&report) {
                    failures.push(format!("{path}: {error}"));
                } else {
                    collect_validate_report_platforms(&report, &mut platforms);
                    bundles.push(report.bundle);
                }
            }
            Err(error) => failures.push(format!("{path}: {error}")),
        }
    }

    if failures.is_empty() {
        ReleaseCheckItem {
            name: "vst3 static validate reports".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} static validate report(s): {}; platforms: {}",
                paths.len(),
                bundles.join(", "),
                format_platform_coverage(&mut platforms)
            ),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "vst3 static validate reports".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "run `vesty validate <bundle.vst3> --static-only --strict --report <path>` after packaging in CI"
                    .to_string(),
            ),
        }
    }
}

const REQUIRED_EXAMPLE_BUNDLES: [&str; 3] = [
    "VestyGain.vst3",
    "VestyWebUIDemo.vst3",
    "VestyMIDISynth.vst3",
];
const REQUIRED_EXAMPLE_VALIDATE_PLATFORMS: [&str; 3] = ["linux-x64", "macos", "windows-x64"];
const ASSET_MANIFEST_FILE: &str = "assets.manifest.json";
const REQUIRED_WEB_UI_EXAMPLE_BUNDLE: &str = "VestyWebUIDemo.vst3";

fn example_validate_coverage_release_check(
    paths: &[Utf8PathBuf],
    require_all_platforms: bool,
) -> ReleaseCheckItem {
    if paths.is_empty() {
        return optional_release_check_missing(
            "vst3 example validator coverage",
            require_all_platforms,
            "release evidence should include Steinberg validator-passed reports for VestyGain, VestyWebUIDemo and VestyMIDISynth on macOS, Windows x64 and Linux x64; generate them with `vesty validate --strict --report <path>`",
        );
    }

    let mut failures = Vec::new();
    let mut bundle_coverage = BTreeSet::new();
    let mut platform_coverage = BTreeSet::new();
    let mut coverage = BTreeSet::<String>::new();
    for path in paths {
        match read_validate_report(path) {
            Ok(report) => {
                if let Err(error) = validate_release_validate_report(&report) {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                let bundle = validate_report_bundle_name(&report);
                if REQUIRED_EXAMPLE_BUNDLES.contains(&bundle.as_str()) {
                    if let Err(error) = validate_required_example_report_bundle(
                        path,
                        &bundle,
                        &REQUIRED_EXAMPLE_BUNDLES,
                    ) {
                        failures.push(format!("{path}: {error}"));
                        continue;
                    }
                    if let Err(error) =
                        validate_required_example_parameter_manifest_evidence(&report, &bundle)
                    {
                        failures.push(format!("{path}: {error}"));
                        continue;
                    }
                    if let Err(error) = validate_required_example_asset_evidence(&report, &bundle) {
                        failures.push(format!("{path}: {error}"));
                        continue;
                    }
                    let platform = match validate_required_example_report_platform(
                        path,
                        &report,
                        &REQUIRED_EXAMPLE_VALIDATE_PLATFORMS,
                        "validator-passed example report",
                    ) {
                        Ok(platform) => platform,
                        Err(error) => {
                            failures.push(format!("{path}: {error}"));
                            continue;
                        }
                    };
                    if require_all_platforms
                        && let Err(error) =
                            validate_required_binary_export_evidence(&report, &bundle, platform)
                    {
                        failures.push(format!("{path}: {error}"));
                        continue;
                    }
                    let key = format!("{bundle}@{platform}");
                    if coverage.contains(&key) {
                        failures.push(format!(
                            "{path}: duplicate validator-passed example report for {key}"
                        ));
                        continue;
                    }
                    bundle_coverage.insert(bundle.clone());
                    platform_coverage.insert(platform);
                    coverage.insert(key);
                }
            }
            Err(error) => failures.push(format!("{path}: {error}")),
        }
    }

    if !failures.is_empty() {
        return ReleaseCheckItem {
            name: "vst3 example validator coverage".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "expected validator-passed `vesty validate --strict --report` JSON".to_string(),
            ),
        };
    }

    if coverage.is_empty() {
        return optional_release_check_missing(
            "vst3 example validator coverage",
            require_all_platforms,
            "no Vesty example validator-passed reports found; expected VestyGain, VestyWebUIDemo and VestyMIDISynth on macOS, Windows x64 and Linux x64; generate them with `vesty validate --strict --report <path>`",
        );
    }

    if require_all_platforms {
        let mut missing = Vec::new();
        for platform in REQUIRED_EXAMPLE_VALIDATE_PLATFORMS {
            for bundle in REQUIRED_EXAMPLE_BUNDLES {
                let key = format!("{bundle}@{platform}");
                if !coverage.contains(&key) {
                    missing.push(key);
                }
            }
        }
        if missing.is_empty() {
            let platforms = platform_coverage
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            ReleaseCheckItem {
                name: "vst3 example validator coverage".to_string(),
                status: "ok".to_string(),
                value: format!(
                    "{} example/platform validator report(s) (full release coverage; platforms: {platforms})",
                    coverage.len()
                ),
                hint: None,
            }
        } else {
            ReleaseCheckItem {
                name: "vst3 example validator coverage".to_string(),
                status: "failed".to_string(),
                value: format!("missing validator-passed reports: {}", missing.join(", ")),
                hint: Some(
                    "run `vesty validate --strict --report <path>` for all three shipped Vesty examples on macOS, Windows x64 and Linux x64 before release"
                        .to_string(),
                ),
            }
        }
    } else {
        let missing = REQUIRED_EXAMPLE_BUNDLES
            .iter()
            .filter(|bundle| !bundle_coverage.contains(**bundle))
            .copied()
            .collect::<Vec<_>>();
        let mut present = bundle_coverage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        present.sort_unstable();
        let platforms = platform_coverage
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        ReleaseCheckItem {
            name: "vst3 example validator coverage".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} of 3 example validator report(s): {}; platforms: {}",
                bundle_coverage.len(),
                present.join(", "),
                if platforms.is_empty() {
                    "unknown"
                } else {
                    platforms.as_str()
                }
            ),
            hint: (!missing.is_empty()).then(|| {
                format!(
                    "full release coverage still needs validator reports for {}",
                    missing.join(", ")
                )
            }),
        }
    }
}

const REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS: [&str; 3] = REQUIRED_EXAMPLE_VALIDATE_PLATFORMS;

fn example_static_validate_coverage_release_check(
    paths: &[Utf8PathBuf],
    require_all_platforms: bool,
) -> ReleaseCheckItem {
    if paths.is_empty() {
        return optional_release_check_missing(
            "ci example static validate coverage",
            require_all_platforms,
            "CI package smoke should upload `vesty validate --static-only --strict --report <path>` reports for gain, web-ui-param-demo and midi-synth",
        );
    }

    let mut failures = Vec::new();
    let mut coverage = BTreeSet::new();
    let mut present_platforms = BTreeSet::new();
    for path in paths {
        match read_validate_report(path) {
            Ok(report) => {
                if let Err(error) = validate_static_validate_report(&report) {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                let bundle = validate_report_bundle_name(&report);
                if !REQUIRED_EXAMPLE_BUNDLES.contains(&bundle.as_str()) {
                    continue;
                }
                if let Err(error) = validate_required_example_report_bundle(
                    path,
                    &bundle,
                    &REQUIRED_EXAMPLE_BUNDLES,
                ) {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                if let Err(error) =
                    validate_required_example_parameter_manifest_evidence(&report, &bundle)
                {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                if let Err(error) = validate_required_example_asset_evidence(&report, &bundle) {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                let platform = match validate_required_example_report_platform(
                    path,
                    &report,
                    &REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS,
                    "static example report",
                ) {
                    Ok(platform) => platform,
                    Err(error) => {
                        failures.push(format!("{path}: {error}"));
                        continue;
                    }
                };
                if require_all_platforms
                    && let Err(error) =
                        validate_required_binary_export_evidence(&report, &bundle, platform)
                {
                    failures.push(format!("{path}: {error}"));
                    continue;
                }
                let key = format!("{bundle}@{platform}");
                if coverage.contains(&key) {
                    failures.push(format!("{path}: duplicate static example report for {key}"));
                    continue;
                }
                present_platforms.insert(platform);
                coverage.insert(key);
            }
            Err(error) => failures.push(format!("{path}: {error}")),
        }
    }

    if !failures.is_empty() {
        return ReleaseCheckItem {
            name: "ci example static validate coverage".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "expected parseable `vesty validate --static-only --strict --report` JSON"
                    .to_string(),
            ),
        };
    }

    if coverage.is_empty() {
        return optional_release_check_missing(
            "ci example static validate coverage",
            require_all_platforms,
            "no Vesty example static validate reports found; expected VestyGain, VestyWebUIDemo and VestyMIDISynth CI artifacts",
        );
    }

    let platforms_to_check = if require_all_platforms {
        REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS.to_vec()
    } else {
        present_platforms.iter().copied().collect::<Vec<_>>()
    };
    let mut missing = Vec::new();
    for platform in &platforms_to_check {
        for bundle in REQUIRED_EXAMPLE_BUNDLES {
            let key = format!("{bundle}@{platform}");
            if !coverage.contains(&key) {
                missing.push(key);
            }
        }
    }

    if missing.is_empty() {
        let platforms = present_platforms
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let full = if require_all_platforms {
            "full release coverage"
        } else {
            "per-platform CI smoke coverage"
        };
        ReleaseCheckItem {
            name: "ci example static validate coverage".to_string(),
            status: "ok".to_string(),
            value: format!(
                "{} example/platform entries ({full}; platforms: {platforms})",
                coverage.len()
            ),
            hint: None,
        }
    } else {
        ReleaseCheckItem {
            name: "ci example static validate coverage".to_string(),
            status: "failed".to_string(),
            value: format!("missing: {}", missing.join(", ")),
            hint: Some(
                "package and run `vesty validate --static-only --strict --report <path>` for all three examples on linux-x64, macos and windows-x64"
                    .to_string(),
            ),
        }
    }
}

fn validate_required_example_report_bundle(
    path: &Utf8Path,
    bundle: &str,
    required_bundles: &[&'static str],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path_bundle) = validate_report_file_name_bundle(path, required_bundles)?
        && path_bundle != bundle
    {
        return Err(format!(
            "example report file name indicates {path_bundle}, but report bundle is {bundle}"
        )
        .into());
    }
    Ok(())
}

fn validate_report_file_name_bundle(
    path: &Utf8Path,
    required_bundles: &[&'static str],
) -> Result<Option<&'static str>, Box<dyn std::error::Error>> {
    let tokens = file_name_tokens(path);
    let mut found = required_bundles
        .iter()
        .copied()
        .filter(|bundle| file_name_tokens_contain_bundle(&tokens, bundle))
        .collect::<Vec<_>>();
    found.sort_unstable();
    found.dedup();
    match found.as_slice() {
        [] => Ok(None),
        [bundle] => Ok(Some(*bundle)),
        _ => Err(format!(
            "validate report file name contains multiple example bundle labels: {}",
            found.join(", ")
        )
        .into()),
    }
}

fn validate_required_example_report_platform(
    path: &Utf8Path,
    report: &ValidateReport,
    required_platforms: &[&'static str],
    report_label: &str,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    let platforms = validate_report_platforms(report);
    if platforms.is_empty() {
        return Err(format!(
            "{report_label} must contain exactly one release platform; could not infer platform from static binaries"
        )
        .into());
    }
    if platforms.len() != 1 {
        return Err(format!(
            "{report_label} must contain exactly one release platform; found {}",
            format_platform_set(&platforms)
        )
        .into());
    }

    let Some(platform) = platforms.iter().next().copied() else {
        return Err(format!("{report_label} platform set unexpectedly became empty").into());
    };
    if !required_platforms.contains(&platform) {
        return Err(format!("{report_label} uses unsupported platform `{platform}`").into());
    }

    if let Some(path_platform) = validate_report_file_name_platform(path, required_platforms)?
        && path_platform != platform
    {
        return Err(format!(
            "{report_label} file name indicates {path_platform}, but static binaries indicate {platform}"
        )
        .into());
    }

    Ok(platform)
}

fn validate_report_file_name_platform(
    path: &Utf8Path,
    required_platforms: &[&'static str],
) -> Result<Option<&'static str>, Box<dyn std::error::Error>> {
    let tokens = file_name_tokens(path);
    let mut found = required_platforms
        .iter()
        .copied()
        .filter(|platform| file_name_tokens_contain_platform(&tokens, platform))
        .collect::<Vec<_>>();
    found.sort_unstable();
    found.dedup();
    match found.as_slice() {
        [] => Ok(None),
        [platform] => Ok(Some(*platform)),
        _ => Err(format!(
            "validate report file name contains multiple platform labels: {}",
            found.join(", ")
        )
        .into()),
    }
}

fn file_name_tokens(path: &Utf8Path) -> Vec<String> {
    let Some(file_name) = path.file_name() else {
        return Vec::new();
    };
    file_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn file_name_tokens_contain_bundle(tokens: &[String], bundle: &str) -> bool {
    let bundle_name = bundle.strip_suffix(".vst3").unwrap_or(bundle);
    let compact_bundle_token = file_name_label_compact_token(bundle_name);
    if !compact_bundle_token.is_empty()
        && tokens
            .iter()
            .any(|token| token.as_str() == compact_bundle_token)
    {
        return true;
    }
    let bundle_tokens = file_name_label_tokens(bundle_name);
    file_name_tokens_contain_sequence(tokens, &bundle_tokens)
}

fn file_name_tokens_contain_platform(tokens: &[String], platform: &str) -> bool {
    let platform_tokens = file_name_label_tokens(platform);
    file_name_tokens_contain_sequence(tokens, &platform_tokens)
}

fn file_name_label_tokens(label: &str) -> Vec<String> {
    let chars = label.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut current = String::new();
    for (index, ch) in chars.iter().copied().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        let previous = index
            .checked_sub(1)
            .and_then(|previous| chars.get(previous));
        let next = chars.get(index + 1).copied();
        let camel_boundary = ch.is_ascii_uppercase()
            && !current.is_empty()
            && (previous.is_some_and(|previous| {
                previous.is_ascii_lowercase() || previous.is_ascii_digit()
            }) || next.is_some_and(|next| next.is_ascii_lowercase()));
        if camel_boundary {
            tokens.push(std::mem::take(&mut current));
        }
        current.push(ch.to_ascii_lowercase());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn file_name_label_compact_token(label: &str) -> String {
    label
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn file_name_tokens_contain_sequence(tokens: &[String], sequence: &[String]) -> bool {
    if sequence.is_empty() || tokens.len() < sequence.len() {
        return false;
    }
    tokens.windows(sequence.len()).any(|window| {
        window
            .iter()
            .map(String::as_str)
            .eq(sequence.iter().map(String::as_str))
    })
}

fn format_platform_set(platforms: &BTreeSet<&'static str>) -> String {
    platforms.iter().copied().collect::<Vec<_>>().join(", ")
}

fn validate_required_example_parameter_manifest_evidence(
    report: &ValidateReport,
    bundle: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !REQUIRED_EXAMPLE_BUNDLES.contains(&bundle) {
        return Ok(());
    }
    let has_manifest = report
        .static_check
        .parameter_manifest
        .as_deref()
        .is_some_and(|path| manifest_evidence_path_matches(path, bundle, PARAMETER_MANIFEST_FILE));
    if has_manifest {
        Ok(())
    } else {
        let actual = report
            .static_check
            .parameter_manifest
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .unwrap_or("<missing>");
        Err(format!(
            "{bundle} report is missing parameter manifest evidence at {bundle}/Contents/Resources/{PARAMETER_MANIFEST_FILE}; actual static_check.parameter_manifest: {actual}"
        )
        .into())
    }
}

fn validate_required_binary_export_evidence(
    report: &ValidateReport,
    bundle: &str,
    platform: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let matching = report
        .static_check
        .binary_exports
        .iter()
        .filter(|check| check.platform == platform)
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Err(format!(
            "{bundle}@{platform} report is missing binary export evidence in static_check.binary_exports"
        )
        .into());
    }
    if matching
        .iter()
        .any(|check| binary_export_check_proves_platform(check, platform))
    {
        return Ok(());
    }

    let details = matching
        .iter()
        .map(|check| {
            let missing = if check.missing_symbols.is_empty() {
                "<none>".to_string()
            } else {
                check.missing_symbols.join(", ")
            };
            format!("{} status={} missing={missing}", check.binary, check.status)
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "{bundle}@{platform} report must include ok binary export evidence; actual: {details}"
    )
    .into())
}

fn manifest_evidence_path_matches(path: &str, bundle: &str, file_name: &str) -> bool {
    report_path_has_bundle_tail(path, bundle, &["Contents", "Resources", file_name])
}

fn validate_required_example_asset_evidence(
    report: &ValidateReport,
    bundle: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if bundle != REQUIRED_WEB_UI_EXAMPLE_BUNDLE {
        return Ok(());
    }
    let has_manifest = report
        .static_check
        .asset_manifest
        .as_deref()
        .is_some_and(|path| manifest_evidence_path_matches(path, bundle, ASSET_MANIFEST_FILE));
    if has_manifest && report.static_check.asset_count > 0 {
        Ok(())
    } else {
        let actual = report
            .static_check
            .asset_manifest
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .unwrap_or("<missing>");
        Err(format!(
            "{REQUIRED_WEB_UI_EXAMPLE_BUNDLE} report is missing UI asset manifest evidence at {REQUIRED_WEB_UI_EXAMPLE_BUNDLE}/Contents/Resources/{ASSET_MANIFEST_FILE}; actual static_check.asset_manifest: {actual}; asset_count: {}",
            report.static_check.asset_count
        )
        .into())
    }
}

fn read_validate_report(path: &Utf8Path) -> Result<ValidateReport, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("validate report", path)?;
    serde_json::from_str(&text)
        .map_err(|error| format!("invalid validate report JSON: {error}").into())
}

const VALIDATE_REPORT_TEXT_MAX_BYTES: usize = 8 * 1024;
const VALIDATE_REPORT_LOG_MAX_BYTES: usize = 256 * 1024;
const VALIDATE_REPORT_MAX_BINARIES: usize = 64;
const VALIDATE_REPORT_MAX_BINARY_EXPORTS: usize = 64;
const VALIDATE_REPORT_MAX_SYMBOLS: usize = 64;

fn validate_validate_report_shape(
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_text("validate report bundle", &report.bundle)?;
    validate_static_bundle_check_shape(&report.static_check)?;
    validate_validator_check_shape(&report.validator)?;
    Ok(())
}

fn validate_static_bundle_check_shape(
    check: &StaticBundleCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_text("static bundle check status", &check.status)?;
    validate_optional_validate_report_text("static bundle check moduleinfo", &check.moduleinfo)?;
    validate_optional_validate_report_text(
        "static bundle check parameter manifest",
        &check.parameter_manifest,
    )?;
    validate_optional_validate_report_text(
        "static bundle check asset manifest",
        &check.asset_manifest,
    )?;
    validate_optional_validate_report_text("static bundle check error", &check.error)?;
    if check.binaries.len() > VALIDATE_REPORT_MAX_BINARIES {
        return Err(format!(
            "static bundle check has too many binaries: {} exceeds maximum {VALIDATE_REPORT_MAX_BINARIES}",
            check.binaries.len()
        )
        .into());
    }
    validate_validate_report_text_list(
        "static bundle check binary",
        &check.binaries,
        VALIDATE_REPORT_MAX_BINARIES,
    )?;
    let mut seen_binary_paths = BTreeSet::new();
    for binary in &check.binaries {
        let Some(key) = report_path_key(binary) else {
            continue;
        };
        if !seen_binary_paths.insert(key.clone()) {
            return Err(format!("duplicate static bundle check binary path `{key}`").into());
        }
    }
    if check.binary_exports.len() > VALIDATE_REPORT_MAX_BINARY_EXPORTS {
        return Err(format!(
            "static bundle check has too many binary export checks: {} exceeds maximum {VALIDATE_REPORT_MAX_BINARY_EXPORTS}",
            check.binary_exports.len()
        )
        .into());
    }
    let mut seen_export_checks = BTreeSet::new();
    for export_check in &check.binary_exports {
        validate_binary_export_check_shape(export_check)?;
        let binary = report_path_key(&export_check.binary)
            .unwrap_or_else(|| export_check.binary.trim().to_string());
        let key = format!("{}@{}", binary, export_check.platform.trim());
        if !seen_export_checks.insert(key.clone()) {
            return Err(format!("duplicate binary export check `{key}`").into());
        }
    }
    Ok(())
}

fn validate_binary_export_check_shape(
    check: &BinaryExportCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_text("binary export check binary", &check.binary)?;
    validate_validate_report_text("binary export check platform", &check.platform)?;
    validate_validate_report_text("binary export check status", &check.status)?;
    validate_optional_validate_report_text("binary export check tool", &check.tool)?;
    validate_optional_validate_report_text("binary export check error", &check.error)?;
    validate_validate_report_text_list(
        "binary export required symbol",
        &check.required_symbols,
        VALIDATE_REPORT_MAX_SYMBOLS,
    )?;
    validate_validate_report_text_list(
        "binary export found symbol",
        &check.found_symbols,
        VALIDATE_REPORT_MAX_SYMBOLS,
    )?;
    validate_validate_report_text_list(
        "binary export missing symbol",
        &check.missing_symbols,
        VALIDATE_REPORT_MAX_SYMBOLS,
    )?;
    Ok(())
}

fn validate_validator_check_shape(
    check: &ValidatorCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_text("validator status", &check.status)?;
    validate_optional_validate_report_text("validator path", &check.path)?;
    validate_optional_validate_report_log_text("validator stdout", &check.stdout)?;
    validate_optional_validate_report_log_text("validator stderr", &check.stderr)?;
    validate_optional_validate_report_text("validator reason", &check.reason)?;
    validate_optional_validate_report_text("validator error", &check.error)?;
    Ok(())
}

fn validate_validate_report_text(label: &str, value: &str) -> Result<(), String> {
    validate_release_action_text(label, value)?;
    if value.len() > VALIDATE_REPORT_TEXT_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {VALIDATE_REPORT_TEXT_MAX_BYTES} bytes"
        ));
    }
    Ok(())
}

fn validate_optional_validate_report_text(
    label: &str,
    value: &Option<String>,
) -> Result<(), String> {
    if let Some(value) = value {
        validate_validate_report_text(label, value)?;
    }
    Ok(())
}

fn validate_optional_validate_report_log_text(
    label: &str,
    value: &Option<String>,
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > VALIDATE_REPORT_LOG_MAX_BYTES {
        return Err(format!(
            "{label} must be at most {VALIDATE_REPORT_LOG_MAX_BYTES} bytes"
        ));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(format!(
            "{label} must not contain control characters other than tab/newline"
        ));
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!(
            "{label} must not contain unsafe Unicode format characters"
        ));
    }
    Ok(())
}

fn validate_validate_report_text_list(
    label: &str,
    values: &[String],
    max_items: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if values.len() > max_items {
        return Err(format!(
            "{label} list has too many items: {} exceeds maximum {max_items}",
            values.len()
        )
        .into());
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_validate_report_text(label, value)?;
        let normalized = value.trim().to_string();
        if !seen.insert(normalized.clone()) {
            return Err(format!("duplicate {label} `{normalized}`").into());
        }
    }
    Ok(())
}

fn non_empty_field<'a>(value: &'a Option<String>, field: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} is missing"))
}

fn empty_or_missing(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").is_empty()
}

fn normalize_report_path(path: &str) -> Option<String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn portable_report_path(path: &Utf8Path) -> String {
    path.as_str().replace('\\', "/")
}

fn release_report_paths_equal(left: &str, right: &str) -> bool {
    match (
        lexical_release_report_path(left),
        lexical_release_report_path(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
fn release_report_path_ends_with(path: &str, suffix: &str) -> bool {
    let (Ok(path), Ok(suffix)) = (
        lexical_release_report_path(path),
        lexical_release_report_path(suffix),
    ) else {
        return false;
    };
    path.components.ends_with(&suffix.components)
}

fn report_path_components(path: &str) -> Option<Vec<String>> {
    let normalized = normalize_report_path(path)?;
    let components = normalized
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if components.is_empty() || components.iter().any(|component| component == "..") {
        None
    } else {
        Some(components)
    }
}

fn report_path_key(path: &str) -> Option<String> {
    report_path_components(path).map(|components| components.join("/"))
}

fn report_bundle_component_index(components: &[String]) -> Option<usize> {
    components
        .iter()
        .position(|component| component.to_ascii_lowercase().ends_with(".vst3"))
}

fn report_path_has_bundle_tail(path: &str, bundle: &str, tail: &[&str]) -> bool {
    let Some(components) = report_path_components(path) else {
        return false;
    };
    let Some(index) = report_bundle_component_index(&components) else {
        return false;
    };
    components[index] == bundle
        && components[index + 1..]
            .iter()
            .map(String::as_str)
            .eq(tail.iter().copied())
}

fn report_binary_path_matches_bundle(path: &str, bundle: &str) -> bool {
    let Some(components) = report_path_components(path) else {
        return false;
    };
    let Some(index) = report_bundle_component_index(&components) else {
        return false;
    };
    components[index] == bundle
        && components
            .get(index + 1)
            .is_some_and(|component| component == "Contents")
        && components.get(index + 2).is_some_and(|component| {
            matches!(component.as_str(), "MacOS" | "x86_64-win" | "x86_64-linux")
        })
        && components.len() > index + 3
}

fn validate_report_paths_self_consistent(
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundle = validate_report_bundle_name(report);
    if bundle.trim().is_empty() {
        return Err("validate report bundle is missing".into());
    }
    if bundle.chars().any(char::is_control) {
        return Err("validate report bundle contains control characters".into());
    }
    if !bundle.to_ascii_lowercase().ends_with(".vst3") {
        return Err(format!("validate report bundle must end with .vst3: {bundle}").into());
    }

    if report.static_check.status != "ok" {
        return Ok(());
    }

    let moduleinfo = non_empty_field(
        &report.static_check.moduleinfo,
        "static bundle check moduleinfo",
    )
    .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
    if !report_path_has_bundle_tail(
        moduleinfo,
        &bundle,
        &["Contents", "Resources", "moduleinfo.json"],
    ) {
        return Err(format!(
            "static bundle check moduleinfo path does not belong to {bundle}: {moduleinfo}"
        )
        .into());
    }

    let mut binary_paths = BTreeSet::new();
    for binary in &report.static_check.binaries {
        let Some(normalized) = report_path_key(binary) else {
            return Err("static bundle check contains an empty binary path".into());
        };
        if !report_binary_path_matches_bundle(&normalized, &bundle) {
            return Err(format!(
                "static bundle check binary path does not belong to {bundle}: {binary}"
            )
            .into());
        }
        binary_paths.insert(normalized);
    }

    for export_check in &report.static_check.binary_exports {
        let Some(normalized) = report_path_key(&export_check.binary) else {
            return Err("binary export check contains an empty binary path".into());
        };
        if !report_binary_path_matches_bundle(&normalized, &bundle) {
            return Err(format!(
                "binary export check path does not belong to {bundle}: {}",
                export_check.binary
            )
            .into());
        }
        if !binary_paths.contains(&normalized) {
            return Err(format!(
                "binary export check references a binary not listed in static_check.binaries: {}",
                export_check.binary
            )
            .into());
        }
    }

    if let Some(path) = report
        .static_check
        .parameter_manifest
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        && !manifest_evidence_path_matches(path, &bundle, PARAMETER_MANIFEST_FILE)
    {
        return Err(
            format!("static_check.parameter_manifest does not belong to {bundle}: {path}").into(),
        );
    }

    if let Some(path) = report
        .static_check
        .asset_manifest
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        if !manifest_evidence_path_matches(path, &bundle, ASSET_MANIFEST_FILE) {
            return Err(
                format!("static_check.asset_manifest does not belong to {bundle}: {path}").into(),
            );
        }
        if report.static_check.asset_count == 0 {
            return Err("static_check.asset_manifest is present but asset_count is 0".into());
        }
    } else if report.static_check.asset_count != 0 {
        return Err(format!(
            "static_check.asset_count is {} but asset_manifest is missing",
            report.static_check.asset_count
        )
        .into());
    }

    Ok(())
}

fn validate_static_bundle_check_self_consistent(
    check: &StaticBundleCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    match check.status.as_str() {
        "ok" => {
            non_empty_field(&check.moduleinfo, "static bundle check moduleinfo")
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            if check.binaries.is_empty() {
                return Err("static bundle check has no platform binaries".into());
            }
            if let Some(error) = check.error.as_deref().map(str::trim)
                && !error.is_empty()
            {
                return Err(
                    format!("static bundle check is ok but still lists error: {error}").into(),
                );
            }
            validate_static_bundle_binary_export_checks(check)?;
        }
        "failed" => {
            non_empty_field(&check.error, "static bundle check failure error")
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            if check
                .moduleinfo
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            {
                return Err("static bundle check failed but still lists moduleinfo".into());
            }
            if !check.binaries.is_empty() {
                return Err("static bundle check failed but still lists binaries".into());
            }
            if !check.binary_exports.is_empty() {
                return Err("static bundle check failed but still lists binary exports".into());
            }
            if check
                .parameter_manifest
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            {
                return Err("static bundle check failed but still lists parameter manifest".into());
            }
            if check
                .asset_manifest
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            {
                return Err("static bundle check failed but still lists asset manifest".into());
            }
            if check.asset_count != 0 {
                return Err(format!(
                    "static bundle check failed but still lists asset_count {}",
                    check.asset_count
                )
                .into());
            }
        }
        other => {
            return Err(format!("static bundle check has unknown status `{other}`").into());
        }
    }
    Ok(())
}

fn validate_validator_check_self_consistent(
    check: &ValidatorCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    match check.status.as_str() {
        "passed" => {
            non_empty_field(&check.path, "validator path")
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            if check.exit_code != Some(0) {
                return Err(format!(
                    "validator exit code is {}",
                    check
                        .exit_code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )
                .into());
            }
            match check.tests_passed {
                Some(passed) if passed > 0 => {}
                Some(_) => return Err("validator reported 0 passed tests".into()),
                None => return Err("validator passed test count is missing".into()),
            }
            match check.tests_failed {
                Some(0) => {}
                Some(failed) => {
                    return Err(format!("validator reported {failed} failed test(s)").into());
                }
                None => return Err("validator failed test count is missing".into()),
            }
            if let Some(error) = check.error.as_deref().map(str::trim)
                && !error.is_empty()
            {
                return Err(format!("validator passed report includes error: {error}").into());
            }
            if let Some(reason) = check.reason.as_deref().map(str::trim)
                && !reason.is_empty()
            {
                return Err(format!("validator passed report includes reason: {reason}").into());
            }
            validate_validator_passed_log_runtime_state(check)?;
            validate_validator_passed_log_summary(check)?;
        }
        "failed" => {
            non_empty_field(&check.path, "validator failed report path")
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            if let Some(reason) = check.reason.as_deref().map(str::trim)
                && !reason.is_empty()
            {
                return Err(format!("validator failed report includes reason: {reason}").into());
            }
            let has_error = check
                .error
                .as_deref()
                .map(str::trim)
                .is_some_and(|error| !error.is_empty());
            let has_nonzero_exit = check.exit_code.is_some_and(|code| code != 0);
            let has_failed_tests = check.tests_failed.is_some_and(|failed| failed > 0);
            if check.tests_passed.is_some() != check.tests_failed.is_some() {
                return Err("validator failed report has partial test counts".into());
            }
            let has_runtime_failure = validator_failed_report_has_runtime_failure(check);
            if check.exit_code == Some(0) && !has_failed_tests && !has_runtime_failure {
                return Err(
                    "validator failed report has exit code 0, no failed tests and no runtime failure evidence"
                        .into(),
                );
            }
            if check.tests_failed == Some(0) && !has_nonzero_exit && !has_runtime_failure {
                return Err(
                    "validator failed report lists zero failed tests without nonzero exit or runtime failure evidence"
                        .into(),
                );
            }
            if !has_error && !has_nonzero_exit && !has_failed_tests {
                return Err(
                    "validator failed report has no error, nonzero exit code or failed test count"
                        .into(),
                );
            }
        }
        "skipped" | "not_run" | "not_found" => {
            non_empty_field(&check.reason, "validator skip reason")
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            if !empty_or_missing(&check.path) {
                return Err(format!("validator {} report includes path", check.status).into());
            }
            if check.exit_code.is_some() {
                return Err(format!("validator {} report includes exit code", check.status).into());
            }
            if check.tests_passed.is_some() {
                return Err(format!(
                    "validator {} report includes passed test count",
                    check.status
                )
                .into());
            }
            if check.tests_failed.is_some() {
                return Err(format!(
                    "validator {} report includes failed test count",
                    check.status
                )
                .into());
            }
            if !empty_or_missing(&check.stdout) {
                return Err(format!("validator {} report includes stdout", check.status).into());
            }
            if !empty_or_missing(&check.stderr) {
                return Err(format!("validator {} report includes stderr", check.status).into());
            }
            if !empty_or_missing(&check.error) {
                return Err(format!("validator {} report includes error", check.status).into());
            }
        }
        other => return Err(format!("validator has unknown status `{other}`").into()),
    }
    Ok(())
}

fn validator_failed_report_has_runtime_failure(check: &ValidatorCheck) -> bool {
    [
        check.stdout.as_deref(),
        check.stderr.as_deref(),
        check.error.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|text| vst3_validator_has_runtime_failure(&text.to_ascii_lowercase()))
}

fn validate_validator_passed_log_runtime_state(
    check: &ValidatorCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, text) in [
        ("stdout", check.stdout.as_deref()),
        ("stderr", check.stderr.as_deref()),
    ] {
        let Some(text) = text else {
            continue;
        };
        if vst3_validator_has_runtime_failure(&text.to_ascii_lowercase()) {
            return Err(format!("validator {label} includes runtime failure evidence").into());
        }
    }
    Ok(())
}

fn validate_validator_passed_log_summary(
    check: &ValidatorCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = (
        check
            .tests_passed
            .ok_or("validator passed test count is missing")?,
        check
            .tests_failed
            .ok_or("validator failed test count is missing")?,
    );
    for (label, text) in [
        ("stdout", check.stdout.as_deref()),
        ("stderr", check.stderr.as_deref()),
    ] {
        let Some(text) = text else {
            continue;
        };
        let Some(summary) = validator_test_summary(text) else {
            continue;
        };
        if summary != expected {
            return Err(format!(
                "validator {label} summary contradicts report counts: log passed={}, failed={}; report passed={}, failed={}",
                summary.0, summary.1, expected.0, expected.1
            )
            .into());
        }
    }
    Ok(())
}

fn validate_release_validate_report(
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_shape(report)?;
    validate_static_bundle_check_self_consistent(&report.static_check)?;
    validate_report_paths_self_consistent(report)?;
    if report.static_check.status != "ok" {
        return Err(format!(
            "static bundle check status is {}",
            report.static_check.status
        )
        .into());
    }
    validate_validator_check_self_consistent(&report.validator)?;
    if report.validator.status != "passed" {
        return Err(format!(
            "validator status is {}{}{}",
            report.validator.status,
            report
                .validator
                .reason
                .as_deref()
                .map(|reason| format!(" ({reason})"))
                .unwrap_or_default(),
            report
                .validator
                .error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default(),
        )
        .into());
    }
    Ok(())
}

fn validate_static_validate_report(
    report: &ValidateReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_validate_report_shape(report)?;
    validate_static_bundle_check_self_consistent(&report.static_check)?;
    validate_report_paths_self_consistent(report)?;
    validate_validator_check_self_consistent(&report.validator)?;
    if report.static_check.status != "ok" {
        return Err(format!(
            "static bundle check status is {}",
            report.static_check.status
        )
        .into());
    }
    if !matches!(
        report.validator.status.as_str(),
        "skipped" | "not_run" | "not_found"
    ) {
        return Err(format!(
            "static validate report must be static-only; validator status is {}",
            report.validator.status
        )
        .into());
    }
    Ok(())
}

fn validate_static_bundle_binary_export_checks(
    check: &StaticBundleCheck,
) -> Result<(), Box<dyn std::error::Error>> {
    for export_check in &check.binary_exports {
        let expected = expected_binary_export_symbols(&export_check.platform).ok_or_else(|| {
            format!(
                "unknown binary export check platform `{}` for {}",
                export_check.platform, export_check.binary
            )
        })?;
        if let Some(inferred) = infer_validate_binary_platform(&export_check.binary)
            && inferred != export_check.platform
        {
            return Err(format!(
                "binary export check platform mismatch for {}: check says {}, binary path implies {inferred}",
                export_check.binary, export_check.platform
            )
            .into());
        }
        let missing_required = expected
            .iter()
            .copied()
            .filter(|symbol| {
                !export_check
                    .required_symbols
                    .iter()
                    .any(|item| item == symbol)
            })
            .collect::<Vec<_>>();
        if !missing_required.is_empty() {
            return Err(format!(
                "{} binary export check for {} has incomplete required symbol list: missing {}",
                export_check.platform,
                export_check.binary,
                missing_required.join(", ")
            )
            .into());
        }
        if export_check.status == "failed" {
            let missing = if export_check.missing_symbols.is_empty() {
                "<unknown>".to_string()
            } else {
                export_check.missing_symbols.join(", ")
            };
            return Err(format!(
                "{} binary export check failed for {}: missing {missing}",
                export_check.platform, export_check.binary
            )
            .into());
        }
        if export_check.status == "ok" {
            if export_check
                .tool
                .as_deref()
                .map(str::trim)
                .filter(|tool| !tool.is_empty())
                .is_none()
            {
                return Err(format!(
                    "{} binary export check for {} is ok but has no inspection tool",
                    export_check.platform, export_check.binary
                )
                .into());
            }
            let missing_found = expected
                .iter()
                .copied()
                .filter(|symbol| !export_check.found_symbols.iter().any(|item| item == symbol))
                .collect::<Vec<_>>();
            if !missing_found.is_empty() {
                return Err(format!(
                    "{} binary export check for {} is ok but did not record found symbols: {}",
                    export_check.platform,
                    export_check.binary,
                    missing_found.join(", ")
                )
                .into());
            }
            if !export_check.missing_symbols.is_empty() {
                return Err(format!(
                    "{} binary export check for {} is ok but still lists missing symbols: {}",
                    export_check.platform,
                    export_check.binary,
                    export_check.missing_symbols.join(", ")
                )
                .into());
            }
        } else if export_check.status == "skipped" {
            if export_check
                .error
                .as_deref()
                .map(str::trim)
                .filter(|error| !error.is_empty())
                .is_none()
            {
                return Err(format!(
                    "{} binary export check for {} was skipped without an explanation",
                    export_check.platform, export_check.binary
                )
                .into());
            }
        } else {
            return Err(format!(
                "{} binary export check for {} has unknown status `{}`",
                export_check.platform, export_check.binary, export_check.status
            )
            .into());
        }
    }
    Ok(())
}

fn binary_export_check_proves_platform(check: &BinaryExportCheck, platform: &str) -> bool {
    check.platform == platform
        && check.status == "ok"
        && check.missing_symbols.is_empty()
        && expected_binary_export_symbols(platform).is_some_and(|expected| {
            expected.iter().all(|symbol| {
                check.required_symbols.iter().any(|item| item == symbol)
                    && check.found_symbols.iter().any(|item| item == symbol)
            })
        })
}

fn strict_static_bundle_check_error(check: &StaticBundleCheck) -> Option<String> {
    if check.status != "ok" {
        return Some(format!(
            "strict validation requires static bundle check ok; actual status: {}",
            check.status
        ));
    }

    let platform_binaries = check
        .binaries
        .iter()
        .filter_map(|binary| {
            infer_validate_binary_platform(binary).and_then(|platform| {
                report_path_key(binary).map(|normalized| (binary, normalized, platform))
            })
        })
        .collect::<Vec<_>>();

    if platform_binaries.is_empty() {
        return Some(
            "strict validation requires at least one supported platform binary".to_string(),
        );
    }

    for (binary, normalized_binary, platform) in platform_binaries {
        let Some(export_check) = check.binary_exports.iter().find(|export_check| {
            export_check.platform == platform
                && report_path_key(&export_check.binary)
                    .is_some_and(|candidate| candidate == normalized_binary)
        }) else {
            return Some(format!(
                "strict validation requires binary export evidence for {binary} ({platform})"
            ));
        };

        if !binary_export_check_proves_platform(export_check, platform) {
            let detail = if export_check.status == "skipped" {
                export_check
                    .error
                    .as_deref()
                    .map(str::trim)
                    .filter(|error| !error.is_empty())
                    .unwrap_or("no reason recorded")
                    .to_string()
            } else if export_check.missing_symbols.is_empty() {
                format!("status {}", export_check.status)
            } else {
                format!(
                    "status {}; missing {}",
                    export_check.status,
                    export_check.missing_symbols.join(", ")
                )
            };
            return Some(format!(
                "strict validation requires ok binary export evidence for {binary} ({platform}); {detail}"
            ));
        }
    }

    None
}

fn expected_binary_export_symbols(platform: &str) -> Option<&'static [&'static str]> {
    vesty_vst3_sys::required_binary_export_tool_symbols(platform)
}

fn collect_validate_report_platforms(report: &ValidateReport, platforms: &mut Vec<&'static str>) {
    for binary in &report.static_check.binaries {
        if let Some(platform) = infer_validate_binary_platform(binary) {
            platforms.push(platform);
        }
    }
}

fn validate_report_platforms(report: &ValidateReport) -> BTreeSet<&'static str> {
    report
        .static_check
        .binaries
        .iter()
        .filter_map(|binary| infer_validate_binary_platform(binary))
        .collect()
}

fn infer_validate_binary_platform(binary: &str) -> Option<&'static str> {
    let binary = binary.replace('\\', "/");
    if binary.contains("/Contents/MacOS/") || binary.starts_with("Contents/MacOS/") {
        Some("macos")
    } else if binary.contains("/Contents/x86_64-win/") || binary.starts_with("Contents/x86_64-win/")
    {
        Some("windows-x64")
    } else if binary.contains("/Contents/x86_64-linux/")
        || binary.starts_with("Contents/x86_64-linux/")
    {
        Some("linux-x64")
    } else {
        None
    }
}

fn validate_report_bundle_name(report: &ValidateReport) -> String {
    let bundle = report.bundle.replace('\\', "/");
    bundle
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(bundle.as_str())
        .to_string()
}

fn format_platform_coverage(platforms: &mut Vec<&'static str>) -> String {
    platforms.sort_unstable();
    platforms.dedup();
    if platforms.is_empty() {
        "unknown".to_string()
    } else {
        platforms.join(", ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SigningEvidencePlatform {
    Macos,
    Windows,
}

impl SigningEvidencePlatform {
    fn label(self) -> &'static str {
        match self {
            SigningEvidencePlatform::Macos => "macOS",
            SigningEvidencePlatform::Windows => "Windows",
        }
    }
}

fn signed_bundle_evidence_release_check(paths: &[Utf8PathBuf], required: bool) -> ReleaseCheckItem {
    if paths.is_empty() {
        return optional_release_check_missing(
            "signed bundle evidence",
            required,
            "pass one or more `--signed-bundle-evidence <log-or-signed-bundle>` paths",
        );
    }

    let mut failures = Vec::new();
    let mut platforms = BTreeSet::new();
    for path in paths {
        match signing_evidence_platforms(path) {
            Ok(found) => {
                if let Err(error) = validate_signing_artifact_path_platform(path, &found) {
                    failures.push(format!("{path}: {error}"));
                }
                platforms.extend(found);
            }
            Err(error) => failures.push(format!("{path}: {error}")),
        }
    }

    if !failures.is_empty() {
        return ReleaseCheckItem {
            name: "signed bundle evidence".to_string(),
            status: "failed".to_string(),
            value: failures.join("; "),
            hint: Some(
                "use codesign/signtool verification logs, or a macOS .vst3 bundle with Contents/_CodeSignature"
                    .to_string(),
            ),
        };
    }

    let missing_required_platforms = [
        (SigningEvidencePlatform::Macos, "macOS codesign"),
        (SigningEvidencePlatform::Windows, "Windows signtool"),
    ]
    .into_iter()
    .filter_map(|(platform, label)| (!platforms.contains(&platform)).then_some(label))
    .collect::<Vec<_>>();
    if required && !missing_required_platforms.is_empty() {
        return ReleaseCheckItem {
            name: "signed bundle evidence".to_string(),
            status: "failed".to_string(),
            value: format!(
                "missing signing coverage: {}",
                missing_required_platforms.join(", ")
            ),
            hint: Some(
                "strict release artifacts require both macOS codesign and Windows signtool verification evidence"
                    .to_string(),
            ),
        };
    }

    let platform_labels = platforms
        .iter()
        .map(|platform| platform.label())
        .collect::<Vec<_>>()
        .join(", ");
    ReleaseCheckItem {
        name: "signed bundle evidence".to_string(),
        status: "ok".to_string(),
        value: format!(
            "{} evidence path(s); platforms: {}",
            paths.len(),
            if platform_labels.is_empty() {
                "unknown"
            } else {
                &platform_labels
            }
        ),
        hint: None,
    }
}

fn notarization_log_release_check(path: Option<&Utf8Path>, required: bool) -> ReleaseCheckItem {
    let Some(path) = path else {
        return optional_release_check_missing(
            "notarization log",
            required,
            "pass `--notarization-log <path>` containing accepted notarytool/stapler evidence",
        );
    };

    match notarization_evidence(path) {
        Ok(evidence) => {
            if let Err(error) = validate_notarization_artifact_path_platform(path) {
                return ReleaseCheckItem {
                    name: "notarization log".to_string(),
                    status: "failed".to_string(),
                    value: format!("{path}: {error}"),
                    hint: Some(
                        "notarization evidence should come from macOS notarytool and stapler logs"
                            .to_string(),
                    ),
                };
            }
            let mut missing = Vec::new();
            if !evidence.accepted {
                missing.push("accepted notarytool result");
            }
            if !evidence.stapled {
                missing.push("stapler success");
            }
            if required && !missing.is_empty() {
                return ReleaseCheckItem {
                    name: "notarization log".to_string(),
                    status: "failed".to_string(),
                    value: format!("missing notarization coverage: {}", missing.join(", ")),
                    hint: Some(
                        "strict release artifacts require both accepted notarytool output and stapler success evidence"
                            .to_string(),
                    ),
                };
            }
            ReleaseCheckItem {
                name: "notarization log".to_string(),
                status: "ok".to_string(),
                value: format!(
                    "{}; accepted={}; stapled={}",
                    path, evidence.accepted, evidence.stapled
                ),
                hint: None,
            }
        }
        Err(error) => ReleaseCheckItem {
            name: "notarization log".to_string(),
            status: "failed".to_string(),
            value: format!("{path}: {error}"),
            hint: Some(
                "include `xcrun notarytool submit --wait` accepted output and stapler output"
                    .to_string(),
            ),
        },
    }
}

fn optional_release_check_missing(
    name: &str,
    required: bool,
    hint: impl Into<String>,
) -> ReleaseCheckItem {
    ReleaseCheckItem {
        name: name.to_string(),
        status: if required { "failed" } else { "skipped" }.to_string(),
        value: if required {
            "required evidence missing".to_string()
        } else {
            "not requested".to_string()
        },
        hint: Some(hint.into()),
    }
}

#[derive(Clone, Debug)]
struct DoctorArtifact {
    path: Utf8PathBuf,
    os: Option<&'static str>,
    report: DoctorReport,
}

#[derive(Clone, Debug)]
struct CiReleaseCheckArtifact {
    path: Utf8PathBuf,
    os: Option<&'static str>,
    report: ReleaseCheckReport,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DoctorArtifactCoverage {
    linux: bool,
    macos: bool,
    windows: bool,
    os_mismatches: Vec<String>,
    missing_checks: Vec<String>,
    run_mismatches: Vec<String>,
}

impl DoctorArtifactCoverage {
    fn from_reports(reports: &[DoctorArtifact], expected_ci_run_url: Option<&str>) -> Self {
        let mut coverage = DoctorArtifactCoverage::default();
        let expected_run = expected_ci_run_url.and_then(github_actions_run_key);
        for artifact in reports {
            let Some(artifact_os) = artifact.os else {
                continue;
            };
            let Some(os) = doctor_report_os_matches_artifact(artifact_os, artifact, &mut coverage)
            else {
                continue;
            };
            match os {
                "Linux" => coverage.linux = true,
                "macOS" => coverage.macos = true,
                "Windows" => coverage.windows = true,
                _ => {}
            }
            coverage
                .missing_checks
                .extend(missing_doctor_checks(os, &artifact.report));
            if let (Some(expected), Some(actual_url)) =
                (expected_run.as_ref(), artifact.report.ci_run_url.as_deref())
            {
                match github_actions_run_key(actual_url) {
                    Some(actual) if actual == *expected => {}
                    Some(actual) => coverage.run_mismatches.push(format!(
                        "{os} expected {}/{} run {}, got {}/{} run {}",
                        expected.owner,
                        expected.repo,
                        expected.run_id,
                        actual.owner,
                        actual.repo,
                        actual.run_id
                    )),
                    None => coverage
                        .run_mismatches
                        .push(format!("{os} has invalid ci_run_url `{actual_url}`")),
                }
            }
        }
        coverage.os_mismatches.sort();
        coverage.os_mismatches.dedup();
        coverage.missing_checks.sort();
        coverage.missing_checks.dedup();
        coverage.run_mismatches.sort();
        coverage.run_mismatches.dedup();
        coverage
    }

    fn present_os(&self) -> Vec<&'static str> {
        [
            ("Linux", self.linux),
            ("macOS", self.macos),
            ("Windows", self.windows),
        ]
        .into_iter()
        .filter_map(|(os, present)| present.then_some(os))
        .collect()
    }

    fn missing_os(&self) -> Vec<&'static str> {
        [
            ("Linux", self.linux),
            ("macOS", self.macos),
            ("Windows", self.windows),
        ]
        .into_iter()
        .filter_map(|(os, present)| (!present).then_some(os))
        .collect()
    }

    fn missing_checks(&self) -> &[String] {
        &self.missing_checks
    }

    fn os_mismatches(&self) -> &[String] {
        &self.os_mismatches
    }

    fn run_mismatches(&self) -> &[String] {
        &self.run_mismatches
    }
}

fn doctor_report_os_matches_artifact<'a>(
    artifact_os: &'a str,
    artifact: &DoctorArtifact,
    coverage: &mut DoctorArtifactCoverage,
) -> Option<&'a str> {
    let Some(report_os) = artifact.report.os.as_deref() else {
        return Some(artifact_os);
    };
    match normalize_doctor_os_label(report_os) {
        Some(normalized) if normalized == artifact_os => Some(artifact_os),
        Some(normalized) => {
            coverage.os_mismatches.push(format!(
                "{} path indicates {artifact_os}, but report os is {normalized}",
                artifact.path
            ));
            None
        }
        None => {
            coverage.os_mismatches.push(format!(
                "{} has unrecognized report os `{report_os}`",
                artifact.path
            ));
            None
        }
    }
}

fn collect_doctor_reports(
    root: &Utf8Path,
) -> Result<Vec<DoctorArtifact>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("doctor artifact path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
    };
    if files.is_empty() {
        return Err(format!("no JSON doctor artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<DoctorReport>(&text)?;
            Ok(DoctorArtifact {
                path: path.clone(),
                os: artifact_os_from_path(&path),
                report,
            })
        })
        .collect()
}

fn collect_ci_release_check_reports(
    root: &Utf8Path,
) -> Result<Vec<CiReleaseCheckArtifact>, Box<dyn std::error::Error>> {
    let metadata =
        require_existing_file_or_directory_no_symlink("release-check artifact path", root)?;
    let files = if metadata.is_file() {
        vec![root.to_path_buf()]
    } else {
        collect_json_files_recursive(root)?
            .into_iter()
            .filter(|path| is_ci_release_check_report_path(path))
            .collect()
    };
    if files.is_empty() {
        return Err(format!("no JSON release-check artifacts found in {root}").into());
    }

    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let report = serde_json::from_str::<ReleaseCheckReport>(&text)?;
            Ok(CiReleaseCheckArtifact {
                path: path.clone(),
                os: artifact_os_from_path(&path),
                report,
            })
        })
        .collect()
}

fn is_ci_release_check_report_path(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name.starts_with("release-check") && name.ends_with(".json")
    })
}

fn validate_ci_release_check_report(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_check_report_shape(report)?;

    for (name, accepted_statuses) in [
        ("host profiles", &["ok"][..]),
        ("protocol snapshot", &["ok", "skipped"][..]),
        ("vst3 binding baseline", &["ok"][..]),
    ] {
        let Some(check) = report.checks.iter().find(|check| check.name == name) else {
            return Err(format!("missing local invariant check `{name}`").into());
        };
        if !accepted_statuses.contains(&check.status.as_str()) {
            return Err(format!(
                "local invariant check `{name}` status {} ({})",
                check.status, check.value
            )
            .into());
        }
        validate_ci_release_check_invariant_value(name, check)?;
    }

    let unexpected_failures = report
        .checks
        .iter()
        .filter(|check| check.status == "failed")
        .filter(|check| !ci_release_check_allows_failed_check(&check.name))
        .map(|check| format!("{}: {}", check.name, check.value))
        .collect::<Vec<_>>();
    if !unexpected_failures.is_empty() {
        return Err(format!(
            "unexpected failed local checks: {}",
            unexpected_failures.join("; ")
        )
        .into());
    }

    Ok(())
}

fn validate_ci_release_check_report_os_matches_path(
    artifact: &CiReleaseCheckArtifact,
) -> Result<(), String> {
    let Some(path_os) = artifact.os else {
        return Ok(());
    };
    let Some(report_os) = artifact.report.os.as_deref() else {
        return Ok(());
    };
    match normalize_doctor_os_label(report_os) {
        Some(normalized) if normalized == path_os => Ok(()),
        Some(normalized) => Err(format!(
            "{} path indicates {path_os}, but report os is {normalized}",
            artifact.path
        )),
        None => Err(format!(
            "{} has unrecognized report os `{report_os}`",
            artifact.path
        )),
    }
}

fn validate_release_check_report_shape(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    match report.status.as_str() {
        "ok" | "failed" => {}
        other => return Err(format!("unexpected report status `{other}`").into()),
    }
    if let Some(os) = &report.os {
        validate_release_action_text("release-check os", os)?;
        if normalize_doctor_os_label(os).is_none() {
            return Err(format!("invalid release-check os `{os}`").into());
        }
    }
    if let Some(ci_run_url) = &report.ci_run_url {
        validate_release_action_text("release-check ci run url", ci_run_url)?;
        if !is_github_actions_run_url(ci_run_url) {
            return Err(format!("invalid ci_run_url `{ci_run_url}`").into());
        }
    }
    if report.checks.is_empty() {
        return Err("release-check report must contain at least one check".into());
    }
    if report.checks.len() > RELEASE_CHECK_MAX_CHECKS {
        return Err(format!(
            "release-check report has too many checks: {} exceeds maximum {RELEASE_CHECK_MAX_CHECKS}",
            report.checks.len()
        )
        .into());
    }

    let mut check_names = BTreeSet::new();
    for check in &report.checks {
        validate_release_action_text("release-check check name", &check.name)?;
        validate_release_action_text(
            &format!("release-check `{}` status", check.name),
            &check.status,
        )?;
        if !matches!(check.status.as_str(), "ok" | "skipped" | "failed") {
            return Err(format!("unexpected check status: {}={}", check.name, check.status).into());
        }
        validate_release_action_text(
            &format!("release-check `{}` value", check.name),
            &check.value,
        )?;
        if let Some(hint) = &check.hint {
            validate_release_action_text(&format!("release-check `{}` hint", check.name), hint)?;
        }
        if !check_names.insert(check.name.as_str()) {
            return Err(format!("duplicate check name(s): {}", check.name).into());
        }
    }
    let has_failed_checks = report.checks.iter().any(|check| check.status == "failed");
    match (report.status.as_str(), has_failed_checks) {
        ("ok", true) => {
            return Err("report status `ok` is inconsistent with failed checks".into());
        }
        ("failed", false) => {
            return Err("report status `failed` is inconsistent with no failed checks".into());
        }
        _ => {}
    }

    validate_release_check_daw_matrix_shape(&report.daw_matrix)?;
    validate_release_check_check_set(&check_names)?;
    validate_release_check_daw_check_consistency(report)?;
    Ok(())
}

fn validate_release_check_check_set(
    check_names: &BTreeSet<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = expected_release_check_names();
    let missing = expected
        .iter()
        .filter(|expected| !check_names.contains(expected.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let extra = check_names
        .iter()
        .filter(|actual| !expected.contains(**actual))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() && extra.is_empty() {
        return Ok(());
    }

    let mut issues = Vec::new();
    if !missing.is_empty() {
        issues.push(format!("missing check(s): {}", missing.join(", ")));
    }
    if !extra.is_empty() {
        issues.push(format!("unknown check(s): {}", extra.join(", ")));
    }
    Err(format!(
        "release-check report check set must match current Vesty release gate: {}",
        issues.join("; ")
    )
    .into())
}

fn expected_release_check_names() -> BTreeSet<String> {
    let rows = vesty_core::host_profiles()
        .iter()
        .map(|profile| missing_daw_row(profile.name, &Utf8PathBuf::from("target/daw-evidence")))
        .collect::<Vec<_>>();
    build_release_check_report(
        rows,
        Utf8Path::new("target/vesty-protocol"),
        true,
        &ReleaseEvidenceOptions::default(),
    )
    .checks
    .into_iter()
    .map(|check| check.name)
    .collect()
}

fn validate_release_check_daw_check_consistency(
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_release_check_item_matches_daw_matrix(
        report,
        &host_profile_release_check(&report.daw_matrix),
    )?;
    validate_release_check_item_matches_daw_matrix(
        report,
        &daw_matrix_release_check(&report.daw_matrix),
    )?;

    let mut expected_daw_smoke_checks = BTreeSet::new();
    for row in &report.daw_matrix {
        let expected = host_row_release_check(row);
        expected_daw_smoke_checks.insert(expected.name.clone());
        validate_release_check_item_matches_daw_matrix(report, &expected)?;
    }

    for check in &report.checks {
        if check.name.starts_with("daw smoke: ") && !expected_daw_smoke_checks.contains(&check.name)
        {
            return Err(format!(
                "release-check `{}` has no matching canonical daw_matrix row",
                check.name
            )
            .into());
        }
    }

    Ok(())
}

fn validate_release_check_item_matches_daw_matrix(
    report: &ReleaseCheckReport,
    expected: &ReleaseCheckItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(actual) = report
        .checks
        .iter()
        .find(|check| check.name == expected.name)
    else {
        return Err(format!(
            "release-check report is missing `{}` check for current daw_matrix",
            expected.name
        )
        .into());
    };
    if actual.status != expected.status || actual.value != expected.value {
        return Err(format!(
            "release-check `{}` is inconsistent with daw_matrix: expected {} ({}) but found {} ({})",
            expected.name, expected.status, expected.value, actual.status, actual.value
        )
        .into());
    }
    Ok(())
}

fn validate_release_check_daw_matrix_shape(
    rows: &[serde_json::Value],
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_count = vesty_core::host_profiles().len();
    if rows.len() != expected_count {
        return Err(format!(
            "release-check daw_matrix must contain exactly {expected_count} release host profile rows, found {}",
            rows.len()
        )
        .into());
    }
    let host_diff = daw_matrix_host_set_diff(rows);
    let host_issues = daw_matrix_host_set_issues(&host_diff);
    if !host_issues.is_empty() {
        return Err(format!(
            "release-check daw_matrix host set mismatch: {}",
            host_issues.join("; ")
        )
        .into());
    }
    let mut seen_hosts = BTreeSet::new();
    for row in rows {
        let object = row
            .as_object()
            .ok_or("release-check daw_matrix rows must be objects")?;
        validate_release_check_daw_matrix_row_keys(object)?;
        let Some(host) = object.get("host").and_then(serde_json::Value::as_str) else {
            return Err("release-check daw_matrix row is missing host".into());
        };
        validate_release_action_text("release-check daw_matrix host", host)?;
        let Some(profile) = vesty_core::find_host_profile(host) else {
            return Err(format!("release-check daw_matrix row has unknown host `{host}`").into());
        };
        if host != profile.name {
            return Err(format!(
                "release-check daw_matrix row host `{host}` must use canonical host name `{}`",
                profile.name
            )
            .into());
        }
        if !seen_hosts.insert(host) {
            return Err(format!("duplicate release-check daw_matrix host `{host}`").into());
        }
        for key in ["evidence", "platform"] {
            let value = object
                .get(key)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    format!("release-check daw_matrix `{host}` field `{key}` must be a string")
                })?;
            validate_release_action_text(
                &format!("release-check daw_matrix `{host}` {key}"),
                value,
            )?;
        }
        for key in [
            "platform_supported",
            "scan",
            "load",
            "ui",
            "ui_host_param",
            "meter_stream",
            "automation",
            "buffer_sample_rate_change",
            "save_restore",
            "offline_render",
        ] {
            if let Some(value) = object.get(key)
                && !value.is_boolean()
            {
                return Err(format!(
                    "release-check daw_matrix `{host}` field `{key}` must be boolean"
                )
                .into());
            }
        }
        validate_release_check_daw_matrix_platform_consistency(host, object)?;
    }
    Ok(())
}

fn validate_release_check_daw_matrix_row_keys(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    const ALLOWED_KEYS: &[&str] = &[
        "host",
        "platform",
        "platform_supported",
        "scan",
        "load",
        "ui",
        "ui_host_param",
        "meter_stream",
        "automation",
        "buffer_sample_rate_change",
        "save_restore",
        "offline_render",
        "evidence",
    ];
    let allowed = ALLOWED_KEYS.iter().copied().collect::<BTreeSet<_>>();
    let unknown = object
        .keys()
        .filter(|key| !allowed.contains(key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "release-check daw_matrix row contains unknown field(s): {}",
            unknown.join(", ")
        )
        .into());
    }
    let missing = ALLOWED_KEYS
        .iter()
        .copied()
        .filter(|key| !object.contains_key(*key))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "release-check daw_matrix row is missing required field(s): {}",
            missing.join(", ")
        )
        .into());
    }
    Ok(())
}

fn validate_release_check_daw_matrix_platform_consistency(
    host: &str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return Ok(());
    };
    let Some(platform_supported) = object
        .get("platform_supported")
        .and_then(serde_json::Value::as_bool)
    else {
        return Ok(());
    };
    let platform = object
        .get("platform")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let platform_is_supported = daw_platform_evidence_supported(profile, platform);
    if platform_supported != platform_is_supported {
        return Err(format!(
            "release-check daw_matrix `{host}` platform_supported={platform_supported} is inconsistent with platform `{platform}` for {}",
            profile.name
        )
        .into());
    }
    Ok(())
}

fn validate_ci_release_check_invariant_value(
    name: &str,
    check: &ReleaseCheckItem,
) -> Result<(), Box<dyn std::error::Error>> {
    match name {
        "host profiles" => {
            let expected_count = vesty_core::host_profiles().len() as u32;
            let expected_value = format!(
                "{expected_count} release host profiles covered: {}",
                release_host_profile_names().join(", ")
            );
            if check.value != expected_value {
                return Err(format!(
                    "local invariant check `host profiles` value `{}` does not exactly match current host profile set `{expected_value}`",
                    check.value,
                )
                .into());
            }
        }
        "protocol snapshot" => {
            if check.status == "skipped" && !check.value.contains("--skip-protocol") {
                return Err(format!(
                    "local invariant check `protocol snapshot` skipped with unexpected value `{}`",
                    check.value
                )
                .into());
            }
            if check.status == "ok" && check.value.trim().is_empty() {
                return Err(
                    "local invariant check `protocol snapshot` ok has empty snapshot path".into(),
                );
            }
        }
        "vst3 binding baseline" => {
            let baseline = vesty_vst3_sys::BINDING_BASELINE;
            let backend = binding_backend_text(baseline.backend);
            if !check.value.contains(baseline.steinberg_sdk)
                || !check.value.contains(baseline.upstream_vst3_crate)
                || !check.value.contains(backend)
            {
                return Err(format!(
                    "local invariant check `vst3 binding baseline` value `{}` does not match current baseline {} / {} / {}",
                    check.value, baseline.steinberg_sdk, baseline.upstream_vst3_crate, backend
                )
                .into());
            }
        }
        _ => {}
    }
    Ok(())
}

fn ci_release_check_allows_failed_check(name: &str) -> bool {
    name.starts_with("daw smoke:")
        || matches!(
            name,
            "daw matrix"
                | "daw smoke matrix"
                | "ci run url"
                | "ci doctor artifacts"
                | "ci release-check artifacts"
                | "platform smoke artifacts"
                | "vst3 validate reports"
                | "vst3 example validator coverage"
                | "vst3 static validate reports"
                | "ci example static validate coverage"
                | "crate publish plan"
                | "crate package readiness"
                | "npm package pack report"
                | "dependency latest baseline"
                | "vst3 SDK header manifest"
                | "vst3 SDK generated bindings plan"
                | "vst3 SDK generated bindings surface"
                | "vst3 SDK generated bindings scaffold"
                | "vst3 SDK generated bindings ABI seed"
                | "vst3 SDK generated bindings ABI layout"
                | "vst3 SDK generated bindings interface skeleton"
                | "signed bundle evidence"
                | "notarization log"
        )
}

fn collect_json_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_json_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_evidence_text_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_evidence_text_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_rust_files_recursive_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_vst3_bundle_dirs_recursive(
    root: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>, Box<dyn std::error::Error>> {
    let mut bundles = Vec::new();
    collect_vst3_bundle_dirs_recursive_inner(root, &mut bundles)?;
    bundles.sort();
    Ok(bundles)
}

fn collect_json_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "JSON artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("JSON artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_json_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_evidence_text_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "evidence artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("evidence artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_evidence_text_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("log") || extension.eq_ignore_ascii_case("txt")
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_rust_files_recursive_inner(
    current: &Utf8Path,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "Rust artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("Rust artifact contains symlink: {path}").into());
        }
        if metadata.is_dir() {
            collect_rust_files_recursive_inner(&path, files)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_vst3_bundle_dirs_recursive_inner(
    current: &Utf8Path,
    bundles: &mut Vec<Utf8PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| "evidence artifact path is not valid utf-8")?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("evidence artifact contains symlink: {path}").into());
        }
        if !metadata.is_dir() {
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
        {
            bundles.push(path);
        } else {
            collect_vst3_bundle_dirs_recursive_inner(&path, bundles)?;
        }
    }
    Ok(())
}

fn doctor_artifact_os(path: &Utf8Path) -> Option<&'static str> {
    let name = Utf8Path::new(path.file_name()?);
    os_from_artifact_path_tokens(&artifact_path_tokens(name))
}

fn normalize_doctor_os_label(value: &str) -> Option<&'static str> {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    match normalized.as_str() {
        "linux" => Some("Linux"),
        "macos" | "darwin" | "osx" => Some("macOS"),
        "windows" | "win32" | "win64" => Some("Windows"),
        _ => None,
    }
}

fn missing_doctor_checks(os: &str, report: &DoctorReport) -> Vec<String> {
    const OK: &[&str] = &["ok"];
    const OK_OR_UNKNOWN: &[&str] = &["ok", "unknown"];
    const OK_OR_SKIPPED: &[&str] = &["ok", "skipped"];

    let mut required = vec![
        ("rustc", OK),
        ("cargo", OK),
        ("node", OK),
        ("npm", OK),
        ("vst3 binding baseline", OK),
        ("vst3 SDK headers", OK_OR_SKIPPED),
        ("vst3 validator", OK),
        ("system webview", OK),
    ];
    match os {
        "Linux" => required.push(("signing: linux release policy", OK_OR_UNKNOWN)),
        "macOS" => {
            required.push(("signing: codesign", OK));
            required.push(("signing: notarytool", OK));
        }
        "Windows" => required.push(("signing: signtool", OK)),
        _ => {}
    }

    required
        .into_iter()
        .filter_map(|(required, accepted_statuses)| {
            let check = report.checks.iter().find(|check| check.name == required)?;
            (!accepted_statuses.contains(&check.status.as_str()))
                .then(|| format!("{os}/{required} status {}", check.status))
        })
        .chain(
            required_doctor_check_names(os)
                .into_iter()
                .filter(|required| !report.checks.iter().any(|check| check.name == *required))
                .map(|required| format!("{os}/{required} missing")),
        )
        .chain(
            unexpected_doctor_checks_for_os(os, report)
                .into_iter()
                .map(|check| format!("{os}/{check} unexpected for {os} doctor report")),
        )
        .collect()
}

fn required_doctor_check_names(os: &str) -> Vec<&'static str> {
    let mut required = vec![
        "rustc",
        "cargo",
        "node",
        "npm",
        "vst3 binding baseline",
        "vst3 SDK headers",
        "vst3 validator",
        "system webview",
    ];
    match os {
        "Linux" => required.push("signing: linux release policy"),
        "macOS" => {
            required.push("signing: codesign");
            required.push("signing: notarytool");
        }
        "Windows" => required.push("signing: signtool"),
        _ => {}
    }
    required
}

fn validate_signing_evidence(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    signing_evidence_platforms(path).map(|_| ())
}

const SIGNING_NOTARIZATION_LOG_MAX_BYTES: usize = 512 * 1024;
const MACOS_CODE_RESOURCES_MAX_BYTES: u64 = 16 * 1024 * 1024;

fn signing_evidence_platforms(
    path: &Utf8Path,
) -> Result<BTreeSet<SigningEvidencePlatform>, Box<dyn std::error::Error>> {
    let metadata = require_existing_file_or_directory_no_symlink("signing evidence path", path)?;
    if metadata.is_dir() {
        if path.extension() == Some("vst3") {
            validate_macos_code_resources(path)?;
            return Ok(BTreeSet::from([SigningEvidencePlatform::Macos]));
        }
        return Err("directory is not a macOS signed .vst3 bundle".into());
    }

    let text = read_text_file_no_symlink("signing evidence path", path)?;
    signing_evidence_platforms_from_text(&text)
}

fn signing_evidence_platforms_from_text(
    text: &str,
) -> Result<BTreeSet<SigningEvidencePlatform>, Box<dyn std::error::Error>> {
    validate_release_evidence_log_text("signing evidence log", text)?;

    let lower = text.to_ascii_lowercase();
    if let Some(reason) = signing_negative_evidence(&lower) {
        return Err(format!("negative signing evidence found: {reason}").into());
    }

    let mut platforms = BTreeSet::new();
    if explicit_truthy_marker(&lower, &["codesign"]) {
        platforms.insert(SigningEvidencePlatform::Macos);
    }
    if explicit_truthy_marker(&lower, &["signtool"]) {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    let macos_codesign =
        lower.contains("valid on disk") && lower.contains("satisfies its designated requirement");
    if macos_codesign {
        platforms.insert(SigningEvidencePlatform::Macos);
    }
    if lower.contains("successfully verified") {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    let windows_signtool_summary =
        lower.contains("signtool") && signtool_error_count(&lower) == Some(0);
    if windows_signtool_summary {
        platforms.insert(SigningEvidencePlatform::Windows);
    }
    if platforms.is_empty() {
        Err("no positive signing marker found".into())
    } else {
        Ok(platforms)
    }
}

fn signing_negative_evidence(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if explicit_falsy_marker_line(
            line,
            &["signed", "signing", "signature", "codesign", "signtool"],
        ) {
            return Some(line.to_string());
        }
        if extract_count_near_marker(line, "number of errors").is_some_and(|errors| errors > 0) {
            return Some(line.to_string());
        }
        if line_contains_any(
            line,
            &[
                "signtool error:",
                "sign tool error:",
                "codesign failed",
                "failed to verify",
                "verification failed",
                "signature verification failed",
                "code object is not signed",
                "not signed at all",
                "invalid signature",
                "signature is invalid",
                "sealed resource is missing or invalid",
                "no signature found",
            ],
        ) {
            return Some(line.to_string());
        }
    }
    None
}

fn signtool_error_count(text: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| extract_count_near_marker(line, "number of errors"))
}

fn validate_macos_code_resources(bundle: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    require_existing_directory_no_symlink("macOS bundle Contents", &bundle.join("Contents"))?;
    require_existing_directory_no_symlink(
        "macOS CodeSignature directory",
        &bundle.join("Contents/_CodeSignature"),
    )?;
    let code_resources = bundle.join("Contents/_CodeSignature/CodeResources");
    let metadata = match require_existing_file_no_symlink("macOS CodeResources", &code_resources) {
        Ok(metadata) => metadata,
        Err(error) if error.to_string().contains("does not exist") => {
            return Err(
                "macOS signed bundle evidence is missing Contents/_CodeSignature/CodeResources"
                    .into(),
            );
        }
        Err(error) => return Err(error),
    };
    if metadata.len() > MACOS_CODE_RESOURCES_MAX_BYTES {
        return Err(format!(
            "macOS CodeResources is too large: {} bytes exceeds maximum {MACOS_CODE_RESOURCES_MAX_BYTES}",
            metadata.len()
        )
        .into());
    }

    let plist = plist::Value::from_file(&code_resources)
        .map_err(|error| format!("CodeResources is not a parseable plist: {error}"))?;
    let dict = plist
        .as_dictionary()
        .ok_or("CodeResources plist root must be a dictionary")?;
    if !has_code_resources_file_map(dict, "files") && !has_code_resources_file_map(dict, "files2") {
        return Err("CodeResources plist must contain files or files2 dictionary entries".into());
    }
    Ok(())
}

fn has_code_resources_file_map(dict: &plist::Dictionary, key: &str) -> bool {
    dict.get(key)
        .and_then(plist::Value::as_dictionary)
        .is_some()
}

fn validate_notarization_evidence(path: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    notarization_evidence(path).map(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NotarizationEvidence {
    accepted: bool,
    stapled: bool,
}

impl NotarizationEvidence {
    const fn is_positive(self) -> bool {
        self.accepted || self.stapled
    }
}

fn notarization_evidence(
    path: &Utf8Path,
) -> Result<NotarizationEvidence, Box<dyn std::error::Error>> {
    let text = read_text_file_no_symlink("notarization evidence", path)?;
    notarization_evidence_from_text(&text)
}

fn notarization_evidence_from_text(
    text: &str,
) -> Result<NotarizationEvidence, Box<dyn std::error::Error>> {
    validate_release_evidence_log_text("notarization evidence log", text)?;

    let lower = text.to_ascii_lowercase();
    if let Some(reason) = notarization_negative_evidence(&lower) {
        return Err(format!("negative notarization evidence found: {reason}").into());
    }

    let accepted = explicit_truthy_marker(&lower, &["notarytool"])
        || explicit_status_marker(&lower, "accepted")
        || notarytool_json_status_accepted(text);
    let stapled = explicit_truthy_marker(&lower, &["stapled"]) || explicit_stapler_success(&lower);
    let evidence = NotarizationEvidence { accepted, stapled };
    if evidence.is_positive() {
        Ok(evidence)
    } else {
        Err("no accepted notarization marker found".into())
    }
}

fn notarization_negative_evidence(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if notarization_metadata_line(line) {
            continue;
        }
        if explicit_falsy_marker_line(line, &["notarization", "notary", "notarytool", "stapled"])
            || explicit_marker_line_matches(line, &["status"], &["rejected", "invalid", "failed"])
        {
            return Some(line.to_string());
        }
        if line_contains_any(
            line,
            &[
                "status: rejected",
                "status=rejected",
                r#""status":"rejected""#,
                r#""status": "rejected""#,
                "status: invalid",
                "status=invalid",
                r#""status":"invalid""#,
                r#""status": "invalid""#,
                "notarytool error",
                "notarization failed",
                "submission failed",
                "stapler failed",
                "staple failed",
                "validate action failed",
            ],
        ) {
            return Some(line.to_string());
        }
    }
    None
}

fn explicit_status_marker(text: &str, value: &str) -> bool {
    text.lines()
        .any(|line| explicit_marker_line_matches(line, &["status"], &[value]))
}

fn notarytool_json_status_accepted(text: &str) -> bool {
    json_status_accepted(text)
        || bracketed_log_section(text, "notarytool").is_some_and(|section| {
            json_status_accepted(&section)
                || bracketed_log_section(&section, "stdout")
                    .is_some_and(|stdout| json_status_accepted(&stdout))
        })
}

fn json_status_accepted(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text.trim())
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_ascii_lowercase)
        })
        .is_some_and(|status| status == "accepted")
}

fn bracketed_log_section(text: &str, section: &str) -> Option<String> {
    let header = format!("[{}]", section.to_ascii_lowercase());
    let mut found = false;
    let mut content = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let is_header = trimmed.starts_with('[') && trimmed.ends_with(']');
        if is_header {
            if found {
                break;
            }
            if trimmed.to_ascii_lowercase() == header {
                found = true;
            }
            continue;
        }
        if found && notarization_metadata_line(trimmed) {
            break;
        }
        if found {
            content.push_str(line);
            content.push('\n');
        }
    }
    found.then_some(content)
}

fn explicit_stapler_success(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim() == "the staple and validate action worked!")
}

fn notarization_metadata_line(line: &str) -> bool {
    split_marker_assignment(line).is_some_and(|(key, _)| {
        matches!(
            key.trim().replace('-', "_").as_str(),
            "notary_log" | "stapler_log"
        )
    })
}

fn validate_release_evidence_log_text(
    label: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty").into());
    }
    if value.len() > SIGNING_NOTARIZATION_LOG_MAX_BYTES {
        return Err(
            format!("{label} must be at most {SIGNING_NOTARIZATION_LOG_MAX_BYTES} bytes").into(),
        );
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(
            format!("{label} must not contain control characters other than tab/newline").into(),
        );
    }
    if value.chars().any(is_release_action_unsafe_format_char) {
        return Err(format!("{label} must not contain unsafe Unicode format characters").into());
    }
    Ok(())
}

fn print_release_check_report(
    report: &ReleaseCheckReport,
    format: &str,
    report_path: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    write_release_check_report(report_path, report)?;
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(report)?),
        "markdown" | "md" => {
            println!("# Vesty Release Check\n");
            println!("status: {}", report.status);
            println!();
            println!("| Check | Status | Value | Hint |");
            println!("| --- | --- | --- | --- |");
            for check in &report.checks {
                println!(
                    "| {} | {} | {} | {} |",
                    check.name,
                    check.status,
                    check.value,
                    check.hint.as_deref().unwrap_or("")
                );
            }
            println!();
            print_daw_matrix(&report.daw_matrix, "markdown")?;
        }
        _ => return Err(format!("unsupported release check format '{format}'").into()),
    }
    Ok(())
}

fn write_release_check_report(
    path: Option<&Utf8Path>,
    report: &ReleaseCheckReport,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = validate_release_check_report_shape(report) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid release-check report: {error}"),
        )
        .into());
    }
    let Some(path) = path else {
        return Ok(());
    };
    let text = serde_json::to_string_pretty(report)?;
    write_text_file(path, &(text + "\n"))
}

#[cfg(test)]
fn collect_reaper_evidence(dir: &Utf8PathBuf) -> serde_json::Value {
    let profile = vesty_core::find_host_profile("reaper").expect("REAPER profile exists");
    collect_reaper_evidence_for_profile(profile, dir)
}

fn collect_reaper_evidence_for_profile(
    profile: &vesty_core::HostProfile,
    dir: &Utf8PathBuf,
) -> serde_json::Value {
    if daw_evidence_dir_status(dir) == DawEvidenceDirStatus::Blocked {
        return missing_daw_row(profile.name, dir);
    }

    let scan_marker = read_first_optional(dir, &["scan-smoke.log", "scan.log"]);
    let load = read_optional(dir.join("load-smoke.log"));
    let restore = read_optional(dir.join("restore-smoke.log"));
    let ui = read_optional(dir.join("ui-smoke.log"));
    let render = read_optional(dir.join("render-smoke.log"));
    let param_watch = read_optional(dir.join("param-watch.log"));
    let bridge_trace = read_optional(dir.join("bridge-trace.log"));
    let meter_stream = read_optional(dir.join("meter-stream.log"));
    let automation = read_optional(dir.join("automation-smoke.log"));
    let buffer_sample_rate = read_first_optional(
        dir,
        &["buffer-sample-rate.log", "buffer-sample-rate-smoke.log"],
    );
    let offline_render = read_optional(dir.join("offline-render.log"));
    let (platform, platform_supported) =
        evidence_platform_for_profile(profile, dir, Some("macOS arm64"));
    let scan = scan_marker
        .as_deref()
        .map(|text| daw_marker_matches_for_profile(profile, text, generic_scan_ok))
        .unwrap_or_else(|| {
            reaper_scan_cache()
                .and_then(read_optional)
                .is_some_and(|text| {
                    daw_marker_matches_for_profile(profile, &text, |text| {
                        [
                            "VestyGain.vst3",
                            "VestyWebUIDemo.vst3",
                            "VestyMIDISynth.vst3",
                        ]
                        .iter()
                        .all(|needle| text.contains(needle))
                    })
                })
        });

    let load_ok = load.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            ["Vesty Gain", "Vesty Web UI Demo", "Vesty MIDI Synth"]
                .iter()
                .all(|needle| text.contains(needle))
                && text.matches("ok=true").count() >= 3
                || generic_load_ok(text)
        })
    });
    let restore_ok = restore.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            (text.contains("track_count=3") && text.matches("ok=true").count() >= 3)
                || generic_restore_ok(text)
        })
    });
    let ui_ok = ui.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            (text.contains("target_fx=VST3: Vesty Web UI Demo (Vesty)|ok=true")
                && text.contains("ui_show_called=true"))
                || generic_ui_ok(text)
        })
    });
    let automation_ok = render
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_automation_ok))
        || automation.as_deref().is_some_and(|text| {
            daw_marker_matches_for_profile(profile, text, generic_automation_ok)
        });
    let ui_host_param_ok = param_watch
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_ui_host_ok))
        || read_optional(dir.join("ui-host-smoke.log"))
            .as_deref()
            .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_ui_host_ok))
        || bridge_trace.as_deref().is_some_and(|text| {
            daw_marker_matches_for_profile(profile, text, bridge_trace_relayed_param_gesture)
        });
    let meter_stream_ok = meter_stream
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, meter_stream_delivered))
        || bridge_trace.as_deref().is_some_and(|text| {
            daw_marker_matches_for_profile(profile, text, meter_stream_delivered)
        });
    let buffer_sample_rate_ok = buffer_sample_rate.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, generic_buffer_sample_rate_change_ok)
    });
    let offline_render_ok = render.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            generic_offline_render_ok(text) || render_file_evidence_ok(text, dir)
        })
    }) || offline_render.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            generic_offline_render_ok(text) || render_file_evidence_ok(text, dir)
        })
    });

    serde_json::json!({
        "host": profile.name,
        "platform": platform,
        "platform_supported": platform_supported,
        "scan": scan,
        "load": load_ok,
        "ui": ui_ok,
        "ui_host_param": ui_host_param_ok,
        "meter_stream": meter_stream_ok,
        "automation": automation_ok,
        "buffer_sample_rate_change": buffer_sample_rate_ok,
        "save_restore": restore_ok,
        "offline_render": offline_render_ok,
        "evidence": dir.to_string(),
    })
}

#[cfg(test)]
fn collect_generic_daw_evidence(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return missing_daw_row(host, dir);
    };
    collect_generic_daw_evidence_for_profile(profile, dir)
}

fn collect_generic_daw_evidence_for_profile(
    profile: &vesty_core::HostProfile,
    dir: &Utf8PathBuf,
) -> serde_json::Value {
    if daw_evidence_dir_status(dir) != DawEvidenceDirStatus::Present {
        return missing_daw_row(profile.name, dir);
    }

    let (platform, platform_supported) = evidence_platform_for_profile(profile, dir, None);
    let scan = read_first_optional(dir, &["scan-smoke.log", "scan.log"]);
    let load = read_first_optional(dir, &["load-smoke.log", "load.log"]);
    let ui = read_first_optional(dir, &["ui-smoke.log", "ui.log"]);
    let ui_host = read_first_optional(dir, &["ui-host-smoke.log", "param-watch.log"]);
    let meter = read_first_optional(dir, &["meter-stream.log"]);
    let bridge_trace = read_optional(dir.join("bridge-trace.log"));
    let automation = read_first_optional(dir, &["automation-smoke.log", "render-smoke.log"]);
    let buffer_sample_rate = read_first_optional(
        dir,
        &["buffer-sample-rate.log", "buffer-sample-rate-smoke.log"],
    );
    let restore = read_first_optional(dir, &["restore-smoke.log", "restore.log"]);
    let render = read_first_optional(dir, &["render-smoke.log", "offline-render.log"]);

    let scan_ok = scan
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_scan_ok));
    let load_ok = load
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_load_ok));
    let ui_ok = ui
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_ui_ok));
    let ui_host_param_ok = ui_host
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_ui_host_ok))
        || bridge_trace.as_deref().is_some_and(|text| {
            daw_marker_matches_for_profile(profile, text, bridge_trace_relayed_param_gesture)
        });
    let meter_stream_ok = meter
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, meter_stream_delivered))
        || bridge_trace.as_deref().is_some_and(|text| {
            daw_marker_matches_for_profile(profile, text, meter_stream_delivered)
        });
    let automation_ok = automation
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_automation_ok));
    let buffer_sample_rate_ok = buffer_sample_rate.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, generic_buffer_sample_rate_change_ok)
    });
    let save_restore_ok = restore
        .as_deref()
        .is_some_and(|text| daw_marker_matches_for_profile(profile, text, generic_restore_ok));
    let offline_render_ok = render.as_deref().is_some_and(|text| {
        daw_marker_matches_for_profile(profile, text, |text| {
            generic_offline_render_ok(text) || render_file_evidence_ok(text, dir)
        })
    });

    serde_json::json!({
        "host": profile.name,
        "platform": platform,
        "platform_supported": platform_supported,
        "scan": scan_ok,
        "load": load_ok,
        "ui": ui_ok,
        "ui_host_param": ui_host_param_ok,
        "meter_stream": meter_stream_ok,
        "automation": automation_ok,
        "buffer_sample_rate_change": buffer_sample_rate_ok,
        "save_restore": save_restore_ok,
        "offline_render": offline_render_ok,
        "evidence": dir.to_string(),
    })
}

fn missing_daw_row(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
    serde_json::json!({
        "host": host,
        "platform": "manual matrix pending",
        "platform_supported": false,
        "scan": false,
        "load": false,
        "ui": false,
        "ui_host_param": false,
        "meter_stream": false,
        "automation": false,
        "buffer_sample_rate_change": false,
        "save_restore": false,
        "offline_render": false,
        "evidence": dir.to_string(),
    })
}

fn read_optional(path: Utf8PathBuf) -> Option<String> {
    read_text_file_no_symlink("DAW evidence marker", &path).ok()
}

fn read_first_optional(dir: &Utf8PathBuf, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| read_optional(dir.join(name)))
}

fn evidence_platform_for_profile(
    profile: &vesty_core::HostProfile,
    dir: &Utf8PathBuf,
    default_platform: Option<&str>,
) -> (String, bool) {
    let platform_path = dir.join("platform.txt");
    let platform_text = read_optional(platform_path.clone());
    let platform_path_exists = fs::symlink_metadata(&platform_path).is_ok();
    let (display, validation_text) = match platform_text {
        Some(text) => {
            let display = text
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| "manual evidence".to_string());
            (display, Some(text.trim().to_string()))
        }
        None if platform_path_exists => ("manual evidence".to_string(), None),
        None => {
            let display = default_platform.unwrap_or("manual evidence").to_string();
            (display.clone(), default_platform.map(str::to_string))
        }
    };
    let supported = validation_text
        .as_deref()
        .is_some_and(|text| daw_platform_evidence_supported(profile, text));
    (display, supported)
}

fn daw_platform_evidence_supported(profile: &vesty_core::HostProfile, value: &str) -> bool {
    required_daw_platform_marker(profile, Some(value.to_string())).is_ok()
}

fn reaper_scan_cache() -> Option<Utf8PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        Utf8PathBuf::from(home)
            .join("Library/Application Support/REAPER/reaper-vstplugins_arm64.ini"),
    )
}

fn render_file_from_log(text: &str) -> Option<Utf8PathBuf> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("render_file") {
            return None;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        (!value.is_empty()).then(|| Utf8PathBuf::from(value))
    })
}

fn render_file_evidence_ok(text: &str, evidence_dir: &Utf8Path) -> bool {
    render_file_from_log(text).is_some_and(|path| {
        let path = if path.is_absolute() {
            path
        } else if path.as_str().split(['/', '\\']).any(|part| part == "..") {
            return false;
        } else {
            evidence_dir.join(path)
        };
        render_file_exists_and_nonempty(path)
    })
}

fn render_file_exists_and_nonempty(path: Utf8PathBuf) -> bool {
    require_existing_file_no_symlink("render file evidence", &path)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
}

fn reaper_param_watch_moved(text: &str) -> bool {
    if !text.contains("target_fx=VST3: Vesty Web UI Demo (Vesty)|ok=true") {
        return false;
    }

    let values = text
        .lines()
        .filter_map(|line| line.split_once("param0="))
        .filter_map(|(_, value)| value.parse::<f64>().ok());
    let mut saw_initial = false;
    let mut saw_target = false;
    for value in values {
        saw_initial |= (0.49..=0.51).contains(&value);
        saw_target |= value >= 0.88;
    }
    saw_initial && saw_target
}

fn daw_marker_matches(text: &str, predicate: impl FnOnce(&str) -> bool) -> bool {
    daw_marker_positive(text) && predicate(text)
}

fn daw_marker_matches_for_profile(
    profile: &vesty_core::HostProfile,
    text: &str,
    predicate: impl FnOnce(&str) -> bool,
) -> bool {
    daw_marker_positive(text) && daw_marker_host_scope_matches(profile, text) && predicate(text)
}

fn daw_marker_positive(text: &str) -> bool {
    !daw_marker_has_missing_assignment(text) && !daw_marker_has_negative_evidence(text)
}

fn daw_marker_host_scope_matches(profile: &vesty_core::HostProfile, text: &str) -> bool {
    for (key, value) in text.lines().flat_map(marker_assignments) {
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        if !matches!(
            key.as_str(),
            "host" | "daw" | "daw_host" | "host_profile" | "profile"
        ) {
            continue;
        }
        let value = value
            .trim()
            .trim_matches(['`', '"', '\''])
            .trim_end_matches([',', ';']);
        let Some(found) = vesty_core::find_host_profile(value) else {
            return false;
        };
        if found.id != profile.id {
            return false;
        }
    }
    true
}

fn bridge_trace_relayed_param_gesture(text: &str) -> bool {
    let legacy_trace = text.contains("ParamGesture { phase: Begin")
        && text.contains("ParamGesture { phase: Perform")
        && text.contains("ParamGesture { phase: End");
    let packet_trace = text.contains(r#""type":"param.begin""#)
        && text.contains(r#""type":"param.perform""#)
        && text.contains(r#""type":"param.end""#);
    (legacy_trace || packet_trace) && text.contains("result=0")
}

fn generic_scan_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["scan", "scan_ok"])
        || [
            "VestyGain.vst3",
            "VestyWebUIDemo.vst3",
            "VestyMIDISynth.vst3",
        ]
        .iter()
        .all(|needle| text.contains(needle))
}

fn generic_load_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["load", "load_ok"])
        || (vesty_plugin_names_present(text) && text.matches("ok=true").count() >= 3)
}

fn generic_ui_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["ui", "ui_ok"])
        || (text.contains("Vesty Web UI Demo") && text.contains("ui_show_called=true"))
}

fn generic_ui_host_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["ui_host_param", "ui_host", "host_param"])
        || reaper_param_watch_moved(text)
}

fn generic_automation_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["automation", "automation_ok"])
        || (text.contains("automation_points=3")
            && text.contains("midi_note_inserted=true")
            && text.contains("project_ready=true"))
}

fn generic_buffer_sample_rate_change_ok(text: &str) -> bool {
    explicit_truthy_marker(
        text,
        &[
            "buffer_sample_rate_change",
            "buffer_change",
            "buffer_size_change",
            "sample_rate_change",
        ],
    ) || (text.contains("buffer_size_changed=true") && text.contains("sample_rate_changed=true"))
}

fn generic_restore_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["save_restore", "restore", "restore_ok"])
        || (text.contains("track_count=3") && text.matches("ok=true").count() >= 3)
}

fn generic_offline_render_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["offline_render", "render", "render_ok"])
}

fn vesty_plugin_names_present(text: &str) -> bool {
    ["Vesty Gain", "Vesty Web UI Demo", "Vesty MIDI Synth"]
        .iter()
        .all(|needle| text.contains(needle))
}

fn explicit_truthy_marker(text: &str, keys: &[&str]) -> bool {
    text.lines()
        .any(|line| explicit_truthy_marker_line(line, keys))
}

fn explicit_truthy_marker_line(line: &str, keys: &[&str]) -> bool {
    explicit_marker_line_matches(line, keys, &["true", "pass", "ok"])
}

fn explicit_falsy_marker_line(line: &str, keys: &[&str]) -> bool {
    explicit_marker_line_matches(
        line,
        keys,
        &[
            "false", "fail", "failed", "error", "invalid", "rejected", "pending",
        ],
    )
}

fn explicit_marker_line_matches(line: &str, keys: &[&str], values: &[&str]) -> bool {
    line.split(';')
        .any(|fragment| explicit_marker_fragment_matches(fragment, keys, values))
}

fn explicit_marker_fragment_matches(fragment: &str, keys: &[&str], values: &[&str]) -> bool {
    let normalized = fragment.trim().to_ascii_lowercase().replace('-', "_");
    let Some((raw_key, raw_value)) = split_marker_assignment(&normalized) else {
        return false;
    };

    let key = raw_key.trim().trim_matches(['`', '"', '\'']);
    let value = raw_value
        .trim()
        .trim_start_matches(['`', '"', '\''])
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .find(|token| !token.is_empty())
        .unwrap_or_default();
    if !values.contains(&value) {
        return false;
    }

    keys.iter().any(|candidate| {
        let candidate = candidate.to_ascii_lowercase().replace('-', "_");
        key == candidate || key == format!("{candidate}_ok")
    })
}

fn line_contains_any(line: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| line.contains(needle))
}

fn split_marker_assignment(line: &str) -> Option<(&str, &str)> {
    if let Some((key, value)) = line.split_once('=') {
        Some((key, value))
    } else {
        line.split_once(':')
    }
}

fn marker_assignments(line: &str) -> impl Iterator<Item = (&str, &str)> {
    line.split(';').filter_map(split_marker_assignment)
}

fn meter_stream_delivered(text: &str) -> bool {
    let flush_sent = text.lines().any(|line| {
        line.contains("meter_flush sent=")
            && !line.contains("meter_flush sent=0")
            && !line.contains("meter_flush sent=false")
    });
    let bridge_packet = text.contains(r#""lane":"meter""#)
        && text.contains(r#""type":"meter.main""#)
        && (text.contains(r#""peaks""#) || text.contains(r#""rms""#));
    let log_frame = text.contains("meter.main")
        && (text.contains("peaks=")
            || text.contains("rms=")
            || text.lines().any(line_has_nonzero_peak));
    flush_sent || bridge_packet || log_frame
}

fn line_has_nonzero_peak(line: &str) -> bool {
    line.split_once("peak=")
        .and_then(|(_, rest)| {
            rest.split(|char: char| !(char.is_ascii_digit() || matches!(char, '.' | '-' | '+')))
                .next()
        })
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value > 0.0)
}

mod development;
use development::*;

mod scaffold;
use scaffold::*;

#[cfg(test)]
mod tests;
