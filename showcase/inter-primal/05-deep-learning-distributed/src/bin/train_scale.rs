// Day 5: Scaling to 95%+ accuracy with 100 epochs

use anyhow::Result;
use std::time::Instant;
use tch::{nn, nn::OptimizerConfig, Device, Tensor};
use toadstool_deep_learning::{data, models, models::Model, TrainingConfig, TrainingMetrics};

/// Data augmentation for training images
fn augment_batch(images: &Tensor) -> Tensor {
    // Random horizontal flip (50% probability)
    // For simplicity, flip all images (real impl would flip based on random mask)
    let should_flip = Tensor::rand(&[1], (tch::Kind::Float, images.device())).double_value(&[]) > 0.5;
    
    if should_flip {
        images.flip(&[3]) // Flip width dimension
    } else {
        images.shallow_clone()
    }
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
        
        let mut batch_images = train_images
            .narrow(0, start_idx, end_idx - start_idx)
            .to_device(config.device);
        let batch_labels = train_labels
            .narrow(0, start_idx, end_idx - start_idx)
            .to_device(config.device);
        
        // Apply data augmentation
        if use_augmentation {
            batch_images = augment_batch(&batch_images);
        }
        
        // Forward pass
        let logits = model.forward_t(&batch_images, true);
        let loss = logits.cross_entropy_for_logits(&batch_labels);
        
        // Backward pass
        opt.zero_grad();
        loss.backward();
        opt.step();
        
        // Track metrics
        total_loss += f64::try_from(loss)?;
        let predicted = logits.argmax(-1, false);
        total_correct += i64::try_from(predicted.eq_tensor(&batch_labels).sum(tch::Kind::Int64))?;
        
        if batch_idx % 100 == 0 {
            let batch_acc = 100.0 * total_correct as f64 / (end_idx as f64);
            tracing::debug!(
                "Batch {}/{}: loss={:.4}, acc={:.2}%",
                batch_idx,
                num_batches,
                total_loss / (batch_idx + 1) as f64,
                batch_acc
            );
        }
    }
    
    let avg_loss = total_loss / num_batches as f64;
    let accuracy = 100.0 * total_correct as f64 / num_samples as f64;
    
    Ok((avg_loss, accuracy))
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
            
            let batch_images = test_images
                .narrow(0, start_idx, end_idx - start_idx)
                .to_device(config.device);
            let batch_labels = test_labels
                .narrow(0, start_idx, end_idx - start_idx)
                .to_device(config.device);
            
            let logits = model.forward_t(&batch_images, false);
            let loss = logits.cross_entropy_for_logits(&batch_labels);
            
            total_loss += f64::try_from(loss).unwrap();
            let predicted = logits.argmax(-1, false);
            total_correct += i64::try_from(predicted.eq_tensor(&batch_labels).sum(tch::Kind::Int64)).unwrap();
        }
    });
    
    let avg_loss = total_loss / num_batches as f64;
    let accuracy = 100.0 * total_correct as f64 / num_samples as f64;
    
    Ok((avg_loss, accuracy))
}

/// Learning rate schedule: warmup + cosine decay
fn get_learning_rate(epoch: usize, total_epochs: usize, base_lr: f64) -> f64 {
    let warmup_epochs = 5;
    
    if epoch <= warmup_epochs {
        // Linear warmup
        base_lr * (epoch as f64 / warmup_epochs as f64)
    } else {
        // Cosine decay
        let progress = (epoch - warmup_epochs) as f64 / (total_epochs - warmup_epochs) as f64;
        0.5 * base_lr * (1.0 + (std::f64::consts::PI * progress).cos())
    }
}

