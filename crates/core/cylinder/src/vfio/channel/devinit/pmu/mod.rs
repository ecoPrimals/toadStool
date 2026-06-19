// SPDX-License-Identifier: AGPL-3.0-or-later
//! PMU FALCON devinit orchestration: diagnostics, VBIOS upload, and execution.

mod falcon_io;
mod types;

pub use falcon_io::{pmu_exec, pmu_falcon_reset, pmu_read_args, pmu_upload_code, pmu_upload_data};
pub use types::{DevinitStatus, FalconDiagnostic, pmu_reg};

use crate::error::DevinitError;
use crate::vfio::device::MappedBar;

use super::script::interpret_boot_scripts;
use super::vbios::{BitTable, parse_pmu_table};

/// Quick VRAM check via PRAMIN sentinel.
fn check_vram_via_pramin(bar0: &MappedBar) -> bool {
    use crate::vfio::memory::{MemoryRegion, PraminRegion};
    if let Ok(mut region) = PraminRegion::new(bar0, 0x0002_6000, 8) {
        region.probe_sentinel(0, 0xCAFE_DEAD).is_working()
    } else {
        false
    }
}

/// Execute devinit with enhanced diagnostics and automatic VBIOS source selection.
pub fn execute_devinit_with_diagnostics(
    bar0: &MappedBar,
    bdf: Option<&str>,
) -> Result<bool, DevinitError> {
    let diag = FalconDiagnostic::probe(bar0, bdf);
    diag.print_report();

    if !diag.status.needs_post {
        return Ok(false);
    }

    let rom = diag.best_vbios(bar0, bdf)?;

    if check_vram_via_pramin(bar0) {
        tracing::info!("VRAM already alive — skipping devinit");
        return Ok(true);
    }

    if diag.secure_boot {
        tracing::info!(
            "secure boot detected — trying PMU FALCON upload (VBIOS signed firmware), \
             then host-side interpreter as fallback"
        );
    }

    tracing::info!("attempting PMU FALCON devinit");
    match execute_devinit(bar0, &rom) {
        Ok(true) => {
            let vram_ok = check_vram_via_pramin(bar0);
            if vram_ok {
                tracing::info!("FALCON devinit succeeded + VRAM alive");
                return Ok(true);
            }
            tracing::warn!("FALCON devinit completed but VRAM still dead");
        }
        Ok(false) => {
            tracing::info!("FALCON reports devinit not needed");
            return Ok(false);
        }
        Err(e) => {
            tracing::error!(error = %e, "FALCON devinit failed");
        }
    }

    tracing::info!("falling back to host-side VBIOS interpreter");
    let stats = interpret_boot_scripts(bar0, &rom)?;
    let vram_ok = check_vram_via_pramin(bar0);
    tracing::info!(
        writes = stats.writes_applied,
        vram = if vram_ok { "ALIVE" } else { "still dead" },
        "VBIOS interpreter fallback result"
    );
    Ok(vram_ok)
}

