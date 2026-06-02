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

/// Legacy kebab-case route alias (manifest / DNS interop).
pub const LEGACY_SECURITY_KEBAB: &str = "bear-dog";

/// Legacy kebab-case route alias (manifest / DNS interop).
pub const LEGACY_COORDINATION_KEBAB: &str = "song-bird";

/// Legacy kebab-case route alias (manifest / DNS interop).
pub const LEGACY_STORAGE_KEBAB: &str = "nest-gate";

/// ToadStool compute service identifier
pub const TOADSTOOL: &str = "toadstool";
