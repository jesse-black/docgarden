#!/usr/bin/env bash
set -euo pipefail

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

if ! command -v covgate >/dev/null 2>&1; then
	echo "agent-env-maintenance: covgate not found on PATH." >&2
	exit 1
fi

echo "agent-env-maintenance: running covgate record-base"
covgate record-base

echo "agent-env-maintenance: Complete!"