/// Execute the full devinit sequence via PMU FALCON.
///
/// Returns Ok(true) if devinit completed, Ok(false) if it wasn't needed,
/// or Err on failure.
pub fn execute_devinit(bar0: &MappedBar, rom: &[u8]) -> Result<bool, DevinitError> {
    let status = DevinitStatus::probe(bar0);
    status.print_summary();

    if !status.needs_post {
        if check_vram_via_pramin(bar0) {
            tracing::info!("devinit already complete + VRAM alive — skipping PMU upload");
            return Ok(false);
        }
        tracing::warn!("devinit_reg says complete but VRAM is dead — clearing stale register");
        let _ = bar0.write_u32(pmu_reg::DEVINIT_STATUS, 0);
    }

    let bit = BitTable::parse(rom)?;
    tracing::debug!(entries = bit.entries.len(), "BIT table");
    for entry in &bit.entries {
        tracing::trace!(
            bit_id = entry.id,
            version = entry.version,
            data_offset = format!("{:#06x}", entry.data_offset),
            data_size = entry.data_size,
            "BIT entry"
        );
    }

    let bit_i = bit.find(b'I').ok_or(DevinitError::BitINotFound)?;

    if bit_i.data_size < 0x1c {
        tracing::info!(
            data_size = bit_i.data_size,
            "BIT I short-form (Kepler): PMU firmware table not present, \
             deferring to host-side interpreter"
        );
        return Err(DevinitError::BitIShortForm {
            data_size: bit_i.data_size,
        });
    }

    if bit_i.version != 1 {
        return Err(DevinitError::BitIUnexpectedLayout {
            version: bit_i.version,
            data_size: bit_i.data_size,
        });
    }

    let pmu_fws = parse_pmu_table(rom, &bit)?;
    tracing::debug!(count = pmu_fws.len(), "PMU firmware entries");
    for fw in &pmu_fws {
        tracing::trace!(
            app_type = format!("{:#04x}", fw.app_type),
            boot = format!(
                "{:#x}+{:#x}({})",
                fw.boot_addr_pmu, fw.boot_addr, fw.boot_size
            ),
            code = format!(
                "{:#x}+{:#x}({})",
                fw.code_addr_pmu, fw.code_addr, fw.code_size
            ),
            data = format!(
                "{:#x}+{:#x}({})",
                fw.data_addr_pmu, fw.data_addr, fw.data_size
            ),
            init = format!("{:#x}", fw.init_addr_pmu),
            args = format!("{:#x}", fw.args_addr_pmu),
            "PMU firmware section"
        );
    }

    let devinit_fw = pmu_fws
        .iter()
        .find(|fw| fw.app_type == 0x04)
        .ok_or(DevinitError::PmuDevinitFirmwareNotFound)?;

    let rom_len = rom.len() as u32;
    if devinit_fw.boot_addr + devinit_fw.boot_size > rom_len
        || devinit_fw.code_addr + devinit_fw.code_size > rom_len
        || devinit_fw.data_addr + devinit_fw.data_size > rom_len
    {
        return Err(DevinitError::DevinitFirmwareBeyondRom);
    }

    tracing::info!("PMU FALCON devinit upload starting");

    pmu_falcon_reset(bar0);

    tracing::debug!(
        bytes = devinit_fw.boot_size,
        addr = format!("{:#x}", devinit_fw.boot_addr_pmu),
        "uploading boot code to PMU IMEM"
    );
    pmu_upload_code(
        bar0,
        rom,
        devinit_fw.boot_addr_pmu,
        devinit_fw.boot_addr,
        devinit_fw.boot_size,
        false,
    );

    tracing::debug!(
        bytes = devinit_fw.code_size,
        addr = format!("{:#x}", devinit_fw.code_addr_pmu),
        "uploading main code to PMU IMEM"
    );
    pmu_upload_code(
        bar0,
        rom,
        devinit_fw.code_addr_pmu,
        devinit_fw.code_addr,
        devinit_fw.code_size,
        true,
    );

    tracing::debug!(
        bytes = devinit_fw.data_size,
        addr = format!("{:#x}", devinit_fw.data_addr_pmu),
        "uploading data to PMU DMEM"
    );
    pmu_upload_data(
        bar0,
        rom,
        devinit_fw.data_addr_pmu,
        devinit_fw.data_addr,
        devinit_fw.data_size,
    );

    let i_data_off = bit_i.data_offset as usize;
    let opcode_img = u16::from_le_bytes([
        rom.get(i_data_off + 0x14).copied().unwrap_or(0),
        rom.get(i_data_off + 0x15).copied().unwrap_or(0),
    ]) as u32;
    let opcode_len = u16::from_le_bytes([
        rom.get(i_data_off + 0x16).copied().unwrap_or(0),
        rom.get(i_data_off + 0x17).copied().unwrap_or(0),
    ]) as u32;

    if opcode_len > 0 && opcode_img + opcode_len <= rom_len {
        let pmu_opcode_addr = pmu_read_args(bar0, devinit_fw.args_addr_pmu + 0x08, 0x08);
        tracing::trace!(
            bytes = opcode_len,
            rom_offset = format!("{:#x}", opcode_img),
            dmem = format!("{:#x}", pmu_opcode_addr),
            "uploading opcode tables"
        );
        pmu_upload_data(bar0, rom, pmu_opcode_addr, opcode_img, opcode_len);
    } else {
        tracing::debug!(
            img = format!("{opcode_img:#x}"),
            len = opcode_len,
            "no opcode table found"
        );
    }

    let script_img = u16::from_le_bytes([
        rom.get(i_data_off + 0x18).copied().unwrap_or(0),
        rom.get(i_data_off + 0x19).copied().unwrap_or(0),
    ]) as u32;
    let script_len = u16::from_le_bytes([
        rom.get(i_data_off + 0x1a).copied().unwrap_or(0),
        rom.get(i_data_off + 0x1b).copied().unwrap_or(0),
    ]) as u32;

    if script_len > 0 && script_img + script_len <= rom_len {
        let pmu_script_addr = pmu_read_args(bar0, devinit_fw.args_addr_pmu + 0x08, 0x10);
        tracing::trace!(
            bytes = script_len,
            rom_offset = format!("{:#x}", script_img),
            dmem = format!("{:#x}", pmu_script_addr),
            "uploading boot scripts"
        );
        pmu_upload_data(bar0, rom, pmu_script_addr, script_img, script_len);
    } else {
        tracing::debug!(
            img = format!("{script_img:#x}"),
            len = script_len,
            "no boot script found"
        );
    }

    tracing::info!("PMU devinit execution");
    let w = |reg, val: u32| {
        let _ = bar0.write_u32(reg, val);
    };
    let r = |reg| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

    w(pmu_reg::FALCON_MBOX0, 0x0000_5000);
    pmu_exec(bar0, devinit_fw.init_addr_pmu);

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(2);
    let mut completed = false;

    while start.elapsed() < timeout {
        let mbox = r(pmu_reg::FALCON_MBOX0);
        if mbox & 0x2000 != 0 {
            completed = true;
            tracing::info!(
                mbox0 = format!("{mbox:#010x}"),
                elapsed_ms = start.elapsed().as_millis(),
                "DEVINIT complete"
            );
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    if !completed {
        let mbox = r(pmu_reg::FALCON_MBOX0);
        let ctrl = r(pmu_reg::FALCON_CTRL);
        tracing::error!(
            mbox0 = format!("{mbox:#010x}"),
            ctrl = format!("{ctrl:#010x}"),
            "DEVINIT timeout"
        );
        return Err(DevinitError::PmuDevinitTimeout { mbox0: mbox });
    }

    if let Some(preos_fw) = pmu_fws.iter().find(|fw| fw.app_type == 0x01) {
        tracing::info!("loading PRE_OS app (fan control)");
        if preos_fw.boot_addr + preos_fw.boot_size <= rom_len
            && preos_fw.code_addr + preos_fw.code_size <= rom_len
            && preos_fw.data_addr + preos_fw.data_size <= rom_len
        {
            pmu_falcon_reset(bar0);
            pmu_upload_code(
                bar0,
                rom,
                preos_fw.boot_addr_pmu,
                preos_fw.boot_addr,
                preos_fw.boot_size,
                false,
            );
            pmu_upload_code(
                bar0,
                rom,
                preos_fw.code_addr_pmu,
                preos_fw.code_addr,
                preos_fw.code_size,
                true,
            );
            pmu_upload_data(
                bar0,
                rom,
                preos_fw.data_addr_pmu,
                preos_fw.data_addr,
                preos_fw.data_size,
            );
            pmu_exec(bar0, preos_fw.init_addr_pmu);
            tracing::info!("PRE_OS app launched on PMU");
        }
    }

    let post_status = DevinitStatus::probe(bar0);
    if !post_status.needs_post {
        tracing::info!("devinit status register shows COMPLETE");
    } else {
        tracing::warn!("devinit status register still shows needs_post");
    }

    Ok(true)
}
