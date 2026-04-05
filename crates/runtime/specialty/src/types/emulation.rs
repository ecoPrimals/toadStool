// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emulation type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::LegacySystemType;

use crate::ToadStoolResult;

/// Legacy system emulator trait
#[async_trait::async_trait]
pub trait LegacyEmulator: Send + Sync {
    /// Get emulator name
    fn name(&self) -> &'static str;

    /// Get supported systems
    fn supported_systems(&self) -> Vec<LegacySystemType>;

    /// Initialize the emulator
    async fn initialize(&mut self, config: &EmulationConfig) -> ToadStoolResult<()>;

    /// Start the emulator
    async fn start(&mut self) -> ToadStoolResult<()>;

    /// Stop the emulator
    async fn stop(&mut self) -> ToadStoolResult<()>;

    /// Reset the emulator
    async fn reset(&mut self) -> ToadStoolResult<()>;

    /// Load disk/ROM image
    async fn load_image(&mut self, image: &Path) -> ToadStoolResult<()>;

    /// Save emulator state
    async fn save_state(&mut self, path: &Path) -> ToadStoolResult<()>;

    /// Load emulator state
    async fn load_state(&mut self, path: &Path) -> ToadStoolResult<()>;

    /// Get emulator status
    async fn get_status(&self) -> ToadStoolResult<EmulationStatus>;
}

/// Emulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulationConfig {
    /// CPU speed (`MHz`)
    pub cpu_speed_mhz: Option<f64>,
    /// Memory size (bytes)
    pub memory_size: usize,
    /// Enable debugging
    pub enable_debugging: bool,
    /// ROM paths
    pub rom_paths: Vec<PathBuf>,
    /// Peripheral configuration
    pub peripherals: HashMap<String, PeripheralConfig>,
}

impl Default for EmulationConfig {
    fn default() -> Self {
        Self {
            cpu_speed_mhz: None,
            memory_size: 65536, // 64KB default
            enable_debugging: false,
            rom_paths: Vec::new(),
            peripherals: HashMap::new(),
        }
    }
}

/// Peripheral configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralConfig {
    /// Peripheral type
    pub peripheral_type: String,
    /// Configuration options
    pub options: HashMap<String, String>,
}

/// Emulation status
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmulationStatus {
    /// Emulator is not initialized
    #[default]
    Uninitialized,
    /// Emulator is ready
    Ready,
    /// Emulation is running
    Running,
    /// Emulation is paused
    Paused,
    /// Emulation stopped
    Stopped,
    /// Emulation encountered an error
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn assert_json_round_trip_eq<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_string(value).expect("serialize");
        let back: T = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(serde_json::to_string(&back).expect("re-serialize"), json);
    }

    fn sample_emulation_config() -> EmulationConfig {
        let mut peripherals = HashMap::new();
        peripherals.insert(
            "uart0".to_string(),
            PeripheralConfig {
                peripheral_type: "serial".to_string(),
                options: HashMap::from([("baud".to_string(), "115200".to_string())]),
            },
        );
        EmulationConfig {
            cpu_speed_mhz: Some(1.0),
            memory_size: 131_072,
            enable_debugging: true,
            rom_paths: vec![PathBuf::from("/tmp/test.rom")],
            peripherals,
        }
    }

    #[test]
    fn emulation_config_default_matches_expected() {
        let c = EmulationConfig::default();
        assert!(c.cpu_speed_mhz.is_none());
        assert_eq!(c.memory_size, 65536);
        assert!(!c.enable_debugging);
        assert!(c.rom_paths.is_empty());
        assert!(c.peripherals.is_empty());
    }

    #[test]
    fn emulation_status_default_is_uninitialized() {
        assert_eq!(EmulationStatus::default(), EmulationStatus::Uninitialized);
    }

    #[test]
    fn emulation_config_clone_preserves_serialization() {
        let a = sample_emulation_config();
        let b = a.clone();
        assert_json_round_trip_eq(&a);
        assert_json_round_trip_eq(&b);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn peripheral_config_clone_preserves_serialization() {
        let p = PeripheralConfig {
            peripheral_type: "gpio".to_string(),
            options: HashMap::from([("pull".to_string(), "up".to_string())]),
        };
        let q = p.clone();
        assert_json_round_trip_eq(&p);
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            serde_json::to_string(&q).unwrap()
        );
    }

    #[test]
    fn emulation_status_clone_and_eq() {
        let variants = [
            EmulationStatus::Uninitialized,
            EmulationStatus::Ready,
            EmulationStatus::Running,
            EmulationStatus::Paused,
            EmulationStatus::Stopped,
            EmulationStatus::Error("fault".to_string()),
        ];
        for v in variants {
            assert_eq!(v.clone(), v);
            assert_json_round_trip_eq(&v);
        }
    }

    #[test]
    fn debug_formats_include_type_names() {
        let cfg = format!("{:?}", sample_emulation_config());
        let per = format!(
            "{:?}",
            PeripheralConfig {
                peripheral_type: "t".to_string(),
                options: HashMap::new(),
            }
        );
        let st = format!("{:?}", EmulationStatus::Running);
        assert!(cfg.contains("EmulationConfig"), "{cfg}");
        assert!(per.contains("PeripheralConfig"), "{per}");
        assert!(st.contains("Running"), "{st}");
        assert!(format!("{:?}", EmulationStatus::Error("e".into())).contains("Error"),);
    }

    #[test]
    fn serde_json_round_trip_emulation_config() {
        assert_json_round_trip_eq(&sample_emulation_config());
        assert_json_round_trip_eq(&EmulationConfig::default());
    }

    #[test]
    fn serde_json_round_trip_peripheral_config() {
        assert_json_round_trip_eq(&PeripheralConfig {
            peripheral_type: "timer".to_string(),
            options: HashMap::new(),
        });
    }

    #[test]
    fn serde_json_round_trip_emulation_status_variants() {
        for s in [
            EmulationStatus::Uninitialized,
            EmulationStatus::Ready,
            EmulationStatus::Running,
            EmulationStatus::Paused,
            EmulationStatus::Stopped,
            EmulationStatus::Error("io".to_string()),
        ] {
            assert_json_round_trip_eq(&s);
            let back: EmulationStatus =
                serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
            assert_eq!(s, back);
        }
    }
}
