#!/bin/bash

# ===================================================================
# ToadStool → NestGate: ML Model Registry
# ===================================================================
# 
# What this demonstrates:
# - Store trained ML models in NestGate
# - Version models (experiments, iterations)
# - Rich metadata (accuracy, training time, hyperparams)
# - Load models for inference
# - Compare model performance
# - Production model promotion
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
RED='\033[0;31m'
NC='\033[0m'

# Configuration
DEMO_MODE=true
NESTGATE_ENDPOINT="${NESTGATE_URL:-http://localhost:8080}"
MODELS_DIR="./models"

echo ""
echo "====================================================================="
echo "  ToadStool → NestGate: ML Model Registry"
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

# Step 2: Train and store first model (experiment 1)
echo ""
echo "Step 2: Training and storing model - Experiment 1..."
echo "   Architecture: SimpleNN (3 layers)"
echo "   Hyperparameters: lr=0.01, batch_size=32"
echo ""

mkdir -p "$MODELS_DIR"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Training model...${NC}"
    sleep 0.5
    
    # Simulate training
    MODEL_1="$MODELS_DIR/model_exp1.pth"
    cat > "$MODEL_1" << EOF
# Trained Model - Experiment 1
# Architecture: SimpleNN
# Training epochs: 20
# Final accuracy: 85.2%
# Model weights: [simulated]
EOF
    
    ACCURACY_1="85.2"
    TRAINING_TIME_1="45"
    
    echo -e "${GREEN}   ✅ Training complete${NC}"
    echo "   📊 Accuracy: ${ACCURACY_1}%"
    echo "   ⏱️  Training time: ${TRAINING_TIME_1}s"
    
    # Store in NestGate
    echo ""
    echo -e "${YELLOW}   [DEMO] Storing model in NestGate...${NC}"
    sleep 0.3
    
    STORAGE_KEY="ml-models/mnist/exp1"
    MODEL_ID="model-exp1-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Model stored${NC}"
    echo "   📦 Key: $STORAGE_KEY"
    echo "   🔑 ID: $MODEL_ID"
    echo "   📊 Metadata:"
    echo "      - Architecture: SimpleNN"
    echo "      - Accuracy: ${ACCURACY_1}%"
    echo "      - Training time: ${TRAINING_TIME_1}s"
    echo "      - Hyperparams: lr=0.01, batch_size=32"
fi

# Step 3: Train improved model (experiment 2)
echo ""
echo "Step 3: Training improved model - Experiment 2..."
echo "   Architecture: SimpleNN (3 layers)"
echo "   Hyperparameters: lr=0.001, batch_size=64 (improved)"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Training model...${NC}"
    sleep 0.5
    
    MODEL_2="$MODELS_DIR/model_exp2.pth"
    cat > "$MODEL_2" << EOF
# Trained Model - Experiment 2
# Architecture: SimpleNN
# Training epochs: 20
# Final accuracy: 88.7%
# Model weights: [simulated]
EOF
    
    ACCURACY_2="88.7"
    TRAINING_TIME_2="48"
    
    echo -e "${GREEN}   ✅ Training complete${NC}"
    echo "   📊 Accuracy: ${ACCURACY_2}% (+3.5%)"
    echo "   ⏱️  Training time: ${TRAINING_TIME_2}s"
    
    echo ""
    echo -e "${YELLOW}   [DEMO] Storing model in NestGate...${NC}"
    sleep 0.3
    
    STORAGE_KEY_2="ml-models/mnist/exp2"
    MODEL_ID_2="model-exp2-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Model stored${NC}"
    echo "   📦 Key: $STORAGE_KEY_2"
    echo "   🔑 ID: $MODEL_ID_2"
fi

# Step 4: Train production model (experiment 3)
echo ""
echo "Step 4: Training production model - Experiment 3..."
echo "   Architecture: DeepNN (5 layers, improved)"
echo "   Hyperparameters: lr=0.001, batch_size=64, dropout=0.2"
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Training model...${NC}"
    sleep 0.5
    
    MODEL_3="$MODELS_DIR/model_exp3.pth"
    cat > "$MODEL_3" << EOF
# Trained Model - Experiment 3
# Architecture: DeepNN (production)
# Training epochs: 30
# Final accuracy: 91.3%
# Model weights: [simulated]
EOF
    
    ACCURACY_3="91.3"
    TRAINING_TIME_3="72"
    
    echo -e "${GREEN}   ✅ Training complete${NC}"
    echo "   📊 Accuracy: ${ACCURACY_3}% 🏆 (+6.1%)"
    echo "   ⏱️  Training time: ${TRAINING_TIME_3}s"
    
    echo ""
    echo -e "${YELLOW}   [DEMO] Storing model in NestGate...${NC}"
    sleep 0.3
    
    STORAGE_KEY_3="ml-models/mnist/exp3"
    MODEL_ID_3="model-exp3-$(date +%s)"
    
    echo -e "${GREEN}   ✅ Model stored${NC}"
    echo "   📦 Key: $STORAGE_KEY_3"
    echo "   🔑 ID: $MODEL_ID_3"
    echo "   🌟 Tagged as: production-candidate"
