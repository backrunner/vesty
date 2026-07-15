---
title: Tooling
description: Scaffold, package, validate, and audit a Vesty project.
order: 4
---

The `vesty` CLI turns repository conventions into repeatable commands.

- [CLI workflows](/docs/tooling/cli) covers scaffolding and local checks.
- [Packaging and validation](/docs/tooling/packaging) produces inspectable VST3 bundles.
- [Release evidence](/docs/tooling/release-evidence) distinguishes local confidence from external proof.
- [AI-assisted development](/docs/tooling/ai-development) installs the companion skill and defines a safe AI workflow.
- [Framework releases](/docs/tooling/framework-release) publishes version-matched registry packages and CLI binaries through CI.

After installing the CLI, run `vesty --help` to inspect the available commands. Framework contributors can still use `cargo run -p vesty-cli -- --help` inside the Vesty workspace.
