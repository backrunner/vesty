#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"
cd "$root"

if [[ ! -f Cargo.toml ]]; then
  echo "error: no Cargo.toml found in $(pwd)" >&2
  exit 2
fi

echo "==> cargo fmt"
cargo fmt --all --check

echo "==> cargo test"
cargo test --workspace

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

if [[ -f ui/package.json ]]; then
  echo "==> UI checks"
  if [[ -f ui/pnpm-lock.yaml ]] && command -v pnpm >/dev/null 2>&1; then
    pnpm --dir ui install --frozen-lockfile
    ui_runner=(pnpm --dir ui run)
  elif [[ -f ui/yarn.lock ]] && command -v yarn >/dev/null 2>&1; then
    yarn --cwd ui install --frozen-lockfile
    ui_runner=(yarn --cwd ui run)
  else
    if [[ -f ui/package-lock.json ]]; then
      npm --prefix ui ci
    else
      npm --prefix ui install
    fi
    ui_runner=(npm --prefix ui run)
  fi

  for script in typecheck test build; do
    if node -e \
      'const p=require("./ui/package.json"); process.exit(p.scripts?.[process.argv[1]] ? 0 : 1)' \
      "$script"; then
      "${ui_runner[@]}" "$script"
    fi
  done
fi

if [[ -f params.specs.json && -f vesty-parameters.json ]]; then
  cli=()
  if [[ -n "${VESTY_MANIFEST:-}" ]]; then
    cli=(cargo run --manifest-path "$VESTY_MANIFEST" -p vesty-cli --)
  elif command -v vesty >/dev/null 2>&1; then
    cli=(vesty)
  elif cargo metadata --no-deps --format-version 1 2>/dev/null | grep -q '"name":"vesty-cli"'; then
    cli=(cargo run -p vesty-cli --)
  fi

  if (( ${#cli[@]} > 0 )); then
    echo "==> parameter manifest"
    "${cli[@]}" param-manifest \
      --specs params.specs.json \
      --out vesty-parameters.json \
      --check
  else
    echo "warning: skipped parameter manifest check; install vesty or set VESTY_MANIFEST" >&2
  fi
fi

echo "==> local Vesty checks passed"
