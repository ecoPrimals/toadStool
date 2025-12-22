#!/bin/bash
# Demo: Rich Metadata Support
# Purpose: Demonstrate NestGate's metadata capabilities for organizing and discovering data
# Prerequisites: None (works in demo mode)
# Expected output: Files stored with rich metadata, queryable by tags

set -euo pipefail

DEMO_NAME="NestGate Rich Metadata"
OUTPUT_DIR="./outputs/metadata-$(date +%s)"
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

# Step 2: Store file with basic metadata
echo "Step 2: Storing file with basic metadata..."
TEST_FILE="$OUTPUT_DIR/mnist_model_v1.bin"
dd if=/dev/urandom of="$TEST_FILE" bs=1K count=500 2>/dev/null

METADATA_JSON=$(cat <<EOF
{
  "model_name": "mnist_classifier",
  "version": "1.0.0",
  "accuracy": 0.95,
  "training_epochs": 10,
  "created_by": "toadstool",
  "tags": ["production", "mnist", "cnn"],
  "framework": "rust-native",
  "input_shape": [28, 28, 1],
  "output_classes": 10
}
EOF
)

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating storage with metadata...${NC}"
    STORAGE_ID="demo-meta-$(date +%s | md5sum | cut -d' ' -f1)"
    sleep 0.5
else
    STORAGE_ID=$(curl -s -X POST "$NESTGATE_ENDPOINT/api/v1/storage/store" \
        -H "Content-Type: application/octet-stream" \
        -H "X-Metadata: $(echo "$METADATA_JSON" | base64 -w0)" \
        --data-binary "@$TEST_FILE" | jq -r '.storage_id')
fi

echo -e "${GREEN}✅ Stored with metadata!${NC}"
echo "   Storage ID: $STORAGE_ID"
echo "   Metadata fields: 9"
echo ""

# Step 3: Query by tags
echo "Step 3: Querying files by tag (tag: 'production')..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating tag query...${NC}"
    echo "   Found 3 files with tag 'production':"
    echo "   {
     \"results\": [
       {
         \"storage_id\": \"$STORAGE_ID\",
         \"model_name\": \"mnist_classifier\",
         \"version\": \"1.0.0\",
         \"accuracy\": 0.95,
         \"tags\": [\"production\", \"mnist\", \"cnn\"]
       },
       {
         \"storage_id\": \"abc123\",
         \"model_name\": \"resnet50\",
         \"version\": \"2.1.0\",
         \"accuracy\": 0.98,
         \"tags\": [\"production\", \"imagenet\", \"resnet\"]
       },
       {
         \"storage_id\": \"def456\",
         \"model_name\": \"bert_base\",
         \"version\": \"1.5.0\",
         \"accuracy\": 0.92,
         \"tags\": [\"production\", \"nlp\", \"transformer\"]
       }
     ],
     \"count\": 3
   }"
    sleep 0.5
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/query?tag=production" | jq .
fi
echo ""

# Step 4: Query by attribute
echo "Step 4: Querying by attribute (accuracy > 0.9)..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating attribute query...${NC}"
    echo "   Found 2 models with accuracy > 0.9:"
    echo "   • mnist_classifier (0.95)"
    echo "   • resnet50 (0.98)"
    sleep 0.3
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/query?filter=accuracy>0.9" | jq '.results[] | {model_name, accuracy}'
fi
echo ""

# Step 5: Update metadata (add new tag)
echo "Step 5: Updating metadata (adding 'verified' tag)..."

UPDATED_TAGS='["production", "mnist", "cnn", "verified"]'

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating metadata update...${NC}"
    sleep 0.3
else
    curl -s -X PATCH "$NESTGATE_ENDPOINT/api/v1/storage/metadata/$STORAGE_ID" \
        -H "Content-Type: application/json" \
        -d "{\"tags\": $UPDATED_TAGS}" > /dev/null
fi

echo -e "${GREEN}✅ Metadata updated!${NC}"
echo "   Added tag: 'verified'"
echo ""

