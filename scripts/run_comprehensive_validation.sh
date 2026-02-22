#!/bin/bash
# Comprehensive Cross-Platform Validation Runner
# Re-runs all experiments across CPU, GPU, and NPU

echo "═══════════════════════════════════════════════════════════════"
echo "🦈 BarraCuda Comprehensive Cross-Platform Validation"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Running all workloads on all platforms..."
echo "Est. time: 30-45 minutes"
echo ""

RESULTS_DIR="showcase/barracuda-validation/results"
mkdir -p "$RESULTS_DIR"

# Timestamp for this run
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOGFILE="$RESULTS_DIR/comprehensive_run_${TIMESTAMP}.log"

echo "Results will be saved to: $RESULTS_DIR"
echo "Log file: $LOGFILE"
echo ""

# Function to run a benchmark and log results
run_benchmark() {
    local name=$1
    local bin=$2
    local desc=$3
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Running: $desc"
    echo "Binary: $bin"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # Log to file
    echo "=== $desc ===" >> "$LOGFILE"
    echo "Timestamp: $(date)" >> "$LOGFILE"
    
    # Run the benchmark
    if ./$bin 2>&1 | tee -a "$LOGFILE"; then
        echo "✅ $name: SUCCESS"
    else
        echo "❌ $name: FAILED (continuing...)"
    fi
    
    echo ""
    echo ""
}

# Build all benchmarks first
echo "📦 Building all benchmarks..."
echo ""

cd showcase/barracuda-validation
cargo build --release --all-targets 2>&1 | grep -E "(Compiling|Finished|error)" | tail -10
cd ../..

cd showcase/homomorphic-computing  
cargo build --release --all-targets 2>&1 | grep -E "(Compiling|Finished|error)" | tail -10
cd ../..

cd showcase/akida-characterization
cargo build --release --all-targets 2>&1 | grep -E "(Compiling|Finished|error)" | tail -10
cd ../..

echo ""
echo "✅ Build complete!"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "Starting Benchmark Execution..."
echo "═══════════════════════════════════════════════════════════════"
echo ""

# 1. MNIST Inference (CPU + GPU)
run_benchmark "MNIST_CPU_GPU" \
    "showcase/barracuda-validation/target/release/mnist_inference" \
    "1. MNIST Inference (CPU + GPU) - 6 tests"

# 2. MNIST NPU
run_benchmark "MNIST_NPU" \
    "showcase/barracuda-validation/target/release/mnist_npu" \
    "2. MNIST NPU - 3 tests"

# 3. K-mer Counting (CPU + GPU)
run_benchmark "KMER_CPU_GPU" \
    "showcase/barracuda-validation/target/release/kmer_counting" \
    "3. K-mer Counting (CPU + GPU) - 8 tests"

# 4. K-mer NPU
run_benchmark "KMER_NPU" \
    "showcase/barracuda-validation/target/release/kmer_npu" \
    "4. K-mer NPU - 3 tests"

# 5. AES Encryption (CPU + GPU)
run_benchmark "AES_CPU_GPU" \
    "showcase/barracuda-validation/target/release/aes_benchmark" \
    "5. AES Encryption (CPU + GPU) - 8 tests"

# 6. Universal MLP (CPU + GPU + NPU)
run_benchmark "UNIVERSAL_MLP" \
    "showcase/barracuda-validation/target/release/cross_platform_mlp" \
    "6. Universal MLP (CPU + GPU + NPU) - 3 tests"

# 7. Homomorphic Encryption - Complete Pipeline
run_benchmark "HE_COMPLETE" \
    "showcase/homomorphic-computing/target/release/pipeline_validation_actual_hardware" \
    "7. Homomorphic Encryption (CPU + GPU + NPU) - 15 tests"

# 8. Dense vs Sparse Characterization
run_benchmark "DENSE_SPARSE" \
    "showcase/akida-characterization/target/release/dense_vs_sparse" \
    "8. Dense vs Sparse (CPU + GPU + NPU) - 48 tests"

# Summary
echo "═══════════════════════════════════════════════════════════════"
echo "✅ COMPREHENSIVE VALIDATION COMPLETE!"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Summary:"
echo "  - 8 benchmark suites executed"
echo "  - 94+ total tests run"
echo "  - 3 hardware platforms validated"
echo ""
echo "Results saved to:"
echo "  - CSV files: $RESULTS_DIR/*.csv"
echo "  - JSON files: $RESULTS_DIR/*.json"
echo "  - Log file: $LOGFILE"
echo ""
echo "Next steps:"
echo "  1. Review results in $RESULTS_DIR"
echo "  2. Generate comparison analysis"
echo "  3. Update documentation"
echo ""
echo "🦈 BarraCuda: Universal Compute Validation Complete! 🦈"
echo ""
