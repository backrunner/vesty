---
title: AI-assisted development
description: Install the Vesty skill and keep AI changes inside the framework's realtime contract.
order: 4
---

Vesty includes a repository-distributed Codex skill at `skills/vesty-plugin-dev`. It gives an AI agent the project-specific workflow that generic Rust knowledge does not provide: stable VST3 identities, parameter manifests, realtime-safe kernel rules, JSBridge ownership, CLI validation, and honest release evidence.

The skill is guidance and automation, not a replacement for listening tests, code review, the Steinberg validator, or a real DAW.

## Install the skill

Install it into your personal Codex skills directory:

```bash
mkdir -p "${CODEX_HOME:-$HOME/.codex}/skills"
cp -R skills/vesty-plugin-dev \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

For framework contributors who want local edits to take effect immediately, use a symbolic link instead:

```bash
ln -s "$PWD/skills/vesty-plugin-dev" \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

Remove an older installed copy or link before replacing it. Restart the Codex session after installation so the skill catalog is refreshed.

Teams can also keep the skill in the repository and tell the agent its exact path. This is useful when a project pins a particular Vesty revision and should not inherit a newer global skill.

## Invoke it explicitly

Mention `$vesty-plugin-dev` in the request when the task changes audio code, host contracts, packaging, or release behavior:

```text
Use $vesty-plugin-dev to create a stereo compressor with threshold, ratio,
attack, release, makeup gain, and bypass. Start headless, add kernel tests,
and stop before Web UI work.
```

```text
Use $vesty-plugin-dev to add a Svelte editor to this existing plugin.
Preserve every parameter ID, initialize controls from current host values,
and verify rejected gestures and editor reload.
```

```text
Use $vesty-plugin-dev to review this kernel for realtime safety. Report
findings first; do not change code until the review identifies a concrete issue.
```

```text
Use $vesty-plugin-dev to package the current plugin for macOS and run every
local validation available. Keep missing DAW, signing, and notarization
evidence explicitly pending.
```

A strong request names the desired behavior, effect or instrument type, UI framework if any, compatibility constraints, and the expected stopping point. Do not ask an agent to “make it release-ready” without also providing access to every required host, platform, signing identity, and external validator.

## What the skill makes the agent do

The skill directs the agent to:

1. Read the nearest `AGENTS.md`, plugin metadata, parameter specifications, and existing architecture before editing.
2. Choose a maintained `vesty new` template when creating a project.
3. Preserve class IDs, bundle IDs, parameter IDs, bridge fields, and generated manifests.
4. Resolve parameter handles outside `process()` and keep callback work bounded and non-blocking.
5. Add kernel tests before UI complexity.
6. Keep the WebView host-authoritative through `ready.paramValues`, edit gestures, and confirmed events.
7. Run deterministic checks and report skipped checks separately.
8. Never substitute fabricated marker files for DAW, validator, signing, CI, or notarization evidence.

## Run the bundled verifier

From a plugin repository that contains `Cargo.toml`, run:

```bash
/path/to/vesty-plugin-dev/scripts/verify.sh .
```

The verifier runs Rust format, tests, and clippy. It also runs UI checks when `ui/package.json` exists and checks the parameter manifest when it can find the Vesty CLI.

For a plugin developed against a sibling framework checkout, identify the CLI workspace explicitly:

```bash
VESTY_MANIFEST=../vesty/Cargo.toml \
  /path/to/vesty-plugin-dev/scripts/verify.sh .
```

Read warnings as unfinished work. In particular, “skipped parameter manifest check” is not equivalent to a passing check.

## Keep human review in the loop

Before accepting AI-generated DSP, inspect every call reachable from `process()` for allocation, locking, blocking, panic, I/O, and unbounded work. Listen for discontinuities at parameter changes and compare offline with realtime rendering. Review the generated diff for accidental identity changes.

For a release candidate, a human still needs to run and preserve:

- official validator output;
- DAW scan, load, automation, save/restore, UI, and offline-render evidence;
- platform WebView smoke tests;
- signing and notarization verification;
- CI artifacts for every claimed target.

The skill helps an agent respect these gates and organize the output. It cannot perform a test on a host or platform that is not available in the environment.

## Keep the skill versioned with Vesty

When Vesty changes `Plugin`, `AudioKernel`, parameter manifests, JSBridge messages, CLI flags, or release gates, update `skills/vesty-plugin-dev` in the same pull request. Validate the package with:

```bash
python3 /path/to/skill-creator/scripts/quick_validate.py \
  skills/vesty-plugin-dev
```

The skill metadata in `agents/openai.yaml` is intentionally small; detailed knowledge belongs in `SKILL.md` and its `references/` files.
