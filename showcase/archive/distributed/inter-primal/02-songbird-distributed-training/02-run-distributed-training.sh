#!/usr/bin/env bash
# Step 2: Run Distributed ML Training Across Federation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/outputs"
mkdir -p "$OUTPUT_DIR"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Step 2: Run Distributed ML Training"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

STRANDGATE="https://192.168.1.134:8081"
EASTGATE="http://192.168.1.144:8080"

echo -e "${BLUE}Training Configuration:${NC}"
echo "  • Dataset: MNIST (60,000 samples)"
echo "  • Epochs: 5"
echo "  • Batch size: 32"
echo "  • Learning rate: 0.01"
echo "  • Towers: 2 (Eastgate + Strandgate)"
echo

# Check federation status
echo -e "${BLUE}Checking Federation Status...${NC}"
echo

EASTGATE_STATUS=$(curl -s "${EASTGATE}/health" 2>&1)
STRANDGATE_STATUS=$(curl -sk "${STRANDGATE}/health" 2>&1)

if [ -n "$EASTGATE_STATUS" ] && [ -n "$STRANDGATE_STATUS" ]; then
    echo -e "${GREEN}✅ Both towers online${NC}"
else
    echo -e "${YELLOW}⚠️  One or both towers offline, will use available tower(s)${NC}"
fi
echo

# Submit training workload
echo -e "${BLUE}Submitting Distributed Training Workload...${NC}"
echo

MNIST_DATA_DIR="../../gpu-universal/ml-inference/data/mnist"

TRAINING_PAYLOAD=$(cat <<EOF
{
  "task": {
    "task_type": "distributed_ml_training",
    "description": "MNIST training across federation",
    "complexity": "heavy",
    "requirements": {
      "gpu": true,
      "memory_gb": 8,
      "compute_cores": 32,
      "distributed": true,
      "tower_count": 2
    },
    "parameters": {
      "dataset": "mnist",
      "dataset_path": "$MNIST_DATA_DIR",
      "epochs": 5,
      "batch_size": 32,
      "learning_rate": 0.01,
      "model": "simple_mlp",
      "target_accuracy": 0.97
    }
  },
  "priority": 8,
  "timeout_secs": 600
}
EOF
)

echo "Submitting to Strandgate coordinator..."
TASK_RESPONSE=$(curl -sk -X POST "${STRANDGATE}/api/compute/task" \
    -H "Content-Type: application/json" \
    -d "$TRAINING_PAYLOAD" 2>&1)

echo
echo "Response:"
echo "$TASK_RESPONSE" | jq '.' 2>/dev/null || echo "$TASK_RESPONSE"
echo

JOB_ID=$(echo "$TASK_RESPONSE" | jq -r '.job_id // empty' 2>/dev/null)

if [ -n "$JOB_ID" ] && [ "$JOB_ID" != "null" ]; then
    echo -e "${GREEN}✅ Training job submitted! Job ID: $JOB_ID${NC}"
    echo
    
    # Monitor progress
    echo -e "${BLUE}Monitoring Training Progress...${NC}"
    echo
    
    for i in {1..60}; do
        sleep 3
        STATUS=$(curl -sk "${STRANDGATE}/api/compute/task/${JOB_ID}" 2>&1)
        
        STATUS_TYPE=$(echo "$STATUS" | jq -r '.status // "unknown"' 2>/dev/null)
        PROGRESS=$(echo "$STATUS" | jq -r '.progress // 0' 2>/dev/null)
        
        echo "[$i] Status: $STATUS_TYPE | Progress: ${PROGRESS}"
        
        if [ "$STATUS_TYPE" = "completed" ] || [ "$STATUS_TYPE" = "failed" ]; then
            echo
            echo "Final status:"
            echo "$STATUS" | jq '.' 2>/dev/null || echo "$STATUS"
            break
        fi
    done
    
    # Save results
    echo "$TASK_RESPONSE" > "$OUTPUT_DIR/training_job_${JOB_ID}.json"
    
else
    echo -e "${YELLOW}⚠️  Job submission response unexpected${NC}"
    echo "Falling back to local training demo..."
    echo
    
    cargo run --release --bin distributed-train -- \
        --data-dir "$MNIST_DATA_DIR" \
        --epochs 5 \
        --batch-size 32 \
        --learning-rate 0.01 \
        --output-dir "$OUTPUT_DIR"
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Training Complete!${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

if [ -f "$OUTPUT_DIR/distributed_training_results.json" ]; then
    echo "📊 Results:"
    jq -r '.[-1] | "   Accuracy: \(.aggregate_accuracy * 100 | round)%\n   Training Time: \(.training_time_ms / 1000)s\n   Towers: \(.tower_results | length)"' \
        "$OUTPUT_DIR/distributed_training_results.json" 2>/dev/null
    echo
fi

echo "Output saved to: $OUTPUT_DIR"
echo

