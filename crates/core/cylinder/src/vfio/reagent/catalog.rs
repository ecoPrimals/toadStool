// SPDX-License-Identifier: AGPL-3.0-or-later

//! Linux-firmware cataloging — scan `/lib/firmware/nvidia/{chip}/` for known blobs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A cataloged firmware blob from linux-firmware or extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareBlob {
    /// Subsystem (e.g. "gr", "acr", "sec2", "pmu").
    pub subsystem: String,
    /// Filename (e.g. "fecs_inst.bin").
    pub filename: String,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Whether this blob is required for ACR boot chain.
    pub acr_required: bool,
}

/// Catalog linux-firmware blobs for a given chip.
///
/// Scans `/lib/firmware/nvidia/{chip}/` for known firmware files and
/// returns a list of those that exist on disk with their metadata.
pub fn catalog_linux_firmware(chip: &str) -> Vec<FirmwareBlob> {
    let base = format!("/lib/firmware/nvidia/{chip}");

    let known_blobs = [
        ("acr", "bl.bin", true),
        ("acr", "ucode_unload.bin", false),
        ("gr", "fecs_bl.bin", true),
        ("gr", "fecs_inst.bin", true),
        ("gr", "fecs_data.bin", true),
        ("gr", "gpccs_bl.bin", true),
        ("gr", "gpccs_inst.bin", true),
        ("gr", "gpccs_data.bin", true),
        ("gr", "sw_ctx.bin", false),
        ("gr", "sw_nonctx.bin", false),
        ("gr", "sw_bundle_init.bin", false),
        ("gr", "sw_method_init.bin", false),
        ("sec2", "desc.bin", true),
        ("sec2", "image.bin", true),
        ("sec2", "sig.bin", true),
        ("pmu", "bl.bin", false),
        ("pmu", "inst.bin", false),
        ("pmu", "data.bin", false),
        ("pmu", "sig.bin", false),
    ];

    let mut found = Vec::new();
    for (subsystem, filename, acr_required) in &known_blobs {
        let path = PathBuf::from(format!("{base}/{subsystem}/{filename}"));
        if path.exists() {
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            found.push(FirmwareBlob {
                subsystem: (*subsystem).to_owned(),
                filename: (*filename).to_owned(),
                path,
                size_bytes,
                acr_required: *acr_required,
            });
        }
    }

    tracing::info!(
        chip = chip,
        found = found.len(),
        acr_required_present = found.iter().filter(|b| b.acr_required).count(),
        "linux-firmware blobs cataloged"
    );

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_blob_serde() {
        let blob = FirmwareBlob {
            subsystem: "gr".to_owned(),
            filename: "fecs_inst.bin".to_owned(),
            path: PathBuf::from("/lib/firmware/nvidia/gv100/gr/fecs_inst.bin"),
            size_bytes: 32768,
            acr_required: true,
        };
        let json = serde_json::to_string(&blob).unwrap();
        let loaded: FirmwareBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.subsystem, "gr");
        assert!(loaded.acr_required);
    }
}
