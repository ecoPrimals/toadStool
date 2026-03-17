// SPDX-License-Identifier: AGPL-3.0-only
//! Thermal watchdog: continuous monitoring with emergency shutdown.
//!
//! Phase 4 of the nvPmu plan. Polls hwmon sensors at a configurable
//! interval and triggers a callback when thresholds are exceeded.
//! On critical thermal events, the GPU's power state can be lowered
//! via sysfs (if supported by the driver).

use crate::error::Result;
use crate::hwmon::HwmonSensors;
use crate::monitor::{MonitorConfig, SafetyStatus, evaluate_safety};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Watchdog handle. Drop to stop the monitoring thread.
pub struct Watchdog {
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Watchdog {
    /// Start a thermal watchdog for a GPU device.
    ///
    /// Polls the hwmon sensors at `config.poll_interval_ms` and calls the
    /// `on_event` callback when status changes or critical thresholds are hit.
    ///
    /// # Errors
    /// Returns error if initial sensor read fails.
    pub fn start<F>(device_path: &Path, config: MonitorConfig, on_event: F) -> Result<Self>
    where
        F: Fn(SafetyStatus, &HwmonSensors) + Send + 'static,
    {
        // Validate we can read sensors before spawning
        let _ = HwmonSensors::from_device(device_path)?;

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let path = device_path.to_path_buf();

        let handle = std::thread::Builder::new()
            .name("nvpmu-watchdog".into())
            .spawn(move || {
                let mut last_status = SafetyStatus::Unknown;

                while running_clone.load(Ordering::Relaxed) {
                    match HwmonSensors::from_device(&path) {
                        Ok(sensors) => {
                            let status = evaluate_safety(&sensors, &config);
                            if status != last_status || status == SafetyStatus::ThermalCritical {
                                on_event(status, &sensors);
                                last_status = status;
                            }

                            if status == SafetyStatus::ThermalCritical {
                                tracing::error!(
                                    "THERMAL CRITICAL — attempting emergency power reduction"
                                );
                                if let Err(e) = attempt_power_reduction(&path) {
                                    tracing::error!("power reduction failed: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("watchdog sensor read failed: {e}");
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(config.poll_interval_ms));
                }

                tracing::info!("nvpmu watchdog stopped");
            })
            .map_err(std::io::Error::other)?;

        Ok(Self {
            running,
            handle: Some(handle),
        })
    }

    /// Signal the watchdog to stop. Does not block.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Whether the watchdog is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Attempt to reduce GPU power via sysfs `power_cap`.
///
/// Writes a reduced power limit to `power1_cap` if the hwmon interface
/// supports it. This is a best-effort emergency measure.
fn attempt_power_reduction(device_path: &Path) -> Result<()> {
    let hwmon_dir = device_path.join("hwmon");
    if !hwmon_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&hwmon_dir)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with("hwmon") {
            continue;
        }
        let cap_path = entry.path().join("power1_cap");
        if cap_path.exists() {
            // Read current cap, write 50% as emergency reduction
            if let Ok(current) = std::fs::read_to_string(&cap_path)
                && let Ok(current_uw) = current.trim().parse::<u64>()
            {
                let reduced = current_uw / 2;
                tracing::warn!(
                    current_w = current_uw / 1_000_000,
                    reduced_w = reduced / 1_000_000,
                    "emergency power cap reduction"
                );
                // Best-effort write — may fail if driver doesn't support it
                let _ = std::fs::write(&cap_path, reduced.to_string());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_start_nonexistent_device() {
        let result = Watchdog::start(
            Path::new("/nonexistent/device"),
            MonitorConfig::default(),
            |_, _| {},
        );
        assert!(result.is_err());
    }
}
