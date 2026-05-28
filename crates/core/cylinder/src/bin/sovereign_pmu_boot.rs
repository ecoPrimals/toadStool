// SPDX-License-Identifier: AGPL-3.0-only
//! sovereign_pmu_boot — Sovereign PMU boot via PRAMIN+DMATRF (bypasses HS mode).
//!
//! HS mode 3 blocks PIO IMEM writes. nvidia-470 uses DMA transfer (DMATRF)
//! to load firmware from VRAM into falcon IMEM, which bypasses HS protection.
//!
//! Usage:
//!   sovereign_pmu_boot <resource0> <pmu_imem.bin> <pmu_dmem.bin> [--dry-run]

#![allow(unsafe_code, dead_code, non_snake_case, non_upper_case_globals)]
#![allow(
    unused_variables, unused_assignments,
    clippy::unreadable_literal, clippy::borrow_as_ptr,
    clippy::manual_div_ceil, clippy::map_unwrap_or,
    clippy::needless_pass_by_value, clippy::cast_lossless,
    clippy::explicit_iter_loop, clippy::unnecessary_unwrap,
)]

use toadstool_cylinder::nv::registers::{falcon, gpc, pgraph, pbus, pmc, pmu, pramin};
use std::io;
use std::os::fd::AsFd;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

const BAR0_SIZE: usize = 16 * 1024 * 1024;

/// Legacy SEC2 base (pre-GV100 topology).
const SEC2_BASE_LEGACY: u32 = 0x840000;
/// WPR2 shadow registers (distinct from PFB WPR2_ADDR_*).
const WPR2_LO: u32 = 0x1FA824;
const WPR2_HI: u32 = 0x1FA828;

struct Bar0 {
    ptr: *mut u32,
    len: usize,
}

impl Bar0 {
    /// # Safety
    /// `fd` must be an open file descriptor to a PCI BAR0 resource file and
    /// `size` must not exceed the device BAR region. Caller ensures exclusive
    /// access to the mapped region (single-threaded diagnostic binary).
    unsafe fn map(fd: std::os::fd::BorrowedFd, size: usize) -> io::Result<Self> {
        // SAFETY: fd is a valid sysfs resource0 file; size is BAR0_SIZE (16 MiB)
        // matching GPU BAR0; MAP_SHARED is required for MMIO coherency.
        let ptr = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                fd,
                0,
            )
        }
        .map_err(|e| io::Error::from_raw_os_error(e.raw_os_error()))?;
        Ok(Self {
            ptr: ptr.cast(),
            len: size,
        })
    }

    fn r32(&self, offset: u32) -> u32 {
        // SAFETY: offset is validated by caller to be within BAR0_SIZE; volatile
        // read is required for MMIO register semantics (no reordering/elision).
        unsafe { std::ptr::read_volatile(self.ptr.add(offset as usize / 4)) }
    }

    fn w32(&self, offset: u32, val: u32) {
        // SAFETY: offset is validated by caller to be within BAR0_SIZE; volatile
        // write is required for MMIO register semantics.
        unsafe { std::ptr::write_volatile(self.ptr.add(offset as usize / 4), val) }
    }
}

impl Drop for Bar0 {
    fn drop(&mut self) {
        // SAFETY: ptr and len were set by a successful mmap in Self::map;
        // Drop runs exactly once.
        unsafe {
            let _ = rustix::mm::munmap(self.ptr.cast(), self.len);
        }
    }
}

fn le_word(data: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([
        data[i * 4],
        data[i * 4 + 1],
        data[i * 4 + 2],
        data[i * 4 + 3],
    ])
}

