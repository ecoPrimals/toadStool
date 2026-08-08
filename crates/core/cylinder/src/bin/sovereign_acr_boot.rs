// SPDX-License-Identifier: AGPL-3.0-or-later
//! sovereign_acr_boot — Sovereign ACR boot for GV100 (Titan V).
//!
//! Stages signed firmware to VRAM via PRAMIN window, writes ACR descriptor
//! to PMU DMEM, and triggers ACR execution. Requires post-SBR cold state
//! with PMU HS ROM running (MB0=0x300).
//!
//! Usage: sovereign_acr_boot <resource0_path> <firmware_dir> [--dry-run]

#![allow(
    unsafe_code,
    dead_code,
    non_snake_case,
    non_upper_case_globals,
    reason = "sovereign ACR falcon boot diagnostic binary — cylinder containment zone"
)]
#![allow(
    unused_variables,
    unused_assignments,
    clippy::unreadable_literal,
    clippy::borrow_as_ptr,
    clippy::manual_div_ceil,
    clippy::map_unwrap_or,
    clippy::needless_pass_by_value,
    clippy::cast_lossless,
    clippy::explicit_iter_loop,
    clippy::unnecessary_unwrap,
    clippy::collapsible_if
)]

#[cfg(target_os = "linux")]
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::AsFd;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::ExitCode;
#[cfg(target_os = "linux")]
use std::sync::atomic::{Ordering, fence};
#[cfg(target_os = "linux")]
use std::thread;
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
use toadstool_cylinder::nv::registers::{falcon, pbus, pfb, pgraph, pmc, pramin};
#[cfg(target_os = "linux")]
use toadstool_hw_safe::open_path;

#[cfg(target_os = "linux")]
const BAR0_SIZE: usize = 16 * 1024 * 1024;

/* PMU falcon (base 0x10A000) — non-standard offset layout for ACR descriptor PIO */
#[cfg(target_os = "linux")]
const PMU_BASE: u32 = falcon::PMU_BASE;
#[cfg(target_os = "linux")]
const PMU_FALCON_IRQMASK: u32 = PMU_BASE + 0x014;
#[cfg(target_os = "linux")]
const PMU_FALCON_IRQDEST: u32 = PMU_BASE + 0x01C;
#[cfg(target_os = "linux")]
const PMU_FALCON_IRQEN: u32 = PMU_BASE + 0x010;
#[cfg(target_os = "linux")]
const PMU_FALCON_MAILBOX0: u32 = PMU_BASE + 0x040;
#[cfg(target_os = "linux")]
const PMU_FALCON_MAILBOX1: u32 = PMU_BASE + 0x044;
#[cfg(target_os = "linux")]
const PMU_FALCON_ITFEN: u32 = PMU_BASE + 0x050;
#[cfg(target_os = "linux")]
const PMU_FALCON_CPUCTL: u32 = PMU_BASE + 0x100;
#[cfg(target_os = "linux")]
const PMU_FALCON_IMEMC: u32 = PMU_BASE + 0x104;
#[cfg(target_os = "linux")]
const PMU_FALCON_IMEMD: u32 = PMU_BASE + 0x108;
#[cfg(target_os = "linux")]
const PMU_FALCON_BOOTVEC: u32 = PMU_BASE + 0x110;
#[cfg(target_os = "linux")]
const PMU_FALCON_HWCFG: u32 = PMU_BASE + 0x11C;
#[cfg(target_os = "linux")]
const PMU_FALCON_DMACTL: u32 = PMU_BASE + 0x148;
#[cfg(target_os = "linux")]
const PMU_FALCON_OS: u32 = PMU_BASE + 0x180;
#[cfg(target_os = "linux")]
const PMU_FALCON_SCTL: u32 = PMU_BASE + 0x240;

#[cfg(target_os = "linux")]
const fn PMU_FALCON_DMEMC(p: u32) -> u32 {
    PMU_BASE + 0x1C0 + p * 8
}
#[cfg(target_os = "linux")]
const fn PMU_FALCON_DMEMD(p: u32) -> u32 {
    PMU_BASE + 0x1C4 + p * 8
}

