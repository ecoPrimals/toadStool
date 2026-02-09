# Inter-Primal Showcase - Quick Start

**Date**: December 18, 2025  
**Status**: ✅ **Ready to Run** (Demo 1 Complete)

---

## 🚀 Run Your First Inter-Primal Demo (5 minutes)

### Prerequisites

Your Songbird federation is already running between 2 towers! Perfect!

### Step 1: Verify Songbird

```bash
curl http://localhost:8000/health
```

If Songbird is on a different port/host:
```bash
export SONGBIRD_URL="http://YOUR_SONGBIRD_HOST:PORT"
```

### Step 2: Run Discovery

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool/showcase/inter-primal/02-songbird-distributed-training

./demo-discover-towers.sh
```

**Expected Output**:
```
🔍 Discovering towers via Songbird...
✅ Found 2 towers:
   1. tower-a @ 192.168.1.100:8081
   2. tower-b @ 192.168.1.101:8081
```

### Step 3: Run Distributed Training

```bash
./demo-distributed-training.sh
```

**Expected Output**:
```
🚀 Distributed training across 2 towers...
Epoch 1/5: Aggregate accuracy 89.4%
...
Epoch 5/5: Aggregate accuracy 97.7%
✅ Training complete! Final accuracy: 97.7%
```

---

## 📊 What You Just Did

✅ **Discovered** ToadStool instances via Songbird  
✅ **Distributed** MNIST training across 2 towers  
✅ **Aggregated** results from multiple GPUs  
✅ **Achieved** 97% accuracy with distributed training  

**This is ToadStool's FIRST inter-primal integration demo!**

---

## 🎯 What's Available

### ✅ Complete: Songbird Distributed Training
- Location: `02-songbird-distributed-training/`
- Time: 5-10 minutes
- Hardware: Your existing 2-tower federation
- Impact: **Proves distributed ML vision**

### 📅 Planned: NestGate ML Pipeline
- Location: `03-nestgate-ml-pipeline/`
- Purpose: Checkpoint storage + model versioning
- Status: Not started (next priority)

### 📅 Planned: Full Ecosystem
- Location: `05-full-ecosystem-ml/`
- Purpose: All 5 primals coordinated
- Status: Not started (after NestGate)

---

## 📚 Full Documentation

- **Gap Analysis**: `PRIMAL_INTEGRATION_GAP_ANALYSIS.md`
- **Integration Plan**: `INTER_PRIMAL_PLAN.md`
- **Progress Report**: `INTER_PRIMAL_PROGRESS.md`
- **Session Summary**: `../SESSION_SUMMARY_DEC_18_INTER_PRIMAL.md`
- **Demo README**: `02-songbird-distributed-training/README.md`

---

## 🔧 Troubleshooting

### Issue: "Songbird not reachable"

**Solution**: Start Songbird federation first
```bash
cd ../../songbird/showcase/02-federation
./QUICK_START.sh
```

### Issue: "No towers discovered"

**Solution**: Demo will auto-start local towers for testing
```bash
# Discovery script automatically starts 2 local ToadStool instances
# Just run: ./demo-discover-towers.sh
```

### Issue: "MNIST data not found"

**Solution**: Data is reused from ml-inference showcase
```bash
cd ../../gpu-universal/ml-inference
cargo run --bin download-mnist
```

---

## 🎉 Success Criteria

When the demo completes successfully, you should see:

✅ **Discovery**: Found 2+ towers via Songbird  
✅ **Training**: Distributed across towers  
✅ **Accuracy**: >90% (ideally ~97%)  
✅ **Speedup**: ~2x faster than single tower  
✅ **Results**: JSON file in `outputs/`  

---

**Status**: ✅ **READY**  
**Time**: 5-10 minutes  
**Value**: 🔥🔥🔥🔥🔥 **CRITICAL MILESTONE**

🚀 **Let's evolve the ecosystem!** 🦀

