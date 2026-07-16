use super::*;

pub(super) fn create_project(
    name: &str,
    kind: Option<&str>,
    ui: Option<&str>,
    template: Option<&str>,
    vesty_path: Option<&Utf8Path>,
    plugin_ui_path: Option<&Utf8Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = Utf8PathBuf::from(name);
    if path_exists_no_follow(&root)? {
        return Err(format!("project path already exists: {root}").into());
    }
    reject_existing_output_parent_symlink("project path", &root)?;
    let project_name = project_name_from_path(&root, name);
    let project_template = template.map(resolve_project_template).transpose()?;
    let kind = canonical_project_kind(
        kind.or_else(|| project_template.map(|template| template.kind))
            .unwrap_or("effect"),
    )?;
    let ui_template = parse_ui_template(
        ui.or_else(|| project_template.map(|template| template.ui))
            .unwrap_or("react"),
    )?;

    let crate_name = sanitize_crate_name(&project_name);
    let plugin_type = pascal_ident(&project_name);
    let params_type = format!("{plugin_type}Params");
    let kernel_type = format!("{plugin_type}Kernel");
    let class_id = class_id_from_name(&project_name);
    let class_id_toml = uuid_string(class_id);
    let class_id_rust = rust_byte_array(class_id);
    let is_instrument = kind == "instrument";
    let package_category = package_category_for_kind(kind);
    let bundle_id = format!("dev.vesty.{crate_name}");
    let with_ui = ui_template != UiTemplate::None;
    let ui_param = UiParamTemplate::for_kind(kind);
    let param_specs_json = default_param_specs_json(kind);
    let parameter_manifest = parameter_manifest_from_specs_json(param_specs_json)?;
    let local_vesty_path = vesty_path
        .map(|path| {
            Utf8PathBuf::from_path_buf(path.canonicalize()?).map_err(
                |_| -> Box<dyn std::error::Error> { "vesty path is not valid utf-8".into() },
            )
        })
        .transpose()?;
    let local_plugin_ui_path = plugin_ui_path
        .map(|path| {
            Utf8PathBuf::from_path_buf(path.canonicalize()?).map_err(
                |_| -> Box<dyn std::error::Error> {
                    "vesty-plugin-ui path is not valid utf-8".into()
                },
            )
        })
        .transpose()?;
    let ui_package_paths = UiPackagePaths::from_plugin_ui_path(local_plugin_ui_path.as_deref());

    fs::create_dir_all(root.join("src"))?;
    if with_ui {
        fs::create_dir_all(root.join("ui/src"))?;
    }
    require_existing_directory_no_symlink("project path", &root)?;
    let ui_config = if with_ui {
        r#"
[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420
"#
    } else {
        ""
    };
    fs::write(
        root.join("vesty.toml"),
        vesty_toml(
            &project_name,
            kind,
            &class_id_toml,
            ui_config,
            &bundle_id,
            package_category,
        ),
    )?;
    fs::write(
        root.join("Cargo.toml"),
        cargo_toml(&project_name, &crate_name, local_vesty_path.as_deref()),
    )?;
    fs::write(
        root.join("README.md"),
        project_readme(&project_name, kind, ui_template),
    )?;
    write_text_file(&root.join("params.specs.json"), param_specs_json)?;
    write_parameter_manifest(&root.join("vesty-parameters.json"), &parameter_manifest)?;
    let source = if is_instrument {
        instrument_template(
            &project_name,
            &plugin_type,
            &params_type,
            &kernel_type,
            &class_id_rust,
            with_ui,
        )
    } else {
        effect_template(
            &project_name,
            &plugin_type,
            &params_type,
            &kernel_type,
            &class_id_rust,
            with_ui,
        )
    };
    fs::write(root.join("src/lib.rs"), source)?;
    if with_ui {
        write_ui_template(
            &root,
            &project_name,
            ui_template,
            &ui_package_paths,
            ui_param,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectTemplate {
    pub(super) id: &'static str,
    pub(super) aliases: &'static [&'static str],
    pub(super) title: &'static str,
    pub(super) kind: &'static str,
    pub(super) ui: &'static str,
    pub(super) description: &'static str,
}

pub(super) const PROJECT_TEMPLATES: &[ProjectTemplate] = &[
    ProjectTemplate {
        id: "gain",
        aliases: &["headless-gain", "effect"],
        title: "Headless Gain Effect",
        kind: "effect",
        ui: "none",
        description: "Minimal audio effect with a sample-accurate gain parameter and no Web UI.",
    },
    ProjectTemplate {
        id: "web-ui-param-demo",
        aliases: &["default", "react-effect"],
        title: "React Web UI Parameter Demo",
        kind: "effect",
        ui: "react",
        description: "Audio effect plus React/Vite Web UI wired to vesty-plugin-ui parameter gestures.",
    },
    ProjectTemplate {
        id: "vanilla-ui-param-demo",
        aliases: &["vanilla-effect"],
        title: "Vanilla Web UI Parameter Demo",
        kind: "effect",
        ui: "vanilla",
        description: "Audio effect plus framework-free TypeScript UI using the core JSBridge SDK.",
    },
    ProjectTemplate {
        id: "vue-ui-param-demo",
        aliases: &["vue-effect"],
        title: "Vue Web UI Parameter Demo",
        kind: "effect",
        ui: "vue",
        description: "Audio effect plus Vue/Vite UI using the vesty-plugin-ui Vue adapter.",
    },
    ProjectTemplate {
        id: "svelte-ui-param-demo",
        aliases: &["svelte-effect"],
        title: "Svelte Web UI Parameter Demo",
        kind: "effect",
        ui: "svelte",
        description: "Audio effect plus Svelte/Vite UI using the vesty-plugin-ui Svelte adapter.",
    },
    ProjectTemplate {
        id: "midi-synth",
        aliases: &["synth", "headless-instrument", "instrument"],
        title: "Headless MIDI Synth",
        kind: "instrument",
        ui: "none",
        description: "Minimal instrument with MIDI note input, sample-accurate volume automation and stereo output.",
    },
    ProjectTemplate {
        id: "web-ui-instrument",
        aliases: &["react-instrument", "instrument-ui"],
        title: "React Web UI Instrument",
        kind: "instrument",
        ui: "react",
        description: "Instrument starter with React/Vite Web UI and the same JSBridge parameter gesture flow.",
    },
];

pub(super) fn resolve_project_template(
    id: &str,
) -> Result<ProjectTemplate, Box<dyn std::error::Error>> {
    let normalized = id.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("project template id must not be empty".into());
    }
    PROJECT_TEMPLATES
        .iter()
        .copied()
        .find(|template| {
            template.id == normalized || template.aliases.contains(&normalized.as_str())
        })
        .ok_or_else(|| {
            format!(
                "unsupported project template '{id}'; run `vesty templates` to list built-in starters"
            )
            .into()
        })
}

pub(super) fn print_template_gallery(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match parse_output_format(format)? {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(PROJECT_TEMPLATES)?);
        }
        OutputFormat::Text => {
            println!("Available Vesty project templates:");
            for template in PROJECT_TEMPLATES {
                let aliases = if template.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" aliases: {}", template.aliases.join(", "))
                };
                println!(
                    "- {}: {} (kind={}, ui={}){}\n  {}",
                    template.id,
                    template.title,
                    template.kind,
                    template.ui,
                    aliases,
                    template.description
                );
            }
            println!(
                "\nUse `vesty new <name> --template <id>`. Explicit `--kind` or `--ui` overrides the template defaults."
            );
        }
    }
    Ok(())
}

