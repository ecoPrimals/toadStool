#!/bin/bash
# Demo: Songbird + ToadStool + NestGate: Complete Compute Pipeline
# Purpose: Show coordinated compute with persistent storage
# Prerequisites: None (works in demo mode)
# Expected output: Complete data flow from orchestration → compute → storage

set -euo pipefail

DEMO_NAME="Complete Compute Pipeline: Songbird + ToadStool + NestGate"
OUTPUT_DIR="./outputs/complete-pipeline-$(date +%s)"
mkdir -p "$OUTPUT_DIR"

echo "🎵🍄🗄️ $DEMO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This demo shows the complete compute-to-storage pipeline:"
echo "  🎵 Songbird: Orchestrates and coordinates"
echo "  🍄 ToadStool: Executes compute workload"
echo "  🗄️  NestGate: Persists results permanently"
echo ""
echo "Flow: User → Songbird → ToadStool → NestGate → User"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
ORANGE='\033[0;33m'
NC='\033[0m'

# Configuration
SONGBIRD_ENDPOINT="${SONGBIRD_ENDPOINT:-http://localhost:8000}"
TOADSTOOL_ENDPOINT="${TOADSTOOL_ENDPOINT:-http://localhost:8080}"
NESTGATE_ENDPOINT="${NESTGATE_ENDPOINT:-http://localhost:8082}"
DEMO_MODE=true

# Step 1: Discover ecosystem
echo "Step 1: Discovering ecosystem services..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [Songbird] Scanning for ecosystem services...${NC}"
    sleep 0.5
    
    cat > "$OUTPUT_DIR/ecosystem-topology.json" <<EOF
{
  "ecosystem_id": "local-complete-pipeline",
  "discovered_at": "$(date -Iseconds)",
  "services": [
    {
      "service_id": "songbird-local",
      "type": "coordinator",
      "endpoint": "$SONGBIRD_ENDPOINT",
      "capabilities": ["coordination", "orchestration", "routing"],
      "version": "0.1.0",
      "status": "healthy"
    },
    {
      "service_id": "toadstool-local",
      "type": "compute",
      "endpoint": "$TOADSTOOL_ENDPOINT",
      "capabilities": ["compute.native", "compute.container", "compute.gpu"],
      "metadata": {
        "platform": "$(uname -s)",
        "arch": "$(uname -m)",
        "gpu": "$([ -x "$(command -v nvidia-smi)" ] && echo "NVIDIA GPU" || echo "none")"
      },
      "status": "healthy"
    },
    {
      "service_id": "nestgate-local",
      "type": "storage",
      "endpoint": "$NESTGATE_ENDPOINT",
      "capabilities": ["persistent_storage", "metadata_query", "versioning"],
      "metadata": {
        "backend": "zfs",
        "compression": "lz4",
        "deduplication": true
      },
      "status": "healthy"
    }
  ],
  "ecosystem_stats": {
    "total_services": 3,
    "coordinators": 1,
    "compute_nodes": 1,
    "storage_nodes": 1,
    "capabilities": 9
  }
}
EOF
    
    echo -e "${GREEN}✅ Discovered complete ecosystem!${NC}"
    echo ""
    cat "$OUTPUT_DIR/ecosystem-topology.json" | jq '.ecosystem_stats'
fi
echo ""

# Step 2: Visualize ecosystem topology
echo "Step 2: Ecosystem topology..."
echo ""
echo "   ┌──────────────────────────────────────────────────┐"
echo "   │          COMPLETE COMPUTE PIPELINE               │"
echo "   └──────────────────────────────────────────────────┘"
echo ""
echo "                        User"
echo "                         │"
echo "                         │ 1. Submit ML training job"
echo "                         ↓"
echo "                   🎵 Songbird"
echo "                  (Orchestrator)"
echo "                         │"
echo "            ┌────────────┼────────────┐"
echo "            │                         │"
echo "  2. Route compute          3. Configure storage"
echo "            │                         │"
echo "            ↓                         ↓"
echo "      🍄 ToadStool              🗄️  NestGate"
echo "    (Compute Engine)         (Persistent Storage)"
echo "            │                         │"
echo "  4. Execute training      5. Store checkpoints"
echo "            │                         │"
echo "            └────────────┬────────────┘"
echo "                         │"
echo "                6. Store final results"
echo "                         │"
echo "                         ↓"
echo "                   🗄️  NestGate"
echo "                         │"
echo "                         │ 7. Return results"
echo "                         ↓"
echo "                   🎵 Songbird"
echo "                         │"
echo "                         │ 8. Deliver to user"
echo "                         ↓"
echo "                        User"
echo ""

# Step 3: Submit ML training job
echo "Step 3: Submitting ML training job through Songbird..."
echo ""

