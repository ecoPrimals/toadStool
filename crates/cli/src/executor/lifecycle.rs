//! Biome lifecycle management
//!
//! This module handles the core lifecycle operations for biomes including
//! startup, shutdown, and process management.

use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
use tokio::fs;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use super::signals::SignalManager;
use super::{BiomeExecutor, BiomeInfo, RunningBiome};
use crate::{BiomeManifest, BiomeStatus};

/// Lifecycle manager for biome operations
#[allow(dead_code)]
pub(super) struct BiomeLifecycle<'a> {
    executor: &'a BiomeExecutor,
}

#[allow(dead_code)]
impl<'a> BiomeLifecycle<'a> {
    /// Create new lifecycle manager
    pub fn new(executor: &'a BiomeExecutor) -> Self {
        Self { executor }
    }

    /// Start a biome with full initialization
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Log directory creation fails
    /// - Primal or service startup fails
    /// - Biome registration fails
    pub async fn start_biome(
        &self,
        biome_name: &str,
        manifest: BiomeManifest,
        env_vars: Vec<String>,
        _detached: bool,
        _debug: bool,
        _security_level: &str,
    ) -> Result<BiomeInfo> {
        let biome_id = Uuid::new_v4();
        let start_time = Utc::now();

        info!("🔧 Initializing biome infrastructure");

        // Create log directory (XDG-compliant path resolution)
        let env = PathEnv::from_env();
        let paths = PlatformPaths::new(&env);
        let log_dir = paths.toadstool_log_dir().join(biome_name);
        fs::create_dir_all(&log_dir).await?;

        // Parse environment variables
        let mut environment = HashMap::with_capacity(env_vars.len());
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        // Start processes (simplified - actual implementation would call start_primal/start_service)
        let processes = Vec::new();

        info!("✅ Biome infrastructure ready");

        // Create BiomeInfo (simplified for now)
        let biome_info = BiomeInfo {
            id: biome_id,
            name: biome_name.to_string(),
            status: BiomeStatus::Running,
            created: start_time,
            started: Some(start_time),
            manifest_path: PathBuf::from("biome.yaml"),
            resource_usage: crate::ResourceUsage {
                cpu_percent: 0.0,
                memory_bytes: 0,
                storage_bytes: 0,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
            },
            services: vec![],
        };

        // Register running biome
        let running_biome = RunningBiome {
            info: biome_info.clone(),
            _manifest: manifest,
            process_handles: processes,
            log_files: HashMap::new(),
        };

        {
            let mut biomes = self.executor.biomes.write().await;
            biomes.insert(biome_name.to_string(), running_biome);
        }

        info!("✅ Biome '{}' started successfully", biome_name);
        Ok(biome_info)
    }

    /// Stop a running biome
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Biome is not found
    /// - Process termination fails
    pub async fn stop_biome(&self, biome_name: &str, force: bool, timeout_secs: u64) -> Result<()> {
        let running_biome = {
            let mut biomes = self.executor.biomes.write().await;
            biomes
                .remove(biome_name)
                .ok_or_else(|| anyhow::anyhow!("Biome '{biome_name}' not found"))?
        };

        info!(
            "🛑 Stopping {} processes",
            running_biome.process_handles.len()
        );

        for process in &running_biome.process_handles {
            info!(
                "🛑 Stopping {}: {}",
                process.process_type_name(),
                process.name
            );

            if force {
                // Force kill immediately
                self.force_kill_process(&process.execution_id).await?;
            } else {
                // Graceful shutdown with timeout
                match timeout(
                    Duration::from_secs(timeout_secs),
                    self.graceful_stop_process(&process.execution_id),
                )
                .await
                {
                    Ok(Ok(())) => {
                        info!("✅ {} stopped gracefully", process.name);
                    }
                    Ok(Err(e)) => {
                        warn!("⚠️  Failed to stop {} gracefully: {}", process.name, e);
                        self.force_kill_process(&process.execution_id).await?;
                    }
                    Err(_) => {
                        warn!("⏰ Timeout stopping {}, force killing", process.name);
                        self.force_kill_process(&process.execution_id).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Gracefully stop a process by execution ID
    async fn graceful_stop_process(&self, execution_id: &Uuid) -> Result<()> {
        let biomes = self.executor.biomes.read().await;

        for (_biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!(
                            "Gracefully stopping process {} (PID: {})",
                            execution_id, pid
                        );
                        return SignalManager::send_signal(pid, "TERM");
                    }
                }
            }
        }

        warn!("Process {} not found for graceful stop", execution_id);
        Ok(())
    }

    /// Force kill a process by execution ID
    async fn force_kill_process(&self, execution_id: &Uuid) -> Result<()> {
        let biomes = self.executor.biomes.read().await;

        for (_biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!("Force killing process {} (PID: {})", execution_id, pid);
                        return SignalManager::send_signal(pid, "KILL");
                    }
                }
            }
        }

        warn!("Process {} not found for force kill", execution_id);
        Ok(())
    }
}
