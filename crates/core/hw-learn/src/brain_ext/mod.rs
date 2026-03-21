// SPDX-License-Identifier: AGPL-3.0-only
//! Extensions to toadStool's `PrecisionBrain` for hardware learning.
//!
//! - **`LearningAdvisor`** — identifies teacher/student GPU pairs and
//!   learning opportunities across the fleet.
//! - **`FirmwareInventory`** — probes firmware availability per GPU
//!   (PMU, GSP, ACR, GR, `GuC`, `HuC`).
//! - **`CapabilityGap`** — describes what's missing for compute to work.

pub mod capability_gap;
pub mod firmware_probe;
pub mod learning_advisor;

pub use capability_gap::CapabilityGap;
pub use firmware_probe::{FirmwareInventory, FirmwareInventoryExt, FwStatus};
pub use learning_advisor::{LearningAdvisor, LearningOpportunity};
