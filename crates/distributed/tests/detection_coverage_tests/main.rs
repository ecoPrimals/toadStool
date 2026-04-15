// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    deprecated,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]
//! Integration tests targeting `universal::detection` behavior and substrate types produced by detection.
//!
//! Exercises `UniversalSubstrateCapabilities::detect_all` and serde, `Debug`, `Clone`, and helper APIs on
//! substrate enums and `UniversalSubstrateCapabilities` used by the detection pipeline.

mod common;
mod language_runtimes;
mod neuromorphic_quantum_edge_iot;
mod operating_systems;
mod specialized_architectures_experimental;
mod traditional_container_biological;
mod universal_substrate_capabilities;
