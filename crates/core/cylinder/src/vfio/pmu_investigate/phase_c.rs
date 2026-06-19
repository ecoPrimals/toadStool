// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::nv::pmu_init::pmu_reg;
use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

use super::{gpc_alive, pmu_ext, power_reg, r, snapshot_power_state, w};

/// Result of Phase C: FBIF / DMEM access probe.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PhaseC {
    /// PIO DMEM read test: can we read DMEM despite HS lock?
    pub pio_dmem_readable: bool,
    /// PIO DMEM write test: can we write+readback DMEM?
    pub pio_dmem_writable: bool,
    /// First 64 words of DMEM (if readable), hex strings.
    pub dmem_dump: Vec<String>,
    /// Falcon DMA transfer register state.
    pub dmactl: u32,
    pub dmatrfbase: u32,
    pub dmatrfmoffs: u32,
    pub dmatrfcmd: u32,
    pub dmatrffboffs: u32,
    /// Queue head write test: did writing head 0 stick?
    pub queue_head_writable: bool,
    /// Queue head write detail.
    pub queue_head_detail: String,
    /// Doorbell/interrupt trigger test.
    pub doorbell_sent: bool,
    pub doorbell_detail: String,
    /// If DMEM writable: did we attempt queue-based PG command?
    pub queue_pg_attempted: bool,
    /// Queue PG command result detail.
    pub queue_pg_detail: String,
    /// GPC state after all Phase C attempts.
    pub gpc_after: u32,
    pub gpc_ungated: bool,
    pub notes: Vec<String>,
}

/// Falcon DMA transfer registers (base-relative offsets, add to PMU base 0x10A000).
mod falcon_dma {
    pub const DMATRFBASE: usize = 0x0010_A110;
    pub const DMATRFMOFFS: usize = 0x0010_A114;
    pub const DMATRFCMD: usize = 0x0010_A118;
    pub const DMATRFFBOFFS: usize = 0x0010_A11C;
}

