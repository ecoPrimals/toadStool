// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stop operations: biome shutdown, process termination, data purge

use std::time::Duration;

use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use super::super::BiomeExecutor;
use super::super::resources::ResourceManager;
use super::super::signals::SignalManager;

impl BiomeExecutor {
    pub(in crate::executor) async fn stop_biome_internal(
        &self,
        biome_name: &str,
        force: bool,
        timeout_secs: u64,
    ) -> crate::Result<()> {
        let running_biome = {
            let mut biomes = self.biomes.write().unwrap_or_else(|e| e.into_inner());
            biomes
                .remove(biome_name)
                .ok_or_else(|| crate::CliError::Other(format!("Biome '{biome_name}' not found")))?
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
                self.force_kill_process(&process.execution_id).await?;
            } else {
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

    async fn graceful_stop_process(&self, execution_id: &Uuid) -> crate::Result<()> {
        if let Some(pid) = ResourceManager::new(self)
            .find_process_pid(execution_id)
            .await
        {
            info!(
                "Gracefully stopping process {} (PID: {})",
                execution_id, pid
            );
            return self.send_signal_to_process(pid, "TERM");
        }
        warn!("Process {} not found for graceful stop", execution_id);
        Ok(())
    }

    async fn force_kill_process(&self, execution_id: &Uuid) -> crate::Result<()> {
        if let Some(pid) = ResourceManager::new(self)
            .find_process_pid(execution_id)
            .await
        {
            info!("Force killing process {} (PID: {})", execution_id, pid);
            return self.send_signal_to_process(pid, "KILL");
        }
        warn!("Process {} not found for force kill", execution_id);
        Ok(())
    }

    pub(in crate::executor) async fn purge_biome_data(
        &self,
        biome_name: &str,
    ) -> crate::Result<()> {
        ResourceManager::new(self)
            .purge_biome_data(biome_name)
            .await
    }

    pub(in crate::executor) async fn wait_for_interruption(&self) -> crate::Result<()> {
        SignalManager::wait_for_interrupt().await
    }

    fn send_signal_to_process(&self, pid: u32, signal: &str) -> crate::Result<()> {
        info!("Sending {} signal to PID {}", signal, pid);
        SignalManager::send_signal(pid, signal)
    }
}