pub(super) fn vesty_toml(
    name: &str,
    kind: &str,
    class_id: &str,
    ui_config: &str,
    bundle_id: &str,
    package_category: &str,
) -> String {
    format!(
        r#"[plugin]
name = "{}"
vendor = "Example"
version = "0.1.0"
kind = "{kind}"
class_id = "{class_id}"
{ui_config}
[package]
bundle_id = "{bundle_id}"
category = "{package_category}"
parameter_manifest = "vesty-parameters.json"
"#,
        toml_escape(name),
    )
}

pub(super) fn default_param_specs_json(kind: &str) -> &'static str {
    if kind == "instrument" {
        r#"{
  "version": 1,
  "parameters": [
    {
      "id": "volume",
      "name": "Volume",
      "kind": { "float": { "min": 0.0, "max": 1.0 } },
      "defaultNormalized": 0.2,
      "unit": null,
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
"#
    } else {
        r#"{
  "version": 1,
  "parameters": [
    {
      "id": "gain",
      "name": "Gain",
      "kind": { "float": { "min": 0.0, "max": 2.0 } },
      "defaultNormalized": 0.5,
      "unit": "x",
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
"#
    }
}

pub(super) fn cargo_toml(
    plugin_name: &str,
    crate_name: &str,
    vesty_path: Option<&Utf8Path>,
) -> String {
    let vesty_dependency = match vesty_path {
        Some(path) => format!(r#"vesty = {{ path = "{}" }}"#, toml_escape(path.as_str())),
        None => format!(r#"vesty = "={}""#, env!("CARGO_PKG_VERSION")),
    };
    format!(
        r#"[package]
name = "__CRATE_NAME__"
version = "0.1.0"
edition = "2024"
description = "__DESCRIPTION__"
publish = false

[workspace]

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
{vesty_dependency}
"#
    )
    .replace("__CRATE_NAME__", crate_name)
    .replace(
        "__DESCRIPTION__",
        &toml_escape(&format!("{plugin_name} VST3 plugin")),
    )
}

pub(super) fn project_readme(plugin_name: &str, kind: &str, ui_template: UiTemplate) -> String {
    let kind_label = if kind == "instrument" {
        "instrument"
    } else {
        "audio effect"
    };
    let ui_section = if ui_template == UiTemplate::None {
        "This project is headless and does not include a Web UI.\n".to_string()
    } else {
        format!(
            r#"This project includes a `{}` Web UI in `ui/`.

```bash
cd ui
npm install
npm run build
```
"#,
            ui_template.label()
        )
    };

    format!(
        r#"# {plugin_name}

Generated by `vesty new` as a VST3 {kind_label}.

## Layout

- `src/lib.rs`: Rust DSP/plugin implementation.
- `vesty.toml`: plugin, UI and package metadata.
- `params.specs.json`: editable parameter schema used to generate the package sidecar.
- `vesty-parameters.json`: generated stable VST3 ParamID manifest referenced by `vesty.toml`.
- `Cargo.toml`: Rust crate manifest; `publish = false` because VST plugins are distributed as `.vst3` bundles.

{ui_section}
## Common Commands

```bash
cargo check
vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check
vesty build --config vesty.toml
vesty package --config vesty.toml --platform macos --binary target/release/lib{crate_name}.dylib
vesty validate target/vesty/{bundle_name}.vst3 --static-only
```

Use `vesty doctor` to check local VST3 validator, WebView and signing/notarization prerequisites.
"#,
        plugin_name = plugin_name,
        kind_label = kind_label,
        ui_section = ui_section,
        crate_name = sanitize_crate_name(plugin_name).replace('-', "_"),
        bundle_name = sanitize_bundle_name_for_readme(plugin_name),
    )
}

pub(super) fn package_category_for_kind(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("instrument") {
        "Instrument"
    } else {
        "Fx"
    }
}

pub(super) fn canonical_project_kind(
    kind: &str,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "effect" | "fx" | "audio-effect" | "audio_effect" => Ok("effect"),
        "instrument" => Ok("instrument"),
        _ => Err(format!(
            "--kind must be one of effect, fx, audio-effect, audio_effect or instrument: {kind}"
        )
        .into()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiTemplate {
    None,
    Vanilla,
    React,
    Vue,
    Svelte,
}

impl UiTemplate {
    fn label(self) -> &'static str {
        match self {
            UiTemplate::None => "none",
            UiTemplate::Vanilla => "vanilla",
            UiTemplate::React => "react",
            UiTemplate::Vue => "vue",
            UiTemplate::Svelte => "svelte",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct UiPackagePaths {
    pub(super) plugin_ui: Option<Utf8PathBuf>,
}

impl UiPackagePaths {
    fn from_plugin_ui_path(plugin_ui_path: Option<&Utf8Path>) -> Self {
        Self {
            plugin_ui: plugin_ui_path.map(Utf8Path::to_path_buf),
        }
    }
}

pub(super) fn parse_ui_template(ui: &str) -> Result<UiTemplate, Box<dyn std::error::Error>> {
    match ui.trim().to_ascii_lowercase().as_str() {
        "none" | "no-ui" | "false" => Ok(UiTemplate::None),
        "vanilla" | "html" | "ts" | "typescript" => Ok(UiTemplate::Vanilla),
        "react" => Ok(UiTemplate::React),
        "vue" => Ok(UiTemplate::Vue),
        "svelte" => Ok(UiTemplate::Svelte),
        _ => Err(format!(
            "unsupported ui template '{ui}'; expected none, vanilla, react, vue or svelte"
        )
        .into()),
    }
}

pub(super) fn write_ui_template(
    root: &Utf8PathBuf,
    name: &str,
    template: UiTemplate,
    package_paths: &UiPackagePaths,
    param: UiParamTemplate,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        root.join("ui/package.json"),
        ui_package_json(name, template, package_paths),
    )?;
    fs::write(root.join("ui/tsconfig.json"), ui_tsconfig(template))?;
    fs::write(
        root.join("ui/index.html"),
        ui_index_html(name, template, param),
    )?;
    match template {
        UiTemplate::None => {}
        UiTemplate::Vanilla => {
            fs::write(root.join("ui/src/index.ts"), ui_index_ts(param))?;
        }
        UiTemplate::React => {
            fs::write(root.join("ui/vite.config.ts"), ui_vite_config(template))?;
            fs::write(root.join("ui/src/main.tsx"), ui_react_main_tsx())?;
            fs::write(root.join("ui/src/App.tsx"), ui_react_app_tsx(param))?;
        }
        UiTemplate::Vue => {
            fs::write(root.join("ui/vite.config.ts"), ui_vite_config(template))?;
            fs::write(root.join("ui/src/main.ts"), ui_vue_main_ts())?;
            fs::write(root.join("ui/src/App.vue"), ui_vue_app(param))?;
        }
        UiTemplate::Svelte => {
            fs::write(root.join("ui/vite.config.ts"), ui_vite_config(template))?;
            fs::write(root.join("ui/src/main.ts"), ui_svelte_main_ts())?;
            fs::write(root.join("ui/src/App.svelte"), ui_svelte_app(param))?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct UiParamTemplate {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) default_normalized: f32,
}

impl UiParamTemplate {
    pub(super) fn for_kind(kind: &str) -> Self {
        if kind == "instrument" {
            Self {
                id: "volume",
                label: "Volume",
                default_normalized: 0.2,
            }
        } else {
            Self {
                id: "gain",
                label: "Gain",
                default_normalized: 0.5,
            }
        }
    }
}

pub(super) fn project_name_from_path(root: &Utf8Path, fallback: &str) -> String {
    root.file_name()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fallback)
        .trim()
        .to_string()
}

pub(super) fn sanitize_crate_name(name: &str) -> String {
    let mut output = String::new();
    for char in name.chars() {
        if char.is_ascii_alphanumeric() {
            output.push(char.to_ascii_lowercase());
        } else if !output.ends_with('-') {
            output.push('-');
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "vesty-plugin".to_string()
    } else {
        output
    }
}

pub(super) fn sanitize_bundle_name_for_readme(name: &str) -> String {
    let sanitized = name
        .chars()
        .filter(|char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-')
        .collect::<String>();
    if sanitized.is_empty() {
        "VestyPlugin".to_string()
    } else {
        sanitized
    }
}

pub(super) fn pascal_ident(name: &str) -> String {
    let mut output = String::new();
    let mut upper_next = true;
    for char in name.chars() {
        if char.is_ascii_alphanumeric() {
            if upper_next {
                output.push(char.to_ascii_uppercase());
                upper_next = false;
            } else {
                output.push(char);
            }
        } else {
            upper_next = true;
        }
    }
    if output.is_empty() || output.starts_with(|char: char| char.is_ascii_digit()) {
        output.insert_str(0, "Vesty");
    }
    output
}

pub(super) fn class_id_from_name(name: &str) -> [u8; 16] {
    let mut bytes = *b"vesty-plugin-id!";
    for (index, byte) in name.bytes().enumerate() {
        let slot = index % bytes.len();
        bytes[slot] = bytes[slot]
            .wrapping_add(byte)
            .rotate_left((index % 8) as u32);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

pub(super) fn uuid_string(bytes: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

pub(super) fn rust_byte_array(bytes: [u8; 16]) -> String {
    let values = bytes
        .iter()
        .map(|byte| format!("0x{byte:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

pub(super) fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn json_string_literal(value: &str) -> String {
    serde_json::Value::String(value.to_string()).to_string()
}

pub(super) fn rust_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for character in value.chars() {
        literal.extend(character.escape_default());
    }
    literal.push('"');
    literal
}

pub(super) fn effect_template(
    plugin_name: &str,
    plugin_type: &str,
    params_type: &str,
    kernel_type: &str,
    class_id: &str,
    with_ui: bool,
) -> String {
    let ui_method = ui_method_template(with_ui);
    format!(
        r#"use vesty::prelude::*;

pub struct {plugin_type} {{
    params: {params_type},
}}

impl Default for {plugin_type} {{
    fn default() -> Self {{
        Self {{
            params: {params_type}::default(),
        }}
    }}
}}

#[derive(Params)]
pub struct {params_type} {{
    gain: FloatParam,
}}

impl Default for {params_type} {{
    fn default() -> Self {{
        Self {{
            gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0).with_unit("x"),
        }}
    }}
}}

pub struct {kernel_type} {{
    gain: ParamHandle,
}}

impl AudioKernel for {kernel_type} {{
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {{
        let initial_gain = context.param_normalized(self.gain).unwrap_or(0.5);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial_gain, frames) {{
            let gain = segment.normalized as f32 * 2.0;
            for channel in 0..channels {{
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    gain,
                );
            }}
        }}
        ProcessResult::Continue
    }}
}}

impl Plugin for {plugin_type} {{
    const INFO: PluginInfo = PluginInfo {{
        name: {plugin_name:?},
        vendor: "Example",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: {class_id},
        kind: PluginKind::AudioEffect,
    }};

    type Params = {params_type};
    type Kernel = {kernel_type};

    fn params(&self) -> &Self::Params {{
        &self.params
    }}

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {{
        {kernel_type} {{
            gain: self.params.resolve_or_invalid("gain"),
        }}
    }}
{ui_method}
}}

vesty::export_vst3!({plugin_type});
"#
    )
}

pub(super) fn instrument_template(
    plugin_name: &str,
    plugin_type: &str,
    params_type: &str,
    kernel_type: &str,
    class_id: &str,
    with_ui: bool,
) -> String {
    let ui_method = ui_method_template(with_ui);
    format!(
        r#"use vesty::prelude::*;

pub struct {plugin_type} {{
    params: {params_type},
}}

impl Default for {plugin_type} {{
    fn default() -> Self {{
        Self {{
            params: {params_type}::default(),
        }}
    }}
}}

#[derive(Params)]
pub struct {params_type} {{
    volume: FloatParam,
}}

impl Default for {params_type} {{
    fn default() -> Self {{
        Self {{
            volume: FloatParam::new("volume", "Volume", 0.0, 1.0, 0.2),
        }}
    }}
}}

pub struct {kernel_type} {{
    sample_rate: f32,
    phase: f32,
    active_notes: u32,
    volume: ParamHandle,
}}

impl AudioKernel for {kernel_type} {{
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {{
        for event in context.events() {{
            match event {{
                Event::NoteOn {{ velocity, .. }} if *velocity > 0.0 => {{
                    self.active_notes = self.active_notes.saturating_add(1);
                }}
                Event::NoteOff {{ .. }} => {{
                    self.active_notes = self.active_notes.saturating_sub(1);
                }}
                _ => {{}}
            }}
        }}

        let initial_volume = context.param_normalized(self.volume).unwrap_or(0.2);
        context.audio_mut().clear_outputs();
        if self.active_notes == 0 {{
            return ProcessResult::Continue;
        }}

        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let outputs = context.audio().output_channels();
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.volume, initial_volume, frames) {{
            let volume = segment.normalized as f32;
            for frame in segment.start_sample as usize..segment.end_sample as usize {{
                let sample = (self.phase * std::f32::consts::TAU).sin() * volume;
                self.phase = (self.phase + 220.0 / self.sample_rate).fract();
                for channel in 0..outputs {{
                    audio.set_output_sample(channel, frame, sample);
                }}
            }}
        }}
        ProcessResult::Continue
    }}
}}

impl Plugin for {plugin_type} {{
    const INFO: PluginInfo = PluginInfo {{
        name: {plugin_name:?},
        vendor: "Example",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: {class_id},
        kind: PluginKind::Instrument,
    }};

    type Params = {params_type};
    type Kernel = {kernel_type};

    fn params(&self) -> &Self::Params {{
        &self.params
    }}

    fn create_kernel(&self, init: KernelInit) -> Self::Kernel {{
        {kernel_type} {{
            sample_rate: init.sample_rate as f32,
            phase: 0.0,
            active_notes: 0,
            volume: self.params.resolve_or_invalid("volume"),
        }}
    }}
{ui_method}
}}

vesty::export_vst3!({plugin_type});
"#
    )
}

pub(super) fn ui_method_template(with_ui: bool) -> &'static str {
    if with_ui {
        r#"
    fn ui(&self) -> Option<UiDescriptor> {
        Some(
            UiDescriptor::web_assets("ui")
                .with_dev_url("http://localhost:5173")
                .with_size(900, 560)
                .with_min_size(640, 420)
                .with_resizable(true),
        )
    }
"#
    } else {
        ""
    }
}

pub(super) fn ui_package_json(
    name: &str,
    template: UiTemplate,
    package_paths: &UiPackagePaths,
) -> String {
    let ui_package_name = format!("{}-editor", sanitize_crate_name(name));
    let plugin_ui_dependency = package_paths
        .plugin_ui
        .as_deref()
        .map(|path| format!("file:{}", path.as_str()))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    let typecheck_script = match template {
        UiTemplate::Vue => "vue-tsc --noEmit",
        UiTemplate::Svelte => "svelte-check --tsconfig ./tsconfig.json",
        UiTemplate::None | UiTemplate::Vanilla | UiTemplate::React => "tsc --noEmit",
    };
    let typescript_dependency = match template {
        // Current Vue and Svelte checkers rely on TypeScript compiler internals changed in 7.x.
        UiTemplate::Vue | UiTemplate::Svelte => "6.0.3",
        UiTemplate::None | UiTemplate::Vanilla | UiTemplate::React => "latest",
    };
    let framework_dependencies = match template {
        UiTemplate::None | UiTemplate::Vanilla => String::new(),
        UiTemplate::React => String::from(
            r#",
    "react": "latest",
    "react-dom": "latest""#,
        ),
        UiTemplate::Vue => String::from(
            r#",
    "vue": "latest""#,
        ),
        UiTemplate::Svelte => String::from(
            r#",
    "svelte": "latest""#,
        ),
    };
    let framework_dev_dependencies = match template {
        UiTemplate::None | UiTemplate::Vanilla => "",
        UiTemplate::React => {
            r#",
    "@types/react": "latest",
    "@types/react-dom": "latest",
    "@vitejs/plugin-react": "latest""#
        }
        UiTemplate::Vue => {
            r#",
    "@vitejs/plugin-vue": "latest",
    "vue-tsc": "3.3.7""#
        }
        UiTemplate::Svelte => {
            r#",
    "@sveltejs/vite-plugin-svelte": "latest",
    "svelte-check": "4.7.2""#
        }
    };
    r#"{
  "name": "__PACKAGE_NAME__",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1",
    "build": "vite build",
    "typecheck": "__TYPECHECK_SCRIPT__"
  },
  "dependencies": {
    "vesty-plugin-ui": __PLUGIN_UI_DEPENDENCY____FRAMEWORK_DEPENDENCIES__
  },
  "devDependencies": {
    "typescript": "__TYPESCRIPT_DEPENDENCY__",
    "vite": "latest"__FRAMEWORK_DEV_DEPENDENCIES__
  }
}
"#
    .replace("__PACKAGE_NAME__", &ui_package_name)
    .replace("__TYPECHECK_SCRIPT__", typecheck_script)
    .replace("__TYPESCRIPT_DEPENDENCY__", typescript_dependency)
    .replace(
        "__PLUGIN_UI_DEPENDENCY__",
        &json_string_literal(&plugin_ui_dependency),
    )
    .replace("__FRAMEWORK_DEPENDENCIES__", &framework_dependencies)
    .replace("__FRAMEWORK_DEV_DEPENDENCIES__", framework_dev_dependencies)
}

pub(super) fn ui_tsconfig(template: UiTemplate) -> String {
    let jsx = if template == UiTemplate::React {
        r#",
    "jsx": "react-jsx""#
    } else {
        ""
    };
    let include = match template {
        UiTemplate::React => r#""src/**/*.ts", "src/**/*.tsx""#,
        UiTemplate::Vue => r#""src/**/*.ts", "src/**/*.vue""#,
        UiTemplate::Svelte => r#""src/**/*.ts", "src/**/*.svelte""#,
        UiTemplate::None | UiTemplate::Vanilla => r#""src/**/*.ts""#,
    };
    r#"{
  "compilerOptions": {
    "module": "ES2022",
    "moduleResolution": "Bundler",
    "preserveSymlinks": true,
    "strict": true,
    "target": "ES2022"__JSX__
  },
  "include": [__INCLUDE__]
}
"#
    .replace("__JSX__", jsx)
    .replace("__INCLUDE__", include)
}

pub(super) fn ui_vite_config(template: UiTemplate) -> &'static str {
    match template {
        UiTemplate::React => {
            r#"import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    dedupe: ["react", "react-dom"],
    preserveSymlinks: true
  }
});
"#
        }
        UiTemplate::Vue => {
            r#"import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    dedupe: ["vue"],
    preserveSymlinks: true
  }
});
"#
        }
        UiTemplate::Svelte => {
            r#"import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    dedupe: ["svelte"],
    preserveSymlinks: true
  }
});
"#
        }
        UiTemplate::None | UiTemplate::Vanilla => "",
    }
}

