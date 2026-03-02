//! JSON-RPC handlers for `ai.nautilus.*` namespace.
//!
//! Exposes ToadStool's standalone evolutionary reservoir computing via JSON-RPC.
//! Uses barracuda's nautilus module (CPU-only, no GPU dep).

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use barracuda::nautilus::{BetaObservation, NautilusBrain, NautilusBrainConfig};

/// Shared nautilus brain state (thread-safe).
pub type NautilusBrainState = Arc<RwLock<NautilusBrain>>;

/// Create a new default `NautilusBrain` wrapped for concurrent access.
pub fn create_brain(instance_name: &str) -> NautilusBrainState {
    Arc::new(RwLock::new(NautilusBrain::new(
        NautilusBrainConfig::default(),
        instance_name,
    )))
}

pub(super) struct NautilusRpcError {
    pub code: i32,
    pub message: String,
}

impl NautilusRpcError {
    fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: toadstool_common::constants::jsonrpc::error_codes::INVALID_PARAMS,
            message: msg.into(),
        }
    }
    fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: toadstool_common::constants::jsonrpc::error_codes::INTERNAL_ERROR,
            message: msg.into(),
        }
    }
}

/// `ai.nautilus.status` — brain status (observations, trained, drifting).
pub async fn handle_status(brain: &NautilusBrainState) -> Result<Value, NautilusRpcError> {
    let b = brain.read().await;
    Ok(json!({
        "observation_count": b.observations.len(),
        "trained": b.trained,
        "drifting": b.drift.is_drifting(),
        "generations": b.shell.history.len(),
        "population_size": b.config.shell_config.pop_size,
        "lineage": b.shell.lineage.iter().map(|id| &id.0).collect::<Vec<_>>(),
    }))
}

/// `ai.nautilus.observe` — feed a physics observation.
pub async fn handle_observe(
    brain: &NautilusBrainState,
    params: &Value,
) -> Result<Value, NautilusRpcError> {
    let beta = params["beta"]
        .as_f64()
        .ok_or_else(|| NautilusRpcError::invalid_params("missing f64 'beta'"))?;
    let plaquette = params["plaquette"].as_f64().unwrap_or(0.0);
    let cg_iters = params["cg_iters"].as_f64().unwrap_or(0.0);
    let acceptance = params["acceptance"].as_f64().unwrap_or(0.0);
    let delta_h_abs = params["delta_h_abs"].as_f64().unwrap_or(0.0);

    let obs = BetaObservation {
        beta,
        plaquette,
        cg_iters,
        acceptance,
        delta_h_abs,
        quenched_plaq: params["quenched_plaq"].as_f64(),
        quenched_plaq_var: params["quenched_plaq_var"].as_f64(),
        anderson_r: params["anderson_r"].as_f64(),
        anderson_lambda_min: params["anderson_lambda_min"].as_f64(),
    };

    let mut b = brain.write().await;
    b.observe(obs);
    Ok(json!({ "observation_count": b.observations.len() }))
}

/// `ai.nautilus.train` — evolve the shell on accumulated observations.
pub async fn handle_train(brain: &NautilusBrainState) -> Result<Value, NautilusRpcError> {
    let mut b = brain.write().await;
    match b.train() {
        Some(mse) => Ok(json!({
            "mse": mse,
            "generations": b.shell.history.len(),
            "trained": true,
        })),
        None => Ok(json!({
            "trained": false,
            "reason": format!(
                "need at least {} observations (have {})",
                b.config.min_observations,
                b.observations.len()
            ),
        })),
    }
}

/// `ai.nautilus.predict` — predict dynamical observables for a beta value.
pub async fn handle_predict(
    brain: &NautilusBrainState,
    params: &Value,
) -> Result<Value, NautilusRpcError> {
    let beta = params["beta"]
        .as_f64()
        .ok_or_else(|| NautilusRpcError::invalid_params("missing f64 'beta'"))?;
    let quenched_plaq = params["quenched_plaq"].as_f64();

    let b = brain.read().await;
    match b.predict_dynamical(beta, quenched_plaq) {
        Some((cg, plaq, acc)) => Ok(json!({
            "beta": beta,
            "cg_iters": cg,
            "plaquette": plaq,
            "acceptance": acc,
        })),
        None => Ok(json!({
            "beta": beta,
            "error": "not trained or prediction failed",
        })),
    }
}

/// `ai.nautilus.screen` — score candidate beta values by information content.
pub async fn handle_screen(
    brain: &NautilusBrainState,
    params: &Value,
) -> Result<Value, NautilusRpcError> {
    let betas: Vec<f64> = params["betas"]
        .as_array()
        .ok_or_else(|| NautilusRpcError::invalid_params("missing array 'betas'"))?
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    let b = brain.read().await;
    let scored = b.screen_candidates(&betas);
    Ok(json!({
        "candidates": scored.iter().map(|(beta, score)| json!({"beta": beta, "score": score})).collect::<Vec<_>>(),
    }))
}

/// `ai.nautilus.edges` — detect concept edges via LOO analysis.
pub async fn handle_edges(brain: &NautilusBrainState) -> Result<Value, NautilusRpcError> {
    let mut b = brain.write().await;
    let edges = b.detect_concept_edges();
    Ok(json!({
        "edges": edges.iter().map(|(beta, err)| json!({"beta": beta, "error": err})).collect::<Vec<_>>(),
        "count": edges.len(),
    }))
}

/// `ai.nautilus.shell.export` — serialize shell to JSON.
pub async fn handle_shell_export(brain: &NautilusBrainState) -> Result<Value, NautilusRpcError> {
    let b = brain.read().await;
    let json_str = b
        .to_json()
        .map_err(|e| NautilusRpcError::internal(e.to_string()))?;
    Ok(json!({ "shell_json": json_str }))
}

/// `ai.nautilus.shell.import` — restore brain from serialized JSON.
pub async fn handle_shell_import(
    brain: &NautilusBrainState,
    params: &Value,
) -> Result<Value, NautilusRpcError> {
    let shell_json = params["shell_json"]
        .as_str()
        .ok_or_else(|| NautilusRpcError::invalid_params("missing string 'shell_json'"))?;

    let restored = NautilusBrain::from_json(shell_json)
        .map_err(|e| NautilusRpcError::invalid_params(e.to_string()))?;

    let mut b = brain.write().await;
    *b = restored;
    Ok(json!({
        "imported": true,
        "observation_count": b.observations.len(),
        "trained": b.trained,
    }))
}