# Step 6: Retrieve metadata without data
echo "Step 6: Retrieving metadata only (no data transfer)..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating metadata-only query...${NC}"
    echo "   {
     \"storage_id\": \"$STORAGE_ID\",
     \"model_name\": \"mnist_classifier\",
     \"version\": \"1.0.0\",
     \"accuracy\": 0.95,
     \"training_epochs\": 10,
     \"created_by\": \"toadstool\",
     \"tags\": [\"production\", \"mnist\", \"cnn\", \"verified\"],
     \"framework\": \"rust-native\",
     \"input_shape\": [28, 28, 1],
     \"output_classes\": 10,
     \"size\": 512000,
     \"created_at\": \"$(date -Iseconds)\",
     \"updated_at\": \"$(date -Iseconds)\"
   }"
    sleep 0.3
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/metadata/$STORAGE_ID" | jq .
fi
echo ""
echo "   ⚡ Fast! Only metadata transferred (no file download)"
echo ""

# Step 7: List all models by framework
echo "Step 7: Listing all rust-native models..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating framework query...${NC}"
    echo "   Found 2 rust-native models:"
    echo "   • mnist_classifier v1.0.0"
    echo "   • linear_regression v0.5.0"
    sleep 0.3
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/query?filter=framework=rust-native" | \
        jq -r '.results[] | "\(.model_name) v\(.version)"'
fi
echo ""

# Step 8: Demonstrate versioning with metadata
echo "Step 8: Demonstrating model versioning..."

echo "   Storing version 1.1.0 with improved accuracy..."
V2_FILE="$OUTPUT_DIR/mnist_model_v1.1.bin"
dd if=/dev/urandom of="$V2_FILE" bs=1K count=500 2>/dev/null

V2_METADATA_JSON=$(cat <<EOF
{
  "model_name": "mnist_classifier",
  "version": "1.1.0",
  "accuracy": 0.97,
  "training_epochs": 15,
  "created_by": "toadstool",
  "tags": ["production", "mnist", "cnn", "improved"],
  "framework": "rust-native",
  "input_shape": [28, 28, 1],
  "output_classes": 10,
  "changelog": "Increased training epochs, improved augmentation"
}
EOF
)

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Storing v1.1.0...${NC}"
    STORAGE_ID_V2="demo-meta-v2-$(date +%s | md5sum | cut -d' ' -f1)"
    sleep 0.5
else
    STORAGE_ID_V2=$(curl -s -X POST "$NESTGATE_ENDPOINT/api/v1/storage/store" \
        -H "Content-Type: application/octet-stream" \
        -H "X-Metadata: $(echo "$V2_METADATA_JSON" | base64 -w0)" \
        --data-binary "@$V2_FILE" | jq -r '.storage_id')
fi

echo -e "${GREEN}✅ Version 1.1.0 stored!${NC}"
echo ""

echo "   Now listing all versions of mnist_classifier:"
if [ "$DEMO_MODE" = true ]; then
    echo "   • v1.0.0: 95.0% accuracy (older)"
    echo "   • v1.1.0: 97.0% accuracy (current)"
    sleep 0.3
else
    curl -s "$NESTGATE_ENDPOINT/api/v1/storage/query?filter=model_name=mnist_classifier" | \
        jq -r '.results[] | "  • v\(.version): \(.accuracy * 100)% accuracy"'
fi
echo ""

# Cleanup
echo "Step 9: Cleaning up..."
if [ "$DEMO_MODE" = false ]; then
    curl -s -X DELETE "$NESTGATE_ENDPOINT/api/v1/storage/delete/$STORAGE_ID" > /dev/null
    curl -s -X DELETE "$NESTGATE_ENDPOINT/api/v1/storage/delete/$STORAGE_ID_V2" > /dev/null
fi
echo -e "${GREEN}✅ Cleanup complete${NC}"
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Demo complete!"
echo ""
echo "📊 Results:"
echo "   • Files stored: 2 (v1.0.0, v1.1.0)"
echo "   • Metadata fields: 9 per file"
echo "   • Queries demonstrated: 4 types"
echo "   • Updates: ✅ Dynamic tagging"
echo ""
echo "💡 What you learned:"
echo "   • Rich metadata enables discovery"
echo "   • Query by tags, attributes, versions"
echo "   • Update metadata without re-uploading data"
echo "   • Fast metadata-only queries"
echo "   • Version management with metadata"
echo ""
echo "🎯 Use cases:"
echo "   • Model registry: Track versions and performance"
echo "   • Dataset catalog: Organize training data"
echo "   • Experiment tracking: Tag and query experiments"
echo "   • Production management: Filter by environment tags"
echo ""
echo "🔗 Next steps:"
echo "   • Try: ../02-performance/demo-throughput.sh (performance)"
echo "   • Try: ../03-data-services/demo-deduplication.sh (ZFS features)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

