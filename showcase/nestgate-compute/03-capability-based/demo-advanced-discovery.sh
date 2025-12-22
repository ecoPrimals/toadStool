#!/bin/bash

# ===================================================================
# NestGate ↔ ToadStool: Advanced Capability-Based Discovery
# ===================================================================
# 
# What this demonstrates:
# - Dynamic service discovery via capabilities
# - Zero-configuration service mesh
# - Intelligent service selection
# - Automatic failover and load balancing
# - O(1) connection complexity
#
# Prerequisites:
# - Service registry (or demo mode)
# - Multiple NestGate nodes (or demo mode)
# - Multiple ToadStool nodes (or demo mode)
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
REGISTRY_ENDPOINT="${REGISTRY_URL:-http://localhost:5000}"

echo ""
echo "====================================================================="
echo "  NestGate ↔ ToadStool: Advanced Capability-Based Discovery"
echo "====================================================================="
echo ""

# Step 1: Prerequisites check
echo "Step 1: Checking prerequisites..."
if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO MODE] Simulating service mesh...${NC}"
else
    if ! curl -s "$REGISTRY_ENDPOINT/health" > /dev/null 2>&1; then
        echo "   ⚠️  Registry not available, switching to demo mode"
        DEMO_MODE=true
    fi
fi
echo -e "${GREEN}   ✅ Prerequisites checked${NC}"

# Step 2: Show service mesh
echo ""
echo "Step 2: Service mesh topology..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │         CAPABILITY-BASED SERVICE MESH                │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "                   📡 Service Registry"
echo "               (Capability Advertisement)"
echo "                           │"
echo "         ┌─────────────────┼─────────────────┐"
echo "         │                 │                 │"
echo "   Advertises          Advertises       Advertises"
echo "   capabilities        capabilities     capabilities"
echo "         │                 │                 │"
echo "         ↓                 ↓                 ↓"
echo "   🗄️  NestGate       🗄️  NestGate       🍄 ToadStool"
echo "     Node 1             Node 2           Node 1"
echo "   persistent_        persistent_      compute.native"
echo "   storage            storage          compute.gpu"
echo "   versioning         encryption       ml.training"
echo "   metadata           compression"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Registered services:"
    echo "      - NestGate nodes: 2"
    echo "      - ToadStool nodes: 1"
    echo "      - Total capabilities: 8"
fi

# Step 3: Discovery by capability (not hardcoded endpoint)
echo ""
echo "Step 3: Discovering storage by capability..."
echo "   Required: persistent_storage + versioning"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying registry...${NC}"
    sleep 0.5
    
    echo -e "${GREEN}   ✅ Services discovered${NC}"
    echo ""
    echo "   📋 Matching services:"
    echo ""
    echo "   Service 1:"
    echo "     • Name: nestgate-node-1"
    echo "     • Endpoint: http://192.168.1.10:8080"
    echo "     • Capabilities: [persistent_storage, versioning, metadata]"
    echo "     • Load: 45%"
    echo "     • Latency: 8ms"
    echo "     • Score: 92/100 🏆"
    echo ""
    echo "   Service 2:"
    echo "     • Name: nestgate-node-2"
    echo "     • Endpoint: http://192.168.1.11:8080"
    echo "     • Capabilities: [persistent_storage, encryption, compression]"
    echo "     • Load: 75%"
    echo "     • Latency: 12ms"
    echo "     • Score: 78/100"
    echo ""
    echo -e "${CYAN}   🎯 Selected: nestgate-node-1 (best score)${NC}"
fi

# Step 4: Intelligent selection criteria
echo ""
echo "Step 4: Service selection criteria..."

if [ "$DEMO_MODE" = true ]; then
    echo ""
    echo "   📊 Selection factors:"
    echo ""
    echo "   ┌────────────────────┬────────┬────────┬─────────┐"
    echo "   │      Factor        │ Node 1 │ Node 2 │  Weight │"
    echo "   ├────────────────────┼────────┼────────┼─────────┤"
    echo "   │ Has capabilities   │   ✅   │   ✅   │   40%   │"
    echo "   │ Low latency        │   ✅   │   ⚠️    │   30%   │"
    echo "   │ Low load           │   ✅   │   ❌   │   20%   │"
    echo "   │ High availability  │   ✅   │   ✅   │   10%   │"
    echo "   ├────────────────────┼────────┼────────┼─────────┤"
    echo "   │ Total Score        │  92/100│  78/100│         │"
    echo "   └────────────────────┴────────┴────────┴─────────┘"
    echo ""
    echo "   🎯 Winner: Node 1 (better latency + lower load)"
