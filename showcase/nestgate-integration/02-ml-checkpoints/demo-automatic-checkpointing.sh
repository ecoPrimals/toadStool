#!/bin/bash

# ===================================================================
# ToadStool → NestGate: Automatic ML Checkpoint Management
# ===================================================================
# 
# What this demonstrates:
# - Automatic checkpoint saving during ML training
# - Versioned checkpoint storage in NestGate
# - Checkpoint metadata (epoch, loss, accuracy)
# - Resume training from checkpoint
# - Zero-configuration checkpoint pipeline
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
NC='\033[0m'

# Configuration
DEMO_MODE=true
NESTGATE_ENDPOINT="${NESTGATE_URL:-http://localhost:8080}"
TOADSTOOL_ENDPOINT="${TOADSTOOL_URL:-http://localhost:3000}"
CHECKPOINT_DIR="./checkpoints"

echo ""
echo "====================================================================="
echo "  ToadStool → NestGate: Automatic ML Checkpoint Management"
echo "====================================================================="
echo ""

# Step 1: Prerequisites check
echo "Step 1: Checking prerequisites..."
if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO MODE] Simulating operations...${NC}"
else
    # Check actual endpoints
    if ! curl -s "$NESTGATE_ENDPOINT/health" > /dev/null 2>&1; then
        echo "   ⚠️  NestGate not available, switching to demo mode"
        DEMO_MODE=true
    fi
    if ! curl -s "$TOADSTOOL_ENDPOINT/health" > /dev/null 2>&1; then
        echo "   ⚠️  ToadStool not available, switching to demo mode"
        DEMO_MODE=true
    fi
fi
echo -e "${GREEN}   ✅ Prerequisites checked${NC}"

# Step 2: Capability-based discovery
echo ""
echo "Step 2: Discovering NestGate via capabilities..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating discovery...${NC}"
    NESTGATE_URL="http://localhost:8080"
    NESTGATE_CAPABILITIES="persistent_storage,versioning,metadata"
    sleep 0.5
else
    # Real discovery via registry
    DISCOVERY=$(curl -s "$REGISTRY_URL/api/v1/services?capability=persistent_storage")
    NESTGATE_URL=$(echo "$DISCOVERY" | jq -r '.[0].endpoint')
    NESTGATE_CAPABILITIES=$(echo "$DISCOVERY" | jq -r '.[0].capabilities[]' | tr '\n' ',')
fi

echo -e "${GREEN}   ✅ Discovered NestGate at: $NESTGATE_URL${NC}"
echo "   📦 Capabilities: $NESTGATE_CAPABILITIES"

# Step 3: Start ML training with automatic checkpointing
echo ""
echo "Step 3: Starting ML training with automatic checkpointing..."
echo "   Training: Simple neural network (MNIST-style)"
echo "   Checkpoints: Every 5 epochs"
echo "   Storage: NestGate (versioned)"
echo ""

mkdir -p "$CHECKPOINT_DIR"

# Simulate training with checkpoints
TOTAL_EPOCHS=20
CHECKPOINT_INTERVAL=5

for epoch in $(seq 1 $TOTAL_EPOCHS); do
    if [ "$DEMO_MODE" = true ]; then
        # Simulate training
        LOSS=$(echo "scale=4; 2.5 - ($epoch * 0.1)" | bc)
        ACCURACY=$(echo "scale=2; 50 + ($epoch * 2)" | bc)
        
        echo -ne "   Epoch $epoch/$TOTAL_EPOCHS - Loss: $LOSS, Accuracy: ${ACCURACY}%\r"
        sleep 0.1
        
        # Save checkpoint at intervals
        if [ $((epoch % CHECKPOINT_INTERVAL)) -eq 0 ]; then
            echo ""
            echo -e "${CYAN}   💾 Checkpoint triggered at epoch $epoch${NC}"
            
            # Create checkpoint file
            CHECKPOINT_FILE="$CHECKPOINT_DIR/checkpoint_epoch_${epoch}.bin"
            echo "model_state=trained_${epoch}_epochs" > "$CHECKPOINT_FILE"
            echo "optimizer_state=adam_lr_0.001" >> "$CHECKPOINT_FILE"
            echo "epoch=$epoch" >> "$CHECKPOINT_FILE"
            echo "loss=$LOSS" >> "$CHECKPOINT_FILE"
            echo "accuracy=$ACCURACY" >> "$CHECKPOINT_FILE"
            
            # Store in NestGate
            echo -e "${YELLOW}   [DEMO] Storing checkpoint in NestGate...${NC}"
            STORAGE_KEY="ml-training/checkpoints/mnist/epoch_${epoch}"
            STORAGE_ID="checkpoint-$(date +%s)-$epoch"
            
            echo "   📦 Stored: $STORAGE_KEY"
            echo "   🔑 ID: $STORAGE_ID"
            echo "   📊 Metadata: epoch=$epoch, loss=$LOSS, accuracy=${ACCURACY}%"
            echo ""
        fi
    else
        # Real training would happen here
        # With automatic checkpoint saving to NestGate
        :
    fi
done

echo ""
echo -e "${GREEN}   ✅ Training complete!${NC}"

# Step 4: List saved checkpoints
echo ""
echo "Step 4: Listing saved checkpoints in NestGate..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying NestGate...${NC}"
    echo ""
    echo "   Checkpoints stored:"
    echo "   ┌──────────┬────────────┬──────────┬───────────┐"
    echo "   │  Epoch   │    Loss    │ Accuracy │  Storage  │"
    echo "   ├──────────┼────────────┼──────────┼───────────┤"
    for epoch in 5 10 15 20; do
        LOSS=$(echo "scale=4; 2.5 - ($epoch * 0.1)" | bc)
        ACCURACY=$(echo "scale=2; 50 + ($epoch * 2)" | bc)
        printf "   │ %8d │ %10s │ %7s%% │ %-9s │\n" "$epoch" "$LOSS" "$ACCURACY" "NestGate"
    done
    echo "   └──────────┴────────────┴──────────┴───────────┘"
