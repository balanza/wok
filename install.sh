#!/bin/bash
set -e

# Detect platform
PLATFORM=""
ARCH=$(uname -m)
OS=$(uname -s)

if [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        PLATFORM="x86_64-unknown-linux-musl"
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
        PLATFORM="aarch64-unknown-linux-musl"
    fi
elif [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        PLATFORM="x86_64-apple-darwin"
    elif [ "$ARCH" = "arm64" ]; then
        PLATFORM="aarch64-apple-darwin"
    fi
fi

if [ -z "$PLATFORM" ]; then
    echo "Error: Unsupported platform: $OS $ARCH"
    echo "Supported platforms:"
    echo "  - Linux x86_64"
    echo "  - Linux ARM64 (aarch64)"
    echo "  - macOS Intel (x86_64)"
    echo "  - macOS Apple Silicon (arm64)"
    exit 1
fi

echo "Installing wok for $PLATFORM..."

# Download binary
BINARY_NAME="wok-$PLATFORM"
URL="https://github.com/balanza/wok/releases/latest/download/$BINARY_NAME"

echo "Downloading from $URL..."
if ! curl -LO "$URL"; then
    echo "Error: Failed to download binary from $URL"
    echo "Please check your internet connection and try again."
    exit 1
fi

# Make executable
chmod +x "$BINARY_NAME"

# Install to /usr/local/bin
INSTALL_DIR="/usr/local/bin"
if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY_NAME" "$INSTALL_DIR/wok"
    echo "Installed wok to $INSTALL_DIR/wok"
else
    echo "Need sudo permission to install to $INSTALL_DIR"
    if sudo mv "$BINARY_NAME" "$INSTALL_DIR/wok"; then
        echo "Installed wok to $INSTALL_DIR/wok"
    else
        echo "Error: Failed to install wok to $INSTALL_DIR"
        echo "You can manually move $BINARY_NAME to a directory in your PATH"
        exit 1
    fi
fi

echo ""
echo "wok installed successfully!"
echo "Run 'wok --help' to get started"
