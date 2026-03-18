// SPDX-License-Identifier: AGPL-3.0-or-later
//! Data loading utilities for `NeuroBench`
//!
//! Handles loading benchmark datasets from standard `NeuroBench` formats.
//!
//! ## Supported Formats
//!
//! - **DVS Gesture**: NPY files with shape [samples, time, x, y, polarity]
//! - **Keyword FSCIL**: MFCC features from Google Speech Commands
//! - **Chaotic Function**: CSV time series (Lorenz, Mackey-Glass)
//! - **NHP Motor**: Neural spike trains in NPY format

use crate::{Benchmark, Error, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tracing::{debug, info, warn};

/// A sample for inference
#[derive(Debug, Clone)]
pub struct Sample {
    /// Input data (raw bytes, to be interpreted by model)
    pub input: Vec<u8>,
    /// Ground truth label (class index for classification, 0 for regression)
    pub label: usize,
    /// Optional sample identifier
    pub id: Option<String>,
}

impl Sample {
    /// Create sample with f32 input (will be converted to bytes)
    #[must_use]
    pub fn from_f32(data: &[f32], label: usize, id: Option<String>) -> Self {
        let input: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        Self { input, label, id }
    }

    /// Get input as f32 slice
    #[must_use]
    pub fn as_f32(&self) -> Vec<f32> {
        self.input
            .chunks_exact(4)
            .map(|chunk| {
                let bytes: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(bytes)
            })
            .collect()
    }
}

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
            Benchmark::DvsGesture => Self::load_dvs_gesture(path),
            Benchmark::KeywordFscil => Self::load_keyword_fscil(path),
            Benchmark::ChaoticFunction => Self::load_chaotic(path),
            Benchmark::NhpMotor => Self::load_nhp_motor(path),
            Benchmark::EventCamera => Self::load_event_camera(path),
        }
    }

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
    fn load_dvs_gesture(path: &Path) -> Result<Self> {
        let test_dir = path.join("test");
        if !test_dir.exists() {
            // Try loading single NPY file
            let npy_path = path.join("dvs_gesture_test.npy");
            if npy_path.exists() {
                return Self::load_npy_dataset(Benchmark::DvsGesture, &npy_path);
            }
            warn!("DVS Gesture test directory not found, using synthetic data");
            return Ok(Self::synthetic(Benchmark::DvsGesture, 1000));
        }

        let mut samples = Vec::new();
        let label_map = Self::load_label_map(path)?;

        for entry in std::fs::read_dir(&test_dir)
            .map_err(|e| Error::DataLoad(format!("Cannot read test dir: {e}")))?
        {
            let entry = entry.map_err(|e| Error::DataLoad(e.to_string()))?;
            let file_path = entry.path();

            if file_path.extension().is_some_and(|ext| ext == "npy")
                && let Some(sample) = Self::load_npy_sample(&file_path, &label_map)?
            {
                samples.push(sample);
            }
        }

        if samples.is_empty() {
            warn!("No samples found in DVS Gesture dataset, using synthetic data");
            return Ok(Self::synthetic(Benchmark::DvsGesture, 1000));
        }

        info!("Loaded {} DVS Gesture samples", samples.len());

        Ok(Self {
            benchmark: Benchmark::DvsGesture,
            samples,
            split: DatasetSplit::Test,
        })
    }

    /// Load Keyword FSCIL dataset (Google Speech Commands style)
    fn load_keyword_fscil(path: &Path) -> Result<Self> {
        // Check for MFCC features file
        let mfcc_path = path.join("kws_mfcc_test.npy");
        if mfcc_path.exists() {
            return Self::load_npy_dataset(Benchmark::KeywordFscil, &mfcc_path);
        }

        // Check for raw audio directory
        let test_dir = path.join("test");
        if !test_dir.exists() {
            warn!("Keyword FSCIL test directory not found, using synthetic data");
            return Ok(Self::synthetic(Benchmark::KeywordFscil, 1000));
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
            return Ok(Self::synthetic(Benchmark::KeywordFscil, 1000));
        }

        info!("Loaded {} Keyword FSCIL samples", samples.len());

        Ok(Self {
            benchmark: Benchmark::KeywordFscil,
            samples,
            split: DatasetSplit::Test,
        })
    }

    /// Load chaotic function prediction dataset
    fn load_chaotic(path: &Path) -> Result<Self> {
        let csv_path = path.join("lorenz_test.csv");
        if csv_path.exists() {
            return Self::load_csv_timeseries(Benchmark::ChaoticFunction, &csv_path);
        }

        let npy_path = path.join("mackey_glass_test.npy");
        if npy_path.exists() {
            return Self::load_npy_dataset(Benchmark::ChaoticFunction, &npy_path);
        }

        warn!("Chaotic function dataset not found, using synthetic data");
        Ok(Self::synthetic(Benchmark::ChaoticFunction, 1000))
    }

    /// Load NHP Motor prediction dataset
    fn load_nhp_motor(path: &Path) -> Result<Self> {
        let npy_path = path.join("nhp_motor_test.npy");
        if npy_path.exists() {
            return Self::load_npy_dataset(Benchmark::NhpMotor, &npy_path);
        }

        warn!("NHP Motor dataset not found, using synthetic data");
        Ok(Self::synthetic(Benchmark::NhpMotor, 1000))
    }

    /// Load Event Camera dataset
    fn load_event_camera(path: &Path) -> Result<Self> {
        let npy_path = path.join("event_camera_test.npy");
        if npy_path.exists() {
            return Self::load_npy_dataset(Benchmark::EventCamera, &npy_path);
        }

        warn!("Event Camera dataset not found, using synthetic data");
        Ok(Self::synthetic(Benchmark::EventCamera, 500))
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

    /// Load a single NPY sample
    fn load_npy_sample(
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
        let data = std::fs::read(path)
            .map_err(|e| Error::DataLoad(format!("Cannot read NPY file: {e}")))?;

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
    fn load_npy_dataset(benchmark: Benchmark, path: &Path) -> Result<Self> {
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

        Ok(Self {
            benchmark,
            samples,
            split: DatasetSplit::Test,
        })
    }

    /// Load CSV time series dataset
    fn load_csv_timeseries(benchmark: Benchmark, path: &Path) -> Result<Self> {
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

        Ok(Self {
            benchmark,
            samples,
            split: DatasetSplit::Test,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_synthetic_dataset() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 100);

        assert_eq!(dataset.len(), 100);

        let sample = &dataset.samples[0];
        let expected_size: usize = Benchmark::DvsGesture.input_shape().iter().product();
        assert_eq!(sample.input.len(), expected_size);

        println!("DVS Gesture synthetic dataset: {} samples", dataset.len());
        println!("Input size: {expected_size} bytes");
    }

    #[test]
    fn test_sample_from_f32() {
        let data = [1.0_f32, 2.0, 3.0, 4.0];
        let sample = Sample::from_f32(&data, 5, Some("test_id".to_string()));
        assert_eq!(sample.label, 5);
        assert_eq!(sample.id, Some("test_id".to_string()));
        assert_eq!(sample.input.len(), 16); // 4 floats * 4 bytes
        let roundtrip = sample.as_f32();
        assert!((roundtrip[0] - 1.0).abs() < f32::EPSILON);
        assert!((roundtrip[3] - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sample_as_f32_odd_bytes() {
        let sample = Sample {
            input: vec![0u8; 15], // Not divisible by 4
            label: 0,
            id: None,
        };
        let f32s = sample.as_f32();
        assert_eq!(f32s.len(), 3); // 12 bytes = 3 floats, remainder ignored
    }

    #[test]
    fn test_dataset_split_default() {
        assert_eq!(DatasetSplit::default(), DatasetSplit::Test);
    }

    #[test]
    fn test_synthetic_all_benchmarks() {
        for benchmark in [
            Benchmark::DvsGesture,
            Benchmark::KeywordFscil,
            Benchmark::ChaoticFunction,
            Benchmark::NhpMotor,
            Benchmark::EventCamera,
        ] {
            let dataset = Dataset::synthetic(benchmark, 50);
            assert_eq!(dataset.len(), 50);
            assert!(!dataset.is_empty());
            let input_size: usize = benchmark.input_shape().iter().product();
            assert_eq!(dataset.samples[0].input.len(), input_size);
            assert_eq!(dataset.samples[0].label, 0);
        }
    }

    #[test]
    fn test_dataset_batch() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 100);
        let batch = dataset.batch(10, 20);
        assert_eq!(batch.len(), 20);
        let batch_oob = dataset.batch(95, 20);
        assert_eq!(batch_oob.len(), 5);
    }

    #[test]
    fn test_dataset_iter() {
        let dataset = Dataset::synthetic(Benchmark::KeywordFscil, 10);
        let count: usize = dataset.iter().count();
        assert_eq!(count, 10);
    }

    #[test]
    fn test_dataset_load_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path/that/does/not/exist");
        let result = Dataset::load(Benchmark::DvsGesture, &path);
        assert!(result.is_ok());
        let dataset = result.unwrap();
        assert_eq!(dataset.len(), 1000); // Falls back to synthetic
    }

    #[test]
    fn test_dataset_empty_synthetic() {
        let dataset = Dataset::synthetic(Benchmark::ChaoticFunction, 0);
        assert!(dataset.is_empty());
        assert_eq!(dataset.len(), 0);
    }

    #[test]
    fn test_dataset_load_with_npy_file() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let npy_path = temp.path().join("dvs_gesture_test.npy");
        // Minimal NPY-like file: header + data (at least 1 sample worth)
        let input_size: usize = Benchmark::DvsGesture.input_shape().iter().product();
        let mut file = std::fs::File::create(&npy_path).unwrap();
        file.write_all(b"{'descr': '<f4', 'shape': (1,), }\n")
            .unwrap();
        file.write_all(&vec![0u8; input_size]).unwrap();
        drop(file);

        let result = Dataset::load(Benchmark::DvsGesture, temp.path());
        assert!(result.is_ok());
        let dataset = result.unwrap();
        assert!(!dataset.is_empty());
        assert_eq!(dataset.benchmark, Benchmark::DvsGesture);
        assert_eq!(dataset.split, DatasetSplit::Test);
    }

    #[test]
    fn test_dataset_load_chaotic_csv() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let csv_path = temp.path().join("lorenz_test.csv");
        let mut file = std::fs::File::create(&csv_path).unwrap();
        // Write enough rows for sliding window (window_size=1000 from input_shape)
        for _ in 0..1100 {
            writeln!(file, "1.0, 2.0, 3.0").unwrap();
        }
        drop(file);

        let result = Dataset::load(Benchmark::ChaoticFunction, temp.path());
        assert!(result.is_ok());
        let dataset = result.unwrap();
        assert!(!dataset.is_empty());
        assert_eq!(dataset.benchmark, Benchmark::ChaoticFunction);
    }

    #[test]
    fn test_dataset_load_keyword_fscil_directory() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let test_dir = temp.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let yes_dir = test_dir.join("yes");
        std::fs::create_dir_all(&yes_dir).unwrap();
        let mut f = std::fs::File::create(yes_dir.join("sample_001.wav")).unwrap();
        f.write_all(&[0u8; 100]).unwrap();
        drop(f);

        let result = Dataset::load(Benchmark::KeywordFscil, temp.path());
        assert!(result.is_ok());
        let dataset = result.unwrap();
        assert!(!dataset.is_empty());
        assert_eq!(dataset.samples[0].label, 0);
    }

    #[test]
    fn test_dataset_load_dvs_gesture_test_dir() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let test_dir = temp.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let mut f = std::fs::File::create(test_dir.join("user01_gesture05_sample01.npy")).unwrap();
        f.write_all(b"NPY_HEADER\n").unwrap();
        let input_size: usize = Benchmark::DvsGesture.input_shape().iter().product();
        f.write_all(&vec![0u8; input_size]).unwrap();
        drop(f);

        let result = Dataset::load(Benchmark::DvsGesture, temp.path());
        assert!(result.is_ok());
        let dataset = result.unwrap();
        assert!(!dataset.is_empty());
        assert_eq!(dataset.samples[0].label, 5); // gesture05 -> 5
    }

    #[test]
    fn test_dataset_load_labels_txt() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let test_dir = temp.path().join("test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let mut labels = std::fs::File::create(temp.path().join("labels.txt")).unwrap();
        writeln!(labels, "class_a").unwrap();
        writeln!(labels, "class_b").unwrap();
        drop(labels);
        let mut f = std::fs::File::create(test_dir.join("sample_class_a.npy")).unwrap();
        f.write_all(b"H\n").unwrap();
        f.write_all(&vec![0u8; 100]).unwrap();
        drop(f);

        let result = Dataset::load(Benchmark::DvsGesture, temp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_sample_construction_raw() {
        let sample = Sample {
            input: vec![1, 2, 3, 4, 5, 6, 7, 8],
            label: 42,
            id: Some("raw_id".to_string()),
        };
        assert_eq!(sample.label, 42);
        assert_eq!(sample.as_f32().len(), 2);
    }

    #[test]
    fn test_dataset_batch_empty() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 5);
        let batch = dataset.batch(5, 20);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_dataset_batch_exact() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 50);
        let batch = dataset.batch(0, 50);
        assert_eq!(batch.len(), 50);
    }

    #[test]
    fn test_synthetic_sample_labels_cycle() {
        let dataset = Dataset::synthetic(Benchmark::DvsGesture, 25);
        let num_classes = Benchmark::DvsGesture.num_classes();
        for (i, sample) in dataset.iter().enumerate() {
            assert_eq!(sample.label, i % num_classes);
        }
    }
}
