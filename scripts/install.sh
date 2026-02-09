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
            if [ -f "$dir/dev.almeidx.versi.desktop" ]; then
                DESKTOP_FILE="$dir/dev.almeidx.versi.desktop"
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
            cp "$DESKTOP_FILE" "$APPS_DIR/dev.almeidx.versi.desktop"
            echo "Installed desktop entry to $APPS_DIR/dev.almeidx.versi.desktop"

            # Remove old desktop entry from previous installations
            rm -f "$APPS_DIR/versi.desktop"

            # Update desktop database if available
            if command -v update-desktop-database >/dev/null 2>&1; then
                update-desktop-database "$APPS_DIR" 2>/dev/null || true
            fi
        fi

        if [ -n "$ICON_FILE" ]; then
            mkdir -p "$ICON_DIR"
            cp "$ICON_FILE" "$ICON_DIR/versi.png"
            echo "Installed icon to $ICON_DIR/versi.png"

            # Ensure the hicolor icon theme index exists so desktop environments
            # can discover icons in the user-local directory
            ICON_BASE="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
            if [ ! -f "$ICON_BASE/index.theme" ] && [ -f /usr/share/icons/hicolor/index.theme ]; then
                cp /usr/share/icons/hicolor/index.theme "$ICON_BASE/index.theme"
            fi

            # Update icon cache if gtk-update-icon-cache is available
            if command -v gtk-update-icon-cache >/dev/null 2>&1; then
                gtk-update-icon-cache --force "$ICON_BASE" 2>/dev/null || true
            fi
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
