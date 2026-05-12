// SPDX-License-Identifier: AGPL-3.0-or-later
//! Display and Logging Operations
//!
//! This module contains all display and logging operations:
//! - `print_biomes_table()` - Pretty-print biomes in table format
//! - `show_log_file()` - Display log file contents
//! - `tail_log_file()` - Follow log file (tail -f behavior)
//!
//! **Deep Debt Principles**:
//! - ✅ Clean separation of concerns
//! - ✅ Modern async I/O
//! - ✅ User-friendly output

use super::*;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Display and logging operation implementations
impl BiomeExecutor {
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // CLI display; async for API consistency
    pub(super) async fn print_biomes_table(
        &self,
        biomes: &[&RunningBiome],
        show_resources: bool,
    ) -> Result<()> {
        if biomes.is_empty() {
            println!("No biomes found");
            return Ok(());
        }

        println!(
            "{:<20} {:<12} {:<10} {:<20} {:<10}",
            "NAME", "STATUS", "SERVICES", "CREATED", "ID"
        );
        println!("{}", "-".repeat(80));

        for biome in biomes {
            let status_str = match &biome.info.status {
                BiomeStatus::Running => "running",
                BiomeStatus::Starting => "starting",
                BiomeStatus::Stopping => "stopping",
                BiomeStatus::Stopped => "stopped",
                BiomeStatus::Error(_) => "error",
                BiomeStatus::Migrating => "migrating",
            };

            let created_str =
                toadstool_common::system_time_serde::format_display(biome.info.created);
            let id_short = biome.info.id.to_string()[..8].to_string();

            println!(
                "{:<20} {:<12} {:<10} {:<20} {:<10}",
                biome.info.name,
                status_str,
                biome.info.services.len(),
                created_str,
                id_short
            );

            if show_resources {
                println!(
                    "  └─ CPU: {:.1}% | Memory: {} MB | Storage: {} MB",
                    biome.info.resource_usage.cpu_percent,
                    biome.info.resource_usage.memory_bytes / (1024 * 1024),
                    biome.info.resource_usage.storage_bytes / (1024 * 1024),
                );
            }
        }

        Ok(())
    }

    pub(super) async fn show_log_file(&self, log_file: &Path, lines: Option<usize>) -> Result<()> {
        let content = fs::read_to_string(log_file).await?;

        if let Some(n) = lines {
            // Show last N lines
            let lines_vec: Vec<&str> = content.lines().collect();
            let start_idx = lines_vec.len().saturating_sub(n);
            for line in &lines_vec[start_idx..] {
                println!("{line}");
            }
        } else {
            // Show all lines
            print!("{content}");
        }

        Ok(())
    }

