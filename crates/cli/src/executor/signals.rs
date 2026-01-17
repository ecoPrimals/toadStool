//! Signal handling for biome execution
//!
//! This module provides Unix signal handling for graceful shutdown and process control.

use anyhow::{Context, Result};
use tracing::info;

/// Signal manager for Unix signal handling
pub(super) struct SignalManager;

impl SignalManager {
    /// Wait for termination signal (SIGTERM or SIGINT)
    ///
    /// # Errors
    ///
    /// Returns an error if signal handler registration fails
    pub async fn wait_for_interrupt() -> Result<()> {
        use tokio::signal;

        #[cfg(unix)]
        {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .context("Failed to register SIGTERM handler")?;
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .context("Failed to register SIGINT handler")?;

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("📡 Received SIGTERM");
                },
                _ = sigint.recv() => {
                    info!("📡 Received SIGINT");
                },
            }
        }

        #[cfg(not(unix))]
        {
            signal::ctrl_c()
                .await
                .context("Failed to wait for Ctrl+C")?;
            info!("📡 Received Ctrl+C");
        }

        Ok(())
    }

    /// Send Unix signal to process
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Signal command execution fails
    /// - Process does not exist
    pub fn send_signal(pid: u32, signal: &str) -> Result<()> {
        use std::process::Command;

        let output = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid.to_string())
            .output()
            .context("Failed to send signal")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to send signal {signal} to PID {pid}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}