TRAINING_JOB="$OUTPUT_DIR/training-job.json"
cat > "$TRAINING_JOB" <<EOF
{
  "job_id": "ml-training-$(date +%s)",
  "job_type": "ml_training",
  "model": "mnist_classifier",
  "dataset": "mnist",
  "training_config": {
    "epochs": 10,
    "batch_size": 64,
    "learning_rate": 0.001,
    "optimizer": "adam"
  },
  "orchestration": {
    "coordinator": "songbird",
    "compute_service": "auto_discover",
    "storage_service": "auto_discover",
    "checkpoint_frequency": 2,
    "persist_results": true,
    "result_versioning": true
  },
  "storage_config": {
    "checkpoint_path": "ml/checkpoints/mnist",
    "final_model_path": "ml/models/mnist",
    "result_metadata": {
      "project": "mnist_demo",
      "tags": ["demo", "mnist", "training"],
      "retention": "permanent"
    }
  }
}
EOF

echo -e "${CYAN}   Training Job Definition:${NC}"
cat "$TRAINING_JOB" | jq '.training_config, .orchestration'
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [User → Songbird] Submitting training job...${NC}"
    sleep 0.5
    JOB_ID="ml-training-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
    echo -e "${GREEN}   ✅ Job accepted: $JOB_ID${NC}"
fi
echo ""

# Step 4: Songbird orchestrates execution
echo "Step 4: Songbird orchestrating execution..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [Songbird] Analyzing job requirements...${NC}"
    echo "     • Need: Compute for ML training"
    echo "     • Need: Storage for checkpoints and results"
    sleep 0.5
    echo ""
    
    echo -e "${PURPLE}   [Songbird] Discovering compute service...${NC}"
    sleep 0.3
    echo -e "${BLUE}     → Found: ToadStool at $TOADSTOOL_ENDPOINT${NC}"
    echo "       Capabilities: native, container, gpu"
    sleep 0.4
    echo ""
    
    echo -e "${PURPLE}   [Songbird] Discovering storage service...${NC}"
    sleep 0.3
    echo -e "${ORANGE}     → Found: NestGate at $NESTGATE_ENDPOINT${NC}"
    echo "       Capabilities: persistent_storage, versioning"
    sleep 0.4
    echo ""
    
    echo -e "${PURPLE}   [Songbird] Configuring checkpoint pipeline...${NC}"
    sleep 0.3
    echo "     • ToadStool will execute training"
    echo "     • Checkpoints every 2 epochs → NestGate"
    echo "     • Final model → NestGate with versioning"
    sleep 0.4
    echo ""
    
    echo -e "${PURPLE}   [Songbird → ToadStool] Dispatching training workload...${NC}"
    sleep 0.5
fi
echo ""

# Step 5: ToadStool executes with checkpoint saves
echo "Step 5: ToadStool executing training with NestGate checkpoints..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${BLUE}   [ToadStool] Initializing training environment...${NC}"
    sleep 0.4
    echo -e "${BLUE}   [ToadStool] Loading MNIST dataset (60,000 samples)...${NC}"
    sleep 0.5
    echo -e "${BLUE}   [ToadStool] Initializing model (784 → 128 → 64 → 10)...${NC}"
    sleep 0.4
    echo ""
    
    echo -e "${CYAN}   Training Progress:${NC}"
    echo ""
    
    # Training loop with checkpoints
    for epoch in 1 2 3 4; do
        loss=$(awk "BEGIN {printf \"%.4f\", 2.3 - ($epoch * 0.3)}")
        accuracy=$(awk "BEGIN {printf \"%.3f\", 0.10 + ($epoch * 0.15)}")
        
        echo -e "${BLUE}   Epoch $epoch/10:${NC}"
        echo "     • Loss: $loss"
        echo "     • Accuracy: $accuracy"
        echo "     • Batches: 937/937"
        sleep 0.5
        
        # Checkpoint every 2 epochs
        if [ $((epoch % 2)) -eq 0 ]; then
            echo ""
            echo -e "${BLUE}   [ToadStool] Generating checkpoint (epoch $epoch)...${NC}"
            sleep 0.3
            
            CHECKPOINT_FILE="$OUTPUT_DIR/checkpoint_epoch_${epoch}.bin"
            dd if=/dev/urandom of="$CHECKPOINT_FILE" bs=1K count=512 2>/dev/null
            CHECKPOINT_SIZE=$(wc -c < "$CHECKPOINT_FILE")
            
            echo -e "${BLUE}   [ToadStool → NestGate] Saving checkpoint...${NC}"
            sleep 0.4
            echo -e "${ORANGE}   [NestGate] Storing checkpoint (${CHECKPOINT_SIZE} bytes)...${NC}"
            sleep 0.3
            echo -e "${ORANGE}   [NestGate] Compressing with LZ4...${NC}"
            sleep 0.2
            COMPRESSED_SIZE=$((CHECKPOINT_SIZE * 70 / 100))
            echo -e "${ORANGE}   [NestGate] Compressed to ${COMPRESSED_SIZE} bytes (30% savings)${NC}"
            sleep 0.2
            echo -e "${GREEN}   ✅ Checkpoint saved: ml/checkpoints/mnist/epoch_${epoch}${NC}"
        fi
        
        echo ""
    done
    
    echo -e "${CYAN}   ... (continuing training) ...${NC}"
    sleep 0.5
    echo ""
    
    echo -e "${BLUE}   Epoch 10/10:${NC}"
    echo "     • Loss: 0.341"
    echo "     • Accuracy: 0.952"
    echo "     • Batches: 937/937"
    sleep 0.5
    echo ""
    
    echo -e "${GREEN}   ✅ Training complete!${NC}"
    echo "     • Final accuracy: 95.2%"
    echo "     • Total time: 4m 23s"
    echo "     • Checkpoints saved: 5"
