// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-benchmark dataset loaders

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{Benchmark, Error, Result};
use tracing::{debug, info, warn};

use super::csv::load_csv_timeseries;
use super::dataset::{Dataset, DatasetSplit};
use super::npy::{load_npy_dataset, load_npy_sample};
use super::sample::Sample;

/// Load DVS Gesture dataset
///
/// Expected structure:
/// ```text
/// dvs_gesture/
///   test/
///     user01_gesture01_sample01.npy
///     ...
///   labels.txt  (class name -> label mapping)
/// ```
pub(super) fn load_dvs_gesture(path: &Path) -> Result<Dataset> {
    let test_dir = path.join("test");
    if !test_dir.exists() {
        // Try loading single NPY file
        let npy_path = path.join("dvs_gesture_test.npy");
        if npy_path.exists() {
            return load_npy_dataset(Benchmark::DvsGesture, &npy_path);
        }
        warn!("DVS Gesture test directory not found, using synthetic data");
        return Ok(Dataset::synthetic(Benchmark::DvsGesture, 1000));
    }

    let mut samples = Vec::new();
    let label_map = load_label_map(path)?;

    for entry in std::fs::read_dir(&test_dir)
        .map_err(|e| Error::DataLoad(format!("Cannot read test dir: {e}")))?
    {
        let entry = entry.map_err(|e| Error::DataLoad(e.to_string()))?;
        let file_path = entry.path();

        if file_path.extension().is_some_and(|ext| ext == "npy")
            && let Some(sample) = load_npy_sample(&file_path, &label_map)?
        {
            samples.push(sample);
        }
    }

    if samples.is_empty() {
        warn!("No samples found in DVS Gesture dataset, using synthetic data");
        return Ok(Dataset::synthetic(Benchmark::DvsGesture, 1000));
    }

    info!("Loaded {} DVS Gesture samples", samples.len());

    Ok(Dataset {
        benchmark: Benchmark::DvsGesture,
        samples,
        split: DatasetSplit::Test,
    })
}

/// Load Keyword FSCIL dataset (Google Speech Commands style)
pub(super) fn load_keyword_fscil(path: &Path) -> Result<Dataset> {
    // Check for MFCC features file
    let mfcc_path = path.join("kws_mfcc_test.npy");
    if mfcc_path.exists() {
        return load_npy_dataset(Benchmark::KeywordFscil, &mfcc_path);
    }

    // Check for raw audio directory
    let test_dir = path.join("test");
    if !test_dir.exists() {
        warn!("Keyword FSCIL test directory not found, using synthetic data");
        return Ok(Dataset::synthetic(Benchmark::KeywordFscil, 1000));
    }

    // Load MFCC features from directory structure
    // speech_commands/
    //   yes/
    //     sample_001.wav
    //   no/
    //     sample_001.wav
    let mut samples = Vec::new();
    let classes: Vec<_> = std::fs::read_dir(&test_dir)
        .map_err(|e| Error::DataLoad(e.to_string()))?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();

    for (label, class_dir) in classes.iter().enumerate() {
        let class_name = class_dir.file_name();
        debug!("Loading class {:?} (label {})", class_name, label);

        for entry in std::fs::read_dir(class_dir.path())
            .map_err(|e| Error::DataLoad(e.to_string()))?
            .filter_map(std::result::Result::ok)
        {
            // For now, load raw bytes - real implementation would compute MFCC
            if let Ok(data) = std::fs::read(entry.path()) {
                samples.push(Sample {
                    input: data,
                    label,
                    id: Some(entry.path().to_string_lossy().to_string()),
                });
            }
        }
    }

    if samples.is_empty() {
        warn!("No samples found in Keyword FSCIL dataset, using synthetic data");
        return Ok(Dataset::synthetic(Benchmark::KeywordFscil, 1000));
    }

    info!("Loaded {} Keyword FSCIL samples", samples.len());

    Ok(Dataset {
        benchmark: Benchmark::KeywordFscil,
        samples,
        split: DatasetSplit::Test,
    })
}

/// Load chaotic function prediction dataset
pub(super) fn load_chaotic(path: &Path) -> Result<Dataset> {
    let csv_path = path.join("lorenz_test.csv");
    if csv_path.exists() {
        return load_csv_timeseries(Benchmark::ChaoticFunction, &csv_path);
    }

    let npy_path = path.join("mackey_glass_test.npy");
    if npy_path.exists() {
        return load_npy_dataset(Benchmark::ChaoticFunction, &npy_path);
    }

    warn!("Chaotic function dataset not found, using synthetic data");
    Ok(Dataset::synthetic(Benchmark::ChaoticFunction, 1000))
}

/// Load NHP Motor prediction dataset
pub(super) fn load_nhp_motor(path: &Path) -> Result<Dataset> {
    let npy_path = path.join("nhp_motor_test.npy");
    if npy_path.exists() {
        return load_npy_dataset(Benchmark::NhpMotor, &npy_path);
    }

    warn!("NHP Motor dataset not found, using synthetic data");
    Ok(Dataset::synthetic(Benchmark::NhpMotor, 1000))
}

/// Load Event Camera dataset
pub(super) fn load_event_camera(path: &Path) -> Result<Dataset> {
    let npy_path = path.join("event_camera_test.npy");
    if npy_path.exists() {
        return load_npy_dataset(Benchmark::EventCamera, &npy_path);
    }

    warn!("Event Camera dataset not found, using synthetic data");
    Ok(Dataset::synthetic(Benchmark::EventCamera, 500))
}

/// Load label mapping from labels.txt
fn load_label_map(path: &Path) -> Result<std::collections::HashMap<String, usize>> {
    let labels_path = path.join("labels.txt");
    let mut map = std::collections::HashMap::new();

    if labels_path.exists() {
        let file = File::open(&labels_path)
            .map_err(|e| Error::DataLoad(format!("Cannot open labels.txt: {e}")))?;

        for (idx, line) in BufReader::new(file).lines().enumerate() {
            if let Ok(label_name) = line {
                let label_name = label_name.trim().to_string();
                if !label_name.is_empty() {
                    map.insert(label_name, idx);
                }
            }
        }
    }

    // Default DVS Gesture classes if no labels.txt
    if map.is_empty() {
        for (idx, gesture) in [
            "hand_clapping",
            "right_hand_wave",
            "left_hand_wave",
            "right_arm_clockwise",
            "right_arm_counter_clockwise",
            "left_arm_clockwise",
            "left_arm_counter_clockwise",
            "arm_roll",
            "air_drums",
            "air_guitar",
            "other",
        ]
        .iter()
        .enumerate()
        {
            map.insert(gesture.to_string(), idx);
        }
    }

    Ok(map)
}