/// Run Phase C: probe DMEM access and attempt queue-based PG command.
///
/// The hypothesis: HS-lock protects IMEM (code) but may leave DMEM (data)
/// accessible via PIO. If so, we can write directly to the PMU's CMD_QUEUE
/// in DMEM and trigger command processing via queue head + doorbell.
pub fn investigate_pmu_phase_c(bar0: &MappedBar) -> PhaseC {
    let mut notes = Vec::new();

    // ── Step 1: Test PIO DMEM read ──────────────────────────────────
    //
    // Set DMEMC port 0 to read mode at offset 0, then read DMEMD.
    // DMEMC format: bits [31:25] = mode, bits [15:2] = offset >> 2
    //   0x02000000 = port 0, auto-increment, read mode, offset 0
    w(bar0, pmu_reg::DMEMC, 0x0200_0000);
    let dmem_word0 = r(bar0, pmu_reg::DMEMD);

    let pio_dmem_readable = !is_pri_fault(dmem_word0) && dmem_word0 != 0xDEAD_DEAD;

    if pio_dmem_readable {
        notes.push(format!(
            "PIO DMEM READ WORKS! DMEM[0]={dmem_word0:#010x} — HS lock does NOT block DMEM reads"
        ));
    } else {
        notes.push(format!(
            "PIO DMEM read blocked: DMEM[0]={dmem_word0:#010x} (HS lock covers DMEM)"
        ));
    }

    tracing::info!(
        dmem_word0 = format_args!("{dmem_word0:#010x}"),
        pio_dmem_readable,
        "Phase C step 1: PIO DMEM read test"
    );

    // ── Step 2: Dump first 64 words of DMEM (if readable) ──────────
    let mut dmem_dump = Vec::new();
    if pio_dmem_readable {
        w(bar0, pmu_reg::DMEMC, 0x0200_0000); // reset to offset 0, auto-inc
        for i in 0..64 {
            let word = r(bar0, pmu_reg::DMEMD); // auto-increments
            dmem_dump.push(format!("{:03x}: {word:#010x}", i * 4));
            if is_pri_fault(word) {
                notes.push(format!("DMEM read faulted at offset {:#x}", i * 4));
                break;
            }
        }
        notes.push(format!(
            "DMEM dump: {} words read successfully",
            dmem_dump.len()
        ));
    }

    // ── Step 3: Test PIO DMEM write ─────────────────────────────────
    //
    // Write to a high DMEM offset (near end of 64KB region) to avoid
    // corrupting active firmware data. DMEM is 64KB on GV100 PMU.
    // Use offset 0xFF00 (near end).
    let test_offset: u32 = 0xFF00;
    let test_pattern: u32 = 0xCAFE_BABE;

    // Write: DMEMC with write mode (bit 26 set) + offset
    w(bar0, pmu_reg::DMEMC, 0x0600_0000 | (test_offset >> 2));
    w(bar0, pmu_reg::DMEMD, test_pattern);

    // Readback: switch to read mode at same offset
    w(bar0, pmu_reg::DMEMC, 0x0200_0000 | (test_offset >> 2));
    let readback = r(bar0, pmu_reg::DMEMD);

    let pio_dmem_writable = readback == test_pattern;

    if pio_dmem_writable {
        notes.push(format!(
            "PIO DMEM WRITE WORKS! Wrote {test_pattern:#010x} at offset {test_offset:#x}, \
             readback={readback:#010x} — DMEM is fully accessible"
        ));
        // Clean up our test write
        w(bar0, pmu_reg::DMEMC, 0x0600_0000 | (test_offset >> 2));
        w(bar0, pmu_reg::DMEMD, 0x0000_0000);
    } else {
        notes.push(format!(
            "PIO DMEM write blocked: wrote {test_pattern:#010x} at {test_offset:#x}, \
             readback={readback:#010x}"
        ));
    }

    tracing::info!(
        test_pattern = format_args!("{test_pattern:#010x}"),
        readback = format_args!("{readback:#010x}"),
        pio_dmem_writable,
        "Phase C step 3: PIO DMEM write test"
    );

    // ── Step 4: Read falcon DMA transfer registers ──────────────────
    let dmactl = r(bar0, pmu_reg::DMACTL);
    let dmatrfbase = r(bar0, falcon_dma::DMATRFBASE);
    let dmatrfmoffs = r(bar0, falcon_dma::DMATRFMOFFS);
    let dmatrfcmd = r(bar0, falcon_dma::DMATRFCMD);
    let dmatrffboffs = r(bar0, falcon_dma::DMATRFFBOFFS);

    notes.push(format!(
        "Falcon DMA: CTL={dmactl:#010x} BASE={dmatrfbase:#010x} \
         MOFFS={dmatrfmoffs:#010x} CMD={dmatrfcmd:#010x} FBOFFS={dmatrffboffs:#010x}"
    ));

    // ── Step 5: Queue head write test ───────────────────────────────
    //
    // Queue head 0 was 0x0 in Phase A. Try writing a value and reading
    // back to see if we have write access.
    let head0_before = r(bar0, pmu_ext::QUEUE_HEAD_0);
    w(bar0, pmu_ext::QUEUE_HEAD_0, 0x0000_0001);
    let head0_after_write = r(bar0, pmu_ext::QUEUE_HEAD_0);

    // Restore original value
    w(bar0, pmu_ext::QUEUE_HEAD_0, head0_before);

    let queue_head_writable = head0_after_write == 0x0000_0001;
    let queue_head_detail = format!(
        "HEAD0: before={head0_before:#010x}, wrote=0x1, readback={head0_after_write:#010x}"
    );

    if queue_head_writable {
        notes.push("Queue HEAD registers are writable from host".into());
    } else {
        notes.push(format!("Queue HEAD write test: {queue_head_detail}"));
    }

    tracing::info!(
        head0_before = format_args!("{head0_before:#010x}"),
        head0_after_write = format_args!("{head0_after_write:#010x}"),
        queue_head_writable,
        "Phase C step 5: queue head write test"
    );

    // ── Step 6: Doorbell / interrupt trigger ─────────────────────────
    //
    // Try triggering a PMU interrupt via IRQSSET bit 4 (ext interrupt
    // typically used for host→falcon doorbell in nouveau).
    let irqstat_before = r(bar0, pmu_ext::IRQSTAT);
    w(bar0, pmu_reg::IRQSSET, 1 << 4); // set ext interrupt bit
    std::thread::sleep(Duration::from_millis(5));
    let irqstat_after = r(bar0, pmu_ext::IRQSTAT);
    let mbox0_after_doorbell = r(bar0, pmu_reg::MAILBOX0);

    let doorbell_sent = true; // we always attempt
    let doorbell_detail = format!(
        "IRQSTAT: {irqstat_before:#010x}→{irqstat_after:#010x}, \
         MBOX0 after doorbell={mbox0_after_doorbell:#010x}"
    );
    notes.push(format!("Doorbell: {doorbell_detail}"));

    tracing::info!(
        irqstat_before = format_args!("{irqstat_before:#010x}"),
        irqstat_after = format_args!("{irqstat_after:#010x}"),
        mbox0 = format_args!("{mbox0_after_doorbell:#010x}"),
        "Phase C step 6: doorbell test"
    );

    // ── Step 7: Queue-based PG command (if DMEM writable) ───────────
    //
    // If we can write to DMEM AND write queue heads, we can construct
    // a proper queue-based command message in DMEM and advance the
    // queue head to trigger PMU processing.
    //
    // The nouveau PMU message header (nvkm/subdev/pmu/priv.h):
    //   struct nvfw_falcon_msg {
    //       u8 unit;    // PMU_UNIT_PG = 0x03 (from nouveau)
    //       u8 size;    // total message size in bytes
    //       u8 ctrl;    // queue ID (0 = CMD_QUEUE)
    //       u8 seq_id;  // sequence number
    //   };
    //
    // Followed by the command-specific payload. For PG:
    //   u8 cmd_type;  // PMG_PG_CMD_ID_ELPG = 0x01, or PG_ALLOW = 0x08
    //   ... engine-specific data ...
    //
    // We place the message at DMEM offset 0x0 (CMD_QUEUE start on
    // many nouveau PMU firmware versions), then advance HEAD to
    // sizeof(message) and trigger the doorbell.
    let mut queue_pg_attempted = false;
    let queue_pg_detail;

    if pio_dmem_writable && queue_head_writable {
        queue_pg_attempted = true;
        notes.push("DMEM + queue heads both writable — attempting queue-based PG command".into());

        let (gpc_pre, _, _) = snapshot_power_state(bar0);

        // Scan DMEM for queue position markers: nouveau typically stores
        // queue base offsets in the first 256 bytes of DMEM as part of
        // the PMU init message response.
        let mut dmem_scan: Vec<u32> = Vec::new();
        w(bar0, pmu_reg::DMEMC, 0x0200_0000); // read, offset 0
        for _ in 0..64 {
            dmem_scan.push(r(bar0, pmu_reg::DMEMD));
        }

        // Look for potential queue base pointers in the init data.
        // Queue offsets are typically aligned to 0x100 and within DMEM size.
        let dmem_size = 64 * 1024u32; // 64KB for GV100 PMU
        let mut candidate_offsets: Vec<(usize, u32)> = Vec::new();
        for (i, &word) in dmem_scan.iter().enumerate() {
            if word > 0x100 && word < dmem_size && word & 0xFF == 0 && !is_pri_fault(word) {
                candidate_offsets.push((i * 4, word));
            }
        }

        if !candidate_offsets.is_empty() {
            notes.push(format!(
                "Found {} potential queue offset markers in DMEM init data: {:?}",
                candidate_offsets.len(),
                candidate_offsets.iter().take(8).collect::<Vec<_>>()
            ));
        }

        // Construct a PG_CMD_ALLOW message at a safe DMEM offset.
        // Use offset 0x4000 (well above init data, within 64KB).
        let msg_offset: u32 = 0x4000;

        // Message header (4 bytes):
        //   unit=0x03 (PG), size=8, ctrl=0 (CMD queue), seq=0x01
        // unit=PG(0x03), size=8, ctrl=CMD_QUEUE(0), seq=1
        let hdr: u32 = 0x03 | (8 << 8) | (0x01 << 24);

        // Payload (4 bytes):
        //   cmd_type=0x08 (PG_CMD_ALLOW), engine=0xFF (all engines)
        let payload: u32 = 0x08 | (0xFF << 8);

        // Write message to DMEM
        w(bar0, pmu_reg::DMEMC, 0x0600_0000 | (msg_offset >> 2));
        w(bar0, pmu_reg::DMEMD, hdr);
        w(bar0, pmu_reg::DMEMD, payload); // auto-increment writes next word

        // Verify write
        w(bar0, pmu_reg::DMEMC, 0x0200_0000 | (msg_offset >> 2));
        let hdr_readback = r(bar0, pmu_reg::DMEMD);
        let payload_readback = r(bar0, pmu_reg::DMEMD);

        let write_verified = hdr_readback == hdr && payload_readback == payload;

        if write_verified {
            notes.push(format!(
                "PG command written to DMEM[{msg_offset:#x}]: hdr={hdr:#010x} payload={payload:#010x}"
            ));

            // Set queue head 0 to point past our message (offset + msg_size)
            let new_head = msg_offset + 8;
            w(bar0, pmu_ext::QUEUE_HEAD_0, new_head);

            // Trigger doorbell interrupt
            w(bar0, pmu_reg::IRQSSET, 1 << 4);

            // Wait for PMU to process
            std::thread::sleep(Duration::from_millis(100));

            let head_after = r(bar0, pmu_ext::QUEUE_HEAD_0);
            let mbox0_after = r(bar0, pmu_reg::MAILBOX0);
            let (gpc_post, _, _) = snapshot_power_state(bar0);

            let pmu_consumed = head_after != new_head;

            queue_pg_detail = format!(
                "Msg at DMEM[{msg_offset:#x}], HEAD0: set={new_head:#x}→read={head_after:#010x}, \
                 PMU consumed={pmu_consumed}, MBOX0={mbox0_after:#010x}, \
                 GPC: {gpc_pre:#010x}→{gpc_post:#010x}"
            );

            if gpc_alive(gpc_post) {
                notes.push("GPC UNGATED via queue-based PG command!".into());
            } else if pmu_consumed {
                notes.push(
                    "PMU consumed queue message but GPCs still gated — \
                     command may need different format or queue base offset"
                        .into(),
                );
            } else {
                notes.push(
                    "PMU did not consume queue message — queue base offset \
                     likely incorrect, or CMD queue is not at HEAD register 0"
                        .into(),
                );
            }

            // Restore head 0
            w(bar0, pmu_ext::QUEUE_HEAD_0, 0);
        } else {
            queue_pg_detail = format!(
                "DMEM write verification failed: hdr={hdr_readback:#010x} (expected {hdr:#010x}), \
                 payload={payload_readback:#010x} (expected {payload:#010x})"
            );
            notes.push(format!("Queue PG: {queue_pg_detail}"));
        }

        // Clean up DMEM
        w(bar0, pmu_reg::DMEMC, 0x0600_0000 | (msg_offset >> 2));
        w(bar0, pmu_reg::DMEMD, 0);
        w(bar0, pmu_reg::DMEMD, 0);
    } else {
        queue_pg_detail = format!(
            "Skipped: dmem_writable={pio_dmem_writable}, head_writable={queue_head_writable}"
        );
        if !pio_dmem_writable && !queue_head_writable {
            notes.push(
                "Cannot attempt queue PG — both DMEM and queue heads are inaccessible".into(),
            );
        } else if !pio_dmem_writable {
            notes.push(
                "Cannot attempt queue PG — DMEM write blocked by HS lock. \
                 The livepatch/kernel-patch path is the most viable alternative."
                    .into(),
            );
        }
    }

    let gpc_final = r(bar0, power_reg::GPC_ENABLES);

    tracing::info!(
        pio_dmem_readable,
        pio_dmem_writable,
        queue_head_writable,
        queue_pg_attempted,
        gpc = format_args!("{gpc_final:#010x}"),
        gpc_ungated = gpc_alive(gpc_final),
        "Phase C complete"
    );

    PhaseC {
        pio_dmem_readable,
        pio_dmem_writable,
        dmem_dump,
        dmactl,
        dmatrfbase,
        dmatrfmoffs,
        dmatrfcmd,
        dmatrffboffs,
        queue_head_writable,
        queue_head_detail,
        doorbell_sent,
        doorbell_detail,
        queue_pg_attempted,
        queue_pg_detail,
        gpc_after: gpc_final,
        gpc_ungated: gpc_alive(gpc_final),
        notes,
    }
}
