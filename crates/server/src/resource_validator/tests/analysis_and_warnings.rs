// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::resource_validator::analysis::{generate_warnings, identify_gaps};

use super::helpers::{base_capabilities, base_estimate};

#[test]
fn identify_gaps_cpu_shortage() {
    let mut est = base_estimate();
    est.cpu_cores = 100;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 4;
    let gaps = identify_gaps(&est, &caps);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].resource_type, "cpu_cores");
    assert_eq!(gaps[0].required, 100);
    assert_eq!(gaps[0].available, 4);
    assert_eq!(gaps[0].shortage, 96);
    assert!(gaps[0].suggestion.contains("96"));
}

#[test]
fn identify_gaps_cpu_exact_fit_no_gap() {
    let mut est = base_estimate();
    est.cpu_cores = 8;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 8;
    assert!(identify_gaps(&est, &caps).is_empty());
}

#[test]
fn identify_gaps_memory_shortage() {
    let mut est = base_estimate();
    est.memory_bytes = 64 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_memory_bytes = 8 * 1024 * 1024 * 1024;
    let gaps = identify_gaps(&est, &caps);
    let m = gaps.iter().find(|g| g.resource_type == "memory").unwrap();
    assert_eq!(m.shortage, 56 * 1024 * 1024 * 1024);
    assert!(m.suggestion.contains("56"));
}

#[test]
fn identify_gaps_storage_shortage() {
    let mut est = base_estimate();
    est.storage_bytes = 500 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_storage_bytes = 100 * 1024 * 1024 * 1024;
    let gaps = identify_gaps(&est, &caps);
    assert!(gaps.iter().any(|g| g.resource_type == "storage"));
    let s = gaps.iter().find(|g| g.resource_type == "storage").unwrap();
    assert_eq!(s.shortage, 400 * 1024 * 1024 * 1024);
}

#[test]
fn identify_gaps_gpu_zero_estimate_skips_gpu_branch() {
    let mut est = base_estimate();
    est.gpu_memory_bytes = 0;
    let mut caps = base_capabilities();
    caps.available_gpu_memory_bytes = 0;
    caps.total_gpu_memory_bytes = 0;
    caps.gpu_count = 0;
    caps.gpu_types.clear();
    let gaps = identify_gaps(&est, &caps);
    assert!(!gaps.iter().any(|g| g.resource_type == "gpu_memory"));
}

#[test]
fn identify_gaps_gpu_no_hardware_uses_fallback_suggestion() {
    let mut est = base_estimate();
    est.gpu_memory_bytes = 8 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_gpu_memory_bytes = 0;
    caps.total_gpu_memory_bytes = 0;
    caps.gpu_count = 0;
    caps.gpu_types.clear();
    let gaps = identify_gaps(&est, &caps);
    let g = gaps
        .iter()
        .find(|g| g.resource_type == "gpu_memory")
        .unwrap();
    assert!(g.suggestion.contains("No GPU detected"));
}

#[test]
fn identify_gaps_gpu_shortage_with_gpu_present() {
    let mut est = base_estimate();
    est.gpu_memory_bytes = 32 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_gpu_memory_bytes = 8 * 1024 * 1024 * 1024;
    caps.total_gpu_memory_bytes = 16 * 1024 * 1024 * 1024;
    caps.gpu_count = 1;
    let gaps = identify_gaps(&est, &caps);
    let g = gaps
        .iter()
        .find(|g| g.resource_type == "gpu_memory")
        .unwrap();
    assert!(g.suggestion.contains("quantization") || g.suggestion.contains("GB"));
}

#[test]
fn identify_gaps_multiple_resource_types() {
    let mut est = base_estimate();
    est.cpu_cores = 64;
    est.memory_bytes = 128 * 1024 * 1024 * 1024;
    est.storage_bytes = 2 * 1024 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 2;
    caps.available_memory_bytes = 1024;
    caps.available_storage_bytes = 1024;
    let gaps = identify_gaps(&est, &caps);
    let types: Vec<_> = gaps.iter().map(|g| g.resource_type.as_str()).collect();
    assert!(types.contains(&"cpu_cores"));
    assert!(types.contains(&"memory"));
    assert!(types.contains(&"storage"));
}

// --- generate_warnings ---

#[test]
fn generate_warnings_high_cpu_not_at_exact_70_percent() {
    let mut est = base_estimate();
    est.cpu_cores = 71;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 100;
    let w = generate_warnings(&est, &caps);
    assert_eq!(w.len(), 1);
    assert!(w[0].contains("CPU"));
}

#[test]
fn generate_warnings_cpu_exactly_70_percent_no_warning() {
    let mut est = base_estimate();
    est.cpu_cores = 70;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 100;
    assert!(generate_warnings(&est, &caps).is_empty());
}

#[test]
fn generate_warnings_cpu_overcommit_no_warning() {
    let mut est = base_estimate();
    est.cpu_cores = 200;
    let mut caps = base_capabilities();
    caps.available_cpu_cores = 100;
    assert!(generate_warnings(&est, &caps).is_empty());
}

#[test]
fn generate_warnings_high_memory() {
    let mut est = base_estimate();
    est.memory_bytes = 75 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.available_memory_bytes = 100 * 1024 * 1024;
    let w = generate_warnings(&est, &caps);
    assert!(w.iter().any(|s| s.contains("memory")));
}

#[test]
fn generate_warnings_gpu_branch_skipped_when_no_gpu() {
    let mut est = base_estimate();
    est.gpu_memory_bytes = 8 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.total_gpu_memory_bytes = 0;
    caps.available_gpu_memory_bytes = 0;
    let w = generate_warnings(&est, &caps);
    assert!(!w.iter().any(|s| s.contains("GPU")));
}

#[test]
fn generate_warnings_high_gpu_memory() {
    let mut est = base_estimate();
    est.gpu_memory_bytes = 8 * 1024 * 1024 * 1024;
    let mut caps = base_capabilities();
    caps.total_gpu_memory_bytes = 10 * 1024 * 1024 * 1024;
    caps.available_gpu_memory_bytes = 10 * 1024 * 1024 * 1024;
    let w = generate_warnings(&est, &caps);
    assert!(w.iter().any(|s| s.contains("GPU")));
}

#[test]
fn generate_warnings_high_storage_above_80_percent() {
    let mut est = base_estimate();
    est.storage_bytes = 900;
    let mut caps = base_capabilities();
    caps.available_storage_bytes = 1000;
    let w = generate_warnings(&est, &caps);
    assert!(
        w.iter()
            .any(|s| s.contains("storage") || s.contains("Storage"))
    );
}

#[test]
fn generate_warnings_storage_at_80_percent_no_warning() {
    let mut est = base_estimate();
    est.storage_bytes = 800;
    let mut caps = base_capabilities();
    caps.available_storage_bytes = 1000;
    assert!(
        !generate_warnings(&est, &caps)
            .iter()
            .any(|s| s.contains("storage") || s.contains("Storage"))
    );
}

#[test]
fn generate_warnings_storage_over_100_percent_no_warning() {
    let mut est = base_estimate();
    est.storage_bytes = 2000;
    let mut caps = base_capabilities();
    caps.available_storage_bytes = 1000;
    assert!(
        !generate_warnings(&est, &caps)
            .iter()
            .any(|s| s.contains("cleanup"))
    );
}
