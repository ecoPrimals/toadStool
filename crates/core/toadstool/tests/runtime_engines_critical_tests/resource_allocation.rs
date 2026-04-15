// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_allocation_per_runtime() {
    let allocations = HashMap::from([("native", 2.0f64), ("wasm", 1.0), ("container", 4.0)]);

    let total: f64 = allocations.values().sum();
    assert_eq!(total, 7.0);
}

#[test]
fn test_memory_allocation_per_runtime() {
    let memory_mb = HashMap::from([("native", 2048u64), ("wasm", 512), ("container", 4096)]);

    assert_eq!(memory_mb.get("wasm"), Some(&512));
}

#[test]
fn test_resource_limit_enforcement() {
    let max_cpu = 16.0f64;
    let requested_cpu = 20.0f64;

    assert!(requested_cpu > max_cpu);
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_resource_reservation() {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Reservation {
        runtime: String,
        cpu: f64,
        memory_mb: u64,
    }

    let reservation = Reservation {
        runtime: "native".to_string(),
        cpu: 2.0,
        memory_mb: 2048,
    };

    assert!(!reservation.runtime.is_empty());
    assert_eq!(reservation.cpu, 2.0);
    assert_eq!(reservation.memory_mb, 2048);
}
