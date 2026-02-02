//! State extraction from Akida NPU layers
//!
//! **CRITICAL RESEARCH COMPONENT**: This module tests whether we can extract
//! internal NPU layer activations, which is essential for reservoir computing.

use akida_driver::InferenceResult;
use akida_models::Model;
use anyhow::{Context, Result};
use ndarray::Array1;
use tracing::{debug, info, warn};

/// Layer activations extracted from NPU
#[derive(Debug, Clone)]
pub struct LayerActivations {
    /// Layer index
    pub layer_idx: usize,

    /// Activation values (flattened)
    pub values: Vec<f32>,

    /// Shape of activations (if known)
    pub shape: Option<Vec<usize>>,
}

/// State extractor for Akida models
pub struct StateExtractor {
    /// Target layer indices to extract
    layer_indices: Vec<usize>,
}

impl StateExtractor {
    /// Create extractor for specific layers
    pub fn new(layer_indices: Vec<usize>) -> Self {
        info!("Creating state extractor for layers: {:?}", layer_indices);
        Self { layer_indices }
    }

    /// Extract all layers
    pub fn all_layers(num_layers: usize) -> Self {
        let layer_indices = (0..num_layers).collect();
        Self::new(layer_indices)
    }

    /// Extract only final layer
    pub fn final_layer_only() -> Self {
        Self::new(vec![0]) // Will be updated when we know layer count
    }

    /// RESEARCH QUESTION 1: Can we extract internal layer states?
    ///
    /// This is THE critical function for reservoir computing feasibility.
    ///
    /// According to BrainChip documentation, the Akida SDK has:
    /// - `model.get_layer(idx)` - retrieve specific layer
    /// - Layer activations can be accessed
    ///
    /// We need to verify this works in our pure Rust driver.
    pub fn extract_states(
        &self,
        _model: &Model,
        result: &InferenceResult,
    ) -> Result<Vec<LayerActivations>> {
        debug!("Attempting to extract layer states");

        // EXPERIMENTAL: Try to extract internal states
        //
        // The BrainChip Python SDK has methods like:
        //   - model.forward(input, layer=N) - get output at layer N
        //   - model.predict(input) - get all layer outputs
        //
        // We need to add similar functionality to our Rust driver!

        // For now, we can only access the final output
        warn!("⚠️  State extraction not yet implemented in pure Rust driver!");
        warn!("    We can currently only access final inference output.");
        warn!("    Need to extend akida-driver to expose internal layer states.");

        // Return final output as a single "layer"
        let final_values: Vec<f32> = result.output.iter().map(|&x| x as f32).collect();

        let final_layer = LayerActivations {
            layer_idx: 0,
            values: final_values,
            shape: None, // Unknown without model introspection
        };

        info!("Extracted final layer: {} values", final_layer.values.len());

        Ok(vec![final_layer])
    }

    /// Extract state as ndarray for easier computation
    pub fn extract_as_array(
        &self,
        model: &Model,
        result: &InferenceResult,
    ) -> Result<Vec<Array1<f32>>> {
        let states = self.extract_states(model, result)?;

        states
            .into_iter()
            .map(|layer| Array1::from_vec(layer.values))
            .map(Ok)
            .collect()
    }

    /// RESEARCH TODO: Extend akida-driver to support layer introspection
    ///
    /// To properly implement reservoir computing, we need to add:
    ///
    /// 1. In `akida-driver/src/inference.rs`:
    ///    - Add `get_layer_output(layer_idx)` method
    ///    - Extend ioctl interface to query internal NPU states
    ///
    /// 2. In `akida-models/src/model.rs`:
    ///    - Add `layer_info()` method to expose layer metadata
    ///    - Parse layer shapes from .fbz file
    ///
    /// 3. In `akida-driver/src/io.rs`:
    ///    - Add ioctl commands for layer state extraction
    ///    - Map NPU memory regions for each layer
    ///
    /// This is FEASIBLE because:
    /// - BrainChip SDK already exposes this functionality
    /// - Akida hardware maintains layer states in NPU SRAM
    /// - We just need to add the correct ioctl interface
    pub fn research_notes() -> &'static str {
        r#"
RESEARCH STATUS: State Extraction

Current Limitations:
  ❌ Can only access final output (not internal layers)
  ❌ No layer introspection in pure Rust driver yet
  ❌ Missing ioctl interface for NPU memory access

What We Need to Add:
  1. Layer metadata parsing from .fbz files
  2. ioctl commands to read NPU layer states
  3. Memory mapping for internal activations

Evidence This Is Possible:
  ✅ BrainChip Python SDK has this functionality
  ✅ Akida hardware stores layer states in NPU SRAM
  ✅ Kernel driver likely supports this (need to verify)

Next Steps:
  1. Research Akida kernel driver source (if available)
  2. Reverse engineer Python SDK layer access
  3. Add ioctl definitions to our Rust driver
  4. Implement layer state extraction

Estimated Effort: 2-4 weeks of driver development
Feasibility: HIGH (hardware supports it, just need driver work)
        "#
    }
}

/// Helper to convert inference output to reservoir state
pub fn inference_to_state(result: &InferenceResult) -> Array1<f32> {
    let values: Vec<f32> = result.output.iter().map(|&x| x as f32).collect();

    Array1::from_vec(values)
}

/// Concatenate states from multiple reservoirs (for ensemble)
pub fn concatenate_states(states: &[Array1<f32>]) -> Result<Array1<f32>> {
    if states.is_empty() {
        anyhow::bail!("Cannot concatenate empty state list");
    }

    let total_size: usize = states.iter().map(|s| s.len()).sum();
    let mut concatenated = Vec::with_capacity(total_size);

    for state in states {
        concatenated.extend_from_slice(state.as_slice().context("Failed to get state slice")?);
    }

    Ok(Array1::from_vec(concatenated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concatenate_states() {
        let state1 = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let state2 = Array1::from_vec(vec![4.0, 5.0]);

        let result = concatenate_states(&[state1, state2]).unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(result[0], 1.0);
        assert_eq!(result[4], 5.0);
    }

    #[test]
    fn test_research_notes() {
        let notes = StateExtractor::research_notes();
        assert!(notes.contains("State Extraction"));
        assert!(notes.contains("Feasibility: HIGH"));
    }
}
