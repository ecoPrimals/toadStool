// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn silicon_unit_all_variants() {
    assert_eq!(SiliconUnit::ALL.len(), 9);
    for unit in &SiliconUnit::ALL {
        assert!(!unit.as_str().is_empty());
    }
}

#[test]
fn shader_core_does_not_require_sovereign() {
    assert!(!SiliconUnit::ShaderCore.requires_sovereign_pipeline());
    assert!(SiliconUnit::ShaderCore.available_via_wgpu_compute());
}

#[test]
fn fixed_function_units_require_sovereign() {
    for unit in &SiliconUnit::ALL {
        if *unit != SiliconUnit::ShaderCore {
            assert!(
                unit.requires_sovereign_pipeline(),
                "{unit} should require sovereign pipeline"
            );
        }
    }
}

#[test]
fn graphics_pipeline_units() {
    assert!(SiliconUnit::Rasterizer.is_graphics_pipeline_unit());
    assert!(SiliconUnit::DepthBuffer.is_graphics_pipeline_unit());
    assert!(SiliconUnit::Rop.is_graphics_pipeline_unit());
    assert!(SiliconUnit::TextureUnit.is_graphics_pipeline_unit());
    assert!(SiliconUnit::Tessellator.is_graphics_pipeline_unit());
    assert!(!SiliconUnit::ShaderCore.is_graphics_pipeline_unit());
    assert!(!SiliconUnit::TensorCore.is_graphics_pipeline_unit());
    assert!(!SiliconUnit::RtCore.is_graphics_pipeline_unit());
    assert!(!SiliconUnit::VideoEncoder.is_graphics_pipeline_unit());
}

#[test]
fn silicon_capabilities_shader_only() {
    let caps = SiliconCapabilities::shader_only();
    assert_eq!(caps.unit_count(), 1);
    assert!(caps.has_unit(SiliconUnit::ShaderCore));
    assert!(!caps.has_unit(SiliconUnit::TensorCore));
}

#[test]
fn silicon_capabilities_discrete_gpu() {
    let caps = SiliconCapabilities::discrete_gpu_baseline(128, 96);
    assert_eq!(caps.unit_count(), 6);
    assert!(caps.has_unit(SiliconUnit::ShaderCore));
    assert!(caps.has_unit(SiliconUnit::Rasterizer));
    assert!(caps.has_unit(SiliconUnit::DepthBuffer));
    assert!(caps.has_unit(SiliconUnit::Rop));
    assert!(caps.has_unit(SiliconUnit::TextureUnit));
    assert!(caps.has_unit(SiliconUnit::Tessellator));
    assert!(!caps.has_unit(SiliconUnit::TensorCore));
    assert_eq!(caps.estimated_tmu_count, 128);
    assert_eq!(caps.estimated_rop_count, 96);
}

#[test]
fn silicon_unit_display() {
    assert_eq!(SiliconUnit::ShaderCore.to_string(), "shader_core");
    assert_eq!(SiliconUnit::TensorCore.to_string(), "tensor_core");
    assert_eq!(SiliconUnit::RtCore.to_string(), "rt_core");
    assert_eq!(SiliconUnit::Rop.to_string(), "rop");
}

#[test]
fn silicon_unit_serde_roundtrip() {
    for unit in &SiliconUnit::ALL {
        let json = serde_json::to_string(unit).expect("serialize");
        let back: SiliconUnit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*unit, back);
    }
}

#[test]
fn performance_measurement_serde() {
    let m = PerformanceMeasurement {
        operation: String::from("math.pairwise.yukawa"),
        silicon_unit: SiliconUnit::RtCore,
        precision_mode: String::from("fp32"),
        throughput_gflops: 5400.0,
        tolerance_achieved: 1e-7,
        gpu_model: String::from("RTX 3090"),
        measured_by: String::from("hotSpring exp076"),
        timestamp: 1_710_000_000,
    };
    let json = serde_json::to_string(&m).expect("serialize");
    let back: PerformanceMeasurement = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&*back.operation, "math.pairwise.yukawa");
    assert_eq!(back.silicon_unit, SiliconUnit::RtCore);
}

#[test]
fn routed_operation_with_fallback() {
    let fallback = RoutedOperation {
        operation: String::from("neighbor_search"),
        silicon_unit: SiliconUnit::ShaderCore,
        precision_mode: String::from("fp32"),
        estimated_throughput_gflops: 540.0,
        reason: String::from("compute BVH fallback"),
        fallback: None,
    };
    let primary = RoutedOperation {
        operation: String::from("neighbor_search"),
        silicon_unit: SiliconUnit::RtCore,
        precision_mode: String::from("fp32"),
        estimated_throughput_gflops: 5400.0,
        reason: String::from("spatial query, 10x over compute"),
        fallback: Some(Box::new(fallback)),
    };
    assert!(primary.fallback.is_some());
    let fb = primary.fallback.as_ref().unwrap();
    assert_eq!(fb.silicon_unit, SiliconUnit::ShaderCore);
}

#[test]
fn multi_unit_routing_plan() {
    let plan = MultiUnitRoutingPlan {
        operations: vec![
            RoutedOperation {
                operation: String::from("neighbor_search"),
                silicon_unit: SiliconUnit::RtCore,
                precision_mode: String::from("fp32"),
                estimated_throughput_gflops: 5400.0,
                reason: String::from("spatial query"),
                fallback: None,
            },
            RoutedOperation {
                operation: String::from("force_eval"),
                silicon_unit: SiliconUnit::ShaderCore,
                precision_mode: String::from("df64"),
                estimated_throughput_gflops: 3240.0,
                reason: String::from("14-digit tolerance"),
                fallback: None,
            },
            RoutedOperation {
                operation: String::from("accumulation"),
                silicon_unit: SiliconUnit::Rop,
                precision_mode: String::from("fp32"),
                estimated_throughput_gflops: 2700.0,
                reason: String::from("additive scatter"),
                fallback: None,
            },
        ],
        total_estimated_throughput_gflops: 11_340.0,
        gpu_target: String::from("RTX 3090"),
    };
    assert_eq!(plan.operations.len(), 3);
    assert_eq!(&*plan.gpu_target, "RTX 3090");
}

#[test]
fn tensor_core_gen_serde() {
    let tc_gen = TensorCoreGen::Ampere;
    let json = serde_json::to_string(&tc_gen).expect("serialize");
    let back: TensorCoreGen = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(tc_gen, back);
}

#[test]
fn rt_core_gen_serde() {
    let rt_gen = RtCoreGen::Ada;
    let json = serde_json::to_string(&rt_gen).expect("serialize");
    let back: RtCoreGen = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(rt_gen, back);
}
