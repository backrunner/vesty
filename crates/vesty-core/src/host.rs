use serde::Serialize;

pub const RELEASE_SMOKE_CHECKS: &[&str] = &[
    "scan",
    "load",
    "ui",
    "ui_host_param",
    "meter_stream",
    "automation",
    "buffer_sample_rate_change",
    "save_restore",
    "offline_render",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostQuirkSeverity {
    Info,
    Warning,
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostQuirkArea {
    Scanning,
    Editor,
    Automation,
    State,
    Render,
    Meter,
    Platform,
    Packaging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HostQuirk {
    pub area: HostQuirkArea,
    pub severity: HostQuirkSeverity,
    pub summary: &'static str,
    pub mitigation: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HostProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub platforms: &'static [&'static str],
    pub notes: &'static [&'static str],
    pub quirks: &'static [HostQuirk],
    pub required_smoke_checks: &'static [&'static str],
}

const REAPER_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Scanning,
        severity: HostQuirkSeverity::Info,
        summary: "REAPER keeps plugin scan/cache state per installation and architecture.",
        mitigation: "Collect scan evidence from the target architecture cache or force a rescan before release smoke.",
    },
    HostQuirk {
        area: HostQuirkArea::Editor,
        severity: HostQuirkSeverity::Required,
        summary: "Editor open/close and parameter relay must be proven with host-side evidence.",
        mitigation: "Use UI trace plus host parameter watch evidence for `ui` and `ui_host_param` checks.",
    },
];

const CUBASE_NUENDO_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Packaging,
        severity: HostQuirkSeverity::Required,
        summary: "Treat Steinberg hosts as the strict reference path for VST3 metadata and validator behavior.",
        mitigation: "Run Steinberg validator and collect Cubase/Nuendo scan/load/UI/automation/buffer-size/sample-rate/save/offline render evidence.",
    },
    HostQuirk {
        area: HostQuirkArea::Automation,
        severity: HostQuirkSeverity::Warning,
        summary: "Latency or IO affecting parameter edits must notify the host with the correct restart flags.",
        mitigation: "Keep `HostChangeFlags` tests and collect host automation evidence for latency-affecting parameters.",
    },
];

const BITWIG_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Platform,
        severity: HostQuirkSeverity::Warning,
        summary: "Linux UI support is scoped to X11 for the MVP; Wayland remains experimental.",
        mitigation: "Collect Bitwig Linux smoke on X11 with WebKitGTK installed and mark Wayland separately.",
    },
    HostQuirk {
        area: HostQuirkArea::Meter,
        severity: HostQuirkSeverity::Required,
        summary: "Meter/analyzer streams are latest-wins and must not block audio processing.",
        mitigation: "Collect `meter_stream` evidence while the UI is open and automation/offline render evidence separately.",
    },
];

const ABLETON_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Editor,
        severity: HostQuirkSeverity::Required,
        summary: "Floating editor lifecycle and WebView attach/detach behavior need host smoke evidence.",
        mitigation: "Open/close the editor repeatedly and record UI plus UI-to-host parameter relay logs.",
    },
    HostQuirk {
        area: HostQuirkArea::Render,
        severity: HostQuirkSeverity::Required,
        summary: "Offline render must be validated independently from realtime playback.",
        mitigation: "Collect an `offline_render` marker from a real Ableton Live render pass.",
    },
];

const STUDIO_ONE_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::State,
        severity: HostQuirkSeverity::Required,
        summary: "Project save/restore must preserve params, custom state and UI config revision.",
        mitigation: "Collect `save_restore` evidence after closing and reopening a project with all examples loaded.",
    },
    HostQuirk {
        area: HostQuirkArea::Automation,
        severity: HostQuirkSeverity::Required,
        summary: "Begin/perform/end edit ordering must be verified from the host automation path.",
        mitigation: "Record parameter automation and confirm host-side parameter movement evidence.",
    },
];

const HOST_PROFILES: &[HostProfile] = &[
    HostProfile {
        id: "reaper",
        name: "REAPER",
        aliases: &["reaper"],
        platforms: &["macos", "windows", "linux"],
        notes: &["Current local evidence covers REAPER on macOS arm64 only."],
        quirks: REAPER_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "cubase-nuendo",
        name: "Cubase/Nuendo",
        aliases: &["cubase", "nuendo", "steinberg"],
        platforms: &["macos", "windows"],
        notes: &["Required release host; evidence is currently external/manual."],
        quirks: CUBASE_NUENDO_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "bitwig",
        name: "Bitwig Studio",
        aliases: &["bitwig", "bitwig-studio"],
        platforms: &["macos", "windows", "linux-x11"],
        notes: &["Wayland support is experimental until a separate smoke path exists."],
        quirks: BITWIG_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "ableton-live",
        name: "Ableton Live",
        aliases: &["ableton", "live", "ableton-live"],
        platforms: &["macos", "windows"],
        notes: &["Evidence is currently external/manual."],
        quirks: ABLETON_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "studio-one",
        name: "Studio One",
        aliases: &["studio-one", "studio one", "presonus"],
        platforms: &["macos", "windows"],
        notes: &["Evidence is currently external/manual."],
        quirks: STUDIO_ONE_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
];

pub fn host_profiles() -> &'static [HostProfile] {
    HOST_PROFILES
}

pub fn find_host_profile(query: &str) -> Option<&'static HostProfile> {
    let query = normalize_host_query(query);
    HOST_PROFILES.iter().find(|profile| {
        normalize_host_query(profile.id) == query
            || normalize_host_query(profile.name) == query
            || profile
                .aliases
                .iter()
                .any(|alias| normalize_host_query(alias) == query)
    })
}

fn normalize_host_query(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}
