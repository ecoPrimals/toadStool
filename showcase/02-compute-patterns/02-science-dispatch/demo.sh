#!/usr/bin/env bash
# NOTE (S169): The demo program invoked below still references JSON-RPC methods removed in S169
# (e.g. science.gpu.*). This showcase needs updating to match the current server API.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
echo "Building demo..."
cargo build --release 2>/dev/null
echo ""
cargo run --release
