// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::{falcon, pmc, pri};

/// Recover the GPU's PRI ring after PCI driver unbind.
///
/// The kernel's PCI framework clears PMC_ENABLE during unbind, which disables
/// PGRAPH and kills PRI ring routing to GPC/TPC/FECS/GPCCS. This function:
/// 1. Re-enables PGRAPH in PMC_ENABLE (bit 12)
/// 2. Acknowledges any pending PRI ring interrupts
/// 3. Enumerates PRI ring stations
/// 4. Starts the PRI ring
/// 5. Verifies top-level falcon registers are accessible
pub(crate) fn recover_pri_ring(bdf: &str) -> Result<String, String> {
    let bar0 = crate::vfio::device::MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024)
        .map_err(|e| format!("BAR0 open failed: {e}"))?;

    // Read current PMC_ENABLE
    let pmc_before = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
    let pgraph_was_on = pmc_before & (1 << 12) != 0;

    // Enable PGRAPH (bit 12) if not already set
    if !pgraph_was_on {
        let new_pmc = pmc_before | (1 << 12);
        bar0.write_u32(pmc::ENABLE as usize, new_pmc)
            .map_err(|e| format!("PMC_ENABLE write failed: {e}"))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let pmc_after = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);

    // Acknowledge pending PRI ring interrupts
    let pri_intr = bar0.read_u32(pri::INTR_STATUS as usize).unwrap_or(0);
    if pri_intr != 0 {
        bar0.write_u32(pri::COMMAND as usize, 0x2)
            .map_err(|e| format!("PRI ring ack failed: {e}"))?;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Enumerate PRI ring stations
    bar0.write_u32(pri::COMMAND as usize, 0x4)
        .map_err(|e| format!("PRI ring enumerate failed: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    let status_enum = bar0.read_u32(pri::STATUS_ENUM as usize).unwrap_or(0xFF);

    // Start PRI ring
    bar0.write_u32(pri::COMMAND as usize, 0x1)
        .map_err(|e| format!("PRI ring start failed: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    let status_start = bar0.read_u32(pri::STATUS_ENUM as usize).unwrap_or(0xFF);

    // Verify falcon accessibility
    let fecs_cpuctl = bar0
        .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
        .unwrap_or(0xDEAD);
    let fecs_pc = bar0
        .read_u32((falcon::FECS_BASE + 0x11C) as usize)
        .unwrap_or(0xDEAD);
    let gpccs_cpuctl = bar0
        .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
        .unwrap_or(0xDEAD);
    let fecs_accessible = fecs_cpuctl & 0xBADF_0000 != 0xBADF_0000;
    let gpccs_accessible = gpccs_cpuctl & 0xBADF_0000 != 0xBADF_0000;

    tracing::info!(
        bdf,
        pmc_before = format_args!("{:#010x}", pmc_before),
        pmc_after = format_args!("{:#010x}", pmc_after),
        pgraph_was_on,
        pri_intr_before = format_args!("{:#010x}", pri_intr),
        status_after_enum = format_args!("{:#010x}", status_enum),
        status_after_start = format_args!("{:#010x}", status_start),
        fecs_cpuctl = format_args!("{:#010x}", fecs_cpuctl),
        fecs_pc = format_args!("{:#010x}", fecs_pc),
        gpccs_cpuctl = format_args!("{:#010x}", gpccs_cpuctl),
        fecs_accessible,
        gpccs_accessible,
        "PRI ring recovery complete"
    );

    // ── Post-recovery IMEM capture ──
    // Now that PGRAPH is enabled and falcon PIO is unblocked (no RM),
    // try to read FECS IMEM. If firmware survived unbind, we capture it.
    let fecs_base = falcon::FECS_BASE as usize;
    let imemc_reg = fecs_base + falcon::IMEMC as usize;
    let imemd_reg = fecs_base + falcon::IMEMD as usize;
    // Set IMEMC: address 0, auto-increment read (bit 25)
    let _ = bar0.write_u32(imemc_reg, 0x0200_0000);
    std::thread::sleep(std::time::Duration::from_micros(100));
    let mut imem_probe = [0u32; 8];
    for slot in &mut imem_probe {
        *slot = bar0.read_u32(imemd_reg).unwrap_or(0xDEAD_DEAD);
    }
    let imem_nonzero = imem_probe.iter().filter(|&&w| w != 0).count();
    tracing::info!(
        bdf,
        imem_nonzero,
        w0 = format_args!("{:#010x}", imem_probe[0]),
        w1 = format_args!("{:#010x}", imem_probe[1]),
        w2 = format_args!("{:#010x}", imem_probe[2]),
        w3 = format_args!("{:#010x}", imem_probe[3]),
        "post-recovery FECS IMEM probe"
    );

    let imem_status = if imem_nonzero > 0 {
        // Full FECS + GPCCS IMEM dump
        let fw_dir = "/var/lib/toadstool/catalysts/firmware";
        let _ = std::fs::create_dir_all(fw_dir);
        for (name, eng_base) in [
            ("fecs", falcon::FECS_BASE),
            ("gpccs", falcon::GPCCS_BASE),
        ] {
            let ic = eng_base + falcon::IMEMC;
            let id = eng_base + falcon::IMEMD;
            let imem_size = 32 * 1024usize;
            let _ = bar0.write_u32(ic as usize, 0x0200_0000);
            std::thread::sleep(std::time::Duration::from_micros(100));
            let mut fw_words = Vec::with_capacity(imem_size / 4);
            for _ in 0..(imem_size / 4) {
                fw_words.push(bar0.read_u32(id as usize).unwrap_or(0));
            }
            let fw_bytes: Vec<u8> = fw_words.iter()
                .flat_map(|w| w.to_le_bytes()).collect();
            let nz = fw_bytes.iter().filter(|&&b| b != 0).count();
            let fw_path = format!("{fw_dir}/{name}_imem_gv100.bin");
            let _ = std::fs::write(&fw_path, &fw_bytes);
            tracing::info!(engine = name, path = fw_path.as_str(),
                size = fw_bytes.len(), nonzero = nz,
                "{name} IMEM captured post-recovery");
        }
        format!(", IMEM={imem_nonzero}/8 words alive")
    } else {
        ", IMEM=wiped".into()
    };

    Ok(format!(
        "PMC {:#010x}→{:#010x}, PGRAPH={}, ring_status={:#x}, \
         FECS={} GPCCS={}{}",
        pmc_before, pmc_after,
        if pmc_after & (1 << 12) != 0 { "ON" } else { "OFF" },
        status_start,
        if fecs_accessible { "accessible" } else { "FAULT" },
        if gpccs_accessible { "accessible" } else { "FAULT" },
        imem_status,
    ))
}
