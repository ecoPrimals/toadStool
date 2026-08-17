// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security context and policies for `ToadStool` workloads

mod provider;

pub use provider::SecurityProvider;
pub use toadstool_core::security::*;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
