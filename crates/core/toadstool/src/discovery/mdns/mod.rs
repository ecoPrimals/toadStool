// SPDX-License-Identifier: AGPL-3.0-or-later
//! # mDNS Discovery Service
//!
//! Implements automatic service discovery using mDNS/DNS-SD (Multicast DNS).
//!
//! ## Evolution (Feb 15, 2026) - Songbird Delegation
//!
//! ToadStool **exposes** mDNS capability to Songbird (comms primal).
//! This follows the ecoPrimal separation of concerns:
//!
//! - **ToadStool**: Owns hardware discovery (GPU, NPU, CPU), exposes mDNS capability
//! - **Songbird**: Owns network discovery, coordinates service mesh
//!
//! ToadStool advertises its capabilities via mDNS. Songbird discovers
//! and coordinates primals across the network. ToadStool does NOT
//! implement vendor-specific service meshes (K8s, Consul, etc.) -
//! that's Songbird's domain.
//!
//! ## Key Concepts
//!
//! - **Service Type**: `_toadstool._tcp.local.`
//! - **Advertise by Capability**: Services advertise WHAT they can do
//! - **Discover by Capability**: Find services by WHAT you need
//! - **No Hardcoding**: Zero hardcoded addresses
//! - **Songbird Integration**: mDNS exposed for comms primal
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
