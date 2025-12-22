#!/bin/bash

# ===================================================================
# ToadStool → NestGate: Dataset Management & Versioning
# ===================================================================
# 
# What this demonstrates:
# - Store training datasets in NestGate
# - Version datasets (v1, v2, v3)
# - Rich metadata (samples, features, splits)
# - ToadStool loads dataset for training
# - Compare training across dataset versions
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
DATASET_DIR="./datasets"

echo ""
echo "====================================================================="
echo "  ToadStool → NestGate: Dataset Management & Versioning"
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

# Step 2: Create initial dataset (v1)
echo ""
echo "Step 2: Creating initial dataset (v1)..."
echo "   Dataset: MNIST-style handwritten digits"
echo "   Samples: 60,000 training + 10,000 test"
echo "   Features: 28x28 grayscale images"
echo ""

mkdir -p "$DATASET_DIR"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Generating dataset v1...${NC}"
    
    # Simulate dataset creation
    DATASET_V1="$DATASET_DIR/mnist_v1.npz"
    cat > "$DATASET_V1" << EOF
# Simulated MNIST Dataset v1
# Training samples: 60,000
# Test samples: 10,000
# Features: 28x28 pixels (784 features)
# Classes: 10 (digits 0-9)
# Format: NumPy compressed
EOF
    
    echo -e "${GREEN}   ✅ Dataset v1 created (240KB)${NC}"
    
    # Store in NestGate
    echo -e "${YELLOW}   [DEMO] Storing dataset v1 in NestGate...${NC}"
    sleep 0.5
    
    STORAGE_KEY="ml-datasets/mnist/v1"
    STORAGE_ID="dataset-mnist-v1-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Stored in NestGate${NC}"
    echo "   📦 Key: $STORAGE_KEY"
    echo "   🔑 ID: $STORAGE_ID"
    echo "   📊 Metadata:"
    echo "      - Training samples: 60,000"
    echo "      - Test samples: 10,000"
    echo "      - Features: 784"
    echo "      - Classes: 10"
    echo "      - Version: v1"
fi

# Step 3: Create improved dataset (v2)
echo ""
echo "Step 3: Creating improved dataset (v2)..."
echo "   Improvements:"
echo "   - Data augmentation applied"
echo "   - Additional 10,000 samples"
echo "   - Better class balance"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Generating dataset v2...${NC}"
    
    DATASET_V2="$DATASET_DIR/mnist_v2.npz"
    cat > "$DATASET_V2" << EOF
# Simulated MNIST Dataset v2
# Training samples: 70,000 (augmented)
# Test samples: 10,000
# Features: 28x28 pixels (784 features)
# Classes: 10 (digits 0-9)
# Improvements: Data augmentation, better balance
EOF
    
    echo -e "${GREEN}   ✅ Dataset v2 created (280KB)${NC}"
    
    echo -e "${YELLOW}   [DEMO] Storing dataset v2 in NestGate...${NC}"
    sleep 0.5
    
    STORAGE_KEY_V2="ml-datasets/mnist/v2"
    STORAGE_ID_V2="dataset-mnist-v2-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Stored in NestGate${NC}"
    echo "   📦 Key: $STORAGE_KEY_V2"
    echo "   🔑 ID: $STORAGE_ID_V2"
    echo "   📊 Metadata:"
    echo "      - Training samples: 70,000 (+10,000)"
    echo "      - Test samples: 10,000"
    echo "      - Features: 784"
    echo "      - Classes: 10 (balanced)"
    echo "      - Version: v2"
    echo "      - Augmentation: Yes"
fi

# Step 4: Create production dataset (v3)
echo ""
echo "Step 4: Creating production dataset (v3)..."
echo "   Production enhancements:"
echo "   - Cleaned labels (manual review)"
echo "   - Validation split added (10%)"
echo "   - Normalized features"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Generating dataset v3...${NC}"
    
    DATASET_V3="$DATASET_DIR/mnist_v3.npz"
    cat > "$DATASET_V3" << EOF
