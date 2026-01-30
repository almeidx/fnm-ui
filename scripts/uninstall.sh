#!/bin/bash

set -e

OS="$(uname -s)"

case "$OS" in
    Darwin)
        APP_PATH="/Applications/Versi.app"
        CONFIG_DIR="$HOME/Library/Application Support/versi"
        CACHE_DIR="$HOME/Library/Caches/versi"

        echo "Uninstalling Versi..."

        if [ -d "$APP_PATH" ]; then
            rm -rf "$APP_PATH"
            echo "Removed $APP_PATH"
        else
            echo "App not found at $APP_PATH (skipped)"
        fi

        if [ -d "$CONFIG_DIR" ]; then
            rm -rf "$CONFIG_DIR"
            echo "Removed $CONFIG_DIR"
        fi

        if [ -d "$CACHE_DIR" ]; then
            rm -rf "$CACHE_DIR"
            echo "Removed $CACHE_DIR"
        fi

        echo ""
        echo "Versi has been uninstalled."
        ;;
    Linux)
        CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/versi"
        CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/versi"
        DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/versi"

        echo "Uninstalling Versi..."

        if [ -d "$CONFIG_DIR" ]; then
            rm -rf "$CONFIG_DIR"
            echo "Removed $CONFIG_DIR"
        fi

        if [ -d "$CACHE_DIR" ]; then
            rm -rf "$CACHE_DIR"
            echo "Removed $CACHE_DIR"
        fi

        if [ -d "$DATA_DIR" ]; then
            rm -rf "$DATA_DIR"
            echo "Removed $DATA_DIR"
        fi

        echo ""
        echo "Versi data has been removed."
        echo "If you installed the binary manually, remove it from your PATH."
        ;;
    *)
        echo "Error: Unsupported OS: $OS"
        exit 1
        ;;
esac
