// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vendor-agnostic hardware capabilities.
//!
//! [`HardwareCapabilities`] is the universal capability surface that every
//! [`ComputeDevice`](crate::ComputeDevice) exposes. Vendor-specific profiles
//! build these capabilities from their own domain knowledge, but consumers
//! only see the vendor-agnostic struct.

/// GPU vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    /// NVIDIA (nouveau, VFIO).
    Nvidia,
    /// AMD (amdgpu DRM).
    Amd,
    /// Intel (future).
    Intel,
    /// Unrecognized or software backend.
    Unknown,
}

impl std::fmt::Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nvidia => f.write_str("NVIDIA"),
            Self::Amd => f.write_str("AMD"),
            Self::Intel => f.write_str("Intel"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// GPU memory technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// GDDR5 (Kepler, Maxwell, older AMD).
    Gddr5,
    /// HBM2 / HBM2e (datacenter: Volta V100, Ampere A100, Vega MI50).
    Hbm2,
    /// HBM3 / HBM3e (datacenter: Hopper H100, MI300).
    Hbm3,
    /// GDDR6 (Turing, RDNA1/2).
    Gddr6,
    /// GDDR6X (Ampere B, Ada consumer).
    Gddr6x,
    /// GDDR7 (Blackwell consumer).
    Gddr7,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gddr5 => f.write_str("GDDR5"),
            Self::Hbm2 => f.write_str("HBM2"),
            Self::Hbm3 => f.write_str("HBM3"),
            Self::Gddr6 => f.write_str("GDDR6"),
            Self::Gddr6x => f.write_str("GDDR6X"),
            Self::Gddr7 => f.write_str("GDDR7"),
        }
    }
}

/// Native wave/warp execution width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaveSize {
    /// 32 threads per warp (NVIDIA, RDNA wave32).
    Wave32,
    /// 64 threads per wave (GCN / CDNA wave64).
    Wave64,
    /// Hardware supports both widths.
    Configurable,
}

/// How the GPU signals dispatch completion to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionStyle {
    /// Poll a register / USERD field until it advances.
    RegisterPoll,
    /// Device writes a fence value that the host polls or waits on.
    DeviceFence,
}

/// Vendor-agnostic hardware capabilities.
///
/// Built from vendor-specific generation profiles at device-open time.
#[derive(Debug, Clone)]
pub struct HardwareCapabilities {
    /// GPU vendor.
    pub vendor: Vendor,
    /// Short device/chip name (e.g. "Blackwell B", "Vega 20").
    pub device_name: &'static str,
    /// Generation / architecture family (e.g. "Kepler", "RDNA2").
    pub generation_name: &'static str,
    /// Whether the ALU supports IEEE 754 binary64 natively.
    pub has_hardware_f64: bool,
    /// Whether `MUFU.RCP64H` (or equivalent) produces correct results.
    pub has_hardware_f64_rcp: bool,
    /// Whether FP64 runs at full rate (1:2 ratio with FP32).
    pub has_full_rate_fp64: bool,
    /// Native warp/wave width.
    pub native_wave_size: WaveSize,
    /// Video memory technology.
    pub memory_type: MemoryType,
    /// How dispatch completion is signalled.
    pub completion_style: CompletionStyle,
    /// Maximum shared (local) memory per workgroup in bytes.
    pub max_shared_mem_bytes: u32,
}

impl HardwareCapabilities {
    /// Placeholder capabilities for backends that haven't implemented introspection.
    pub const UNKNOWN: Self = Self {
        vendor: Vendor::Unknown,
        device_name: "unknown",
        generation_name: "unknown",
        has_hardware_f64: false,
        has_hardware_f64_rcp: false,
        has_full_rate_fp64: false,
        native_wave_size: WaveSize::Wave32,
        memory_type: MemoryType::Gddr6,
        completion_style: CompletionStyle::DeviceFence,
        max_shared_mem_bytes: 0,
    };
}

impl std::fmt::Display for HardwareCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({}, {}, f64={})",
            self.vendor,
            self.device_name,
            self.generation_name,
            self.memory_type,
            self.has_hardware_f64,
        )
    }
}

// ── Vendor-Agnostic Boot Pipeline ────────────────────────────────────────

