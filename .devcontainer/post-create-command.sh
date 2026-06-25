#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVCONTAINER_PROFILE="${HOME}/.local/state/nix/profiles/devcontainer"

mkdir -p "$(dirname "${DEVCONTAINER_PROFILE}")"
nix profile add --profile "${DEVCONTAINER_PROFILE}" "path:${SCRIPT_DIR}#devcontainer-tools"

export PATH="${DEVCONTAINER_PROFILE}/bin:${HOME}/.nix-profile/bin:${HOME}/.cargo/bin:${PATH}"

cd "${WORKSPACE_FOLDER:-${SCRIPT_DIR}/..}"

if ! rustup show active-toolchain >/dev/null 2>&1; then
  rustup toolchain install
fi

if ! command -v covgate >/dev/null 2>&1; then
  cargo install covgate --locked
fi
