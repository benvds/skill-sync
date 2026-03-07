#!/bin/sh
# install.sh — Download and install skills-sync from GitHub Releases
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/benvds/skills-sync/main/install.sh | sh

set -e

# Repository benvds/name on GitHub
REPO="benvds/skills-sync"
BINARY="skills-sync"
INSTALL_DIR="${HOME}/.local/bin"

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  Darwin-arm64)
    TARGET="aarch64-apple-darwin"
    ;;
  Linux-x86_64)
    TARGET="x86_64-unknown-linux-musl"
    ;;
  *)
    echo "error: unsupported platform: ${OS}-${ARCH}" >&2
    echo "  Supported platforms: macOS ARM64 (Darwin-arm64), Linux x86_64 (Linux-x86_64)" >&2
    exit 1
    ;;
esac

# Fetch the latest version tag from GitHub API
echo "Fetching latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "${LATEST}" ]; then
  echo "error: could not determine latest release version" >&2
  echo "  Check that ${REPO} has published releases." >&2
  exit 1
fi

# Build archive name and download URL
ARCHIVE="${BINARY}-${LATEST}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"

echo "Downloading ${BINARY} ${LATEST} for ${TARGET}..."
curl -fsSL "${URL}" -o "/tmp/${ARCHIVE}"

# Extract to install directory
mkdir -p "${INSTALL_DIR}"
tar -xzf "/tmp/${ARCHIVE}" -C "/tmp"
mv "/tmp/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"
rm "/tmp/${ARCHIVE}"

echo "Installed ${BINARY} ${LATEST} to ${INSTALL_DIR}/${BINARY}"

# Suggest adding to PATH if not already present
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    echo ""
    echo "hint: ${INSTALL_DIR} is not in your PATH."
    echo "  Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "    export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    echo ""
    ;;
esac
