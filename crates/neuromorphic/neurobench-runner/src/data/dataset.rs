// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`Dataset`] container and loading dispatch

use std::path::Path;

use crate::{Benchmark, Result};
use tracing::{info, warn};

use super::benchmarks::{
    load_chaotic, load_dvs_gesture, load_event_camera, load_keyword_fscil, load_nhp_motor,
};
use super::sample::Sample;

/// Dataset for a benchmark
pub struct Dataset {
    /// Benchmark type
    pub benchmark: Benchmark,
    /// All samples
    pub samples: Vec<Sample>,
    /// Split type (train/val/test)
    pub split: DatasetSplit,
}

/// Dataset split type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatasetSplit {
    /// Training partition for model fitting.
    Train,
    /// Validation partition for hyperparameter tuning.
    Validation,
    /// Test partition for final evaluation.
    #[default]
    Test,
}

impl Dataset {
    /// Load dataset from directory
    ///
    /// Searches for standard `NeuroBench` dataset formats:
    /// - `{benchmark}_test.npy` - `NumPy` format
    /// - `{benchmark}_test.csv` - CSV format
    /// - `{benchmark}/test/` - Directory with samples
    ///
    /// # Errors
    ///
    /// Returns `Err` if the path cannot be read, NPY/CSV parsing fails, or I/O errors occur.
    pub fn load<P: AsRef<Path>>(benchmark: Benchmark, path: P) -> Result<Self> {
        let path = path.as_ref();
        info!(
            "Loading {} dataset from {}",
            benchmark.description(),
            path.display()
        );

        if !path.exists() {
            warn!("Dataset path not found: {}", path.display());
            info!("Generating synthetic data for testing");
            return Ok(Self::synthetic(benchmark, 1000));
        }

        // Try different loading strategies based on benchmark type
        match benchmark {
            Benchmark::DvsGesture => load_dvs_gesture(path),
            Benchmark::KeywordFscil => load_keyword_fscil(path),
            Benchmark::ChaoticFunction => load_chaotic(path),
            Benchmark::NhpMotor => load_nhp_motor(path),
            Benchmark::EventCamera => load_event_camera(path),
        }
    }

    /// Generate synthetic dataset for testing
    #[must_use]
    pub fn synthetic(benchmark: Benchmark, num_samples: usize) -> Self {
        let input_shape = benchmark.input_shape();
        let input_size: usize = input_shape.iter().product();
        let num_classes = benchmark.num_classes();

        let samples: Vec<Sample> = (0..num_samples)
            .map(|i| {
                // Generate pseudo-random input based on index (0..256 fits in u8)
                let input: Vec<u8> = (0..input_size)
                    .map(|j| u8::try_from((i * 17 + j * 13) % 256).unwrap_or(0))
                    .collect();

                Sample {
                    input,
                    label: i % num_classes,
                    id: Some(format!("synthetic_{i}")),
                }
            })
            .collect();

        Self {
            benchmark,
            samples,
            split: DatasetSplit::Test,
        }
    }

    /// Number of samples
    #[must_use]
    pub const fn len(&self) -> usize {
        self.samples.len()
    }

    /// Is dataset empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Iterate over samples
    pub fn iter(&self) -> impl Iterator<Item = &Sample> {
        self.samples.iter()
    }

    /// Get a batch of samples
    #[must_use]
    pub fn batch(&self, start: usize, size: usize) -> &[Sample] {
        let end = (start + size).min(self.samples.len());
        &self.samples[start..end]
    }
}
