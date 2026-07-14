---
title: Release evidence
description: Know which checks are local and which require real hosts or platforms.
order: 3
---

Vesty's release gate is intentionally strict. A successful unit test cannot prove that a DAW scanned the plugin, that WebView2 attached on Windows, or that Apple accepted a notarization submission.

## Local evidence

- Workspace tests and clippy.
- Generated protocol and parameter manifest drift checks.
- Crate publish order and package readiness.
- npm pack boundaries and dependency latest baseline.
- Platform-local strict static bundle validation.

## External evidence

- Scan, load, UI, automation, save/restore, and offline render in supported DAWs.
- macOS, Windows x64, and Linux X11 system WebView smoke reports.
- Steinberg validator reports for every example and platform.
- GitHub Actions run URL and per-OS artifacts.
- macOS/Windows signing verification.
- Apple notarytool acceptance and stapler success.

## Run the final gate

```bash
vesty release-check \
  --strict \
  --require-release-artifacts \
  --release-evidence-dir target/release-evidence \
  --report target/release-check.json \
  --plan target/release-action-plan.json
```

A failed report is useful: the generated action plan names the missing evidence and the canonical path where it belongs. Never replace missing external evidence with fabricated marker files.

## Current project status

Vesty is alpha. Local implementation gates pass, while real DAW, cross-platform WebView, complete validator matrix, signing, and notarization evidence remain release requirements.

