#!/bin/bash

# ===================================================================
# NestGate ↔ ToadStool: Distributed Storage & Compute
# ===================================================================
# 
# What this demonstrates:
# - Distributed data storage across multiple nodes
# - ToadStool workloads access distributed data
# - Load balancing and replication
# - Multi-node coordination
# - Resilient data access
#
# Prerequisites:
# - NestGate nodes (or demo mode)
# - ToadStool nodes (or demo mode)
#
# ===================================================================

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Configuration
DEMO_MODE=true
NESTGATE_NODES=("node1:8080" "node2:8080" "node3:8080")
TOADSTOOL_NODES=("node1:3000" "node2:3000")

echo ""
echo "====================================================================="
echo "  NestGate ↔ ToadStool: Distributed Storage & Compute"
echo "====================================================================="
echo ""

# Step 1: Prerequisites check
echo "Step 1: Checking prerequisites..."
if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO MODE] Simulating multi-node environment...${NC}"
else
    # Check actual nodes
    for node in "${NESTGATE_NODES[@]}"; do
        if ! curl -s "http://$node/health" > /dev/null 2>&1; then
            echo "   ⚠️  NestGate $node not available, switching to demo mode"
            DEMO_MODE=true
            break
        fi
    done
fi
echo -e "${GREEN}   ✅ Prerequisites checked${NC}"

# Step 2: Show distributed topology
echo ""
echo "Step 2: Distributed system topology..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │        DISTRIBUTED STORAGE TOPOLOGY                  │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "                    User/Application"
echo "                           │"
echo "                           │"
echo "             ┌─────────────┼─────────────┐"
echo "             │             │             │"
echo "             ↓             ↓             ↓"
echo "       🗄️  NestGate   🗄️  NestGate   🗄️  NestGate"
echo "         Node 1        Node 2        Node 3"
echo "       (Primary)     (Replica)     (Replica)"
echo "             │             │             │"
echo "             └─────────────┼─────────────┘"
echo "                           │"
echo "                    Data replicated"
echo "                  across all nodes"
echo "                           │"
echo "             ┌─────────────┼─────────────┐"
echo "             │                           │"
echo "             ↓                           ↓"
echo "       🍄 ToadStool                🍄 ToadStool"
echo "         Node 1                      Node 2"
echo "      (Compute A)                 (Compute B)"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Topology:"
    echo "      - NestGate nodes: 3 (replicated)"
    echo "      - ToadStool nodes: 2 (compute)"
    echo "      - Replication factor: 3x"
    echo "      - Load balancing: Round-robin"
fi

# Step 3: Store dataset with replication
echo ""
echo "Step 3: Storing large dataset with replication..."
echo "   Dataset: training_data_large.parquet"
echo "   Size: 10GB"
echo "   Replication: 3x (all nodes)"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Uploading to distributed NestGate...${NC}"
    
    # Simulate chunked upload
    for chunk in $(seq 1 5); do
        echo -ne "      Chunk $chunk/5: Uploading...\r"
        sleep 0.3
    done
    echo "      Chunk 5/5: Complete!      "
    
    sleep 0.5
    
    echo ""
    echo -e "${GREEN}   ✅ Dataset stored and replicated${NC}"
    echo "   📦 Primary: NestGate Node 1"
    echo "   🔄 Replica 1: NestGate Node 2"
    echo "   🔄 Replica 2: NestGate Node 3"
    echo "   📊 Distribution:"
    echo "      - Node 1: Chunks 1,2"
    echo "      - Node 2: Chunks 3,4"
    echo "      - Node 3: Chunk 5 + backup"
fi

# Step 4: ToadStool discovers distributed data
echo ""
echo "Step 4: ToadStool discovering distributed data..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying distributed storage...${NC}"
    sleep 0.5
    
    echo -e "${GREEN}   ✅ Data locations discovered${NC}"
    echo "   📍 Available nodes:"
    echo "      - NestGate Node 1: Chunks 1,2 (10ms latency)"
    echo "      - NestGate Node 2: Chunks 3,4 (12ms latency)"
    echo "      - NestGate Node 3: Chunk 5 (15ms latency)"
    echo ""
    echo "   🎯 Selected: Node 1 (lowest latency)"
fi

