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

#![allow(dead_code)] // Functions are used via commands.rs

use super::*;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Display and logging operation implementations
impl BiomeExecutor {
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

            let created_str = biome.info.created.format("%Y-%m-%d %H:%M").to_string();
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

    #[allow(dead_code)]
    pub(super) async fn show_log_file(&self, log_file: &Path, lines: Option<usize>) -> Result<()> {
        let content = fs::read_to_string(log_file).await?;

        if let Some(n) = lines {
            // Show last N lines
            let lines_vec: Vec<&str> = content.lines().collect();
            let start_idx = lines_vec.len().saturating_sub(n);
            for line in &lines_vec[start_idx..] {
                println!("{}", line);
            }
        } else {
            // Show all lines
            print!("{}", content);
        }

        Ok(())
    }

    // Used by commands.rs for 'logs --follow' command
    pub(super) async fn tail_log_file(&self, log_file: &Path, initial_lines: usize) -> Result<()> {
        use tokio::time::{sleep, Duration};

        // Show initial lines
        self.show_log_file(log_file, Some(initial_lines)).await?;

        // Open file for tailing
        let file = fs::File::open(log_file).await?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - wait and retry
                    sleep(Duration::from_millis(100)).await;
                }
                Ok(_) => {
                    print!("{}", line);
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
