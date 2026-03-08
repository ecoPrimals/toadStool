// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::composition_constraints::{Constraint, ConstraintSatisfaction};
use crate::fractal_integration::FractalRuntime;
use crate::layer_adaptation::{AdaptedCapabilities, StorageType};
use tracing::debug;

use super::estimators;

pub(crate) fn evaluate_constraint(
    runtime: &FractalRuntime,
    capabilities: &AdaptedCapabilities,
    constraint: &Constraint,
) -> ConstraintSatisfaction {
    match constraint {
        Constraint::RequiresGPU => evaluate_requires_gpu(runtime),
        Constraint::PrefersGPU => evaluate_prefers_gpu(runtime),
        Constraint::MinMemoryGB(required_gb) => evaluate_min_memory_gb(capabilities, *required_gb),
        Constraint::MinCPUCores(required_cores) => {
            evaluate_min_cpu_cores(capabilities, *required_cores)
        }
        Constraint::MaxLatencyMs(max_ms) => evaluate_max_latency_ms(runtime, *max_ms),
        Constraint::PreferredLatencyMs(preferred_ms) => {
            evaluate_preferred_latency_ms(runtime, *preferred_ms)
        }
        Constraint::MinBandwidthGbps(required_gbps) => {
            evaluate_min_bandwidth_gbps(capabilities, *required_gbps)
        }
        Constraint::PreferredBandwidthGbps(preferred_gbps) => {
            evaluate_preferred_bandwidth_gbps(capabilities, *preferred_gbps)
        }
        Constraint::RequiresCapability(cap) => evaluate_requires_capability(capabilities, cap),
        Constraint::PrefersCapability(cap) => evaluate_prefers_capability(capabilities, cap),
        Constraint::MustBeLocal => evaluate_must_be_local(runtime),
        Constraint::PreferLocal => evaluate_prefer_local(runtime),
        Constraint::RequiresLayer(required_layer) => {
            evaluate_requires_layer(runtime, required_layer)
        }
        Constraint::PrefersLayer(preferred_layer) => {
            evaluate_prefers_layer(runtime, preferred_layer)
        }
        Constraint::RequiresPersistentStorage => evaluate_requires_persistent_storage(capabilities),
        Constraint::MaxCostPerHour(max_cost) => evaluate_max_cost_per_hour(runtime, *max_cost),
        Constraint::MinimizeCost => evaluate_minimize_cost(runtime),
        Constraint::Custom { name, hard, value } => evaluate_custom(name, *hard, value),
    }
}

fn evaluate_requires_gpu(runtime: &FractalRuntime) -> ConstraintSatisfaction {
    if runtime.has_gpu_access() {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: "No GPU access available".to_string(),
        }
    }
}

fn evaluate_prefers_gpu(runtime: &FractalRuntime) -> ConstraintSatisfaction {
    if runtime.has_direct_gpu_access() {
        ConstraintSatisfaction::Satisfied
    } else if runtime.has_gpu_access() {
        ConstraintSatisfaction::Partial(0.7)
    } else {
        ConstraintSatisfaction::Partial(0.0)
    }
}

fn evaluate_min_memory_gb(
    capabilities: &AdaptedCapabilities,
    required_gb: f64,
) -> ConstraintSatisfaction {
    if let Some(available_bytes) = capabilities.compute.memory_bytes {
        #[expect(
            clippy::cast_precision_loss,
            reason = "integer count to f64 acceptable"
        )]
        let available_gb = available_bytes as f64 / 1_073_741_824.0;
        if available_gb >= required_gb {
            ConstraintSatisfaction::Satisfied
        } else {
            ConstraintSatisfaction::Unsatisfied {
                reason: format!(
                    "Insufficient memory: need {required_gb}GB, have {available_gb:.2}GB"
                ),
            }
        }
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: "Memory information unavailable".to_string(),
        }
    }
}

fn evaluate_min_cpu_cores(
    capabilities: &AdaptedCapabilities,
    required_cores: usize,
) -> ConstraintSatisfaction {
    if let Some(available_cores) = capabilities.compute.cpu_cores {
        if available_cores >= required_cores {
            ConstraintSatisfaction::Satisfied
        } else {
            ConstraintSatisfaction::Unsatisfied {
                reason: format!(
                    "Insufficient CPU cores: need {required_cores}, have {available_cores}"
                ),
            }
        }
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: "CPU information unavailable".to_string(),
        }
    }
}

fn evaluate_max_latency_ms(runtime: &FractalRuntime, max_ms: u64) -> ConstraintSatisfaction {
    let estimated_latency = estimators::estimate_latency_ms(runtime);
    if estimated_latency <= max_ms {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: format!("Latency too high: need <{max_ms}ms, estimated {estimated_latency}ms"),
        }
    }
}

