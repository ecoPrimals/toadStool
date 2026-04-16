// SPDX-License-Identifier: AGPL-3.0-or-later
//! Filesystem-backed “mDNS” discovery (runtime sockets + JSON registry).

use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use toadstool::error::ToadStoolResult;

use crate::platforms::*;

use super::DiscoveryMethod;

/// mDNS Discovery Method
pub struct MDNSDiscovery {
    pub(super) service_types: Vec<String>,
    pub(super) timeout: Duration,
}

impl DiscoveryMethod for MDNSDiscovery {
    fn get_name(&self) -> &str {
        "mDNS Discovery"
    }

    fn discover(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = ToadStoolResult<Vec<Arc<dyn EdgeDevice>>>> + Send + '_,
        >,
    > {
        Box::pin(async move {
            // Primary: filesystem-based discovery for edge devices registering via biomeOS runtime
            // Edge devices on the same host (e.g. Raspberry Pi running ToadStool) register sockets
            if let Some(devices) = self.discover_via_filesystem().await? {
                if !devices.is_empty() {
                    return Ok(devices);
                }
            }

            // Fallback: scan for _toadstool-edge._tcp service type via filesystem polling
            // Devices can register by creating JSON descriptor in $XDG_RUNTIME_DIR/edge-devices/
            if let Some(devices) = self.discover_via_edge_registry().await? {
                if !devices.is_empty() {
                    return Ok(devices);
                }
            }

            debug!("mDNS/filesystem discovery: no edge devices found");
            Ok(Vec::new())
        })
    }

    fn is_available(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { true })
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "mDNS Device".to_string(),
            "Network Service".to_string(),
            "ToadStool Edge".to_string(),
        ]
    }
}

impl MDNSDiscovery {
    /// Discover edge devices via biomeOS runtime directory (Unix sockets for edge proxies)
    async fn discover_via_filesystem(&self) -> ToadStoolResult<Option<Vec<Arc<dyn EdgeDevice>>>> {
        let biomeos_dir = toadstool::platform_paths::biomeos_runtime_dir();

        if !biomeos_dir.exists() {
            return Ok(None);
        }

        let mut devices = Vec::new();
        let mut entries = match tokio::fs::read_dir(&biomeos_dir).await {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "sock") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem == "toadstool-edge" || stem.ends_with("-edge") {
                    debug!("Found edge device socket: {}", path.display());
                    if let Some(device) = self.create_edge_device_from_socket(&path).await {
                        devices.push(device);
                    }
                }
            }
        }

        if devices.is_empty() {
            Ok(None)
        } else {
            Ok(Some(devices))
        }
    }

    /// Discover via edge device registry ($XDG_RUNTIME_DIR/edge-devices/*.json)
    async fn discover_via_edge_registry(
        &self,
    ) -> ToadStoolResult<Option<Vec<Arc<dyn EdgeDevice>>>> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .or_else(|| std::env::var("BIOMEOS_RUNTIME_DIR").ok())
            .unwrap_or_else(|| {
                std::env::temp_dir()
                    .join("toadstool-runtime")
                    .to_string_lossy()
                    .into_owned()
            });

        let edge_dir = std::path::PathBuf::from(&runtime_dir).join("edge-devices");
        if !edge_dir.exists() {
            return Ok(None);
        }

        let mut devices = Vec::new();
        let mut entries = match tokio::fs::read_dir(&edge_dir).await {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(device) = self.parse_edge_registry_entry(&content) {
                        devices.push(device);
                    }
                }
            }
        }

        if devices.is_empty() {
            Ok(None)
        } else {
            Ok(Some(devices))
        }
    }

    async fn create_edge_device_from_socket(
        &self,
        path: &std::path::Path,
    ) -> Option<Arc<dyn EdgeDevice>> {
        if !path.exists() {
            debug!("Edge socket {:?} no longer exists, skipping", path);
            return None;
        }
        let device = LinuxEdgeDevice::from_socket_path(path.to_path_buf());
        info!("Created LinuxEdge device from socket: {:?}", path);
        Some(Arc::new(device))
    }

    fn parse_edge_registry_entry(&self, content: &str) -> ToadStoolResult<Arc<dyn EdgeDevice>> {
        let device = LinuxEdgeDevice::from_registry_json(content)?;
        info!("Parsed edge registry entry: {}", device.get_info().name);
        Ok(Arc::new(device))
    }
}
