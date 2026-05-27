#[cfg(test)]
mod tests {
    use crate::{
        BufferHandle, ComputeDevice, DispatchDims, MemoryDomain, ShaderInfo,
    };

    use super::super::NvVfioComputeDevice;

    #[test]
    fn cold_dispatch_returns_fecs_error() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        let result = dev.dispatch(
            &[0u8; 64],
            &[],
            DispatchDims::new(1, 1, 1),
            &ShaderInfo::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FECS"));
    }

    #[test]
    fn cold_alloc_returns_unsupported() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(dev.alloc(4096, MemoryDomain::Vram).is_err());
    }

    #[test]
    fn with_sm_populates_caps() {
        let dev = NvVfioComputeDevice::with_sm("0000:25:00.0".into(), 70);
        let caps = dev.capabilities();
        assert_eq!(caps.vendor, crate::hardware::Vendor::Nvidia);
        assert_ne!(caps.device_name, "unknown");
    }

    #[test]
    fn new_has_unknown_caps() {
        let dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert_eq!(dev.capabilities().vendor, crate::hardware::Vendor::Unknown);
    }

    #[test]
    fn fecs_ready_flag() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(!dev.is_fecs_ready());
        dev.set_fecs_ready(true);
        assert!(dev.is_fecs_ready());
    }

    #[test]
    fn warm_fecs_enables_alloc_gate() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(dev.alloc(4096, MemoryDomain::Vram).is_err());
        let err = dev.alloc(4096, MemoryDomain::Vram).unwrap_err();
        assert!(err.to_string().contains("FECS"));

        dev.set_fecs_ready(true);
        let err = dev.alloc(4096, MemoryDomain::Vram).unwrap_err();
        assert!(
            err.to_string().contains("VFIO not opened"),
            "with FECS ready but no VFIO, should hit VFIO gate: {err}"
        );
    }

    #[test]
    fn warm_fecs_enables_dispatch_gate() {
        let mut dev = NvVfioComputeDevice::with_sm("0000:01:00.0".into(), 70);
        dev.set_fecs_ready(true);
        let err = dev
            .dispatch(
                &[0u8; 64],
                &[],
                DispatchDims::new(1, 1, 1),
                &ShaderInfo::default(),
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("VFIO not opened"),
            "with FECS ready but no VFIO, should hit VFIO gate: {err}"
        );
    }

    #[test]
    fn dispatch_rejects_empty_shader() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        dev.set_fecs_ready(true);
        let err = dev
            .dispatch(
                &[],
                &[],
                DispatchDims::new(1, 1, 1),
                &ShaderInfo::default(),
            )
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("VFIO not opened") || msg.contains("non-empty"),
            "empty shader binary should fail: {msg}"
        );
    }

    #[test]
    fn free_unknown_handle_returns_not_found() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        let err = dev.free(BufferHandle(999)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("VFIO") || msg.contains("not found") || msg.contains("999"),
            "unknown handle should error: {msg}"
        );
    }

    #[test]
    fn is_vfio_open_default_false() {
        let dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(!dev.is_vfio_open());
    }

    #[test]
    fn kepler_sm_uses_v21_qmd() {
        use crate::nv::generation;

        let dev = NvVfioComputeDevice::with_sm("0000:25:00.0".into(), 37);
        let profile = generation::profile_for_sm(37);
        assert_eq!(profile.qmd_version, generation::QmdVersion::V21);
        assert!(matches!(
            profile.page_table_format,
            generation::PageTableFormat::V1TwoLevel
        ));
        assert_eq!(profile.boot_strategy, generation::BootStrategy::NoAcr);
        assert_eq!(dev.capabilities().vendor, crate::hardware::Vendor::Nvidia);
    }

    #[test]
    fn kepler_doorbell_address() {
        let addr = crate::vfio::channel::registers::usermode::gk104_doorbell(0);
        assert_eq!(addr, 0x3000);
        let addr7 = crate::vfio::channel::registers::usermode::gk104_doorbell(7);
        assert_eq!(addr7, 0x3000 + 7 * 8);
    }
}
