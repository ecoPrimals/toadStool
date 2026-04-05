// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # glowPlug — Hardware-Agnostic Device Lifecycle Interface
//!
//! glowPlug is toadStool's universal device lifecycle subsystem. Like a
//! diesel engine's glow plug system, it is the interface that manages how
//! hardware is discovered, personality-swapped, health-monitored, and
//! exposed to the ecosystem.
//!
//! ## Architecture
//!
//! glowPlug is the **mailbox and ringmaker** — the public interface that the
//! ecosystem talks to. [`ember`] is its internal subsystem
//! that **holds** exclusive resources (the holder/warmer/immortalizer).
//!
//! ```text
//! ecosystem (barraCuda, coralReef, biomeOS)
//!      |
//!   JSON-RPC IPC
//!      |
//!   glowPlug (this crate)
//!      ├── DevicePersonality — what mode/driver is the device in?
//!      ├── DeviceSlot — a managed device with current personality
//!      ├── SwapOrchestrator — quiesce → persist → swap → restore → health
//!      ├── HealthMonitor — passive (sysfs) + active (probe) health
//!      ├── FirmwareInterface — boundary for opaque firmware (FECS, UEFI, NPU)
//!      └── ember (subsystem)
//!           ├── HeldResource — exclusive handle + metadata
//!           ├── ResourceHandle — abstract over VFIO/USB/DRM/HSM fds
//!           └── lend/reclaim — hand off and take back
//! ```
//!
//! ## Hardware Agnosticism
//!
//! coralReef's `coral-glowplug` and `coral-ember` are the **first evolution**
//! (GPU/VFIO-specific). toadStool's glowPlug generalizes that pattern:
//!
//! - **GPU**: VFIO passthrough, DRM drivers, BAR0 MMIO, FECS firmware
//! - **NPU**: Akida MMIO, neuromorphic firmware
//! - **CPU**: governors, isolation, frequency scaling
//! - **USB**: host/gadget modes, interface claims
//! - **HSM / TEE**: cryptographic sessions, enclave lifecycle
//! - **Bluetooth**: HCI sockets, controller modes
//! - **Display**: DRM primary nodes, framebuffer access
//!
//! Each hardware class implements the glowPlug traits. The orchestration
//! (swap lifecycle, health monitoring, journal, metadata persistence) is
//! identical across all hardware — only the handle type and personality
//! variants differ.

pub mod device_id;
pub mod device_slot;
pub mod discovery;
pub mod firmware;
pub mod health;
pub mod personality;
pub mod swap;

// Re-export ember as the holder subsystem
pub use toadstool_ember as ember;

// Re-export key types
pub use device_id::DeviceId;
pub use device_slot::DeviceSlot;
pub use discovery::DeviceDiscovery;
pub use firmware::FirmwareInterface;
pub use health::{HealthProbe, HealthStatus};
pub use personality::DevicePersonality;
pub use swap::SwapOrchestrator;
