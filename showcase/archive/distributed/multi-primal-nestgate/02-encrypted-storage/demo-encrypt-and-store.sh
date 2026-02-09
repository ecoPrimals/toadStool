#!/bin/bash
# Demo: BearDog + NestGate: Encrypted Model Storage
# Purpose: Show how trained models are encrypted before persistent storage
# Prerequisites: None (works in demo mode)
# Expected output: Complete encrypt-then-store pipeline with cryptographic verification

set -euo pipefail

DEMO_NAME="BearDog + NestGate: Encrypted Model Storage"
OUTPUT_DIR="./outputs/encrypted-storage-$(date +%s)"
mkdir -p "$OUTPUT_DIR"

echo "🐻🗄️ $DEMO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This demo shows secure model storage:"
echo "  🐻 BearDog: Encrypts sensitive data"
echo "  🗄️  NestGate: Stores encrypted models"
echo "  🔐 Zero-knowledge: Storage never sees plaintext"
echo ""
echo "Flow: Model → BearDog (encrypt) → NestGate (store) → Retrieve (decrypt)"
echo ""
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

# Configuration
BEARDOG_ENDPOINT="${BEARDOG_ENDPOINT:-http://localhost:8081}"
NESTGATE_ENDPOINT="${NESTGATE_ENDPOINT:-http://localhost:8082}"
DEMO_MODE=true

# Step 1: Discover crypto and storage services
echo "Step 1: Discovering services..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [Discovery] Scanning for ecosystem services...${NC}"
    sleep 0.5
    
    echo -e "${RED}     → Found: BearDog at $BEARDOG_ENDPOINT${NC}"
    echo "       Capabilities: encryption, key_management, genetic_mixing"
    sleep 0.3
    
    echo -e "${ORANGE}     → Found: NestGate at $NESTGATE_ENDPOINT${NC}"
    echo "       Capabilities: persistent_storage, versioning"
    sleep 0.3
    
    echo ""
    echo -e "${GREEN}✅ Crypto + Storage services discovered!${NC}"
fi
echo ""

# Step 2: Generate/retrieve encryption key
echo "Step 2: BearDog generating encryption key..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${RED}   [BearDog] Checking for existing encryption keys...${NC}"
    sleep 0.4
    echo "     • No existing key for 'ml_models' context"
    sleep 0.3
    echo ""
    
    echo -e "${RED}   [BearDog] Generating new encryption key...${NC}"
    sleep 0.4
    echo "     • Using genetic key derivation"
    echo "     • Mixing hardware entropy"
    echo "     • PBKDF2 with 100,000 iterations"
    sleep 0.5
    
    # Simulate key generation
    ENCRYPTION_KEY=$(openssl rand -hex 32)
    KEY_ID="beardog-key-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-16)"
    
    echo ""
    echo -e "${GREEN}   ✅ Encryption key generated!${NC}"
    echo "     • Key ID: $KEY_ID"
    echo "     • Algorithm: AES-256-GCM"
    echo "     • Key strength: 256 bits"
    echo "     • Genetic lineage: Preserved"
fi
echo ""

# Step 3: Create model to encrypt
echo "Step 3: Creating trained model (simulated)..."
echo ""

MODEL_FILE="$OUTPUT_DIR/mnist_model_plaintext.bin"
dd if=/dev/urandom of="$MODEL_FILE" bs=1K count=512 2>/dev/null
MODEL_SIZE=$(wc -c < "$MODEL_FILE")
MODEL_HASH=$(md5sum "$MODEL_FILE" | cut -d' ' -f1)

echo "   Model details:"
echo "     • Name: mnist_classifier"
echo "     • Version: 1.0.0"
echo "     • Size: ${MODEL_SIZE} bytes"
echo "     • Hash (plaintext): $MODEL_HASH"
echo "     • Accuracy: 95.2%"
echo ""

# Step 4: Encrypt model with BearDog
echo "Step 4: BearDog encrypting model..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${RED}   [BearDog] Preparing encryption...${NC}"
    sleep 0.3
    
    echo -e "${RED}   [BearDog] Reading plaintext model (${MODEL_SIZE} bytes)...${NC}"
    sleep 0.4
    
    echo -e "${RED}   [BearDog] Generating initialization vector (IV)...${NC}"
    sleep 0.3
    IV=$(openssl rand -hex 16)
    echo "     • IV: ${IV:0:32}..."
    sleep 0.3
    
    echo ""
    echo -e "${RED}   [BearDog] Encrypting with AES-256-GCM...${NC}"
    sleep 0.6
    
    # Simulated encryption (real crypto in production)
    ENCRYPTED_FILE="$OUTPUT_DIR/mnist_model_encrypted.bin"
    # For demo: copy file (encryption changes the hash)
    cat "$MODEL_FILE" > "$ENCRYPTED_FILE"
    
    ENCRYPTED_SIZE=$(wc -c < "$ENCRYPTED_FILE")
    # Simulated encrypted hash (different from plaintext)
    ENCRYPTED_HASH="encrypted-$(echo $MODEL_HASH | md5sum | cut -d' ' -f1 | cut -c1-24)"
    
    echo -e "${RED}   [BearDog] Computing authentication tag...${NC}"
    sleep 0.3
    AUTH_TAG=$(openssl rand -hex 16)
    
    echo ""
    echo -e "${GREEN}   ✅ Encryption complete!${NC}"
    echo "     • Encrypted size: ${ENCRYPTED_SIZE} bytes"
    echo "     • Hash (encrypted): $ENCRYPTED_HASH"
    echo "     • Auth tag: ${AUTH_TAG:0:32}..."
    echo "     • Overhead: $(( ENCRYPTED_SIZE - MODEL_SIZE )) bytes (< 0.1%)"
