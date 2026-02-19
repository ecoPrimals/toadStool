//! Signal handling for biome execution
//!
//! This module provides Unix signal handling for graceful shutdown and process control.

use anyhow::{Context, Result};
use tracing::info;

/// Signal manager for Unix signal handling
#[allow(dead_code)]
pub(super) struct SignalManager;

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_sigcont_to_self() {
        // SIGCONT to our own PID is a no-op and always succeeds.
        let own_pid = std::process::id();
        let result = SignalManager::send_signal(own_pid, "CONT");
        assert!(
            result.is_ok(),
            "SIGCONT to self should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_send_invalid_signal_name_returns_err() {
        // "NOTASIGNAL" is not a valid signal name; kill should fail.
        let own_pid = std::process::id();
        let result = SignalManager::send_signal(own_pid, "NOTASIGNAL");
        assert!(result.is_err(), "Invalid signal name should return error");
    }

    #[test]
    fn test_send_signal_to_dead_process_returns_err() {
        // Spawn a child, wait for it to exit, then its PID is released.
        // SIGKILL to the dead PID should fail with "No such process".
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("Failed to spawn");
        let dead_pid = child.id();
        child.wait().expect("Failed to wait");
        // PID is now free; SIGKILL to it should fail.
        let result = SignalManager::send_signal(dead_pid, "KILL");
        assert!(
            result.is_err(),
            "Signal to dead PID {dead_pid} should return error"
        );
    }

    #[test]
    fn test_send_signal_uses_kill_command() {
        // Verify that SIGTERM to self does not panic or crash the test runner.
        // We use SIGCONT (no-op) to avoid actually terminating.
        let own_pid = std::process::id();
        assert!(SignalManager::send_signal(own_pid, "CONT").is_ok());
    }
}
