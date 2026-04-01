#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

if ! command -v nix >/dev/null 2>&1; then
  printf '%s\n' "nix is not installed or not on PATH" >&2
  exit 1
fi

mkdir -p "${HOME}/.config/nix"
cat > "${HOME}/.config/nix/nix.conf" <<'EOF'
experimental-features = nix-command flakes
EOF

nix run "path:${SCRIPT_DIR}#homeConfigurations.vscode.activationPackage"
