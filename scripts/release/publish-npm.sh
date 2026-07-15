#!/usr/bin/env bash
set -euo pipefail

release_tag="${VESTY_RELEASE_VERSION:?VESTY_RELEASE_VERSION must contain the v-prefixed release tag}"
release_version="${release_tag#v}"
npm_tag="latest"

if [[ "${release_version}" == *-* ]]; then
  npm_tag="${release_version#*-}"
  npm_tag="${npm_tag%%.*}"
fi

publish_package() {
  local directory="$1"
  local name
  local version

  name=$(node -p "require('./${directory}/package.json').name")
  version=$(node -p "require('./${directory}/package.json').version")

  if [[ "${version}" != "${release_version}" ]]; then
    echo "package version mismatch for ${name}: ${version} != ${release_version}" >&2
    exit 1
  fi

  if npm view "${name}@${version}" version >/dev/null 2>&1; then
    echo "skipping ${name}@${version}; it is already published"
  else
    npm publish "./${directory}" --access public --provenance --tag "${npm_tag}"
  fi

  for attempt in $(seq 1 30); do
    if [[ "$(npm view "${name}@${version}" version 2>/dev/null || true)" == "${version}" ]]; then
      return 0
    fi
    echo "waiting for ${name}@${version} to reach npm (${attempt}/30)"
    sleep 10
  done

  echo "timed out waiting for ${name}@${version} to reach npm" >&2
  return 1
}

publish_package packages/plugin-ui
publish_package packages/react
publish_package packages/vue
publish_package packages/svelte
