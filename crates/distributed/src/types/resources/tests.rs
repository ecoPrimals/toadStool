// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use proptest::prelude::*;

fn arb_resource_allocation() -> impl Strategy<Value = ResourceAllocation> {
    (
        // Use integer-ish cpu_cores: JSON roundtrip can change float representation
        (1u32..1024u32).prop_map(|n| n as f64),
        (1024u64..(1u64 << 40)), // 1KB .. 1TB
        (1024u64..(1u64 << 40)),
        (100u64..(1u64 << 30)), // 100 B/s .. 1 GB/s
        prop::option::of(((0u32..16), (1024u64..(1u64 << 30)), (1u32..128))),
        // Exclude Float from custom_resources: JSON roundtrip can change float representation
        prop::collection::hash_map(
            "[a-z_]{1,20}",
            prop_oneof![
                any::<i64>().prop_map(ResourceValue::Integer),
                "[a-zA-Z0-9_]{0,50}".prop_map(ResourceValue::String),
                any::<bool>().prop_map(ResourceValue::Boolean),
            ],
            0..5,
        ),
    )
        .prop_map(
            |(cpu_cores, memory_bytes, storage_bytes, network_bandwidth, gpu, custom_resources)| {
                ResourceAllocation {
                    cpu_cores,
                    memory_bytes,
                    storage_bytes,
                    network_bandwidth,
                    gpu_allocation: gpu.map(|(device_id, memory_bytes, compute_units)| {
                        GpuAllocation {
                            device_id,
                            memory_bytes,
                            compute_units,
                        }
                    }),
                    custom_resources,
                }
            },
        )
}

fn arb_backoff_strategy() -> impl Strategy<Value = BackoffStrategy> {
    prop_oneof![
        (1u64..60_000u64).prop_map(|delay_ms| BackoffStrategy::Fixed { delay_ms }),
        ((100u64..5_000u64), (50u64..2_000u64)).prop_map(|(initial_ms, increment_ms)| {
            BackoffStrategy::Linear {
                initial_ms,
                increment_ms,
            }
        }),
        ((100u64..10_000u64), (1_000u64..60_000u64))
            .prop_map(|(base_ms, max_ms)| BackoffStrategy::Exponential { base_ms, max_ms }),
        ((100u64..10_000u64), (1_000u64..60_000u64)).prop_map(|(base_ms, max_ms)| {
            BackoffStrategy::ExponentialJittered { base_ms, max_ms }
        }),
    ]
}

fn arb_network_config() -> impl Strategy<Value = NetworkConfig> {
    (
        ((1u16..65535), (1u16..65535)).prop_filter("port_range ordered", |(a, b)| a <= b),
        prop_oneof![
            Just(NetworkSecurityLevel::Low),
            Just(NetworkSecurityLevel::Medium),
            Just(NetworkSecurityLevel::High),
            Just(NetworkSecurityLevel::Maximum),
        ],
        prop::collection::vec("[a-z]{4,10}", 0..5),
    )
        .prop_map(|(port_range, security_level, protocols)| NetworkConfig {
            port_range,
            security_level,
            protocols,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_resource_allocation_json_roundtrip(alloc in arb_resource_allocation()) {
        let json = serde_json::to_string(&alloc).unwrap();
        let restored: ResourceAllocation = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(alloc, restored);
    }

    #[test]
    fn prop_backoff_strategy_json_roundtrip(strategy in arb_backoff_strategy()) {
        let json = serde_json::to_string(&strategy).unwrap();
        let restored: BackoffStrategy = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(format!("{strategy:?}"), format!("{restored:?}"));
    }

    #[test]
    fn prop_network_config_json_roundtrip(config in arb_network_config()) {
        let json = serde_json::to_string(&config).unwrap();
        let restored: NetworkConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config.port_range, restored.port_range);
        prop_assert_eq!(format!("{:?}", config.security_level), format!("{:?}", restored.security_level));
        prop_assert_eq!(config.protocols, restored.protocols);
    }
}

#[test]
fn resource_requirements_default_validation() {
    let req = ResourceRequirements::default();
    assert!((req.cpu.min_cores - 1.0).abs() < f64::EPSILON);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024);
    assert!(req.network.bandwidth_mbps.is_none());
    assert!(req.gpu.is_none());
}

