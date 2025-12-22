// Single-tower training (baseline)

use anyhow::Result;
use std::time::Instant;
use tch::{nn, nn::OptimizerConfig, Device};
use toadstool_deep_learning::{data, models, models::Model, TrainingConfig, TrainingMetrics};

fn train_epoch(
    model: &models::resnet18::ResNet18,
    _vs: &nn::VarStore,
    opt: &mut nn::Optimizer,
    dataset: &dyn data::DataLoader,
    config: &TrainingConfig,
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
        
        let batch_images = train_images
            .narrow(0, start_idx, end_idx - start_idx)
            .to_device(config.device);
        let batch_labels = train_labels
            .narrow(0, start_idx, end_idx - start_idx)
            .to_device(config.device);
        
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
        
        if batch_idx % 50 == 0 {
            let batch_acc = 100.0 * total_correct as f64 / (end_idx as f64);
            tracing::info!(
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

fn main() -> Result<()> {
    toadstool_deep_learning::init_logging();
    
    tracing::info!("🧠 ToadStool Deep Learning - Single Tower Training");
    tracing::info!("📊 ResNet-18 on CIFAR-10");
    
    // Configuration
    let mut config = TrainingConfig::default();
    config.epochs = 10; // Quick test: 10 epochs
    config.batch_size = 128;
    config.learning_rate = 0.1;
    
    // Check CUDA availability
    if !tch::Cuda::is_available() {
        tracing::warn!("CUDA not available, using CPU");
        config.device = Device::Cpu;
    } else {
        let device_count = tch::Cuda::device_count();
        tracing::info!("🎮 Found {} CUDA device(s)", device_count);
        config.device = Device::Cuda(0);
    }
    
    tracing::info!("Device: {:?}", config.device);
    tracing::info!("Epochs: {}", config.epochs);
    tracing::info!("Batch size: {}", config.batch_size);
    tracing::info!("Learning rate: {}", config.learning_rate);
    
    // Load dataset
    tracing::info!("📦 Loading CIFAR-10...");
    let dataset = data::load_dataset("cifar10", "datasets/cifar-10-binary")?;
    tracing::info!("✅ Dataset loaded");
    
    // Create model
    tracing::info!("🏗️  Creating ResNet-18...");
    let vs = nn::VarStore::new(config.device);
    let model = models::resnet18::ResNet18::new(&vs.root(), dataset.num_classes());
    tracing::info!("✅ Model created ({:.1}M parameters)", model.num_parameters() as f64 / 1e6);
    
    // Optimizer
    let mut opt = nn::Adam::default().build(&vs, config.learning_rate)?;
    
    tracing::info!("🚀 Starting training...");
    tracing::info!("");
    
    let training_start = Instant::now();
    let mut all_metrics = Vec::new();
    
    for epoch in 1..=config.epochs {
        let epoch_start = Instant::now();
        
        tracing::info!("═══ Epoch {}/{} ═══", epoch, config.epochs);
        
        // Train
        let (train_loss, train_acc) = train_epoch(&model, &vs, &mut opt, dataset.as_ref(), &config)?;
        
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
            "Test:  loss={:.4}, acc={:.2}%",
            test_loss,
            test_acc
        );
        tracing::info!(
            "Time: {:.1}s ({:.0} samples/sec)",
            epoch_time,
            samples_per_sec
        );
        tracing::info!("");
        
        all_metrics.push(metrics);
        
        // Save checkpoint every 5 epochs
        if epoch % 5 == 0 {
            let checkpoint_path = format!("checkpoints/resnet18-cifar10-epoch{}.pt", epoch);
            vs.save(&checkpoint_path)?;
            tracing::info!("💾 Checkpoint saved: {}", checkpoint_path);
        }
    }
    
    let total_time = training_start.elapsed().as_secs_f64();
    
    tracing::info!("🎉 Training complete!");
    tracing::info!("Total time: {:.1} minutes", total_time / 60.0);
    
    if let Some(best) = all_metrics.iter().max_by(|a, b| {
        a.test_accuracy.partial_cmp(&b.test_accuracy).unwrap()
    }) {
        tracing::info!("Best test accuracy: {:.2}% (epoch {})", best.test_accuracy, best.epoch);
    }
    
    // Save final model
    vs.save("checkpoints/resnet18-cifar10-final.pt")?;
    tracing::info!("💾 Final model saved");
    
    // Save metrics
    let metrics_json = serde_json::to_string_pretty(&all_metrics)?;
    std::fs::write("outputs/training-metrics.json", metrics_json)?;
    tracing::info!("📊 Metrics saved");
    
    Ok(())
}