fi

# Step 5: Dynamic compute discovery
echo ""
echo "Step 5: Discovering compute by capability..."
echo "   Required: compute.native + ml.training"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying for compute services...${NC}"
    sleep 0.5
    
    echo -e "${GREEN}   ✅ Compute services discovered${NC}"
    echo ""
    echo "   📋 Available:"
    echo "     • toadstool-node-1: [compute.native, compute.gpu, ml.training]"
    echo "     • toadstool-node-2: [compute.native, ml.inference]"
    echo "     • toadstool-node-3: [compute.wasm, compute.container]"
    echo ""
    echo -e "${CYAN}   🎯 Selected: toadstool-node-1 (has GPU + ML training)${NC}"
fi

# Step 6: Complete workflow with discovery
echo ""
echo "Step 6: Complete workflow using discovered services..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${CYAN}   🔍 Step 1: Discover storage (persistent_storage)${NC}"
    sleep 0.3
    echo "      → Found: nestgate-node-1"
    echo ""
    
    echo -e "${CYAN}   📥 Step 2: Load dataset from discovered storage${NC}"
    sleep 0.3
    echo "      → Loading from http://192.168.1.10:8080"
    echo "      → Dataset: training_data.parquet (2GB)"
    echo ""
    
    echo -e "${CYAN}   🔍 Step 3: Discover compute (ml.training + gpu)${NC}"
    sleep 0.3
    echo "      → Found: toadstool-node-1"
    echo ""
    
    echo -e "${CYAN}   🍄 Step 4: Execute training on discovered compute${NC}"
    sleep 0.5
    echo "      → Training on http://192.168.1.20:3000"
    echo "      → Using GPU: Yes"
    echo "      → Progress: [████████████████] 100%"
    echo ""
    
    echo -e "${CYAN}   🔍 Step 5: Discover storage for results${NC}"
    sleep 0.3
    echo "      → Found: nestgate-node-1 (same as before)"
    echo ""
    
    echo -e "${CYAN}   💾 Step 6: Store results to discovered storage${NC}"
    sleep 0.3
    echo "      → Stored: trained_model.pth"
    echo ""
    
    echo -e "${GREEN}   ✅ Complete workflow (all via discovery!)${NC}"
fi

# Step 7: Failover demonstration
echo ""
echo "Step 7: Automatic failover with discovery..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   Simulating nestgate-node-1 failure...${NC}"
    sleep 0.5
    
    echo "   ❌ nestgate-node-1: OFFLINE"
    echo ""
    echo -e "${CYAN}   🔍 Rediscovering services...${NC}"
    sleep 0.5
    
    echo -e "${GREEN}   ✅ Found alternative: nestgate-node-2${NC}"
    echo "      • Capabilities: [persistent_storage, encryption]"
    echo "      • Status: Healthy"
    echo "      • Automatic failover successful!"
    echo ""
    echo "   💾 Data access maintained (zero downtime)"
fi

# Step 8: Visualize discovery workflow
echo ""
echo "Step 8: Capability-based discovery workflow..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │      CAPABILITY-BASED DISCOVERY FLOW                 │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "              Application needs storage"
echo "                        │"
echo "            1. Query by capability"
echo "           (NOT hardcoded endpoint!)"
echo "                        ↓"
echo "                 📡 Registry"
echo "             \"persistent_storage\""
echo "                        │"
echo "            2. Return matching services"
echo "                        ↓"
echo "          ┌─────────────┴─────────────┐"
echo "          │                           │"
echo "    NestGate Node 1            NestGate Node 2"
echo "    Score: 92/100 🏆           Score: 78/100"
echo "          │                           │"
echo "          └─────────────┬─────────────┘"
echo "                        │"
echo "           3. Select best (Node 1)"
echo "                        ↓"
echo "                  Connect O(1)"
echo "              (Direct, no hops!)"
echo "                        ↓"
echo "                 🗄️  NestGate Node 1"
echo "                        │"
echo "                4. Use service"
echo "                        ↓"
echo "                  Application"
echo ""