fi

# Step 5: List all models in registry
echo ""
echo "Step 5: Querying model registry..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Querying NestGate model registry...${NC}"
    echo ""
    echo "   Models in registry:"
    echo "   ┌─────────┬───────────────┬───────────┬────────┬──────────┐"
    echo "   │  Exp#   │ Architecture  │ Accuracy  │  Time  │  Status  │"
    echo "   ├─────────┼───────────────┼───────────┼────────┼──────────┤"
    echo "   │   1     │   SimpleNN    │   85.2%   │  45s   │ Baseline │"
    echo "   │   2     │   SimpleNN    │   88.7%   │  48s   │ Improved │"
    echo "   │   3     │   DeepNN      │   91.3%   │  72s   │  🏆 Best │"
    echo "   └─────────┴───────────────┴───────────┴────────┴──────────┘"
    echo ""
    echo "   💡 All experiments tracked and versioned"
fi

# Step 6: Promote best model to production
echo ""
echo "Step 6: Promoting best model to production..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${CYAN}   📋 Model evaluation criteria:${NC}"
    echo "      - Accuracy: ${ACCURACY_3}% ✅ (> 90% required)"
    echo "      - Training time: ${TRAINING_TIME_3}s ✅ (< 120s required)"
    echo "      - Architecture: DeepNN ✅ (production-ready)"
    echo ""
    echo -e "${GREEN}   ✅ Model exp3 meets all criteria${NC}"
    echo ""
    echo -e "${YELLOW}   [DEMO] Promoting to production...${NC}"
    sleep 0.5
    
    # Create production alias
    PROD_KEY="ml-models/mnist/production"
    
    echo -e "${GREEN}   ✅ Model promoted${NC}"
    echo "   🚀 Production key: $PROD_KEY"
    echo "   🔗 Points to: $STORAGE_KEY_3"
    echo "   📅 Promoted at: $(date)"
fi

# Step 7: Load model for inference
echo ""
echo "Step 7: Loading production model for inference..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Loading model from NestGate...${NC}"
    sleep 0.3
    
    echo -e "${GREEN}   ✅ Model loaded${NC}"
    echo "   📦 Loaded: ml-models/mnist/production"
    echo "   🔑 Model ID: $MODEL_ID_3"
    echo "   📊 Accuracy: ${ACCURACY_3}%"
    echo ""
    echo -e "${CYAN}   Running inference on 10 test samples...${NC}"
    
    # Simulate inference
    for i in $(seq 1 10); do
        DIGIT=$((RANDOM % 10))
        CONFIDENCE=$((85 + RANDOM % 15))
        echo -ne "      Sample $i: Predicted=$DIGIT, Confidence=${CONFIDENCE}%\r"
        sleep 0.1
    done
    echo ""
    echo ""
    echo -e "${GREEN}   ✅ Inference complete${NC}"
    echo "   📊 Batch accuracy: 90.0%"
fi

# Step 8: Visualize model registry workflow
echo ""
echo "Step 8: Model registry workflow..."
echo ""
echo "   ┌──────────────────────────────────────────────────────┐"
echo "   │            MODEL REGISTRY WORKFLOW                   │"
echo "   └──────────────────────────────────────────────────────┘"
echo ""
echo "           Train Multiple Models"
echo "              (Experiments)"
echo "                    │"
echo "         ┌──────────┼──────────┐"
echo "         │          │          │"
echo "       Exp 1      Exp 2      Exp 3"
echo "      85.2%      88.7%      91.3% 🏆"
echo "         │          │          │"
echo "         │          │          │"
echo "         └──────────┴──────────┘"
echo "                    │"
echo "          1. Store in NestGate"
echo "         (Model Registry)"
echo "                    ↓"
echo "              🗄️  NestGate"
echo "           (Versioned Storage)"
echo "                    │"
echo "         ┌──────────┼──────────┐"
echo "         │          │          │"
echo "      Model v1   Model v2   Model v3"
echo "     Baseline   Improved  Production"
echo "         │          │          │"
echo "         │          │          │"
echo "         │          │    2. Promote best"
echo "         │          │          ↓"
echo "         │          │    Production Alias"
echo "         │          │          │"
echo "         │          │    3. Load for inference"
echo "         │          │          ↓"
echo "         │          │    🍄 ToadStool"
echo "         │          │      (Inference)"
echo "         └──────────┴──────────┘"
echo ""

