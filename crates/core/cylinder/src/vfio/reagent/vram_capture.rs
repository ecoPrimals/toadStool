// SPDX-License-Identifier: AGPL-3.0-or-later

//! VRAM firmware capture — read staged falcon blobs via BAR0 PRAMIN window.

use std::path::{Path, PathBuf};

use super::ReagentError;

/// Known VRAM firmware staging addresses from Exp 160 mmiotrace analysis.
/// nvidia stages firmware blobs in VRAM before DMA-loading to falcon IMEM.
pub mod vram_firmware_addrs {
    /// FECS firmware staged at this VRAM address before BootROM DMA (Exp 160, nvidia-535).
    pub const FECS_VRAM_ADDR_535: u64 = 0x802F_D458;
    /// FECS code size (from nvidia-535 mmiotrace — 25632 bytes, matches fecs_inst.bin).
    pub const FECS_CODE_SIZE: usize = 25632;
    /// GPCCS firmware typically follows FECS in VRAM (offset determined at runtime).
    pub const GPCCS_VRAM_OFFSET_HINT: u64 = 0x10000;
}

/// Attempt to read VRAM content through the PRAMIN window (BAR0 0x700000).
///
/// When nvidia is loaded and PRAMIN is configured, the 1 MiB window at
/// BAR0 offset 0x700000 maps a configurable region of GPU VRAM. By writing
/// the target VRAM page address to `NV_PBUS_BAR0_WINDOW` (0x1700), we can
/// read arbitrary VRAM contents.
///
/// Returns the bytes read, or an error if PRAMIN is not configured or the
/// read fails.
pub fn read_vram_via_pramin(
    bar0: &crate::vfio::device::MappedBar,
    vram_addr: u64,
    len: usize,
) -> Result<Vec<u8>, ReagentError> {
    const PRAMIN_BASE: usize = 0x70_0000;
    const PRAMIN_SIZE: usize = 0x10_0000; // 1 MiB window
    const BAR0_WINDOW_REG: usize = 0x1700;

    if len > PRAMIN_SIZE {
        return Err(ReagentError::PraminSizeExceeded {
            len,
            max: PRAMIN_SIZE,
        });
    }

    let page_addr = (vram_addr >> 16) as u32;
    bar0.write_u32(BAR0_WINDOW_REG, page_addr)?;

    let page_offset = (vram_addr & 0xFFFF) as usize;
    let mut data = Vec::with_capacity(len);

    for i in (0..len).step_by(4) {
        let offset = PRAMIN_BASE + page_offset + i;
        let word = bar0.read_u32(offset)?;
        data.extend_from_slice(&word.to_le_bytes());
    }

    data.truncate(len);

    let nonzero = data.iter().filter(|&&b| b != 0).count();
    tracing::info!(
        vram_addr = format!("0x{vram_addr:x}"),
        len = len,
        nonzero_bytes = nonzero,
        "VRAM read via PRAMIN"
    );

    Ok(data)
}

/// Attempt VRAM firmware capture for all known falcon staging addresses.
///
/// While nvidia is loaded and FECS is running, the firmware blobs are staged
/// in VRAM at known addresses. This function reads them through the PRAMIN
/// window, bypassing the HS IMEM PIO block entirely.
///
/// Returns paths to captured firmware files, or errors for each.
pub fn capture_vram_firmware(
    bar0: &crate::vfio::device::MappedBar,
    output_dir: &Path,
) -> Vec<(String, Result<PathBuf, ReagentError>)> {
    use vram_firmware_addrs::*;

    std::fs::create_dir_all(output_dir).ok();
    let mut results = Vec::new();

    // Capture FECS from VRAM
    let fecs_path = output_dir.join("fecs_vram_capture.bin");
    let fecs_result =
        read_vram_via_pramin(bar0, FECS_VRAM_ADDR_535, FECS_CODE_SIZE).and_then(|data| {
            let nonzero = data.iter().filter(|&&b| b != 0).count();
            if nonzero < FECS_CODE_SIZE / 10 {
                return Err(ReagentError::VramCaptureEmpty {
                    name: "FECS",
                    nonzero,
                    total: FECS_CODE_SIZE,
                    addr: FECS_VRAM_ADDR_535,
                });
            }
            std::fs::write(&fecs_path, &data).map_err(ReagentError::WriteRecipe)?;
            tracing::info!(
                path = %fecs_path.display(),
                size = data.len(),
                nonzero = nonzero,
                "FECS firmware captured from VRAM"
            );
            Ok(fecs_path.clone())
        });
    results.push(("fecs_vram".to_owned(), fecs_result));

    // Attempt GPCCS capture at hinted offset after FECS
    let gpccs_addr = FECS_VRAM_ADDR_535 + GPCCS_VRAM_OFFSET_HINT;
    let gpccs_size = 12643; // matches gpccs_inst.bin
    let gpccs_path = output_dir.join("gpccs_vram_capture.bin");
    let gpccs_result = read_vram_via_pramin(bar0, gpccs_addr, gpccs_size).and_then(|data| {
        let nonzero = data.iter().filter(|&&b| b != 0).count();
        if nonzero < gpccs_size / 10 {
            return Err(ReagentError::VramCaptureEmpty {
                name: "GPCCS",
                nonzero,
                total: gpccs_size,
                addr: gpccs_addr,
            });
        }
        std::fs::write(&gpccs_path, &data).map_err(ReagentError::WriteRecipe)?;
        tracing::info!(
            path = %gpccs_path.display(),
            size = data.len(),
            nonzero = nonzero,
            "GPCCS firmware captured from VRAM"
        );
        Ok(gpccs_path.clone())
    });
    results.push(("gpccs_vram".to_owned(), gpccs_result));

    results
}