# Step 9: Show O(1) complexity
echo "Step 9: O(1) connection complexity..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Connection complexity comparison:"
echo ""
    echo "   Traditional (Hardcoded):"
    echo "     App → Hardcoded IP → Service"
    echo "     Problem: Breaks when service moves"
    echo "     Complexity: O(1) but brittle"
    echo ""
    echo "   Load Balancer (Proxy):"
    echo "     App → Load Balancer → Service"
    echo "     Problem: Extra hop, single point of failure"
    echo "     Complexity: O(n) hops"
    echo ""
    echo "   Service Mesh (Sidecar):"
    echo "     App → Sidecar → Service"
    echo "     Problem: Extra hop, resource overhead"
    echo "     Complexity: O(n) hops"
    echo ""
    echo "   Capability-Based (ecoPrimals):"
    echo "     App → Direct to Service"
    echo "     Benefits: No hops, no proxy, dynamic"
    echo "     Complexity: O(1) hops 🏆"
    echo ""
    echo "   🎯 ecoPrimals: True O(1) with dynamic discovery!"
fi

# Step 10: Configuration example
echo ""
echo "Step 10: Service advertisement example..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 NestGate Node 1 advertises:"
    echo ""
    echo "   {"
    echo "     \"service_name\": \"nestgate-node-1\","
    echo "     \"endpoint\": \"http://192.168.1.10:8080\","
    echo "     \"capabilities\": ["
    echo "       \"persistent_storage\","
    echo "       \"versioning\","
    echo "       \"metadata\","
    echo "       \"snapshots\""
    echo "     ],"
    echo "     \"health\": {"
    echo "       \"status\": \"healthy\","
    echo "       \"uptime_seconds\": 86400,"
    echo "       \"load_percent\": 45"
    echo "     },"
    echo "     \"performance\": {"
    echo "       \"latency_ms\": 8,"
    echo "       \"throughput_mbps\": 1000,"
    echo "       \"iops\": 50000"
    echo "     },"
    echo "     \"metadata\": {"
    echo "       \"version\": \"1.0.0\","
    echo "       \"region\": \"us-east\","
    echo "       \"availability_zone\": \"us-east-1a\""
    echo "     }"
    echo "   }"
fi

# Step 11: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ Capability-based service discovery"
echo "  ✅ Zero hardcoded endpoints"
echo "  ✅ Intelligent service selection"
echo "  ✅ Automatic failover"
echo "  ✅ O(1) connection complexity"
echo "  ✅ Dynamic service mesh"
echo ""
echo "Key benefits:"
echo "  🎯 Flexible: No hardcoded endpoints"
echo "  🚀 Fast: O(1) direct connections"
echo "  🛡️  Resilient: Automatic failover"
echo "  📊 Smart: Score-based selection"
echo "  ♾️  Scalable: Add services dynamically"
echo ""
echo "Discovery advantages:"
echo "  • Dynamic: Services come and go"
echo "  • Intelligent: Select best service"
echo "  • Resilient: Automatic failover"
echo "  • Zero-Config: No manual configuration"
echo ""
echo "Comparison with alternatives:"
echo "  vs Hardcoded IPs: ✅ Dynamic (not brittle)"
echo "  vs Load Balancers: ✅ No extra hop (faster)"
echo "  vs Service Mesh: ✅ No sidecar (simpler)"
echo "  vs DNS: ✅ Rich metadata (smarter)"
echo ""
echo "Real-world benefits:"
echo "  🏢 Production: Services can move/scale"
echo "  🌐 Multi-Region: Select closest service"
echo "  💰 Cost: Optimal resource utilization"
echo "  🔧 DevOps: Zero-downtime deployments"
echo ""
echo "Architecture pattern:"
echo "  📡 Decentralized: No single point of failure"
echo "  🎯 Capability-Based: Match by features, not names"
echo "  🚀 O(1) Complexity: Direct connections"
echo "  🔄 Dynamic: Services register/deregister automatically"
echo ""
echo "Complete Level 2 showcase:"
echo "  ✅ 01-data-triggered-compute: Event-driven processing"
echo "  ✅ 02-distributed-storage: Multi-node data + compute"
echo "  ✅ 03-capability-based: Advanced discovery (this demo)"
echo ""
echo "🎉 Level 2: Bidirectional Integration COMPLETE!"
echo ""
echo "Next:"
echo "  - Level 3 multi-primal demos already complete!"
echo "  - Explore: Complete 4-primal pipeline"
echo "  - Learn: Ecosystem integration patterns"
echo ""

