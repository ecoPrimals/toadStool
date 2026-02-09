# ToadStool Deep Learning - Usage Guide

**Version**: 1.0  
**Date**: December 19, 2025

---

## 🎯 Overview

ToadStool Deep Learning provides production-grade distributed ML training across multiple GPU towers using the ToadStool + Songbird ecosystem.

**Features**:
- ResNet-18 image classification
- CIFAR-10 dataset support
- Single-tower and distributed training
- Advanced optimizations (mixed precision, LR scheduling)
- Checkpoint management
- Songbird federation integration

---

## 🚀 Quick Start

### 1. Download Dataset

```bash
cd showcase/inter-primal/05-deep-learning-distributed

# Download CIFAR-10 (binary format, ~170 MB)
./target/release/download-cifar10

# Dataset will be in: datasets/cifar-10-binary/
```

### 2. Single-Tower Training

```bash
# Train ResNet-18 on CIFAR-10 (10 epochs, ~5 minutes on GPU)
PYTORCH_PATH=$(python3 -c "import torch; print(torch.__path__[0])")
export LD_LIBRARY_PATH="$PYTORCH_PATH/lib:${LD_LIBRARY_PATH:-}"
export LIBTORCH_USE_PYTORCH=1

./target/release/train-single
```

**Expected output**:
```
Epoch 1/10: Train=45.2%, Test=48.3%
Epoch 2/10: Train=58.7%, Test=60.1%
...
Epoch 10/10: Train=88.3%, Test=85.2%
Best test accuracy: 85.2%
```

### 3. Distributed Training (2 Towers)

```bash
# Train across Eastgate + Strandgate
./demo-day2-distributed.sh
```

---

## 📚 Training Modes

### Mode 1: Baseline (10 epochs)

**Purpose**: Quick validation

```bash
./target/release/train-single
```

**Specs**:
- Epochs: 10
- Batch size: 128
- Optimizer: Adam (LR: 0.1)
- Expected accuracy: ~85%
- Time: ~5 minutes (GPU)

---

### Mode 2: Distributed (20 epochs)

**Purpose**: Multi-tower coordination

```bash
./demo-day2-distributed.sh
```

**Specs**:
- Towers: 2 (Eastgate + Strandgate)
- Data sharding: 50/50 split
- Gradient synchronization: All-reduce
- Expected accuracy: ~88%
- Time: ~10 minutes (2x GPU)

---

### Mode 3: Scaling (100 epochs)

**Purpose**: Maximum accuracy

```bash
./demo-day5-scale.sh
```

**Specs**:
- Epochs: 100 (with early stopping)
- Batch size: 128
- Optimizer: SGD (momentum=0.9, wd=5e-4)
- LR schedule: Warmup + cosine decay
- Data augmentation: Random flip
- Expected accuracy: ~95%
- Time: ~30-60 minutes (GPU)

---

## 🛠️ Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LIBTORCH_USE_PYTORCH` | Use Python PyTorch libs | `1` |
| `LD_LIBRARY_PATH` | PyTorch library path | (auto-detected) |
| `RUST_LOG` | Logging level | `info` |

### Training Config

Edit `src/bin/train_*.rs` to customize:

```rust
let mut config = TrainingConfig::default();
config.epochs = 100;          // Number of epochs
config.batch_size = 128;      // Batch size
config.learning_rate = 0.1;   // Base learning rate
config.device = Device::Cuda(0); // GPU device
```

---

## 📊 Monitoring

### Real-Time Logs

Training prints progress every epoch:

```
═══ Epoch 50/100 (LR: 0.045612) ═══
Train: loss=0.234, acc=92.15%
Test:  loss=0.312, acc=90.43% 🎯
Time: 44.2s (1132 samples/sec)
✨ New best accuracy: 90.43%
💾 Checkpoint saved
```

### Metrics Files

All metrics saved to `outputs/`:

- `training-metrics.json` - Epoch-by-epoch metrics
- `training-metrics-100epoch.json` - Extended training metrics
- `DAY*_REPORT.md` - Summary reports

**Example metrics.json**:
```json
[
  {
    "epoch": 1,
    "train_loss": 1.8234,
    "train_accuracy": 32.45,
    "test_loss": 1.6543,
    "test_accuracy": 38.21,
    "epoch_time_secs": 45.2,
    "samples_per_sec": 1105.3
  },
  ...
]
```

### Checkpoints

Models saved to `checkpoints/`:

- `resnet18-cifar10-epoch{N}.pt` - Every 10 epochs
- `resnet18-cifar10-best.pt` - Best test accuracy
- `resnet18-cifar10-final.pt` - Final model

**Load checkpoint**:
```rust
use tch::nn::VarStore;

let mut vs = VarStore::new(Device::Cuda(0));
vs.load("checkpoints/resnet18-cifar10-best.pt")?;
```

---

## 🏗️ Architecture

### Model: ResNet-18

- **Parameters**: 11.7M
- **Layers**: 18 convolutional layers
- **Input**: 32x32x3 (CIFAR-10)
- **Output**: 10 classes
- **Features**: Skip connections, batch normalization

**Classes** (CIFAR-10):
1. airplane
2. automobile
3. bird
4. cat
5. deer
6. dog
7. frog
8. horse
9. ship
10. truck

### Dataset: CIFAR-10

- **Train**: 50,000 images
- **Test**: 10,000 images
- **Size**: ~170 MB (binary format)
- **Source**: https://www.cs.toronto.edu/~kriz/cifar.html

---

