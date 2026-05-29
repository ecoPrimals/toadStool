// SPDX-License-Identifier: AGPL-3.0-only
//! capture_pmu_falcon — Capture GV100 PMU falcon state and IMEM/DMEM via PIO read.
//!
//! Usage: capture_pmu_falcon <resource0_path> [output_dir] [rw]

#![allow(unsafe_code, dead_code, non_snake_case, non_upper_case_globals)]
#![allow(
    clippy::unreadable_literal, clippy::borrow_as_ptr,
    clippy::cast_lossless, clippy::explicit_iter_loop,
    clippy::collapsible_if,
)]

use toadstool_cylinder::nv::registers::{falcon, gpc, pgraph, pmc, pmu};
use std::io;
use std::os::fd::AsFd;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{fence, Ordering};

const BAR0_SIZE: usize = 16 * 1024 * 1024;

/// Legacy SEC2 base (pre-GV100 topology — retained for diagnostic comparison).
const SEC2_BASE_LEGACY: u32 = 0x840000;

use toadstool_cylinder::bin_helpers::Bar0;

fn cpuctl_state(cpuctl: u32) -> &'static str {
    if cpuctl & 0x20 != 0 {
        "RUNNING"
    } else if cpuctl & 0x10 != 0 {
        "HALTED"
    } else {
        "UNKNOWN"
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <resource0_path> [output_dir] [rw]", args[0]);
        return ExitCode::from(1);
    }

    let res_path = &args[1];
    let out_dir = args.get(2).map_or("/tmp/pmu_capture", String::as_str);
    let rw = args.get(3).is_some_and(|a| a == "rw");

    let file = if rw {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(res_path)
    } else {
        std::fs::File::open(res_path)
    };
    let file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!("open resource0: {e}");
            return ExitCode::from(1);
        }
    };

    // SAFETY: BAR0 resource0 is a valid MMIO region for this GPU BDF.
    let bar0 = unsafe {
        match Bar0::map_with_prot(file.as_fd(), BAR0_SIZE, rw) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("mmap: {e}");
                return ExitCode::from(1);
            }
        }
    };

    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("mkdir {out_dir}: {e}");
        return ExitCode::from(1);
    }

    println!("=== GV100 Falcon State (nvidia-470 LIVE) ===");
    let pmc_boot0 = bar0.r32(pmc::BOOT0);
    let pmc_enable = bar0.r32(pmc::ENABLE);
    println!("PMC_BOOT_0: 0x{pmc_boot0:08x}");
    println!("PMC_ENABLE: 0x{pmc_enable:08x}");

    println!("\n--- PMU Falcon ---");
    let cpuctl = bar0.r32(pmu::BASE + falcon::CPUCTL);
    let hwcfg = bar0.r32(pmu::BASE + falcon::HWCFG);
    println!(
        "CPUCTL:  0x{cpuctl:08x} ({})",
        cpuctl_state(cpuctl)
    );
    println!("HWCFG:   0x{hwcfg:08x}");
    println!("PC:      0x{:08x}", bar0.r32(pmu::BASE + falcon::PC));
    println!("SCTL:    0x{:08x}", bar0.r32(pmu::BASE + falcon::SCTL));
    let mb0 = bar0.r32(pmu::BASE + falcon::MAILBOX0);
    let mb1 = bar0.r32(pmu::BASE + falcon::MAILBOX1);
    let exci = bar0.r32(pmu::BASE + falcon::EXCI);
    let bootvec = bar0.r32(pmu::BASE + falcon::BOOTVEC);
    println!("MB0:     0x{mb0:08x}");
    println!("MB1:     0x{mb1:08x}");
    println!("EXCI:    0x{exci:08x}");
    println!("BOOTVEC: 0x{bootvec:08x}");

    let imem_pages = ((hwcfg >> 9) & 0x1FF) as i32;
    let dmem_pages = (hwcfg & 0x1FF) as i32;
    let imem_bytes = imem_pages * 256;
    let dmem_bytes = dmem_pages * 256;
    println!(
        "IMEM: {imem_pages} pages ({} KB), DMEM: {dmem_pages} pages ({} KB)",
        imem_bytes / 1024,
        dmem_bytes / 1024
    );

    println!("\n--- FECS Falcon ---");
    println!("CPUCTL: 0x{:08x}", bar0.r32(falcon::FECS_BASE + falcon::CPUCTL));
    println!("HWCFG:  0x{:08x}", bar0.r32(falcon::FECS_BASE + falcon::HWCFG));
    println!("PC:     0x{:08x}", bar0.r32(falcon::FECS_BASE + falcon::PC));
    println!("SCTL:   0x{:08x}", bar0.r32(falcon::FECS_BASE + falcon::SCTL));

    println!("\n--- GPCCS Falcon ---");
    println!("CPUCTL: 0x{:08x}", bar0.r32(falcon::GPCCS_BASE + falcon::CPUCTL));
    println!("HWCFG:  0x{:08x}", bar0.r32(falcon::GPCCS_BASE + falcon::HWCFG));
    println!("PC:     0x{:08x}", bar0.r32(falcon::GPCCS_BASE + falcon::PC));

    println!("\n--- SEC2 Falcon ---");
    println!(
        "CPUCTL: 0x{:08x}",
        bar0.r32(SEC2_BASE_LEGACY + falcon::CPUCTL)
    );
    println!(
        "MB0:    0x{:08x}",
        bar0.r32(SEC2_BASE_LEGACY + falcon::MAILBOX0)
    );
    println!("PC:     0x{:08x}", bar0.r32(SEC2_BASE_LEGACY + falcon::PC));

    println!("\n--- GR Engine ---");
    let gr_status = bar0.r32(pgraph::STATUS);
    println!("GR_STATUS:     0x{gr_status:08x}");
    println!("GR_FECS_OS:    0x{:08x}", bar0.r32(0x409500));
    println!("PGRAPH_STATUS: 0x{:08x}", bar0.r32(0x400110));

    println!("\n--- TPC Power Status ---");
    let mut tpc_status = Vec::new();
    for gpc_id in 0..6 {
        let tpc_en = bar0.r32(gpc::tpc_enable(gpc_id));
        println!("GPC{gpc_id} TPC_EN: 0x{tpc_en:08x}");
        tpc_status.push(serde_json::json!({"gpc": gpc_id, "tpc_en": format!("0x{tpc_en:08x}")}));
    }

    let mut capture = serde_json::json!({
        "pmc_boot0": format!("0x{pmc_boot0:08x}"),
        "pmc_enable": format!("0x{pmc_enable:08x}"),
        "pmu": {
            "cpuctl": format!("0x{cpuctl:08x}"),
            "hwcfg": format!("0x{hwcfg:08x}"),
            "mb0": format!("0x{mb0:08x}"),
            "imem_bytes": imem_bytes,
            "dmem_bytes": dmem_bytes,
        },
        "gr_status": format!("0x{gr_status:08x}"),
        "tpc": tpc_status,
        "rw_capture": rw,
    });

    if rw {
        println!("\n--- PMU IMEM Capture (PIO read, {} KB) ---", imem_bytes / 1024);
        let mut buf = vec![0u32; (imem_bytes / 4) as usize];
        bar0.w32(pmu::BASE + falcon::IMEMC, 0x02000000);
        for word in buf.iter_mut() {
            *word = bar0.r32(pmu::BASE + falcon::IMEMD);
        }

        let imem_nonzero = buf.iter().filter(|&&w| w != 0).count();
        println!("IMEM: {imem_nonzero}/{} words non-zero", imem_bytes / 4);

        let imem_path = Path::new(out_dir).join("pmu_imem.bin");
        let imem_bytes_u = imem_bytes as usize;
        let imem_raw: Vec<u8> = buf
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .take(imem_bytes_u)
            .collect();
        if let Err(e) = std::fs::write(&imem_path, &imem_raw) {
            eprintln!("write {}: {e}", imem_path.display());
            return ExitCode::from(1);
        }
        println!(
            "Written: {} ({imem_bytes} bytes)",
            imem_path.display()
        );

        println!("\n--- PMU DMEM Capture (PIO read, {} KB) ---", dmem_bytes / 1024);
        let mut buf = vec![0u32; (dmem_bytes / 4) as usize];
        bar0.w32(pmu::BASE + falcon::DMEMC, 0x02000000);
        for word in buf.iter_mut() {
            *word = bar0.r32(pmu::BASE + falcon::DMEMD);
        }

        let dmem_nonzero = buf.iter().filter(|&&w| w != 0).count();
        println!("DMEM: {dmem_nonzero}/{} words non-zero", dmem_bytes / 4);

        let dmem_path = Path::new(out_dir).join("pmu_dmem.bin");
        let dmem_bytes_u = dmem_bytes as usize;
        let dmem_raw: Vec<u8> = buf
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .take(dmem_bytes_u)
            .collect();
        if let Err(e) = std::fs::write(&dmem_path, &dmem_raw) {
            eprintln!("write {}: {e}", dmem_path.display());
            return ExitCode::from(1);
        }
        println!(
            "Written: {} ({dmem_bytes} bytes)",
            dmem_path.display()
        );

        capture["imem_nonzero_words"] = serde_json::json!(imem_nonzero);
        capture["dmem_nonzero_words"] = serde_json::json!(dmem_nonzero);
        capture["dmem_path"] = serde_json::json!(dmem_path.display().to_string());
        capture["imem_path"] = serde_json::json!(imem_path.display().to_string());

        println!("\n--- FECS IMEM Test (HS mode — expect zeros) ---");
        bar0.w32(falcon::FECS_BASE + falcon::IMEMC, 0x02000000);
        let fecs_w0 = bar0.r32(falcon::FECS_BASE + falcon::IMEMD);
        let fecs_w1 = bar0.r32(falcon::FECS_BASE + falcon::IMEMD);
        let hs_protected = fecs_w0 == 0 && fecs_w1 == 0;
        println!(
            "FECS IMEM[0]: 0x{fecs_w0:08x}, [1]: 0x{fecs_w1:08x} ({})",
            if hs_protected {
                "HS-protected"
            } else {
                "READABLE"
            }
        );
        capture["fecs_imem_hs_protected"] = serde_json::json!(hs_protected);
    }

    fence(Ordering::SeqCst);

    println!(
        "{}",
        serde_json::to_string_pretty(&capture).unwrap_or_default()
    );

    ExitCode::SUCCESS
}