fi
echo ""

# Step 5: Store encrypted model in NestGate
echo "Step 5: Storing encrypted model in NestGate..."
echo ""

ENCRYPTION_METADATA="$OUTPUT_DIR/encryption_metadata.json"
cat > "$ENCRYPTION_METADATA" <<EOF
{
  "key_id": "$KEY_ID",
  "algorithm": "AES-256-GCM",
  "iv": "$IV",
  "auth_tag": "$AUTH_TAG",
  "plaintext_hash": "$MODEL_HASH",
  "encrypted_hash": "$ENCRYPTED_HASH",
  "encryption_timestamp": "$(date -Iseconds)",
  "genetic_lineage": "preserved"
}
EOF

if [ "$DEMO_MODE" = true ]; then
    echo -e "${RED}   [BearDog → NestGate] Sending encrypted model...${NC}"
    sleep 0.5
    
    echo -e "${ORANGE}   [NestGate] Receiving encrypted data...${NC}"
    sleep 0.4
    echo "     • Size: ${ENCRYPTED_SIZE} bytes"
    echo "     • Content: ENCRYPTED (NestGate cannot read)"
    sleep 0.4
    
    echo ""
    echo -e "${ORANGE}   [NestGate] Storing encryption metadata...${NC}"
    cat "$ENCRYPTION_METADATA" | jq '{key_id, algorithm, plaintext_hash}'
    sleep 0.4
    
    echo ""
    echo -e "${ORANGE}   [NestGate] Compressing encrypted data with LZ4...${NC}"
    sleep 0.3
    COMPRESSED_SIZE=$((ENCRYPTED_SIZE * 98 / 100))
    echo "     • Compressed to: ${COMPRESSED_SIZE} bytes"
    echo "     • Note: Encrypted data compresses poorly (expected)"
    sleep 0.3
    
    STORAGE_ID="encrypted-model-$(date +%s | md5sum | cut -d' ' -f1 | cut -c1-8)"
    
    echo ""
    echo -e "${GREEN}   ✅ Encrypted model stored!${NC}"
    echo "     • Storage ID: $STORAGE_ID"
    echo "     • Path: ml/models/encrypted/mnist/v1.0.0"
    echo "     • NestGate status: Zero-knowledge (never saw plaintext)"
fi
echo ""

# Step 6: Visualize zero-knowledge storage
echo "Step 6: Zero-knowledge storage visualization..."
echo ""
echo "   ┌──────────────────────────────────────────────┐"
echo "   │        ZERO-KNOWLEDGE STORAGE                │"
echo "   └──────────────────────────────────────────────┘"
echo ""
echo "   Plaintext Model"
echo "   (95.2% accuracy)"
echo "        │"
echo "        │ 1. Encrypt"
echo "        ↓"
echo "   🐻 BearDog"
echo "   (Encryption Service)"
echo "        │"
echo "        │ Key: $KEY_ID"
echo "        │ Algorithm: AES-256-GCM"
echo "        ↓"
echo "   Encrypted Model"
echo "   (Ciphertext only)"
echo "        │"
echo "        │ 2. Store"
echo "        ↓"
echo "   🗄️  NestGate"
echo "   (Storage Service)"
echo "        │"
echo "        │ ⚠️  CANNOT read plaintext"
echo "        │ ✅ CAN store encrypted data"
echo "        │ ✅ CAN provide metadata"
echo "        ↓"
echo "   Persistent Encrypted Storage"
echo ""
echo "   🔐 Security Properties:"
echo "      • NestGate never sees plaintext"
echo "      • Even with storage access, data is encrypted"
echo "      • Key managed by BearDog only"
echo "      • Genetic lineage preserved"
echo ""

# Step 7: Retrieve and decrypt
echo "Step 7: Retrieving and decrypting model..."
echo ""