## 🔧 Advanced Usage

### Custom Model

Create `src/models/custom.rs`:

```rust
use tch::nn::{self, Module};

pub struct CustomNet {
    fc1: nn::Linear,
    fc2: nn::Linear,
}

impl CustomNet {
    pub fn new(vs: &nn::Path, num_classes: i64) -> Self {
        let fc1 = nn::linear(vs / "fc1", 3072, 512, Default::default());
        let fc2 = nn::linear(vs / "fc2", 512, num_classes, Default::default());
        CustomNet { fc1, fc2 }
    }
}

impl Module for CustomNet {
    fn forward(&self, xs: &Tensor) -> Tensor {
        xs.view([-1, 3072])
          .apply(&self.fc1)
          .relu()
          .apply(&self.fc2)
    }
}
```

### Custom Dataset

Implement `DataLoader` trait:

```rust
use toadstool_deep_learning::data::DataLoader;

struct MyDataset {
    train_images: Tensor,
    train_labels: Tensor,
    test_images: Tensor,
    test_labels: Tensor,
}

impl DataLoader for MyDataset {
    fn train_images(&self) -> &Tensor { &self.train_images }
    fn train_labels(&self) -> &Tensor { &self.train_labels }
    fn test_images(&self) -> &Tensor { &self.test_images }
    fn test_labels(&self) -> &Tensor { &self.test_labels }
    fn num_classes(&self) -> i64 { 10 }
}
```

---

## 🐛 Troubleshooting

### "CUDA not available"

**Problem**: No GPU detected

**Solutions**:
1. Check GPU: `nvidia-smi`
2. Install CUDA toolkit
3. Verify PyTorch CUDA: `python3 -c "import torch; print(torch.cuda.is_available())"`
4. Fall back to CPU (slow): Training will use CPU automatically

---

### "libtorch_cpu.so: cannot open shared object file"

**Problem**: PyTorch libraries not in `LD_LIBRARY_PATH`

**Solution**:
```bash
PYTORCH_PATH=$(python3 -c "import torch; print(torch.__path__[0])")
export LD_LIBRARY_PATH="$PYTORCH_PATH/lib:${LD_LIBRARY_PATH:-}"
```

---

### "Dataset not found"

**Problem**: CIFAR-10 not downloaded

**Solution**:
```bash
./target/release/download-cifar10
```

---

### Low Accuracy (<80%)

**Possible causes**:
1. **Too few epochs** - Train longer (50-100 epochs)
2. **Learning rate too high/low** - Try 0.01 or 0.001
3. **No data augmentation** - Use Day 5 training (has augmentation)
4. **Overfitting** - Add weight decay or dropout

---

## 📈 Performance Benchmarks

### Single Tower (Eastgate, RTX 2070)

| Configuration | Epochs | Accuracy | Time |
|---------------|--------|----------|------|
| Baseline | 10 | ~85% | ~5 min |
| Optimized | 50 | ~92% | ~20 min |
| Scaled | 100 | ~95% | ~40 min |

### Distributed (2 Towers)

| Configuration | Speedup | Accuracy | Notes |
|---------------|---------|----------|-------|
| Data Parallel | 1.8x | Same as single | Near-linear scaling |
| With Sync Overhead | 1.6x | Same as single | Gradient communication |

---

## 🔐 Security

### Model Encryption (BearDog)

Encrypt trained models:

```bash
# Export checkpoint
MODEL="checkpoints/resnet18-cifar10-best.pt"

# Encrypt with BearDog
beardog stream-encrypt \
    --key tower-key \
    --input $MODEL \
    --output $MODEL.enc
```

### Secure Distribution

Transfer encrypted models between towers:

```bash
# On coordinator
beardog key export --key-id tower-a-key --output tower-a.json
scp tower-a.json $MODEL.enc eastgate:/ml/models/

# On Eastgate
beardog key import --input tower-a.json
beardog stream-decrypt --input $MODEL.enc --output $MODEL
```

---

## 📚 API Reference

### TrainingConfig

```rust
pub struct TrainingConfig {
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub device: Device,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        TrainingConfig {
            epochs: 10,
            batch_size: 128,
            learning_rate: 0.1,
            device: Device::cuda_if_available(),
        }
    }
}
```

### TrainingMetrics

```rust
#[derive(Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub epoch: usize,
    pub train_loss: f64,
    pub train_accuracy: f64,
    pub test_loss: f64,
    pub test_accuracy: f64,
    pub epoch_time_secs: f64,
    pub samples_per_sec: f64,
}
```

### Model Trait

```rust
pub trait Model {
    fn build(vs: &nn::Path, num_classes: i64) -> Self;
    fn num_parameters(&self) -> usize;
}
```

---

## 🎓 Examples

See `showcase/inter-primal/05-deep-learning-distributed/examples/` for:

- `basic_training.rs` - Minimal training example
- `custom_augmentation.rs` - Data augmentation examples
- `transfer_learning.rs` - Fine-tuning pre-trained models
- `inference.rs` - Model inference/prediction

---

## 📞 Support

### Documentation

- **Main README**: `../README.md`
- **Architecture**: `docs/ARCHITECTURE.md`
- **Week 1 Plan**: `WEEK1_PLAN.md`

### Community

- **GitHub**: https://github.com/ecoPrimals/toadstool
- **Discord**: (coming soon)

---

**Created**: December 19, 2025  
**Author**: ToadStool Team  
**Status**: Production Ready

🧠🦀🚀 **Happy Training!** 🚀🦀🧠

