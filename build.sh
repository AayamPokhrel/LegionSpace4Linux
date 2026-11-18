#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "=========================================================="
echo " Building Lenovo Legion CLI Utility (Release Mode)"
echo "=========================================================="

if ! command -v cargo &>/dev/null; then
    echo "Cargo not found in current PATH. Attempting nix-shell environment..."
    nix-shell -p cargo rustc --run "cd legion-cli && cargo build --release"
else
    cd legion-cli
    cargo build --release
    cd ..
fi

echo ""
echo "Build succeeded!"
echo "Binaries available at:"
echo "  - ${SCRIPT_DIR}/legion-cli/target/release/legion-cli"
echo "  - ${SCRIPT_DIR}/legion-cli/target/release/legionctl"
echo "=========================================================="

