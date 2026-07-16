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

mod vst3_sdk;
use vst3_sdk::*;
mod release_evidence;
use release_evidence::*;

mod ci_evidence;
use ci_evidence::*;

mod ci_vst3_artifacts;
use ci_vst3_artifacts::*;

mod release_evidence_validation;
use release_evidence_validation::*;

mod publishing;
use publishing::*;
mod package_validation;
use package_validation::*;

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
