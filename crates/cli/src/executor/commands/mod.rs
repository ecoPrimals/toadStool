// SPDX-License-Identifier: AGPL-3.0-only
//! Public CLI Commands for Biome Execution
//!
//! This module contains all user-facing commands:
//! - `new()` - Constructor
//! - `run_biome()` - Start biome in foreground
//! - `up_biome()` - Start biome in background (detached)
//! - `down_biome()` - Stop running biome
//! - `list_biomes()` - List all biomes
//! - `show_logs()` - View biome/service logs
//!
//! **Deep Debt Principles**:
//! - ✅ Real implementations (no mocks)
//! - ✅ Modern async/await
//! - ✅ Capability-based discovery (no hardcoded registry)

mod down_list;
mod logs;
mod new_run;
mod up_background;

#[cfg(test)]
mod tests;