pub(super) fn ui_index_html(name: &str, template: UiTemplate, param: UiParamTemplate) -> String {
    let body = match template {
        UiTemplate::React => {
            r#"<div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>"#
        }
        UiTemplate::Vue | UiTemplate::Svelte => {
            r#"<div id="app"></div>
    <script type="module" src="/src/main.ts"></script>"#
        }
        UiTemplate::None | UiTemplate::Vanilla => {
            r#"<main>
      <label>
        __PARAM_LABEL__
        <input id="__PARAM_ID__" type="range" min="0" max="1" step="0.001" />
      </label>
      <output id="value"></output>
    </main>
    <script type="module" src="/src/index.ts"></script>"#
        }
    };
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{}</title>
  </head>
  <body>
    {body}
  </body>
</html>
"#,
        toml_escape(name)
    )
    .replace("__PARAM_ID__", param.id)
    .replace("__PARAM_LABEL__", param.label)
}

pub(super) fn ui_index_ts(param: UiParamTemplate) -> String {
    r##"import { createBridge, type BridgeReadyPayload, type ParamChangedEvent } from "vesty-plugin-ui";

const bridge = createBridge();
const control = document.querySelector<HTMLInputElement>("#__PARAM_ID__");
const value = document.querySelector<HTMLOutputElement>("#value");
const PARAM_ID = "__PARAM_ID__";
const PARAM_DEFAULT = __PARAM_DEFAULT__;
let editing = false;

function clampNormalized(value: unknown, fallback = PARAM_DEFAULT) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : fallback;
}

