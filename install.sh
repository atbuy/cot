#!/usr/bin/env sh
# install.sh - install cot from GitHub Releases
set -eu

REPO="atbuy/cot"
VERSION=""
USE_SYSTEM=0

usage() {
  cat <<USAGE
Usage: install.sh [OPTIONS]

Install cot from GitHub Releases.

Options:
  --version <tag>   Install a specific release (e.g. v0.1.0). Default: latest
  --system          Install to /usr/local/bin instead of ~/.local/bin (requires sudo)
  --help            Show this help message
USAGE
}

fail() {
  echo "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      [ $# -ge 2 ] || fail "Option --version requires a value."
      VERSION="$2"
      shift 2
      ;;
    --version=*)
      VERSION="${1#*=}"
      shift
      ;;
    --system)
      USE_SYSTEM=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ "${USE_SYSTEM}" = "1" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
fi

for tool in curl tar sha256sum; do
  command -v "${tool}" >/dev/null 2>&1 || fail "Required tool not found: ${tool}"
done

[ "$(uname -s)" = "Linux" ] || fail "cot only runs on Linux."

ARCH="$(uname -m)"
[ "${ARCH}" = "x86_64" ] || fail "There is no prebuilt release for ${ARCH}. Build from source instead."

os_field() {
  sed -n "s/^$1=//p" /etc/os-release | tr -d "\"'" | head -n 1
}

# Each distribution release gets its own build, as GTK is linked dynamically.
OS_ID="$(os_field ID)"
case "${OS_ID}" in
  arch)
    DISTRO="arch"
    DEPENDENCIES="sudo pacman -S --needed grim wl-clipboard gtk4 gtk4-layer-shell"
    ;;
  ubuntu)
    DISTRO="ubuntu-$(os_field VERSION_ID)"
    DEPENDENCIES="sudo apt install grim wl-clipboard libgtk-4-1 libgtk4-layer-shell0"
    ;;
  *)
    fail "There is no prebuilt release for ${OS_ID:-this system}. Build from source instead."
    ;;
esac

if [ -z "${VERSION}" ]; then
  echo "Fetching latest release version..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "${VERSION}" ] || fail "Failed to fetch the latest release version from GitHub."
fi

ARCHIVE="cot-${DISTRO}-${ARCH}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
echo "Installing cot ${VERSION} (${DISTRO}/${ARCH})..."

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT INT TERM

if ! curl -fsSL "${BASE_URL}/${ARCHIVE}" -o "${WORK_DIR}/${ARCHIVE}" \
  || ! curl -fsSL "${BASE_URL}/checksums.txt" -o "${WORK_DIR}/checksums.txt"; then
  echo "Download failed. Check that ${ARCHIVE} exists for ${VERSION} at:" >&2
  echo "  https://github.com/${REPO}/releases" >&2
  exit 1
fi

(cd "${WORK_DIR}" && grep -F "  ${ARCHIVE}" checksums.txt | sha256sum --check --status -) \
  || fail "Checksum verification failed for ${ARCHIVE}. Not installing it."

tar -xzf "${WORK_DIR}/${ARCHIVE}" -C "${WORK_DIR}"
[ -f "${WORK_DIR}/cot" ] || fail "Expected binary not found in ${ARCHIVE}."

if [ "${USE_SYSTEM}" = "1" ]; then
  sudo install -Dm755 "${WORK_DIR}/cot" "${INSTALL_DIR}/cot"
else
  mkdir -p "${INSTALL_DIR}" || fail "Cannot create ${INSTALL_DIR}. Try --system."
  install -m755 "${WORK_DIR}/cot" "${INSTALL_DIR}/cot"
fi

echo "Installed to ${INSTALL_DIR}/cot"

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "${INSTALL_DIR} is not on your PATH. Add this to your shell profile:"
    echo ""
    # shellcheck disable=SC2016
    echo '  export PATH="${HOME}/.local/bin:${PATH}"'
    ;;
esac

echo ""
if ! "${INSTALL_DIR}/cot" version 2>/dev/null; then
  echo "cot is installed but can't start yet, as some libraries are missing."
fi

MISSING=0
"${INSTALL_DIR}/cot" version >/dev/null 2>&1 || MISSING=1
for tool in grim wl-copy; do
  command -v "${tool}" >/dev/null 2>&1 || MISSING=1
done
if [ "${MISSING}" = "1" ]; then
  echo ""
  echo "Install the remaining dependencies with:"
  echo ""
  echo "  ${DEPENDENCIES}"
fi
