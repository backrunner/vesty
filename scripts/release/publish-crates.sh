#!/usr/bin/env bash
set -euo pipefail

plan_path="${1:-target/publish-plan.json}"
release_tag="${VESTY_RELEASE_VERSION:?VESTY_RELEASE_VERSION must contain the v-prefixed release tag}"
release_version="${release_tag#v}"

: "${CARGO_REGISTRY_TOKEN:?the crates.io trusted publisher token must be exposed as CARGO_REGISTRY_TOKEN}"

if [[ ! -f "${plan_path}" ]]; then
  echo "publish plan not found: ${plan_path}" >&2
  exit 1
fi

wait_for_crate() {
  local name="$1"
  local version="$2"

  for attempt in $(seq 1 30); do
    if curl -fsS \
      -A "vesty-release/${version}" \
      "https://crates.io/api/v1/crates/${name}/${version}" \
      >/dev/null 2>&1 \
      && cargo info --registry crates-io "${name}@${version}" >/dev/null 2>&1; then
      return 0
    fi

    echo "waiting for ${name}@${version} to reach the crates.io index (${attempt}/30)"
    sleep 10
  done

  echo "timed out waiting for ${name}@${version} to reach the crates.io index" >&2
  return 1
}

while IFS=$'\t' read -r name version; do
  if [[ "${version}" != "${release_version}" ]]; then
    echo "publish plan version mismatch for ${name}: ${version} != ${release_version}" >&2
    exit 1
  fi

  if curl -fsS \
    -A "vesty-release/${version}" \
    "https://crates.io/api/v1/crates/${name}/${version}" \
    >/dev/null 2>&1; then
    echo "skipping ${name}@${version}; it is already published"
  else
    cargo publish --locked -p "${name}"
  fi

  wait_for_crate "${name}" "${version}"
done < <(jq -r '.packages[] | [.name, .version] | @tsv' "${plan_path}")