function normalizedFromReady(ready: BridgeReadyPayload) {
  return clampNormalized(ready.paramValues.find((param) => param.id === PARAM_ID)?.normalized);
}

function setNormalized(normalized: number) {
  const next = clampNormalized(normalized);
  if (control) control.value = String(next);
  if (value) value.value = next.toFixed(3);
}

async function main() {
  const ready = await bridge.ready();
  console.log("Vesty ready", ready.pluginName);
  console.log("Vesty snapshot", ready.snapshot);
  setNormalized(normalizedFromReady(ready));
  const unsubscribe = bridge.subscribe<ParamChangedEvent>("param.changed", (event) => {
    if (event.id === PARAM_ID) setNormalized(event.normalized);
  });
  window.addEventListener("pagehide", unsubscribe, { once: true });
}

function begin(event: PointerEvent) {
  if (!control || editing) return;
  editing = true;
  control.setPointerCapture(event.pointerId);
  void bridge.beginParamEdit(PARAM_ID);
}

function perform() {
  if (!control) return;
  const normalized = clampNormalized(Number(control.value));
  setNormalized(normalized);
  void bridge.performParamEdit(PARAM_ID, normalized);
}

function end(event?: PointerEvent) {
  if (!control || !editing) return;
  editing = false;
  if (event && control.hasPointerCapture(event.pointerId)) {
    control.releasePointerCapture(event.pointerId);
  }
  void bridge.endParamEdit(PARAM_ID);
}

