#!/bin/sh
# Install a verified release: sh install.sh [--version vX.Y.Z] [--install-dir DIR]
set -eu

REPO="benvds/skill-sync"
BINARY="skill-sync"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${SKILL_SYNC_VERSION:-}"

usage() {
    echo "Usage: sh install.sh [--version vX.Y.Z] [--install-dir DIR]"
    echo "Defaults: latest release, \${HOME}/.local/bin"
    echo "Environment: SKILL_SYNC_VERSION, INSTALL_DIR"
}
fail() { echo "error: $*" >&2; exit 1; }
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version|--install-dir)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail "$1 requires a value"
            case "$1" in
                --version) VERSION="$2" ;;
                --install-dir) INSTALL_DIR="$2" ;;
            esac
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) fail "unknown option: $1" ;;
    esac
done
[ -n "$INSTALL_DIR" ] || fail "install directory cannot be empty"
case "$INSTALL_DIR" in
    /*) ;;
    *) INSTALL_DIR="$(pwd)/${INSTALL_DIR}" ;;
esac

OS=$(uname -s)
ARCH=$(uname -m)
case "${OS}-${ARCH}" in
    Darwin-arm64) TARGET="aarch64-apple-darwin" ;;
    Linux-x86_64) TARGET="x86_64-unknown-linux-musl" ;;
    *) fail "unsupported platform: ${OS}-${ARCH}; use macOS ARM64 or Linux x86_64, or build with Cargo" ;;
esac

if [ -z "$VERSION" ]; then
    echo "Fetching latest release..."
    RELEASE=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")
    VERSION=$(printf '%s\n' "$RELEASE" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
fi
case "$VERSION" in
    ''|*[!a-zA-Z0-9._-]*) fail "invalid release version: $VERSION" ;;
esac

WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/skill-sync.XXXXXX")
STAGED=""
cleanup() {
    rm -rf "$WORK_DIR"
    if [ -n "$STAGED" ]; then rm -f "$STAGED"; fi
}
trap cleanup 0
trap 'exit 1' HUP INT TERM

ARCHIVE="${BINARY}-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"
echo "Downloading ${BINARY} ${VERSION} for ${TARGET}..."
curl -fsSL "$URL" -o "$WORK_DIR/$ARCHIVE"
curl -fsSL "${URL}.sha256" -o "$WORK_DIR/checksum"
EXPECTED=$(awk -v archive="$ARCHIVE" 'length($1) == 64 && $2 == archive { print $1 }' "$WORK_DIR/checksum")
case "$EXPECTED" in
    ''|*[!a-fA-F0-9]*) fail "invalid or missing SHA-256 checksum for $ARCHIVE" ;;
esac
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "$WORK_DIR/$ARCHIVE" | awk '{ print $1 }')
else
    ACTUAL=$(shasum -a 256 "$WORK_DIR/$ARCHIVE" | awk '{ print $1 }')
fi
[ "$ACTUAL" = "$EXPECTED" ] || fail "SHA-256 checksum mismatch; existing installation preserved"

tar -xzf "$WORK_DIR/$ARCHIVE" -C "$WORK_DIR" "$BINARY"
[ -f "$WORK_DIR/$BINARY" ] && [ ! -L "$WORK_DIR/$BINARY" ] || fail "release does not contain a regular binary"
mkdir -p "$INSTALL_DIR"
[ ! -d "$INSTALL_DIR/$BINARY" ] || fail "destination is a directory: $INSTALL_DIR/$BINARY"
# Stage on the destination filesystem so replacement is one atomic rename.
STAGED=$(mktemp "$INSTALL_DIR/.skill-sync.XXXXXX")
cp "$WORK_DIR/$BINARY" "$STAGED"
chmod 755 "$STAGED"
mv -f "$STAGED" "$INSTALL_DIR/$BINARY"
STAGED=""
echo "Installed ${BINARY} ${VERSION} to ${INSTALL_DIR}/${BINARY}"
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "Add ${INSTALL_DIR} to your PATH to run ${BINARY}." ;;
esac
