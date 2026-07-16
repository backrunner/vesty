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

mod daw_evidence;
use daw_evidence::*;
mod release_planning;
use release_planning::*;

mod release_readiness;
use release_readiness::*;
mod platform_smoke;
use platform_smoke::*;

mod smoke_host;
use smoke_host::*;

mod validate_evidence;
use validate_evidence::*;
mod ci_release_checks;
use ci_release_checks::*;
mod daw_collection;
use daw_collection::*;

mod development;
use development::*;

mod scaffold;
use scaffold::*;

#[cfg(test)]
mod tests;
