#!/usr/bin/env bash
set -euo pipefail

SETUP_LABEL="agent-env-setup"

if [[ "${DEBUG:-}" == "1" ]]; then
	set -x
fi

if [[ "$(id -u)" -eq 0 ]]; then
	SUDO=""
else
	SUDO="sudo"
fi

if ! command -v apt-get >/dev/null 2>&1; then
	echo "This setup script currently supports Debian/Ubuntu environments with apt-get." >&2
	exit 1
fi

export DEBIAN_FRONTEND=noninteractive
CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
YQ_VERSION="${YQ_VERSION:-v4.48.1}"
CARGO_LLVM_COV_VERSION="${CARGO_LLVM_COV_VERSION:-latest}"
CARGO_MACHETE_VERSION="${CARGO_MACHETE_VERSION:-latest}"
CARGO_DENY_VERSION="${CARGO_DENY_VERSION:-latest}"
COVGATE_VERSION="${COVGATE_VERSION:-latest}"

need_cmd() {
	local cmd="$1"
	! command -v "$cmd" >/dev/null 2>&1
}

linux_arch() {
	local arch
	arch="$(uname -m)"

	case "$arch" in
	x86_64)
		echo "x86_64"
		;;
	aarch64 | arm64)
		echo "aarch64"
		;;
	*)
		echo "unsupported architecture: ${arch}" >&2
		return 1
		;;
	esac
}

github_asset_arch() {
	local arch
	arch="$(linux_arch)"

	case "$arch" in
	x86_64)
		echo "x86_64"
		;;
	aarch64)
		echo "aarch64"
		;;
	esac
}

github_api_arch() {
	local arch
	arch="$(linux_arch)"

	case "$arch" in
	x86_64)
		echo "amd64"
		;;
	aarch64)
		echo "arm64"
		;;
	esac
}

ensure_fd_command() {
	if need_cmd fd && ! need_cmd fdfind; then
		local fdfind_path
		fdfind_path="$(command -v fdfind)"
		$SUDO ln -sf "$fdfind_path" /usr/local/bin/fd
		echo "${SETUP_LABEL}: linked fd -> ${fdfind_path}"
	fi
}

ensure_yq() {
	if ! need_cmd yq; then
		echo "${SETUP_LABEL}: yq already installed"
		return 0
	fi

	local arch tmp_dir url
	arch="$(github_api_arch)"
	tmp_dir="$(mktemp -d)"
	url="https://github.com/mikefarah/yq/releases/download/${YQ_VERSION}/yq_linux_${arch}"

	echo "${SETUP_LABEL}: installing yq ${YQ_VERSION}"
	curl -fsSL "${url}" -o "${tmp_dir}/yq"
	chmod +x "${tmp_dir}/yq"
	$SUDO install -m 0755 "${tmp_dir}/yq" /usr/local/bin/yq
	rm -rf "${tmp_dir}"
}

get_latest_release() {
	local repo="$1"
	curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" | jq -r '.tag_name'
}

resolve_release_tag() {
	local repo="$1"
	local requested_version="$2"

	if [[ "${requested_version}" == "latest" ]]; then
		get_latest_release "${repo}"
	else
		echo "${requested_version}"
	fi
}

ensure_cargo_tool_binary() {
	local binary_name="$1"
	local repo="$2"
	local requested_version="$3"
	local archive_name="$4"

	if command -v "${binary_name}" >/dev/null 2>&1; then
		echo "${SETUP_LABEL}: ${binary_name} already installed"
		return 0
	fi

	local version url tmp_dir binary_path
	version="$(resolve_release_tag "${repo}" "${requested_version}")"
	archive_name="${archive_name//\{version\}/${version}}"
	url="https://github.com/${repo}/releases/download/${version}/${archive_name}"
	tmp_dir="$(mktemp -d)"

	echo "${SETUP_LABEL}: downloading ${binary_name} ${version}"
	mkdir -p "${CARGO_BIN_DIR}"
	curl -fsSL "${url}" -o "${tmp_dir}/archive.tar.gz"
	tar -xzf "${tmp_dir}/archive.tar.gz" -C "${tmp_dir}"
	binary_path="$(find "${tmp_dir}" -name "${binary_name}" -type f -perm -111 | head -n 1)"

	if [[ -z "${binary_path}" ]]; then
		echo "${SETUP_LABEL}: error: binary ${binary_name} not found in archive" >&2
		rm -rf "${tmp_dir}"
		exit 1
	fi

	install -m 0755 "${binary_path}" "${CARGO_BIN_DIR}/${binary_name}"
	rm -rf "${tmp_dir}"
}

APT_PACKAGES=()

# Useful agentic tooling
need_cmd curl && APT_PACKAGES+=(curl)
need_cmd jq && APT_PACKAGES+=(jq)
need_cmd rg && APT_PACKAGES+=(ripgrep)
need_cmd fdfind && APT_PACKAGES+=(fd-find)
need_cmd eza && APT_PACKAGES+=(eza)
need_cmd shellcheck && APT_PACKAGES+=(shellcheck)
need_cmd shfmt && APT_PACKAGES+=(shfmt)

if ((${#APT_PACKAGES[@]} > 0)); then
	$SUDO apt-get update
	$SUDO apt-get install -y --no-install-recommends "${APT_PACKAGES[@]}"
	echo "${SETUP_LABEL}: installed apt packages: ${APT_PACKAGES[*]}"
else
	echo "${SETUP_LABEL}: required apt-managed tools already present; nothing to install."
fi

ensure_fd_command
ensure_yq

# Rust workflow tools
if need_cmd rustup; then
	echo "${SETUP_LABEL}: rustup not found, skipping rust toolchain setup"
else
	rustup component add llvm-tools-preview || true
fi

ensure_cargo_tool_binary \
	"cargo-llvm-cov" \
	"taiki-e/cargo-llvm-cov" \
	"${CARGO_LLVM_COV_VERSION}" \
	"cargo-llvm-cov-$(github_asset_arch)-unknown-linux-gnu.tar.gz"
ensure_cargo_tool_binary \
	"cargo-machete" \
	"bnjbvr/cargo-machete" \
	"${CARGO_MACHETE_VERSION}" \
	"cargo-machete-{version}-$(github_asset_arch)-unknown-linux-musl.tar.gz"
ensure_cargo_tool_binary \
	"cargo-deny" \
	"EmbarkStudios/cargo-deny" \
	"${CARGO_DENY_VERSION}" \
	"cargo-deny-{version}-$(github_asset_arch)-unknown-linux-musl.tar.gz"
ensure_cargo_tool_binary \
	"covgate" \
	"jesse-black/covgate" \
	"${COVGATE_VERSION}" \
	"covgate-{version}-$(github_asset_arch)-unknown-linux-musl.tar.gz"

echo "${SETUP_LABEL}: Complete!"
