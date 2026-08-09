// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security context and policies for `ToadStool` workloads

mod provider;

pub use toadstool_core::security::*;
pub use provider::SecurityProvider;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