fi
echo ""

# Step 6: Save final model to NestGate
echo "Step 6: Saving final model to NestGate with versioning..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    FINAL_MODEL="$OUTPUT_DIR/mnist_classifier_final.bin"
    dd if=/dev/urandom of="$FINAL_MODEL" bs=1K count=1024 2>/dev/null
    MODEL_SIZE=$(wc -c < "$FINAL_MODEL")
    
    MODEL_METADATA="$OUTPUT_DIR/model_metadata.json"
    cat > "$MODEL_METADATA" <<EOF
{
  "model_name": "mnist_classifier",
  "version": "1.0.0",
  "accuracy": 0.952,
  "loss": 0.341,
  "epochs_trained": 10,
  "dataset": "mnist",
  "architecture": "dense_784_128_64_10",
  "framework": "rust_native",
  "created_at": "$(date -Iseconds)",
  "tags": ["production", "mnist", "demo", "v1"],
  "training_job_id": "$JOB_ID",
  "coordinator": "songbird"
}
EOF
    
    echo -e "${BLUE}   [ToadStool] Preparing final model...${NC}"
    sleep 0.3
    echo -e "${BLUE}   [ToadStool → NestGate] Uploading model (${MODEL_SIZE} bytes)...${NC}"
    sleep 0.5
    
    echo -e "${ORANGE}   [NestGate] Receiving model...${NC}"
    sleep 0.4
    echo -e "${ORANGE}   [NestGate] Storing metadata...${NC}"
    cat "$MODEL_METADATA" | jq '{model_name, version, accuracy, tags}'
    sleep 0.4
    echo ""
    echo -e "${ORANGE}   [NestGate] Compressing with LZ4...${NC}"
    sleep 0.3
    COMPRESSED_MODEL_SIZE=$((MODEL_SIZE * 65 / 100))
    echo -e "${ORANGE}   [NestGate] Compressed to ${COMPRESSED_MODEL_SIZE} bytes (35% savings)${NC}"
    sleep 0.3
    
    echo -e "${ORANGE}   [NestGate] Creating ZFS snapshot...${NC}"
    sleep 0.3
    echo -e "${ORANGE}   [NestGate] Generating content hash...${NC}"
    MODEL_HASH=$(md5sum "$FINAL_MODEL" | cut -d' ' -f1)
    sleep 0.3
    
    echo ""
    echo -e "${GREEN}   ✅ Model saved successfully!${NC}"
    echo "     • Storage ID: model-$MODEL_HASH"
    echo "     • Path: ml/models/mnist/v1.0.0"
    echo "     • Original size: ${MODEL_SIZE} bytes"
    echo "     • Stored size: ${COMPRESSED_MODEL_SIZE} bytes"
    echo "     • Version: 1.0.0"
    echo "     • Tags: production, mnist, demo"
fi
echo ""

# Step 7: Results aggregation and reporting
echo "Step 7: Songbird aggregating results and reporting..."
echo ""

