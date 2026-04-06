#!/bin/sh
set -e

REPO="aabele/realtime-markdown-viewer"
BINARY_NAME="rtm"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo "rtm - Realtime Markdown Viewer"
echo "================================"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)  OS_TAG="linux" ;;
    darwin) OS_TAG="macos" ;;
    *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64)  ARCH_TAG="amd64" ;;
    aarch64|arm64) ARCH_TAG="arm64" ;;
    *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ARTIFACT="rtm-${OS_TAG}-${ARCH_TAG}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"

echo "Downloading ${ARTIFACT}..."
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

if command -v curl > /dev/null 2>&1; then
    curl -fSL -o "$TMP" "$DOWNLOAD_URL"
elif command -v wget > /dev/null 2>&1; then
    wget -qO "$TMP" "$DOWNLOAD_URL"
else
    echo "Error: curl or wget required"
    exit 1
fi

chmod +x "$TMP"

if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP" "$INSTALL_DIR/$BINARY_NAME"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "$TMP" "$INSTALL_DIR/$BINARY_NAME"
fi

echo ""
echo "Installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "Usage:  rtm [directory]"
echo "Config: ~/.rtmrc"
echo "Help:   rtm --help"
