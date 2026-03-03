// SPDX-License-Identifier: AGPL-3.0-or-later
//! Power Measurement Infrastructure
//!
//! **Deep Debt**: Measure actual power, don't hardcode estimates!
//!
//! This module implements real power measurement via:
//! - Linux RAPL (Running Average Power Limit) for CPU
//! - nvidia-smi / rocm-smi for GPU
//! - BrainChip Akida API for NPU
//!
//! Falls back to estimates only when hardware APIs unavailable.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Power measurement result
#[derive(Clone, Debug)]
pub struct PowerMeasurement {
    /// Power in watts
    pub watts: f64,
    /// Whether this is measured (true) or estimated (false)
    pub is_measured: bool,
    /// Measurement method
    pub method: String,
}

/// CPU power monitor (Linux RAPL)
///
/// **Deep Debt**: Real measurement via /sys/class/powercap
pub struct CpuPowerMonitor {
    rapl_path: Option<PathBuf>,
}

impl CpuPowerMonitor {
    /// Create new CPU power monitor
    ///
    /// Attempts to detect RAPL interface on Linux
    pub fn new() -> Result<Self> {
        // Try to find RAPL interface
        let rapl_path = Self::find_rapl_interface();

        Ok(Self { rapl_path })
    }

    /// Find RAPL interface on Linux
    fn find_rapl_interface() -> Option<PathBuf> {
        // Look for intel-rapl or AMD equivalent
        let base = PathBuf::from("/sys/class/powercap");

        if !base.exists() {
            return None;
        }

        // Look for intel-rapl:0 (package energy)
        for entry in fs::read_dir(&base).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();

            if let Some(name) = path.file_name()?.to_str() {
                if name.starts_with("intel-rapl:") || name.starts_with("amd-rapl:") {
                    // Check if this is package-0 (CPU package)
                    let name_file = path.join("name");
                    if let Ok(contents) = fs::read_to_string(&name_file) {
                        if contents.trim() == "package-0" {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Measure actual CPU power consumption
    ///
    /// **Real Measurement**: Reads RAPL energy counters over time
    pub fn measure_watts(&self) -> Result<PowerMeasurement> {
        if let Some(ref rapl_path) = self.rapl_path {
            // Read energy counter (microjoules)
            let energy_uj_path = rapl_path.join("energy_uj");

            if energy_uj_path.exists() {
                // Try to read energy counter (may fail with permission denied)
                match self.read_energy_uj(&energy_uj_path) {
                    Ok(start_energy) => {
                        let start_time = Instant::now();

                        std::thread::sleep(Duration::from_millis(1000));

                        match self.read_energy_uj(&energy_uj_path) {
                            Ok(end_energy) => {
                                let elapsed = start_time.elapsed();

                                // Calculate power (energy / time)
                                let energy_j =
                                    (end_energy.wrapping_sub(start_energy)) as f64 / 1_000_000.0;
                                let watts = energy_j / elapsed.as_secs_f64();

                                return Ok(PowerMeasurement {
                                    watts,
                                    is_measured: true,
                                    method: "RAPL".to_string(),
                                });
                            }
                            Err(_) => {
                                // Permission denied - fall through to estimate
                            }
                        }
                    }
                    Err(_) => {
                        // Permission denied - fall through to estimate
                    }
                }
            }
        }

        // Fallback: Estimate based on typical CPU TDP
        Ok(PowerMeasurement {
            watts: 25.0,
            is_measured: false,
            method: "Estimate (no RAPL/permissions)".to_string(),
        })
    }

    /// Read energy counter in microjoules
    fn read_energy_uj(&self, path: &PathBuf) -> Result<u64> {
        let contents = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read RAPL (may need sudo): {e}"))?;
        let value = contents.trim().parse::<u64>()?;
        Ok(value)
    }
}

/// GPU power monitor (nvidia-smi / rocm-smi)
///
/// **Deep Debt**: Real measurement via GPU vendor tools
pub struct GpuPowerMonitor {
    vendor: GpuVendor,
}

#[derive(Clone, Debug)]
enum GpuVendor {
    Nvidia,
    Amd,
    Unknown,
}

impl GpuPowerMonitor {
    /// Create new GPU power monitor
    ///
    /// Detects GPU vendor at runtime
    pub fn new() -> Result<Self> {
        let vendor = Self::detect_vendor();
        Ok(Self { vendor })
    }

    /// Detect GPU vendor
    fn detect_vendor() -> GpuVendor {
        // Try nvidia-smi first
        if std::process::Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .is_ok()
        {
            return GpuVendor::Nvidia;
        }

        // Try rocm-smi
        if std::process::Command::new("rocm-smi")
            .arg("--version")
            .output()
            .is_ok()
        {
            return GpuVendor::Amd;
        }

        GpuVendor::Unknown
    }

    /// Measure actual GPU power consumption
    ///
    /// **Real Measurement**: Query GPU vendor APIs
    pub fn measure_watts(&self) -> Result<PowerMeasurement> {
        match self.vendor {
            GpuVendor::Nvidia => self.measure_nvidia(),
            GpuVendor::Amd => self.measure_amd(),
            GpuVendor::Unknown => Ok(PowerMeasurement {
                watts: 150.0,
                is_measured: false,
                method: "Estimate (no GPU tools)".to_string(),
            }),
        }
    }

    /// Measure NVIDIA GPU power via nvidia-smi
    fn measure_nvidia(&self) -> Result<PowerMeasurement> {
        // nvidia-smi --query-gpu=power.draw --format=csv,noheader,nounits
        let output = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=power.draw", "--format=csv,noheader,nounits"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("nvidia-smi failed"));
        }

        let watts_str = String::from_utf8(output.stdout)?;
        let watts = watts_str.trim().parse::<f64>()?;

        Ok(PowerMeasurement {
            watts,
            is_measured: true,
            method: "nvidia-smi".to_string(),
        })
    }

    /// Measure AMD GPU power via rocm-smi
    fn measure_amd(&self) -> Result<PowerMeasurement> {
        // rocm-smi --showpower
        let output = std::process::Command::new("rocm-smi")
            .arg("--showpower")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("rocm-smi failed"));
        }

        // Parse output (format: "Average Graphics Package Power: 123.45 W")
        let output_str = String::from_utf8(output.stdout)?;

        for line in output_str.lines() {
            if line.contains("Average Graphics Package Power:") {
                if let Some(watts_str) = line.split(':').nth(1) {
                    let watts_str = watts_str.trim().replace("W", "");
                    if let Ok(watts) = watts_str.parse::<f64>() {
                        return Ok(PowerMeasurement {
                            watts,
                            is_measured: true,
                            method: "rocm-smi".to_string(),
                        });
                    }
                }
            }
        }

        Err(anyhow!("Failed to parse rocm-smi output"))
    }
}

/// NPU power monitor (Akida API)
///
/// **Deep Debt**: Real measurement via BrainChip Akida API
pub struct NpuPowerMonitor {
    // Removed unused field - detection happens in measure_watts()
}

impl NpuPowerMonitor {
    /// Create new NPU power monitor
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Measure actual NPU power consumption via Linux hwmon sysfs
    ///
    /// Queries /sys/bus/pci/devices/{PCIe_ADDRESS}/hwmon/hwmonX/power1_input
    /// Falls back to typical estimate if hwmon data unavailable
    pub fn measure_watts(&self) -> Result<PowerMeasurement> {
        // Try known Akida PCIe addresses
        for pcie_addr in &["0000:a1:00.0", "0000:e2:00.0"] {
            let hwmon_dir = format!("/sys/bus/pci/devices/{pcie_addr}/hwmon");
            if let Ok(entries) = std::fs::read_dir(&hwmon_dir) {
                for entry in entries.flatten() {
                    let power_path = entry.path().join("power1_input");
                    if let Ok(power_str) = std::fs::read_to_string(&power_path) {
                        if let Ok(power_uw) = power_str.trim().parse::<f64>() {
                            let watts = power_uw / 1_000_000.0;
                            return Ok(PowerMeasurement {
                                watts,
                                is_measured: true,
                                method: format!("hwmon sysfs ({pcie_addr})"),
                            });
                        }
                    }
                }
            }
        }

        // Fallback: typical estimate with explicit warning
        tracing::warn!("NPU power: using typical estimate (hwmon unavailable)");
        Ok(PowerMeasurement {
            watts: 2.0,
            is_measured: false,
            method: "Estimate (hwmon unavailable)".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_power_monitor_creation() {
        let monitor = CpuPowerMonitor::new().unwrap();

        // Should create successfully even without RAPL
        if monitor.rapl_path.is_some() {
            println!("RAPL available at: {:?}", monitor.rapl_path);
        } else {
            println!("RAPL not available (will use estimates)");
        }
    }

    #[test]
    fn test_cpu_power_measurement() {
        let monitor = CpuPowerMonitor::new().unwrap();
        let measurement = monitor.measure_watts().unwrap();

        println!(
            "CPU Power: {:.2}W ({})",
            measurement.watts, measurement.method
        );

        if measurement.is_measured {
            println!("✅ Real RAPL measurement!");
        } else {
            println!("⚠️  Using estimate (RAPL requires root/permissions)");
        }

        // Should return reasonable value
        assert!(measurement.watts > 0.0);
        assert!(measurement.watts < 500.0);
    }

    #[test]
    fn test_gpu_power_monitor_creation() {
        let monitor = GpuPowerMonitor::new().unwrap();

        println!("GPU Vendor: {:?}", monitor.vendor);
    }

    #[test]
    fn test_gpu_power_measurement() {
        let monitor = GpuPowerMonitor::new().unwrap();
        let measurement = monitor.measure_watts().unwrap();

        println!(
            "GPU Power: {:.2}W ({})",
            measurement.watts, measurement.method
        );

        // Should return reasonable value
        assert!(measurement.watts > 0.0);
        assert!(measurement.watts < 1000.0);
    }
}