/// Result of probing a device's identity and state.
///
/// Every `BootPipeline` implementation defines its own probe result type
/// containing vendor-specific fields (BOOT0 for NVIDIA, GRBM_STATUS for AMD,
/// etc.), but all share this summary for cross-vendor consumers.
#[derive(Debug, Clone)]
pub struct BootProbeInfo {
    /// Vendor that produced this device.
    pub vendor: Vendor,
    /// Human-readable device family (e.g. "Kepler", "Vega", "Stratix 10").
    pub family: String,
    /// Whether the device is in a warm (already-initialized) state.
    pub warm: bool,
    /// Raw identity register value (BOOT0, GRBM_STATUS, etc.).
    pub identity_raw: u32,
}

/// Result of device initialization (devinit phase).
#[derive(Debug, Clone)]
pub struct BootInitInfo {
    /// Whether device memory is confirmed alive after init.
    pub memory_alive: bool,
    /// Number of MMIO writes applied during init.
    pub writes_applied: usize,
    /// Human-readable description of the init method used.
    pub method: String,
}

/// Vendor-agnostic boot pipeline for any PCIe device.
///
/// This trait captures the universal structure of cold/warm boot:
///
/// ```text
/// probe → is_warm? → devinit → engine_init → verify
/// ```
///
/// Each vendor/device family implements its own strategy for each phase.
/// The trait uses `dyn RegisterAccess` for BAR register I/O, which is
/// already vendor-agnostic.
///
/// # Implementors
///
/// - NVIDIA Kepler (`KeplerInit`): VBIOS interpreter devinit, PIO falcon boot
/// - NVIDIA Volta (`VoltaInit`): HBM2 training, ACR/SEC2 falcon boot
/// - AMD Vega (`VegaInit`): GRBM/UMC probing (stub)
///
/// # Design
///
/// Associated types allow each implementation to carry vendor-specific
/// detail in probe/init results while the trait signature remains generic.
/// The `summary` methods bridge vendor-specific results to cross-vendor
/// `BootProbeInfo`/`BootInitInfo` for consumers that don't need the details.
pub trait BootPipeline: Send + Sync + std::fmt::Debug {
    /// Vendor-specific probe result type.
    type ProbeResult: std::fmt::Debug + Clone;
    /// Vendor-specific init result type.
    type InitResult: std::fmt::Debug + Clone;

    /// Device family name (e.g. "Kepler", "Volta", "Vega 20").
    fn device_family(&self) -> &str;

    /// Phase 1: Probe the device — read identity registers, detect warm/cold.
    fn probe(
        &self,
        bar: &dyn crate::vfio::device::RegisterAccess,
    ) -> Result<Self::ProbeResult, crate::error::DriverError>;

    /// Is the device in a warm (already-initialized) state?
    fn is_warm(&self, probe: &Self::ProbeResult) -> bool;

    /// Summarize probe results into a vendor-agnostic `BootProbeInfo`.
    fn probe_summary(&self, probe: &Self::ProbeResult) -> BootProbeInfo;

    /// Phase 2: Device initialization — memory training, VBIOS, POST.
    ///
    /// For warm devices, implementations should return immediately with
    /// `memory_alive: true` and `method: "warm-skip"`.
    fn devinit(
        &self,
        bar: &dyn crate::vfio::device::RegisterAccess,
        probe: &Self::ProbeResult,
    ) -> Result<Self::InitResult, crate::error::DriverError>;

    /// Summarize init results into a vendor-agnostic `BootInitInfo`.
    fn init_summary(&self, init: &Self::InitResult) -> BootInitInfo;

    /// Phase 3: Engine initialization — firmware upload, ungating.
    fn engine_init(
        &self,
        bar: &dyn crate::vfio::device::RegisterAccess,
        probe: &Self::ProbeResult,
    ) -> Result<(), crate::error::DriverError>;

    /// Phase 4: Final verification — timers, memory, engine health.
    ///
    /// Returns `true` if the device is compute-ready.
    fn verify(
        &self,
        bar: &dyn crate::vfio::device::RegisterAccess,
    ) -> Result<bool, crate::error::DriverError>;
}

