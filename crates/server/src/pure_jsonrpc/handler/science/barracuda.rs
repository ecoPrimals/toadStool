// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::pure_jsonrpc::types::JsonRpcError;
use toadstool_common::interned_strings::capabilities;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

const BARRACUDA_ACTIVATION_FUNCTIONS: &[&str] = &[
    "sigmoid",
    "relu",
    "gelu",
    "swish",
    "mish",
    "softplus",
    "leaky_relu",
];

const BARRACUDA_SPECIAL_FUNCTIONS: &[&str] = &[
    "tridiagonal_ql",
    "anderson_diagonalize",
    "plasma_dispersion_z",
    "plasma_dispersion_w",
    "hill_dose_response",
    "population_pk_monte_carlo",
];

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_activations_list() -> JsonRpcResult {
    Ok(serde_json::json!({
        "activations": BARRACUDA_ACTIVATION_FUNCTIONS,
        "batch_variants": ["sigmoid_batch", "relu_batch", "gelu_batch", "swish_batch"],
        "precision": "f64",
        "provider": capabilities::ACTIVATIONS,
        "domain": "science",
    }))
}

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_rng_capabilities() -> JsonRpcResult {
    Ok(serde_json::json!({
        "cpu_prng": {
            "lcg": {
                "function": "rng.lcg_step",
                "algorithm": "Knuth TAOCP Vol 2 LCG",
                "output": "u64",
            },
            "uniform_f64": {
                "function": "rng.uniform_f64_sequence",
                "range": "[0.0, 1.0)",
            },
        },
        "gpu_prng": {
            "xoshiro128ss": {
                "shader": "prng_xoshiro_wgsl",
                "modes": ["f32", "f64"],
            },
        },
        "domain": "science",
    }))
}

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_special_functions() -> JsonRpcResult {
    Ok(serde_json::json!({
        "functions": BARRACUDA_SPECIAL_FUNCTIONS,
        "categories": {
            "eigensolver": ["tridiagonal_ql", "anderson_diagonalize"],
            "plasma_physics": ["plasma_dispersion_z", "plasma_dispersion_w"],
            "pharmacology": ["hill_dose_response", "population_pk_monte_carlo"],
        },
        "provider": capabilities::SPECIAL_FUNCTIONS,
        "domain": "science",
    }))
}
