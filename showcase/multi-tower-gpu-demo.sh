#!/usr/bin/env bash
# Multi-Tower GPU Compute Federation Demo
# Demonstrates distributing GPU workloads across 2 LAN towers

set -e

echo "🏢 ToadStool Multi-Tower GPU Federation Demo"
echo "=============================================="
echo ""

# Configuration
TOWER_A_IP="${TOWER_A_IP:-192.168.1.100}"
TOWER_B_IP="${TOWER_B_IP:-192.168.1.101}"
TOWER_A_PORT="${TOWER_A_PORT:-8081}"
TOWER_B_PORT="${TOWER_B_PORT:-8082}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Detect local compute resources
echo "📊 Step 1: Detecting Local Compute Resources"
echo "=============================================="
echo ""

detect_local_gpu() {
    echo -e "${BLUE}🔍 Local GPU Detection:${NC}"
    
    # Check for NVIDIA
    if command -v nvidia-smi &> /dev/null; then
        echo -e "${GREEN}✅ NVIDIA GPU:${NC}"
        nvidia-smi --query-gpu=name,memory.total,compute_cap --format=csv,noheader | while read -r line; do
            echo "   $line"
        done
    fi
    
    # Check for AMD
    if lspci | grep -i "vga.*amd" &> /dev/null; then
        echo -e "${GREEN}✅ AMD GPU:${NC}"
        lspci | grep -i "vga.*amd" | sed 's/^/   /'
    fi
    
    # CPU Info
    echo -e "${GREEN}✅ CPU:${NC}"
    echo "   Cores: $(nproc)"
    echo "   Model: $(lscpu | grep 'Model name' | cut -d: -f2 | xargs)"
    
    echo ""
}

detect_local_gpu

# Step 2: Test local federation endpoints
echo "📡 Step 2: Testing Federation Endpoints"
echo "=============================================="
echo ""

test_tower_connection() {
    local tower_name=$1
    local tower_ip=$2
    local tower_port=$3
    
    echo -e "${BLUE}Testing $tower_name ($tower_ip:$tower_port)...${NC}"
    
    # Test basic connectivity
    if timeout 2 bash -c ">/dev/tcp/$tower_ip/$tower_port" 2>/dev/null; then
        echo -e "${GREEN}✅ $tower_name is reachable${NC}"
        
        # Try to get capabilities (if ToadStool is running)
        if curl -s --max-time 2 "http://$tower_ip:$tower_port/health" &>/dev/null; then
            echo -e "${GREEN}✅ $tower_name ToadStool is running${NC}"
            
            # Get capabilities if API exists
            capabilities=$(curl -s --max-time 2 "http://$tower_ip:$tower_port/capabilities" 2>/dev/null || echo "{}")
            if [ "$capabilities" != "{}" ]; then
                echo "   Capabilities: $capabilities"
            fi
        else
            echo -e "${YELLOW}⚠️  $tower_name ToadStool not running (start with: cargo run --release)${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  $tower_name not reachable (check network/firewall)${NC}"
        echo "   Tip: Update IP in this script or set environment variables:"
        echo "   export TOWER_A_IP=<your-tower-a-ip>"
        echo "   export TOWER_B_IP=<your-tower-b-ip>"
    fi
    echo ""
}

test_tower_connection "Tower A" "$TOWER_A_IP" "$TOWER_A_PORT"
test_tower_connection "Tower B" "$TOWER_B_IP" "$TOWER_B_PORT"

# Step 3: Create federation configuration
echo "⚙️  Step 3: Creating Federation Configuration"
echo "=============================================="
echo ""

cat > /tmp/toadstool-federation-demo.toml << EOF
# ToadStool Multi-Tower Federation Configuration

[federation]
enabled = true
local_tower_id = "tower-local"

# Tower registry
[[federation.towers]]
id = "tower-a"
name = "Tower A (Main Workstation)"
endpoint = "http://${TOWER_A_IP}:${TOWER_A_PORT}"
capabilities = ["gpu", "cuda", "compute"]
priority = 100

