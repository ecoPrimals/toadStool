#!/bin/bash
# Demo: Simple Storage Operations
# Purpose: Demonstrate basic NestGate storage: store, retrieve, list, delete
# Prerequisites: None (works in demo mode)
# Expected output: Files stored and retrieved successfully

set -euo pipefail

DEMO_NAME="NestGate Simple Storage"
OUTPUT_DIR="./outputs/simple-storage-$(date +%s)"
mkdir -p "$OUTPUT_DIR"

echo "🚀 $DEMO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if NestGate is available
NESTGATE_ENDPOINT="${NESTGATE_ENDPOINT:-http://localhost:8082}"
DEMO_MODE=false

echo "Step 1: Checking NestGate availability..."
if curl -s -f "$NESTGATE_ENDPOINT/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ NestGate is running at $NESTGATE_ENDPOINT${NC}"
else
    echo -e "${YELLOW}🟡 NestGate not detected - running in DEMO MODE${NC}"
    echo "   (Operations will be simulated)"
    DEMO_MODE=true
fi
echo ""

# Step 2: Create test data
echo "Step 2: Creating test data..."
TEST_FILE="$OUTPUT_DIR/test_data.txt"
echo "This is test data from ToadStool" > "$TEST_FILE"
echo "Timestamp: $(date)" >> "$TEST_FILE"
echo "Purpose: Demonstrate NestGate storage" >> "$TEST_FILE"
FILE_SIZE=$(wc -c < "$TEST_FILE")
echo -e "${GREEN}✅ Created test file: $FILE_SIZE bytes${NC}"
echo ""

# Step 3: Store file in NestGate
echo "Step 3: Storing file in NestGate..."
STORAGE_KEY="demo/simple-storage/test-$(date +%s)"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating storage operation...${NC}"
    STORAGE_ID="demo-id-$(date +%s | md5sum | cut -d' ' -f1)"
    sleep 0.5
else
    # Real NestGate API call
    RESPONSE=$(curl -s -X POST "$NESTGATE_ENDPOINT/api/v1/storage/store" \
        -H "Content-Type: application/octet-stream" \
        -H "X-Storage-Key: $STORAGE_KEY" \
        -H "X-Content-Type: text/plain" \
        --data-binary "@$TEST_FILE")
    STORAGE_ID=$(echo "$RESPONSE" | jq -r '.storage_id')
fi

echo -e "${GREEN}✅ Stored successfully!${NC}"
echo "   Storage Key: $STORAGE_KEY"
echo "   Storage ID: $STORAGE_ID"
echo ""

# Step 4: List stored files
echo "Step 4: Listing stored files..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating list operation...${NC}"
    echo "   Found 3 files:"
    echo "   - demo/simple-storage/test-1 (245 bytes)"
    echo "   - demo/simple-storage/test-2 (312 bytes)"
    echo "   - $STORAGE_KEY ($FILE_SIZE bytes)"
    sleep 0.5
else
    # Real NestGate API call
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/list?prefix=demo/simple-storage/" | jq .
fi
echo ""

# Step 5: Retrieve file
echo "Step 5: Retrieving file from NestGate..."
RETRIEVED_FILE="$OUTPUT_DIR/retrieved_data.txt"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating retrieval...${NC}"
    cp "$TEST_FILE" "$RETRIEVED_FILE"
    sleep 0.5
else
    # Real NestGate API call
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/retrieve/$STORAGE_ID" \
        -o "$RETRIEVED_FILE"
fi

RETRIEVED_SIZE=$(wc -c < "$RETRIEVED_FILE")
echo -e "${GREEN}✅ Retrieved successfully: $RETRIEVED_SIZE bytes${NC}"
echo ""

# Step 6: Verify integrity
echo "Step 6: Verifying data integrity..."
ORIGINAL_HASH=$(md5sum "$TEST_FILE" | cut -d' ' -f1)
RETRIEVED_HASH=$(md5sum "$RETRIEVED_FILE" | cut -d' ' -f1)

if [ "$ORIGINAL_HASH" = "$RETRIEVED_HASH" ]; then
    echo -e "${GREEN}✅ Integrity verified! Hashes match:${NC}"
    echo "   Original:  $ORIGINAL_HASH"
    echo "   Retrieved: $RETRIEVED_HASH"
else
    echo -e "❌ Integrity check failed!"
    exit 1
fi
echo ""

# Step 7: Delete file (optional cleanup)
echo "Step 7: Cleaning up..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating deletion...${NC}"
    sleep 0.3
else
    # Real NestGate API call
    curl -s -X DELETE "$NESTGATE_ENDPOINT/api/v1/storage/delete/$STORAGE_ID" > /dev/null
fi

echo -e "${GREEN}✅ Cleanup complete${NC}"
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Demo complete!"
echo ""
echo "📊 Results:"
echo "   • Stored: $FILE_SIZE bytes"
echo "   • Retrieved: $RETRIEVED_SIZE bytes"
echo "   • Integrity: ✅ Verified"
echo "   • Mode: $([ "$DEMO_MODE" = true ] && echo "Demo (simulated)" || echo "Live (real NestGate)")"
echo ""
echo "💡 What you learned:"
echo "   • Basic storage operations (store/retrieve/list/delete)"
echo "   • Data integrity verification with checksums"
echo "   • Graceful degradation (demo mode)"
echo "   • NestGate provides simple, reliable storage"
echo ""
echo "📂 Output saved to: $OUTPUT_DIR"
echo ""
echo "🔗 Next steps:"
echo "   • Try: ./demo-large-files.sh (large file handling)"
echo "   • Try: ./demo-metadata.sh (rich metadata support)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

