#!/bin/sh
# opencli-rs installer — detects OS/Arch and downloads the right binary
# Usage: curl -fsSL https://raw.githubusercontent.com/xxx/opencli-rs/main/scripts/install.sh | sh

set -e

REPO="nashsu/opencli-rs"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="opencli-rs"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

info() { printf "${CYAN}$1${NC}\n"; }
success() { printf "${GREEN}$1${NC}\n"; }
error() { printf "${RED}Error: $1${NC}\n" >&2; exit 1; }

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    linux*)  OS="unknown-linux-musl" ;;
    darwin*) OS="apple-darwin" ;;
    mingw*|msys*|cygwin*) OS="pc-windows-msvc" ;;
    *) error "Unsupported OS: $OS" ;;
esac

# Detect Arch
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)  ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

TARGET="${ARCH}-${OS}"

# Get latest version
info "Detecting latest version..."
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$VERSION" ]; then
    error "Could not detect latest version. Check https://github.com/${REPO}/releases"
fi
info "Latest version: ${VERSION}"

# Determine archive format
if echo "$OS" | grep -q "windows"; then
    EXT="zip"
    ARCHIVE="${BINARY_NAME}-${TARGET}.zip"
else
    EXT="tar.gz"
    ARCHIVE="${BINARY_NAME}-${TARGET}.tar.gz"
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

# Download
info "Downloading ${ARCHIVE}..."
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

if ! curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}"; then
    error "Download failed. Binary may not exist for ${TARGET}.\nCheck: https://github.com/${REPO}/releases/tag/${VERSION}"
fi

# Extract
info "Extracting..."
cd "$TMPDIR"
if [ "$EXT" = "zip" ]; then
    unzip -q "$ARCHIVE"
else
    tar xzf "$ARCHIVE"
fi

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY_NAME" "$INSTALL_DIR/"
else
    info "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "$BINARY_NAME" "$INSTALL_DIR/"
fi

chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Verify
if command -v "$BINARY_NAME" >/dev/null 2>&1; then
    INSTALLED_VERSION=$("$BINARY_NAME" --version 2>/dev/null || echo "unknown")
    success "✓ ${BINARY_NAME} installed successfully! (${INSTALLED_VERSION})"
    echo ""
    echo "  Get started:"
    echo "    ${BINARY_NAME} --help"
    echo "    ${BINARY_NAME} hackernews top --limit 5"
else
    success "✓ Installed to ${INSTALL_DIR}/${BINARY_NAME}"
    echo "  Make sure ${INSTALL_DIR} is in your PATH."
fi
