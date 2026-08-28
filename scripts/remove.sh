#!/bin/bash

set -e

TARGET="/usr/bin/virus"

if [ -f "$TARGET" ]; then
    echo "Usuwanie $TARGET..."
    sudo rm -f "$TARGET"
    echo "Plik został pomyślnie usunięty."
else
    echo "Plik $TARGET nie istnieje w systemie."
fi
