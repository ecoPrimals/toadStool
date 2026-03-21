// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `UniversalScheduler` and `ResourceCoordinator`
//!
//! Organised by semantic domain to keep each module focused and under 300 lines.
//! Shared mock types and factory helpers live in `helpers`.

mod universal_scheduler_tests {
    pub mod capabilities;
    pub mod coordinator;
    pub mod execution;
    pub mod helpers;
    pub mod priority;
    pub mod resources;
    pub mod routing;
    pub mod scheduling;
}