#[test]
fn cpu_requirements_construction() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(8.0),
    };
    assert!((cpu.min_cores - 4.0).abs() < f64::EPSILON);
    assert_eq!(cpu.max_cores, Some(8.0));
}

#[test]
fn memory_requirements_construction() {
    let mem = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,
        max_bytes: Some(16 * 1024 * 1024 * 1024),
    };
    assert_eq!(mem.min_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn storage_requirements_construction() {
    let st = StorageRequirements {
        min_bytes: 10 * 1024 * 1024 * 1024,
        max_bytes: None,
    };
    assert_eq!(st.min_bytes, 10 * 1024 * 1024 * 1024);
}

#[test]
fn gpu_requirements_construction() {
    let gpu = GpuRequirements {
        min_memory_gb: 8.0,
        compute_capability: Some("8.0".to_string()),
    };
    assert!((gpu.min_memory_gb - 8.0).abs() < f64::EPSILON);
    assert_eq!(gpu.compute_capability.as_deref(), Some("8.0"));
}

#[test]
fn resource_requirements_to_from_core() {
    let distributed = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: Some(4.0),
        },
        memory: MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 20 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: Some(100),
            latency_ms: Some(50),
        },
        gpu: Some(GpuRequirements {
            min_memory_gb: 4.0,
            compute_capability: Some("7.5".to_string()),
        }),
    };
    let core_req: toadstool::resources::ResourceRequirements = distributed.clone().into();
    let back: ResourceRequirements = core_req.into();
    assert!((back.cpu.min_cores - distributed.cpu.min_cores).abs() < f64::EPSILON);
    assert_eq!(back.memory.min_bytes, distributed.memory.min_bytes);
    assert_eq!(back.gpu.as_ref().map(|g| g.min_memory_gb), Some(4.0));
}

#[test]
fn distributed_retry_config_default() {
    let config = DistributedRetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert!(!config.retry_conditions.is_empty());
}

#[test]
fn resource_allocation_default() {
    let alloc = ResourceAllocation::default();
    assert!((alloc.cpu_cores - 1.0).abs() < f64::EPSILON);
    assert_eq!(alloc.memory_bytes, 1024 * 1024 * 1024);
    assert_eq!(alloc.storage_bytes, 10 * 1024 * 1024 * 1024);
    assert!(alloc.gpu_allocation.is_none());
}

#[test]
fn network_config_default() {
    let config = NetworkConfig::default();
    assert_eq!(config.port_range.0, 8000);
    assert_eq!(config.port_range.1, 9000);
    assert!(matches!(
        config.security_level,
        NetworkSecurityLevel::Medium
    ));
}

#[test]
fn resource_limits_default() {
    let limits = ResourceLimits::default();
    assert!((limits.max_cpu_cores - 4.0).abs() < f64::EPSILON);
    assert_eq!(limits.max_memory_bytes, 8 * 1024 * 1024 * 1024);
}

#[test]
fn resource_value_variants() {
    let _i = ResourceValue::Integer(42);
    let _f = ResourceValue::Float(3.5_f64);
    let _s = ResourceValue::String("test".to_string());
    let _b = ResourceValue::Boolean(true);
}

#[test]
fn gpu_allocation_construction() {
    let alloc = GpuAllocation {
        device_id: 0,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        compute_units: 40,
    };
    assert_eq!(alloc.device_id, 0);
    assert_eq!(alloc.memory_bytes, 8 * 1024 * 1024 * 1024);
}
