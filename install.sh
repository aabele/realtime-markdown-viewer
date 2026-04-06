#!/bin/sh
set -e

REPO="https://github.com/aabele/realtime-markdown-viewer.git"
BINARY_NAME="rtm"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
TMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "rtm - Realtime Markdown Viewer"
echo "================================"

if ! command -v cargo > /dev/null 2>&1; then
    echo "Error: Rust toolchain not found."
    echo "Install it first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "Cloning..."
git clone --depth 1 "$REPO" "$TMP_DIR/rtm" 2>&1 | tail -1

echo "Building (release)..."
cd "$TMP_DIR/rtm"
cargo build --release --quiet

strip "target/release/$BINARY_NAME" 2>/dev/null || true

SIZE=$(du -h "target/release/$BINARY_NAME" | cut -f1)

if [ -w "$INSTALL_DIR" ]; then
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
fi

echo ""
echo "Installed $BINARY_NAME ($SIZE) to $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "Usage:  rtm <directory>"
echo "Config: ~/.rtmrc"
echo "Help:   rtm --help"
