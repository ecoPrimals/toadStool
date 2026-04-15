// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bluetooth adapter presence check (device enumeration deferred).

use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use toadstool::error::ToadStoolResult;

use crate::platforms::*;

use super::DiscoveryMethod;

/// Bluetooth Discovery Method
pub struct BluetoothDiscovery {
    pub(super) scan_duration: Duration,
    pub(super) device_types: Vec<String>,
}

#[async_trait::async_trait]
impl DiscoveryMethod for BluetoothDiscovery {
    fn get_name(&self) -> &str {
        "Bluetooth Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        if !self.is_available().await {
            debug!("No Bluetooth adapters found via sysfs");
            return Ok(Vec::new());
        }
        // Bluetooth device discovery requires pairing and service enumeration,
        // which depends on the BlueZ D-Bus API or direct HCI commands.
        // Return empty for now — the adapter check above is the real evolution.
        debug!("Bluetooth adapter present but device enumeration requires BlueZ D-Bus integration");
        Ok(Vec::new())
    }

    async fn is_available(&self) -> bool {
        // Probe for Bluetooth adapters via sysfs (pure Rust, no libc)
        let bt_class = std::path::Path::new("/sys/class/bluetooth");
        if !bt_class.exists() {
            return false;
        }
        match std::fs::read_dir(bt_class) {
            Ok(entries) => entries.filter_map(|e| e.ok()).next().is_some(),
            Err(_) => false,
        }
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Bluetooth Device".to_string(),
            "ESP32 Bluetooth".to_string(),
        ]
    }
}
