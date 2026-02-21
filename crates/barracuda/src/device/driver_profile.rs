//! GPU Driver Profile — data-driven shader specialisation.
//!
//! This module answers the question **"who is driving the hardware?"** and
//! translates that into concrete shader strategies.  It complements
//! `capabilities` (which answers "what can the hardware do?") by providing
//! the compiler/driver layer between the application and the silicon.
//!
//! ## Design
//!
//! A single `GpuDriverProfile` struct consolidates:
//! - Driver identity (`DriverKind`: NVK, RADV, proprietary NVIDIA, …)
//! - Shader compiler backend (`CompilerKind`: NAK, ACO, PTXAS, …)
//! - GPU microarchitecture (`GpuArch`: Volta, Turing, RDNA2, …)
//! - FP64 hardware rate classification (`Fp64Rate`)
//! - Active workarounds (`Workaround`: NVK exp/log crash, …)
//!
//! Query `GpuDriverProfile` at dispatch time instead of re-running
//! string-matching logic scattered across the codebase.
//!
//! ## Sovereign Compute Evolution
//!
//! All four Sovereign phases are tracked here:
//! - Phase 1 ✓  Profile detection + eigensolve strategy
//! - Phase 2 ✓  NAK contribution (SM70/RDNA2/AppleM latency tables)
//! - Phase 3 ✓  ILP reorderer + loop unroller wired into `compile_shader_f64()`
//! - Phase 4    Specialised codegen — deferred until upstream bottleneck confirmed
//!
//! Reference: `docs/specs/SOVEREIGN_COMPUTE_EVOLUTION.md`

use std::fmt;

use crate::device::WgpuDevice;

// ── Driver identity ───────────────────────────────────────────────────────────

/// GPU driver/compiler identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DriverKind {
    NvidiaProprietary,
    Nvk,
    Radv,
    Intel,
    Software,
    Unknown,
}

/// GPU shader compiler backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerKind {
    /// NVIDIA proprietary PTX assembler
    NvidiaPtxas,
    /// Mesa NAK (Rust-based NVIDIA compiler)
    Nak,
    /// Mesa ACO (AMD compiler)
    Aco,
    /// Intel ANV compiler
    Anv,
    /// Software rasterizer (llvmpipe, swiftshader)
    Software,
    Unknown,
}

// ── GPU architecture ──────────────────────────────────────────────────────────

/// GPU microarchitecture generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuArch {
    /// NVIDIA Volta (SM70) — Titan V, Quadro GV100
    Volta,
    /// NVIDIA Turing (SM75) — RTX 2000 series
    Turing,
    /// NVIDIA Ampere (SM80/86) — RTX 3000 series
    Ampere,
    /// NVIDIA Ada Lovelace (SM89) — RTX 4000 series
    Ada,
    /// AMD RDNA 2 — RX 6000 series
    Rdna2,
    /// AMD RDNA 3 — RX 7000 series
    Rdna3,
    /// AMD CDNA 2 — MI200 series
    Cdna2,
    /// Intel Arc (Alchemist/Battlemage)
    IntelArc,
    /// Apple M-series GPU (Apple Silicon — M1/M2/M3/M4 family)
    ///
    /// Runs via Metal + wgpu's Metal backend. FP64 is emulated in software
    /// (Apple GPUs only have f32 hardware). ILP window empirically ~4 cycles.
    AppleM,
    /// Software rasterizer
    Software,
    Unknown,
}

// ── FP64 rate ─────────────────────────────────────────────────────────────────

/// FP64 hardware rate classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fp64Rate {
    /// Full rate: FP64:FP32 = 1:2 (Titan V, MI250, etc.)
    Full,
    /// Throttled by vendor SDK but accessible via Vulkan
    Throttled,
    /// Hardware rate 1:64 (consumer Ada, Turing)
    Minimal,
    /// Software emulated
    Software,
}

// ── Workarounds ───────────────────────────────────────────────────────────────

/// A known driver/compiler workaround that must be active for a given profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Workaround {
    /// NVK exp(f64) crashes — substitute with polynomial approximation
    NvkExpF64Crash,
    /// NVK log(f64) crashes — substitute with polynomial approximation
    NvkLogF64Crash,
}

// ── Eigensolve strategy ───────────────────────────────────────────────────────

/// Eigensolve dispatch strategy chosen based on driver/arch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EigensolveStrategy {
    /// Warp-packed: 32 independent matrices per workgroup (NVIDIA)
    WarpPacked { wg_size: u32 },
    /// Wave-packed: 64 independent matrices per workgroup (AMD)
    WavePacked { wave_size: u32 },
    /// Standard: one matrix per workgroup
    Standard,
}