# Step 5: Distributed compute workload
echo ""
echo "Step 5: Running distributed compute workload..."
echo "   Task: Train ML model on distributed data"
echo "   Strategy: Data-local computation"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${CYAN}   🍄 ToadStool Node 1: Processing chunks 1,2${NC}"
    sleep 0.3
    echo "      Loading from local NestGate Node 1..."
    echo "      Training on 4GB data..."
    sleep 0.4
    echo "      Partial results: Loss=1.5, Acc=75%"
    echo ""
    
    echo -e "${CYAN}   🍄 ToadStool Node 2: Processing chunks 3,4${NC}"
    sleep 0.3
    echo "      Loading from NestGate Node 2..."
    echo "      Training on 4GB data..."
    sleep 0.4
    echo "      Partial results: Loss=1.6, Acc=73%"
    echo ""
    
    echo -e "${GREEN}   ✅ Distributed compute complete${NC}"
    echo "   📊 Aggregate results:"
    echo "      - Total data processed: 10GB"
    echo "      - Nodes used: 2"
    echo "      - Average loss: 1.55"
    echo "      - Average accuracy: 74%"
    echo "      - Total time: 45s (vs 120s single-node)"
    echo "      - Speedup: 2.67x"
fi

# Step 6: Store results back to distributed storage
echo ""
echo "Step 6: Storing results to distributed storage..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Storing trained model...${NC}"
    sleep 0.5
    
    echo -e "${GREEN}   ✅ Model stored with replication${NC}"
    echo "   📦 Primary: NestGate Node 1 (model.pth, 500MB)"
    echo "   🔄 Replica 1: NestGate Node 2 (syncing...)"
    echo "   🔄 Replica 2: NestGate Node 3 (syncing...)"
    sleep 0.3
    echo "   ✅ Replication complete (3x)"
fi

# Step 7: Demonstrate load balancing
echo ""
echo "Step 7: Load balancing across storage nodes..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Simulating 10 concurrent reads:"
    echo ""
    
    for req in $(seq 1 10); do
        NODE=$((1 + RANDOM % 3))
        LATENCY=$((8 + RANDOM % 10))
        echo "      Request $req → NestGate Node $NODE (${LATENCY}ms)"
        sleep 0.1
    done
    
    echo ""
    echo -e "${GREEN}   ✅ Load distributed across all nodes${NC}"
    echo "   📊 Distribution:"
    echo "      - Node 1: 4 requests (40%)"
    echo "      - Node 2: 3 requests (30%)"
    echo "      - Node 3: 3 requests (30%)"
fi

# Step 8: Demonstrate failover
echo ""
echo "Step 8: Failover demonstration..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${CYAN}   Simulating Node 2 failure...${NC}"
    sleep 0.5
    
    echo "   ⚠️  NestGate Node 2: OFFLINE"
    echo ""
    echo -e "${YELLOW}   [DEMO] ToadStool attempting to read data...${NC}"
    sleep 0.3
    
    echo "   🔍 Primary (Node 2): FAILED"
    echo "   🔄 Failover to replica (Node 1): SUCCESS"
    echo ""
    echo -e "${GREEN}   ✅ Data access maintained${NC}"
    echo "   📊 No data loss, no service interruption"
    echo ""
    echo -e "${CYAN}   Node 2 back online${NC}"
    echo "   🔄 Replication sync in progress..."
    sleep 0.5
    echo "   ✅ System fully healthy"
fi

# Step 9: Visualize complete workflow
echo ""
echo "Step 9: Distributed storage workflow..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │     DISTRIBUTED STORAGE & COMPUTE FLOW               │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "                     User uploads 10GB dataset"
echo "                               │"
echo "                   1. Store with replication"
echo "                               ↓"
echo "                  ┌────────────┴────────────┐"
echo "                  │                         │"
echo "            🗄️  NestGate            🗄️  NestGate"
echo "             Node 1 (4GB)            Node 2 (4GB)"
echo "            Primary copy            Replica copy"
echo "                  │                         │"
echo "       ┌──────────┴──────────┐             │"
echo "       │                     │             │"
echo " 2. ToadStool         2. ToadStool         │"
echo "    reads local          reads local       │"
echo "       │                     │             │"
echo "       ↓                     ↓             ↓"
echo " 🍄 ToadStool          🍄 ToadStool   🗄️  NestGate"
echo "   Node 1                Node 2         Node 3"
echo " Process 4GB           Process 4GB    Backup (2GB)"
echo "       │                     │"
echo "       └──────────┬──────────┘"
echo "                  │"
echo "       3. Aggregate results"
echo "                  ↓"
echo "           Combined model"
echo "                  │"
echo "    4. Store back (replicated)"
echo "                  ↓"
echo "         All NestGate nodes"
echo ""