# Step 9: Show model metadata
echo "Step 9: Model metadata example..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📋 Complete metadata for production model:"
    echo ""
    echo "   {"
    echo "     \"model_id\": \"$MODEL_ID_3\","
    echo "     \"storage_key\": \"ml-models/mnist/exp3\","
    echo "     \"production_alias\": \"ml-models/mnist/production\","
    echo "     \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "     \"model_info\": {"
    echo "       \"architecture\": \"DeepNN\","
    echo "       \"layers\": 5,"
    echo "       \"parameters\": 125000,"
    echo "       \"framework\": \"PyTorch\""
    echo "     },"
    echo "     \"training_info\": {"
    echo "       \"dataset_version\": \"v3\","
    echo "       \"epochs\": 30,"
    echo "       \"batch_size\": 64,"
    echo "       \"learning_rate\": 0.001,"
    echo "       \"dropout\": 0.2"
    echo "     },"
    echo "     \"performance\": {"
    echo "       \"train_accuracy\": 91.3,"
    echo "       \"val_accuracy\": 90.8,"
    echo "       \"test_accuracy\": 90.5,"
    echo "       \"training_time_seconds\": 72"
    echo "     },"
    echo "     \"deployment\": {"
    echo "       \"status\": \"production\","
    echo "       \"promoted_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "       \"promoted_by\": \"ml-team\","
    echo "       \"approval_required\": true"
    echo "     },"
    echo "     \"storage_info\": {"
    echo "       \"size_bytes\": 500000,"
    echo "       \"format\": \"pytorch\","
    echo "       \"compression\": \"gzip\""
    echo "     }"
    echo "   }"
fi

# Step 10: Compare all experiments
echo ""
echo "Step 10: Experiment comparison..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo "   📊 Detailed comparison:"
    echo ""
    echo "   ┌────────┬──────────┬───────┬──────┬─────────┬──────────┐"
    echo "   │  Exp   │   Arch   │  Acc  │ Time │   LR    │  Result  │"
    echo "   ├────────┼──────────┼───────┼──────┼─────────┼──────────┤"
    echo "   │   1    │ SimpleNN │ 85.2% │  45s │  0.01   │ Baseline │"
    echo "   │   2    │ SimpleNN │ 88.7% │  48s │ 0.001   │  Better  │"
    echo "   │   3    │  DeepNN  │ 91.3% │  72s │ 0.001   │ 🏆 Prod  │"
    echo "   └────────┴──────────┴───────┴──────┴─────────┴──────────┘"
    echo ""
    echo "   🎯 Key insights:"
    echo "      • Lower learning rate improved accuracy (exp1 → exp2)"
    echo "      • Deeper architecture further improved (exp2 → exp3)"
    echo "      • Training time acceptable (<120s threshold)"
    echo "      • Exp3 selected for production deployment"
fi

# Step 11: Cleanup
echo ""
echo "Step 11: Cleanup..."
if [ -d "$MODELS_DIR" ]; then
    rm -rf "$MODELS_DIR"
    echo -e "${GREEN}   ✅ Demo model files cleaned up${NC}"
fi

# Step 12: Summary
echo ""
echo "====================================================================="
echo "  Demo Complete! ✨"
echo "====================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ Store trained models in NestGate"
echo "  ✅ Version models (experiments)"
echo "  ✅ Rich metadata (accuracy, hyperparams)"
echo "  ✅ Compare model performance"
echo "  ✅ Promote best model to production"
echo "  ✅ Load models for inference"
echo ""
echo "Key benefits:"
echo "  🔬 Track all experiments"
echo "  📊 Compare model performance"
echo "  🚀 Easy production deployment"
echo "  🗄️  Versioned, persistent storage"
echo "  🔄 Rollback to previous models"
echo ""
echo "Model registry advantages:"
echo "  • Reproducibility: Know exactly what model is in production"
echo "  • Audit Trail: Complete history of all experiments"
echo "  • A/B Testing: Easy to compare and switch models"
echo "  • Collaboration: Team shares models via registry"
echo ""
echo "Real-world use cases:"
echo "  🔬 Research: Track all experiments, compare architectures"
echo "  🏭 Production: Manage production models, enable rollback"
echo "  🎓 Education: Share trained models with students"
echo "  🤝 MLOps: Integrate with CI/CD pipelines"
echo ""
echo "Next steps:"
echo "  - Explore: Level 2 bidirectional demos (data-triggered compute)"
echo "  - Try: Level 3 multi-primal (distributed model training)"
echo "  - Learn: Complete ML pipeline (checkpoints + datasets + models)"
echo ""

