# Tutorial: Persistent ML Pipeline with NestGate

**Purpose**: Learn how ToadStool trains ML models with persistent checkpoints and versioning via NestGate

**Level**: Intermediate  
**Time**: 10 minutes  
**Prerequisites**: Basic ML knowledge, understanding of checkpointing

---

## What You'll Learn

1. **Checkpoint Saving** - How to save training checkpoints to persistent storage
2. **Resume Training** - How to resume from a checkpoint if interrupted
3. **Model Versioning** - How to version and tag production models
4. **Capability Discovery** - How ToadStool discovers NestGate by capability
5. **Production ML** - Real-world ML pipeline patterns

---

## Architecture

### Data Flow

```
ToadStool Training
    ↓ (every 2 epochs)
Save Checkpoint
    ↓
NestGate Storage
    ↓ (ZFS snapshots)
Persistent Checkpoint
    ↓ (on completion)
Versioned Model
```

### Components

```
┌─────────────────────────────────┐
│  ToadStool Compute              │
│  • GPU training                 │
│  • Backpropagation              │
│  • Gradient descent             │
└────────────┬────────────────────┘
             │
             │ HTTP API
             │
┌────────────▼────────────────────┐
│  NestGate Storage               │
│  • Checkpoint API               │
│  • Model versioning API         │
│  • ZFS deduplication            │
│  • Snapshot management          │
└─────────────────────────────────┘
```

---

## Quick Start

### Step 1: Start NestGate (Optional)

```bash
# If you have NestGate installed:
cd ~/nestgate
cargo run --release

# Or use environment variable to point to existing NestGate:
export NESTGATE_ENDPOINT=http://nestgate-server:8084
```

### Step 2: Run Training Demo

```bash
cd showcase/inter-primal/03-nestgate-ml-pipeline
./demo-train-with-checkpoints.sh
```

**What You'll See**:
```
🚀 ToadStool + NestGate: ML Training with Checkpoints

Step 1: Discovering storage service...
✅ Storage service healthy

Step 2: Loading MNIST data...
✅ Loaded 1000 training samples

Step 3: Initializing model...
✅ No existing checkpoints - starting fresh

Step 4: Training with checkpoint saving...

Epoch 1/10:
  Loss: 0.4700, Accuracy: 86.0%
  Time: 1234ms

Epoch 2/10:
  Loss: 0.4400, Accuracy: 87.0%
  💾 Saving checkpoint...
     ✅ Checkpoint saved: mnist_checkpoint_epoch_2
     Hash: a3b2c1d4e5f6...

...

✅ Training Complete!
📊 Final Results:
   Epochs trained: 10
   Final accuracy: 95.0%
   Checkpoints saved: 5

💾 Saving final model to NestGate...
✅ Model version saved: mnist_v1_1734723456
   Tags: ["production", "showcase"]

🎉 ToadStool + NestGate integration demonstrated!
```

---

## Key Concepts

### 1. Checkpoint Saving

**Why?** Training can be interrupted (power loss, OOM, network issues)

**Pattern**:
```rust
// Every N epochs:
let checkpoint = Checkpoint {
    model_name: "mnist_model".to_string(),
    epoch: current_epoch,
    accuracy: current_accuracy,
    loss: current_loss,
    created_at: Utc::now(),
    data_hash: compute_hash(&model_weights),
};

nestgate.save_checkpoint(&checkpoint, &model_weights).await?;
```

### 2. Resume from Checkpoint

**Why?** Don't lose days of training progress

**Pattern**:
```rust
// On restart:
let checkpoints = nestgate.list_checkpoints("mnist_model").await?;
let latest = checkpoints.iter().max_by_key(|c| c.epoch).unwrap();

let (checkpoint, weights) = nestgate.load_checkpoint(&latest.checkpoint_id).await?;
network.load_weights(&weights);
let start_epoch = checkpoint.epoch + 1;

// Continue training from start_epoch
```

### 3. Model Versioning

**Why?** Production requires version control and rollback

**Pattern**:
```rust
// After training completes:
let version = ModelVersion {
    model_name: "mnist_model".to_string(),
    version: "1.0.0".to_string(),
    accuracy: 0.95,
    tags: vec!["production", "v1"],
    created_at: Utc::now(),
};

nestgate.save_model_version(&version, &final_weights).await?;
```

---

## Capability-Based Discovery

### Self-Knowledge Architecture

ToadStool knows: "I need persistent storage"  
ToadStool discovers: "Service X provides persistent storage"  
ToadStool uses: Service X (which happens to be NestGate)

