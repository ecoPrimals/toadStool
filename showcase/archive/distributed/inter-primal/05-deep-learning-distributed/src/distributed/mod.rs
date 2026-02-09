// Distributed training coordination

pub mod coordinator;
pub mod worker;
pub mod gradient_sync;

/// Tower information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TowerInfo {
    pub name: String,
    pub url: String,
    pub gpu_model: String,
    pub vram_gb: usize,
}

/// Training task for a tower
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingTask {
    pub task_id: String,
    pub shard_start: usize,
    pub shard_end: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
}

/// Training result from a tower
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingResult {
    pub task_id: String,
    pub loss: f64,
    pub accuracy: f64,
    pub samples_trained: usize,
    pub time_secs: f64,
}

