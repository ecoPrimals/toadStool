// SPDX-License-Identifier: AGPL-3.0-only

use crate::fractal_integration::FractalRuntime;
use crate::layer_adaptation::{AdaptedCapabilities, NetworkAccess};

pub(crate) fn estimate_latency_ms(runtime: &FractalRuntime) -> u64 {
    let layer_str = runtime.deployment_layer().to_string();

    if layer_str.contains("BareMetalOS") {
        1
    } else if layer_str.contains("Container") {
        5
    } else if layer_str.contains("VM") {
        10
    } else if layer_str.contains("Cloud") {
        50
    } else {
        20
    }
}

pub(crate) fn estimate_bandwidth_gbps(capabilities: &AdaptedCapabilities) -> f64 {
    match capabilities.network.network_access {
        NetworkAccess::Direct => 100.0,
        NetworkAccess::HostNamespace => 40.0,
        NetworkAccess::CloudVPC => 10.0,
    }
}

pub(crate) fn estimate_cost_per_hour(runtime: &FractalRuntime) -> f64 {
    let layer_str = runtime.deployment_layer().to_string();

    if layer_str.contains("BareMetalOS") || layer_str.contains("Middleware") {
        0.0
    } else if layer_str.contains("Container") {
        0.01
    } else if layer_str.contains("VM") {
        0.10
    } else if layer_str.contains("Cloud") {
        if runtime.has_gpu_access() {
            5.00
        } else {
            0.50
        }
    } else {
        0.10
    }
}
