// SPDX-License-Identifier: AGPL-3.0-or-later
//! Property tests: JSON serde round-trip for core workload/resource types (via `toadstool-core` strategies).

use proptest::prelude::*;
use toadstool::proptest_strategies::{arb_resource_requirements, arb_workload_type};

proptest! {
    #[test]
    fn workload_type_json_roundtrip(wt in arb_workload_type()) {
        let json = serde_json::to_string(&wt).unwrap();
        let back: toadstool::WorkloadType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(wt, back);
    }

    #[test]
    fn resource_requirements_json_roundtrip(req in arb_resource_requirements()) {
        let json = serde_json::to_string(&req).unwrap();
        let back: toadstool::ResourceRequirements = serde_json::from_str(&json).unwrap();
        let near = |a: f64, b: f64| (a - b).abs() < f64::EPSILON * 4096.0;
        prop_assert!(near(req.cpu.min_cores, back.cpu.min_cores));
        match (req.cpu.max_cores, back.cpu.max_cores) {
            (Some(a), Some(b)) => prop_assert!(near(a, b)),
            (None, None) => {}
            _ => prop_assert!(false, "max_cores optionality mismatch"),
        }
        prop_assert_eq!(req.cpu.architecture, back.cpu.architecture);
        prop_assert_eq!(req.memory.min_bytes, back.memory.min_bytes);
        prop_assert_eq!(req.memory.max_bytes, back.memory.max_bytes);
        prop_assert_eq!(req.storage.min_bytes, back.storage.min_bytes);
        prop_assert_eq!(req.storage.max_bytes, back.storage.max_bytes);
        prop_assert_eq!(req.storage.storage_type, back.storage.storage_type);
        prop_assert_eq!(req.network.min_bandwidth, back.network.min_bandwidth);
        prop_assert_eq!(req.network.max_bandwidth, back.network.max_bandwidth);
        prop_assert_eq!(req.network.max_latency_ms, back.network.max_latency_ms);
        prop_assert_eq!(format!("{:?}", req.gpu), format!("{:?}", back.gpu));
    }
}
