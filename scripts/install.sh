#!/bin/bash

set -e

URL="https://github.com/HackerOS-Linux-System/HackerScript/releases/download/v0.4/hackerc"
TARGET_DIR="/usr/bin"
TEMP_FILE=$(mktemp)

echo "Pobieranie pliku..."
curl -sSL "$URL" -o "$TEMP_FILE"

echo "Nadawanie uprawnień do wykonywania..."
chmod +x "$TEMP_FILE"

echo "Instalacja w $TARGET_DIR/..."
sudo mv "$TEMP_FILE" "$TARGET_DIR/hackerc"

echo "Instalacja zakończona pomyślnie."
