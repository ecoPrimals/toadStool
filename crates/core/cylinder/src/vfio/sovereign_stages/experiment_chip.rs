// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::vfio::device::MappedBar;

/// Auto-detect chip identity from BAR0 MMIO reads.
///
/// Probes NVIDIA BOOT0 at offset 0 first; if unrecognized, probes AMD GRBM_STATUS
/// at offset 0x8010. Distinguishes:
/// - NVIDIA GPU (chip name + SM version)
/// - AMD GPU present (cold boot not implemented — probe-only via [`VegaInit`])
/// - No responsive GPU (unmapped BAR0 or all-ones reads)
///
/// [`VegaInit`]: crate::vfio::amd_metal::VegaInit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChipDetection {
    /// NVIDIA GPU identified from BOOT0.
    Nvidia {
        /// Chip codename (e.g. `"gv100"`).
        chip: &'static str,
        /// SM architecture version (e.g. 70).
        sm: u32,
    },
    /// AMD GPU identified from GRBM register map.
    ///
    /// Warm detection works via [`VegaInit::probe`]; cold boot (`devinit`,
    /// `engine_init`) is not implemented.
    AmdPresent {
        /// GPU family label (e.g. `"Vega 20"`).
        family: &'static str,
        /// GRBM_STATUS register value at probe time.
        grbm_status: u32,
    },
    /// BAR0 reads indicate no responsive GPU.
    NotFound {
        /// BOOT0 (offset 0) read value.
        boot0: u32,
        /// GRBM_STATUS (offset 0x8010) read value.
        grbm_status: u32,
    },
}

impl ChipDetection {
    /// Human-readable diagnostic for operators and experiment logs.
    #[must_use]
    pub fn diagnostic(&self) -> String {
        match self {
            Self::Nvidia { chip, sm } => format!("NVIDIA {chip} (SM {sm})"),
            Self::AmdPresent { family, grbm_status } => format!(
                "AMD {family} present (GRBM_STATUS=0x{grbm_status:08x}) — \
                 cold boot not implemented; warm probe via VegaInit only"
            ),
            Self::NotFound { boot0, grbm_status } => format!(
                "no GPU found (BOOT0=0x{boot0:08x}, GRBM_STATUS=0x{grbm_status:08x})"
            ),
        }
    }
}

/// AMD Vega GRBM_STATUS offset within BAR0 (GFX906 / MI50 register map).
pub(crate) const AMD_GRBM_STATUS: u32 = 0x8010;

/// Auto-detect chip from BAR0 MMIO.
#[must_use]
pub fn detect_chip(bar0: &MappedBar) -> ChipDetection {
    let boot0 = bar0.read_u32(0x0000_0000).unwrap_or(0xFFFF_FFFF);

    if boot0 != 0 && boot0 != 0xFFFF_FFFF
        && let Some(sm) = crate::nv::identity::boot0_to_sm(boot0) {
            let chip = crate::nv::identity::chip_name(sm);
            return ChipDetection::Nvidia { chip, sm };
        }

    let grbm_status = bar0
        .read_u32(AMD_GRBM_STATUS as usize)
        .unwrap_or(0xFFFF_FFFF);
    if grbm_status != 0 && grbm_status != 0xFFFF_FFFF {
        tracing::info!(
            grbm_status = format!("0x{grbm_status:08x}"),
            "detect_chip: AMD GPU present — cold boot not implemented"
        );
        return ChipDetection::AmdPresent {
            family: "Vega 20",
            grbm_status,
        };
    }

    ChipDetection::NotFound {
        boot0,
        grbm_status,
    }
}

/// Legacy `(chip_name, sm_version)` tuple for experiment stages.
///
/// Returns `("unknown", 0)` for AMD or unrecognized hardware.
pub(crate) fn detect_chip_legacy(bar0: &MappedBar) -> (&'static str, u32) {
    match detect_chip(bar0) {
        ChipDetection::Nvidia { chip, sm } => (chip, sm),
        ChipDetection::AmdPresent { .. } => ("amd-vega20", 0),
        ChipDetection::NotFound { .. } => ("unknown", 0),
    }
}
