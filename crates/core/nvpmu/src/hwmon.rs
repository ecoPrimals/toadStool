// SPDX-License-Identifier: AGPL-3.0-only
//! GPU sensor reading via Linux hwmon sysfs.
//!
//! Each hwmon device under `/sys/class/hwmon/hwmonN/` or a GPU's
//! `hwmon/hwmonN/` subdirectory exposes temperature, clock, and power
//! sensors as plain text files.

use crate::error::{NvPmuError, Result};
use std::path::{Path, PathBuf};

/// Snapshot of hwmon sensor readings for a single GPU.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HwmonSensors {
    /// Hwmon sysfs directory path.
    pub hwmon_path: PathBuf,
    /// GPU temperature in millidegrees Celsius (e.g. 45000 = 45°C).
    pub temp_mc: Option<i64>,
    /// Critical temperature threshold in millidegrees Celsius.
    pub temp_crit_mc: Option<i64>,
    /// Current GPU clock in `MHz` (from `freq1_input` or `clock` attribute).
    pub clock_mhz: Option<u64>,
    /// Current memory clock in `MHz`.
    pub mem_clock_mhz: Option<u64>,
    /// Current power draw in microwatts.
    pub power_uw: Option<u64>,
    /// Power limit in microwatts.
    pub power_limit_uw: Option<u64>,
    /// Fan speed in RPM (if fan sensor exists).
    pub fan_rpm: Option<u64>,
}

impl HwmonSensors {
    /// Read all available sensors from a GPU's sysfs device directory.
    ///
    /// Searches `{device_path}/hwmon/hwmonN/` for sensor files.
    ///
    /// # Errors
    /// Returns error if no hwmon directory is found.
    pub fn from_device(device_path: &Path) -> Result<Self> {
        let hwmon_dir = device_path.join("hwmon");
        let hwmon_path = find_first_hwmon(&hwmon_dir)?;
        Self::from_hwmon_path(&hwmon_path)
    }

    /// Read sensors from a specific hwmon directory.
    ///
    /// # Errors
    /// Returns error on I/O failure.
    pub fn from_hwmon_path(path: &Path) -> Result<Self> {
        Ok(Self {
            hwmon_path: path.to_path_buf(),
            temp_mc: read_sensor_i64(path, "temp1_input").ok(),
            temp_crit_mc: read_sensor_i64(path, "temp1_crit").ok(),
            clock_mhz: read_sensor_u64(path, "freq1_input")
                .map(|hz| hz / 1_000_000)
                .ok(),
            mem_clock_mhz: read_sensor_u64(path, "freq2_input")
                .map(|hz| hz / 1_000_000)
                .ok(),
            power_uw: read_sensor_u64(path, "power1_average").ok(),
            power_limit_uw: read_sensor_u64(path, "power1_cap").ok(),
            fan_rpm: read_sensor_u64(path, "fan1_input").ok(),
        })
    }

    /// Temperature in degrees Celsius, if available.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn temp_c(&self) -> Option<f64> {
        self.temp_mc.map(|mc| mc as f64 / 1000.0)
    }

    /// Power draw in watts, if available.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn power_w(&self) -> Option<f64> {
        self.power_uw.map(|uw| uw as f64 / 1_000_000.0)
    }
}

fn find_first_hwmon(hwmon_dir: &Path) -> Result<PathBuf> {
    if !hwmon_dir.exists() {
        return Err(NvPmuError::SensorNotFound(format!(
            "no hwmon directory at {}",
            hwmon_dir.display()
        )));
    }
    for entry in std::fs::read_dir(hwmon_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("hwmon") {
            return Ok(entry.path());
        }
    }
    Err(NvPmuError::SensorNotFound(format!(
        "no hwmonN subdirectory in {}",
        hwmon_dir.display()
    )))
}

fn read_sensor_i64(hwmon: &Path, name: &str) -> Result<i64> {
    let path = hwmon.join(name);
    let s = std::fs::read_to_string(&path)?.trim().to_string();
    s.parse::<i64>().map_err(|e| NvPmuError::Parse {
        path: path.display().to_string(),
        source: e,
    })
}

fn read_sensor_u64(hwmon: &Path, name: &str) -> Result<u64> {
    let path = hwmon.join(name);
    let s = std::fs::read_to_string(&path)?.trim().to_string();
    s.parse::<u64>().map_err(|e| NvPmuError::Parse {
        path: path.display().to_string(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_c_conversion() {
        let s = HwmonSensors {
            hwmon_path: PathBuf::from("/test"),
            temp_mc: Some(45000),
            temp_crit_mc: Some(105000),
            clock_mhz: None,
            mem_clock_mhz: None,
            power_uw: Some(120_000_000),
            power_limit_uw: Some(250_000_000),
            fan_rpm: None,
        };
        assert!((s.temp_c().unwrap() - 45.0).abs() < 0.01);
        assert!((s.power_w().unwrap() - 120.0).abs() < 0.01);
    }

    #[test]
    fn temp_c_none_when_missing() {
        let s = HwmonSensors {
            hwmon_path: PathBuf::from("/test"),
            temp_mc: None,
            temp_crit_mc: None,
            clock_mhz: None,
            mem_clock_mhz: None,
            power_uw: None,
            power_limit_uw: None,
            fan_rpm: None,
        };
        assert!(s.temp_c().is_none());
        assert!(s.power_w().is_none());
    }

    #[test]
    fn from_device_nonexistent() {
        let result = HwmonSensors::from_device(Path::new("/nonexistent/device"));
        assert!(result.is_err());
    }
}
