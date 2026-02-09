# Showcase Cleanup and Re-validation Plan
## February 8, 2026

## 🎯 Objective

Clean up older showcases and re-validate the rest using the new ToadStool + BarraCUDA architecture. Add NPU raytracing showcase to demonstrate NPU vs GPU comparison.

---

## 📂 Showcase Inventory

### ✅ Keep & Re-validate (BarraCUDA/Hardware)

**1. neuromorphic/** ✅ KEEP - Core NPU showcases
- `01-akida-detection/` - Hardware discovery
- `02-akida-bioinformatics/` - k-mer filtering
- `03-akida-llm-intent/` - Intent classification
- **Action**: Re-validate with new dual-backend drivers

**2. barracuda-validation/** ✅ KEEP - Core validation
- Performance benchmarks
- Cross-vendor validation
- **Action**: Re-run with ToadStool hardware discovery

**3. gpu-universal/** ✅ KEEP - GPU operations
- Universal GPU demonstrations
- WGSL shader showcase
- **Action**: Re-profile with ToadStool integration

**4. homomorphic-computing/** ✅ KEEP - FHE showcases
- FHE operations (21.1x speedup)
- Encrypted training
- **Action**: Re-validate with current architecture

**5. whitePaper/** ✅ KEEP - Benchmark data
- Performance results
- Validation reports
- **Action**: Re-run benchmarks with ToadStool

---

### 🗑️ Clean Up (Older/Outdated)

**1. gaming-evolution/** ⚠️ REVIEW
- Large showcase (26 scripts, 49 files)
- Not core to ToadStool/BarraCUDA mission
- **Action**: Archive or simplify significantly

**2. inter-primal/** ⚠️ REVIEW
- 123 files, multi-primal demos
- May be outdated with new architecture
- **Action**: Archive old versions, keep only working demos

**3. local-capabilities/** ⚠️ REVIEW
- Local execution showcases
- May overlap with ToadStool core now
- **Action**: Consolidate into ToadStool examples

**4. multi-primal-nestgate/** ⚠️ REVIEW
- 23 files (9 .bin files)
- NestGate integration
- **Action**: Archive if not actively used

**5. real-world/** ⚠️ REVIEW
- 75 files, 5 real-world scenarios
- May need updating for new architecture
- **Action**: Re-validate or archive

**6. secure-enclave/** ⚠️ REVIEW
- 6 markdown files
- Planning/gaps documents
- **Action**: Archive to docs/planning

**7. akida-characterization/** ✅ KEEP
- NPU characterization data
- **Action**: Update with new drivers

**8. biomes/** ⚠️ REVIEW
- 7 YAML files
- May be outdated
- **Action**: Review and potentially archive

**9. python-ml/** ⚠️ REVIEW
- Python ML showcase
- May overlap with BarraCUDA
- **Action**: Re-validate or archive

**10. src/** ⚠️ REVIEW
- `main.rs`, `distributed_compute_demo.rs`
- May be outdated
- **Action**: Review and update or remove

**11. workloads/** ⚠️ REVIEW
- 17 TOML files
- May be superseded by ToadStool
- **Action**: Review relevance

---

## 🆕 New Showcase: NPU Raytracing

### Goal: Demonstrate NPU vs GPU for Raytracing

**Showcase**: `neuromorphic/04-raytracing-comparison/`

**Demonstrates:**
- ✅ Raytracing on NPU (event-driven, sparse)
- ✅ Raytracing on GPU (parallel, dense)
- ✅ Performance comparison
- ✅ ToadStool automatic device selection
- ✅ BarraCUDA shader execution

**Structure:**
```
showcase/neuromorphic/04-raytracing-comparison/
├── Cargo.toml
├── README.md
├── demo.sh
├── src/
│   ├── lib.rs
│   ├── scene.rs           # Ray tracing scene setup
│   ├── npu_raytracer.rs   # NPU implementation
│   ├── gpu_raytracer.rs   # GPU implementation (WGSL)
│   └── benchmark.rs       # Performance comparison
└── shaders/
    └── raytrace.wgsl      # GPU raytracing shader
```

**Expected Results:**
- NPU: Better for sparse scenes (event-driven)
- GPU: Better for dense scenes (parallel)
- Demonstrate workload selection via ToadStool

---

## 🔧 Action Plan

### Phase 1: Archive Older Showcases
```bash
# Move old showcases to archive
mkdir -p showcase/archive/gaming
mv showcase/gaming-evolution showcase/archive/gaming/

mkdir -p showcase/archive/distributed
mv showcase/inter-primal showcase/archive/distributed/
mv showcase/multi-primal-nestgate showcase/archive/distributed/

mkdir -p showcase/archive/misc
mv showcase/local-capabilities showcase/archive/misc/
mv showcase/real-world showcase/archive/misc/
mv showcase/secure-enclave showcase/archive/misc/
mv showcase/biomes showcase/archive/misc/
mv showcase/python-ml showcase/archive/misc/
mv showcase/workloads showcase/archive/misc/
mv showcase/src showcase/archive/misc/
```

### Phase 2: Re-validate Core Showcases

**neuromorphic/**
```bash
cd showcase/neuromorphic/01-akida-detection
# Update to use new dual-backend
./demo.sh

cd ../02-akida-bioinformatics
./demo-kmer-filtering.sh

cd ../03-akida-llm-intent
cargo run --example train_intent_classifier
```

**barracuda-validation/**
```bash
cd showcase/barracuda-validation
# Re-run all benchmarks with ToadStool
cargo test --release
```

**gpu-universal/**
```bash
cd showcase/gpu-universal
# Re-profile all operations
cargo run --release --example matmul_demo
```

**homomorphic-computing/**
```bash
cd showcase/homomorphic-computing
# Re-validate FHE
cargo run --release --example fhe_ntt_validation
```

### Phase 3: Create NPU Raytracing Showcase

```bash
# Create new showcase
mkdir -p showcase/neuromorphic/04-raytracing-comparison
cd showcase/neuromorphic/04-raytracing-comparison

# Build and run
cargo build --release
./demo.sh

# Compare NPU vs GPU
cargo run --release --example benchmark_comparison
```

---

## 📊 Expected Outcomes

### After Cleanup
```
showcase/
├── neuromorphic/          ✅ CORE (4 showcases)
│   ├── 01-akida-detection
│   ├── 02-akida-bioinformatics
│   ├── 03-akida-llm-intent
│   └── 04-raytracing-comparison 🆕
├── barracuda-validation/  ✅ CORE
├── gpu-universal/         ✅ CORE
├── homomorphic-computing/ ✅ CORE
├── whitePaper/            ✅ KEEP (results)
├── akida-characterization/ ✅ KEEP (data)
└── archive/               📦 ARCHIVED
    ├── gaming/
    ├── distributed/
    └── misc/
```

### Re-validation Metrics

**Target:**
- All core showcases working with new architecture
- Performance benchmarks updated
- NPU raytracing comparison complete
- Documentation current

---

## 🎯 Priority Order

1. **Archive old showcases** (10 min)
2. **Re-validate neuromorphic** (20 min)
3. **Re-validate barracuda-validation** (15 min)
4. **Create NPU raytracing showcase** (30 min)
5. **Re-profile gpu-universal** (20 min)
6. **Update showcase README** (10 min)

**Total**: ~2 hours

---

## 📝 Notes

- Keep all result data (CSVs, JSONs)
- Archive rather than delete
- Update all READMEs to reflect ToadStool architecture
- Add raytracing as proof of NPU capabilities

**Status**: Ready to execute