    // Used by commands.rs for 'logs --follow' command
    pub(super) async fn tail_log_file(&self, log_file: &Path, initial_lines: usize) -> Result<()> {
        // Show initial lines
        self.show_log_file(log_file, Some(initial_lines)).await?;

        // Open file for tailing
        let file = fs::File::open(log_file).await?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        // Poll at interval (proper async pattern for tail -f without inotify)
        const LOG_POLL_INTERVAL_MS: u64 = 100;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(LOG_POLL_INTERVAL_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - wait for next poll tick
                    interval.tick().await;
                }
                Ok(_) => {
                    print!("{line}");
                    line.clear();
                }
                Err(e) => {
                    warn!("Error reading log file: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::types::RunningBiome;
    use crate::{
        BiomeInfo, BiomeManifest, BiomeMetadata, BiomeNetworking, BiomeResources, BiomeSecurity,
        BiomeStatus, ResourceUsage, ServiceInfo,
    };
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    fn make_running_biome(
        name: &str,
        status: BiomeStatus,
        services_count: usize,
        cpu: f64,
        memory_mb: u64,
    ) -> RunningBiome {
        let now = std::time::SystemTime::now();
        let services: Vec<ServiceInfo> = (0..services_count)
            .map(|i| ServiceInfo {
                name: format!("svc-{i}"),
                status: "running".to_string(),
                replicas: 1,
                ports: vec![],
                health: "healthy".to_string(),
            })
            .collect();

        RunningBiome {
            info: BiomeInfo {
                id: Uuid::new_v4(),
                name: name.to_string(),
                status,
                created: now,
                started: Some(now),
                manifest_path: PathBuf::from("."),
                resource_usage: ResourceUsage {
                    cpu_percent: cpu,
                    memory_bytes: memory_mb * 1024 * 1024,
                    storage_bytes: 0,
                    network_rx_bytes: 0,
                    network_tx_bytes: 0,
                },
                services,
            },
            _manifest: BiomeManifest {
                metadata: BiomeMetadata {
                    name: name.to_string(),
                    version: "1.0".to_string(),
                    description: None,
                    author: None,
                    created: now,
                    updated: now,
                    tags: vec![],
                },
                primals: HashMap::new(),
                services: HashMap::new(),
                resources: BiomeResources {
                    cpu_limit: None,
                    memory_limit: None,
                    storage_limit: None,
                    gpu_limit: None,
                    network_bandwidth: None,
                },
                security: BiomeSecurity {
                    isolation_level: "standard".to_string(),
                    trust_level: "medium".to_string(),
                    security_required: false,
                    crypto_policies: vec![],
                    allowed_networks: vec![],
                    forbidden_syscalls: vec![],
                },
                networking: BiomeNetworking {
                    mode: "bridge".to_string(),
                    dns_servers: vec![],
                    port_mappings: vec![],
                    network_policies: vec![],
                },
                storage: crate::BiomeStorage {
                    storage_integration: None,
                    datasets: vec![],
                    volumes: vec![],
                    backup_policy: None,
                },
            },
            process_handles: vec![],
            log_files: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_print_biomes_table_empty() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let biomes: Vec<&RunningBiome> = vec![];
        let result = executor.print_biomes_table(&biomes, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_print_biomes_table_with_biomes() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let rb1 = make_running_biome("biome-a", BiomeStatus::Running, 2, 10.5, 256);
        let rb2 = make_running_biome("biome-b", BiomeStatus::Stopped, 1, 0.0, 128);
        let biomes: Vec<&RunningBiome> = vec![&rb1, &rb2];
        let result = executor.print_biomes_table(&biomes, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_print_biomes_table_with_resources() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let rb = make_running_biome("test", BiomeStatus::Running, 3, 25.0, 512);
        let biomes: Vec<&RunningBiome> = vec![&rb];
        let result = executor.print_biomes_table(&biomes, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_print_biomes_table_all_status_variants() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let statuses = [
            BiomeStatus::Running,
            BiomeStatus::Starting,
            BiomeStatus::Stopping,
            BiomeStatus::Stopped,
            BiomeStatus::Error("oops".to_string()),
            BiomeStatus::Migrating,
        ];
        let biomes: Vec<RunningBiome> = statuses
            .iter()
            .enumerate()
            .map(|(i, s)| make_running_biome(&format!("b{i}"), s.clone(), 0, 0.0, 0))
            .collect();
        let refs: Vec<&RunningBiome> = biomes.iter().collect();
        let result = executor.print_biomes_table(&refs, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_log_file_all_lines() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "line1").unwrap();
        writeln!(tmp, "line2").unwrap();
        tmp.flush().unwrap();

        let executor = BiomeExecutor::new().await.expect("executor");
        let result = executor.show_log_file(tmp.path(), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_log_file_last_n_lines() {
        let mut tmp = NamedTempFile::new().unwrap();
        for i in 1..=10 {
            writeln!(tmp, "line {i}").unwrap();
        }
        tmp.flush().unwrap();

        let executor = BiomeExecutor::new().await.expect("executor");
        let result = executor.show_log_file(tmp.path(), Some(3)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_log_file_nonexistent() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let result = executor
            .show_log_file(Path::new("/nonexistent/path/to/log"), None)
            .await;
        assert!(result.is_err());
    }
}
