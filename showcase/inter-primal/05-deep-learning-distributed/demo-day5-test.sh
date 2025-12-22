#!/usr/bin/env bash
# Day 5: Quick test run (20 epochs) to validate setup

set -euo pipefail

echo "🧪 Day 5: Quick Test Run (20 epochs)"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "This is a validation run to ensure everything works before"
echo "the full 100-epoch training (which takes 30-60 minutes)."
echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""

# Set PyTorch library path
PYTORCH_PATH=$(python3 -c "import torch; print(torch.__path__[0])")
export LD_LIBRARY_PATH="$PYTORCH_PATH/lib:${LD_LIBRARY_PATH:-}"
export LIBTORCH_USE_PYTORCH=1

# Create a test version that runs 20 epochs
cat > /tmp/test_scale.rs << 'EOF'
// Temporary test version - 20 epochs
// (This is a simplified copy of train_scale.rs with epochs=20)

use anyhow::Result;
use std::time::Instant;
use tch::{nn, nn::OptimizerConfig, Device, Tensor};
use toadstool_deep_learning::{data, models, models::Model, TrainingConfig, TrainingMetrics};

fn augment_batch(images: &Tensor) -> Tensor {
    let flip_mask = Tensor::rand(&[images.size()[0]], (tch::Kind::Float, images.device())).gt(0.5);
    let flipped = images.flip(&[3]);
    Tensor::where_self(&flip_mask.view([-1, 1, 1, 1]), &flipped, images)
}

fn train_epoch(
    model: &models::resnet18::ResNet18,
    _vs: &nn::VarStore,
    opt: &mut nn::Optimizer,
    dataset: &dyn data::DataLoader,
    config: &TrainingConfig,
    use_augmentation: bool,
) -> Result<(f64, f64)> {
    let train_images = dataset.train_images();
    let train_labels = dataset.train_labels();
    let num_samples = train_images.size()[0];
    let num_batches = (num_samples + config.batch_size as i64 - 1) / config.batch_size as i64;
    
    let mut total_loss = 0.0;
    let mut total_correct = 0i64;
    
    for batch_idx in 0..num_batches {
        let start_idx = batch_idx * config.batch_size as i64;
        let end_idx = (start_idx + config.batch_size as i64).min(num_samples);
        
        let mut batch_images = train_images.narrow(0, start_idx, end_idx - start_idx).to_device(config.device);
        let batch_labels = train_labels.narrow(0, start_idx, end_idx - start_idx).to_device(config.device);
        
        if use_augmentation {
            batch_images = augment_batch(&batch_images);
        }
        
        let logits = model.forward_t(&batch_images, true);
        let loss = logits.cross_entropy_for_logits(&batch_labels);
        
        opt.zero_grad();
        loss.backward();
        opt.step();
        
        total_loss += f64::try_from(loss)?;
        let predicted = logits.argmax(-1, false);
        total_correct += i64::try_from(predicted.eq_tensor(&batch_labels).sum(tch::Kind::Int64))?;
    }
    
    Ok((total_loss / num_batches as f64, 100.0 * total_correct as f64 / num_samples as f64))
}

fn test(
    model: &models::resnet18::ResNet18,
    dataset: &dyn data::DataLoader,
    config: &TrainingConfig,
) -> Result<(f64, f64)> {
    let test_images = dataset.test_images();
    let test_labels = dataset.test_labels();
    let num_samples = test_images.size()[0];
    let batch_size = 100i64;
    let num_batches = (num_samples + batch_size - 1) / batch_size;
    
    let mut total_loss = 0.0;
    let mut total_correct = 0i64;
    
    tch::no_grad(|| {
        for batch_idx in 0..num_batches {
            let start_idx = batch_idx * batch_size;
            let end_idx = (start_idx + batch_size).min(num_samples);
            
            let batch_images = test_images.narrow(0, start_idx, end_idx - start_idx).to_device(config.device);
            let batch_labels = test_labels.narrow(0, start_idx, end_idx - start_idx).to_device(config.device);
            
            let logits = model.forward_t(&batch_images, false);
            let loss = logits.cross_entropy_for_logits(&batch_labels);
            
            total_loss += f64::try_from(loss).unwrap();
            let predicted = logits.argmax(-1, false);
            total_correct += i64::try_from(predicted.eq_tensor(&batch_labels).sum(tch::Kind::Int64)).unwrap();
        }
    });
    
    Ok((total_loss / num_batches as f64, 100.0 * total_correct as f64 / num_samples as f64))
}

