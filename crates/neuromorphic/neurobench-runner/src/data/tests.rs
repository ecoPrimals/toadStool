// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::Benchmark;
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
