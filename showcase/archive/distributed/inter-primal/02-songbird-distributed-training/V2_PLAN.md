# V2: Real Cross-Tower Execution Plan

**Goal**: Execute distributed training across physical towers via Songbird  
**Status**: Planning → Implementation  
**ETA**: 1 hour

---

## 🎯 V2 Objectives

### Core Goals
1. ✅ Connect Eastgate's Songbird to Strandgate's federation
2. ✅ Submit ML training task via `/api/compute/task`
3. ✅ Execute on physical GPUs (RTX 2070 + RTX 3070)
4. ✅ Validate cross-tower communication
5. ✅ Measure real performance

### Success Criteria
- Both towers execute training concurrently
- Results aggregated from real GPUs
- Network latency measured
- Accuracy remains >90%
- Full logs from both towers

---

## 📊 Architecture

### Current (V1)
```
Developer → ToadStool Binary → Local Simulation
                                 ↓
                         (2 virtual towers)
                                 ↓
                    Results: 94.81% accuracy
```

### Target (V2)
```
Developer → ToadStool Coordinator → Songbird (Strandgate)
                                         ↓
                               Auto-Discover Towers
                                         ↓
                        ┌────────────────┴──────────────┐
                        ↓                                ↓
                  Tower A (Eastgate)            Tower B (Strandgate)
                  RTX 2070                      RTX 3070 + Dual EPYC
                        ↓                                ↓
                  Train 30k samples            Train 30k samples
                        ↓                                ↓
                        └────────────────┬──────────────┘
                                         ↓
                           Songbird Aggregates Results
                                         ↓
Developer ← Results (GPU-accelerated, cross-tower)
```

---

## 🔧 Implementation Steps

### Step 1: Connect Eastgate to Federation (15 min)

**Actions**:
1. Start Songbird on Eastgate
2. Connect to Strandgate's federation
3. Verify mesh formation

**Commands**:
```bash
# On Eastgate
cd ../songbird/showcase/02-federation/scripts

# Start and connect to Strandgate
SONGBIRD_PORT=8080 \
SONGBIRD_NODE_ID="tower-a-eastgate" \
SONGBIRD_PEERS="192.168.1.134:8081" \
./start-tower.sh
```

**Verification**:
```bash
# Check federation status
curl -sk https://192.168.1.134:8081/api/federation/status

# Should show 2 active nodes
```

---

### Step 2: Update Training Coordinator (20 min)

**Changes Needed**:
1. Implement real `/api/compute/task` submission
2. Add task status polling
3. Implement result collection
4. Add GPU requirement specification

**API Flow**:
```rust
// Submit training task to Songbird
let task = ComputeTaskRequest {
    task_type: "ml_training",
    requirements: vec![
        Requirement { name: "gpu", value: "nvidia" },
        Requirement { name: "ml-training", value: "mnist" },
    ],
    data: serde_json::to_value(&TrainingConfig {
        epochs: 10,
        batch_size: 64,
        learning_rate: 0.01,
        model_arch: "mlp-784-128-10",
    })?,
};

// POST /api/compute/task
let job_id = songbird.submit_task(task).await?;

// Poll status: GET /api/compute/task/{job_id}
let result = songbird.wait_for_completion(job_id).await?;
```

---

### Step 3: Deploy ToadStool to Towers (10 min)

**Method**: Use Songbird's deployment API

**Commands**:
```bash
# Deploy training worker to Strandgate
curl -k -X POST https://192.168.1.134:8081/api/deployment/binary \
  -F "binary=@target/release/distributed-train-worker" \
  -F "service_name=toadstool-ml-worker" \
  -F "start_after_upload=true"

# Eastgate runs worker locally (already built)
```

**Worker Responsibilities**:
- Listen for Songbird task assignments
- Load MNIST partition
- Execute training on local GPU
- Report results back to Songbird

---

### Step 4: Execute and Validate (15 min)

**Execution Flow**:
1. Coordinator submits task to Songbird
2. Songbird discovers 2 capable towers
3. Songbird partitions data (30k each)
4. Songbird routes tasks to both towers
5. Towers execute on GPUs concurrently
6. Results stream back to Songbird
7. Songbird aggregates and returns

**Metrics to Collect**:
- Training time per tower
- Network latency (task submission → result)
- GPU utilization on each tower
- Accuracy from each partition
- Aggregate accuracy

