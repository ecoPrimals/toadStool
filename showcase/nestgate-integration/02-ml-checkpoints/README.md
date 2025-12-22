# ML Checkpoint Management with NestGate

**Level**: 1 (One-Way Integration)  
**Time**: 10 minutes  
**Goal**: Demonstrate automatic ML checkpoint saving to NestGate

---

## 🎯 What This Demonstrates

**Automatic ML Checkpointing Pipeline**:
- ToadStool trains ML model
- Automatic checkpoint saving every N epochs
- Checkpoints stored in NestGate (versioned)
- Resume training from any checkpoint
- Zero-configuration workflow

---

## 🚀 Quick Start

```bash
./demo-automatic-checkpointing.sh
```

---

## 📊 What You'll See

### Training Progress
- 20 epochs of simulated ML training
- Loss decreasing, accuracy increasing
- Automatic checkpoints at epochs 5, 10, 15, 20

### Checkpoint Storage
- Each checkpoint stored in NestGate
- Versioned storage (v1, v2, v3...)
- Rich metadata (epoch, loss, accuracy)

### Resume Capability
- Load checkpoint from epoch 10
- Resume training from that point
- No training progress lost

### Visual Flow
```
ML Training → Generate Checkpoint → Store in NestGate
   (ToadStool)                         (Persistent)
       ↓                                      ↓
   Continue training              Can resume from any checkpoint
```

---

## 💡 Key Concepts

### Automatic Checkpointing
- **What**: Periodic saving of model state during training
- **Why**: Never lose training progress, resume from any point
- **How**: ToadStool automatically saves to NestGate every N epochs

### Versioned Storage
- **What**: Each checkpoint is a separate version
- **Why**: Track training evolution, compare different stages
- **How**: NestGate provides built-in versioning

### Checkpoint Metadata
- **What**: Training metrics stored with checkpoint
- **Why**: Understand model performance at each stage
- **How**: Epoch, loss, accuracy, learning rate, etc.

### Resume Training
- **What**: Load checkpoint and continue training
- **Why**: Handle interruptions gracefully
- **How**: Restore model + optimizer state from NestGate

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│           AUTOMATIC CHECKPOINT PIPELINE             │
└─────────────────────────────────────────────────────┘

        ToadStool (ML Training)
                │
                │ Every N epochs
                ↓
         Generate Checkpoint
         (model + optimizer)
                │
                │ Automatic
                ↓
            🗄️ NestGate
         (Persistent Storage)
                │
      ┌─────────┼─────────┐
      │         │         │
   Version   Metadata  Retrieve
      │         │         │
      ↓         ↓         ↓
   v1,v2...  epoch,   Resume
             loss,    training
           accuracy
```

---

## 📋 Demo Flow

1. **Prerequisites Check**: Verify NestGate availability (or demo mode)
2. **Discovery**: Find NestGate via capability-based discovery
3. **Training**: Run 20 epochs with simulated training
4. **Checkpointing**: Save at epochs 5, 10, 15, 20
5. **Storage**: Each checkpoint stored in NestGate
6. **Query**: List all saved checkpoints
7. **Resume**: Load checkpoint from epoch 10
8. **Visualization**: Show complete workflow

---

## 🎓 Real-World Use Cases

### Research & Experimentation
- Save checkpoints during long training runs
- Never lose experiment results
- Compare model performance at different epochs

### Production ML Training
- Handle interruptions gracefully (power loss, crashes)
- Resume expensive training without starting over
- Implement early stopping with best checkpoint

### Cost Optimization
- Resume training on spot instances
- Don't waste compute on re-training
- Save money on interrupted jobs

### Reproducibility
- Complete training history
- Audit trail of model development
- Share checkpoints with team

---

## 🔧 Configuration

### Checkpoint Interval
```bash
# Default: every 5 epochs
CHECKPOINT_INTERVAL=5

# More frequent: every epoch
CHECKPOINT_INTERVAL=1

# Less frequent: every 10 epochs
CHECKPOINT_INTERVAL=10
```

### Storage Location
```bash
# Default: ml-training/checkpoints/mnist/
STORAGE_KEY_PREFIX="ml-training/checkpoints/mnist/"

# Custom: your-experiment/checkpoints/
STORAGE_KEY_PREFIX="your-experiment/checkpoints/"
```

---

## 💾 Checkpoint Contents

Each checkpoint includes:

1. **Model State**
   - Trained weights
   - Model architecture
   - Layer configurations

2. **Optimizer State**
   - Learning rate
   - Momentum values
   - Adam beta values

3. **Training Metadata**
   - Current epoch
   - Training loss
   - Validation accuracy
   - Hyperparameters

4. **Storage Metadata**
   - Checkpoint ID
   - Storage key
   - Creation timestamp
   - File size

---

## 🚀 Performance

### Checkpoint Size
- Small model (MNIST): ~300KB per checkpoint
- Medium model (ResNet): ~100MB per checkpoint
- Large model (GPT-style): ~1-10GB per checkpoint

### Storage Speed
- Checkpoint save: <1 second (small models)
- Checkpoint load: <1 second (small models)
- Minimal impact on training time

### Storage Efficiency
- NestGate compression: 2-3x reduction
- Deduplication: Share common weights
- Incremental checkpoints: Only save differences

---

## ➡️ Next Steps

### Explore Related Demos

**Level 1 (One-Way Integration)**:
- **03-dataset-management**: Store and version training datasets
- **04-model-registry**: Store and compare trained models

**Level 2 (Bidirectional)**:
- **01-data-triggered-compute**: Automatically train when data arrives
- **02-distributed-storage**: Distribute checkpoints across nodes

**Level 3 (Multi-Primal)**:
- **03-coordinated-compute**: Songbird orchestrates checkpointed training
- **02-encrypted-storage**: BearDog encrypts checkpoints

---

## 🎯 Success Criteria

After running this demo, you should understand:
- ✅ How automatic checkpointing works
- ✅ Why versioned storage is valuable
- ✅ How to resume training from checkpoints
- ✅ When to use checkpointing in production

---

## 🔍 Deep Dive

### Implementation Pattern

```python
# Pseudocode: ToadStool training loop with automatic checkpointing

def train_with_checkpointing(model, data, nestgate_client):
    for epoch in range(total_epochs):
        # Train for one epoch
        loss, accuracy = train_epoch(model, data)
        
        # Automatic checkpoint at intervals
        if epoch % checkpoint_interval == 0:
            # Create checkpoint
            checkpoint = {
                'model_state': model.state_dict(),
                'optimizer_state': optimizer.state_dict(),
                'epoch': epoch,
                'loss': loss,
                'accuracy': accuracy
            }
            
            # Store in NestGate (automatic!)
            nestgate_client.store_checkpoint(
                key=f"ml-training/checkpoints/epoch_{epoch}",
                data=checkpoint,
                metadata={'epoch': epoch, 'loss': loss}
            )
```

---

## 📚 References

- **NestGate Storage API**: `../../../nestgate-standalone/01-storage-basics/`
- **ToadStool ML Examples**: `../../../python-ml/`
- **Multi-Primal Checkpointing**: `../../../multi-primal-nestgate/03-coordinated-compute/`

---

*Demo Level: 1 (One-Way Integration)*  
*Dependencies: NestGate (optional, demo mode available)*  
*Time to Complete: 10 minutes*