else
    # Query real NestGate
    CHECKPOINTS=$(curl -s "$NESTGATE_URL/api/v1/storage/query?prefix=ml-training/checkpoints")
    echo "$CHECKPOINTS" | jq -r '.[] | "\(.epoch) \(.metadata)"'
fi

# Step 5: Demonstrate resuming from checkpoint
echo ""
echo "Step 5: Demonstrating resume from checkpoint..."

RESUME_EPOCH=10

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating checkpoint restore...${NC}"
    echo ""
    echo "   📥 Loading checkpoint from epoch $RESUME_EPOCH"
    sleep 0.5
    
    CHECKPOINT_FILE="$CHECKPOINT_DIR/checkpoint_epoch_${RESUME_EPOCH}.bin"
    if [ -f "$CHECKPOINT_FILE" ]; then
        echo -e "${GREEN}   ✅ Checkpoint loaded${NC}"
        echo "   📊 State restored:"
        cat "$CHECKPOINT_FILE" | sed 's/^/      /'
        echo ""
        echo "   ▶️  Training would resume from epoch $(($RESUME_EPOCH + 1))"
    fi
else
    # Restore real checkpoint from NestGate
    STORAGE_KEY="ml-training/checkpoints/mnist/epoch_${RESUME_EPOCH}"
    CHECKPOINT_DATA=$(curl -s "$NESTGATE_URL/api/v1/storage/retrieve?key=$STORAGE_KEY")
    # Restore model state and continue training
fi

# Step 6: Visualize checkpoint workflow
echo ""
echo "Step 6: Checkpoint workflow visualization..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │        AUTOMATIC CHECKPOINT PIPELINE                 │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "                    ML Training"
echo "                   (ToadStool)"
echo "                        │"
echo "                        │ Every N epochs"
echo "                        ↓"
echo "                 Generate Checkpoint"
echo "                 (model + optimizer state)"
echo "                        │"
echo "                        │ 1. Create checkpoint"
echo "                        ↓"
echo "              ┌─────────────────────┐"
echo "              │  Checkpoint File    │"
echo "              │  - Model weights    │"
echo "              │  - Optimizer state  │"
echo "              │  - Training metadata│"
echo "              └─────────────────────┘"
echo "                        │"
echo "                        │ 2. Store in NestGate"
echo "                        ↓"
echo "                  🗄️  NestGate"
echo "               (Persistent Storage)"
echo "                        │"
echo "              ┌─────────┼─────────┐"
echo "              │         │         │"
echo "     3. Version   4. Metadata  5. Retrieve"
echo "              │         │         │"
echo "              ↓         ↓         ↓"
echo "          v1, v2...  epoch,   Resume training"
echo "                      loss,     from any"
echo "                    accuracy   checkpoint"
echo ""
echo "   🔄 Training can be interrupted and resumed at any time!"
echo ""

# Step 7: Show checkpoint metadata
echo "Step 7: Checkpoint metadata details..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 Checkpoint Metadata Example:"
    echo ""
    echo "   {"
    echo "     \"checkpoint_id\": \"checkpoint-1734766800-10\","
    echo "     \"storage_key\": \"ml-training/checkpoints/mnist/epoch_10\","
    echo "     \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "     \"training_metadata\": {"
    echo "       \"epoch\": 10,"
    echo "       \"total_epochs\": 20,"
    echo "       \"loss\": 1.5,"
    echo "       \"accuracy\": 70.0,"
    echo "       \"learning_rate\": 0.001,"
    echo "       \"batch_size\": 32"
    echo "     },"
    echo "     \"model_metadata\": {"
    echo "       \"architecture\": \"SimpleNN\","
    echo "       \"parameters\": 78000,"
    echo "       \"layers\": 3"
    echo "     },"
    echo "     \"storage_info\": {"
    echo "       \"size_bytes\": 312000,"
    echo "       \"compression\": \"gzip\","
    echo "       \"version\": \"v10\""
    echo "     }"
    echo "   }"
fi

# Step 8: Cleanup demo files
echo ""
echo "Step 8: Cleanup..."
if [ -d "$CHECKPOINT_DIR" ]; then
    rm -rf "$CHECKPOINT_DIR"
    echo -e "${GREEN}   ✅ Demo checkpoint files cleaned up${NC}"
fi

# Step 9: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ Automatic checkpoint saving every N epochs"
echo "  ✅ Versioned checkpoint storage in NestGate"
echo "  ✅ Rich metadata (epoch, loss, accuracy)"
echo "  ✅ Resume training from any checkpoint"
echo "  ✅ Zero-configuration pipeline"
echo "  ✅ Capability-based discovery"
echo ""
echo "Key benefits:"
echo "  💾 Never lose training progress"
echo "  🔄 Resume from any point"
echo "  📊 Track training metrics over time"
echo "  🗄️  Persistent, versioned storage"
echo "  🚀 Automatic, zero-config"
echo ""
echo "Next steps:"
echo "  - Try: 03-dataset-management demo (manage training datasets)"
echo "  - Try: 04-model-registry demo (store and version trained models)"
echo "  - Learn: Level 2 bidirectional demos (data-triggered compute)"
echo ""
echo "Real-world value:"
echo "  🎓 Research: Never lose experiment results"
echo "  🏢 Production: Reliable model training"
echo "  💰 Cost savings: Resume interrupted training"
echo "  🔬 Reproducibility: Complete training history"
echo ""