---

## 📝 Files to Create/Modify

### New Files
```
showcase/inter-primal/02-songbird-distributed-training/
├── v2/
│   ├── 01-connect-federation.sh      # Start Songbird, join federation
│   ├── 02-deploy-workers.sh          # Deploy ML workers to towers
│   ├── 03-run-real-cross-tower.sh    # Execute V2 training
│   ├── coordinator.rs                 # Real Songbird task submission
│   ├── worker.rs                      # Tower-side training worker
│   └── README.md                      # V2 documentation
```

### Modified Files
```
src/
└── main.rs  # Add --v2 flag to enable real Songbird routing
```

---

## 🔍 Key Differences: V1 vs V2

| Aspect | V1 (Validated) | V2 (Target) |
|--------|----------------|-------------|
| **Orchestration** | Simulated locally | Real Songbird federation |
| **Execution** | Single process | Multi-tower GPUs |
| **Data** | In-memory partitions | Network-transferred chunks |
| **Results** | Immediate | Async via Songbird |
| **GPUs** | None (simulation) | RTX 2070 + RTX 3070 |
| **Network** | None | Real cross-tower |
| **Latency** | ~0ms | Measured |
| **Scalability** | 1 node | N nodes |

---

## 🎓 Learning Outcomes

### V2 Will Demonstrate
1. **Real Federation**: Actual multi-tower coordination
2. **GPU Utilization**: Training on physical hardware
3. **Network Performance**: Cross-tower communication costs
4. **Load Distribution**: Songbird's intelligent routing
5. **Production Pattern**: Scalable to N towers

---

## 🚧 Challenges & Solutions

### Challenge 1: Data Transfer
**Issue**: 60MB MNIST data needs to be on both towers  
**Solution**: Pre-sync data OR use Songbird's data transfer API

### Challenge 2: Binary Deployment
**Issue**: Worker binary needs to be on Strandgate  
**Solution**: Already solved! Use `/api/deployment/binary` (proven in previous session)

### Challenge 3: Result Aggregation
**Issue**: Results come from 2 separate processes  
**Solution**: Songbird handles this via `/api/compute/task/{id}` status endpoint

### Challenge 4: GPU Access
**Issue**: Workers need GPU access permissions  
**Solution**: Run workers with appropriate permissions, verify CUDA/ROCm available

---

## 📊 Expected Results

### Performance Expectations
- **Training Time**: 60-90 seconds (vs 75s in V1)
  - Actual GPU computation: faster
  - Network overhead: +15-30s
- **Accuracy**: 94-96% (same as V1)
- **Network Latency**: 10-50ms per request
- **GPU Utilization**: 70-90% on both GPUs

### Success Metrics
- ✅ Both GPUs utilized (nvidia-smi confirms)
- ✅ Cross-tower communication working
- ✅ Accuracy within 2% of V1
- ✅ Automatic failover if one tower fails
- ✅ Clean logs from both towers

---

## ⏭️ After V2

### V3 Enhancements (Future)
1. **Gradient Synchronization**: All-Reduce for weight updates
2. **Fault Tolerance**: Handle tower failures mid-training
3. **Dynamic Scaling**: Add/remove towers during training
4. **Heterogeneous GPUs**: Handle different GPU capabilities
5. **Model Parallelism**: Split model layers across towers

---

## 🎯 V2 Execution Plan

### Phase 1: Federation Setup (15 min)
- [x] Understand Songbird federation
- [ ] Start Eastgate Songbird
- [ ] Connect to Strandgate
- [ ] Verify mesh

### Phase 2: Implementation (30 min)
- [ ] Create coordinator with real task submission
- [ ] Create worker binary
- [ ] Test locally first
- [ ] Deploy to Strandgate

### Phase 3: Execution (15 min)
- [ ] Run V2 training
- [ ] Collect results
- [ ] Validate accuracy
- [ ] Document performance

### Phase 4: Validation (10 min)
- [ ] Check GPU utilization
- [ ] Measure network latency
- [ ] Compare to V1
- [ ] Document learnings

---

**Total Time**: ~70 minutes  
**Risk Level**: Low (V1 pattern proven)  
**Value**: High (real distributed ML!)

---

**Status**: Ready to execute! 🚀

