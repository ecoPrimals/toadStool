// SPDX-License-Identifier: AGPL-3.0-or-later
//! Science domain handlers for JSON-RPC.
//!
//! Routes scientific compute through toadStool's workload infrastructure.
//! Springs (wetSpring, airSpring, hotSpring, etc.) call these methods to request
//! GPU/NPU compute without coupling to barraCuda directly.

mod barracuda;
mod compute;
mod gpu;
mod npu;
mod substrate;

pub(super) use super::science_domains::{
    deploy_capability_call, deploy_graph_status, discovery_direct_rpc, discovery_primal_health,
    discovery_primals, discovery_topology, ecology_offload,
};
pub(super) use barracuda::{
    science_activations_list, science_rng_capabilities, science_special_functions,
};
pub(super) use compute::{
    science_compute_cancel, science_compute_result, science_compute_status, science_compute_submit,
};
pub(super) use gpu::{science_gpu_capabilities, science_gpu_dispatch};
pub(super) use npu::{science_npu_capabilities, science_npu_dispatch};
pub(super) use substrate::{science_substrate_discover, science_substrate_probe};

#[cfg(test)]
mod tests;
