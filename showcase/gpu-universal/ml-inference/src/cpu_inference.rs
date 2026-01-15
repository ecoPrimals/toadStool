//! CPU inference implementation

use crate::{network::SimpleNetwork, InferenceResult};
use anyhow::Result;
use ndarray::Array1;
use std::time::Instant;

pub struct CpuInference {
    network: SimpleNetwork,
}

impl CpuInference {
    pub fn new(network: SimpleNetwork) -> Self {
        Self { network }
    }

    /// Run inference on single sample
    pub fn infer(&self, input: &Array1<f32>) -> Result<InferenceResult> {
        let start = Instant::now();

        // Forward pass
        let output = self.network.forward_cpu(input)?;

        // Get prediction
        let (predicted_class, confidence) = self.network.predict(&output);

        let latency = start.elapsed();

        Ok(InferenceResult {
            predicted_class,
            confidence,
            all_probabilities: output.to_vec(),
            latency,
            backend: "CPU".to_string(),
        })
    }

    /// Run inference on batch
    pub fn infer_batch(&self, inputs: &[Array1<f32>]) -> Result<Vec<InferenceResult>> {
        inputs.iter().map(|input| self.infer(input)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_inference() {
        let network = SimpleNetwork::new();
        let inference = CpuInference::new(network);

        let input = Array1::from_vec(vec![0.5; 784]);
        let result = inference.infer(&input).unwrap();

        assert!(result.predicted_class < 10);
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);
        assert_eq!(result.all_probabilities.len(), 10);
        assert_eq!(result.backend, "CPU");
    }
}
