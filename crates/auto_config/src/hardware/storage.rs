//! Storage detection and type classification

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// Storage information and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub total_gb: f64,
    pub available_gb: f64,
    pub storage_type: StorageType,
}

impl Default for StorageInfo {
    fn default() -> Self {
        Self {
            total_gb: 100.0,
            available_gb: 80.0,
            storage_type: StorageType::SSD,
        }
    }
}

/// Storage device type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVME,
    Unknown,
}

/// Detect storage configuration
pub async fn detect_storage(_detector: &HardwareDetector) -> ToadStoolResult<StorageInfo> {
    let mut storage_info = StorageInfo::default();

    // Linux storage detection
    if cfg!(target_os = "linux") {
        if let Ok(output) = tokio::process::Command::new("df")
            .arg("-BG")
            .arg("/")
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(total_gb) = parts[1].trim_end_matches('G').parse::<f64>() {
                        storage_info.total_gb = total_gb;
                    }
                    if let Ok(available_gb) = parts[3].trim_end_matches('G').parse::<f64>() {
                        storage_info.available_gb = available_gb;
                    }
                    break;
                }
            }
        }
    }

    // Detect storage type (SSD vs HDD)
    storage_info.storage_type = detect_storage_type().await?;

    debug!(
        "Detected storage: {:.1} GB total, {:.1} GB available, type: {:?}",
        storage_info.total_gb, storage_info.available_gb, storage_info.storage_type
    );

    Ok(storage_info)
}

/// Detect storage type (SSD vs HDD)
async fn detect_storage_type() -> ToadStoolResult<StorageType> {
    // Linux: check rotational attribute
    if cfg!(target_os = "linux") {
        if let Ok(rotational) = tokio::fs::read_to_string("/sys/block/sda/queue/rotational").await {
            if rotational.trim() == "0" {
                return Ok(StorageType::SSD);
            }
            return Ok(StorageType::HDD);
        }
    }

    // Default assumption: SSD for modern systems
    Ok(StorageType::SSD)
}

/// Calculate storage performance score
pub fn calculate_storage_score(storage_info: &StorageInfo) -> f64 {
    let capacity_score = (storage_info.total_gb / 1000.0 * 50.0).min(50.0);
    let type_score = match storage_info.storage_type {
        StorageType::NVME => 50.0,
        StorageType::SSD => 40.0,
        StorageType::HDD => 20.0,
        StorageType::Unknown => 25.0,
    };

    capacity_score + type_score
}
