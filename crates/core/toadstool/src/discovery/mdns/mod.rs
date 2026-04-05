// SPDX-License-Identifier: AGPL-3.0-or-later
//! # mDNS Discovery Service
//!
//! Implements automatic service discovery using mDNS/DNS-SD (Multicast DNS).
//!
//! ## Evolution (Feb 15, 2026) - Coordination service delegation
//!
//! ToadStool **exposes** mDNS capability to the coordination service (comms plane).
//! This follows the ecoPrimal separation of concerns:
//!
//! - **ToadStool**: Owns hardware discovery (GPU, NPU, CPU), exposes mDNS capability
//! - **Coordination service**: Owns network discovery and coordinates the service mesh
//!
//! ToadStool advertises its capabilities via mDNS. The coordination service discovers
//! and coordinates peers across the network. ToadStool does NOT
//! implement vendor-specific service meshes (K8s, Consul, etc.) —
//! that belongs to the coordination layer.
//!
//! ## Key Concepts
//!
//! - **Service Type**: `_toadstool._tcp.local.`
//! - **Advertise by Capability**: Services advertise WHAT they can do
//! - **Discover by Capability**: Find services by WHAT you need
//! - **No Hardcoding**: Zero hardcoded addresses
//! - **Coordination integration**: mDNS exposed for the comms / discovery plane
//!
//! ## Example
//!
//! ```rust,no_run
//! use toadstool::discovery::MdnsDiscoveryService;
//! use toadstool::self_identity::SelfIdentity;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let identity = SelfIdentity::new()
//!     .with_network("toadstool-01".to_string(), Some(8084), vec!["http".to_string()]);
//!
//! let mdns = MdnsDiscoveryService::new()?;
//! mdns.advertise(&identity)?;
//!
//! // Discover services by capability
//! let storage_services = mdns.discover_by_capability("storage", std::time::Duration::from_secs(5)).await?;
//! # Ok(())
//! # }
//! ```

mod constants;
mod parse;
mod service;

pub use constants::TOADSTOOL_SERVICE_TYPE;
pub use service::MdnsDiscoveryService;
