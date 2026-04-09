// SPDX-License-Identifier: AGPL-3.0-or-later
//! ⚠️ DEPRECATED: Legacy primal name constants
//!
//! **For IPC addressing only** (socket paths, endpoint IDs, message routing).
//! These are canonical names for addressing — NOT for capability matching.
//! Use `capabilities::*` for discovery; use these only when you already have
//! a discovered service and need its name for socket paths or routing.

/// Legacy IPC route string for security services (matches `BEARDOG_SOCKET` era paths).
pub const LEGACY_SECURITY_LABEL: &str = "beardog";

/// Legacy IPC route string for coordination services.
pub const LEGACY_COORDINATION_LABEL: &str = "songbird";

/// Legacy IPC route string for storage services.
pub const LEGACY_STORAGE_LABEL: &str = "nestgate";

/// Legacy IPC route string for intelligence / routing services.
pub const LEGACY_INTELLIGENCE_LABEL: &str = "squirrel";

/// Legacy IPC route label for crypto/security services
/// **DEPRECATED**: Use [`LEGACY_SECURITY_LABEL`] or `capabilities::CRYPTO` for discovery
#[deprecated(note = "use LEGACY_SECURITY_LABEL or capabilities::CRYPTO / SECURITY")]
pub const BEARDOG: &str = LEGACY_SECURITY_LABEL;

/// Legacy IPC route label for coordination / mesh
/// **DEPRECATED**: Use [`LEGACY_COORDINATION_LABEL`] or `capabilities::COORDINATION`
#[deprecated(note = "use LEGACY_COORDINATION_LABEL or capabilities::COORDINATION")]
pub const SONGBIRD: &str = LEGACY_COORDINATION_LABEL;

/// Legacy IPC route label for storage / artifacts
/// **DEPRECATED**: Use [`LEGACY_STORAGE_LABEL`] or `capabilities::STORAGE`
#[deprecated(note = "use LEGACY_STORAGE_LABEL or capabilities::STORAGE")]
pub const NESTGATE: &str = LEGACY_STORAGE_LABEL;

/// Legacy IPC route label for routing / agent IPC
/// **DEPRECATED**: Use [`LEGACY_INTELLIGENCE_LABEL`] or `capabilities::ROUTING` / `INTELLIGENCE`
#[deprecated(note = "use LEGACY_INTELLIGENCE_LABEL or capabilities::ROUTING / INTELLIGENCE")]
pub const SQUIRREL: &str = LEGACY_INTELLIGENCE_LABEL;

/// ToadStool compute service identifier
pub const TOADSTOOL: &str = "toadstool";
