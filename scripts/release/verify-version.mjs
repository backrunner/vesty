import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const tag = process.argv[2];

if (!tag || !/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/.test(tag)) {
  throw new Error(`release tag must be v-prefixed semver without build metadata, got ${tag ?? "<missing>"}`);
}

const version = tag.slice(1);
const cargo = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
  encoding: "utf8"
});

if (cargo.status !== 0) {
  process.stderr.write(cargo.stderr);
  throw new Error("cargo metadata failed");
}

const metadata = JSON.parse(cargo.stdout);
const publishableCrates = metadata.packages.filter((pkg) => pkg.publish === null || pkg.publish.length > 0);

for (const pkg of publishableCrates) {
  if (pkg.version !== version) {
    throw new Error(`${pkg.name} uses ${pkg.version}; release tag requires ${version}`);
  }
}

const packageDirectories = ["packages/plugin-ui/package.json"];

for (const manifestPath of packageDirectories) {
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (manifest.version !== version) {
    throw new Error(`${manifest.name} uses ${manifest.version}; release tag requires ${version}`);
  }

  if (manifest.name !== "vesty-plugin-ui") {
    throw new Error(`${manifestPath} must publish vesty-plugin-ui`);
  }
}

console.log(
  `release ${tag}: ${publishableCrates.length} Rust crates and ${packageDirectories.length} npm packages share version ${version}`
);