[[federation.towers]]
id = "tower-b"
name = "Tower B (Secondary Server)"
endpoint = "http://${TOWER_B_IP}:${TOWER_B_PORT}"
capabilities = ["gpu", "opencl", "compute"]
priority = 90

# Distribution strategy
[federation.distribution]
strategy = "capability_aware"  # Route based on capabilities
load_balance = true
failover_enabled = true

# Work splitting
[federation.work_splitting]
enabled = true
min_work_units = 2
max_work_units = 16
split_threshold = 100  # Split jobs larger than this

# Network settings
[federation.network]
discovery_method = "manual"  # manual, mdns, songbird
health_check_interval_secs = 30
timeout_secs = 60

# Job coordination
[federation.coordination]
result_aggregation = "automatic"
progress_tracking = true
fault_tolerance = true
EOF

echo -e "${GREEN}✅ Configuration created: /tmp/toadstool-federation-demo.toml${NC}"
echo ""

# Step 4: Demonstrate workload distribution
echo "🎮 Step 4: GPU Workload Distribution Example"
echo "=============================================="
echo ""

cat << 'EOF'
Example workload distribution across towers:

Scenario: Matrix Multiplication (Large Dataset)
- Input: 10,000 x 10,000 matrices
- Operation: A × B = C
- Estimated time: 120 seconds on single GPU

Federation Strategy:
┌─────────────────────────────────────────────┐
│ Tower A (Local)                             │
│ ├─ GPU: RTX 2070 SUPER (8GB)               │
│ ├─ Assigned: 60% of workload               │
│ ├─ Matrices: 6,000 rows                    │
│ └─ Est. time: 72 seconds                   │
└─────────────────────────────────────────────┘
             │
             ├─ Network (LAN) ─→
             ↓
┌─────────────────────────────────────────────┐
│ Tower B (Remote)                            │
│ ├─ GPU: [Your Tower B GPU]                 │
│ ├─ Assigned: 40% of workload               │
│ ├─ Matrices: 4,000 rows                    │
│ └─ Est. time: 48 seconds                   │
└─────────────────────────────────────────────┘
             │
             └─ Results aggregate ─→ Total time: ~75s

Performance Gain:
- Single tower: 120 seconds
- Federated: 75 seconds
- Speedup: 1.6x
- Network overhead: ~3 seconds

EOF

echo ""

# Step 5: Create test workload manifest
echo "📝 Step 5: Creating Test Workload Manifest"
echo "=============================================="
echo ""

cat > /tmp/gpu-federation-test.toml << 'EOF'
[workload]
name = "gpu-federation-test"
description = "Test GPU compute across federated towers"
type = "gpu_compute"

[workload.requirements]
min_parallel_threads = 1024
memory_mb = 1024
precision = "fp32"
operations = ["matrix_multiply", "vector_ops"]

[workload.distribution]
strategy = "auto"  # Let federation decide
allow_splitting = true
prefer_local = false  # Allow remote execution

[workload.kernel]
type = "opencl"
entry_point = "matrix_multiply"
source = """
__kernel void matrix_multiply(
    __global const float* A,
    __global const float* B,
    __global float* C,
    const int M, const int N, const int K
) {
    int row = get_global_id(0);
    int col = get_global_id(1);
    
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int i = 0; i < K; i++) {
            sum += A[row * K + i] * B[i * N + col];
        }
        C[row * N + col] = sum;
    }
}
"""

[workload.input]
matrix_size = 1024
data_type = "float32"
generate = "random"

[workload.output]
format = "binary"
destination = "/tmp/gpu-federation-result.bin"
EOF

echo -e "${GREEN}✅ Test workload created: /tmp/gpu-federation-test.toml${NC}"
echo ""

# Step 6: Show federation monitoring
echo "📊 Step 6: Federation Monitoring Dashboard"
echo "=============================================="
echo ""

cat << 'EOF'
Real-time Federation Dashboard:

