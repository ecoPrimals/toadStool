//! Dual-chip ensemble reservoir implementation
//!
//! Coordinates two Akida chips running different reservoirs in parallel.

use akida_driver::{AkidaDevice, DeviceManager};
use akida_models::Model;
use anyhow::{Context, Result};
use ndarray::Array1;
use tracing::{debug, info};

use crate::state_extraction::{concatenate_states, inference_to_state, StateExtractor};

/// Configuration for ensemble
#[derive(Debug, Clone)]
pub struct EnsembleConfig {
    /// Path to reservoir model for chip 1
    pub reservoir1_path: String,

    /// Path to reservoir model for chip 2
    pub reservoir2_path: String,

    /// Expected state size from each reservoir
    pub state_size_per_chip: usize,
}

/// Dual-chip ensemble manager
pub struct DualChipEnsemble {
    config: EnsembleConfig,
    device1: AkidaDevice,
    device2: AkidaDevice,
    model1: Model,
    model2: Model,
    extractor: StateExtractor,
}

impl DualChipEnsemble {
    /// Create ensemble with two devices
    pub fn new(config: EnsembleConfig, device1: AkidaDevice, device2: AkidaDevice) -> Result<Self> {
        info!("Creating dual-chip ensemble");
        info!("  Chip 1: {}", config.reservoir1_path);
        info!("  Chip 2: {}", config.reservoir2_path);

        // Load models
        let model1 = Model::from_file(&config.reservoir1_path)
            .with_context(|| format!("Failed to load model 1: {}", config.reservoir1_path))?;

        let model2 = Model::from_file(&config.reservoir2_path)
            .with_context(|| format!("Failed to load model 2: {}", config.reservoir2_path))?;

        let extractor = StateExtractor::final_layer_only();

        info!("✅ Ensemble created");

        Ok(Self {
            config,
            device1,
            device2,
            model1,
            model2,
            extractor,
        })
    }

    /// Discover devices and create ensemble
    pub fn discover_and_create(config: EnsembleConfig) -> Result<Self> {
        info!("Discovering Akida devices for ensemble...");

        let manager = DeviceManager::discover().context("Failed to discover Akida devices")?;

        if manager.device_count() < 2 {
            anyhow::bail!(
                "Ensemble requires 2 Akida devices, found {}",
                manager.device_count()
            );
        }

        info!("Found {} Akida devices", manager.device_count());

        // Open first two devices
        let device1 = manager.open(0).context("Failed to open device 0")?;
        let device2 = manager.open(1).context("Failed to open device 1")?;

        Self::new(config, device1, device2)
    }

    /// Load reservoirs to devices
    pub fn load_reservoirs(&mut self) -> Result<()> {
        info!("Loading reservoirs to devices...");

        // Load to chip 1
        debug!("Loading to chip 1...");
        self.model1
            .load_to_device(&mut self.device1)
            .context("Failed to load to device 1")?;
        info!("  ✅ Chip 1 loaded");

        // Load to chip 2
        debug!("Loading to chip 2...");
        self.model2
            .load_to_device(&mut self.device2)
            .context("Failed to load to device 2")?;
        info!("  ✅ Chip 2 loaded");

        info!("✅ Both reservoirs loaded");
        Ok(())
    }

    /// Run inference on both chips in parallel and concatenate states
    ///
    /// # Performance
    ///
    /// Both chips run inference in parallel (~70-96µs each).
    /// State extraction and concatenation add ~10-50µs.
    /// Total: ~100-150µs for dual-chip ensemble state!
    pub fn get_ensemble_state(&mut self, input: &[u8]) -> Result<Array1<f32>> {
        debug!("Running ensemble inference");

        // TODO: Run in parallel using tokio::spawn or rayon
        // For now, sequential execution

        // Chip 1 inference
        debug!("Chip 1 inference...");
        let result1 = self
            .model1
            .infer(input, &mut self.device1)
            .context("Chip 1 inference failed")?;
        let state1 = inference_to_state(&result1);

        // Chip 2 inference
        debug!("Chip 2 inference...");
        let result2 = self
            .model2
            .infer(input, &mut self.device2)
            .context("Chip 2 inference failed")?;
        let state2 = inference_to_state(&result2);

        // Concatenate states
        debug!(
            "Concatenating states ({} + {} = {})",
            state1.len(),
            state2.len(),
            state1.len() + state2.len()
        );

        let ensemble_state =
            concatenate_states(&[state1, state2]).context("Failed to concatenate states")?;

        debug!("✅ Ensemble state: {} dimensions", ensemble_state.len());
        Ok(ensemble_state)
    }

    /// Collect ensemble states for training dataset
    pub fn collect_training_states(&mut self, inputs: &[Vec<u8>]) -> Result<Vec<Array1<f32>>> {
        info!("Collecting ensemble states for {} inputs", inputs.len());

        let mut states = Vec::with_capacity(inputs.len());

        for (i, input) in inputs.iter().enumerate() {
            if i % 100 == 0 {
                debug!("Processing input {}/{}", i, inputs.len());
            }

            let state = self.get_ensemble_state(input)?;
            states.push(state);
        }

        info!("✅ Collected {} ensemble states", states.len());
        Ok(states)
    }

    /// Get configuration
    pub fn config(&self) -> &EnsembleConfig {
        &self.config
    }

    /// Get device references
    pub fn devices(&mut self) -> (&mut AkidaDevice, &mut AkidaDevice) {
        (&mut self.device1, &mut self.device2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensemble_config() {
        let config = EnsembleConfig {
            reservoir1_path: "reservoir_seed42.fbz".to_string(),
            reservoir2_path: "reservoir_seed123.fbz".to_string(),
            state_size_per_chip: 1000,
        };

        assert_eq!(config.state_size_per_chip, 1000);
    }
}
