#!/bin/bash
# Demo: Complete ML Pipeline - All Primals Integration
# Purpose: Show Songbird + ToadStool + NestGate + BearDog working together
# Prerequisites: None (works in demo mode)
# Expected output: Complete secure ML pipeline with all ecosystem services

set -euo pipefail

DEMO_NAME="Complete ML Pipeline: All Primals Integration"
OUTPUT_DIR="./outputs/complete-pipeline-all-primals-$(date +%s)"
mkdir -p "$OUTPUT_DIR"

echo "🎵🍄🗄️🐻 $DEMO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "THE ULTIMATE DEMO: All 4 primals working together!"
echo ""
echo "  🎵 Songbird: Orchestrates and coordinates"
echo "  🍄 ToadStool: Executes ML training"
echo "  🐻 BearDog: Encrypts sensitive data"
echo "  🗄️  NestGate: Stores encrypted models"
echo ""
echo "Flow: Complete Secure ML Pipeline"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
ORANGE='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

DEMO_MODE=true

# Step 1: Discover complete ecosystem
echo "Step 1: Discovering complete ecosystem..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [Songbird] Discovering all ecosystem services...${NC}"
    sleep 0.6
    
    echo ""
    echo -e "${PURPLE}   [Songbird] Found 4 primal services:${NC}"
    sleep 0.3
    echo -e "${PURPLE}     • Songbird (coordinator) - localhost:8000${NC}"
    sleep 0.2
    echo -e "${BLUE}     • ToadStool (compute) - localhost:8080${NC}"
    sleep 0.2
    echo -e "${RED}     • BearDog (crypto) - localhost:8081${NC}"
    sleep 0.2
    echo -e "${ORANGE}     • NestGate (storage) - localhost:8082${NC}"
    sleep 0.3
    
    echo ""
    echo -e "${GREEN}✅ Complete ecosystem discovered!${NC}"
fi
echo ""

# Step 2: Visualize complete ecosystem
echo "Step 2: Complete ecosystem architecture..."
echo ""
echo "   ┌────────────────────────────────────────────────────────┐"
echo "   │         COMPLETE ML PIPELINE (4 PRIMALS)               │"
echo "   └────────────────────────────────────────────────────────┘"
echo ""
echo "                          User"
echo "                            │"
echo "                            │ 1. Submit ML job"
echo "                            ↓"
echo "                      🎵 Songbird"
echo "                    (Orchestrator)"
echo "                            │"
echo "          ┌─────────────────┼─────────────────┐"
echo "          │                 │                 │"
echo "    2. Route          3. Configure      4. Configure"
echo "     compute           crypto           storage"
echo "          │                 │                 │"
echo "          ↓                 ↓                 ↓"
echo "    🍄 ToadStool       🐻 BearDog      🗄️  NestGate"
echo "   (ML Training)     (Encryption)    (Storage)"
echo "          │                 │                 │"
echo "          │                 │                 │"
echo "    5. Train model           │                 │"
echo "          │                 │                 │"
echo "          │  6. Generate checkpoint           │"
echo "          ├──────────►│                       │"
echo "          │           │ 7. Encrypt            │"
echo "          │           ├──────────────────────►│"
echo "          │           │                8. Store│"
echo "          │           │                  (encrypted)"
echo "          │           │                       │"
echo "          │  9. Training complete             │"
echo "          ├──────────►│                       │"
echo "          │           │ 10. Encrypt model     │"
echo "          │           ├──────────────────────►│"
echo "          │           │             11. Store │"
echo "          │           │              (encrypted)"
echo "          │           │                       │"
echo "          └───────────┴───────────────────────┘"
echo "                            │"
echo "                   12. Report complete"
echo "                            │"
echo "                            ↓"
echo "                      🎵 Songbird"
echo "                            │"
echo "                   13. Return results"
echo "                            ↓"
echo "                          User"
echo ""

# Step 3: Submit secure ML training job
echo "Step 3: Submitting secure ML training job..."
echo ""

