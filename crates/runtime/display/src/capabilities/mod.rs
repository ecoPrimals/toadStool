// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability discovery and advertisement
//!
//! Implements self-knowledge and capability-based discovery for
//! display backend services.
//!
//! **Deep Debt Compliance:**
//! - ✅ Self-knowledge only (discovers own hardware)
//! - ✅ No hardcoding (runtime discovery)
//! - ✅ Capability-based (advertises via files)
//! - ✅ Agnostic (no primal-specific logic)

mod operations;
mod paths;
mod types;

#[cfg(test)]
mod tests;

pub use types::{CapabilityMetadata, DisplayCapabilities, DisplayInfo, InputDeviceInfo};

// SAFETY REVIEW:
//
// Unsafe usage in this module: NONE
//
// All path resolution now uses toadstool_common::platform_paths which
// internally handles XDG compliance without unsafe code.
//
// Grade: ✅ SAFE
//
// Public API: 100% SAFE

// Pending enhancements:
//
// 1. Query actual display modes from DRM (get_connectors, get_modes) for resolution/refresh
// 2. Add display hotplug detection
// 3. Add input device hotplug detection
// 4. Add capability versioning
// 5. Add capability expiry (TTL)
// 6. Add health check mechanism
