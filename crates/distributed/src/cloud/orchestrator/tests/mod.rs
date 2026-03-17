// SPDX-License-Identifier: AGPL-3.0-only
//! Cloud orchestrator tests — split by concern for WateringHole line limits

#[expect(
    clippy::float_cmp,
    clippy::module_inception,
    reason = "test module; comparing exact literals"
)]
mod common;

mod capacity;
mod config;
mod deployment;
