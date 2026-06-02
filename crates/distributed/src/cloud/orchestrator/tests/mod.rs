// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cloud orchestrator tests — split by concern for WateringHole line limits

mod common;

mod capacity;
mod config;
mod deployment;
mod internal_branches;

#[path = "../../../../tests/cloud_orchestrator_coverage_tests.rs"]
mod coverage_expansion;