// ── GpuDriverProfile ──────────────────────────────────────────────────────────

/// Unified GPU driver profile for data-driven shader specialisation.
///
/// Consolidates driver detection, compiler quality knowledge, and known
/// workarounds. Query this instead of string-matching device names at
/// dispatch time.
///
/// ## Construction
///
/// ```rust,no_run
/// # use barracuda::device::{WgpuDevice, driver_profile::GpuDriverProfile};
/// # async fn example() -> barracuda::error::Result<()> {
/// let device = WgpuDevice::new().await?;
/// let profile = GpuDriverProfile::from_device(&device);
/// println!("{profile}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct GpuDriverProfile {
    pub driver: DriverKind,
    pub compiler: CompilerKind,
    pub arch: GpuArch,
    pub fp64_rate: Fp64Rate,
    pub workarounds: Vec<Workaround>,
}

impl GpuDriverProfile {
    /// Build a driver profile from a `WgpuDevice` using runtime detection.
    pub fn from_device(device: &WgpuDevice) -> Self {
        let driver = Self::detect_driver(device);
        let compiler = Self::detect_compiler(driver);
        let arch = Self::detect_arch(device);
        let fp64_rate = Self::detect_fp64_rate(&arch, driver);
        let workarounds = Self::detect_workarounds(driver);

        Self {
            driver,
            compiler,
            arch,
            fp64_rate,
            workarounds,
        }
    }

    /// Optimal eigensolve dispatch strategy for this driver/arch combination.
    ///
    /// hotSpring measured 2.2× NVK speedup with warp-packing (Titan V, Feb 2026).
    /// Neutral on proprietary NVIDIA (scheduler already handles wg1 efficiently).
    ///
    /// ### AMD RDNA2/RDNA3 (ACO compiler)
    ///
    /// Empirically measured on RX 6950 XT (RDNA2/NAVI21, Feb 2026):
    /// - `wg_size=32`: 67.7 ms  ← optimal
    /// - `wg_size=64`: 117.1 ms ← 1.7× slower
    ///
    /// Root cause: ACO targets **wave32 mode** for compute shaders on RDNA2.
    /// Use `WarpPacked { wg_size: 32 }` for all current ACO targets.
    pub fn optimal_eigensolve_strategy(&self) -> EigensolveStrategy {
        match (self.compiler, self.arch) {
            (CompilerKind::Nak, _) => EigensolveStrategy::WarpPacked { wg_size: 32 },
            (CompilerKind::Aco, GpuArch::Rdna2 | GpuArch::Rdna3) => {
                EigensolveStrategy::WarpPacked { wg_size: 32 }
            }
            (CompilerKind::Aco, GpuArch::Cdna2) => EigensolveStrategy::WavePacked { wave_size: 64 },
            (CompilerKind::NvidiaPtxas, _) => EigensolveStrategy::WarpPacked { wg_size: 32 },
            _ => EigensolveStrategy::Standard,
        }
    }

    /// Whether `exp(f64)` needs software substitution on this driver.
    pub fn needs_exp_f64_workaround(&self) -> bool {
        self.workarounds.contains(&Workaround::NvkExpF64Crash)
    }

    /// Whether `log(f64)` needs software substitution on this driver.
    pub fn needs_log_f64_workaround(&self) -> bool {
        self.workarounds.contains(&Workaround::NvkLogF64Crash)
    }

    /// Whether this driver supports f64 builtins (exp, log, etc.) natively.
    pub fn supports_f64_builtins(&self) -> bool {
        !matches!(self.driver, DriverKind::Nvk | DriverKind::Software)
    }

    /// Whether this is an open-source driver (NVK or RADV).
    pub fn is_open_source(&self) -> bool {
        matches!(self.driver, DriverKind::Nvk | DriverKind::Radv)
    }

    /// Return the `LatencyModel` appropriate for this GPU architecture.
    ///
    /// The model provides per-operation cycle counts used by the WGSL ILP
    /// scheduler (`@ilp_region` reorderer, Phase 3 `WgslDependencyGraph`).
    ///
    /// - NVIDIA Volta/Turing/Ampere/Ada → `Sm70LatencyModel` (DFMA = 8 cy)
    /// - AMD RDNA2/RDNA3/CDNA2 → `Rdna2LatencyModel` (VFMA64 ≈ 4 cy)
    /// - Unknown/Intel/Software → `ConservativeModel` (safe overestimate)
    #[must_use]
    pub fn latency_model(&self) -> Box<dyn crate::device::latency::LatencyModel> {
        crate::device::latency::model_for_arch(self.arch)
    }

