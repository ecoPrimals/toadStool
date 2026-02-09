# V2: Real Cross-Tower Execution

**Status**: In Progress  
**Blocked By**: Songbird TLS configuration (requires `aws-lc-rs` or `ring` feature)

---

## 🎯 Goal

Execute distributed ML training across physical towers via Songbird federation.

---

## 🚧 Current Blocker

Songbird fails to start with TLS error:
```
Could not automatically determine the process-level CryptoProvider from Rustls crate features.
Call CryptoProvider::install_default() before this point to select a provider manually,
or make sure exactly one of the 'aws-lc-rs' and 'ring' features is enabled.
```

**Resolution**: Songbird team needs to add `aws-lc-rs` or `ring` to Cargo.toml features.

---

## 📊 V2 Architecture (Planned)

```
ToadStool Coordinator → Songbird (Strandgate)
                             ↓
                   Auto-Discover Towers
                             ↓
              ┌──────────────┴──────────────┐
              ↓                              ↓
        Tower A (Eastgate)            Tower B (Strandgate)
        RTX 2070                      RTX 3070
              ↓                              ↓
        Train 30k samples            Train 30k samples
              ↓                              ↓
              └──────────────┬──────────────┘
                             ↓
                Songbird Aggregates Results
```

---

## 🔧 V2 Implementation Plan

### Phase 1: Songbird Federation ❌ Blocked
- Start local Songbird
- Connect to Strandgate federation
- Verify mesh formation

### Phase 2: Task Submission API (Ready to implement)
Create coordinator that uses Songbird's `/api/compute/task`:

```rust
// Submit to Songbird
let task = ComputeTaskRequest {
    task: Task::builder("ml_training")
        .with_gpu()
        .with_cpu(8.0)
        .with_memory(16384)
        .build(),
    priority: Some(8),
    timeout_secs: Some(600),
};

let response = client
    .post("https://192.168.1.134:8081/api/compute/task")
    .json(&task)
    .send()
    .await?;

let job_id = response.json::<ComputeTaskResponse>().await?.job_id;
```

### Phase 3: Worker Deployment (Ready to implement)
- Deploy ML worker binary to towers
- Workers listen for Songbird task assignments
- Execute on local GPU
- Report results back

### Phase 4: Execution & Validation
- Submit training task
- Monitor via `/api/compute/task/{job_id}`
- Collect results
- Validate accuracy

---

## 🎓 What V1 Proved

✅ **Pattern Validated**: 94.81% accuracy with local simulation  
✅ **Data Partitioning**: Works correctly  
✅ **Result Aggregation**: Math is sound  
✅ **Architecture**: Design is proven  

**V2 adds**: Real network, real GPUs, real Songbird routing

---

## ⏭️ Alternative: V2-Lite (Without Full Federation)

While Songbird federation is being fixed, we can demonstrate V2 pattern with direct API calls:

1. **Direct Task Submission**: Skip federation, submit directly to Strandgate
2. **Manual Coordination**: Coordinator manages both towers directly
3. **Result Collection**: Direct API polling

This proves the pattern works, even if Songbird routing is manual.

---

## 📝 Files Created

```
v2/
├── README.md (this file)
├── V2_PLAN.md (detailed plan)
├── 01-start-local-songbird.sh (blocked by TLS)
├── 01-start-local-only.sh (blocked by TLS)
└── (more files when unblocked)
```

---

## 🔍 Next Steps

### Option A: Wait for Songbird Fix
- Songbird team adds TLS crypto provider
- Retry federation setup
- Full V2 implementation

### Option B: V2-Lite Now
- Skip federation
- Direct API calls to both towers
- Prove cross-tower works
- Upgrade to full federation later

**Recommendation**: Option B - prove the pattern works now, upgrade later.

---

## 📊 Expected V2 Results

| Metric | V1 (Proven) | V2 (Target) |
|--------|-------------|-------------|
| Accuracy | 94.81% | 94-96% |
| Time | 75s | 60-90s |
| GPUs | 0 (sim) | 2 (real) |
| Network | None | Measured |
| Towers | 1 (sim 2) | 2 (real) |

---

**Status**: Paused pending Songbird TLS fix  
**Alternative**: V2-Lite ready to implement


