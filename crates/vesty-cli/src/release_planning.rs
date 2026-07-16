use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseCheckReport {
    pub(super) status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) ci_run_url: Option<String>,
    pub(super) checks: Vec<ReleaseCheckItem>,
    pub(super) daw_matrix: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseCheckItem {
    pub(super) name: String,
    pub(super) status: String,
    pub(super) value: String,
    pub(super) hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseActionPlan {
    pub(super) version: u32,
    pub(super) status: String,
    pub(super) summary: ReleaseActionPlanSummary,
    pub(super) protocol_snapshot: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) evidence_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) release_evidence_dir: Option<String>,
    pub(super) actions: Vec<ReleaseActionItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseActionPlanSummary {
    pub(super) ok: usize,
    pub(super) failed: usize,
    pub(super) skipped: usize,
    pub(super) action_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseActionItem {
    pub(super) check: String,
    pub(super) status: String,
    pub(super) priority: String,
    pub(super) value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) evidence_path: Option<String>,
    pub(super) commands: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ReleaseEvidenceOptions {
    pub(super) ci_doctor_dir: Option<Utf8PathBuf>,
    pub(super) ci_release_check_dir: Option<Utf8PathBuf>,
    pub(super) platform_smoke_dir: Option<Utf8PathBuf>,
    pub(super) ci_run_url: Option<String>,
    pub(super) validate_reports: Vec<Utf8PathBuf>,
    pub(super) static_validate_reports: Vec<Utf8PathBuf>,
    pub(super) publish_plan_report: Option<Utf8PathBuf>,
    pub(super) crate_package_report: Option<Utf8PathBuf>,
    pub(super) npm_pack_report: Option<Utf8PathBuf>,
    pub(super) dependency_baseline_report: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_manifest: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_binding_plan: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_binding_surface: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_scaffold: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_abi_seed: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_abi: Option<Utf8PathBuf>,
    pub(super) vst3_sdk_interface_skeleton: Option<Utf8PathBuf>,
    pub(super) signed_bundle_evidence: Vec<Utf8PathBuf>,
    pub(super) notarization_log: Option<Utf8PathBuf>,
    pub(super) require_release_artifacts: bool,
}

pub(super) fn build_release_check_report(
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

pub(super) fn current_release_check_os_label() -> Option<&'static str> {
    match doctor_os_label() {
        "Linux" => Some("Linux"),
        "macOS" => Some("macOS"),
        "Windows" => Some("Windows"),
        _ => None,
    }
}

pub(super) fn release_check_complete(report: &ReleaseCheckReport) -> bool {
    report.status == "ok"
}

pub(super) fn build_release_action_plan(
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

pub(super) fn release_action_for_check(
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

pub(super) fn release_action_evidence_path(
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

pub(super) fn release_action_commands(
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

pub(super) fn example_validator_matrix_commands(release_evidence_dir: &str) -> Vec<String> {
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

pub(super) fn example_static_validate_matrix_commands(release_evidence_dir: &str) -> Vec<String> {
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

pub(super) fn write_release_action_plan(
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

pub(super) fn apply_release_evidence_dir(
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

pub(super) fn release_evidence_candidate_file(
    dir: &Utf8Path,
    relative: &str,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    release_evidence_candidate_path(dir, relative, ReleaseEvidenceCandidateKind::File)
}

pub(super) fn release_evidence_candidate_dir(
    dir: &Utf8Path,
    relative: &str,
) -> Result<Option<Utf8PathBuf>, Box<dyn std::error::Error>> {
    release_evidence_candidate_path(dir, relative, ReleaseEvidenceCandidateKind::Directory)
}

pub(super) fn require_existing_directory_no_symlink(
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

pub(super) fn existing_directory_no_symlink(
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

pub(super) fn reject_existing_path_symlink(
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

pub(super) fn require_existing_file_no_symlink(
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

pub(super) fn read_text_file_no_symlink(
    label: &str,
    path: &Utf8Path,
) -> Result<String, Box<dyn std::error::Error>> {
    require_existing_file_no_symlink(label, path)?;
    Ok(fs::read_to_string(path)?)
}

pub(super) fn require_existing_file_or_directory_no_symlink(
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
pub(super) enum ReleaseEvidenceCandidateKind {
    File,
    Directory,
}

pub(super) fn release_evidence_candidate_path(
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

pub(super) fn platform_smoke_dir_has_non_pending_or_invalid_reports(
    dir: &Utf8Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    match collect_platform_smoke_reports(dir) {
        Ok(reports) => Ok(reports
            .iter()
            .any(|(_, report)| !platform_smoke_report_is_pending_template(report))),
        Err(_) => Ok(!collect_json_files_recursive(dir)?.is_empty()),
    }
}

pub(super) fn apply_validate_reports_from_evidence_dir(
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

pub(super) fn json_file_matches_non_validate_release_schema(path: &Utf8Path) -> bool {
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

pub(super) fn json_value_looks_like_non_validate_release_artifact(
    path: &Utf8Path,
    text: &str,
) -> bool {
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

pub(super) fn validate_report_path_prefers_static(path: &Utf8Path) -> bool {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name().unwrap_or_default().to_ascii_lowercase();
    path_lower.contains("/package/")
        || path_lower.contains("/static-validate/")
        || file_lower.contains("static-validate")
}

pub(super) fn validate_report_path_prefers_release(path: &Utf8Path) -> bool {
    let path_lower = portable_report_path(path).to_ascii_lowercase();
    let file_lower = path.file_name().unwrap_or_default().to_ascii_lowercase();
    path_lower.contains("/validator/")
        || file_lower.contains(".validate.")
        || (file_lower.contains("validate") && !file_lower.contains("static-validate"))
}

pub(super) fn apply_signing_and_notarization_from_evidence_dir(
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

pub(super) fn push_existing_unique(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    if path.exists() && !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

pub(super) fn push_standard_signing_evidence(paths: &mut Vec<Utf8PathBuf>, path: Utf8PathBuf) {
    if !signing_evidence_is_pending_template(&path) {
        push_existing_unique(paths, path);
    }
}

pub(super) fn push_standard_release_validate_report(
    paths: &mut Vec<Utf8PathBuf>,
    path: Utf8PathBuf,
) {
    match read_validate_report(&path) {
        Ok(report) if validate_report_is_pending_template(&report) => {}
        Ok(_) | Err(_) => push_existing_unique(paths, path),
    }
}

pub(super) fn push_standard_static_validate_report(
    paths: &mut Vec<Utf8PathBuf>,
    path: Utf8PathBuf,
) {
    match read_validate_report(&path) {
        Ok(report) if validate_report_is_pending_template(&report) => {}
        Ok(_) | Err(_) => push_existing_unique(paths, path),
    }
}

pub(super) fn validate_report_is_pending_template(report: &ValidateReport) -> bool {
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

pub(super) fn signing_evidence_is_pending_template(path: &Utf8Path) -> bool {
    read_text_file_no_symlink("signing evidence path", path)
        .is_ok_and(|text| signing_evidence_is_pending_template_text(&text))
}

pub(super) fn signing_evidence_is_pending_template_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("signed=pending")
        && (lower.contains("codesign verify=pending") || lower.contains("signtool verify=pending"))
}

pub(super) fn notarization_evidence_is_pending_template(path: &Utf8Path) -> bool {
    read_text_file_no_symlink("notarization evidence", path)
        .is_ok_and(|text| notarization_evidence_is_pending_template_text(&text))
}

pub(super) fn notarization_evidence_is_pending_template_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("notarization=pending") && lower.contains("stapled=pending")
}

pub(super) fn read_ci_run_url_file(
    path: &Utf8Path,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
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
