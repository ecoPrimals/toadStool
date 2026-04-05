// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Additional type tests (serde, config, etc.)

mod basic_types;
mod capacity_broadcast_misc;
mod config_serde;
mod helpers;
mod job_plan_serde;
mod job_splitting_coordination;
mod node_network_serde;
