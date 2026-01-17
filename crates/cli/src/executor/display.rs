//! Display and UI utilities for biome management
//!
//! This module provides pretty-printing and log display functionality.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::BiomeInfo;

/// Display manager for UI and logging
pub(super) struct DisplayManager;

impl DisplayManager {
    /// Print biomes in a formatted table
    ///
    /// # Errors
    ///
    /// Returns an error if table formatting fails
    pub async fn print_biomes_table(biomes: &HashMap<String, BiomeInfo>) -> Result<()> {
        if biomes.is_empty() {
            println!("\n{}", "No biomes currently running".yellow());
            return Ok(());
        }

        println!("\n{}", "🍄 Running Biomes".bright_green().bold());
        println!("{}", "─".repeat(80).bright_black());

        // Header
        println!(
            "{:<20} {:<12} {:<15} {:<10}",
            "NAME".bold(),
            "STATUS".bold(),
            "STARTED".bold(),
            "SERVICES".bold()
        );
        println!("{}", "─".repeat(80).bright_black());

        // Sort biomes by name for consistent output
        let mut sorted: Vec<_> = biomes.iter().collect();
        sorted.sort_by_key(|(name, _)| *name);

        for (name, info) in sorted {
            let status_str = match &info.status {
                crate::BiomeStatus::Running => "running".green(),
                crate::BiomeStatus::Starting => "starting".yellow(),
                crate::BiomeStatus::Stopping => "stopping".yellow(),
                crate::BiomeStatus::Stopped => "stopped".bright_black(),
                crate::BiomeStatus::Error(_) => "error".red(),
                crate::BiomeStatus::Migrating => "migrating".cyan(),
            };

            let started = info.started
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "not started".to_string());

            println!(
                "{:<20} {:<12} {:<15} {:<10}",
                name.bright_white(),
                status_str,
                started,
                info.services.len().to_string().yellow()
            );
        }

        println!();
        Ok(())
    }

    /// Display entire log file
    ///
    /// # Errors
    ///
    /// Returns an error if log file cannot be read
    pub async fn show_log_file(log_path: &Path) -> Result<()> {
        let content = fs::read_to_string(log_path)
            .await
            .context("Failed to read log file")?;
        println!("{content}");
        Ok(())
    }

    /// Tail log file (last N lines)
    ///
    /// # Errors
    ///
    /// Returns an error if log file cannot be read
    pub async fn tail_log_file(log_path: &Path, lines: usize) -> Result<()> {
        let file = fs::File::open(log_path)
            .await
            .context("Failed to open log file")?;
        let reader = BufReader::new(file);
        let mut all_lines = reader.lines();
        let mut buffer = Vec::new();

        while let Some(line) = all_lines
            .next_line()
            .await
            .context("Failed to read line")?
        {
            buffer.push(line);
            if buffer.len() > lines {
                buffer.remove(0);
            }
        }

        for line in buffer {
            println!("{line}");
        }

        Ok(())
    }

    /// Get log file path for a biome
    pub fn get_log_path(biome_name: &str, component: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}/{component}.log"))
    }
}
