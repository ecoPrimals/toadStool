#!/bin/bash

# ===================================================================
# NestGate ↔ ToadStool: Data-Triggered Compute
# ===================================================================
# 
# What this demonstrates:
# - NestGate triggers ToadStool compute when data arrives
# - Event-driven architecture (data events)
# - Automatic processing pipeline
# - Results stored back in NestGate
# - Bidirectional workflow
#
# Prerequisites:
# - NestGate endpoint (or demo mode)
# - ToadStool endpoint (or demo mode)
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
NESTGATE_ENDPOINT="${NESTGATE_URL:-http://localhost:8080}"
TOADSTOOL_ENDPOINT="${TOADSTOOL_URL:-http://localhost:3000}"
WATCH_DIR="./watch-data"

echo ""
echo "====================================================================="
echo "  NestGate ↔ ToadStool: Data-Triggered Compute"
echo "====================================================================="
echo ""

# Step 1: Prerequisites check
echo "Step 1: Checking prerequisites..."
if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO MODE] Simulating operations...${NC}"
else
    if ! curl -s "$NESTGATE_ENDPOINT/health" > /dev/null 2>&1; then
        echo "   ⚠️  NestGate not available, switching to demo mode"
        DEMO_MODE=true
    fi
fi
echo -e "${GREEN}   ✅ Prerequisites checked${NC}"

# Step 2: Configure event subscription
echo ""
echo "Step 2: Configuring NestGate event subscription..."
echo "   Watching: ml-datasets/incoming/"
echo "   Event type: file_created"
echo "   Action: trigger ToadStool compute"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Setting up event subscription...${NC}"
    sleep 0.5
    
    SUBSCRIPTION_ID="sub-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Event subscription created${NC}"
    echo "   🔔 Subscription ID: $SUBSCRIPTION_ID"
    echo "   📁 Watching path: ml-datasets/incoming/"
    echo "   🎯 Target: ToadStool at $TOADSTOOL_ENDPOINT"
    echo "   ⚙️  Action: process_dataset"
fi

# Step 3: Upload dataset to trigger processing
echo ""
echo "Step 3: Uploading new dataset to NestGate..."
echo "   Dataset: customer_data_batch_001.csv"
echo "   Size: 5,000 records"
echo ""

mkdir -p "$WATCH_DIR"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Creating dataset...${NC}"
    
    DATASET_FILE="$WATCH_DIR/customer_data_batch_001.csv"
    cat > "$DATASET_FILE" << EOF
customer_id,purchase_amount,category,timestamp
001,125.50,electronics,2025-12-21T09:00:00Z
002,89.99,clothing,2025-12-21T09:01:00Z
003,245.00,electronics,2025-12-21T09:02:00Z
...
[5,000 records total]
EOF
    
    echo -e "${YELLOW}   [DEMO] Uploading to NestGate...${NC}"
    sleep 0.5
    
    STORAGE_KEY="ml-datasets/incoming/customer_data_batch_001.csv"
    STORAGE_ID="data-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Dataset uploaded${NC}"
    echo "   📦 Key: $STORAGE_KEY"
    echo "   🔑 ID: $STORAGE_ID"
    echo "   📊 Size: 5,000 records"
fi

# Step 4: NestGate triggers event
echo ""
echo "Step 4: NestGate event triggered..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${CYAN}   🔔 Event detected: file_created${NC}"
    echo "   📁 File: $STORAGE_KEY"
    echo "   📅 Timestamp: $(date)"
    sleep 0.3
    
    echo ""
    echo -e "${CYAN}   🚀 Triggering ToadStool compute...${NC}"
    sleep 0.5
    
    WORKLOAD_ID="workload-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Compute triggered${NC}"
    echo "   🔧 Workload ID: $WORKLOAD_ID"
    echo "   🎯 Task: analyze_customer_data"
    echo "   📦 Input: $STORAGE_KEY"
fi

# Step 5: ToadStool processes data
echo ""
echo "Step 5: ToadStool processing dataset..."
echo "   Task: Customer segmentation analysis"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Running analysis...${NC}"
    
    # Simulate processing stages
    for step in "Loading data" "Preprocessing" "Feature extraction" "Clustering" "Generating insights"; do
        echo -ne "      $step...\r"
        sleep 0.3
    done
    echo "      Completed!                       "
    
    echo ""
    echo -e "${GREEN}   ✅ Analysis complete${NC}"
    echo "   📊 Results:"
    echo "      - Segments identified: 4"
    echo "      - High-value customers: 127 (2.5%)"
    echo "      - Average purchase: \$156.33"
    echo "      - Processing time: 2.3s"
fi

# Step 6: ToadStool stores results back in NestGate
echo ""
echo "Step 6: Storing results back in NestGate..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Storing analysis results...${NC}"
    sleep 0.5
    
    RESULTS_KEY="ml-results/customer-analysis/batch_001_results.json"
    RESULTS_ID="results-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Results stored${NC}"
    echo "   📦 Key: $RESULTS_KEY"
    echo "   🔑 ID: $RESULTS_ID"
    echo "   📊 Metadata:"
    echo "      - Input: customer_data_batch_001.csv"
    echo "      - Records processed: 5,000"
    echo "      - Segments: 4"
    echo "      - Processing time: 2.3s"
fi

