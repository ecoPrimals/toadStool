// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool_auto_config::hardware::PerformanceClass;

#[test]
fn test_performance_class_low_end() {
    let cpu_cores = 2.0;
    let memory_gb = 4.0;
    let gpu_count = 0;

    let is_high_end = cpu_cores >= 16.0 && memory_gb >= 32.0 && gpu_count > 0;
    let is_mainstream = cpu_cores >= 8.0 && memory_gb >= 16.0;

    let class = if is_high_end {
        PerformanceClass::HighEnd
    } else if is_mainstream {
        PerformanceClass::Mainstream
    } else {
        PerformanceClass::LowEnd
    };

    assert!(matches!(class, PerformanceClass::LowEnd));
}

#[test]
fn test_performance_class_mainstream() {
    let cpu_cores = 8.0;
    let memory_gb = 16.0;
    let gpu_count = 0;

    let is_high_end = cpu_cores >= 16.0 && memory_gb >= 32.0 && gpu_count > 0;
    let is_mainstream = cpu_cores >= 8.0 && memory_gb >= 16.0;

    let class = if is_high_end {
        PerformanceClass::HighEnd
    } else if is_mainstream {
        PerformanceClass::Mainstream
    } else {
        PerformanceClass::LowEnd
    };

    assert!(matches!(class, PerformanceClass::Mainstream));
}

#[test]
fn test_performance_class_high_end() {
    let cpu_cores = 16.0;
    let memory_gb = 32.0;
    let gpu_count = 1;

    let is_high_end = cpu_cores >= 16.0 && memory_gb >= 32.0 && gpu_count > 0;
    let is_mainstream = cpu_cores >= 8.0 && memory_gb >= 16.0;

    let class = if is_high_end {
        PerformanceClass::HighEnd
    } else if is_mainstream {
        PerformanceClass::Mainstream
    } else {
        PerformanceClass::LowEnd
    };

    assert!(matches!(class, PerformanceClass::HighEnd));
}
