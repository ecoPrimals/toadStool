// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    missing_docs,
    reason = "tags mirror historical manifest keys; module docs describe intent"
)]
//! Canonical serde tag names and legacy manifest aliases for biome / auth manifest interop.
//!
//! Use these instead of inline literals for [`Deserialize`](serde::Deserialize) /
//! [`Serialize`](serde::Serialize) of capability-oriented configs. Legacy aliases exist
//! only for older manifests that used product-era primal names.

pub const TOADSTOOL: &str = "ToadStool";
pub const COORDINATION: &str = "Coordination";
pub const SECURITY_SERVICE: &str = "SecurityService";
pub const STORAGE_SERVICE: &str = "StorageService";
pub const INTELLIGENCE_SERVICE: &str = "IntelligenceService";
pub const BIOME_OS: &str = "BiomeOS";

/// Legacy manifest key (PascalCase product name).
pub const LEGACY_SONGBIRD_PASCAL: &str = "Songbird";
pub const LEGACY_SONGBIRD_LOWER: &str = super::primals::LEGACY_COORDINATION_LABEL;

pub const LEGACY_BEARDOG_PASCAL: &str = "BearDog";
pub const LEGACY_BEARDOG_CAMEL: &str = "bearDog";
pub const LEGACY_BEARDOG_LOWER: &str = super::primals::LEGACY_SECURITY_LABEL;

pub const LEGACY_NESTGATE_PASCAL: &str = "NestGate";
pub const LEGACY_NESTGATE_LOWER: &str = super::primals::LEGACY_STORAGE_LABEL;
pub const LEGACY_NESTGATE_KEBAB: &str = super::primals::LEGACY_STORAGE_KEBAB;

pub const LEGACY_SQUIRREL_PASCAL: &str = "Squirrel";
pub const LEGACY_SQUIRREL_LOWER: &str = super::primals::LEGACY_INTELLIGENCE_LABEL;
