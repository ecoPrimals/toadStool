#!/usr/bin/env bash
# ToadStool Biological Computing - Full Demonstration Suite
set -euo pipefail

SHOWCASE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOADSTOOL_ROOT="$(cd "$SHOWCASE_ROOT/.." && pwd)"
CLI="$TOADSTOOL_ROOT/target/release/toadstool-cli"

# Build if needed
if [ ! -f "$CLI" ]; then
    echo "Building toadstool-cli..."
    (cd "$TOADSTOOL_ROOT" && cargo build --release --bin toadstool-cli)
fi

export TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED=1

cat << 'EOF'

╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║       🍄 TOADSTOOL BIOLOGICAL COMPUTING SHOWCASE 🍄              ║
║                                                                   ║
║   "Run anything, anywhere" meets biological self-organization     ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝

Welcome to ToadStool's biological computing demonstrations!

These demos show REAL workload execution using biology-inspired patterns:
  • Cell Division: Recursive spawning
  • Swarm Intelligence: Parallel cooperation
  • Immune Response: Adaptive self-healing

All powered by ToadStool's RuntimeOrchestrator - NO SIMULATION!

EOF

echo "Press ENTER to start..."
read

# Demo 1: Cell Division
cat << 'EOF'

═══════════════════════════════════════════════════════════════════
DEMO 1: 🧬 CELL DIVISION
═══════════════════════════════════════════════════════════════════

Pattern: MITOSIS (Cell replication)
What it shows: ToadStool spawning child ToadStools recursively

A parent workload spawns 2 daughter workloads, each capable of
further division. This demonstrates hierarchical workload trees!

EOF

echo "Press ENTER to run cell division..."
read

"$CLI" execute "$SHOWCASE_ROOT/workloads/cell-division.toml"

echo ""
echo "✅ Cell division complete!"
echo ""
echo "Press ENTER for next demo..."
read

# Demo 2: Swarm Intelligence
cat << 'EOF'

═══════════════════════════════════════════════════════════════════
DEMO 2: 🐜 SWARM INTELLIGENCE  
═══════════════════════════════════════════════════════════════════

Pattern: ANT COLONY OPTIMIZATION
What it shows: Parallel ToadStool workers cooperating via shared state

5 worker workloads execute in parallel, communicating through a
"pheromone map" (shared state file). Emergent behavior from simple agents!

EOF

echo "Press ENTER to run swarm intelligence..."
read

"$CLI" execute "$SHOWCASE_ROOT/workloads/swarm-intelligence.toml"

echo ""
echo "✅ Swarm intelligence complete!"
echo ""
echo "Press ENTER for next demo..."
read

# Demo 3: Immune Response
cat << 'EOF'

═══════════════════════════════════════════════════════════════════
DEMO 3: 🛡️  IMMUNE RESPONSE
═══════════════════════════════════════════════════════════════════

Pattern: ADAPTIVE IMMUNE SYSTEM
What it shows: Self-healing through dynamic defender spawning

System detects threats and spawns specialized "defender" workloads
to neutralize each threat. Just like T-cells attacking pathogens!

EOF

echo "Press ENTER to run immune response..."
read

"$CLI" execute "$SHOWCASE_ROOT/workloads/immune-response.toml"

echo ""
echo "✅ Immune response complete!"
echo ""

# Final summary
cat << 'EOF'

╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║              🎉 BIOLOGICAL COMPUTING SHOWCASE COMPLETE! 🎉       ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝

What you just witnessed:

✅ RECURSIVE SPAWNING (Cell Division)
   ToadStool workloads spawning child workloads dynamically

✅ PARALLEL COOPERATION (Swarm Intelligence)
   Multiple ToadStools coordinating through shared state

✅ ADAPTIVE RESPONSE (Immune System)
   Self-healing through dynamic workload generation

All using ToadStool's core RuntimeOrchestrator!

═══════════════════════════════════════════════════════════════════

🧬 BIOLOGICAL COMPUTING PRINCIPLES DEMONSTRATED:

1. SELF-ORGANIZATION
   Workloads organize themselves without central control

2. EMERGENCE
   Complex behavior arising from simple rules

3. ADAPTATION
   System responds dynamically to changing conditions

4. COOPERATION
   Multiple agents working toward common goals

5. RECURSION
   Structures spawning similar structures

═══════════════════════════════════════════════════════════════════

This is what happens when you model computing on biology!

ToadStool: "Run anything, anywhere" + "Self-organizing systems"
         = Universal Biological Computing Platform

🚀 Ready for production. Ready for the future.

EOF