# Simulated MNIST Dataset v3 (Production)
# Training samples: 63,000
# Validation samples: 7,000
# Test samples: 10,000
# Features: 28x28 pixels (784 features, normalized)
# Classes: 10 (digits 0-9)
# Quality: Production-ready, cleaned labels
EOF
    
    echo -e "${GREEN}   ✅ Dataset v3 created (300KB)${NC}"
    
    echo -e "${YELLOW}   [DEMO] Storing dataset v3 in NestGate...${NC}"
    sleep 0.5
    
    STORAGE_KEY_V3="ml-datasets/mnist/v3"
    STORAGE_ID_V3="dataset-mnist-v3-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Stored in NestGate${NC}"
    echo "   📦 Key: $STORAGE_KEY_V3"
    echo "   🔑 ID: $STORAGE_ID_V3"
    echo "   📊 Metadata:"
    echo "      - Training samples: 63,000"
    echo "      - Validation samples: 7,000"
    echo "      - Test samples: 10,000"
    echo "      - Features: 784 (normalized)"
    echo "      - Classes: 10 (cleaned labels)"
    echo "      - Version: v3"
    echo "      - Quality: Production"
fi

# Step 5: List all dataset versions
echo ""
echo "Step 5: Listing all dataset versions in NestGate..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying NestGate...${NC}"
    echo ""
    echo "   Dataset versions stored:"
    echo "   ┌─────────┬──────────┬────────┬────────────┬──────────────┐"
    echo "   │ Version │ Training │ Test   │    Size    │    Quality   │"
    echo "   ├─────────┼──────────┼────────┼────────────┼──────────────┤"
    echo "   │   v1    │  60,000  │ 10,000 │   240 KB   │   Baseline   │"
    echo "   │   v2    │  70,000  │ 10,000 │   280 KB   │  Augmented   │"
    echo "   │   v3    │  63,000  │ 10,000 │   300 KB   │  Production  │"
    echo "   └─────────┴──────────┴────────┴────────────┴──────────────┘"
    echo ""
    echo "   💡 Each version preserved independently"
fi

# Step 6: Train models on different versions
echo ""
echo "Step 6: Training models on different dataset versions..."
echo ""

declare -A RESULTS

for version in v1 v2 v3; do
    echo -e "${CYAN}   Training on dataset $version...${NC}"
    
    if [ "$DEMO_MODE" = true ]; then
        echo -e "${YELLOW}   [DEMO] Loading dataset $version from NestGate...${NC}"
        sleep 0.3
        
        # Simulate training
        case $version in
            v1)
                ACCURACY="85.2"
                TRAINING_TIME="45"
                ;;
            v2)
                ACCURACY="88.7"
                TRAINING_TIME="52"
                ;;
            v3)
                ACCURACY="91.3"
                TRAINING_TIME="50"
                ;;
        esac
        
        echo "   📊 Training complete:"
        echo "      - Accuracy: ${ACCURACY}%"
        echo "      - Training time: ${TRAINING_TIME}s"
        echo ""
        
        RESULTS[$version]="$ACCURACY"
    fi
done

# Step 7: Compare results
echo "Step 7: Comparing results across dataset versions..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   Performance comparison:"
    echo "   ┌─────────┬───────────┬────────────┬─────────────────┐"
    echo "   │ Version │ Accuracy  │    Time    │   Improvement   │"
    echo "   ├─────────┼───────────┼────────────┼─────────────────┤"
    echo "   │   v1    │   85.2%   │    45s     │   (baseline)    │"
    echo "   │   v2    │   88.7%   │    52s     │   +3.5% acc     │"
    echo "   │   v3    │   91.3%   │    50s     │   +6.1% acc 🏆  │"
    echo "   └─────────┴───────────┴────────────┴─────────────────┘"
    echo ""
    echo "   🎯 Best: Dataset v3 (production quality)"
    echo "   📈 Improvement: 6.1 percentage points over baseline"
fi

