// SPDX-License-Identifier: AGPL-3.0-or-later
//! wgpu_backend unit tests — adapter info, fingerprint, precision routing.

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod wgpu_backend_tests {
    use super::super::types::{
        GpuAdapterInfo, GpuDeviceType, HardwareFingerprint, is_nvidia_ada_lovelace,
    };
    use super::super::*;

    fn make_test_fingerprint(
        device_type: GpuDeviceType,
        supports_f64: bool,
        f64_compute_unreliable: bool,
        driver: &str,
        name: &str,
    ) -> HardwareFingerprint {
        let info = wgpu::AdapterInfo {
            name: name.to_owned(),
            vendor: 0x10de,
            device: 0x2684,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: driver.to_owned(),
            driver_info: "test".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        HardwareFingerprint::from_adapter_info(
            &info,
            device_type,
            supports_f64,
            f64_compute_unreliable,
            65535,
        )
    }

    #[test]
    fn test_hardware_fingerprint_discrete_f64() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvidia", "Test GPU");
        assert!(fp.estimated_tflops_f32 > 0.0);
        assert!(fp.estimated_tflops_f64 > 0.0);
        assert!(fp.sovereign_capable);
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::F64Native)
        );
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::MdForce));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Fft));
    }

    #[test]
    fn test_hardware_fingerprint_integrated_no_f64() {
        let fp = make_test_fingerprint(GpuDeviceType::Integrated, false, false, "anv", "Test GPU");
        assert!(fp.estimated_tflops_f32 > 0.0);
        assert_eq!(fp.estimated_tflops_f64, 0.0);
        assert!(
            !fp.capabilities
                .contains(&SubstrateCapabilityKind::F64Native)
        );
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::Df64Emulation)
        );
    }

    #[test]
    fn test_hardware_fingerprint_nvk_has_md_force() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvk", "Test GPU");
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::MdForce));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Eigen));
        assert!(fp.capabilities.contains(&SubstrateCapabilityKind::Cg));
    }

    fn make_test_adapter_info(
        name: &str,
        driver: &str,
        supports_f64: bool,
        f64_unreliable: bool,
        f64_shared_mem: bool,
        safe_alloc: u64,
    ) -> GpuAdapterInfo {
        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_ada = is_nvidia_ada_lovelace(name);
        let is_prop_nv = driver.contains("nvidia") && !driver.contains("nvk");
        let zeros_risk = (is_nvk && supports_f64) || (is_ada && is_prop_nv);
        GpuAdapterInfo {
            name: name.to_owned(),
            driver: driver.to_owned(),
            driver_info: String::new(),
            vendor_id: 0,
            device_id: 0,
            backend: "Vulkan".to_owned(),
            device_type: GpuDeviceType::Discrete,
            max_compute_workgroups_per_dimension: 65535,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 256,
            max_compute_workgroup_size_z: 64,
            max_buffer_size: 4_294_967_296,
            supports_shader_f64: supports_f64,
            f64_compute_unreliable: f64_unreliable,
            f64_shared_memory_reliable: f64_shared_mem,
            f64_zeros_risk: zeros_risk,
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            fingerprint: make_test_fingerprint(
                GpuDeviceType::Discrete,
                supports_f64,
                f64_unreliable,
                driver,
                name,
            ),
            safe_allocation_limit: safe_alloc,
            silicon: None,
        }
    }

    #[test]
    fn test_gpu_adapter_info_allocation_guard() {
        let info = make_test_adapter_info("Test", "nvk", true, false, false, 1_200_000_000);

        assert!(info.is_allocation_safe(1_000_000_000));
        assert!(!info.is_allocation_safe(2_000_000_000));
        assert!(info.is_nvk());
        assert!(info.is_sovereign_capable());
    }

    #[test]
    fn test_gpu_adapter_info_non_nvk() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);

        assert!(info.is_allocation_safe(4_000_000_000));
        assert!(!info.is_nvk());
    }

    #[test]
    fn test_gpu_device_type_variants() {
        assert_eq!(GpuDeviceType::Discrete, GpuDeviceType::Discrete);
        assert_ne!(GpuDeviceType::Discrete, GpuDeviceType::Integrated);
    }

    #[test]
    fn test_substrate_capability_kind_equality() {
        assert_eq!(
            SubstrateCapabilityKind::F64Native,
            SubstrateCapabilityKind::F64Native
        );
        assert_ne!(
            SubstrateCapabilityKind::F64Native,
            SubstrateCapabilityKind::Df64Emulation
        );
    }

    #[test]
    fn test_f64_compute_unreliable_nvk_volta() {
        let fp =
            make_test_fingerprint(GpuDeviceType::Discrete, true, true, "nvk", "NVIDIA Titan V");
        assert!(
            !fp.capabilities
                .contains(&SubstrateCapabilityKind::F64Native)
        );
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::Df64Emulation)
        );
    }

    #[test]
    fn test_f64_compute_unreliable_nvk_non_volta() {
        let fp = make_test_fingerprint(
            GpuDeviceType::Discrete,
            true,
            false,
            "nvk",
            "NVIDIA GeForce RTX 3080",
        );
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::F64Native)
        );
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::Df64Emulation)
        );
    }

    #[test]
    fn test_has_reliable_f64_nvk_volta() {
        let info =
            make_test_adapter_info("NVIDIA Titan V", "nvk", true, true, false, 1_200_000_000);
        assert!(info.supports_shader_f64);
        assert!(info.f64_compute_unreliable);
        assert!(!info.has_reliable_f64());
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::Df64Only);
    }

    #[test]
    fn test_subgroup_size_fields() {
        let mut info_zero =
            make_test_adapter_info("Test", "anv", false, false, false, 4_294_967_296);
        info_zero.device_type = GpuDeviceType::Integrated;
        info_zero.min_subgroup_size = 0;
        info_zero.max_subgroup_size = 0;

        assert_eq!(info_zero.min_subgroup_size, 0);
        assert_eq!(info_zero.max_subgroup_size, 0);

        let info_populated = GpuAdapterInfo {
            min_subgroup_size: 32,
            max_subgroup_size: 32,
            ..info_zero.clone()
        };
        assert_eq!(info_populated.min_subgroup_size, 32);
        assert_eq!(info_populated.max_subgroup_size, 32);
    }

    #[test]
    fn test_max_2d_dispatch() {
        let mut info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        info.max_compute_workgroups_per_dimension = 4096;
        let (max_x, max_y) = info.max_2d_dispatch();
        assert_eq!(max_x, 4096);
        assert_eq!(max_y, 4096);
    }

    #[test]
    fn test_precision_routing_f32_only() {
        let info = make_test_adapter_info("Intel iGPU", "anv", false, false, false, 4_294_967_296);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::F32Only);
    }

    #[test]
    fn test_precision_routing_df64_only() {
        let info =
            make_test_adapter_info("NVIDIA Titan V", "nvk", true, true, false, 1_200_000_000);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::Df64Only);
    }

    #[test]
    fn test_precision_routing_no_shared_mem() {
        let info = make_test_adapter_info(
            "NVIDIA RTX 4070",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_precision_routing_full_native() {
        let info = make_test_adapter_info("Future GPU", "nvidia", true, false, true, 4_294_967_296);
        assert_eq!(info.precision_routing(), PrecisionRoutingAdvice::F64Native);
    }

    #[test]
    fn test_f64_shared_memory_reliable_field() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        assert!(!info.f64_shared_memory_reliable);
        assert!(info.has_reliable_f64());
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_sovereign_binary_capable_field() {
        let info = make_test_adapter_info("Test", "nvidia", true, false, false, 4_294_967_296);
        assert!(!info.fingerprint.sovereign_binary_capable);
        assert!(info.fingerprint.sovereign_capable);
    }

    #[test]
    fn test_ada_lovelace_proprietary_f64_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 4070",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert!(
            info.f64_zeros_risk,
            "Ada Lovelace + proprietary should have f64_zeros_risk"
        );
        assert!(
            !info.fused_ops_healthy(),
            "fused ops should not be healthy on Ada Lovelace proprietary"
        );
        assert_eq!(
            info.precision_routing(),
            PrecisionRoutingAdvice::F64NativeNoSharedMem
        );
    }

    #[test]
    fn test_ada_lovelace_nvk_f64_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 4090",
            "nvk",
            true,
            false,
            false,
            1_200_000_000,
        );
        assert!(info.f64_zeros_risk, "NVK + f64 should have f64_zeros_risk");
        assert!(!info.fused_ops_healthy());
    }

    #[test]
    fn test_non_ada_proprietary_no_zeros_risk() {
        let info = make_test_adapter_info(
            "NVIDIA GeForce RTX 3090",
            "nvidia",
            true,
            false,
            false,
            4_294_967_296,
        );
        assert!(
            !info.f64_zeros_risk,
            "Ampere + proprietary should not have f64_zeros_risk"
        );
        assert!(info.fused_ops_healthy());
    }

    #[test]
    fn test_sovereign_compile_capability_present() {
        let fp = make_test_fingerprint(GpuDeviceType::Discrete, true, false, "nvidia", "Test GPU");
        assert!(
            fp.capabilities
                .contains(&SubstrateCapabilityKind::SovereignCompile),
            "sovereign-capable adapters should have SovereignCompile capability"
        );
    }

    #[test]
    fn test_sovereign_compile_absent_for_empty_driver() {
        let info = wgpu::AdapterInfo {
            name: "Unknown".to_owned(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::Cpu,
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
        };
        let fp = HardwareFingerprint::from_adapter_info(&info, GpuDeviceType::Cpu, false, false, 1);
        assert!(
            !fp.capabilities
                .contains(&SubstrateCapabilityKind::SovereignCompile),
            "empty-driver adapters should not have SovereignCompile"
        );
        assert!(!fp.sovereign_capable);
    }

    #[test]
    fn test_is_nvidia_ada_lovelace_detection() {
        assert!(is_nvidia_ada_lovelace("NVIDIA GeForce RTX 4070"));
        assert!(is_nvidia_ada_lovelace("NVIDIA GeForce RTX 4090"));
        assert!(is_nvidia_ada_lovelace("NVIDIA L40"));
        assert!(is_nvidia_ada_lovelace("NVIDIA RTX 4000 Ada Generation"));
        assert!(!is_nvidia_ada_lovelace("NVIDIA GeForce RTX 3090"));
        assert!(!is_nvidia_ada_lovelace("NVIDIA Titan V"));
        assert!(!is_nvidia_ada_lovelace("AMD Radeon RX 6950 XT"));
    }

    /// GPU f64 reduction smoke test (P1 — groundSpring V84-V100).
    ///
    /// Validates that all adapter configurations correctly flag
    /// f64 shared-memory as unreliable via the naga/SPIR-V pipeline
    /// and that precision routing steers callers to safe paths.
    #[test]
    fn test_f64_reduction_smoke_all_adapters() {
        let configs = [
            ("NVIDIA RTX 4090", "nvidia", true, false),
            ("NVIDIA RTX 4070", "nvidia", true, false),
            ("NVIDIA RTX 3090", "nvidia", true, false),
            ("NVIDIA Titan V", "nvk", true, true),
            ("NVIDIA RTX 3080", "nvk", true, false),
            ("Intel Arc A770", "anv", false, false),
            ("AMD RX 7900 XTX", "radv", false, false),
        ];

        for (name, driver, f64_support, f64_unreliable) in configs {
            let info = make_test_adapter_info(
                name,
                driver,
                f64_support,
                f64_unreliable,
                false,
                4_294_967_296,
            );

            assert!(
                !info.f64_shared_memory_reliable,
                "{name}: f64 shared-memory must be unreliable via naga/SPIR-V"
            );

            let routing = info.precision_routing();
            match (f64_support, f64_unreliable) {
                (false, _) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::F32Only,
                    "{name}: no f64 → F32Only"
                ),
                (true, true) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::Df64Only,
                    "{name}: unreliable f64 → Df64Only"
                ),
                (true, false) => assert_eq!(
                    routing,
                    PrecisionRoutingAdvice::F64NativeNoSharedMem,
                    "{name}: f64 OK but shared-mem broken → F64NativeNoSharedMem"
                ),
            }
        }
    }

    /// Validates that fused_ops_healthy correctly tracks f64_zeros_risk
    /// across NVK, Ada Lovelace proprietary, and safe configurations.
    #[test]
    fn test_fused_ops_healthy_matrix() {
        let cases = [
            ("NVIDIA RTX 4070", "nvidia", true, false, true), // Ada + proprietary → risk
            ("NVIDIA RTX 3090", "nvidia", true, false, false), // Ampere + proprietary → no risk
            ("NVIDIA RTX 4090", "nvk", true, false, true),    // NVK + f64 → risk
            ("NVIDIA RTX 3090", "nvk", true, false, true),    // NVK + f64 → risk
            ("Intel Arc A770", "anv", false, false, false),   // No f64 → no risk
        ];

        for (name, driver, f64_support, f64_unreliable, expect_risk) in cases {
            let info = make_test_adapter_info(
                name,
                driver,
                f64_support,
                f64_unreliable,
                false,
                4_294_967_296,
            );
            assert_eq!(
                info.f64_zeros_risk, expect_risk,
                "{name}/{driver}: f64_zeros_risk mismatch"
            );
            assert_eq!(
                info.fused_ops_healthy(),
                !expect_risk,
                "{name}/{driver}: fused_ops_healthy mismatch"
            );
        }
    }

    #[test]
    fn test_silicon_probe_rtx4090_has_tensor_rt_cores() {
        use super::super::initialization::probe_silicon_capabilities;
        use toadstool_core::silicon::{RtCoreGen, SiliconUnit, TensorCoreGen};

        let info = wgpu::AdapterInfo {
            name: "NVIDIA GeForce RTX 4090".to_owned(),
            vendor: 0x10de,
            device: 0x2684,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: "nvidia".to_owned(),
            driver_info: "560.35.03".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        let caps = probe_silicon_capabilities(&info, GpuDeviceType::Discrete);

        assert_eq!(caps.tensor_cores, Some(TensorCoreGen::Ada));
        assert_eq!(caps.rt_cores, Some(RtCoreGen::Ada));
        assert!(caps.has_video_encoder);
        assert!(caps.rasterizer_available);
        assert!(caps.tessellator_available);
        assert!(caps.estimated_tmu_count > 0);
        assert!(caps.estimated_rop_count > 0);
        assert!(caps.has_unit(SiliconUnit::TensorCore));
        assert!(caps.has_unit(SiliconUnit::RtCore));
        assert!(caps.has_unit(SiliconUnit::VideoEncoder));
        assert!(caps.has_unit(SiliconUnit::Rasterizer));
    }

    #[test]
    fn test_silicon_probe_titan_v_volta_tensor_no_rt() {
        use super::super::initialization::probe_silicon_capabilities;
        use toadstool_core::silicon::{SiliconUnit, TensorCoreGen};

        let info = wgpu::AdapterInfo {
            name: "NVIDIA Titan V".to_owned(),
            vendor: 0x10de,
            device: 0x1d81,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: "nvk".to_owned(),
            driver_info: "mesa".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        let caps = probe_silicon_capabilities(&info, GpuDeviceType::Discrete);

        assert_eq!(caps.tensor_cores, Some(TensorCoreGen::Volta));
        assert_eq!(caps.rt_cores, None);
        assert!(!caps.has_unit(SiliconUnit::RtCore));
        assert!(caps.has_unit(SiliconUnit::TensorCore));
    }

    #[test]
    fn test_silicon_probe_intel_igpu_basic() {
        use super::super::initialization::probe_silicon_capabilities;
        use toadstool_core::silicon::SiliconUnit;

        let info = wgpu::AdapterInfo {
            name: "Intel UHD Graphics 770".to_owned(),
            vendor: 0x8086,
            device: 0x4680,
            device_type: wgpu::DeviceType::IntegratedGpu,
            driver: "anv".to_owned(),
            driver_info: "mesa".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        let caps = probe_silicon_capabilities(&info, GpuDeviceType::Integrated);

        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_none());
        assert!(!caps.has_video_encoder);
        assert!(caps.rasterizer_available);
        assert!(caps.has_unit(SiliconUnit::ShaderCore));
        assert!(caps.has_unit(SiliconUnit::TextureUnit));
        assert!(!caps.has_unit(SiliconUnit::TensorCore));
    }

    #[test]
    fn test_silicon_probe_amd_rdna3_has_rt() {
        use super::super::initialization::probe_silicon_capabilities;
        use toadstool_core::silicon::SiliconUnit;

        let info = wgpu::AdapterInfo {
            name: "AMD Radeon RX 7900 XTX".to_owned(),
            vendor: 0x1002,
            device: 0x744c,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: "radv".to_owned(),
            driver_info: "mesa".to_owned(),
            backend: wgpu::Backend::Vulkan,
        };
        let caps = probe_silicon_capabilities(&info, GpuDeviceType::Discrete);

        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_some());
        assert!(caps.has_video_encoder);
        assert!(caps.has_unit(SiliconUnit::RtCore));
        assert!(caps.estimated_tmu_count > 0);
    }

    #[test]
    fn test_silicon_probe_cpu_fallback_minimal() {
        use super::super::initialization::probe_silicon_capabilities;
        use toadstool_core::silicon::SiliconUnit;

        let info = wgpu::AdapterInfo {
            name: "llvmpipe (LLVM 17)".to_owned(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::Cpu,
            driver: "llvmpipe".to_owned(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
        };
        let caps = probe_silicon_capabilities(&info, GpuDeviceType::Cpu);

        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_none());
        assert!(!caps.has_video_encoder);
        assert!(!caps.rasterizer_available);
        assert_eq!(caps.available_units, vec![SiliconUnit::ShaderCore]);
    }
}
