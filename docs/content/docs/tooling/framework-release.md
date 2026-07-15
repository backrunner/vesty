---
title: Framework releases
description: Publish version-matched crates, npm packages, CLI binaries, and a GitHub Release through CI.
order: 5
---

Vesty releases are tag-driven. A release tag publishes the framework SDK and CLI as one compatible set; uploading a CLI binary without its Rust and npm dependencies is not a valid release.

## Configure repository environments

Complete the first crates.io and npm publications manually. Both registries require a package to exist before its owner can add a trusted publisher. After the packages exist, create two protected GitHub environments with no registry secrets:

- `crates-io` protects the `publish-crates` job. For every Vesty crate, add a GitHub trusted publisher with repository owner `backrunner`, repository name `vesty`, workflow filename `release.yml`, and environment `crates-io`.
- `npm` protects the `publish-npm` job. For all four `@vesty/*` packages, add a GitHub Actions trusted publisher with repository owner `backrunner`, repository name `vesty`, workflow filename `release.yml`, and environment `npm`.

Require reviewer approval on both environments. The workflow requests GitHub OIDC identity tokens and exchanges them for short-lived registry credentials; it does not read a crates.io or npm publishing secret.

The GitHub repository must exist at `backrunner/vesty` before either registry publisher is configured. If the final repository owner changes, update the documentation and use that exact owner in all 17 publisher records.

## Prepare a version

Use one SemVer value across `[workspace.package].version`, every Vesty crate, and every `packages/*/package.json`. Internal `@vesty/plugin-ui` peer and development dependencies must use the same exact version.

For example, an alpha release uses a tag such as `v0.1.0-alpha.1`; a stable release uses `v0.1.0`. Do not add build metadata to release tags.

Before tagging, run:

```bash
node scripts/release/verify-version.mjs v0.1.0-alpha.1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm test
cargo run -p vesty-cli -- publish-plan --out target/publish-plan.json
cargo run -p vesty-cli -- crate-package --out target/crate-package.json
cargo run -p vesty-cli -- npm-pack --out target/npm-pack.json
```

The final stable release still requires the external DAW, platform WebView, validator, signing, and notarization evidence described in [Release evidence](/docs/tooling/release-evidence).

## Publish through CI

Create and push an annotated tag only after the version commit is on `main`:

```bash
git tag -a v0.1.0-alpha.1 -m "Vesty v0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

The release workflow then:

1. Verifies every Rust and npm package matches the tag.
2. Generates and checks publish-plan, crate-package, and npm-pack evidence.
3. Builds Linux x64, universal macOS, and Windows x64 CLI archives.
4. Publishes 13 crates in dependency order and waits for each crates.io index entry.
5. Publishes `@vesty/plugin-ui` before the React, Vue, and Svelte adapters.
6. Runs each released CLI against a generated Rust project and runs an additional React template build.
7. Generates `SHA256SUMS`, build provenance attestations, and the GitHub Release.

Registry publication is immutable and cannot be rolled back. If a job fails after some packages were published, fix the failure and rerun the same tag workflow. The scripts skip versions already present in a registry and continue from the first missing package. The GitHub Release is withheld until every smoke test passes.

Prerelease versions use their SemVer channel as the npm dist-tag, such as `alpha`, `beta`, or `rc`. Stable versions use `latest`.

## Verify the published release

Download the archives from the completed workflow or GitHub Release, verify them against `SHA256SUMS`, and run:

```bash
vesty --version
vesty templates
vesty new release-smoke --template gain
cargo check --manifest-path release-smoke/Cargo.toml
```

For a prerelease installer test, pass its explicit tag:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/backrunner/vesty/main/scripts/install.sh \
  | VESTY_VERSION=v0.1.0-alpha.1 sh
```
