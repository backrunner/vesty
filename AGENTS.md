# AGENTS.md

## Project Status

Vesty is an alpha Rust-first framework for VST3 plugins with realtime-safe DSP and system WebView UIs. The repository has been renamed from `vesti` to `vesty`; all crate names, npm package names, CLI references, example configs, parameter manifests, CI references, and documentation should use `vesty`, `Vesty`, or `VESTY` as appropriate.

The current workspace combines:

- Rust crates for plugin traits, DSP process contexts, typed parameters, realtime queues, VST3 integration, WebView UI runtime, packaging, validation, CLI workflows, and macros.
- The `vesty-plugin-ui` npm package for JSBridge integration and React/Vue/Svelte subpath adapters.
- Example plugins under `examples/`.
- Research, architecture, implementation status, and completion-audit notes under `.agents/`.

The codebase is not release-complete. Release gates still require external evidence such as DAW smoke tests, platform WebView checks, Steinberg validator output, signing verification, CI artifacts, and notarization/stapling logs.

## Project Goals

- Provide a Rust-first path for authoring VST3 plugins with deterministic realtime audio behavior.
- Keep audio/DSP native while allowing editor UIs to be built with ordinary web frameworks.
- Keep realtime boundaries explicit and testable.
- Provide typed JSBridge protocols and generated TypeScript artifacts for UI integration.
- Make scaffolding, packaging, validation, and release-evidence workflows reproducible through the `vesty` CLI.
- Maintain a conservative MVP scope: VST3 first, direct `wry` WebView embedding, no Tauri dependency.

## Development Rules

- Prefer existing workspace patterns over new abstractions.
- Keep changes scoped to the touched crate/package and its direct contracts.
- Do not allocate, lock, block, perform JSON work, call WebView APIs, or format logs from audio `process` paths.
- Keep parameter IDs and generated parameter manifests stable. If the algorithm namespace or specs change, regenerate affected `vesty-parameters.json` files with `vesty param-manifest`.
- Keep JSBridge protocol changes synchronized across Rust exports and `packages/plugin-ui` TypeScript sources.
- Treat `target/`, `node_modules/`, `.vst3` bundles, and generated `dist/` folders as build artifacts unless a task explicitly says otherwise.
- Use `rg`/`rg --files` for searches.
- In Codex shell sessions for this project, prefix shell commands with `rtk` when available.
- Use `apply_patch` for manual source/doc edits.
- Do not revert unrelated user changes.

## Verification

Run the smallest meaningful check for the change, and widen coverage when touching shared contracts.

Common checks:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- param-manifest --specs examples/gain/params.specs.json --out examples/gain/vesty-parameters.json --check
```

For packaging/release work, also consult `.agents/14-completion-audit.md`.

## Git Identity

Repository commits should be authored as:

```text
BackRunner <dev@backrunner.top>
```

Before committing, verify the repository-local identity:

```bash
git config user.name BackRunner
git config user.email dev@backrunner.top
```

## Commit Convention

All human- and agent-authored commits must use this format:

```text
xxx(comp): desc
```

Rules:

- `xxx` is a lowercase type such as `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`, `build`, or `perf`.
- `comp` is a lowercase scope such as `core`, `params`, `vst3`, `cli`, `web`, `docs`, `agents`, `ci`, or `workspace`.
- `desc` is short, imperative, and lower sentence case.
- Keep each commit focused on one coherent change.
- Prefer several small commits over one mixed commit.

Examples:

```text
feat(cli): add parameter manifest command
fix(params): preserve stable vst3 ids
docs(agents): document project workflow
ci(workspace): add release evidence checks
```
