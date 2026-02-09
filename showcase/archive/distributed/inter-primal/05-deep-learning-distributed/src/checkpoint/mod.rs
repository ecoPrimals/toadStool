// Checkpoint and resume functionality

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Training checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Checkpoint version for compatibility
    pub version: String,
    
    /// Epoch number
    pub epoch: usize,
    
    /// Global step number
    pub step: usize,
    
    /// Training loss at checkpoint
    pub train_loss: f64,
    
    /// Training accuracy at checkpoint
    pub train_accuracy: f64,
    
    /// Test loss at checkpoint
    pub test_loss: f64,
    
    /// Test accuracy at checkpoint
    pub test_accuracy: f64,
    
    /// Best test accuracy so far
    pub best_test_accuracy: f64,
    
    /// Best test accuracy epoch
    pub best_epoch: usize,
    
    /// Learning rate at checkpoint
    pub learning_rate: f64,
    
    /// Total training time (seconds)
    pub total_training_time_secs: f64,
    
    /// Checkpoint creation time
    pub created_at: SystemTime,
    
    /// Model configuration
    pub model_config: ModelConfig,
    
    /// Training configuration
    pub training_config: TrainingConfigSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_type: String,
    pub num_parameters: i64,
    pub num_classes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfigSnapshot {
    pub dataset: String,
    pub batch_size: usize,
    pub base_learning_rate: f64,
    pub optimizer: String,
    pub num_towers: usize,
}

/// Checkpoint manager
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    keep_last_n: usize,
    save_best: bool,
}

impl CheckpointManager {
    /// Create new checkpoint manager
    pub fn new<P: AsRef<Path>>(checkpoint_dir: P) -> Result<Self> {
        let checkpoint_dir = checkpoint_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&checkpoint_dir)?;
        
        Ok(Self {
            checkpoint_dir,
            keep_last_n: 5, // Keep last 5 checkpoints
            save_best: true,
        })
    }
    
    /// Configure how many checkpoints to keep
    pub fn keep_last_n(mut self, n: usize) -> Self {
        self.keep_last_n = n;
        self
    }
    
    /// Configure whether to save best checkpoint
    pub fn save_best(mut self, save: bool) -> Self {
        self.save_best = save;
        self
    }
    
    /// Save checkpoint
    pub fn save_checkpoint(
        &self,
        metadata: &CheckpointMetadata,
        model_path: &str,
        is_best: bool,
    ) -> Result<PathBuf> {
        // Save metadata
        let metadata_path = self.checkpoint_dir.join(format!(
            "checkpoint-epoch{:04}-metadata.json",
            metadata.epoch
        ));
        
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;
        
        // Copy model weights
        let model_dest = self.checkpoint_dir.join(format!(
            "checkpoint-epoch{:04}.pt",
            metadata.epoch
        ));
        std::fs::copy(model_path, &model_dest)?;
        
        tracing::info!("💾 Checkpoint saved: epoch {} (test_acc: {:.2}%)", 
            metadata.epoch, metadata.test_accuracy);
        
        // Save as best if applicable
        if is_best && self.save_best {
            let best_model = self.checkpoint_dir.join("best-model.pt");
            let best_metadata = self.checkpoint_dir.join("best-model-metadata.json");
            
            std::fs::copy(&model_dest, &best_model)?;
            std::fs::copy(&metadata_path, &best_metadata)?;
            
            tracing::info!("🏆 New best model saved! Accuracy: {:.2}%", metadata.test_accuracy);
        }
        
        // Clean up old checkpoints
        self.cleanup_old_checkpoints()?;
        
        Ok(model_dest)
    }
    
    /// Load checkpoint
    pub fn load_checkpoint<P: AsRef<Path>>(&self, checkpoint_path: P) -> Result<CheckpointMetadata> {
        let checkpoint_path = checkpoint_path.as_ref();
        
        // Determine metadata path
        let metadata_path = if checkpoint_path.to_str().unwrap().ends_with(".pt") {
            checkpoint_path.with_extension("json")
        } else {
            checkpoint_path.to_path_buf()
        };
        
        let metadata_json = std::fs::read_to_string(&metadata_path)?;
        let metadata: CheckpointMetadata = serde_json::from_str(&metadata_json)?;
        
        tracing::info!("📂 Loaded checkpoint from epoch {}", metadata.epoch);
        tracing::info!("   Test accuracy: {:.2}%", metadata.test_accuracy);
        tracing::info!("   Training time: {:.1}s", metadata.total_training_time_secs);
        
        Ok(metadata)
    }
    
    /// Find latest checkpoint
    pub fn find_latest_checkpoint(&self) -> Result<Option<PathBuf>> {
        let mut checkpoints = Vec::new();
        
        for entry in std::fs::read_dir(&self.checkpoint_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("pt") &&
               path.file_name().and_then(|s| s.to_str()).map(|s| s.starts_with("checkpoint-")) == Some(true) {
                checkpoints.push(path);
            }
        }
        
        checkpoints.sort();
        Ok(checkpoints.last().cloned())
    }
    
    /// Find best checkpoint
    pub fn find_best_checkpoint(&self) -> Result<Option<PathBuf>> {
        let best_path = self.checkpoint_dir.join("best-model.pt");
        if best_path.exists() {
            Ok(Some(best_path))
        } else {
            Ok(None)
        }
    }
    
    /// List all checkpoints
    pub fn list_checkpoints(&self) -> Result<Vec<(PathBuf, CheckpointMetadata)>> {
        let mut checkpoints = Vec::new();
        
        for entry in std::fs::read_dir(&self.checkpoint_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") &&
               path.file_name().and_then(|s| s.to_str()).map(|s| s.contains("metadata")) == Some(true) {
                if let Ok(metadata_json) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<CheckpointMetadata>(&metadata_json) {
                        let model_path = path.with_extension("pt");
                        if model_path.exists() {
                            checkpoints.push((model_path, metadata));
                        }
                    }
                }
            }
        }
        
        checkpoints.sort_by_key(|(_, meta)| meta.epoch);
        Ok(checkpoints)
    }
    
    /// Clean up old checkpoints (keep last N)
    fn cleanup_old_checkpoints(&self) -> Result<()> {
        let mut checkpoints = self.list_checkpoints()?;
        
        if checkpoints.len() > self.keep_last_n {
            checkpoints.sort_by_key(|(_, meta)| meta.epoch);
            
            let to_remove = checkpoints.len() - self.keep_last_n;
            for (path, metadata) in checkpoints.iter().take(to_remove) {
                // Don't delete if it's the best
                if metadata.test_accuracy == metadata.best_test_accuracy {
                    continue;
                }
                
                let metadata_path = path.with_extension("json");
                let _ = std::fs::remove_file(path);
                let _ = std::fs::remove_file(metadata_path);
                
                tracing::debug!("🗑️  Removed old checkpoint: epoch {}", metadata.epoch);
            }
        }
        
        Ok(())
    }
}

