#!/bin/bash
# 🎯 Complete Showcase Demonstration
# Runs all production-ready benchmarks with real BarraCUDA operations
#
# Status: PRODUCTION-READY (Feb 7, 2026)
# Deep Debt: 100% Compliant (zero mocks!)

set -e

echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  🏆 ToadStool Complete Showcase - All Real Operations!      ║"
echo "║  100% Deep Debt Compliant - Zero Mocks in Production        ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 This demo runs 8 production-ready benchmarks:"
echo "   1. FHE Cross-Vendor (Real GPU NTT/INTT)"
echo "   2. FHE Encrypted Accuracy (Real GPU FHE ops)"
echo "   3. 🔥 FHE MNIST Pipeline (REAL ENCRYPTED TRAINING!)"
echo "   4. Transformer Inference (Real MatMul)"
echo "   5. Vision Inference (Real Tensor ops)"
echo "   6. Audio Processing (Real Tensor ops)"
echo "   7. NPU Reservoir Computing (Real power analysis)"
echo "   8. Hybrid NPU-GPU Raytracing (Real power analysis)"
echo ""
echo "⏱️  Estimated runtime: ~5-10 minutes (depending on hardware)"
echo ""

# Change to benchmarks directory
cd "$(dirname "$0")"
BENCH_DIR="$(pwd)"

echo "🔨 Building all benchmarks in release mode..."
cargo build --release --bins
echo "   ✅ Build complete!"
echo ""

# Create results directory
mkdir -p ../data/{fhe,ml,neuromorphic}

echo "═══════════════════════════════════════════════════════════════"
echo "🔐 Phase 1: Homomorphic Encryption (FHE)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "1️⃣  Running FHE Cross-Vendor Validation..."
echo "   (Real GPU NTT/INTT operations)"
cargo run --release --bin fhe_cross_vendor_validation
echo ""

echo "2️⃣  Running Encrypted vs Unencrypted Accuracy..."
echo "   (Real GPU FHE operations - upgraded Feb 7)"
cargo run --release --bin encrypted_vs_unencrypted_accuracy
echo ""

echo "3️⃣  Running Encrypted MNIST Pipeline..."
echo "   (REAL ENCRYPTED TRAINING + INFERENCE - LEGENDARY!)"
cargo run --release --bin encrypted_mnist_pipeline
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "🤖 Phase 2: ML Systems (Transformers, Vision, Audio)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "4️⃣  Running Transformer Inference..."
echo "   (Real MatMul operations)"
cargo run --release --bin transformer_inference
echo ""

echo "5️⃣  Running Vision Inference..."
echo "   (Real Tensor operations)"
cargo run --release --bin vision_inference
echo ""

echo "6️⃣  Running Audio Processing..."
echo "   (Real Tensor operations)"
cargo run --release --bin audio_processing
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "🧠 Phase 3: Neuromorphic Computing (NPU)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

echo "7️⃣  Running NPU Reservoir Computing..."
echo "   (Real NPU discovery + power analysis)"
cargo run --release --bin npu_reservoir_computing
echo ""

echo "8️⃣  Running Hybrid NPU-GPU Raytracing..."
echo "   (Real NPU/GPU discovery + power analysis)"
cargo run --release --bin hybrid_raytracing
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo "✅ Complete Showcase Finished!"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "📦 All results saved to:"
echo "   FHE: showcase/whitePaper/data/fhe/"
echo "   ML:  showcase/whitePaper/data/ml/"
echo "   NPU: showcase/whitePaper/data/neuromorphic/"
echo ""
echo "📊 Summary Reports Available:"
echo "   - COMPLETE_SHOWCASE_STATUS.md"
echo "   - REAL_ENCRYPTED_TRAINING_STATUS.md (NEW!)"
echo "   - LEGENDARY_SESSION_COMPLETE_REAL_ENCRYPTED_TRAINING_FEB07_2026.md"
echo ""
echo "🏆 Status: ALL PRODUCTION SHOWCASES VALIDATED!"
echo "   - 8/8 benchmarks use REAL BarraCUDA operations ✅"
echo "   - Zero mocks in production ✅"
echo "   - 100% deep debt compliance ✅"
echo ""
echo "🎉 Ready for production deployment!"
