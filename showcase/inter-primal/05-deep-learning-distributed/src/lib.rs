// 🧠 ToadStool Deep Learning - Cross-Tower Distributed Training
// Production deep learning across physical towers

pub mod models;
pub mod data;
pub mod distributed;
pub mod songbird_client;
pub mod optimization;
pub mod checkpoint;

use std::path::PathBuf;

/// Configuration for training
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub model: String,
    pub dataset: String,
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub device: tch::Device,
    pub checkpoint_dir: PathBuf,
    pub num_towers: usize,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model: "resnet18".to_string(),
            dataset: "cifar10".to_string(),
            epochs: 100,
            batch_size: 128,
            learning_rate: 0.1,
            device: tch::Device::Cuda(0),
            checkpoint_dir: PathBuf::from("checkpoints"),
            num_towers: 1,
        }
    }
}

/// Training metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrainingMetrics {
    pub epoch: usize,
    pub train_loss: f64,
    pub train_accuracy: f64,
    pub test_loss: f64,
    pub test_accuracy: f64,
    pub epoch_time_secs: f64,
    pub samples_per_sec: f64,
}

/// Initialize logging
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