**No "NestGate" hardcoded in ToadStool code!**

### Discovery Methods

1. **Environment Variable** (Production/K8s)
   ```bash
   export STORAGE_ENDPOINT=http://nestgate-service:8084
   ```

2. **Capabilities File** (Developer Config)
   ```toml
   [[services]]
   capabilities = ["persistent-storage", "model-versioning"]
   endpoint = "http://localhost:8084"
   ```

3. **mDNS** (Zero-Config LAN)
   ```
   Auto-discovers NestGate on local network
   ```

---

## Production Benefits

### 1. Fault Tolerance
- Training interruption? Resume from last checkpoint
- No wasted GPU hours
- No lost progress

### 2. Versioning
- Track model evolution
- Roll back if new version underperforms
- A/B test different versions

### 3. Deduplication
- NestGate uses ZFS deduplication
- Checkpoints share common data blocks
- Save storage space

### 4. Snapshots
- Point-in-time recovery
- Compliance requirements
- Audit trail

---

## Real-World Scenarios

### Scenario 1: Multi-Day Training

```
Day 1: Train 100 epochs → Save checkpoint
Day 2: Power outage at epoch 50
Day 2 (recovery): Resume from epoch 100
Day 3: Complete 200 epochs → Version 1.0.0
```

**Benefit**: Saved 100 epochs of wasted retraining!

### Scenario 2: Model Registry

```
v1.0.0: 95% accuracy (production)
v1.1.0: 96% accuracy (staging)
v1.1.1: 94% accuracy (staging - regression!)
    ↓ Rollback to v1.1.0
v1.2.0: 97% accuracy (production)
```

**Benefit**: Safe experimentation with rollback capability

### Scenario 3: Distributed Training

```
Tower A: Trains shard 1 → Checkpoint A
Tower B: Trains shard 2 → Checkpoint B
Coordinator: Merges A + B → Final model
```

**Benefit**: Each tower can recover independently

---

## API Reference

### Checkpoint API

```rust
// Save checkpoint
save_checkpoint(checkpoint: &Checkpoint, data: &[u8]) -> Result<String>

// Load checkpoint
load_checkpoint(checkpoint_id: &str) -> Result<(Checkpoint, Vec<u8>)>

// List checkpoints
list_checkpoints(model_name: &str) -> Result<Vec<Checkpoint>>
```

### Model Versioning API

```rust
// Save model version
save_model_version(version: &ModelVersion, data: &[u8]) -> Result<String>

// Load model version
load_model_version(version_id: &str) -> Result<(ModelVersion, Vec<u8>)>

// List versions
list_model_versions(model_name: &str) -> Result<Vec<ModelVersion>>
```

---

## Next Steps

### Extend This Demo

1. **Real MNIST Training** - Replace mock with actual backpropagation
2. **Automatic Resume** - Detect and resume from latest checkpoint
3. **Version Comparison** - Compare accuracy across versions
4. **Distributed Checkpointing** - Coordinate checkpoints across towers

### Integrate with Other Primals

1. **+ Songbird** - Distributed training with coordinated checkpoints
2. **+ BearDog** - Encrypted model weights and checkpoints
3. **+ Squirrel** - AI-driven checkpoint frequency optimization

---

## Troubleshooting

### Issue: "Storage service not responding"

**Solution**: Demo runs in local-only mode - checkpoints aren't actually saved but training continues

### Issue: "Checkpoint hash mismatch"

**Solution**: Data corruption detected - checkpoint invalid, start from previous checkpoint

### Issue: "No checkpoints found"

**Solution**: Fresh training run - this is expected for first run

---

## Files in This Demo

```
03-nestgate-ml-pipeline/
├── README.md                        # This file
├── Cargo.toml                       # Project config
├── demo-train-with-checkpoints.sh   # Main demo script
├── src/
│   ├── nestgate_client.rs           # NestGate API client
│   └── train_with_checkpoints.rs    # Training with checkpointing
└── outputs/
    └── checkpoints/                 # Local checkpoint cache
```

---

## Success Criteria

You've successfully completed this demo when:

- [x] Training runs for 10 epochs
- [x] Checkpoints saved every 2 epochs (5 total)
- [x] Final model versioned and tagged
- [x] No "NestGate" hardcoded in code
- [x] Capability-based discovery used

---

**Status**: ✅ **Tutorial Ready**  
**Difficulty**: ⭐⭐ Intermediate  
**Prerequisites**: Basic ML, checkpointing concepts

🚀 **Production ML pipelines, powered by ToadStool + NestGate!** 🦀

