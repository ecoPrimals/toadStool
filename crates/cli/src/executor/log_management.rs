//! Log management utilities for biome logs
//!
//! This module handles log file display, tailing, and filtering.

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Display log file with filtering options
pub async fn show_log_file(
    log_file: &PathBuf,
    lines: usize,
    timestamps: bool,
    level_filter: Option<String>,
    grep_pattern: Option<String>,
) -> Result<()> {
    use tokio::fs;
    use tokio::io::{AsyncBufReadExt, BufReader};

    info!("📄 Reading log file: {}", log_file.display());

    if !log_file.exists() {
        println!("Log file not found: {}", log_file.display());
        return Ok(());
    }

    let file = fs::File::open(log_file).await?;
    let reader = BufReader::new(file);
    let mut all_lines = Vec::new();

    let mut lines_stream = reader.lines();
    while let Some(line) = lines_stream.next_line().await? {
        all_lines.push(line);
    }

    let start_idx = if all_lines.len() > lines {
        all_lines.len() - lines
    } else {
        0
    };

    for line in &all_lines[start_idx..] {
        // Apply filters if specified
        if let Some(pattern) = &grep_pattern {
            if !line.contains(pattern) {
                continue;
            }
        }

        if let Some(level) = &level_filter {
            if !line.to_lowercase().contains(&level.to_lowercase()) {
                continue;
            }
        }

        if timestamps {
            println!("{line}");
        } else {
            // Basic timestamp stripping (remove first timestamp-like pattern)
            let cleaned_line = if line.len() > 20 && line.chars().nth(19) == Some(' ') {
                &line[20..]
            } else {
                line
            };
            println!("{cleaned_line}");
        }
    }

    Ok(())
}

/// Tail log file (follow mode) with filtering
pub async fn tail_log_file(
    log_file: &PathBuf,
    initial_lines: usize,
    timestamps: bool,
    level_filter: Option<String>,
    grep_pattern: Option<String>,
) -> Result<()> {
    use std::io::SeekFrom;
    use tokio::fs;
    use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
    use tokio::time::Duration;

    info!("👁️  Tailing log file: {}", log_file.display());

    if !log_file.exists() {
        println!("Log file not found: {}", log_file.display());
        return Ok(());
    }

    // Show initial lines
    show_log_file(
        log_file,
        initial_lines,
        timestamps,
        level_filter.clone(),
        grep_pattern.clone(),
    )
    .await?;

    // Start tailing
    let mut file = fs::File::open(log_file).await?;
    file.seek(SeekFrom::End(0)).await?;

    println!("--- Following log file (Ctrl+C to stop) ---");

    // ✅ MODERN ASYNC: Use tokio::time::interval instead of sleep loop
    // This provides consistent timing and better async scheduling
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let reader = BufReader::new(&mut file);
        let mut lines_stream = reader.lines();

        while let Some(line) = lines_stream.next_line().await? {
            // Apply filters
            if let Some(pattern) = &grep_pattern {
                if !line.contains(pattern) {
                    continue;
                }
            }

            if let Some(level) = &level_filter {
                if !line.to_lowercase().contains(&level.to_lowercase()) {
                    continue;
                }
            }

            if timestamps {
                println!("{line}");
            } else {
                let cleaned_line = if line.len() > 20 && line.chars().nth(19) == Some(' ') {
                    &line[20..]
                } else {
                    &line
                };
                println!("{cleaned_line}");
            }
        }

        // Wait for next interval tick (more efficient than sleep)
        interval.tick().await;
    }
}
