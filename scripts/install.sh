#!/usr/bin/env sh
set -eu

repository="${VESTY_REPOSITORY:-orchiliao/vesty}"
version="${VESTY_VERSION:-latest}"
install_dir="${VESTY_INSTALL_DIR:-${HOME}/.local/bin}"

case "$(uname -s)" in
  Darwin)
    asset="vesty-universal-apple-darwin.tar.gz"
    ;;
  Linux)
    case "$(uname -m)" in
      x86_64 | amd64)
        asset="vesty-x86_64-unknown-linux-musl.tar.gz"
        ;;
      *)
        echo "Vesty does not publish a Linux binary for architecture $(uname -m) yet." >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "This installer supports macOS and Linux. Use install.ps1 on Windows." >&2
    exit 1
    ;;
esac

case "${version}" in
  latest)
    download_base="https://github.com/${repository}/releases/latest/download"
    ;;
  v*)
    download_base="https://github.com/${repository}/releases/download/${version}"
    ;;
  *)
    echo "VESTY_VERSION must be latest or a v-prefixed release tag." >&2
    exit 1
    ;;
esac

temporary_dir="$(mktemp -d)"
trap 'rm -rf "${temporary_dir}"' EXIT HUP INT TERM

curl --proto '=https' --tlsv1.2 -fLsS \
  "${download_base}/${asset}" \
  -o "${temporary_dir}/${asset}"
curl --proto '=https' --tlsv1.2 -fLsS \
  "${download_base}/SHA256SUMS" \
  -o "${temporary_dir}/SHA256SUMS"

expected_checksum="$(awk -v asset="${asset}" '$2 == asset || $2 == "*" asset { print $1; exit }' "${temporary_dir}/SHA256SUMS")"
if [ -z "${expected_checksum}" ]; then
  echo "SHA256SUMS does not contain ${asset}." >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual_checksum="$(sha256sum "${temporary_dir}/${asset}" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
  actual_checksum="$(shasum -a 256 "${temporary_dir}/${asset}" | awk '{ print $1 }')"
else
  echo "Install sha256sum or shasum before installing Vesty." >&2
  exit 1
fi

if [ "${actual_checksum}" != "${expected_checksum}" ]; then
  echo "Checksum verification failed for ${asset}." >&2
  exit 1
fi

archive_root="${asset%.tar.gz}"
tar -xzf "${temporary_dir}/${asset}" -C "${temporary_dir}"
mkdir -p "${install_dir}"
install -m 0755 "${temporary_dir}/${archive_root}/vesty" "${install_dir}/vesty"

echo "Installed vesty to ${install_dir}/vesty"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) echo "Add ${install_dir} to PATH before running vesty." ;;
esac