# Step 10: Performance comparison
echo "Step 10: Performance comparison..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Single-Node vs Distributed:"
    echo ""
    echo "   ┌─────────────────┬──────────────┬──────────────┐"
    echo "   │    Metric       │ Single-Node  │ Distributed  │"
    echo "   ├─────────────────┼──────────────┼──────────────┤"
    echo "   │ Data Size       │    10 GB     │    10 GB     │"
    echo "   │ Processing Time │    120 s     │     45 s     │"
    echo "   │ Network I/O     │   10 GB      │    2 GB      │"
    echo "   │ Speedup         │     1.0x     │   2.67x 🚀   │"
    echo "   │ Availability    │    99.0%     │   99.99%     │"
    echo "   │ Resilience      │     Low      │    High ✅   │"
    echo "   └─────────────────┴──────────────┴──────────────┘"
    echo ""
    echo "   🎯 Key advantages:"
    echo "      • Data locality: Compute runs where data lives"
    echo "      • Parallel processing: Multiple nodes simultaneously"
    echo "      • Reduced network: Only results transferred"
    echo "      • High availability: Automatic failover"
fi

# Step 11: Show configuration
echo ""
echo "Step 11: Distributed storage configuration..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 Configuration:"
    echo ""
    echo "   {"
    echo "     \"storage_config\": {"
    echo "       \"nodes\": 3,"
    echo "       \"replication_factor\": 3,"
    echo "       \"consistency\": \"eventual\","
    echo "       \"chunk_size_mb\": 64"
    echo "     },"
    echo "     \"compute_config\": {"
    echo "       \"strategy\": \"data_local\","
    echo "       \"load_balancing\": \"round_robin\","
    echo "       \"failover\": \"automatic\""
    echo "     },"
    echo "     \"network_config\": {"
    echo "       \"topology\": \"mesh\","
    echo "       \"discovery\": \"mdns\","
    echo "       \"compression\": true"
    echo "     }"
    echo "   }"
fi

# Step 12: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ Distributed data storage (3 nodes)"
echo "  ✅ Data replication (3x redundancy)"
echo "  ✅ Data-local compute (minimize network)"
echo "  ✅ Load balancing (round-robin)"
echo "  ✅ Automatic failover (high availability)"
echo "  ✅ Bidirectional workflow (NestGate ↔ ToadStool)"
echo ""
echo "Key benefits:"
echo "  🚀 Performance: 2.67x faster (parallel processing)"
echo "  📡 Efficiency: 80% less network I/O"
echo "  🛡️  Resilience: Automatic failover"
echo "  ♾️  Scalability: Add nodes linearly"
echo "  🎯 Locality: Compute runs near data"
echo ""
echo "Distributed patterns:"
echo "  • Data-Local Compute: Process where data lives"
echo "  • Replication: Multiple copies for availability"
echo "  • Load Balancing: Distribute work evenly"
echo "  • Failover: Automatic recovery from failures"
echo ""
echo "Real-world use cases:"
echo "  📊 Big Data: Process large datasets efficiently"
echo "  🎓 Research: Distributed ML training"
echo "  🌐 CDN: Content delivery with edge caching"
echo "  🏢 Enterprise: High-availability data services"
echo ""
echo "Architecture benefits:"
echo "  🔄 Bidirectional: ToadStool reads and writes to NestGate"
echo "  🌐 Distributed: Scale horizontally"
echo "  🛡️  Resilient: Survive node failures"
echo "  ⚡ Fast: Parallel processing + data locality"
echo ""
echo "Next steps:"
echo "  - Try: 03-capability-based demo (advanced discovery)"
echo "  - Explore: Level 3 multi-primal (complete ecosystem)"
echo "  - Learn: Songbird orchestration patterns"
echo ""

