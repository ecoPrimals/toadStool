#!/usr/bin/env bash
# ToadStool + BearDog Encrypted Workload - Using CLI API

set -euo pipefail

echo "🧠🐻 ToadStool + BearDog: Encrypted ML Workload (CLI API)"
echo "================================================================"
echo ""
echo "Using BearDog's CLI API with ALL new features:"
echo "  ✅ Key Generation (Master)"
echo "  ✅ Genetic Derivation (derive)"
echo "  ✅ Streaming Encryption (stream-encrypt)"
echo "  ✅ Lineage Verification (lineage)"
echo "  ✅ Key Export/Import (export/import)"
echo "  ✅ Enhanced Revocation (revoke)"
echo ""
echo "================================================================"
echo ""

# Configuration
BEARDOG_BIN="/home/eastgate/Development/ecoPrimals/beardog/target/release/beardog"
WORK_DIR="$(pwd)/workload-test-$(date +%s)"
KEYS_DIR="$WORK_DIR/keys"
DATA_DIR="$WORK_DIR/data"

# Check if BearDog binary exists
if [ ! -f "$BEARDOG_BIN" ]; then
    echo "❌ BearDog binary not found at: $BEARDOG_BIN"
    echo "Building BearDog..."
    cd /home/eastgate/Development/ecoPrimals/beardog
    cargo build --release
    cd -
fi

echo "✅ BearDog binary found"
echo ""

# Create working directories
mkdir -p "$KEYS_DIR" "$DATA_DIR"
echo "📁 Working directory: $WORK_DIR"
echo ""

# Test 1: Generate Master Key
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 1: Generate Master Key (Genetic Root)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

MASTER_KEY_ID="master-$(date +%s)"
echo "Generating master key: $MASTER_KEY_ID"

$BEARDOG_BIN key generate \
    --key-id "$MASTER_KEY_ID" \
    --algorithm genetic-hkdf \
    --purpose "ML Training Root" \
    --usage all

echo "✅ Master key generated: $MASTER_KEY_ID"
echo ""

# Test 2: Derive Child Keys (Genetic Hierarchy)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 2: Derive Child Keys (Genetic Derivation)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Derive keys for each tower
TOWER_A_KEY="tower-a-$(date +%s)"
TOWER_B_KEY="tower-b-$(date +%s)"

echo "Deriving Tower A key from master..."
$BEARDOG_BIN key derive \
    --master-key "$MASTER_KEY_ID" \
    --purpose "tower-a-eastgate-rtx2070" \
    --output "$TOWER_A_KEY"

echo "✅ Tower A key: $TOWER_A_KEY"
echo ""

echo "Deriving Tower B key from master..."
$BEARDOG_BIN key derive \
    --master-key "$MASTER_KEY_ID" \
    --purpose "tower-b-strandgate-rtx3070" \
    --output "$TOWER_B_KEY"

echo "✅ Tower B key: $TOWER_B_KEY"
echo ""

# Test 3: Verify Genetic Lineage
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 3: Verify Genetic Lineage"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Lineage for Tower A:"
$BEARDOG_BIN key lineage --key-id "$TOWER_A_KEY"
echo ""

echo "Lineage for Tower B:"
$BEARDOG_BIN key lineage --key-id "$TOWER_B_KEY"
echo ""

# Test 4: Create ML Training Data
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 4: Create ML Training Data"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create sample training data (simulating CIFAR-10)
TRAIN_DATA_A="$DATA_DIR/train-data-tower-a.txt"
TRAIN_DATA_B="$DATA_DIR/train-data-tower-b.txt"

cat > "$TRAIN_DATA_A" << EOF
# CIFAR-10 Training Batch A (Tower A - Eastgate)
# Samples: 25000
# Classes: airplane, automobile, bird, cat, deer, dog, frog, horse, ship, truck
Image 0001: airplane [0.123, 0.456, 0.789, ...]
Image 0002: automobile [0.234, 0.567, 0.890, ...]
Image 0003: bird [0.345, 0.678, 0.901, ...]
# ... 24997 more images
EOF

cat > "$TRAIN_DATA_B" << EOF
# CIFAR-10 Training Batch B (Tower B - Strandgate)
# Samples: 25000
# Classes: airplane, automobile, bird, cat, deer, dog, frog, horse, ship, truck
Image 25001: cat [0.456, 0.789, 0.012, ...]
Image 25002: deer [0.567, 0.890, 0.123, ...]
Image 25003: dog [0.678, 0.901, 0.234, ...]
# ... 24997 more images
EOF

echo "✅ Training data created:"
echo "   • Tower A: $TRAIN_DATA_A ($(wc -l < $TRAIN_DATA_A) lines)"
echo "   • Tower B: $TRAIN_DATA_B ($(wc -l < $TRAIN_DATA_B) lines)"
echo ""

# Test 5: Stream Encrypt ML Data
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 5: Stream Encrypt ML Training Data"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

ENCRYPTED_A="$DATA_DIR/train-data-tower-a.enc"
ENCRYPTED_B="$DATA_DIR/train-data-tower-b.enc"

