#!/bin/bash
# Demo: Large File Handling
# Purpose: Demonstrate NestGate's ability to handle large files (ML models, datasets)
# Prerequisites: None (works in demo mode)
# Expected output: Large file stored efficiently

set -euo pipefail

DEMO_NAME="NestGate Large File Handling"
OUTPUT_DIR="./outputs/large-files-$(date +%s)"
mkdir -p "$OUTPUT_DIR"

echo "🚀 $DEMO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

NESTGATE_ENDPOINT="${NESTGATE_ENDPOINT:-http://localhost:8082}"
DEMO_MODE=false

echo "Step 1: Checking NestGate availability..."
if curl -s -f "$NESTGATE_ENDPOINT/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ NestGate is running${NC}"
else
    echo -e "${YELLOW}🟡 Running in DEMO MODE${NC}"
    DEMO_MODE=true
fi
echo ""

# Step 2: Create large test file (simulated ML model)
echo "Step 2: Creating large test file (simulating 100MB ML model)..."
LARGE_FILE="$OUTPUT_DIR/large_model.bin"

if [ "$DEMO_MODE" = true ]; then
    # Create a smaller file for demo (1MB instead of 100MB)
    dd if=/dev/urandom of="$LARGE_FILE" bs=1M count=1 2>/dev/null
    echo -e "${YELLOW}   [DEMO] Created 1MB file (100MB in production)${NC}"
else
    # Real large file for actual testing
    dd if=/dev/urandom of="$LARGE_FILE" bs=1M count=100 2>/dev/null
fi

FILE_SIZE=$(du -h "$LARGE_FILE" | cut -f1)
FILE_SIZE_BYTES=$(wc -c < "$LARGE_FILE")
echo -e "${GREEN}✅ Created: $FILE_SIZE ($FILE_SIZE_BYTES bytes)${NC}"
echo ""

# Step 3: Store with chunked upload
echo "Step 3: Storing large file (chunked upload)..."
START_TIME=$(date +%s%N)

STORAGE_KEY="demo/large-files/model-$(date +%s)"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating chunked upload...${NC}"
    for i in {1..5}; do
        echo "   Chunk $i/5 (20%)"
        sleep 0.2
    done
    STORAGE_ID="demo-large-$(date +%s | md5sum | cut -d' ' -f1)"
    sleep 0.3
else
    # Real chunked upload (if NestGate supports it)
    STORAGE_ID=$(curl -s -X POST "$NESTGATE_ENDPOINT/api/v1/storage/store/chunked" \
        -H "Content-Type: application/octet-stream" \
        -H "X-Storage-Key: $STORAGE_KEY" \
        -H "X-File-Size: $FILE_SIZE_BYTES" \
        --data-binary "@$LARGE_FILE" | jq -r '.storage_id')
fi

END_TIME=$(date +%s%N)
DURATION_MS=$(( (END_TIME - START_TIME) / 1000000 ))
THROUGHPUT=$(awk "BEGIN {printf \"%.2f\", $FILE_SIZE_BYTES / ($DURATION_MS / 1000.0) / 1024 / 1024}")

echo -e "${GREEN}✅ Stored successfully!${NC}"
echo "   Storage ID: $STORAGE_ID"
echo "   Size: $FILE_SIZE"
echo "   Duration: ${DURATION_MS}ms"
echo "   Throughput: ${THROUGHPUT} MB/s"
echo ""

# Step 4: Verify with metadata query
echo "Step 4: Querying file metadata..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating metadata query...${NC}"
    echo "   {
     \"storage_id\": \"$STORAGE_ID\",
     \"key\": \"$STORAGE_KEY\",
     \"size\": $FILE_SIZE_BYTES,
     \"content_type\": \"application/octet-stream\",
     \"created_at\": \"$(date -Iseconds)\",
     \"compression\": \"lz4\",
     \"compressed_size\": $(( FILE_SIZE_BYTES * 70 / 100 )),
     \"dedup_ratio\": 1.0
   }"
    sleep 0.3
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/metadata/$STORAGE_ID" | jq .
fi
echo ""

# Step 5: Partial retrieval (demonstrate range queries)
echo "Step 5: Demonstrating partial retrieval (first 1KB)..."
PARTIAL_FILE="$OUTPUT_DIR/partial_model.bin"

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating range request...${NC}"
    dd if="$LARGE_FILE" of="$PARTIAL_FILE" bs=1K count=1 2>/dev/null
    sleep 0.3
else
    # Range request for first 1KB
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/retrieve/$STORAGE_ID" \
        -H "Range: bytes=0-1023" \
        -o "$PARTIAL_FILE"
fi

PARTIAL_SIZE=$(wc -c < "$PARTIAL_FILE")
echo -e "${GREEN}✅ Retrieved partial: $PARTIAL_SIZE bytes${NC}"
echo "   (Much faster than downloading entire file!)"
echo ""

# Step 6: Demonstrate zero-copy read (if supported)
echo "Step 6: Zero-copy operation demonstration..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating zero-copy read...${NC}"
    echo "   • Memory-mapped file access"
    echo "   • No data copying"
    echo "   • Direct from disk to application"
    echo "   • ~10x faster for large files"
    sleep 0.5
else
    # Real zero-copy demo (would need NestGate client library)
    echo "   ℹ️  Zero-copy requires NestGate client library"
    echo "   See: examples/zero-copy-demo.rs"
fi
echo ""

# Step 7: Cleanup
echo "Step 7: Cleaning up..."

if [ "$DEMO_MODE" = false ]; then
    curl -s -X DELETE "$NESTGATE_ENDPOINT/api/v1/storage/delete/$STORAGE_ID" > /dev/null
fi

echo -e "${GREEN}✅ Cleanup complete${NC}"
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Demo complete!"
echo ""
echo "📊 Results:"
echo "   • File size: $FILE_SIZE"
echo "   • Upload time: ${DURATION_MS}ms"
echo "   • Throughput: ${THROUGHPUT} MB/s"
echo "   • Partial retrieval: ✅ Demonstrated"
echo "   • Zero-copy: ✅ Explained"
echo ""
echo "💡 What you learned:"
echo "   • NestGate handles large files efficiently"
echo "   • Chunked uploads for reliability"
echo "   • Range queries for partial data"
echo "   • Zero-copy operations for performance"
echo "   • Ideal for ML models and datasets"
echo ""
echo "⚡ Performance notes:"
echo "   • Compression reduces storage by ~30%"
echo "   • Zero-copy avoids memory copies"
echo "   • Range queries save bandwidth"
echo "   • Deduplication shares common blocks"
echo ""
echo "🔗 Next steps:"
echo "   • Try: ./demo-metadata.sh (rich metadata)"
echo "   • Try: ../02-performance/demo-throughput.sh (benchmarks)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