# Step 8: Visualize dataset workflow
echo ""
echo "Step 8: Dataset versioning workflow..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │         DATASET VERSIONING WORKFLOW                  │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "             Create Dataset v1"
echo "            (Initial collection)"
echo "                    │"
echo "                    │ 1. Store in NestGate"
echo "                    ↓"
echo "              🗄️  NestGate"
echo "           (Versioned Storage)"
echo "                    │"
echo "         ┌──────────┼──────────┐"
echo "         │          │          │"
echo "        v1         v2         v3"
echo "     Baseline  Augmented  Production"
echo "         │          │          │"
echo "         └──────────┴──────────┘"
echo "                    │"
echo "         2. ToadStool loads any version"
echo "                    ↓"
echo "            🍄 ToadStool"
echo "          (ML Training)"
echo "                    │"
echo "         3. Train and compare"
echo "                    ↓"
echo "        Select best dataset"
echo "         (v3: 91.3% acc)"
echo ""

# Step 9: Show dataset metadata
echo "Step 9: Dataset metadata example..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 Complete metadata for dataset v3:"
    echo ""
    echo "   {"
    echo "     \"dataset_id\": \"dataset-mnist-v3-$(date +%s)\","
    echo "     \"storage_key\": \"ml-datasets/mnist/v3\","
    echo "     \"version\": \"v3\","
    echo "     \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "     \"dataset_info\": {"
    echo "       \"name\": \"MNIST Handwritten Digits\","
    echo "       \"task\": \"image_classification\","
    echo "       \"classes\": 10,"
    echo "       \"features\": 784"
    echo "     },"
    echo "     \"splits\": {"
    echo "       \"train\": 63000,"
    echo "       \"validation\": 7000,"
    echo "       \"test\": 10000"
    echo "     },"
    echo "     \"preprocessing\": {"
    echo "       \"normalization\": \"min-max\","
    echo "       \"augmentation\": true,"
    echo "       \"cleaned_labels\": true"
    echo "     },"
    echo "     \"quality\": {"
    echo "       \"status\": \"production\","
    echo "       \"reviewed\": true,"
    echo "       \"baseline_accuracy\": 91.3"
    echo "     },"
    echo "     \"storage_info\": {"
    echo "       \"size_bytes\": 307200,"
    echo "       \"format\": \"npz\","
    echo "       \"compression\": \"gzip\""
    echo "     }"
    echo "   }"
fi

# Step 10: Cleanup
echo ""
echo "Step 10: Cleanup..."
if [ -d "$DATASET_DIR" ]; then
    rm -rf "$DATASET_DIR"
    echo -e "${GREEN}   ✅ Demo dataset files cleaned up${NC}"
fi

# Step 11: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ Store datasets in NestGate"
echo "  ✅ Version datasets (v1, v2, v3)"
echo "  ✅ Rich metadata (samples, features, quality)"
echo "  ✅ Load datasets for training"
echo "  ✅ Compare results across versions"
echo "  ✅ Independent version preservation"
echo ""
echo "Key benefits:"
echo "  📊 Track dataset evolution"
echo "  🔄 Reproduce experiments exactly"
echo "  🎯 Compare dataset improvements"
echo "  🗄️  Persistent, versioned storage"
echo "  🚀 Zero-configuration loading"
echo ""
echo "Dataset versioning advantages:"
echo "  • Reproducibility: Train on exact same data"
echo "  • A/B Testing: Compare dataset improvements"
echo "  • Audit Trail: Know what data trained each model"
echo "  • Rollback: Use previous version if needed"
echo ""
echo "Real-world use cases:"
echo "  🔬 Research: Track dataset improvements"
echo "  🏭 Production: Version production datasets"
echo "  🎓 Education: Share exact datasets with students"
echo "  🤝 Collaboration: Team uses same dataset versions"
echo ""
echo "Next steps:"
echo "  - Try: 04-model-registry demo (store trained models)"
echo "  - Try: Level 2 data-triggered compute (auto-train on new data)"
echo "  - Try: Level 3 multi-primal (distributed dataset management)"
echo ""

