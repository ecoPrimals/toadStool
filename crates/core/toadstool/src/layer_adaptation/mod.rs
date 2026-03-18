// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Capability Adaptation Based on Deployment Layer
//!
//! This module adapts Toadstool's capabilities based on the detected deployment
//! layer, ensuring appropriate exposure of resources (GPU, storage, network) for
//! each environment.
//!
//! # Philosophy
//!
//! **Adaptation over assumption**: Don't assume what capabilities we should expose.
//! Detect the layer, then adapt capabilities accordingly.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::deployment_layer::{DeploymentLayer, LayerDetector};
//! use toadstool::layer_adaptation::LayerCapabilityAdapter;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut detector = LayerDetector::new();
//! let layer = detector.detect().await?;
//!
//! let adapter = LayerCapabilityAdapter::new(layer);
//! let capabilities = adapter.get_adapted_capabilities();
//!
//! // Capabilities are now appropriate for the layer
//! if capabilities.has_direct_gpu_access() {
//!     println!("Can use GPU directly");
//! } else {
//!     println!("GPU via host or cloud APIs");
//! }
//! # Ok(())
//! # }
//! ```

pub mod adapters;
pub mod detection;
pub mod types;

// Re-export all public types for external consumers
pub use adapters::LayerCapabilityAdapter;
pub use detection::{
    detect_network_bandwidth, detect_storage_read_bandwidth, detect_storage_write_bandwidth,
    get_available_disk, get_total_memory,
};
pub use types::{
    AdaptedCapabilities, CapabilityMetadata, ComputeCapabilities, GpuAccess, NetworkAccess,
    NetworkCapabilities, StorageCapabilities, StorageType, compute_capabilities,
    network_capabilities, storage_capabilities,
};