/* FECS falcon */
#[cfg(target_os = "linux")]
const FECS_BASE: u32 = falcon::FECS_BASE;
#[cfg(target_os = "linux")]
const FECS_FALCON_CPUCTL: u32 = FECS_BASE + falcon::CPUCTL;
#[cfg(target_os = "linux")]
const FECS_FALCON_MAILBOX0: u32 = FECS_BASE + falcon::MAILBOX0;
#[cfg(target_os = "linux")]
const FECS_FALCON_MAILBOX1: u32 = FECS_BASE + falcon::MAILBOX1;
#[cfg(target_os = "linux")]
const FECS_FALCON_SCTL: u32 = FECS_BASE + falcon::SCTL;
#[cfg(target_os = "linux")]
const FECS_FALCON_OS: u32 = FECS_BASE + falcon::OS;

/* GPCCS falcon */
#[cfg(target_os = "linux")]
const GPCCS_BASE: u32 = falcon::GPCCS_BASE;
#[cfg(target_os = "linux")]
const GPCCS_FALCON_CPUCTL: u32 = GPCCS_BASE + falcon::CPUCTL;
#[cfg(target_os = "linux")]
const GPCCS_FALCON_MAILBOX0: u32 = GPCCS_BASE + falcon::MAILBOX0;

/* VRAM address where we stage firmware (arbitrary, within first 64 MB) */
#[cfg(target_os = "linux")]
const FW_STAGING_VRAM_BASE: u64 = 0x01000000; /* 16 MB into VRAM */

#[cfg(target_os = "linux")]
use toadstool_cylinder::bin_helpers::Bar0;

#[cfg(target_os = "linux")]
fn load_file(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

#[cfg(target_os = "linux")]
fn cpuctl_state(cpuctl: u32) -> &'static str {
    if cpuctl & 0x20 != 0 {
        "RUNNING"
    } else if cpuctl & 0x10 != 0 {
        "HALTED"
    } else {
        "???"
    }
}

