#!/usr/bin/env bash
# ARCHIVED (S169): This demo references JSON-RPC methods no longer in toadStool.
# See showcase/00-local-primal/ for current demos.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
echo "Building demo..."
cargo build --release 2>/dev/null
echo ""
cargo run --release