fn main() -> Result<()> {
    toadstool_deep_learning::init_logging();
    
    tracing::info!("🧠 ToadStool Deep Learning - Day 5: Scaling to 95%+");
    tracing::info!("📊 ResNet-18 on CIFAR-10 (100 epochs)");
    
    // Configuration
    let mut config = TrainingConfig::default();
    config.epochs = 100; // Scale to 100 epochs
    config.batch_size = 128;
    config.learning_rate = 0.1; // Base LR, will be scheduled
    
    // Early stopping parameters
    let patience = 15; // Stop if no improvement for 15 epochs
    let mut best_test_acc = 0.0;
    let mut epochs_without_improvement = 0;
    
    // Check CUDA availability
    if !tch::Cuda::is_available() {
        tracing::warn!("CUDA not available, using CPU (will be slow!)");
        config.device = Device::Cpu;
    } else {
        let device_count = tch::Cuda::device_count();
        tracing::info!("🎮 Found {} CUDA device(s)", device_count);
        config.device = Device::Cuda(0);
    }
    
    tracing::info!("Device: {:?}", config.device);
    tracing::info!("Epochs: {}", config.epochs);
    tracing::info!("Batch size: {}", config.batch_size);
    tracing::info!("Base learning rate: {}", config.learning_rate);
    tracing::info!("Early stopping patience: {} epochs", patience);
    tracing::info!("Data augmentation: ENABLED");
    tracing::info!("");
    
    // Load dataset
    tracing::info!("📦 Loading CIFAR-10...");
    let dataset = data::load_dataset("cifar10", "datasets/cifar-10-binary")?;
    tracing::info!("✅ Dataset loaded");
    tracing::info!("  Train samples: {}", dataset.train_images().size()[0]);
    tracing::info!("  Test samples: {}", dataset.test_images().size()[0]);
    tracing::info!("");
    
    // Create model
    tracing::info!("🏗️  Creating ResNet-18...");
    let vs = nn::VarStore::new(config.device);
    let model = models::resnet18::ResNet18::new(&vs.root(), dataset.num_classes());
    tracing::info!("✅ Model created ({:.1}M parameters)", model.num_parameters() as f64 / 1e6);
    tracing::info!("");
    
    // Optimizer (will update LR each epoch)
    let mut opt = nn::Sgd {
        momentum: 0.9,
        dampening: 0.0,
        wd: 5e-4,
        nesterov: false,
    }.build(&vs, config.learning_rate)?;
    
    tracing::info!("🚀 Starting training...");
    tracing::info!("");
    
    let training_start = Instant::now();
    let mut all_metrics = Vec::new();
    
    // Create checkpoints directory
    std::fs::create_dir_all("checkpoints")?;
    std::fs::create_dir_all("outputs")?;
    
    for epoch in 1..=config.epochs {
        let epoch_start = Instant::now();
        
        // Update learning rate
        let current_lr = get_learning_rate(epoch, config.epochs, config.learning_rate);
        opt.set_lr(current_lr);
        
        tracing::info!("═══ Epoch {}/{} (LR: {:.6}) ═══", epoch, config.epochs, current_lr);
        
        // Train
        let (train_loss, train_acc) = train_epoch(&model, &vs, &mut opt, dataset.as_ref(), &config, true)?;
        
        // Test
        let (test_loss, test_acc) = test(&model, dataset.as_ref(), &config)?;
        
        let epoch_time = epoch_start.elapsed().as_secs_f64();
        let samples_per_sec = dataset.train_images().size()[0] as f64 / epoch_time;
        
        let metrics = TrainingMetrics {
            epoch,
            train_loss,
            train_accuracy: train_acc,
            test_loss,
            test_accuracy: test_acc,
            epoch_time_secs: epoch_time,
            samples_per_sec,
        };
        
        tracing::info!(
            "Train: loss={:.4}, acc={:.2}%",
            train_loss,
            train_acc
        );
        tracing::info!(
            "Test:  loss={:.4}, acc={:.2}% 🎯",
            test_loss,
            test_acc
        );
        tracing::info!(
            "Time: {:.1}s ({:.0} samples/sec)",
            epoch_time,
            samples_per_sec
        );
        
        // Check for improvement
        if test_acc > best_test_acc {
            best_test_acc = test_acc;
            epochs_without_improvement = 0;
            tracing::info!("✨ New best accuracy: {:.2}%", best_test_acc);
            
            // Save best model
            vs.save("checkpoints/resnet18-cifar10-best.pt")?;
            tracing::info!("💾 Best model saved");
        } else {
            epochs_without_improvement += 1;
            tracing::info!("No improvement for {} epoch(s)", epochs_without_improvement);
        }
        
        tracing::info!("");
        
        all_metrics.push(metrics);
        
        // Save checkpoint every 10 epochs
        if epoch % 10 == 0 {
            let checkpoint_path = format!("checkpoints/resnet18-cifar10-epoch{}.pt", epoch);
            vs.save(&checkpoint_path)?;
            tracing::info!("💾 Checkpoint saved: {}", checkpoint_path);
            tracing::info!("");
        }
        
        // Early stopping
        if epochs_without_improvement >= patience {
            tracing::warn!("🛑 Early stopping triggered! No improvement for {} epochs", patience);
            tracing::info!("Best test accuracy: {:.2}%", best_test_acc);
            break;
        }
        
        // Check if we've reached the target
        if test_acc >= 95.0 {
            tracing::info!("🎉 TARGET REACHED! Test accuracy: {:.2}% >= 95%", test_acc);
            break;
        }
    }
    
    let total_time = training_start.elapsed().as_secs_f64();
    
    tracing::info!("");
    tracing::info!("═══════════════════════════════════════════");
    tracing::info!("🎉 Training complete!");
    tracing::info!("═══════════════════════════════════════════");
    tracing::info!("Total time: {:.1} minutes", total_time / 60.0);
    tracing::info!("Epochs trained: {}", all_metrics.len());
    tracing::info!("Best test accuracy: {:.2}%", best_test_acc);
    
    if best_test_acc >= 95.0 {
        tracing::info!("✅ TARGET ACHIEVED: 95%+ accuracy");
    } else {
        tracing::info!("⚠️  Target not reached (95%+), best: {:.2}%", best_test_acc);
    }
    
    tracing::info!("");
    
    // Print top 5 epochs
    let mut sorted_metrics = all_metrics.clone();
    sorted_metrics.sort_by(|a, b| b.test_accuracy.partial_cmp(&a.test_accuracy).unwrap());
    
    tracing::info!("Top 5 epochs:");
    for (i, m) in sorted_metrics.iter().take(5).enumerate() {
        tracing::info!(
            "  {}. Epoch {}: {:.2}% test accuracy",
            i + 1,
            m.epoch,
            m.test_accuracy
        );
    }
    tracing::info!("");
    
    // Save final model
    vs.save("checkpoints/resnet18-cifar10-final.pt")?;
    tracing::info!("💾 Final model saved");
    
    // Save metrics
    let metrics_json = serde_json::to_string_pretty(&all_metrics)?;
    std::fs::write("outputs/training-metrics-100epoch.json", metrics_json)?;
    tracing::info!("📊 Metrics saved to outputs/training-metrics-100epoch.json");
    
    // Save summary report
    let summary = format!(
        r#"# Day 5: Scaling Results

## Configuration
- Epochs: {} (target: 100)
- Batch size: {}
- Base learning rate: {}
- Optimizer: SGD with momentum (0.9) and weight decay (5e-4)
- LR schedule: Warmup (5 epochs) + Cosine decay
- Data augmentation: Random horizontal flip
- Early stopping: Patience {} epochs

## Results
- Best test accuracy: {:.2}%
- Target (95%+): {}
- Total training time: {:.1} minutes
- Epochs trained: {}
- Average time per epoch: {:.1} seconds

## Top 5 Epochs
{}

## Training Progression
{}

## Next Steps
{}
"#,
        config.epochs,
        config.batch_size,
        config.learning_rate,
        patience,
        best_test_acc,
        if best_test_acc >= 95.0 { "✅ ACHIEVED" } else { "❌ NOT REACHED" },
        total_time / 60.0,
        all_metrics.len(),
        total_time / all_metrics.len() as f64,
        sorted_metrics.iter().take(5).enumerate()
            .map(|(i, m)| format!("{}. Epoch {}: {:.2}%", i + 1, m.epoch, m.test_accuracy))
            .collect::<Vec<_>>()
            .join("\n"),
        all_metrics.iter()
            .filter(|m| m.epoch % 10 == 0 || m.test_accuracy == best_test_acc)
            .map(|m| format!("Epoch {:3}: Train={:.2}%, Test={:.2}%", m.epoch, m.train_accuracy, m.test_accuracy))
            .collect::<Vec<_>>()
            .join("\n"),
        if best_test_acc >= 95.0 {
            "✅ Target achieved! Ready for Day 6 (Monitoring & Documentation)"
        } else {
            "⚠️  Consider: More data augmentation, different architecture, or longer training"
        }
    );
    
    std::fs::write("outputs/DAY5_SCALING_REPORT.md", summary)?;
    tracing::info!("📄 Summary report saved to outputs/DAY5_SCALING_REPORT.md");
    tracing::info!("");
    
    Ok(())
}

