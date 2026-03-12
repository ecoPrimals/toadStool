// SPDX-License-Identifier: AGPL-3.0-only
//! Continuous GPU monitoring with safety thresholds.
//!
//! Polls hwmon sensors at a configurable interval and reports safety
//! status. Phase 0 is read-only — it cannot throttle or shut down
//! the GPU, only report that thresholds have been exceeded.

use crate::error::{NvPmuError, Result};
use crate::hwmon::HwmonSensors;
use std::path::Path;

/// Configuration for continuous monitoring.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorConfig {
    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Temperature warning threshold in millidegrees Celsius (default: 85°C).
    pub temp_warn_mc: i64,
    /// Temperature critical threshold in millidegrees Celsius (default: 95°C).
    pub temp_crit_mc: i64,
    /// Power warning threshold in microwatts (default: 300W).
    pub power_warn_uw: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            temp_warn_mc: 85_000,
            temp_crit_mc: 95_000,
            power_warn_uw: 300_000_000,
        }
    }
}

/// Safety status from a single monitoring sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SafetyStatus {
    /// All sensors within safe limits.
    Normal,
    /// Temperature above warning threshold but below critical.
    ThermalWarning,
    /// Temperature above critical threshold — compute should stop.
    ThermalCritical,
    /// Power draw above warning threshold.
    PowerWarning,
    /// No sensors available for this GPU.
    Unknown,
}

impl SafetyStatus {
    /// Whether it is safe to dispatch compute workloads.
    #[must_use]
    pub const fn compute_safe(self) -> bool {
        matches!(self, Self::Normal | Self::ThermalWarning | Self::PowerWarning)
    }
}

/// Single monitoring sample with timestamp.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorSample {
    pub sensors: HwmonSensors,
    pub status: SafetyStatus,
}

/// Evaluate safety status for a sensor reading against the config.
#[must_use]
pub fn evaluate_safety(sensors: &HwmonSensors, config: &MonitorConfig) -> SafetyStatus {
    if let Some(temp) = sensors.temp_mc {
        if temp >= config.temp_crit_mc {
            return SafetyStatus::ThermalCritical;
        }
        if temp >= config.temp_warn_mc {
            return SafetyStatus::ThermalWarning;
        }
    }

    if let Some(power) = sensors.power_uw {
        if power >= config.power_warn_uw {
            return SafetyStatus::PowerWarning;
        }
    }

    if sensors.temp_mc.is_none() && sensors.power_uw.is_none() {
        return SafetyStatus::Unknown;
    }

    SafetyStatus::Normal
}

/// Take a single monitoring sample from a GPU device path.
///
/// # Errors
/// Returns error if sensors cannot be read.
pub fn sample(device_path: &Path, config: &MonitorConfig) -> Result<MonitorSample> {
    let sensors = HwmonSensors::from_device(device_path)?;
    let status = evaluate_safety(&sensors, config);

    if status == SafetyStatus::ThermalCritical {
        tracing::error!(
            temp_mc = sensors.temp_mc,
            limit = config.temp_crit_mc,
            "THERMAL CRITICAL — GPU temperature exceeds safety limit"
        );
    }

    Ok(MonitorSample { sensors, status })
}

/// Assert thermal safety — returns error if temperature exceeds the
/// critical threshold.
///
/// # Errors
/// Returns [`NvPmuError::ThermalLimit`] if temperature exceeds limit.
pub fn assert_thermal_safe(sensors: &HwmonSensors, config: &MonitorConfig) -> Result<()> {
    if let Some(temp) = sensors.temp_mc {
        if temp >= config.temp_crit_mc {
            return Err(NvPmuError::ThermalLimit {
                temp_mc: temp,
                limit_mc: config.temp_crit_mc,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_sensors(temp_mc: Option<i64>, power_uw: Option<u64>) -> HwmonSensors {
        HwmonSensors {
            hwmon_path: PathBuf::from("/test"),
            temp_mc,
            temp_crit_mc: None,
            clock_mhz: None,
            mem_clock_mhz: None,
            power_uw,
            power_limit_uw: None,
            fan_rpm: None,
        }
    }

    #[test]
    fn normal_status() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(45_000), Some(120_000_000));
        assert_eq!(evaluate_safety(&sensors, &config), SafetyStatus::Normal);
        assert!(SafetyStatus::Normal.compute_safe());
    }

    #[test]
    fn thermal_warning() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(87_000), None);
        assert_eq!(
            evaluate_safety(&sensors, &config),
            SafetyStatus::ThermalWarning
        );
    }

    #[test]
    fn thermal_critical() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(96_000), None);
        assert_eq!(
            evaluate_safety(&sensors, &config),
            SafetyStatus::ThermalCritical
        );
        assert!(!SafetyStatus::ThermalCritical.compute_safe());
    }

    #[test]
    fn power_warning() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(50_000), Some(310_000_000));
        assert_eq!(
            evaluate_safety(&sensors, &config),
            SafetyStatus::PowerWarning
        );
    }

    #[test]
    fn unknown_no_sensors() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(None, None);
        assert_eq!(evaluate_safety(&sensors, &config), SafetyStatus::Unknown);
    }

    #[test]
    fn assert_thermal_safe_ok() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(45_000), None);
        assert!(assert_thermal_safe(&sensors, &config).is_ok());
    }

    #[test]
    fn assert_thermal_safe_err() {
        let config = MonitorConfig::default();
        let sensors = make_sensors(Some(96_000), None);
        assert!(assert_thermal_safe(&sensors, &config).is_err());
    }
}