fn evaluate_preferred_latency_ms(
    runtime: &FractalRuntime,
    preferred_ms: u64,
) -> ConstraintSatisfaction {
    let estimated_latency = estimators::estimate_latency_ms(runtime);
    if estimated_latency <= preferred_ms {
        ConstraintSatisfaction::Satisfied
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "integer count to f64 acceptable"
        )]
        let ratio = preferred_ms as f64 / estimated_latency as f64;
        ConstraintSatisfaction::Partial(ratio.min(1.0))
    }
}

fn evaluate_min_bandwidth_gbps(
    capabilities: &AdaptedCapabilities,
    required_gbps: f64,
) -> ConstraintSatisfaction {
    let estimated_bandwidth = estimators::estimate_bandwidth_gbps(capabilities);
    if estimated_bandwidth >= required_gbps {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: format!(
                "Insufficient bandwidth: need {required_gbps}Gbps, estimated {estimated_bandwidth}Gbps"
            ),
        }
    }
}

fn evaluate_preferred_bandwidth_gbps(
    capabilities: &AdaptedCapabilities,
    preferred_gbps: f64,
) -> ConstraintSatisfaction {
    let estimated_bandwidth = estimators::estimate_bandwidth_gbps(capabilities);
    if estimated_bandwidth >= preferred_gbps {
        ConstraintSatisfaction::Satisfied
    } else {
        let ratio = estimated_bandwidth / preferred_gbps;
        ConstraintSatisfaction::Partial(ratio.min(1.0))
    }
}

fn evaluate_requires_capability(
    capabilities: &AdaptedCapabilities,
    cap: &str,
) -> ConstraintSatisfaction {
    let has_cap = capabilities.to_capability_list().iter().any(|c| c == cap);

    if has_cap {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: format!("Required capability '{cap}' not available"),
        }
    }
}

fn evaluate_prefers_capability(
    capabilities: &AdaptedCapabilities,
    cap: &str,
) -> ConstraintSatisfaction {
    let has_cap = capabilities.to_capability_list().iter().any(|c| c == cap);

    if has_cap {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Partial(0.5)
    }
}

fn evaluate_must_be_local(runtime: &FractalRuntime) -> ConstraintSatisfaction {
    let layer_str = runtime.deployment_layer().to_string();
    if !layer_str.contains("Cloud") {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: "Running in cloud, but must be local".to_string(),
        }
    }
}

fn evaluate_prefer_local(runtime: &FractalRuntime) -> ConstraintSatisfaction {
    let layer_str = runtime.deployment_layer().to_string();
    if !layer_str.contains("Cloud") {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Partial(0.3)
    }
}

fn evaluate_requires_layer(
    runtime: &FractalRuntime,
    required_layer: &str,
) -> ConstraintSatisfaction {
    let current_layer = runtime.deployment_layer().to_string();
    if current_layer.contains(required_layer) {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: format!("Wrong layer: need '{required_layer}', have '{current_layer}'"),
        }
    }
}

fn evaluate_prefers_layer(
    runtime: &FractalRuntime,
    preferred_layer: &str,
) -> ConstraintSatisfaction {
    let current_layer = runtime.deployment_layer().to_string();
    if current_layer.contains(preferred_layer) {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Partial(0.5)
    }
}

fn evaluate_requires_persistent_storage(
    capabilities: &AdaptedCapabilities,
) -> ConstraintSatisfaction {
    let has_persistent = !matches!(
        capabilities.storage.storage_type,
        StorageType::HostFilesystem
    );

    if has_persistent {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: "No persistent storage available".to_string(),
        }
    }
}

fn evaluate_max_cost_per_hour(runtime: &FractalRuntime, max_cost: f64) -> ConstraintSatisfaction {
    let estimated_cost = estimators::estimate_cost_per_hour(runtime);
    if estimated_cost <= max_cost {
        ConstraintSatisfaction::Satisfied
    } else {
        ConstraintSatisfaction::Unsatisfied {
            reason: format!("Too expensive: need <${max_cost}/hr, estimated ${estimated_cost}/hr"),
        }
    }
}

fn evaluate_minimize_cost(runtime: &FractalRuntime) -> ConstraintSatisfaction {
    let cost = estimators::estimate_cost_per_hour(runtime);
    let score = 1.0 / (1.0 + cost);
    ConstraintSatisfaction::Partial(score)
}

fn evaluate_custom(name: &str, hard: bool, value: &str) -> ConstraintSatisfaction {
    debug!(
        "Custom constraint '{}' = '{}' (hard: {})",
        name, value, hard
    );
    ConstraintSatisfaction::Satisfied
}