control?.addEventListener("pointerdown", begin);
control?.addEventListener("input", perform);
control?.addEventListener("pointerup", end);
control?.addEventListener("pointercancel", end);
control?.addEventListener("lostpointercapture", end);

void main();
"##
    .replace("__PARAM_ID__", param.id)
    .replace("__PARAM_DEFAULT__", &param.default_normalized.to_string())
}

pub(super) fn ui_react_main_tsx() -> &'static str {
    r#"import { createRoot } from "react-dom/client";
import { App } from "./App";

createRoot(document.getElementById("root")!).render(<App />);
"#
}

pub(super) fn ui_react_app_tsx(param: UiParamTemplate) -> String {
    r#"import { useEffect, useMemo, useRef, useState } from "react";
import { createBridge, type BridgeReadyPayload, type ParamChangedEvent } from "vesty-plugin-ui";
import { VestyBridgeProvider, useVestyBridge, useVestyParamEdit } from "vesty-plugin-ui/react";

const PARAM_ID = "__PARAM_ID__";
const PARAM_DEFAULT = __PARAM_DEFAULT__;

function clampNormalized(value: unknown, fallback = PARAM_DEFAULT) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : fallback;
}

function normalizedFromReady(ready: BridgeReadyPayload) {
  return clampNormalized(ready.paramValues.find((param) => param.id === PARAM_ID)?.normalized);
}