#[cfg(target_os = "linux")]
fn stage_to_vram(bar0: &Bar0, data: &[u8], vram_addr: u64, dry_run: bool) -> io::Result<()> {
    let len = data.len();
    let mut offset = 0usize;
    while offset < len {
        let page_base = (vram_addr + offset as u64) & !0xFFFF;
        let page_off = ((vram_addr + offset as u64) & 0xFFFF) as u32;
        let window_val = (page_base >> 16) as u32;

        if !dry_run {
            bar0.w32(pbus::BAR0_WINDOW, window_val);
            fence(Ordering::SeqCst);
            let readback = bar0.r32(pbus::BAR0_WINDOW);
            if readback != window_val {
                println!(
                    "  [FAIL] BAR0_WINDOW readback mismatch: wrote 0x{window_val:08X}, read 0x{readback:08X}"
                );
                return Err(io::Error::other("BAR0_WINDOW readback mismatch"));
            }
        }

        let mut chunk = 0x10000 - page_off as usize;
        if chunk > len - offset {
            chunk = len - offset;
        }

        if !dry_run {
            let mut i = 0usize;
            while i < chunk {
                let copy_len = if i + 4 <= chunk { 4 } else { chunk - i };
                let mut word = 0u32;
                for b in 0..copy_len {
                    word |= (data[offset + i + b] as u32) << (b * 8);
                }
                bar0.w32(pramin::BASE + page_off + i as u32, word);
                i += 4;
            }
        }

        offset += chunk;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_pmu_dmem(bar0: &Bar0, dmem_offset: u32, data: &[u32], dry_run: bool) {
    if dry_run {
        return;
    }
    bar0.w32(PMU_FALCON_DMEMC(0), (1 << 24) | (dmem_offset & 0xFFFF));
    for &word in data {
        bar0.w32(PMU_FALCON_DMEMD(0), word);
    }
}

#[cfg(target_os = "linux")]
fn read_pmu_dmem(bar0: &Bar0, dmem_offset: u32) -> u32 {
    bar0.w32(PMU_FALCON_DMEMC(0), dmem_offset & 0xFFFF);
    bar0.r32(PMU_FALCON_DMEMD(0))
}

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <resource0_path> <firmware_dir> [--dry-run]",
            args[0]
        );
        eprintln!("  firmware_dir should contain gr/ and acr/ subdirs");
        return ExitCode::from(1);
    }

    let res0_path = &args[1];
    let fw_dir = Path::new(&args[2]);
    let dry_run = args.get(3).is_some_and(|a| a == "--dry-run");

    if dry_run {
        println!("*** DRY RUN — no hardware writes ***\n");
    }

    let file = match open_path(Path::new(res0_path), true, true) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("open resource0: {e}");
            return ExitCode::from(1);
        }
    };

    // SAFETY: BAR0 resource0 is a valid MMIO region for this GPU BDF.
    let bar0 = unsafe {
        match Bar0::map(file.as_fd(), BAR0_SIZE) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("mmap resource0: {e}");
                return ExitCode::from(1);
            }
        }
    };

    let mut fatal = false;
    let mut fecs_running = false;

    /* Phase 0: Verify GPU identity and state */
    println!("=== Phase 0: GPU Identity & State ===");
    let boot0 = bar0.r32(pmc::BOOT0);
    println!("  BOOT0:       0x{boot0:08X}");
    if boot0 == 0xFFFF_FFFF {
        println!("  [FATAL] GPU in link-down state. Run SBR + rescan first.");
        fatal = true;
    }

    if !fatal {
        let pmc_enable = bar0.r32(pmc::ENABLE);
        let pmu_cpuctl = bar0.r32(PMU_FALCON_CPUCTL);
        let pmu_mb0 = bar0.r32(PMU_FALCON_MAILBOX0);
        let pmu_sctl = bar0.r32(PMU_FALCON_SCTL);
        let fecs_cpuctl = bar0.r32(FECS_FALCON_CPUCTL);

        println!("  PMC_ENABLE:  0x{pmc_enable:08X}");
        println!(
            "  PMU CPUCTL:  0x{pmu_cpuctl:08X} ({})",
            cpuctl_state(pmu_cpuctl)
        );
        println!("  PMU MB0:     0x{pmu_mb0:08X}");
        println!(
            "  PMU SCTL:    0x{pmu_sctl:08X} (HS mode {})",
            pmu_sctl & 0xF
        );
        println!(
            "  FECS CPUCTL: 0x{fecs_cpuctl:08X} ({})",
            cpuctl_state(fecs_cpuctl)
        );

        if pmu_cpuctl & 0x20 == 0 || pmu_mb0 != 0x300 {
            println!("  [FATAL] PMU not in expected post-SBR state (RUNNING + MB0=0x300).");
            println!("  Run SBR first to get PMU HS ROM to this state.");
            fatal = true;
        } else {
            println!("  [OK] PMU HS ROM running, ACR ready (MB0=0x300)\n");
        }
    }

    let mut fecs_blob: Option<Vec<u8>> = None;
    let mut gpccs_blob: Option<Vec<u8>> = None;
    let mut fecs_bl_padded = 0usize;
    let mut fecs_total = 0usize;
    let mut gpccs_total = 0usize;
    let fecs_vram_addr = FW_STAGING_VRAM_BASE;
    let gpccs_vram_addr = FW_STAGING_VRAM_BASE + 0x10000;

    if !fatal {
        /* Phase 1: Load firmware blobs */
        println!("=== Phase 1: Load Firmware Blobs ===");

        let acr_bl = load_file(&fw_dir.join("acr/bl.bin"));
        let acr_bl_size = acr_bl.as_ref().map_or(0, Vec::len);
        println!(
            "  acr/bl.bin:         {} ({acr_bl_size} bytes)",
            if acr_bl.is_some() { "OK" } else { "MISSING" }
        );

        let fecs_bl = load_file(&fw_dir.join("gr/fecs_bl.bin"));
        let fecs_bl_size = fecs_bl.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/fecs_bl.bin:     {} ({fecs_bl_size} bytes)",
            if fecs_bl.is_some() { "OK" } else { "MISSING" }
        );

        let fecs_inst = load_file(&fw_dir.join("gr/fecs_inst.bin"));
        let fecs_inst_size = fecs_inst.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/fecs_inst.bin:   {} ({fecs_inst_size} bytes)",
            if fecs_inst.is_some() { "OK" } else { "MISSING" }
        );

        let fecs_data = load_file(&fw_dir.join("gr/fecs_data.bin"));
        let fecs_data_size = fecs_data.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/fecs_data.bin:   {} ({fecs_data_size} bytes)",
            if fecs_data.is_some() { "OK" } else { "MISSING" }
        );

        let gpccs_bl = load_file(&fw_dir.join("gr/gpccs_bl.bin"));
        let gpccs_bl_size = gpccs_bl.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/gpccs_bl.bin:    {} ({gpccs_bl_size} bytes)",
            if gpccs_bl.is_some() { "OK" } else { "MISSING" }
        );

        let gpccs_inst = load_file(&fw_dir.join("gr/gpccs_inst.bin"));
        let gpccs_inst_size = gpccs_inst.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/gpccs_inst.bin:  {} ({gpccs_inst_size} bytes)",
            if gpccs_inst.is_some() {
                "OK"
            } else {
                "MISSING"
            }
        );

        let gpccs_data = load_file(&fw_dir.join("gr/gpccs_data.bin"));
        let gpccs_data_size = gpccs_data.as_ref().map_or(0, Vec::len);
        println!(
            "  gr/gpccs_data.bin:  {} ({gpccs_data_size} bytes)",
            if gpccs_data.is_some() {
                "OK"
            } else {
                "MISSING"
            }
        );

        let sec2_image = load_file(&fw_dir.join("sec2/image.bin"));
        let sec2_image_size = sec2_image.as_ref().map_or(0, Vec::len);
        println!(
            "  sec2/image.bin:     {} ({sec2_image_size} bytes)",
            if sec2_image.is_some() {
                "OK"
            } else {
                "MISSING"
            }
        );

        let sec2_desc = load_file(&fw_dir.join("sec2/desc.bin"));
        let sec2_desc_size = sec2_desc.as_ref().map_or(0, Vec::len);
        println!(
            "  sec2/desc.bin:      {} ({sec2_desc_size} bytes)",
            if sec2_desc.is_some() { "OK" } else { "MISSING" }
        );

        let sec2_sig = load_file(&fw_dir.join("sec2/sig.bin"));
        let sec2_sig_size = sec2_sig.as_ref().map_or(0, Vec::len);
        println!(
            "  sec2/sig.bin:       {} ({sec2_sig_size} bytes)",
            if sec2_sig.is_some() { "OK" } else { "MISSING" }
        );

        if let (
            Some(fecs_bl),
            Some(fecs_inst),
            Some(fecs_data),
            Some(gpccs_bl),
            Some(gpccs_inst),
            Some(gpccs_data),
        ) = (
            fecs_bl, fecs_inst, fecs_data, gpccs_bl, gpccs_inst, gpccs_data,
        ) {
            println!("  [OK] All critical blobs loaded.\n");

            /* Phase 2: Stage firmware to VRAM via PRAMIN */
            println!("=== Phase 2: Stage Firmware to VRAM ===");

            fecs_bl_padded = (fecs_bl_size + 0xFF) & !0xFF;
            fecs_total = fecs_bl_padded + fecs_inst_size + fecs_data_size;
            let mut fecs_buf = vec![0u8; fecs_total + 0x100];
            fecs_buf[..fecs_bl_size].copy_from_slice(&fecs_bl);
            fecs_buf[fecs_bl_padded..fecs_bl_padded + fecs_inst_size].copy_from_slice(&fecs_inst);
            fecs_buf
                [fecs_bl_padded + fecs_inst_size..fecs_bl_padded + fecs_inst_size + fecs_data_size]
                .copy_from_slice(&fecs_data);

            println!("  FECS blob: {fecs_total} bytes → VRAM 0x{fecs_vram_addr:08X}");
            println!(
                "    bl: {fecs_bl_size} bytes at +0, inst: {fecs_inst_size} at +0x{fecs_bl_padded:X}, data: {fecs_data_size} at +0x{:X}",
                fecs_bl_padded + fecs_inst_size
            );

            if stage_to_vram(&bar0, &fecs_buf[..fecs_total], fecs_vram_addr, dry_run).is_err() {
                println!("  [FAIL] FECS staging failed.");
                fatal = true;
            } else {
                println!("  [OK] FECS staged.");
                fecs_blob = Some(fecs_buf);
            }

            if !fatal {
                let gpccs_bl_padded = (gpccs_bl_size + 0xFF) & !0xFF;
                gpccs_total = gpccs_bl_padded + gpccs_inst_size + gpccs_data_size;
                let mut gpccs_buf = vec![0u8; gpccs_total + 0x100];
                gpccs_buf[..gpccs_bl_size].copy_from_slice(&gpccs_bl);
                gpccs_buf[gpccs_bl_padded..gpccs_bl_padded + gpccs_inst_size]
                    .copy_from_slice(&gpccs_inst);
                gpccs_buf[gpccs_bl_padded + gpccs_inst_size
                    ..gpccs_bl_padded + gpccs_inst_size + gpccs_data_size]
                    .copy_from_slice(&gpccs_data);

                println!("  GPCCS blob: {gpccs_total} bytes → VRAM 0x{gpccs_vram_addr:08X}");

                if stage_to_vram(&bar0, &gpccs_buf[..gpccs_total], gpccs_vram_addr, dry_run)
                    .is_err()
                {
                    println!("  [FAIL] GPCCS staging failed.");
                    fatal = true;
                } else {
                    println!("  [OK] GPCCS staged.");
                    gpccs_blob = Some(gpccs_buf);
                }
            }

            if !fatal {
                if !dry_run {
                    if let Some(ref blob) = fecs_blob {
                        println!("  Verifying FECS PRAMIN readback...");
                        bar0.w32(pbus::BAR0_WINDOW, (fecs_vram_addr >> 16) as u32);
                        fence(Ordering::SeqCst);
                        let first_word = bar0.r32(pramin::BASE);
                        let expected = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
                        println!(
                            "    First word: wrote 0x{expected:08X}, read 0x{first_word:08X} — {}",
                            if first_word == expected {
                                "MATCH"
                            } else {
                                "MISMATCH"
                            }
                        );
                    }
                }
                println!();

                /* Phase 3: Write ACR descriptor to PMU DMEM */
                println!("=== Phase 3: Write ACR Descriptor to PMU DMEM ===");

                println!("  Pre-write DMEM[0x00-0x3F]:");
                for i in 0..16 {
                    let val = read_pmu_dmem(&bar0, i * 4);
                    println!("    [0x{:02X}] = 0x{val:08X}", i * 4);
                }

                let gpccs_bl_padded = (gpccs_bl_size + 0xFF) & !0xFF;
                let mut acr_desc = [0u32; 21];
                acr_desc[8] = 0x00000004;
                acr_desc[9] = (fecs_vram_addr & 0xFFFF_FFFF) as u32;
                acr_desc[10] = (fecs_vram_addr >> 32) as u32;
                acr_desc[11] = 0x00000000;
                acr_desc[12] = fecs_bl_padded as u32;
                acr_desc[13] = fecs_bl_padded as u32;
                acr_desc[14] = fecs_total as u32;
                acr_desc[15] = 0x00000000;
                acr_desc[16] = (gpccs_vram_addr & 0xFFFF_FFFF) as u32;
                acr_desc[17] = (gpccs_vram_addr >> 32) as u32;
                acr_desc[18] = gpccs_total as u32;
                acr_desc[19] = 0x00000034;
                acr_desc[20] = 0x0000002F;

                println!("  ACR descriptor to write:");
                for (i, &val) in acr_desc.iter().enumerate() {
                    println!("    DMEM[0x{:02X}] = 0x{val:08X}", i * 4);
                }

                write_pmu_dmem(&bar0, 0x0000, &acr_desc, dry_run);

                if !dry_run {
                    println!("  Verifying DMEM readback...");
                    let mut mismatches = 0;
                    for (i, &expected) in acr_desc.iter().enumerate() {
                        let val = read_pmu_dmem(&bar0, (i * 4) as u32);
                        if val != expected {
                            println!(
                                "    [0x{:02X}]: wrote 0x{expected:08X}, read 0x{val:08X} — MISMATCH",
                                i * 4
                            );
                            mismatches += 1;
                        }
                    }
                    if mismatches == 0 {
                        println!("    [OK] All 21 words match.");
                    } else {
                        println!(
                            "    [WARN] {mismatches} mismatches — PMU DMEM may be HS-protected."
                        );
                    }
                }
                println!();

                /* Phase 4: Trigger ACR via PMU mailbox */
                println!("=== Phase 4: Trigger ACR Execution ===");
                println!("  Current PMU MB0: 0x{:08X}", bar0.r32(PMU_FALCON_MAILBOX0));
                println!("  Current PMU MB1: 0x{:08X}", bar0.r32(PMU_FALCON_MAILBOX1));

                if !dry_run {
                    bar0.w32(PMU_FALCON_MAILBOX1, (FW_STAGING_VRAM_BASE >> 8) as u32);
                    fence(Ordering::SeqCst);

                    bar0.w32(PMU_FALCON_MAILBOX0, 0x00000001);
                    fence(Ordering::SeqCst);

                    println!(
                        "  Wrote MB1 = 0x{:08X} (staging addr >> 8)",
                        (FW_STAGING_VRAM_BASE >> 8) as u32
                    );
                    println!("  Wrote MB0 = 0x00000001 (ACR trigger)");

                    println!("  Polling PMU MB0 for response...");
                    for i in 0..100 {
                        thread::sleep(Duration::from_millis(10));
                        let mb0 = bar0.r32(PMU_FALCON_MAILBOX0);
                        let cpuctl = bar0.r32(PMU_FALCON_CPUCTL);
                        if mb0 != 0x00000001 || i < 5 || i % 10 == 0 {
                            println!(
                                "    [{:3} ms] MB0=0x{mb0:08X} CPUCTL=0x{cpuctl:08X}",
                                (i + 1) * 10
                            );
                        }
                        if mb0 != 0x00000001 {
                            println!("    PMU responded: MB0 = 0x{mb0:08X}");
                            break;
                        }
                        if cpuctl & 0x20 == 0 {
                            println!("    PMU stopped running! CPUCTL = 0x{cpuctl:08X}");
                            break;
                        }
                    }
                }
                println!();

                /* Phase 5: Check results */
                println!("=== Phase 5: Post-ACR State ===");
                let pmu_cpuctl = bar0.r32(PMU_FALCON_CPUCTL);
                let pmu_mb0 = bar0.r32(PMU_FALCON_MAILBOX0);
                let fecs_cpuctl = bar0.r32(FECS_FALCON_CPUCTL);
                let fecs_mb0 = bar0.r32(FECS_FALCON_MAILBOX0);
                let fecs_os = bar0.r32(FECS_FALCON_OS);
                let gpccs_cpuctl = bar0.r32(GPCCS_FALCON_CPUCTL);
                let gpccs_mb0 = bar0.r32(GPCCS_FALCON_MAILBOX0);
                let gr_status = bar0.r32(pgraph::STATUS);
                let wpr2_lo = bar0.r32(pfb::WPR2_ADDR_LO);
                let wpr2_hi = bar0.r32(pfb::WPR2_ADDR_HI);
                let wpr2_ctrl = bar0.r32(pfb::WPR2_CTRL);

                println!("  PMU:   CPUCTL=0x{pmu_cpuctl:08X} MB0=0x{pmu_mb0:08X}");
                println!(
                    "  FECS:  CPUCTL=0x{fecs_cpuctl:08X} MB0=0x{fecs_mb0:08X} OS=0x{fecs_os:08X}"
                );
                println!("  GPCCS: CPUCTL=0x{gpccs_cpuctl:08X} MB0=0x{gpccs_mb0:08X}");
                println!("  GR_STATUS: 0x{gr_status:08X}");
                println!("  WPR2: LO=0x{wpr2_lo:08X} HI=0x{wpr2_hi:08X} CTRL=0x{wpr2_ctrl:08X}");

                fecs_running = fecs_cpuctl & 0x20 != 0;
                if fecs_running {
                    println!("\n  *** FECS IS RUNNING — ACR may have succeeded! ***");
                } else if fecs_cpuctl & 0x10 != 0 {
                    println!("\n  FECS still halted. ACR did not load firmware to FECS.");
                }

                let result = serde_json::json!({
                    "tool": "sovereign_acr_boot",
                    "dry_run": dry_run,
                    "fatal": fatal,
                    "fecs_running": fecs_running,
                    "pmu": {
                        "cpuctl": format!("0x{pmu_cpuctl:08X}"),
                        "mb0": format!("0x{pmu_mb0:08X}"),
                    },
                    "fecs": {
                        "cpuctl": format!("0x{fecs_cpuctl:08X}"),
                        "mb0": format!("0x{fecs_mb0:08X}"),
                        "os": format!("0x{fecs_os:08X}"),
                    },
                    "gr_status": format!("0x{gr_status:08X}"),
                    "wpr2": {
                        "lo": format!("0x{wpr2_lo:08X}"),
                        "hi": format!("0x{wpr2_hi:08X}"),
                        "ctrl": format!("0x{wpr2_ctrl:08X}"),
                    },
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        } else {
            println!("  [FATAL] Missing critical firmware blobs.");
            fatal = true;
        }

        drop(acr_bl);
        drop(sec2_image);
        drop(sec2_desc);
        drop(sec2_sig);
    }

    drop(fecs_blob);
    drop(gpccs_blob);

    ExitCode::SUCCESS
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This tool requires Linux");
    std::process::exit(1);
}