fn print_falcon(bar0: &Bar0, name: &str, base: u32) {
    let ctrl = bar0.r32(base + 0x100);
    let pc = bar0.r32(base + 0x030);
    let sctl = bar0.r32(base + 0x240);
    let state = if ctrl == 0xbadf1100 {
        "NOT POWERED"
    } else if ctrl & 0x20 != 0 {
        "RUNNING"
    } else if ctrl & 0x10 != 0 {
        "HALTED"
    } else {
        "UNKNOWN"
    };
    println!(
        "  {name:<6}: cpuctl=0x{ctrl:08x} pc=0x{pc:08x} sctl=0x{sctl:08x} [{state}]"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <resource0> <pmu_imem.bin> <pmu_dmem.bin> [--dry-run]",
            args[0]
        );
        return ExitCode::from(1);
    }

    let res_path = &args[1];
    let imem_path = &args[2];
    let dmem_path = &args[3];
    let dry_run = args.get(4).is_some_and(|a| a == "--dry-run");

    println!("=== Sovereign PMU Boot via DMATRF — GV100 ===");
    println!("  dry-run: {}\n", if dry_run { "YES" } else { "NO" });

    let imem = match std::fs::read(imem_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("IMEM: {e}");
            return ExitCode::from(1);
        }
    };
    let dmem = match std::fs::read(dmem_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("DMEM: {e}");
            return ExitCode::from(1);
        }
    };

    let imem_size = imem.len() as i64;
    let dmem_size = dmem.len() as i64;
    println!("  IMEM: {imem_size} B, DMEM: {dmem_size} B\n");

    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(res_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("open: {e}");
            return ExitCode::from(1);
        }
    };

    // SAFETY: BAR0 resource0 is a valid MMIO region for this GPU BDF.
    let bar0 = unsafe {
        match Bar0::map(file.as_fd(), BAR0_SIZE) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("mmap: {e}");
                return ExitCode::from(1);
            }
        }
    };

    let boot0 = bar0.r32(pmc::BOOT0);
    println!(
        "  BOOT0: 0x{boot0:08x} {}",
        if boot0 == 0x140000a1 {
            "(GV100)"
        } else {
            "(?)"
        }
    );
    if boot0 == 0xFFFF_FFFF || boot0 == 0 {
        println!("FATAL: link dead");
        return ExitCode::from(1);
    }

    println!("\n--- Pre-boot state ---");
    print_falcon(&bar0, "PMU", pmu::BASE);
    print_falcon(&bar0, "FECS", falcon::FECS_BASE);
    print_falcon(&bar0, "GPCCS", falcon::GPCCS_BASE);
    print_falcon(&bar0, "SEC2", SEC2_BASE_LEGACY);

    let pmc_en = bar0.r32(pmc::ENABLE);
    let hwcfg = bar0.r32(pmu::BASE + falcon::HWCFG);
    let sctl = bar0.r32(pmu::BASE + falcon::SCTL);
    println!(
        "  PMC_ENABLE: 0x{pmc_en:08x}, PMU SCTL: 0x{sctl:08x} (HS={})",
        (sctl >> 12) & 3
    );
    println!(
        "  PMU MB0: 0x{:08x}, EXCI: 0x{:08x}\n",
        bar0.r32(pmu::BASE + falcon::MAILBOX0),
        bar0.r32(pmu::BASE + falcon::EXCI)
    );

    let mut booted = false;
    let mut dmatrf_ok = 0u32;
    let mut dmatrf_total = 0u32;

    if dry_run {
        println!("=== DRY RUN ===");
        print_summary(booted, dry_run, dmatrf_ok, dmatrf_total);
        return ExitCode::from(1);
    }

    /* Phase 1: Enable ITFEN (interface enable) for DMA engine */
    println!("--- Phase 1: Enable PMU DMA interface ---");
    bar0.w32(pmu::BASE + falcon::ITFEN, 0x00000004);
    println!(
        "  ITFEN <- 0x04, readback: 0x{:08x}",
        bar0.r32(pmu::BASE + falcon::ITFEN)
    );

    /* Phase 2: Stage firmware to VRAM via PRAMIN */
    println!("\n--- Phase 2: Stage IMEM to VRAM via PRAMIN ---");

    let vram_page: u32 = 0x00060000;
    bar0.w32(pbus::BAR0_WINDOW, vram_page >> 16);
    println!(
        "  PRAMIN window -> page 0x{vram_page:08x} (readback: 0x{:08x})",
        bar0.r32(pbus::BAR0_WINDOW)
    );

    let upload = if imem_size < 65536 { imem_size } else { 65536 };
    for i in 0..(upload / 4) {
        let word = le_word(&imem, i as usize);
        bar0.w32(pramin::BASE + (i * 4) as u32, word);
    }

    let pramin_v0 = bar0.r32(pramin::BASE);
    let pramin_exp = le_word(&imem, 0);
    println!(
        "  PRAMIN[0]: got=0x{pramin_v0:08x} want=0x{pramin_exp:08x} {}",
        if pramin_v0 == pramin_exp { "OK" } else { "FAIL" }
    );
    println!("  Staged {upload} B to VRAM@0x{vram_page:08x}");

    /* Phase 3: DMATRF from VRAM to PMU IMEM */
    println!("\n--- Phase 3: DMATRF VRAM->PMU IMEM ---");

    let block_size: u32 = 256;
    let n_blocks = ((upload as u32) + block_size - 1) / block_size;
    dmatrf_total = n_blocks;
    println!("  {n_blocks} blocks x 256B");

    let dma_base = vram_page;
    bar0.w32(pmu::DMATRFBASE, dma_base);
    println!(
        "  DMATRFBASE <- 0x{dma_base:08x} (readback: 0x{:08x})",
        bar0.r32(pmu::DMATRFBASE)
    );

    let t0 = Instant::now();
    let mut ok = 0u32;
    'dmatrf: for block in 0..n_blocks {
        let imem_off = block * block_size;
        let vram_off = block * block_size;

        bar0.w32(pmu::DMATRFMOFFS, imem_off);
        bar0.w32(pmu::DMATRFFBOFFS, vram_off);
        bar0.w32(pmu::DMATRFCMD, 0x00000012);

        let mut timeout = 0;
        while bar0.r32(pmu::DMATRFCMD) & 0x02 != 0 {
            thread::sleep(Duration::from_micros(1));
            timeout += 1;
            if timeout > 10000 {
                println!("  TIMEOUT at block {block}");
                break 'dmatrf;
            }
        }
        ok += 1;
    }
    dmatrf_ok = ok;
    let dmatrf_ms = t0.elapsed().as_millis();
    println!("  DMATRF: {ok}/{n_blocks} blocks OK in {dmatrf_ms}ms");

    /* Phase 3b: Check IMEM readback after DMATRF */
    println!("\n--- Phase 3b: IMEM readback after DMATRF ---");
    bar0.w32(pmu::BASE + falcon::IMEMC, 0x02000000);
    let imem_v0 = bar0.r32(pmu::BASE + falcon::IMEMD);
    let imem_exp = pramin_exp;
    println!(
        "  IMEM[0]: 0x{imem_v0:08x} (want=0x{imem_exp:08x}) {}",
        if imem_v0 == imem_exp {
            "DMATRF OK!"
        } else {
            "still HS ROM"
        }
    );

    /* Phase 4: Upload DMEM via PIO (works in HS mode) */
    println!("\n--- Phase 4: Upload DMEM via PIO ---");
    let dmem_upload = if dmem_size < 65536 { dmem_size } else { 65536 };
    bar0.w32(pmu::BASE + falcon::DMEMC, 0x01000000);
    for i in 0..(dmem_upload / 4) {
        let word = le_word(&dmem, i as usize);
        bar0.w32(pmu::BASE + falcon::DMEMD, word);
    }
    bar0.w32(pmu::BASE + falcon::DMEMC, 0x02000000);
    let dmem_v0 = bar0.r32(pmu::BASE + falcon::DMEMD);
    let dmem_exp = le_word(&dmem, 0);
    println!(
        "  DMEM[0]: got=0x{dmem_v0:08x} want=0x{dmem_exp:08x} {}",
        if dmem_v0 == dmem_exp { "OK" } else { "MISMATCH" }
    );

    /* Phase 5: Boot */
    println!("\n--- Phase 5: STARTCPU ---");
    bar0.w32(pmu::BASE + falcon::BOOTVEC, 0);
    bar0.w32(pmu::BASE + falcon::MAILBOX0, 0);
    bar0.w32(pmu::BASE + falcon::CPUCTL, 0x02);
    println!("  STARTCPU issued");

    /* Phase 6: Poll */
    println!("\n--- Phase 6: Poll (5s) ---");
    let t0 = Instant::now();
    let mut last_mb0 = 0xFFFF_FFFFu32;
    let mut last_pc = 0xFFFF_FFFFu32;

    while t0.elapsed() < Duration::from_secs(5) {
        let cur_mb0 = bar0.r32(pmu::BASE + falcon::MAILBOX0);
        let cur_pc = bar0.r32(pmu::BASE + falcon::PC);
        let ctrl = bar0.r32(pmu::BASE + falcon::CPUCTL);
        let exci = bar0.r32(pmu::BASE + falcon::EXCI);

        if cur_mb0 != last_mb0 || cur_pc != last_pc {
            let elapsed = t0.elapsed().as_millis();
            println!(
                "  t={elapsed}ms: mb0=0x{cur_mb0:08x} pc=0x{cur_pc:08x} ctrl=0x{ctrl:08x} exci=0x{exci:08x}"
            );
            last_mb0 = cur_mb0;
            last_pc = cur_pc;
        }

        if cur_mb0 == 0x300 {
            println!("  PMU BOOT: ACR ready!");
            booted = true;
            break;
        }
        if cur_mb0 != 0 && (cur_mb0 & 0xFF00_0000) == 0 {
            println!("  PMU SIGNAL: mb0=0x{cur_mb0:08x}");
            booted = true;
            break;
        }
        if exci == 0x04070000 {
            println!("  FAIL: HS trap");
            break;
        }
        if ctrl & 0x10 != 0 {
            println!("  STALLED: ctrl=0x{ctrl:08x}");
            break;
        }

        thread::sleep(Duration::from_millis(1));
    }

    /* Final state */
    println!("\n--- Final State ---");
    print_falcon(&bar0, "PMU", pmu::BASE);
    print_falcon(&bar0, "FECS", falcon::FECS_BASE);
    print_falcon(&bar0, "GPCCS", falcon::GPCCS_BASE);
    print_falcon(&bar0, "SEC2", SEC2_BASE_LEGACY);
    println!(
        "  PMU MB0: 0x{:08x}, MB1: 0x{:08x}, EXCI: 0x{:08x}",
        bar0.r32(pmu::BASE + falcon::MAILBOX0),
        bar0.r32(pmu::BASE + falcon::MAILBOX1),
        bar0.r32(pmu::BASE + falcon::EXCI)
    );
    println!("  GR_STATUS: 0x{:08x}", bar0.r32(pgraph::STATUS));
    println!(
        "  WPR2: lo=0x{:08x} hi=0x{:08x}",
        bar0.r32(WPR2_LO),
        bar0.r32(WPR2_HI)
    );

    for gpc_id in 0..6 {
        let tpc = bar0.r32(gpc::tpc_enable(gpc_id));
        if tpc != 0 {
            println!("  GPC{gpc_id} TPC_EN: 0x{tpc:08x}");
        }
    }

    println!(
        "\n=== {} ===",
        if booted {
            "SOVEREIGN PMU BOOT SUCCEEDED"
        } else {
            "INCOMPLETE"
        }
    );

    print_summary(booted, dry_run, dmatrf_ok, dmatrf_total);

    if booted {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_summary(booted: bool, dry_run: bool, dmatrf_ok: u32, dmatrf_total: u32) {
    let result = serde_json::json!({
        "tool": "sovereign_pmu_boot",
        "booted": booted,
        "dry_run": dry_run,
        "dmatrf_blocks_ok": dmatrf_ok,
        "dmatrf_blocks_total": dmatrf_total,
        "status": if booted { "SOVEREIGN PMU BOOT SUCCEEDED" } else { "INCOMPLETE" },
    });
    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
}
