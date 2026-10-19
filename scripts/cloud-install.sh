#!/usr/bin/env bash
set -euo pipefail
utsc_repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$utsc_repo_root"
source scripts/cloud-env.sh
mkdir -p "$UTSC_TOOLCHAIN_ROOT"
if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
    case "$(uname -sm)" in
        'Linux x86_64') utsc_host=x86_64-unknown-linux-gnu ;;
        'Linux aarch64') utsc_host=aarch64-unknown-linux-gnu ;;
        *) echo 'Cloud bootstrap supports Linux x86_64 and aarch64.' >&2; exit 2 ;;
    esac
    utsc_install_tmp="$(mktemp -d "$UTSC_TOOLCHAIN_ROOT/install.XXXXXX")"
    trap 'rm -rf -- "$utsc_install_tmp"' EXIT
    utsc_installer_url="https://static.rust-lang.org/rustup/dist/$utsc_host/rustup-init"
    curl --fail --silent --show-error "$utsc_installer_url" -o "$utsc_install_tmp/rustup-init"
    curl --fail --silent --show-error "$utsc_installer_url.sha256" -o "$utsc_install_tmp/rustup-init.sha256"
    read -r utsc_expected_hash _ < "$utsc_install_tmp/rustup-init.sha256"
    printf '%s  %s\n' "$utsc_expected_hash" "$utsc_install_tmp/rustup-init" | sha256sum --check -
    chmod +x "$utsc_install_tmp/rustup-init"
    "$utsc_install_tmp/rustup-init" -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt --component clippy
fi
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
# Installs are locked, noninteractive, and do not execute npm lifecycle scripts.
npm ci --ignore-scripts --cache /workspace/.cache/npm
cargo fetch --locked
cargo build --locked
