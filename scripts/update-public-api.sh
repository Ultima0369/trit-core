#!/usr/bin/env bash
# Update the public API snapshot used by CI.
#
# Usage:
#   ./scripts/update-public-api.sh
#
# The generated snapshot is written to api/public-api.txt.
# Run this script whenever you intentionally change the public API
# and want CI to enforce the new baseline.

set -euo pipefail

cd "$(dirname "$0")/.."

cargo public-api -ss --all-features > api/public-api.txt

echo "Updated api/public-api.txt ($(wc -l < api/public-api.txt) lines)"
