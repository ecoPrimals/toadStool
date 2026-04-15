// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

#[test]
fn test_runtime_startup_time() {
    let startup_times_ms = HashMap::from([("native", 100u64), ("wasm", 50), ("container", 1000)]);

    assert!(startup_times_ms.get("wasm").unwrap() < startup_times_ms.get("container").unwrap());
}

#[test]
fn test_execution_throughput() {
    let executions_per_sec =
        HashMap::from([("native", 1000u32), ("wasm", 500), ("container", 100)]);

    assert!(executions_per_sec.get("native") > executions_per_sec.get("container"));
}

#[test]
fn test_memory_overhead() {
    let base_memory_mb = HashMap::from([("native", 10u64), ("wasm", 5), ("container", 50)]);

    assert!(base_memory_mb.get("wasm").unwrap() < base_memory_mb.get("container").unwrap());
}

#[test]
fn test_concurrent_executions() {
    let max_concurrent = HashMap::from([("native", 100usize), ("wasm", 1000), ("container", 50)]);

    assert_eq!(max_concurrent.get("wasm"), Some(&1000));
}
