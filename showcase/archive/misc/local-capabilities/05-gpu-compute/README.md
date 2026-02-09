# 🍄 Level 4: GPU Compute → See `gpu-universal/`

**Time**: 30 minutes  
**Prerequisites**: NVIDIA or AMD GPU  
**Location**: `showcase/gpu-universal/`

---

## 🎯 What This Is

**GPU compute demos are already built!** See the `gpu-universal/` directory:

```bash
cd ../../gpu-universal
cat README.md
```

---

## 📋 Available GPU Demos

All demos are in `showcase/gpu-universal/`:

### Local GPU Workloads
```bash
cd ../../gpu-universal/local
./01-simple-cuda-workload.sh
./02-benchmark-gpu.sh
```

### ML Inference
```bash
cd ../../gpu-universal/ml-inference
# Multiple BERT, ResNet, ViT demos available
```

### Distributed GPU Coordination
```bash
cd ../../gpu-universal/distributed
# Multi-GPU workload distribution
```

---

## 🚀 Quick Start

```bash
# Check if you have a GPU
nvidia-smi  # NVIDIA
rocm-smi    # AMD

# Go to GPU demos
cd ../../gpu-universal

# Run quick start
./QUICK_START.md
```

---

## 🎓 What You'll Learn

✅ **GPU workload execution** - CUDA/ROCm  
✅ **ML inference** - BERT, ResNet, ViT  
✅ **Multi-GPU coordination** - Distributed compute  
✅ **Performance benchmarking** - GPU utilization

---

## ➡️ Next Level

### Level 5: Production Patterns
```bash
cd ../06-production-patterns
cat README.md
```

---

**🍄 GPU Power with ToadStool!**