# Step 7: Visualize complete workflow
echo ""
echo "Step 7: Data-triggered workflow visualization..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │        DATA-TRIGGERED COMPUTE WORKFLOW               │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "                    User"
echo "                      │"
echo "           1. Upload new dataset"
echo "                      ↓"
echo "                🗄️  NestGate"
echo "            (Event-Driven Storage)"
echo "                      │"
echo "            ┌─────────┴─────────┐"
echo "            │                   │"
echo "      Store data          Trigger event"
echo "            │                   │"
echo "            ↓                   ↓"
echo "      📁 Stored          🔔 file_created"
echo "                              │"
echo "               2. Notify ToadStool"
echo "                              ↓"
echo "                        🍄 ToadStool"
echo "                     (Compute Engine)"
echo "                              │"
echo "              3. Process automatically"
echo "                              │"
echo "                  ┌───────────┴───────────┐"
echo "                  │                       │"
echo "            Load data              Run analysis"
echo "                  │                       │"
echo "                  ↓                       ↓"
echo "          From NestGate          Customer segments"
echo "                                          │"
echo "                         4. Store results back"
echo "                                          ↓"
echo "                                    🗄️  NestGate"
echo "                                     (Results)"
echo "                                          │"
echo "                         5. Notify completion"
echo "                                          ↓"
echo "                                        User"
echo ""
echo "   🔄 Fully automatic: Upload → Process → Store → Notify"
echo ""

# Step 8: Show event subscription details
echo "Step 8: Event subscription configuration..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 Subscription details:"
    echo ""
    echo "   {"
    echo "     \"subscription_id\": \"$SUBSCRIPTION_ID\","
    echo "     \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "     \"watch_config\": {"
    echo "       \"path\": \"ml-datasets/incoming/\","
    echo "       \"event_types\": [\"file_created\", \"file_updated\"],"
    echo "       \"recursive\": true"
    echo "     },"
    echo "     \"action_config\": {"
    echo "       \"service\": \"toadstool\","
    echo "       \"endpoint\": \"$TOADSTOOL_ENDPOINT\","
    echo "       \"action\": \"process_dataset\","
    echo "       \"timeout_seconds\": 300"
    echo "     },"
    echo "     \"storage_config\": {"
    echo "       \"results_path\": \"ml-results/customer-analysis/\","
    echo "       \"store_automatically\": true,"
    echo "       \"include_metadata\": true"
    echo "     },"
    echo "     \"notification_config\": {"
    echo "       \"on_success\": true,"
    echo "       \"on_failure\": true,"
    echo "       \"method\": \"webhook\""
    echo "     }"
    echo "   }"
fi

# Step 9: Demonstrate multiple events
echo ""
echo "Step 9: Processing multiple datasets (batch)..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    for batch in 002 003 004; do
        echo -e "${CYAN}   📁 New dataset: customer_data_batch_${batch}.csv${NC}"
        sleep 0.2
        echo "      🔔 Event triggered"
        sleep 0.2
        echo "      🍄 ToadStool processing..."
        sleep 0.3
        echo "      ✅ Results stored"
        echo ""
    done
    
    echo -e "${GREEN}   ✅ All batches processed${NC}"
    echo "   📊 Summary:"
    echo "      - Batches processed: 4"
    echo "      - Total records: 20,000"
    echo "      - Total time: 9.2s"
    echo "      - Average: 2.3s per batch"
fi

# Step 10: Cleanup
echo ""
echo "Step 10: Cleanup..."
if [ -d "$WATCH_DIR" ]; then
    rm -rf "$WATCH_DIR"
    echo -e "${GREEN}   ✅ Demo files cleaned up${NC}"
fi

# Step 11: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ NestGate triggers ToadStool on data arrival"
echo "  ✅ Event-driven architecture (file_created events)"
echo "  ✅ Automatic processing pipeline"
echo "  ✅ Results stored back in NestGate"
echo "  ✅ Bidirectional workflow"
echo "  ✅ Batch processing support"
echo ""
echo "Key benefits:"
echo "  🚀 Zero manual intervention"
echo "  ⚡ Immediate processing"
echo "  🔄 Complete automation"
echo "  📊 Automatic result storage"
echo "  🎯 Event-driven architecture"
echo ""
echo "Event-driven advantages:"
echo "  • Real-Time: Process data immediately upon arrival"
echo "  • Scalable: Handle burst workloads automatically"
echo "  • Reliable: Guaranteed delivery and processing"
echo "  • Auditable: Complete event and processing history"
echo ""
echo "Real-world use cases:"
echo "  📊 Analytics: Process new data files automatically"
echo "  🤖 ML Training: Retrain on new data arrival"
echo "  📸 Media: Process uploaded images/videos"
echo "  📧 Reporting: Generate reports on new data"
echo ""
echo "Architecture pattern:"
echo "  ♻️  Event-Driven: Reactive, automatic, scalable"
echo "  🔄 Bidirectional: NestGate → ToadStool → NestGate"
echo "  🎯 Decoupled: Services don't need to know each other"
echo "  📡 Observable: Complete event and processing logs"
echo ""
echo "Next steps:"
echo "  - Try: 02-distributed-storage demo (multi-node storage)"
echo "  - Try: 03-capability-based demo (advanced discovery)"
echo "  - Explore: Level 3 multi-primal (complete ecosystem)"
echo ""

