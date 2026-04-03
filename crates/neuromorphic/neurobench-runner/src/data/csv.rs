// SPDX-License-Identifier: AGPL-3.0-only
//! CSV time series loading

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{Benchmark, Error, Result};

use super::dataset::{Dataset, DatasetSplit};
use super::sample::Sample;

/// Load CSV time series dataset
pub(super) fn load_csv_timeseries(benchmark: Benchmark, path: &Path) -> Result<Dataset> {
    use tracing::info;

    info!("Loading CSV timeseries from {}", path.display());

    let file =
        File::open(path).map_err(|e| Error::DataLoad(format!("Cannot open CSV file: {e}")))?;

    let reader = BufReader::new(file);
    let mut samples = Vec::new();
    let input_shape = benchmark.input_shape();
    let window_size = input_shape.get(1).copied().unwrap_or(100);
    let features = input_shape.get(2).copied().unwrap_or(1);

    let mut all_data: Vec<Vec<f32>> = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| Error::DataLoad(e.to_string()))?;
        let values: Vec<f32> = line
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if values.len() >= features {
            all_data.push(values);
        }
    }

    // Create sliding window samples
    for (i, window) in all_data.windows(window_size).enumerate() {
        let input: Vec<f32> = window.iter().flatten().copied().collect();
        samples.push(Sample::from_f32(&input, 0, Some(format!("ts_{i}"))));
    }

    info!("Created {} time series samples", samples.len());

    Ok(Dataset {
        benchmark,
        samples,
        split: DatasetSplit::Test,
    })
}
