//! Resource management for biomes
//!
//! This module handles resource cleanup, PID tracking, and data management.

use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;
use tracing::info;
use uuid::Uuid;

use super::{BiomeExecutor, BiomeInfo};

/// Resource manager for biome data and processes
#[allow(dead_code)]
pub(super) struct ResourceManager<'a> {
    executor: &'a BiomeExecutor,
}

#[allow(dead_code)]
impl<'a> ResourceManager<'a> {
    /// Create new resource manager
    pub fn new(executor: &'a BiomeExecutor) -> Self {
        Self { executor }
    }

    /// Purge all data for a biome
    ///
    /// # Errors
    ///
    /// Returns an error if directory removal fails
    pub async fn purge_biome_data(&self, biome_name: &str) -> Result<()> {
        let data_dir = PathBuf::from(format!("/tmp/toadstool/data/{biome_name}"));
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));

        if data_dir.exists() {
            fs::remove_dir_all(&data_dir).await?;
            info!("🗑️  Removed data directory: {}", data_dir.display());
        }

        if log_dir.exists() {
            fs::remove_dir_all(&log_dir).await?;
            info!("🗑️  Removed log directory: {}", log_dir.display());
        }

        Ok(())
    }

    /// Get actual PID for a biome process
    ///
    /// # Errors
    ///
    /// Returns an error if PID cannot be found
    pub async fn get_actual_pid(&self, biome_name: &str) -> Result<u32> {
        let biomes = self.executor.biomes.read().await;

        if let Some(biome) = biomes.get(biome_name) {
            // Return the first valid PID
            for process in &biome.process_handles {
                if let Some(pid) = process.pid {
                    return Ok(pid);
                }
            }
            anyhow::bail!("No PIDs found for biome '{biome_name}'");
        }

        anyhow::bail!("Biome '{biome_name}' not found");
    }

    /// Find process by execution ID
    pub async fn find_process_pid(&self, execution_id: &Uuid) -> Option<u32> {
        let biomes = self.executor.biomes.read().await;

        for (_biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    return process.pid;
                }
            }
        }

        None
    }

    /// Check if biome exists
    pub async fn biome_exists(&self, biome_name: &str) -> bool {
        let biomes = self.executor.biomes.read().await;
        biomes.contains_key(biome_name)
    }

    /// Get biome info (convert from RunningBiome)
    pub async fn get_biome_info(&self, biome_name: &str) -> Option<BiomeInfo> {
        let biomes = self.executor.biomes.read().await;
        biomes.get(biome_name).map(|rb| rb.info.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_executor() -> BiomeExecutor {
        BiomeExecutor::new()
            .await
            .expect("BiomeExecutor should construct in test environment")
    }

    #[tokio::test]
    async fn test_biome_exists_on_empty_registry() {
        let exec = make_executor().await;
        let rm = ResourceManager::new(&exec);
        assert!(!rm.biome_exists("nonexistent").await);
    }

    #[tokio::test]
    async fn test_get_biome_info_missing_returns_none() {
        let exec = make_executor().await;
        let rm = ResourceManager::new(&exec);
        assert!(rm.get_biome_info("ghost").await.is_none());
    }

    #[tokio::test]
    async fn test_find_process_pid_empty_registry() {
        let exec = make_executor().await;
        let rm = ResourceManager::new(&exec);
        let uuid = uuid::Uuid::new_v4();
        assert!(rm.find_process_pid(&uuid).await.is_none());
    }

    #[tokio::test]
    async fn test_get_actual_pid_unknown_biome_returns_err() {
        let exec = make_executor().await;
        let rm = ResourceManager::new(&exec);
        let result = rm.get_actual_pid("not-a-biome").await;
        assert!(result.is_err(), "Should error for unknown biome");
    }

    #[tokio::test]
    async fn test_concurrent_resource_reads() {
        let exec = std::sync::Arc::new(make_executor().await);
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let exec = std::sync::Arc::clone(&exec);
                tokio::spawn(async move {
                    let rm = ResourceManager::new(&exec);
                    let name = format!("biome-{i}");
                    (
                        !rm.biome_exists(&name).await,
                        rm.get_biome_info(&name).await.is_none(),
                    )
                })
            })
            .collect();

        for h in handles {
            let (not_found, no_info) = h.await.unwrap();
            assert!(not_found);
            assert!(no_info);
        }
    }
}
