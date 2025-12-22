pub mod mnist;
pub mod network;
pub mod songbird_client;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTrainingStats {
    pub epoch: usize,
    pub tower_results: Vec<TowerTrainingResult>,
    pub aggregate_loss: f32,
    pub aggregate_accuracy: f32,
    pub training_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerTrainingResult {
    pub tower_id: String,
    pub samples_trained: usize,
    pub loss: f32,
    pub accuracy: f32,
    pub time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPartition {
    pub partition_id: usize,
    pub start_idx: usize,
    pub end_idx: usize,
    pub tower_id: String,
}

