// SPDX-License-Identifier: AGPL-3.0-or-later
//! NPY loading helpers

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::{Benchmark, Error, Result};

use super::dataset::{Dataset, DatasetSplit};
use super::sample::Sample;

/// Load a single NPY sample
pub(super) fn load_npy_sample(
    path: &Path,
    label_map: &std::collections::HashMap<String, usize>,
) -> Result<Option<Sample>> {
    // Extract label from filename (e.g., user01_gesture05_sample01.npy -> gesture05 -> 5)
    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // Try to extract gesture class from filename
    let label = filename
        .split('_')
        .find(|p| p.starts_with("gesture"))
        .map_or_else(
            || label_map.get(filename).copied().unwrap_or(0),
            |gesture_part| {
                gesture_part
                    .strip_prefix("gesture")
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0)
            },
        );

    // Read NPY file (simplified - real impl would parse NPY header)
    let data =
        std::fs::read(path).map_err(|e| Error::DataLoad(format!("Cannot read NPY file: {e}")))?;

    // Skip NPY header (simplified - assumes standard header)
    let header_end = data.iter().position(|&b| b == b'\n').unwrap_or(0) + 1;
    let input = if header_end < data.len() {
        data[header_end..].to_vec()
    } else {
        data
    };

    Ok(Some(Sample {
        input,
        label,
        id: Some(filename.to_string()),
    }))
}

/// Load NPY dataset (multiple samples in one file)
pub(super) fn load_npy_dataset(benchmark: Benchmark, path: &Path) -> Result<Dataset> {
    use tracing::info;

    info!("Loading NPY dataset from {}", path.display());

    let mut file =
        File::open(path).map_err(|e| Error::DataLoad(format!("Cannot open NPY file: {e}")))?;

    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| Error::DataLoad(format!("Cannot read NPY file: {e}")))?;

    // Simplified NPY parsing - real implementation would parse header properly
    // For now, generate synthetic samples based on file content
    let input_shape = benchmark.input_shape();
    let sample_size: usize = input_shape[1..].iter().product();

    let num_samples = data.len() / sample_size.max(1);
    let num_samples = num_samples.min(1000); // Cap at 1000 for testing

    info!("Found {} samples in NPY file", num_samples);

    let samples: Vec<Sample> = (0..num_samples)
        .map(|i| {
            let start = i * sample_size;
            let end = (start + sample_size).min(data.len());
            Sample {
                input: data[start..end].to_vec(),
                label: i % benchmark.num_classes(),
                id: Some(format!("npy_sample_{i}")),
            }
        })
        .collect();

    Ok(Dataset {
        benchmark,
        samples,
        split: DatasetSplit::Test,
    })
}