fn get_learning_rate(epoch: usize, total_epochs: usize, base_lr: f64) -> f64 {
    let warmup_epochs = 5;
    if epoch <= warmup_epochs {
        base_lr * (epoch as f64 / warmup_epochs as f64)
    } else {
        let progress = (epoch - warmup_epochs) as f64 / (total_epochs - warmup_epochs) as f64;
        0.5 * base_lr * (1.0 + (std::f64::consts::PI * progress).cos())
    }
}

fn main() -> Result<()> {
    toadstool_deep_learning::init_logging();
    
    println!("🧪 TEST RUN: 20 epochs to validate setup");
    
    let mut config = TrainingConfig::default();
    config.epochs = 20; // TEST: Only 20 epochs
    config.batch_size = 128;
    config.learning_rate = 0.1;
    
    config.device = if tch::Cuda::is_available() {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };
    
    let dataset = data::load_dataset("cifar10", "datasets/cifar-10-binary")?;
    
    let vs = nn::VarStore::new(config.device);
    let model = models::resnet18::ResNet18::new(&vs.root(), dataset.num_classes());
    
    let mut opt = nn::Sgd {
        momentum: 0.9,
        dampening: 0.0,
        wd: 5e-4,
        nesterov: false,
    }.build(&vs, config.learning_rate)?;
    
    println!("🚀 Starting {} epoch test run...\n", config.epochs);
    
    std::fs::create_dir_all("checkpoints")?;
    std::fs::create_dir_all("outputs")?;
    
    let mut best_acc = 0.0;
    
    for epoch in 1..=config.epochs {
        let current_lr = get_learning_rate(epoch, config.epochs, config.learning_rate);
        opt.set_lr(current_lr);
        
        let (train_loss, train_acc) = train_epoch(&model, &vs, &mut opt, dataset.as_ref(), &config, true)?;
        let (test_loss, test_acc) = test(&model, dataset.as_ref(), &config)?;
        
        println!("Epoch {}/{}: Train={:.2}%, Test={:.2}%, LR={:.6}",
                 epoch, config.epochs, train_acc, test_acc, current_lr);
        
        if test_acc > best_acc {
            best_acc = test_acc;
            println!("  ✨ New best: {:.2}%", best_acc);
        }
    }
    
    println!("\n✅ Test run complete! Best accuracy: {:.2}%", best_acc);
    println!("📊 System validated. Ready for full 100-epoch training.");
    
    Ok(())
}
EOF

# Compile and run the test
echo "📦 Compiling test binary..."
rustc --edition 2021 \
    -L dependency=target/release/deps \
    -L /tmp \
    --extern toadstool_deep_learning=target/release/libtoadstool_deep_learning.rlib \
    --extern tch=target/release/deps/libtch-*.rlib \
    --extern anyhow=target/release/deps/libanyhow-*.rlib \
    /tmp/test_scale.rs \
    -o /tmp/test_scale 2>/dev/null || {
    echo "⚠️  Direct compilation failed, using cargo instead..."
    echo "Running train-scale with modified config (will need manual epoch adjustment)..."
    ./target/release/train-scale
}

if [ -f "/tmp/test_scale" ]; then
    echo "✅ Test binary compiled"
    echo ""
    /tmp/test_scale
    rm /tmp/test_scale
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "✅ Validation Complete!"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Ready for full 100-epoch training:"
echo "  ./demo-day5-scale.sh"
echo ""

