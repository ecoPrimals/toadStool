//! Data loading utilities for NeuroBench
//!
//! Handles loading benchmark datasets.

use crate::{Benchmark, Error, Result};
use std::path::Path;
use tracing::info;

/// A sample for inference
#[derive(Debug, Clone)]
pub struct Sample {
    /// Input data
    pub input: Vec<u8>,
    /// Ground truth label
    pub label: usize,
    /// Optional sample identifier
    pub id: Option<String>,
}

/// Dataset for a benchmark
pub struct Dataset {
    /// Benchmark type
    pub benchmark: Benchmark,
    /// All samples
    pub samples: Vec<Sample>,
}

impl Dataset {
    /// Load dataset from directory
    pub fn load<P: AsRef<Path>>(benchmark: Benchmark, path: P) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading {} dataset from {}", benchmark.description(), path.display());
        
        if !path.exists() {
            // Generate synthetic data for testing
            info!("Dataset not found, generating synthetic data");
            return Ok(Self::synthetic(benchmark, 1000));
        }
        
        // In real implementation, would load from:
        // - DVS Gesture: .npy files or H5 format
        // - Keyword FSCIL: MFCC features as .npy
        // - Chaotic: Time series CSV
        // etc.
        
        Err(Error::DataLoad(format!(
            "Dataset loading not yet implemented for {:?}",
            benchmark
        )))
    }
    
    /// Generate synthetic dataset for testing
    pub fn synthetic(benchmark: Benchmark, num_samples: usize) -> Self {
        let input_shape = benchmark.input_shape();
        let input_size: usize = input_shape.iter().product();
        let num_classes = benchmark.num_classes();
        
        let samples: Vec<Sample> = (0..num_samples)
            .map(|i| {
                // Generate pseudo-random input based on index
                let input: Vec<u8> = (0..input_size)
                    .map(|j| ((i * 17 + j * 13) % 256) as u8)
                    .collect();
                
                Sample {
                    input,
                    label: i % num_classes,
                    id: Some(format!("synthetic_{}", i)),
                }
            })
            .collect();
        
        Self { benchmark, samples }
    }
    
    /// Number of samples
    pub fn len(&self) -> usize {
        self.samples.len()
    }
    
    /// Is dataset empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    
    /// Iterate over samples
    pub fn iter(&self) -> impl Iterator<Item = &Sample> {
        self.samples.iter()
    }
    
    /// Get a batch of samples
    pub fn batch(&self, start: usize, size: usize) -> &[Sample] {
        let end = (start + size).min(self.samples.len());
        &self.samples[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_synthetic_dataset() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 100);
        
        assert_eq!(dataset.len(), 100);
        
        let sample = &dataset.samples[0];
        let expected_size: usize = Benchmark::DvsGesture.input_shape().iter().product();
        assert_eq!(sample.input.len(), expected_size);
        
        println!("DVS Gesture synthetic dataset: {} samples", dataset.len());
        println!("Input size: {} bytes", expected_size);
    }
}
