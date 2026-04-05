// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sensor reading via `nvidia-smi` for proprietary NVIDIA drivers.
//!
//! The proprietary NVIDIA driver does not expose hwmon sensors via sysfs.
//! Discovered on RTX 3090 with driver 580.119.02 (March 2026). Only nouveau
//! provides hwmon for NVIDIA GPUs. This module parses nvidia-smi CSV output
//! as a fallback sensor source.

use crate::error::{NvPmuError, Result};
use crate::hwmon::HwmonSensors;
use std::path::PathBuf;
use std::process::Command;

/// Read NVIDIA GPU sensors via nvidia-smi CLI.
///
/// Queries temperature, power, clocks, fan speed, and memory usage.
///
/// # Errors
/// Returns error if nvidia-smi is not available or fails.
pub fn read_sensors_via_smi(gpu_index: u32) -> Result<NvidiaSmiSensors> {
    let output = Command::new("nvidia-smi")
        .args([
            &format!("--id={gpu_index}"),
            "--query-gpu=name,temperature.gpu,power.draw,power.limit,clocks.gr,clocks.mem,fan.speed,memory.used,memory.total,pci.bus_id,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| NvPmuError::SensorNotFound(format!("nvidia-smi not found: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NvPmuError::SensorNotFound(format!(
            "nvidia-smi failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_smi_csv(stdout.trim())
}

/// Discover all NVIDIA GPUs via nvidia-smi.
///
/// # Errors
/// Returns error if nvidia-smi is not available.
pub fn discover_via_smi() -> Result<Vec<NvidiaSmiSensors>> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,temperature.gpu,power.draw,power.limit,clocks.gr,clocks.mem,fan.speed,memory.used,memory.total,pci.bus_id,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| NvPmuError::SensorNotFound(format!("nvidia-smi not found: {e}")))?;

    if !output.status.success() {
        return Err(NvPmuError::SensorNotFound("nvidia-smi failed".into()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && let Ok(sensors) = parse_smi_csv(trimmed)
        {
            gpus.push(sensors);
        }
    }
    Ok(gpus)
}

/// Sensor readings from nvidia-smi.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NvidiaSmiSensors {
    /// GPU product name (e.g. "NVIDIA `GeForce` RTX 3090").
    pub name: String,
    /// PCI BDF address (e.g. "00000000:41:00.0").
    pub bdf: String,
    /// NVIDIA driver version string.
    pub driver_version: String,
    /// GPU temperature in degrees Celsius.
    pub temp_c: Option<f64>,
    /// Current power draw in watts.
    pub power_w: Option<f64>,
    /// Power limit in watts.
    pub power_limit_w: Option<f64>,
    /// Graphics clock in `MHz`.
    pub clock_mhz: Option<u64>,
    /// Memory clock in `MHz`.
    pub mem_clock_mhz: Option<u64>,
    /// Fan speed as percentage (0–100).
    pub fan_pct: Option<u64>,
    /// VRAM used in `MiB`.
    pub mem_used_mib: Option<u64>,
    /// Total VRAM in `MiB`.
    pub mem_total_mib: Option<u64>,
}

impl NvidiaSmiSensors {
    /// Convert to the generic `HwmonSensors` format for unified monitoring.
    #[must_use]
    pub fn to_hwmon_sensors(&self) -> HwmonSensors {
        HwmonSensors {
            hwmon_path: PathBuf::from("/dev/null"),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "millidegrees from f64 celsius fits i64"
            )]
            temp_mc: self.temp_c.map(|t| (t * 1000.0) as i64),
            temp_crit_mc: None,
            clock_mhz: self.clock_mhz,
            mem_clock_mhz: self.mem_clock_mhz,
            power_uw: self.power_w.map(|w| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "microwatts from watts fits u64; power is always positive"
                )]
                {
                    (w * 1_000_000.0) as u64
                }
            }),
            power_limit_uw: self.power_limit_w.map(|w| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "microwatts from watts fits u64; power is always positive"
                )]
                {
                    (w * 1_000_000.0) as u64
                }
            }),
            fan_rpm: None,
        }
    }
}

fn parse_smi_csv(line: &str) -> Result<NvidiaSmiSensors> {
    let fields: Vec<&str> = line.split(", ").collect();
    if fields.len() < 11 {
        return Err(NvPmuError::SensorNotFound(format!(
            "nvidia-smi CSV: expected 11 fields, got {}",
            fields.len()
        )));
    }

    Ok(NvidiaSmiSensors {
        name: fields[0].trim().to_string(),
        temp_c: parse_opt_f64(fields[1]),
        power_w: parse_opt_f64(fields[2]),
        power_limit_w: parse_opt_f64(fields[3]),
        clock_mhz: parse_opt_u64(fields[4]),
        mem_clock_mhz: parse_opt_u64(fields[5]),
        fan_pct: parse_opt_u64(fields[6]),
        mem_used_mib: parse_opt_u64(fields[7]),
        mem_total_mib: parse_opt_u64(fields[8]),
        bdf: fields[9].trim().to_string(),
        driver_version: fields[10].trim().to_string(),
    })
}

fn parse_opt_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "[N/A]" || s == "N/A" {
        return None;
    }
    s.parse().ok()
}

fn parse_opt_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s == "[N/A]" || s == "N/A" {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_smi_output() {
        let line = "NVIDIA GeForce RTX 3090, 62, 140.19, 420.00, 1800, 9501, 55, 2312, 24576, 00000000:41:00.0, 580.119.02";
        let sensors = parse_smi_csv(line).unwrap();
        assert_eq!(sensors.name, "NVIDIA GeForce RTX 3090");
        assert!((sensors.temp_c.unwrap() - 62.0).abs() < 0.1);
        assert!((sensors.power_w.unwrap() - 140.19).abs() < 0.1);
        assert!((sensors.power_limit_w.unwrap() - 420.0).abs() < 0.1);
        assert_eq!(sensors.clock_mhz, Some(1800));
        assert_eq!(sensors.mem_clock_mhz, Some(9501));
        assert_eq!(sensors.fan_pct, Some(55));
        assert_eq!(sensors.mem_used_mib, Some(2312));
        assert_eq!(sensors.mem_total_mib, Some(24576));
        assert_eq!(sensors.bdf, "00000000:41:00.0");
        assert_eq!(sensors.driver_version, "580.119.02");
    }

    #[test]
    fn to_hwmon_sensors_conversion() {
        let smi = NvidiaSmiSensors {
            name: "RTX 3090".into(),
            bdf: "41:00.0".into(),
            driver_version: "580".into(),
            temp_c: Some(62.0),
            power_w: Some(140.0),
            power_limit_w: Some(420.0),
            clock_mhz: Some(1800),
            mem_clock_mhz: Some(9501),
            fan_pct: Some(55),
            mem_used_mib: Some(2312),
            mem_total_mib: Some(24576),
        };
        let hwmon = smi.to_hwmon_sensors();
        assert_eq!(hwmon.temp_mc, Some(62000));
        assert_eq!(hwmon.power_uw, Some(140_000_000));
        assert_eq!(hwmon.clock_mhz, Some(1800));
    }

    #[test]
    fn parse_na_fields() {
        let line = "GPU, [N/A], 100.00, 300.00, 1200, 5000, N/A, 1000, 8192, 00:00.0, 500.00";
        let sensors = parse_smi_csv(line).unwrap();
        assert!(sensors.temp_c.is_none());
        assert!(sensors.fan_pct.is_none());
        assert!(sensors.power_w.is_some());
    }
}
