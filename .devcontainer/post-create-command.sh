#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

if ! command -v nix >/dev/null 2>&1; then
  printf '%s\n' "nix is not installed or not on PATH" >&2
  exit 1
fi

mkdir -p "${HOME}/.config/nix"

NIX_CONF="${HOME}/.config/nix/nix.conf"

touch "${NIX_CONF}"

if ! grep -Eq '^[[:space:]]*experimental-features[[:space:]]*=.*\bnix-command\b.*\bflakes\b' "${NIX_CONF}"; then
  printf '%s\n' "experimental-features = nix-command flakes" >> "${NIX_CONF}"
fi

nix run "path:${SCRIPT_DIR}#homeConfigurations.vscode.activationPackage"