FINAL_RESULTS="$OUTPUT_DIR/final_results.json"
cat > "$FINAL_RESULTS" <<EOF
{
  "job_id": "$JOB_ID",
  "status": "completed",
  "training_results": {
    "model_name": "mnist_classifier",
    "version": "1.0.0",
    "final_accuracy": 0.952,
    "final_loss": 0.341,
    "epochs_completed": 10,
    "training_time_seconds": 263
  },
  "storage_results": {
    "checkpoints_saved": 5,
    "checkpoint_storage_path": "ml/checkpoints/mnist",
    "final_model_storage_id": "model-$MODEL_HASH",
    "final_model_path": "ml/models/mnist/v1.0.0",
    "total_storage_used_mb": $(( (CHECKPOINT_SIZE * 5 + MODEL_SIZE) / 1024 )),
    "compression_ratio": 0.68
  },
  "execution_details": {
    "compute_service": "toadstool-local",
    "storage_service": "nestgate-local",
    "coordinator": "songbird-local",
    "compute_time_seconds": 263,
    "storage_time_seconds": 12,
    "coordination_overhead_seconds": 3
  },
  "next_steps": {
    "model_accessible_at": "nestgate://ml/models/mnist/v1.0.0",
    "can_resume_from": "nestgate://ml/checkpoints/mnist/epoch_10",
    "queryable_by_tags": ["production", "mnist", "demo"]
  }
}
EOF

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [ToadStool → Songbird] Reporting completion...${NC}"
    sleep 0.3
    echo -e "${ORANGE}   [NestGate → Songbird] Confirming storage...${NC}"
    sleep 0.3
    echo -e "${PURPLE}   [Songbird] Aggregating results from all services...${NC}"
    sleep 0.4
    echo -e "${PURPLE}   [Songbird] Generating final report...${NC}"
    sleep 0.3
    echo ""
fi

echo -e "${GREEN}✅ Pipeline complete!${NC}"
echo ""
cat "$FINAL_RESULTS" | jq '.training_results, .storage_results'
echo ""

# Step 8: Demonstrate retrieval
echo "Step 8: Demonstrating model retrieval from NestGate..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [User] Query: Find production MNIST models${NC}"
    sleep 0.5
    echo ""
    echo -e "${ORANGE}   [NestGate] Searching by tag 'production' AND 'mnist'...${NC}"
    sleep 0.4
    echo "   {
     \"results\": [
       {
         \"model_name\": \"mnist_classifier\",
         \"version\": \"1.0.0\",
         \"accuracy\": 0.952,
         \"storage_id\": \"model-$MODEL_HASH\",
         \"tags\": [\"production\", \"mnist\", \"demo\", \"v1\"]
       }
     ],
     \"count\": 1
   }"
    sleep 0.5
    echo ""
    
    echo -e "${PURPLE}   [User] Retrieve model for inference${NC}"
    sleep 0.3
    echo -e "${ORANGE}   [NestGate] Loading model-$MODEL_HASH...${NC}"
    sleep 0.4
    echo -e "${ORANGE}   [NestGate] Decompressing...${NC}"
    sleep 0.3
    echo -e "${GREEN}   ✅ Model ready for inference!${NC}"
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Complete pipeline demo finished!"
echo ""
echo "📊 Pipeline Summary:"
echo "   • Training: 10 epochs, 95.2% accuracy"
echo "   • Checkpoints: 5 saved automatically"
echo "   • Final model: Versioned and stored"
echo "   • Compression: 32% storage savings"
echo "   • Total time: 4m 38s (263s training + 15s overhead)"
echo ""
echo "💡 What you learned:"
echo "   • Songbird orchestrates the entire pipeline"
echo "   • ToadStool executes compute workloads"
echo "   • NestGate provides persistent storage"
echo "   • Checkpoints saved automatically during training"
echo "   • Final models versioned and queryable"
echo "   • Zero data loss (everything persisted)"
echo "   • Efficient storage (compression + deduplication)"
echo ""
echo "🎯 Key integration patterns:"
echo "   • Capability-based service discovery"
echo "   • Automatic checkpoint pipeline"
echo "   • Versioned model storage"
echo "   • Metadata-driven queries"
echo "   • Compression and deduplication"
echo "   • End-to-end orchestration"
echo ""
echo "🌟 Production benefits:"
echo "   • Training never lost (checkpoints every N epochs)"
echo "   • Models automatically versioned"
echo "   • Easy rollback (retrieve any checkpoint)"
echo "   • Efficient storage (ZFS compression + dedup)"
echo "   • Queryable by metadata (tags, accuracy, version)"
echo "   • Reproducible (all artifacts preserved)"
echo ""
echo "📂 Output saved to: $OUTPUT_DIR"
echo ""
echo "🔗 Next steps:"
echo "   • Try: ../01-complete-ml-pipeline/demo-full-pipeline.sh (all primals)"
echo "   • Try: ../02-encrypted-storage/ (add BearDog encryption)"
echo "   • See: ../../nestgate-integration/ for more storage patterns"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Three-primal coordination achieved!"
echo "   🎵 Songbird: Orchestration"
echo "   🍄 ToadStool: Computation"
echo "   🗄️  NestGate: Persistence"
echo ""

