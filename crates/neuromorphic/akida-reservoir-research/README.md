# Akida Reservoir Computing Research 🔬🧠

**Status**: 🚧 Active Research (January 29, 2026)  
**Goal**: Implement echo state networks (reservoir computing) on Akida neuromorphic hardware  
**Hardware**: 2x BrainChip Akida AKD1000 (160 NPUs total)

---

## 🎯 Research Objectives

1. **State Extraction**: Verify we can access internal NPU layer activations
2. **Recurrent Support**: Test Akida's RNN/TENN capabilities for echo state dynamics
3. **Dual-Chip Ensemble**: Implement reservoir ensemble with different random seeds
4. **Readout Training**: Train output layer on CPU/GPU using collected states
5. **Performance Validation**: Measure end-to-end latency vs traditional approaches

---

## 🔬 Approach: Echo State Networks (ESN)

### Architecture

```
Input → [Reservoir (Echo State)] → Readout Layer → Output
        ↑ RANDOM, FIXED              ↑ TRAINABLE
        (Akida NPUs)                 (CPU/GPU)
```

### Key Innovations

1. **Dual-Chip Ensemble**
   - Chip 1: Reservoir with seed=42
   - Chip 2: Reservoir with seed=123
   - Concatenate states: 2000D representation
   - Richer dynamics, better generalization

2. **Swappable Reservoirs**
   - Pre-generate multiple reservoir configs (.fbz files)
   - Load different configs at runtime (~5-20ms)
   - Test various random initializations

3. **Hybrid Training**
   - Reservoir: Fixed weights (no training on Akida)
   - Readout: Simple linear regression (CPU/GPU)
   - Fast training (no backpropagation!)

---

## 📊 Expected Performance

```
Load time (one-time):      5-20ms per chip
Reservoir inference:       70-96µs per chip (parallel!)
State extraction:          ~10-50µs (estimated)
Readout (CPU):             ~500µs
─────────────────────────────────────────────
Total inference:           ~600µs (0.6ms)

vs Traditional GPU:        1-10ms
Expected Speedup:          1.6-16x faster! ⚡
```

---

## 🔍 Critical Research Questions

### 1. State Extraction API ⚠️

**Question**: Can we access internal NPU layer activations?

**From BrainChip Documentation**:
- ✅ Akida SDK has `get_layer()` method
- ✅ Can access layer-wise activations
- ❓ Need to verify this works in our pure Rust driver

**Test**: `test-state-extraction` binary

### 2. Recurrent Architecture Support ⚠️

**Question**: Does Akida support RNN/temporal dynamics for echo state networks?

**From BrainChip Documentation**:
- ✅ Akida supports RNNs
- ✅ Has Temporal Event-based Neural Nets (TENNs)
- ❓ Need to verify echo state property preservation

**Test**: `generate-reservoir` binary (create recurrent model)

### 3. Load Behavior ⚠️

**Question**: Does `load_to_device()` replace previous model or append?

**Test**: Load multiple models sequentially, observe behavior

---

## 🚀 Research Pipeline

### Phase 1: Verification (Current)

```bash
# Test 1: Can we extract internal states?
cargo run --bin test-state-extraction

# Test 2: Can we generate reservoir models?
cargo run --bin generate-reservoir -- --seed 42 --size 1000

# Test 3: Can we load to dual chips?
cargo run --bin dual-chip-ensemble
```

### Phase 2: Prototype (If Phase 1 succeeds)

1. Generate reservoir models with various seeds
2. Load to 2 Akida chips
3. Collect states from large dataset
4. Train readout layer (linear regression)
5. Validate accuracy

### Phase 3: Optimization (If Phase 2 succeeds)

1. Benchmark latency end-to-end
2. Optimize state extraction
3. Test various reservoir sizes
4. Compare to GPU baseline
5. Publish results

---

## 📚 Research Background

### Reservoir Computing

**Key Insight**: You don't need to train the entire network!

- **Reservoir**: Random, fixed weights (never trained)
- **Readout**: Simple linear layer (trained with linear regression)
- **Benefits**: Fast training, no backprop, good for hardware

### Echo State Networks

Requirements:
1. ✅ Random initialization (done offline)
2. ✅ Fixed weights (Akida is inference-only)
3. ✅ Reconfigurable (load different .fbz files)
4. ❓ State extraction (need to verify)
5. ❓ Temporal dynamics (need to verify)

### Why Akida?

- ✅ 160 NPUs (80 per chip) - massive parallelism
- ✅ Ultra-low latency (70-96µs)
- ✅ Fixed weights during inference (perfect for ESM!)
- ✅ Supports RNNs and temporal processing
- ✅ Reconfigurable (swap reservoir configs)

---

## 🔬 Experiments

### Experiment 1: State Extraction Test

**File**: `src/bin/test_state_extraction.rs`

**Goal**: Verify we can access internal NPU activations

**Method**:
1. Load simple CNN model to Akida
2. Run inference on test input
3. Attempt to extract layer activations
4. Compare to final output

**Success Criteria**: Can read internal layer states

### Experiment 2: Reservoir Generation

**File**: `src/bin/generate_reservoir.rs`

**Goal**: Create reservoir models with random weights

**Method**:
1. Generate random weight matrices (NumPy/Rust)
2. Ensure echo state property (spectral radius < 1)
3. Convert to Akida .fbz format
4. Validate model structure

**Success Criteria**: Can create valid .fbz reservoir models

### Experiment 3: Dual-Chip Ensemble

**File**: `src/bin/dual_chip_ensemble.rs`

**Goal**: Run two reservoirs in parallel with different seeds

**Method**:
1. Load reservoir_seed42.fbz to chip 0
2. Load reservoir_seed123.fbz to chip 1
3. Run same input through both
4. Extract and concatenate states
5. Measure latency

**Success Criteria**: Sub-1ms end-to-end inference

---

## 📈 Potential Impact

### If Successful

**Scientific**:
- 📄 Novel neuromorphic reservoir computing approach
- 🔬 First hardware ESN on Akida (to our knowledge)
- 🎓 Publishable research contribution

**Practical**:
- ⚡ Ultra-fast inference (<1ms)
- 🔋 Ultra-low power (1-2W total)
- 🎯 Perfect for edge AI applications
- 🚀 Scalable to multiple chips

**Deep Debt Alignment**:
- ✅ Pure Rust (no Python dependencies)
- ✅ Vendor Agnostic (works on any Akida chip)
- ✅ Capability-Based (runtime discovery)
- ✅ No Mocks (real hardware only)

---

## 🎓 References

**Reservoir Computing**:
- Jaeger, H. (2001). "The echo state approach to analysing and training recurrent neural networks"
- Lukoševičius, M., & Jaeger, H. (2009). "Reservoir computing approaches to recurrent neural network training"

**Neuromorphic Computing**:
- BrainChip Akida Documentation: https://doc.brainchipinc.com/
- Open Neuromorphic: https://open-neuromorphic.org/

**Our Implementation**:
- Pure Rust Akida driver: `../akida-driver/`
- Model parser: `../akida-models/`
- Validation benchmarks: `../cross-substrate-validation/`

---

## ⚠️ Current Status

**Date**: January 29, 2026  
**Phase**: 1 (Verification)  
**Hardware**: 2x Akida AKD1000 available  
**Blockers**: None (exploring capabilities)

**Next Steps**:
1. Implement state extraction test
2. Test recurrent model generation
3. Prototype dual-chip ensemble
4. Measure baselines

---

**This is CUTTING-EDGE research! Let's discover what Akida can do!** 🍄🧠🔬✨
