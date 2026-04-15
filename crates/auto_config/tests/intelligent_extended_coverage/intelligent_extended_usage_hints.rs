// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool_auto_config::intelligent::UsageHints;

#[test]
fn test_usage_hints_default() {
    let hints = UsageHints::default();

    assert_eq!(hints.predicted_workload_types.len(), 0);
    assert_eq!(hints.expected_cpu_usage, 0.0);
    assert_eq!(hints.expected_memory_usage, 0.0);
    assert!(!hints.prefers_gpu);
    assert!(!hints.prefers_containers);
}

#[test]
fn test_usage_hints_is_cpu_intensive() {
    let hints = UsageHints {
        expected_cpu_usage: 0.8,
        ..Default::default()
    };

    assert!(hints.is_cpu_intensive());
}

#[test]
fn test_usage_hints_is_not_cpu_intensive() {
    let hints = UsageHints {
        expected_cpu_usage: 0.5,
        ..Default::default()
    };

    assert!(!hints.is_cpu_intensive());
}

#[test]
fn test_usage_hints_is_memory_intensive() {
    let hints = UsageHints {
        expected_memory_usage: 0.8,
        ..Default::default()
    };

    assert!(hints.is_memory_intensive());
}

#[test]
fn test_usage_hints_is_not_memory_intensive() {
    let hints = UsageHints {
        expected_memory_usage: 0.5,
        ..Default::default()
    };

    assert!(!hints.is_memory_intensive());
}

#[test]
fn test_usage_hints_cpu_intensive_threshold() {
    let test_cases = vec![
        (0.6, false),
        (0.7, false),
        (0.71, true),
        (0.8, true),
        (1.0, true),
    ];

    for (cpu_usage, expected) in test_cases {
        let hints = UsageHints {
            expected_cpu_usage: cpu_usage,
            ..Default::default()
        };
        assert_eq!(hints.is_cpu_intensive(), expected, "CPU usage: {cpu_usage}");
    }
}

#[test]
fn test_usage_hints_memory_intensive_threshold() {
    let test_cases = vec![
        (0.6, false),
        (0.7, false),
        (0.71, true),
        (0.8, true),
        (1.0, true),
    ];

    for (memory_usage, expected) in test_cases {
        let hints = UsageHints {
            expected_memory_usage: memory_usage,
            ..Default::default()
        };
        assert_eq!(
            hints.is_memory_intensive(),
            expected,
            "Memory usage: {memory_usage}"
        );
    }
}

#[test]
fn test_usage_hints_clone() {
    let hints = UsageHints {
        predicted_workload_types: vec!["development".to_string()],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.6,
        prefers_gpu: true,
        prefers_containers: false,
    };

    let cloned = hints.clone();
    assert_eq!(
        hints.predicted_workload_types,
        cloned.predicted_workload_types
    );
    assert_eq!(hints.expected_cpu_usage, cloned.expected_cpu_usage);
}
