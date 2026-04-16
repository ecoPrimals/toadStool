// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage detection and type classification

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;
use toadstool_common::constants::platform_paths::sysfs;

use super::HardwareDetector;

/// Storage information and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Total storage in GB.
    pub total_gb: f64,
    /// Available storage in GB.
    pub available_gb: f64,
    /// Storage device type.
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

/// Parse `df` stdout (e.g. `df -h /` or `df -BG /`) and return `(total_gb, available_gb)` from the
/// first data row after the header.
fn parse_df_available(stdout: &str) -> Option<(f64, f64)> {
    let mut lines = stdout.lines();
    lines.next()?; // skip header
    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let total_gb = parts[1].trim_end_matches('G').parse::<f64>().ok()?;
            let available_gb = parts[3].trim_end_matches('G').parse::<f64>().ok()?;
            return Some((total_gb, available_gb));
        }
    }
    None
}

/// Map Linux `queue/rotational` sysfs content to a [`StorageType`].
fn classify_rotational(value: &str) -> StorageType {
    match value.trim() {
        "0" => StorageType::SSD,
        "1" => StorageType::HDD,
        _ => StorageType::Unknown,
    }
}

/// Storage device type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// Hard disk drive.
    HDD,
    /// Solid-state drive.
    SSD,
    /// NVMe SSD.
    NVME,
    /// Unknown storage type.
    Unknown,
}

/// Detect storage configuration
pub async fn detect_storage(_detector: &HardwareDetector) -> ToadStoolResult<StorageInfo> {
    let mut storage_info = StorageInfo::default();

    // Linux storage detection
    if cfg!(target_os = "linux")
        && let Ok(output) = tokio::process::Command::new("df")
            .arg("-BG")
            .arg("/")
            .output()
            .await
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some((total_gb, available_gb)) = parse_df_available(&output_str) {
            storage_info.total_gb = total_gb;
            storage_info.available_gb = available_gb;
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
    if cfg!(target_os = "linux")
        && let Ok(rotational) = tokio::fs::read_to_string(sysfs::BLOCK_SDA_QUEUE_ROTATIONAL).await
    {
        return Ok(classify_rotational(rotational.trim()));
    }

    // Default assumption: SSD for modern systems
    Ok(StorageType::SSD)
}

/// Calculate storage performance score
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::{StorageType, classify_rotational, parse_df_available};

    #[test]
    fn parse_df_available_typical_df_h() {
        let stdout = r"Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        99G   19G   75G   20% /
";
        assert_eq!(parse_df_available(stdout), Some((99.0, 75.0)));
    }

    #[test]
    fn parse_df_available_df_bg_style() {
        let stdout = r"Filesystem     1G-blocks  Used Available Use% Mounted on
/dev/nvme0n1p2     468G   99G      345G  23% /
";
        assert_eq!(parse_df_available(stdout), Some((468.0, 345.0)));
    }

    #[test]
    fn parse_df_available_skips_to_first_full_row() {
        let stdout = r"Filesystem     Size  Used Avail Use% Mounted on

/dev/mapper/root   250G   100G  140G  42% /
";
        assert_eq!(parse_df_available(stdout), Some((250.0, 140.0)));
    }

    #[test]
    fn parse_df_available_no_data_rows() {
        assert_eq!(parse_df_available("Filesystem  Size\n"), None);
        assert_eq!(parse_df_available(""), None);
    }

    #[test]
    fn parse_df_available_malformed_numbers() {
        let stdout = r"Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        abc   19G   75G   20% /
";
        assert_eq!(parse_df_available(stdout), None);
    }

    #[test]
    fn classify_rotational_ssd_hdd_unknown() {
        assert!(matches!(classify_rotational("0"), StorageType::SSD));
        assert!(matches!(classify_rotational("0\n"), StorageType::SSD));
        assert!(matches!(classify_rotational("1"), StorageType::HDD));
        assert!(matches!(classify_rotational(" 1 "), StorageType::HDD));
        assert!(matches!(classify_rotational("2"), StorageType::Unknown));
        assert!(matches!(classify_rotational(""), StorageType::Unknown));
        assert!(matches!(classify_rotational("yes"), StorageType::Unknown));
    }
}