// ── Device Topology ──────────────────────────────────────────────────────

/// A single addressable function within a PCIe device.
///
/// For single-die GPUs this is the whole device. For multi-die devices
/// like the Tesla K80 (two GK210 dies behind a PLX switch), each die
/// is a separate `DeviceFunction` with its own BDF address.
///
/// This generalizes beyond GPUs to any multi-function PCIe device:
/// AMD chiplets, Intel tiles, FPGAs with multiple endpoints, etc.
#[derive(Debug, Clone)]
pub struct DeviceFunction {
    /// PCI BDF address (e.g. "0000:4b:00.0").
    pub bdf: String,
    /// Function index within the device (0-based).
    pub function_index: usize,
    /// Hardware vendor.
    pub vendor: Vendor,
}

/// Describes the topology of a physical device that may expose multiple
/// PCIe functions, each requiring independent initialization.
///
/// The K80's two GK210 dies share a VBIOS ROM but have independent VRAM,
/// PMU, and FECS — each die is initialized separately via its own
/// `BootPipeline` invocation. Single-die devices have exactly one function.
#[derive(Debug)]
pub struct DeviceTopology {
    /// Human-readable device name (e.g. "Tesla K80", "Titan V", "MI50").
    pub name: String,
    /// Per-function info, ordered by BDF.
    pub functions: Vec<DeviceFunction>,
    /// Shared firmware/VBIOS ROM bytes (parsed once, used by all functions).
    pub shared_firmware: Option<Vec<u8>>,
}

/// Result of initializing a single function within a device.
#[derive(Debug)]
pub struct FunctionBootResult {
    /// BDF of this function.
    pub bdf: String,
    /// Function index.
    pub function_index: usize,
    /// Probe summary (if probe succeeded).
    pub probe: Option<BootProbeInfo>,
    /// Init summary (if devinit succeeded).
    pub init: Option<BootInitInfo>,
    /// Whether verify passed.
    pub compute_ready: bool,
    /// Error description if any phase failed.
    pub error: Option<String>,
}

/// Result of initializing all functions of a device.
#[derive(Debug)]
pub struct DeviceBootResult {
    /// Per-function results.
    pub functions: Vec<FunctionBootResult>,
    /// Whether all functions reached compute-ready.
    pub all_ready: bool,
}

impl DeviceTopology {
    /// Create a single-function device.
    pub fn single(
        name: impl Into<String>,
        bdf: impl Into<String>,
        vendor: Vendor,
    ) -> Self {
        Self {
            name: name.into(),
            functions: vec![DeviceFunction {
                bdf: bdf.into(),
                function_index: 0,
                vendor,
            }],
            shared_firmware: None,
        }
    }

    /// Create a dual-function device (e.g. Tesla K80 with two dies).
    pub fn dual(
        name: impl Into<String>,
        bdf0: impl Into<String>,
        bdf1: impl Into<String>,
        vendor: Vendor,
    ) -> Self {
        Self {
            name: name.into(),
            functions: vec![
                DeviceFunction {
                    bdf: bdf0.into(),
                    function_index: 0,
                    vendor,
                },
                DeviceFunction {
                    bdf: bdf1.into(),
                    function_index: 1,
                    vendor,
                },
            ],
            shared_firmware: None,
        }
    }

    /// Attach shared firmware (VBIOS ROM, bitstream, etc.).
    pub fn with_firmware(mut self, fw: Vec<u8>) -> Self {
        self.shared_firmware = Some(fw);
        self
    }

