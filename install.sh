#!/bin/sh
set -e

# Romance CLI installer
# Usage: curl -sSL https://romance.dev/install | sh

REPO="romance-dev/romance"
INSTALL_DIR="/usr/local/bin"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
    linux) TARGET="${ARCH}-unknown-linux-gnu" ;;
    darwin) TARGET="${ARCH}-apple-darwin" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Get latest version
VERSION=$(curl -sSL "https://api.romance.dev/v1/releases/latest" | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
    echo "Failed to fetch latest version"
    exit 1
fi

# Download
URL="https://releases.romance.dev/v${VERSION}/romance-${TARGET}.tar.gz"
echo "Downloading Romance CLI v${VERSION} for ${TARGET}..."
curl -sSL "$URL" -o /tmp/romance.tar.gz
tar -xzf /tmp/romance.tar.gz -C /tmp

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv /tmp/romance "$INSTALL_DIR/romance"
else
    sudo mv /tmp/romance "$INSTALL_DIR/romance"
fi

# Clean up
rm -f /tmp/romance.tar.gz

echo ""
echo "Romance CLI v${VERSION} installed successfully!"
echo ""
echo "Get started:"
echo "  romance activate <your-license-key>"
echo "  romance new my-app"
echo ""