╔══════════════════════════════════════════════════════════╗
║  🏢 ToadStool Multi-Tower Federation                    ║
╠══════════════════════════════════════════════════════════╣
║                                                           ║
║  Active Towers: 2                                        ║
║  Total GPUs: 2                                           ║
║  Total CPU Cores: 32 (16 per tower)                     ║
║  Network Status: ✅ Healthy (2.3ms latency)             ║
║                                                           ║
╠══════════════════════════════════════════════════════════╣
║  TOWER STATUS                                            ║
╠══════════════════════════════════════════════════════════╣
║                                                           ║
║  📍 Tower A (Local) - 192.168.1.100                     ║
║     └─ GPU: RTX 2070 SUPER (8GB)                        ║
║        ├─ Utilization: 75% ████████████████░░░░░░       ║
║        ├─ Memory: 6.2GB / 8GB (78%)                     ║
║        └─ Current Job: matrix_multiply_part_1           ║
║     └─ CPU: 16 cores @ 3.8GHz                           ║
║        └─ Utilization: 45%                               ║
║                                                           ║
║  📍 Tower B (Remote) - 192.168.1.101                    ║
║     └─ GPU: [Your GPU]                                   ║
║        ├─ Utilization: 68% ██████████████░░░░░░░        ║
║        ├─ Memory: [varies]                               ║
║        └─ Current Job: matrix_multiply_part_2           ║
║     └─ CPU: 16 cores @ [speed]                          ║
║        └─ Utilization: 38%                               ║
║                                                           ║
╠══════════════════════════════════════════════════════════╣
║  ACTIVE JOBS                                             ║
╠══════════════════════════════════════════════════════════╣
║                                                           ║
║  🎮 Job: gpu-federation-test                            ║
║     ├─ Type: Matrix Multiplication                       ║
║     ├─ Distribution: 60% Tower A, 40% Tower B           ║
║     ├─ Progress: ████████████████████░░░░░░ 75%         ║
║     ├─ Time Elapsed: 56s / ~75s estimated               ║
║     └─ Network Traffic: 145 MB transferred              ║
║                                                           ║
╠══════════════════════════════════════════════════════════╣
║  STATISTICS (Last Hour)                                  ║
╠══════════════════════════════════════════════════════════╣
║                                                           ║
║  Jobs Completed: 12                                      ║
║  Average Speedup: 1.54x (vs single tower)               ║
║  Network Efficiency: 96.2%                               ║
║  Failure Rate: 0%                                        ║
║  Total Compute Time Saved: 8.3 minutes                  ║
║                                                           ║
╚══════════════════════════════════════════════════════════╝
EOF

echo ""

# Step 7: Next steps
echo "🚀 Step 7: Next Steps"
echo "=============================================="
echo ""

cat << EOF
To actually run the federation demo:

1. Start ToadStool on Tower A:
   ${BLUE}ssh user@$TOWER_A_IP${NC}
   cd /path/to/toadstool
   cargo run --release --features runtime-gpu-opencl

2. Start ToadStool on Tower B:
   ${BLUE}ssh user@$TOWER_B_IP${NC}
   cd /path/to/toadstool
   cargo run --release --features runtime-gpu-opencl

3. Submit federated workload (from either tower):
   ${BLUE}cargo run --release -- execute /tmp/gpu-federation-test.toml \
     --config /tmp/toadstool-federation-demo.toml${NC}

4. Monitor federation:
   ${BLUE}cargo run --release -- federation status${NC}

5. Real distributed GPU test:
   ${BLUE}cd showcase/real-world/01-gpu-classroom
   ./demo.sh --federation --towers 2${NC}

Configuration created at:
- /tmp/toadstool-federation-demo.toml
- /tmp/gpu-federation-test.toml

Current setup:
- Tower A: $TOWER_A_IP:$TOWER_A_PORT
- Tower B: $TOWER_B_IP:$TOWER_B_PORT

Update IPs if needed:
export TOWER_A_IP=<your-ip>
export TOWER_B_IP=<your-ip>
EOF

echo ""
echo "=============================================="
echo "🎉 Federation Demo Setup Complete!"
echo ""
echo "Your multi-tower GPU federation is ready."
echo "Configure tower IPs above and start ToadStool on each tower."
echo "🍄 ToadStool - Distributed Universal Compute"