JOB_CONFIG="$OUTPUT_DIR/secure-ml-job.json"
cat > "$JOB_CONFIG" <<EOF
{
  "job_id": "secure-ml-$(date +%s)",
  "job_type": "secure_ml_training",
  "model": "mnist_classifier",
  "dataset": "mnist",
  "training_config": {
    "epochs": 10,
    "batch_size": 64,
    "learning_rate": 0.001
  },
  "security_config": {
    "encrypt_checkpoints": true,
    "encrypt_final_model": true,
    "encryption_service": "auto_discover",
    "key_derivation": "genetic_hardware_entropy"
  },
  "storage_config": {
    "checkpoint_frequency": 2,
    "persist_encrypted": true,
    "storage_service": "auto_discover",
    "zero_knowledge": true
  },
  "orchestration": {
    "coordinator": "songbird",
    "compute_service": "auto_discover",
    "crypto_service": "auto_discover",
    "storage_service": "auto_discover"
  }
}
EOF

echo -e "${CYAN}   Secure ML Job Configuration:${NC}"
cat "$JOB_CONFIG" | jq '.security_config, .storage_config.zero_knowledge'
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [User → Songbird] Submitting secure ML job...${NC}"
    sleep 0.5
    JOB_ID="secure-ml-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
    echo -e "${GREEN}   ✅ Job accepted: $JOB_ID${NC}"
fi
echo ""

# Step 4: Songbird orchestrates the complete pipeline
echo "Step 4: Songbird orchestrating 4-primal pipeline..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [Songbird] Analyzing security requirements...${NC}"
    echo "     • Checkpoints must be encrypted ✓"
    echo "     • Final model must be encrypted ✓"
    echo "     • Zero-knowledge storage required ✓"
    sleep 0.5
    echo ""
    
    echo -e "${PURPLE}   [Songbird] Discovering services...${NC}"
    sleep 0.3
    echo -e "${BLUE}     → Compute: ToadStool (ML training capability)${NC}"
    sleep 0.3
    echo -e "${RED}     → Crypto: BearDog (AES-256-GCM encryption)${NC}"
    sleep 0.3
    echo -e "${ORANGE}     → Storage: NestGate (zero-knowledge storage)${NC}"
    sleep 0.3
    
    echo ""
    echo -e "${PURPLE}   [Songbird] Configuring secure pipeline:${NC}"
    sleep 0.3
    echo "     1. ToadStool trains model"
    echo "     2. Every 2 epochs → BearDog encrypts checkpoint"
    echo "     3. Encrypted checkpoint → NestGate stores"
    echo "     4. Final model → BearDog encrypts"
    echo "     5. Encrypted model → NestGate stores"
    sleep 0.5
    
    echo ""
    echo -e "${PURPLE}   [Songbird → ToadStool] Dispatching training workload...${NC}"
    sleep 0.5
fi
echo ""

# Step 5: ToadStool training with encrypted checkpoints
echo "Step 5: ToadStool training with BearDog encryption..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${BLUE}   [ToadStool] Initializing training...${NC}"
    sleep 0.4
    echo -e "${BLUE}   [ToadStool] Loading MNIST (60,000 samples)...${NC}"
    sleep 0.5
    echo ""
    
    echo -e "${CYAN}   Training with Encrypted Checkpoints:${NC}"
    echo ""
    
    # Training loop with encrypted checkpoints
    for epoch in 1 2 3 4; do
        loss=$(awk "BEGIN {printf \"%.4f\", 2.3 - ($epoch * 0.3)}")
        accuracy=$(awk "BEGIN {printf \"%.3f\", 0.10 + ($epoch * 0.15)}")
        
        echo -e "${BLUE}   Epoch $epoch/10:${NC} Loss: $loss, Accuracy: $accuracy"
        sleep 0.5
        
        # Encrypted checkpoint every 2 epochs
        if [ $((epoch % 2)) -eq 0 ]; then
            echo ""
            echo -e "${BLUE}   [ToadStool] Checkpoint ready (epoch $epoch)${NC}"
            sleep 0.3
            
            echo -e "${BLUE}   [ToadStool → BearDog] Sending checkpoint for encryption...${NC}"
            sleep 0.4
            
            echo -e "${RED}   [BearDog] Encrypting checkpoint (AES-256-GCM)...${NC}"
            sleep 0.5
            CHECKPOINT_KEY="beardog-key-ckpt-$epoch-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
            echo -e "${RED}   [BearDog] Key: $CHECKPOINT_KEY${NC}"
            echo -e "${RED}   [BearDog] ✓ Encrypted (512KB → 512KB + 16B auth tag)${NC}"
            sleep 0.4
            
            echo ""
            echo -e "${RED}   [BearDog → NestGate] Sending encrypted checkpoint...${NC}"
            sleep 0.4
            
            echo -e "${ORANGE}   [NestGate] Receiving ENCRYPTED checkpoint${NC}"
            echo -e "${ORANGE}   [NestGate] ⚠️  Cannot read plaintext (zero-knowledge)${NC}"
            echo -e "${ORANGE}   [NestGate] Storing: ml/checkpoints/encrypted/epoch_${epoch}${NC}"
            sleep 0.4
            
            echo -e "${GREEN}   ✅ Encrypted checkpoint saved securely!${NC}"
            echo ""
        fi
    done
    
    echo -e "${CYAN}   ... (continuing training) ...${NC}"
    sleep 0.5
    echo ""
    
    echo -e "${BLUE}   Epoch 10/10:${NC} Loss: 0.341, Accuracy: 0.952"
    echo -e "${GREEN}   ✅ Training complete! (95.2% accuracy)${NC}"
