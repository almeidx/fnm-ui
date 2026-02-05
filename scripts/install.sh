#!/bin/bash

set -e

OS="$(uname -s)"

case "$OS" in
    Darwin)
        APP_NAME="Versi.app"
        INSTALL_DIR="/Applications"

        # Find the app bundle - check current directory and script directory
        SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
        APP_PATH=""

        if [ -d "./$APP_NAME" ]; then
            APP_PATH="./$APP_NAME"
        elif [ -d "$SCRIPT_DIR/$APP_NAME" ]; then
            APP_PATH="$SCRIPT_DIR/$APP_NAME"
        else
            echo "Error: Cannot find $APP_NAME"
            echo "Please run this script from the directory containing $APP_NAME"
            exit 1
        fi

        echo "Installing Versi..."

        # Remove quarantine attribute
        echo "Removing quarantine attribute..."
        xattr -cr "$APP_PATH"

        # Check if already installed
        if [ -d "$INSTALL_DIR/$APP_NAME" ]; then
            echo "Existing installation found. Replacing..."
            rm -rf "$INSTALL_DIR/$APP_NAME"
        fi

        # Move to Applications
        echo "Moving to $INSTALL_DIR..."
        mv "$APP_PATH" "$INSTALL_DIR/"

        echo ""
        echo "Installation complete!"
        echo "You can now launch Versi from your Applications folder."
        ;;
    Linux)
        BIN_DIR="$HOME/.local/bin"
        APPS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
        ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"

        SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

        BINARY=""
        if [ -f "./versi" ]; then
            BINARY="./versi"
        elif [ -f "$SCRIPT_DIR/versi" ]; then
            BINARY="$SCRIPT_DIR/versi"
        else
            echo "Error: Cannot find versi binary"
            echo "Please run this script from the directory containing the versi binary"
            exit 1
        fi

        DESKTOP_FILE=""
        for dir in "." "$SCRIPT_DIR"; do
            if [ -f "$dir/versi.desktop" ]; then
                DESKTOP_FILE="$dir/versi.desktop"
                break
            fi
        done

        ICON_FILE=""
        for dir in "." "$SCRIPT_DIR"; do
            if [ -f "$dir/versi.png" ]; then
                ICON_FILE="$dir/versi.png"
                break
            fi
        done

        echo "Installing Versi..."

        mkdir -p "$BIN_DIR"
        cp "$BINARY" "$BIN_DIR/versi"
        chmod +x "$BIN_DIR/versi"
        echo "Installed binary to $BIN_DIR/versi"

        if [ -n "$DESKTOP_FILE" ]; then
            mkdir -p "$APPS_DIR"
            cp "$DESKTOP_FILE" "$APPS_DIR/versi.desktop"
            echo "Installed desktop entry to $APPS_DIR/versi.desktop"
        fi

        if [ -n "$ICON_FILE" ]; then
            mkdir -p "$ICON_DIR"
            cp "$ICON_FILE" "$ICON_DIR/versi.png"
            echo "Installed icon to $ICON_DIR/versi.png"
        fi

        echo ""
        echo "Installation complete!"
        if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
            echo "Note: $BIN_DIR is not in your PATH."
            echo "Add it with: export PATH=\"\$HOME/.local/bin:\$PATH\""
        fi
        ;;
    *)
        echo "Error: Unsupported OS: $OS"
        exit 1
        ;;
esac
