// Training coordinator for distributed execution

use super::{TowerInfo, TrainingTask, TrainingResult};

pub struct DistributedCoordinator {
    towers: Vec<TowerInfo>,
}

impl DistributedCoordinator {
    pub fn new(towers: Vec<TowerInfo>) -> Self {
        Self { towers }
    }
    
    pub fn create_tasks(&self, dataset_size: usize, batch_size: usize) -> Vec<TrainingTask> {
        let shard_size = dataset_size / self.towers.len();
        
        self.towers.iter().enumerate().map(|(i, _tower)| {
            TrainingTask {
                task_id: format!("task-{}", i),
                shard_start: i * shard_size,
                shard_end: if i == self.towers.len() - 1 {
                    dataset_size
                } else {
                    (i + 1) * shard_size
                },
                batch_size,
                learning_rate: 0.1,
            }
        }).collect()
    }
    
    pub fn aggregate_results(&self, results: Vec<TrainingResult>) -> (f64, f64) {
        let total_samples: usize = results.iter().map(|r| r.samples_trained).sum();
        let avg_loss: f64 = results.iter()
            .map(|r| r.loss * r.samples_trained as f64)
            .sum::<f64>() / total_samples as f64;
        let avg_accuracy: f64 = results.iter()
            .map(|r| r.accuracy * r.samples_trained as f64)
            .sum::<f64>() / total_samples as f64;
        
        (avg_loss, avg_accuracy)
    }
}