export function App() {
  const bridge = useMemo(() => createBridge(), []);
  const [name, setName] = useState("Vesty");

  useEffect(() => {
    let mounted = true;
    void bridge.ready().then((ready) => {
      if (mounted && ready && typeof ready === "object" && "pluginName" in ready) {
        setName(String((ready as { pluginName: unknown }).pluginName));
      }
    });
    return () => {
      mounted = false;
    };
  }, [bridge]);

  return (
    <VestyBridgeProvider bridge={bridge}>
      <PluginControls name={name} />
    </VestyBridgeProvider>
  );
}

function PluginControls({ name }: { name: string }) {
  const bridge = useVestyBridge();
  const param = useVestyParamEdit(PARAM_ID);
  const [normalized, setNormalized] = useState(PARAM_DEFAULT);
  const editing = useRef(false);

  useEffect(() => {
    let mounted = true;
    void bridge.ready().then((ready) => {
      if (mounted) setNormalized(normalizedFromReady(ready));
    });
    const unsubscribe = bridge.subscribe<ParamChangedEvent>("param.changed", (event) => {
      if (event.id === PARAM_ID) setNormalized(clampNormalized(event.normalized));
    });
    return () => {
      mounted = false;
      unsubscribe();
    };
  }, [bridge]);

  function begin(event: React.PointerEvent<HTMLInputElement>) {
    if (editing.current) return;
    editing.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
    void param.begin();
  }

  function end(event?: React.PointerEvent<HTMLInputElement>) {
    if (!editing.current) return;
    editing.current = false;
    if (event && event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    void param.end();
  }

  return (
    <main>
      <h1>{name}</h1>
      <label>
        __PARAM_LABEL__
        <input
          type="range"
          min="0"
          max="1"
          step="0.001"
          value={normalized}
          onPointerDown={begin}
          onChange={(event) => {
            const normalized = Number(event.currentTarget.value);
            setNormalized(normalized);
            void param.perform(normalized);
          }}
          onPointerUp={end}
          onPointerCancel={end}
          onLostPointerCapture={end}
        />
      </label>
      <output>{normalized.toFixed(3)}</output>
    </main>
  );
}
"#
    .replace("__PARAM_ID__", param.id)
    .replace("__PARAM_LABEL__", param.label)
    .replace("__PARAM_DEFAULT__", &param.default_normalized.to_string())
}