fi
echo ""

# Step 6: Encrypt and store final model
echo "Step 6: Encrypting and storing final model..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${BLUE}   [ToadStool] Final model ready (95.2% accuracy)${NC}"
    sleep 0.4
    echo -e "${BLUE}   [ToadStool → BearDog] Sending model for encryption...${NC}"
    sleep 0.5
    
    echo ""
    echo -e "${RED}   [BearDog] Generating encryption key...${NC}"
    sleep 0.4
    echo "     • Using genetic key derivation"
    echo "     • Hardware entropy: 256 bits"
    echo "     • Algorithm: AES-256-GCM"
    sleep 0.5
    
    FINAL_MODEL_KEY="beardog-key-final-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
    echo ""
    echo -e "${RED}   [BearDog] Encrypting final model...${NC}"
    echo "     • Key ID: $FINAL_MODEL_KEY"
    echo "     • Plaintext: 1MB model weights"
    echo "     • Ciphertext: 1MB + 16B auth tag"
    sleep 0.6
    
    MODEL_PLAINTEXT_HASH=$(openssl rand -hex 16)
    echo ""
    echo -e "${RED}   [BearDog] Computing plaintext hash: ${MODEL_PLAINTEXT_HASH:0:32}${NC}"
    sleep 0.3
    echo -e "${GREEN}   ✅ Encryption complete!${NC}"
    
    echo ""
    echo -e "${RED}   [BearDog → NestGate] Sending encrypted model...${NC}"
    sleep 0.5
    
    echo -e "${ORANGE}   [NestGate] Receiving ENCRYPTED model${NC}"
    sleep 0.4
    echo -e "${ORANGE}   [NestGate] Size: 1,048,592 bytes (encrypted)${NC}"
    echo -e "${ORANGE}   [NestGate] ⚠️  Content: CIPHERTEXT ONLY${NC}"
    echo -e "${ORANGE}   [NestGate] ⚠️  NestGate CANNOT decrypt (zero-knowledge)${NC}"
    sleep 0.5
    
    echo ""
    echo -e "${ORANGE}   [NestGate] Storing encryption metadata:${NC}"
    echo "     {
       \"key_id\": \"$FINAL_MODEL_KEY\",
       \"algorithm\": \"AES-256-GCM\",
       \"plaintext_hash\": \"${MODEL_PLAINTEXT_HASH:0:32}\",
       \"model_name\": \"mnist_classifier\",
       \"version\": \"1.0.0\",
       \"accuracy\": 0.952,
       \"encrypted\": true
     }"
    sleep 0.5
    
    echo ""
    echo -e "${ORANGE}   [NestGate] Compressing encrypted data...${NC}"
    sleep 0.3
    echo -e "${ORANGE}   [NestGate] Creating ZFS snapshot...${NC}"
    sleep 0.3
    
    STORAGE_ID="encrypted-model-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
    echo ""
    echo -e "${GREEN}   ✅ Encrypted model stored securely!${NC}"
    echo "     • Storage ID: $STORAGE_ID"
    echo "     • Path: ml/models/encrypted/mnist/v1.0.0"
    echo "     • Encrypted: YES (AES-256-GCM)"
    echo "     • Zero-knowledge: YES"
fi
echo ""