    // ── Internal detection helpers ────────────────────────────────────────────

    fn detect_driver(device: &WgpuDevice) -> DriverKind {
        if device.is_nvk() {
            DriverKind::Nvk
        } else if device.is_nvidia_proprietary() {
            DriverKind::NvidiaProprietary
        } else if device.is_radv() {
            DriverKind::Radv
        } else {
            let name = device.adapter_info().name.to_lowercase();
            if name.contains("intel") || name.contains("iris") {
                DriverKind::Intel
            } else if name.contains("llvmpipe")
                || name.contains("swiftshader")
                || name.contains("software")
            {
                DriverKind::Software
            } else {
                DriverKind::Unknown
            }
        }
    }

    fn detect_compiler(driver: DriverKind) -> CompilerKind {
        match driver {
            DriverKind::NvidiaProprietary => CompilerKind::NvidiaPtxas,
            DriverKind::Nvk => CompilerKind::Nak,
            DriverKind::Radv => CompilerKind::Aco,
            DriverKind::Intel => CompilerKind::Anv,
            DriverKind::Software => CompilerKind::Software,
            DriverKind::Unknown => CompilerKind::Unknown,
        }
    }

    fn detect_arch(device: &WgpuDevice) -> GpuArch {
        let name = device.adapter_info().name.to_lowercase();

        if name.contains("titan v") || name.contains("gv100") || name.contains("v100") {
            return GpuArch::Volta;
        }
        if name.contains("rtx 20") || name.contains("rtx20") || name.contains("tu1") {
            return GpuArch::Turing;
        }
        if name.contains("rtx 30") || name.contains("rtx30") || name.contains("a100") {
            return GpuArch::Ampere;
        }
        if name.contains("rtx 40") || name.contains("rtx40") || name.contains("l40") {
            return GpuArch::Ada;
        }

        if name.contains("rx 6") || name.contains("rx6") {
            return GpuArch::Rdna2;
        }
        if name.contains("rx 7") || name.contains("rx7") {
            return GpuArch::Rdna3;
        }
        if name.contains("mi2") || name.contains("mi3") {
            return GpuArch::Cdna2;
        }

        if name.contains("arc") || name.contains("a770") || name.contains("a750") {
            return GpuArch::IntelArc;
        }

        if name.contains("apple m") || name.contains("apple paravirtual") {
            return GpuArch::AppleM;
        }

        if name.contains("llvmpipe") || name.contains("swiftshader") {
            return GpuArch::Software;
        }

        GpuArch::Unknown
    }

    fn detect_fp64_rate(arch: &GpuArch, driver: DriverKind) -> Fp64Rate {
        match arch {
            GpuArch::Volta => Fp64Rate::Full,
            GpuArch::Ampere => Fp64Rate::Throttled,
            GpuArch::Ada => Fp64Rate::Throttled,
            GpuArch::Turing => Fp64Rate::Throttled,
            GpuArch::Rdna2 | GpuArch::Rdna3 => Fp64Rate::Throttled,
            GpuArch::Cdna2 => Fp64Rate::Full,
            GpuArch::IntelArc => Fp64Rate::Minimal,
            GpuArch::AppleM => Fp64Rate::Software,
            GpuArch::Software => Fp64Rate::Software,
            GpuArch::Unknown => {
                if matches!(driver, DriverKind::Software) {
                    Fp64Rate::Software
                } else {
                    Fp64Rate::Throttled
                }
            }
        }
    }

    fn detect_workarounds(driver: DriverKind) -> Vec<Workaround> {
        match driver {
            DriverKind::Nvk => vec![Workaround::NvkExpF64Crash, Workaround::NvkLogF64Crash],
            _ => vec![],
        }
    }
}

impl fmt::Display for GpuDriverProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "GPU Driver Profile:")?;
        writeln!(f, "  Driver:   {:?}", self.driver)?;
        writeln!(f, "  Compiler: {:?}", self.compiler)?;
        writeln!(f, "  Arch:     {:?}", self.arch)?;
        writeln!(f, "  FP64:     {:?}", self.fp64_rate)?;
        if !self.workarounds.is_empty() {
            writeln!(f, "  Workarounds: {:?}", self.workarounds)?;
        }
        writeln!(f, "  Eigensolve: {:?}", self.optimal_eigensolve_strategy())?;
        Ok(())
    }
}