pub(super) fn ui_vue_main_ts() -> &'static str {
    r##"import { createApp } from "vue";
import App from "./App.vue";

createApp(App).mount("#app");
"##
}

pub(super) fn ui_vue_app(param: UiParamTemplate) -> String {
    r#"<script setup lang="ts">
import { onMounted, onScopeDispose, ref } from "vue";
import { createBridge, type BridgeReadyPayload, type ParamChangedEvent } from "vesty-plugin-ui";
import { useVestyParamEdit } from "vesty-plugin-ui/vue";

const bridge = createBridge();
const PARAM_ID = "__PARAM_ID__";
const PARAM_DEFAULT = __PARAM_DEFAULT__;
const param = useVestyParamEdit(PARAM_ID, bridge);
const normalized = ref(PARAM_DEFAULT);
const name = ref("Vesty");
let editing = false;
let unsubscribeParamChanged: (() => void) | undefined;

function clampNormalized(value: unknown, fallback = PARAM_DEFAULT) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : fallback;
}

function normalizedFromReady(ready: BridgeReadyPayload) {
  return clampNormalized(ready.paramValues.find((param) => param.id === PARAM_ID)?.normalized);
}

onMounted(async () => {
  const ready = await bridge.ready();
  if (ready && typeof ready === "object" && "pluginName" in ready) {
    name.value = String((ready as { pluginName: unknown }).pluginName);
  }
  normalized.value = normalizedFromReady(ready);
  unsubscribeParamChanged = bridge.subscribe<ParamChangedEvent>("param.changed", (event) => {
    if (event.id === PARAM_ID) normalized.value = clampNormalized(event.normalized);
  });
});

