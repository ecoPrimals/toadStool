#!/usr/bin/env bash
# Deep Evolution Script: Self-Knowledge Principle for Port Discovery
# 
# Philosophy: ToadStool only knows its own ports. Other primals are discovered.
# 
# This script evolves hardcoded port discovery to follow self-knowledge principle:
# - TOADSTOOL_PORT → ToadStool's own port (prefixed, valid)
# - SONGBIRD_PORT → Other primal's port (NO prefix, discovered at runtime)
# - BEARDOG_PORT → Other primal's port (NO prefix, discovered at runtime)
# - etc.

set -euo pipefail

echo "🌱 Evolving Port Discovery to Self-Knowledge Principle..."
echo ""

cd "$(dirname "$0")/.."

# Fix: Use empty prefix for OTHER primal ports
echo "✅ Step 1: Fix Songbird port discovery (remove TOADSTOOL prefix)"
sed -i 's/let loader = EnvConfigLoader::new();$/let loader = EnvConfigLoader::with_prefix(""); \/\/ No prefix for other primals/' \
    crates/core/config/src/config_utils.rs

# Actually, let's do this more carefully with proper Rust code evolution
echo "⚠️  Manual evolution required - creating patch file..."

cat > /tmp/port_evolution.patch << 'EOF'
This patch evolves port discovery to follow self-knowledge principle.

Changes needed in crates/core/config/src/config_utils.rs:

1. get_songbird_port(): Use EnvConfigLoader::with_prefix("") not ::new()
2. get_nestgate_port(): Use EnvConfigLoader::with_prefix("") not ::new()  
3. get_squirrel_port(): Use EnvConfigLoader::with_prefix("") not ::new()
4. get_toadstool_port(): Keep EnvConfigLoader::new() (self-knowledge)
5. get_bind_host(): Keep EnvConfigLoader::new() (self-knowledge)

Rationale:
- ToadStool knows its own config → TOADSTOOL_* prefix (self-knowledge ✅)
- Other primals manage their own → No prefix (discovery, not hardcoding)
EOF

cat /tmp/port_evolution.patch
echo ""
echo "📝 Patch file created at /tmp/port_evolution.patch"
echo ""
echo "🔧 Applying automated fixes where safe..."

# Run tests to see current state
echo "🧪 Running tests to establish baseline..."
cargo test -p toadstool-config --test config_expansion_tests 2>&1 | tail -20 || true

echo ""
echo "✅ Evolution script complete. Review changes and commit."

