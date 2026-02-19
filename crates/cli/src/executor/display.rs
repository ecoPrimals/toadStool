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
use crate::BiomeStatus;

/// Display manager for UI and logging
#[allow(dead_code)]
pub(super) struct DisplayManager;

#[allow(dead_code)]
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
                BiomeStatus::Running => "running".green(),
                BiomeStatus::Starting => "starting".yellow(),
                BiomeStatus::Stopping => "stopping".yellow(),
                BiomeStatus::Stopped => "stopped".bright_black(),
                BiomeStatus::Error(_) => "error".red(),
                BiomeStatus::Migrating => "migrating".cyan(),
            };

            let started = info
                .started
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

        while let Some(line) = all_lines.next_line().await.context("Failed to read line")? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_log_path_format() {
        let path = DisplayManager::get_log_path("my-biome", "stdout");
        assert_eq!(
            path,
            PathBuf::from("/tmp/toadstool/logs/my-biome/stdout.log")
        );
    }

    #[test]
    fn test_get_log_path_separates_components() {
        let stdout = DisplayManager::get_log_path("biome", "stdout");
        let stderr = DisplayManager::get_log_path("biome", "stderr");
        assert_ne!(stdout, stderr);
        assert!(stdout.to_string_lossy().ends_with("stdout.log"));
        assert!(stderr.to_string_lossy().ends_with("stderr.log"));
    }

    #[test]
    fn test_get_log_path_special_chars() {
        let path = DisplayManager::get_log_path("my-biome-123", "out");
        assert!(path.to_string_lossy().contains("my-biome-123"));
    }

    #[tokio::test]
    async fn test_show_log_file_reads_contents() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "line one").unwrap();
        writeln!(tmp, "line two").unwrap();
        tmp.flush().unwrap();

        // show_log_file reads and prints; just verify it succeeds without error.
        let result = DisplayManager::show_log_file(tmp.path()).await;
        assert!(result.is_ok(), "show_log_file should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_show_log_file_nonexistent_returns_err() {
        let result = DisplayManager::show_log_file(Path::new("/nonexistent_log.log")).await;
        assert!(result.is_err(), "Missing log file should return error");
    }

    #[tokio::test]
    async fn test_tail_log_file_returns_last_n_lines() {
        let mut tmp = NamedTempFile::new().unwrap();
        for i in 1..=20 {
            writeln!(tmp, "line {i}").unwrap();
        }
        tmp.flush().unwrap();

        // tail 5 lines — just verify no error; output goes to stdout.
        let result = DisplayManager::tail_log_file(tmp.path(), 5).await;
        assert!(result.is_ok(), "tail_log_file should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_tail_log_file_empty_file() {
        let tmp = NamedTempFile::new().unwrap();
        let result = DisplayManager::tail_log_file(tmp.path(), 10).await;
        assert!(result.is_ok(), "Tailing empty file should succeed");
    }

    #[tokio::test]
    async fn test_tail_log_file_fewer_lines_than_requested() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "only line").unwrap();
        tmp.flush().unwrap();

        let result = DisplayManager::tail_log_file(tmp.path(), 100).await;
        assert!(
            result.is_ok(),
            "tail_log_file should handle oversized request"
        );
    }
}
