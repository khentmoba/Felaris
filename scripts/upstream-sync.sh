#!/usr/bin/env bash
# Veluno upstream sync & conflict checker wrapper
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$DIR/upstream_sync.py" "$@"