/// Training state for resume
#[derive(Debug, Clone)]
pub struct TrainingState {
    pub start_epoch: usize,
    pub best_test_accuracy: f64,
    pub best_epoch: usize,
    pub total_training_time: f64,
}

impl TrainingState {
    pub fn new() -> Self {
        Self {
            start_epoch: 0,
            best_test_accuracy: 0.0,
            best_epoch: 0,
            total_training_time: 0.0,
        }
    }
    
    pub fn from_checkpoint(metadata: &CheckpointMetadata) -> Self {
        Self {
            start_epoch: metadata.epoch + 1,
            best_test_accuracy: metadata.best_test_accuracy,
            best_epoch: metadata.best_epoch,
            total_training_time: metadata.total_training_time_secs,
        }
    }
}

impl Default for TrainingState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;
    
    #[test]
    fn test_checkpoint_metadata_serialization() {
        let metadata = CheckpointMetadata {
            version: "1.0".to_string(),
            epoch: 10,
            step: 1000,
            train_loss: 0.5,
            train_accuracy: 85.0,
            test_loss: 0.6,
            test_accuracy: 83.0,
            best_test_accuracy: 83.0,
            best_epoch: 10,
            learning_rate: 0.01,
            total_training_time_secs: 600.0,
            created_at: UNIX_EPOCH,
            model_config: ModelConfig {
                model_type: "ResNet18".to_string(),
                num_parameters: 11_700_000,
                num_classes: 10,
            },
            training_config: TrainingConfigSnapshot {
                dataset: "CIFAR-10".to_string(),
                batch_size: 128,
                base_learning_rate: 0.1,
                optimizer: "Adam".to_string(),
                num_towers: 2,
            },
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CheckpointMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.epoch, 10);
        assert_eq!(deserialized.test_accuracy, 83.0);
    }
    
    #[test]
    fn test_training_state() {
        let state = TrainingState::new();
        assert_eq!(state.start_epoch, 0);
        assert_eq!(state.best_test_accuracy, 0.0);
        
        let metadata = CheckpointMetadata {
            version: "1.0".to_string(),
            epoch: 10,
            step: 1000,
            train_loss: 0.5,
            train_accuracy: 85.0,
            test_loss: 0.6,
            test_accuracy: 83.0,
            best_test_accuracy: 85.0,
            best_epoch: 8,
            learning_rate: 0.01,
            total_training_time_secs: 600.0,
            created_at: UNIX_EPOCH,
            model_config: ModelConfig {
                model_type: "ResNet18".to_string(),
                num_parameters: 11_700_000,
                num_classes: 10,
            },
            training_config: TrainingConfigSnapshot {
                dataset: "CIFAR-10".to_string(),
                batch_size: 128,
                base_learning_rate: 0.1,
                optimizer: "Adam".to_string(),
                num_towers: 1,
            },
        };
        
        let state = TrainingState::from_checkpoint(&metadata);
        assert_eq!(state.start_epoch, 11); // Resume from next epoch
        assert_eq!(state.best_test_accuracy, 85.0);
        assert_eq!(state.best_epoch, 8);
    }
}