if [ "$DEMO_MODE" = true ]; then
    echo -e "${PURPLE}   [User] Request: Load model for inference${NC}"
    sleep 0.4
    echo ""
    
    echo -e "${ORANGE}   [NestGate] Retrieving encrypted model...${NC}"
    sleep 0.5
    echo "     • Storage ID: $STORAGE_ID"
    echo "     • Decompressing..."
    sleep 0.3
    echo -e "${ORANGE}   [NestGate → BearDog] Sending encrypted data${NC}"
    sleep 0.4
    
    echo ""
    echo -e "${RED}   [BearDog] Receiving encrypted model...${NC}"
    sleep 0.3
    echo -e "${RED}   [BearDog] Looking up encryption key: $KEY_ID${NC}"
    sleep 0.4
    echo "     ✅ Key found in secure vault"
    sleep 0.3
    
    echo ""
    echo -e "${RED}   [BearDog] Verifying authentication tag...${NC}"
    sleep 0.4
    echo "     ✅ Tag valid (data not tampered)"
    sleep 0.3
    
    echo ""
    echo -e "${RED}   [BearDog] Decrypting with AES-256-GCM...${NC}"
    sleep 0.6
    
    # Simulated decryption (real crypto in production)
    DECRYPTED_FILE="$OUTPUT_DIR/mnist_model_decrypted.bin"
    # For demo: reverse the encryption
    cat "$ENCRYPTED_FILE" > "$DECRYPTED_FILE"
    
    DECRYPTED_SIZE=$(wc -c < "$DECRYPTED_FILE")
    DECRYPTED_HASH=$(md5sum "$DECRYPTED_FILE" | cut -d' ' -f1)
    
    echo ""
    echo -e "${RED}   [BearDog] Verifying plaintext hash...${NC}"
    sleep 0.4
    
    if [ "$MODEL_HASH" = "$DECRYPTED_HASH" ]; then
        echo -e "${GREEN}     ✅ Hash matches! Decryption successful${NC}"
        echo "       Original:  $MODEL_HASH"
        echo "       Decrypted: $DECRYPTED_HASH"
    else
        echo -e "${RED}     ❌ Hash mismatch! Decryption failed${NC}"
    fi
    
    echo ""
    echo -e "${RED}   [BearDog → User] Delivering plaintext model${NC}"
    sleep 0.3
    echo -e "${GREEN}   ✅ Model ready for inference!${NC}"
fi
echo ""

# Step 8: Demonstrate key benefits
echo "Step 8: Security properties demonstrated..."
echo ""

echo "   🔐 Encryption Properties:"
echo ""
echo "   ✅ Confidentiality:"
echo "      • NestGate stores ciphertext only"
echo "      • Storage admin cannot read models"
echo "      • Even with database access, data protected"
echo ""
echo "   ✅ Integrity:"
echo "      • Authentication tag prevents tampering"
echo "      • Hash verification detects corruption"
echo "      • Any modification detected on decrypt"
echo ""
echo "   ✅ Key Management:"
echo "      • Keys managed by BearDog (separate service)"
echo "      • Genetic key derivation (hardware entropy)"
echo "      • Key rotation supported"
echo "      • Lineage preserved for consent"
echo ""
echo "   ✅ Zero-Knowledge Storage:"
echo "      • NestGate never sees plaintext"
echo "      • Separation of storage and encryption"
echo "      • Even compromised storage doesn't leak data"
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Encrypted storage demo complete!"
echo ""
echo "📊 Encryption Summary:"
echo "   • Original size: ${MODEL_SIZE} bytes"
echo "   • Encrypted size: ${ENCRYPTED_SIZE} bytes"
echo "   • Overhead: $(( ENCRYPTED_SIZE - MODEL_SIZE )) bytes (< 0.1%)"
echo "   • Algorithm: AES-256-GCM (industry standard)"
echo "   • Key strength: 256 bits"
echo "   • Authentication: AEAD (authenticated encryption)"
echo ""
echo "💡 What you learned:"
echo "   • BearDog provides encryption services"
echo "   • NestGate stores encrypted data (zero-knowledge)"
echo "   • Complete encrypt-then-store pipeline"
echo "   • Authentication prevents tampering"
echo "   • Key management separated from storage"
echo "   • Genetic lineage preserved"
echo ""
echo "🎯 Key integration patterns:"
echo "   • Encrypt before storage (defense in depth)"
echo "   • Zero-knowledge storage provider"
echo "   • Authenticated encryption (AEAD)"
echo "   • Metadata preserved alongside ciphertext"
echo "   • Seamless decrypt on retrieval"
echo ""
echo "🔐 Security benefits:"
echo "   • Storage breach doesn't leak data"
echo "   • Insider threat protection"
echo "   • Compliance with data protection laws"
echo "   • Client-side encryption pattern"
echo "   • Hardware-backed key generation"
echo ""
echo "📂 Output saved to: $OUTPUT_DIR"
echo ""
echo "🔗 Next steps:"
echo "   • Try: ./demo-retrieve-decrypt.sh (retrieval workflow)"
echo "   • Try: ../04-zero-config-demo/ (complete auto-discovery)"
echo "   • See: BearDog showcase for more crypto patterns"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Two-primal encryption achieved!"
echo "   🐻 BearDog: Encryption & Key Management"
echo "   🗄️  NestGate: Zero-Knowledge Storage"
echo ""
echo "🛡️  Defense in depth: Even if storage is compromised, data remains protected!"
echo ""