# Step 7: Results aggregation
echo "Step 7: Songbird aggregating results..."
echo ""

FINAL_RESULTS="$OUTPUT_DIR/secure-pipeline-results.json"
cat > "$FINAL_RESULTS" <<EOF
{
  "job_id": "$JOB_ID",
  "status": "completed",
  "training_results": {
    "model_name": "mnist_classifier",
    "version": "1.0.0",
    "final_accuracy": 0.952,
    "final_loss": 0.341,
    "epochs_completed": 10
  },
  "security_results": {
    "checkpoints_encrypted": 5,
    "final_model_encrypted": true,
    "encryption_algorithm": "AES-256-GCM",
    "key_derivation": "genetic_hardware_entropy",
    "zero_knowledge_storage": true,
    "encryption_keys": [
      "$FINAL_MODEL_KEY"
    ]
  },
  "storage_results": {
    "encrypted_checkpoints_stored": 5,
    "encrypted_model_stored": true,
    "storage_service": "nestgate",
    "storage_mode": "zero_knowledge",
    "plaintext_never_stored": true
  },
  "services_used": {
    "coordinator": "songbird",
    "compute": "toadstool",
    "encryption": "beardog",
    "storage": "nestgate"
  },
  "security_properties": {
    "confidentiality": "AES-256-GCM encryption",
    "integrity": "AEAD authentication tags",
    "zero_knowledge": "Storage never sees plaintext",
    "key_management": "BearDog genetic derivation",
    "defense_in_depth": "Encryption + Secure Storage"
  }
}
EOF

if [ "$DEMO_MODE" = true ]; then
    echo -e "${BLUE}   [ToadStool → Songbird] Training complete${NC}"
    sleep 0.3
    echo -e "${RED}   [BearDog → Songbird] All data encrypted${NC}"
    sleep 0.3
    echo -e "${ORANGE}   [NestGate → Songbird] All data stored securely${NC}"
    sleep 0.3
    
    echo ""
    echo -e "${PURPLE}   [Songbird] Aggregating results from all 4 primals...${NC}"
    sleep 0.5
fi

echo ""
echo -e "${GREEN}✅ Complete secure ML pipeline finished!${NC}"
echo ""
cat "$FINAL_RESULTS" | jq '.training_results, .security_properties'
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 COMPLETE 4-PRIMAL INTEGRATION SUCCESS!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📊 What Happened:"
echo "   1. 🎵 Songbird orchestrated the entire pipeline"
echo "   2. 🍄 ToadStool trained the ML model (95.2% accuracy)"
echo "   3. 🐻 BearDog encrypted all sensitive data (checkpoints + model)"
echo "   4. 🗄️  NestGate stored encrypted data (zero-knowledge)"
echo ""
echo "🔐 Security Properties Achieved:"
echo "   ✅ End-to-end encryption (all data encrypted before storage)"
echo "   ✅ Zero-knowledge storage (NestGate never saw plaintext)"
echo "   ✅ Authenticated encryption (AEAD with auth tags)"
echo "   ✅ Genetic key derivation (hardware entropy)"
echo "   ✅ Defense in depth (multiple security layers)"
echo ""
echo "💡 Key Integration Patterns:"
echo "   • 4 primals working together seamlessly"
echo "   • Capability-based service discovery"
echo "   • Automatic encryption pipeline"
echo "   • Zero-knowledge storage pattern"
echo "   • Complete orchestration by Songbird"
echo ""
echo "🎯 Production Benefits:"
echo "   • Security: Even storage breach doesn't leak data"
echo "   • Privacy: Client-side encryption pattern"
echo "   • Compliance: Data protection regulations satisfied"
echo "   • Sovereignty: Self-hosted, no cloud dependencies"
echo "   • Auditability: Complete security audit trail"
echo ""
echo "🌟 This is the COMPLETE ecoPrimals vision:"
echo "   → Coordinated (Songbird)"
echo "   → Computed (ToadStool)"
echo "   → Encrypted (BearDog)"
echo "   → Persisted (NestGate)"
echo "   → All working together automatically!"
echo ""
echo "📂 Output saved to: $OUTPUT_DIR"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🚀 You've seen the complete ecosystem in action!"
echo ""
echo "🎵🍄🐻🗄️ **All 4 primals integrated successfully!** 🚀"
echo ""

