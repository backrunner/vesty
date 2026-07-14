---
title: Troubleshooting
description: Diagnose common build, editor, automation, and release failures.
order: 6
---

## The WebView feature does not compile

Install the platform WebView development dependencies, then run the same feature gate as CI:

```bash
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo test -p vesty-ui-wry --features wry-backend
```

Linux requires GTK/WebKitGTK/X11 development packages. Windows uses WebView2. macOS uses WKWebView.

## The editor shows default instead of restored values

Initialize controls from `ready.paramValues`, not `ready.params[].defaultNormalized`. Subscribe to `param.changed`, and keep the control bound to confirmed host state.

## A parameter edit returns host_rejected

The host declined `performEdit`. Do not update the controller or keep optimistic UI state. End the gesture, restore the last confirmed value, and inspect host automation or read-only flags.

## Strict validation reports missing exports

Run validation on the same platform as the packaged binary. Ensure `GetPluginFactory` and platform module entry points are exported, and install the required inspection tool (`nm`, `dumpbin`, or the configured equivalent).

## release-check still fails after tests pass

This is expected when external evidence is missing. Open the generated action plan and collect the named DAW, platform, validator, CI, signing, or notarization artifact. Do not use `--skip-protocol` or placeholder evidence for a final release.

## The audio callback faults

Vesty silences output after a panic, but the algorithm still needs correction. Reproduce the block in a unit test, run with `NoAllocGuard`, check all capacity assumptions, and inspect realtime diagnostics outside the callback.