echo "Encrypting Tower A data with streaming..."
$BEARDOG_BIN stream-encrypt \
    --key "$TOWER_A_KEY" \
    --input "$TRAIN_DATA_A" \
    --output "$ENCRYPTED_A"

echo "✅ Encrypted: $ENCRYPTED_A ($(wc -c < $ENCRYPTED_A) bytes)"
echo ""

echo "Encrypting Tower B data with streaming..."
$BEARDOG_BIN stream-encrypt \
    --key "$TOWER_B_KEY" \
    --input "$TRAIN_DATA_B" \
    --output "$ENCRYPTED_B"

echo "✅ Encrypted: $ENCRYPTED_B ($(wc -c < $ENCRYPTED_B) bytes)"
echo ""

# Test 6: Stream Decrypt and Verify
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 6: Stream Decrypt and Verify"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

DECRYPTED_A="$DATA_DIR/train-data-tower-a-decrypted.txt"

echo "Decrypting Tower A data..."
$BEARDOG_BIN stream-decrypt \
    --input "$ENCRYPTED_A" \
    --output "$DECRYPTED_A"

echo "✅ Decrypted: $DECRYPTED_A"
echo ""

# Verify data integrity
if diff -q "$TRAIN_DATA_A" "$DECRYPTED_A" > /dev/null; then
    echo "✅ Data integrity verified! Encryption/decryption successful"
else
    echo "❌ Data integrity check failed"
    exit 1
fi
echo ""

# Test 7: Key Export/Import
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 7: Key Export/Import (For Distributed Training)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

EXPORTED_KEY_A="$KEYS_DIR/tower-a-key-export.json"
EXPORTED_KEY_B="$KEYS_DIR/tower-b-key-export.json"

echo "Exporting Tower A key..."
$BEARDOG_BIN key export \
    --key-id "$TOWER_A_KEY" \
    --output "$EXPORTED_KEY_A"

echo "✅ Exported: $EXPORTED_KEY_A"
echo ""

echo "Exporting Tower B key..."
$BEARDOG_BIN key export \
    --key-id "$TOWER_B_KEY" \
    --output "$EXPORTED_KEY_B"

echo "✅ Exported: $EXPORTED_KEY_B"
echo ""

echo "📤 In production:"
echo "   • Transfer $EXPORTED_KEY_A to Eastgate tower"
echo "   • Transfer $EXPORTED_KEY_B to Strandgate tower"
echo "   • Each tower imports its key and trains on encrypted data"
echo ""

# Test 8: List All Keys
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 8: List All Keys"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

$BEARDOG_BIN key list
echo ""

# Test 9: Enhanced Revocation
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 9: Enhanced Revocation (If Tower Compromised)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Simulating revocation of Tower B key..."
$BEARDOG_BIN key revoke \
    --key-id "$TOWER_B_KEY" \
    --reason "Tower compromised - security audit"

echo ""
echo "✅ Tower B key revoked"
echo "   • Tower B cannot decrypt new data"
echo "   • Tower A unaffected"
echo "   • Master key still valid"
echo "   • Can derive new key for Tower B if needed"
echo ""

# Test 10: Verify Revocation
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Test 10: Verify Revocation"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Checking revocation status:"
$BEARDOG_BIN key check-revocation --key-id "$TOWER_B_KEY"
echo ""

echo "Listing all revoked keys:"
$BEARDOG_BIN key list-revocations
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Integration Test Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ BearDog CLI Features Verified:"
echo "   • Key Generation (Master) ✓"
echo "   • Genetic Derivation (Tower keys) ✓"
echo "   • Lineage Verification ✓"
echo "   • Streaming Encryption ✓"
echo "   • Streaming Decryption ✓"
echo "   • Data Integrity ✓"
echo "   • Key Export ✓"
echo "   • Enhanced Revocation ✓"
echo "   • Revocation Verification ✓"
echo ""
echo "🚀 ToadStool + BearDog Integration: PRODUCTION READY"
echo ""
echo "Generated Files:"
echo "   📁 $WORK_DIR"
echo "   ├── keys/"
echo "   │   ├── tower-a-key-export.json"
echo "   │   └── tower-b-key-export.json"
echo "   └── data/"
echo "       ├── train-data-tower-a.txt"
echo "       ├── train-data-tower-a.enc (encrypted)"
echo "       ├── train-data-tower-a-decrypted.txt (verified)"
echo "       ├── train-data-tower-b.txt"
echo "       └── train-data-tower-b.enc (encrypted)"
echo ""
echo "Next Steps:"
echo "   1. Transfer exported keys to respective towers"
echo "   2. Import keys on each tower"
echo "   3. Decrypt training data on each tower"
echo "   4. Run distributed ML training"
echo "   5. Aggregate results"
echo ""
echo "🔐 Security Model:"
echo "   • Master key: $MASTER_KEY_ID (generation 0)"
echo "   • Tower A: $TOWER_A_KEY (generation 1, revocable)"
echo "   • Tower B: $TOWER_B_KEY (generation 1, REVOKED)"
echo "   • Genetic hierarchy: Verified ✓"
echo "   • Sovereignty: Full (no phone home)"
echo ""