onScopeDispose(() => {
  unsubscribeParamChanged?.();
});

function perform() {
  void param.perform(normalized.value);
}

function begin(event: PointerEvent) {
  if (editing) return;
  editing = true;
  const input = event.currentTarget as HTMLInputElement;
  input.setPointerCapture(event.pointerId);
  void param.begin();
}

function end(event?: PointerEvent) {
  if (!editing) return;
  editing = false;
  const input = event?.currentTarget as HTMLInputElement | undefined;
  if (event && input?.hasPointerCapture(event.pointerId)) {
    input.releasePointerCapture(event.pointerId);
  }
  void param.end();
}
</script>

<template>
  <main>
    <h1>{{ name }}</h1>
    <label>
      __PARAM_LABEL__
      <input
        v-model.number="normalized"
        type="range"
        min="0"
        max="1"
        step="0.001"
        @pointerdown="begin"
        @input="perform"
        @pointerup="end"
        @pointercancel="end"
        @lostpointercapture="end"
      />
    </label>
    <output>{{ normalized.toFixed(3) }}</output>
  </main>
</template>
"#
    .replace("__PARAM_ID__", param.id)
    .replace("__PARAM_LABEL__", param.label)
    .replace("__PARAM_DEFAULT__", &param.default_normalized.to_string())
}

pub(super) fn ui_svelte_main_ts() -> &'static str {
    r##"import { mount } from "svelte";
import App from "./App.svelte";

mount(App, {
  target: document.getElementById("app")!
});
"##
}

pub(super) fn ui_svelte_app(param: UiParamTemplate) -> String {
    r#"<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { createBridge, type BridgeReadyPayload, type ParamChangedEvent } from "vesty-plugin-ui";
  import { vestyParamEdit } from "vesty-plugin-ui/svelte";

  const bridge = createBridge();
  const PARAM_ID = "__PARAM_ID__";
  const PARAM_DEFAULT = __PARAM_DEFAULT__;
  const param = vestyParamEdit(PARAM_ID, bridge);
  let normalized = PARAM_DEFAULT;
  let name = "Vesty";
  let editing = false;
  let unsubscribeParamChanged: (() => void) | undefined;

  function clampNormalized(value: unknown, fallback = PARAM_DEFAULT) {
    return typeof value === "number" && Number.isFinite(value)
      ? Math.min(1, Math.max(0, value))
      : fallback;
  }

  function normalizedFromReady(ready: BridgeReadyPayload) {
    return clampNormalized(ready.paramValues.find((param) => param.id === PARAM_ID)?.normalized);
  }

  onMount(async () => {
    const ready = await bridge.ready();
    if (ready && typeof ready === "object" && "pluginName" in ready) {
      name = String((ready as { pluginName: unknown }).pluginName);
    }
    normalized = normalizedFromReady(ready);
    unsubscribeParamChanged = bridge.subscribe<ParamChangedEvent>("param.changed", (event) => {
      if (event.id === PARAM_ID) normalized = clampNormalized(event.normalized);
    });
  });

  onDestroy(() => {
    unsubscribeParamChanged?.();
  });

  function perform() {
    void param.perform(normalized);
  }

  function begin(event: PointerEvent) {
    if (editing) return;
    editing = true;
    const input = event.currentTarget as HTMLInputElement;
    input.setPointerCapture(event.pointerId);
    void param.begin();
  }

  function end(event?: PointerEvent) {
    if (!editing) return;
    editing = false;
    const input = event?.currentTarget as HTMLInputElement | undefined;
    if (event && input?.hasPointerCapture(event.pointerId)) {
      input.releasePointerCapture(event.pointerId);
    }
    void param.end();
  }
</script>

<main>
  <h1>{name}</h1>
  <label>
    __PARAM_LABEL__
    <input
      bind:value={normalized}
      type="range"
      min="0"
      max="1"
      step="0.001"
      onpointerdown={begin}
      oninput={perform}
      onpointerup={end}
      onpointercancel={end}
      onlostpointercapture={end}
    />
  </label>
  <output>{normalized.toFixed(3)}</output>
</main>
"#
    .replace("__PARAM_ID__", param.id)
    .replace("__PARAM_LABEL__", param.label)
    .replace("__PARAM_DEFAULT__", &param.default_normalized.to_string())
}
