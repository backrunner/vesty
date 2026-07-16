use super::*;

#[cfg(test)]
pub(super) fn collect_reaper_evidence(dir: &Utf8PathBuf) -> serde_json::Value {
    let profile = vesty_core::find_host_profile("reaper").expect("REAPER profile exists");
    collect_reaper_evidence_for_profile(profile, dir)
}

pub(super) fn collect_reaper_evidence_for_profile(
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
pub(super) fn collect_generic_daw_evidence(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
    let Some(profile) = vesty_core::find_host_profile(host) else {
        return missing_daw_row(host, dir);
    };
    collect_generic_daw_evidence_for_profile(profile, dir)
}

pub(super) fn collect_generic_daw_evidence_for_profile(
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

pub(super) fn missing_daw_row(host: &str, dir: &Utf8PathBuf) -> serde_json::Value {
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

pub(super) fn read_optional(path: Utf8PathBuf) -> Option<String> {
    read_text_file_no_symlink("DAW evidence marker", &path).ok()
}

pub(super) fn read_first_optional(dir: &Utf8PathBuf, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| read_optional(dir.join(name)))
}

pub(super) fn evidence_platform_for_profile(
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

pub(super) fn daw_platform_evidence_supported(
    profile: &vesty_core::HostProfile,
    value: &str,
) -> bool {
    required_daw_platform_marker(profile, Some(value.to_string())).is_ok()
}

pub(super) fn reaper_scan_cache() -> Option<Utf8PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        Utf8PathBuf::from(home)
            .join("Library/Application Support/REAPER/reaper-vstplugins_arm64.ini"),
    )
}

pub(super) fn render_file_from_log(text: &str) -> Option<Utf8PathBuf> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("render_file") {
            return None;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        (!value.is_empty()).then(|| Utf8PathBuf::from(value))
    })
}

pub(super) fn render_file_evidence_ok(text: &str, evidence_dir: &Utf8Path) -> bool {
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

pub(super) fn render_file_exists_and_nonempty(path: Utf8PathBuf) -> bool {
    require_existing_file_no_symlink("render file evidence", &path)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
}

pub(super) fn reaper_param_watch_moved(text: &str) -> bool {
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

pub(super) fn daw_marker_matches(text: &str, predicate: impl FnOnce(&str) -> bool) -> bool {
    daw_marker_positive(text) && predicate(text)
}

pub(super) fn daw_marker_matches_for_profile(
    profile: &vesty_core::HostProfile,
    text: &str,
    predicate: impl FnOnce(&str) -> bool,
) -> bool {
    daw_marker_positive(text) && daw_marker_host_scope_matches(profile, text) && predicate(text)
}

pub(super) fn daw_marker_positive(text: &str) -> bool {
    !daw_marker_has_missing_assignment(text) && !daw_marker_has_negative_evidence(text)
}

pub(super) fn daw_marker_host_scope_matches(profile: &vesty_core::HostProfile, text: &str) -> bool {
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

pub(super) fn bridge_trace_relayed_param_gesture(text: &str) -> bool {
    let legacy_trace = text.contains("ParamGesture { phase: Begin")
        && text.contains("ParamGesture { phase: Perform")
        && text.contains("ParamGesture { phase: End");
    let packet_trace = text.contains(r#""type":"param.begin""#)
        && text.contains(r#""type":"param.perform""#)
        && text.contains(r#""type":"param.end""#);
    (legacy_trace || packet_trace) && text.contains("result=0")
}

pub(super) fn generic_scan_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["scan", "scan_ok"])
        || [
            "VestyGain.vst3",
            "VestyWebUIDemo.vst3",
            "VestyMIDISynth.vst3",
        ]
        .iter()
        .all(|needle| text.contains(needle))
}

pub(super) fn generic_load_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["load", "load_ok"])
        || (vesty_plugin_names_present(text) && text.matches("ok=true").count() >= 3)
}

pub(super) fn generic_ui_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["ui", "ui_ok"])
        || (text.contains("Vesty Web UI Demo") && text.contains("ui_show_called=true"))
}

pub(super) fn generic_ui_host_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["ui_host_param", "ui_host", "host_param"])
        || reaper_param_watch_moved(text)
}

pub(super) fn generic_automation_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["automation", "automation_ok"])
        || (text.contains("automation_points=3")
            && text.contains("midi_note_inserted=true")
            && text.contains("project_ready=true"))
}

pub(super) fn generic_buffer_sample_rate_change_ok(text: &str) -> bool {
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

pub(super) fn generic_restore_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["save_restore", "restore", "restore_ok"])
        || (text.contains("track_count=3") && text.matches("ok=true").count() >= 3)
}

pub(super) fn generic_offline_render_ok(text: &str) -> bool {
    explicit_truthy_marker(text, &["offline_render", "render", "render_ok"])
}

pub(super) fn vesty_plugin_names_present(text: &str) -> bool {
    ["Vesty Gain", "Vesty Web UI Demo", "Vesty MIDI Synth"]
        .iter()
        .all(|needle| text.contains(needle))
}

pub(super) fn explicit_truthy_marker(text: &str, keys: &[&str]) -> bool {
    text.lines()
        .any(|line| explicit_truthy_marker_line(line, keys))
}

pub(super) fn explicit_truthy_marker_line(line: &str, keys: &[&str]) -> bool {
    explicit_marker_line_matches(line, keys, &["true", "pass", "ok"])
}

pub(super) fn explicit_falsy_marker_line(line: &str, keys: &[&str]) -> bool {
    explicit_marker_line_matches(
        line,
        keys,
        &[
            "false", "fail", "failed", "error", "invalid", "rejected", "pending",
        ],
    )
}

pub(super) fn explicit_marker_line_matches(line: &str, keys: &[&str], values: &[&str]) -> bool {
    line.split(';')
        .any(|fragment| explicit_marker_fragment_matches(fragment, keys, values))
}

pub(super) fn explicit_marker_fragment_matches(
    fragment: &str,
    keys: &[&str],
    values: &[&str],
) -> bool {
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

pub(super) fn line_contains_any(line: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| line.contains(needle))
}

pub(super) fn split_marker_assignment(line: &str) -> Option<(&str, &str)> {
    if let Some((key, value)) = line.split_once('=') {
        Some((key, value))
    } else {
        line.split_once(':')
    }
}

pub(super) fn marker_assignments(line: &str) -> impl Iterator<Item = (&str, &str)> {
    line.split(';').filter_map(split_marker_assignment)
}

pub(super) fn meter_stream_delivered(text: &str) -> bool {
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

pub(super) fn line_has_nonzero_peak(line: &str) -> bool {
    line.split_once("peak=")
        .and_then(|(_, rest)| {
            rest.split(|char: char| !(char.is_ascii_digit() || matches!(char, '.' | '-' | '+')))
                .next()
        })
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value > 0.0)
}
