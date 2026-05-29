// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign warm handoff orchestrator — the diesel engine's driver rotation pipeline.
//!
//! Composes kernel module management, binary patching, sysfs driver
//! bind/unbind, and tier classification into a single operation. The
//! operator makes one RPC call; the daemon handles everything.
//!
//! # Pipeline
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ 1. Module Preparation                                               │
//! │    Patched: find stock .ko → binary-patch → insmod                 │
//! │    System:  verify module loaded (or load it)                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 2. Seeder Bind                                                      │
//! │    unbind current driver → driver_override → drivers_probe         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 3. Settle                                                           │
//! │    Wait for seeder hardware initialization                          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 4. Bridge Pin + FLR Disable                                         │
//! │    Pin ancestor bridge power, disable FLR for warm swap             │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 5. Warm Swap                                                        │
//! │    unbind seeder (teardown NOP'd) → driver_override → bind vfio    │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 6. Tier Classification                                              │
//! │    BAR0 register probes → SovereignTier determination               │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 7. Module Cleanup                                                   │
//! │    rmmod patched module (if we loaded it), delete /tmp/.ko          │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

mod config;
mod errors;
mod lock;
mod module_deps;
mod pipeline;
mod pri_recovery;
mod rollback;
mod rm_trigger;
mod runtime_probe;
mod steps;
#[cfg(test)]
mod tests;
mod types;

pub use errors::HandoffError;
pub use types::{HandoffCapabilityProfile, HandoffConfig, HandoffResult, HandoffStep, ModuleSourceConfig, RmChannelEvidence};
pub use pipeline::{
    PipelineSignal, execute_handoff, execute_handoff_with_heartbeat,
    execute_handoff_with_signals,
};
pub use runtime_probe::{RuntimeServicesProbe, probe_runtime_services};