    /// Number of functions in this device.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_display() {
        assert_eq!(Vendor::Nvidia.to_string(), "NVIDIA");
        assert_eq!(Vendor::Amd.to_string(), "AMD");
        assert_eq!(Vendor::Intel.to_string(), "Intel");
        assert_eq!(Vendor::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn memory_type_display() {
        assert_eq!(MemoryType::Gddr5.to_string(), "GDDR5");
        assert_eq!(MemoryType::Hbm2.to_string(), "HBM2");
        assert_eq!(MemoryType::Hbm3.to_string(), "HBM3");
        assert_eq!(MemoryType::Gddr7.to_string(), "GDDR7");
    }

    #[test]
    fn unknown_capabilities_are_conservative() {
        let caps = HardwareCapabilities::UNKNOWN;
        assert_eq!(caps.vendor, Vendor::Unknown);
        assert!(!caps.has_hardware_f64);
        assert!(!caps.has_hardware_f64_rcp);
        assert!(!caps.has_full_rate_fp64);
    }

    #[test]
    fn capabilities_display() {
        let caps = HardwareCapabilities {
            vendor: Vendor::Nvidia,
            device_name: "Blackwell B",
            generation_name: "Blackwell",
            has_hardware_f64: true,
            has_hardware_f64_rcp: false,
            has_full_rate_fp64: false,
            native_wave_size: WaveSize::Wave32,
            memory_type: MemoryType::Gddr7,
            completion_style: CompletionStyle::DeviceFence,
            max_shared_mem_bytes: 49152,
        };
        let s = caps.to_string();
        assert!(s.contains("NVIDIA"));
        assert!(s.contains("Blackwell B"));
        assert!(s.contains("GDDR7"));
    }

    #[test]
    fn vendor_equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Vendor::Nvidia);
        set.insert(Vendor::Amd);
        assert!(set.contains(&Vendor::Nvidia));
        assert!(!set.contains(&Vendor::Intel));
    }

    #[test]
    fn device_topology_single() {
        let topo = DeviceTopology::single("Titan V", "0000:02:00.0", Vendor::Nvidia);
        assert_eq!(topo.function_count(), 1);
        assert_eq!(topo.functions[0].bdf, "0000:02:00.0");
        assert_eq!(topo.functions[0].function_index, 0);
        assert_eq!(topo.functions[0].vendor, Vendor::Nvidia);
        assert!(topo.shared_firmware.is_none());
    }

    #[test]
    fn device_topology_dual() {
        let topo = DeviceTopology::dual(
            "Tesla K80",
            "0000:4b:00.0",
            "0000:4c:00.0",
            Vendor::Nvidia,
        );
        assert_eq!(topo.function_count(), 2);
        assert_eq!(topo.functions[0].bdf, "0000:4b:00.0");
        assert_eq!(topo.functions[1].bdf, "0000:4c:00.0");
        assert_eq!(topo.functions[1].function_index, 1);
    }

    #[test]
    fn device_topology_with_firmware() {
        let topo = DeviceTopology::single("MI50", "0000:03:00.0", Vendor::Amd)
            .with_firmware(vec![0x55, 0xAA, 0x80]);
        assert!(topo.shared_firmware.is_some());
        assert_eq!(topo.shared_firmware.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn boot_probe_info_debug() {
        let info = BootProbeInfo {
            vendor: Vendor::Nvidia,
            family: "Volta".to_string(),
            warm: true,
            identity_raw: 0x1400_00a1,
        };
        assert!(format!("{info:?}").contains("Volta"));
        assert!(info.warm);
    }

    #[test]
    fn boot_init_info_debug() {
        let info = BootInitInfo {
            memory_alive: true,
            writes_applied: 276,
            method: "vbios-interpreter".to_string(),
        };
        assert!(info.memory_alive);
        assert_eq!(info.writes_applied, 276);
    }

    #[test]
    fn function_boot_result_defaults() {
        let result = FunctionBootResult {
            bdf: "0000:02:00.0".to_string(),
            function_index: 0,
            probe: None,
            init: None,
            compute_ready: false,
            error: Some("not implemented".to_string()),
        };
        assert!(!result.compute_ready);
        assert!(result.error.is_some());
    }

    #[test]
    fn device_boot_result_all_ready() {
        let result = DeviceBootResult {
            functions: vec![
                FunctionBootResult {
                    bdf: "0000:4b:00.0".to_string(),
                    function_index: 0,
                    probe: None,
                    init: None,
                    compute_ready: true,
                    error: None,
                },
                FunctionBootResult {
                    bdf: "0000:4c:00.0".to_string(),
                    function_index: 1,
                    probe: None,
                    init: None,
                    compute_ready: true,
                    error: None,
                },
            ],
            all_ready: true,
        };
        assert!(result.all_ready);
        assert_eq!(result.functions.len(), 2);
    }
}
