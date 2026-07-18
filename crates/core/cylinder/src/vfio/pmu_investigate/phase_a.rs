// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::nv::pmu_init::{PmuSnapshot, pmu_reg};
use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

use super::{pmu_ext, r};

/// Result of Phase A: PMU liveness probe.
#[derive(Debug, Clone)]
pub struct PhaseA {
    pub snapshot: PmuSnapshot,
    pub pc_0: u32,
    pub pc_1: u32,
    pub pc_advancing: bool,
    pub hs_locked: bool,
    pub irqstat: u32,
    pub irqmask: u32,
    pub os_reg: u32,
    pub fbif_ctl: u32,
    pub fbif_transcfg: u32,
    pub queue_heads: Vec<u32>,
    pub queue_tails: Vec<u32>,
    pub notes: Vec<String>,
}

/// Probe PMU falcon state after nouveau unbind.
pub fn run_phase_a(bar0: &MappedBar) -> PhaseA {
    let mut notes: Vec<String> = Vec::new();

    let snapshot = PmuSnapshot::capture(bar0);
    let falcon_state = snapshot.falcon_state();
    notes.push(format!("PMU falcon state: {falcon_state:?}"));
    notes.push(snapshot.summary());

    // Read PC twice to check if it's advancing
    let pc_0 = r(bar0, pmu_reg::PC);
    std::thread::sleep(Duration::from_millis(5));
    let pc_1 = r(bar0, pmu_reg::PC);
    let pc_advancing = pc_0 != pc_1 && !is_pri_fault(pc_0) && !is_pri_fault(pc_1);

    if pc_advancing {
        notes.push(format!(
            "PMU PC advancing: {pc_0:#x} → {pc_1:#x} (delta {})",
            pc_1.wrapping_sub(pc_0)
        ));
    } else if pc_0 == pc_1 {
        notes.push(format!(
            "PMU PC static: {pc_0:#x} (may be in tight loop or halted)"
        ));
    }

    let hs_locked = snapshot.sctl & 0x02 != 0;
    if hs_locked {
        notes.push("PMU HS-locked (SCTL bit 1 set) — secure firmware running".into());
    }

    // Extended PMU registers
    let irqstat = r(bar0, pmu_ext::IRQSTAT);
    let irqmask = r(bar0, pmu_ext::IRQMASK);
    let os_reg = r(bar0, pmu_ext::OS);
    let fbif_ctl = r(bar0, pmu_ext::FBIF_CTL);
    let fbif_transcfg = r(bar0, pmu_ext::FBIF_TRANSCFG);

    let queue_heads = vec![
        r(bar0, pmu_ext::QUEUE_HEAD_0),
        r(bar0, pmu_ext::QUEUE_HEAD_1),
        r(bar0, pmu_ext::QUEUE_HEAD_2),
        r(bar0, pmu_ext::QUEUE_HEAD_3),
    ];
    let queue_tails = vec![
        r(bar0, pmu_ext::QUEUE_TAIL_0),
        r(bar0, pmu_ext::QUEUE_TAIL_1),
        r(bar0, pmu_ext::QUEUE_TAIL_2),
        r(bar0, pmu_ext::QUEUE_TAIL_3),
    ];

    // Check if queues have pending messages
    for (i, (h, t)) in queue_heads.iter().zip(queue_tails.iter()).enumerate() {
        if h != t && !is_pri_fault(*h) && !is_pri_fault(*t) {
            notes.push(format!(
                "PMU queue {i} has pending data: head={h:#x} tail={t:#x}"
            ));
        }
    }

    notes.push(format!(
        "PMU IRQSTAT={irqstat:#010x} IRQMASK={irqmask:#010x} OS={os_reg:#010x}"
    ));
    notes.push(format!(
        "PMU FBIF_CTL={fbif_ctl:#010x} FBIF_TRANSCFG={fbif_transcfg:#010x}"
    ));

    tracing::info!(
        cpuctl = format_args!("{:#010x}", snapshot.cpuctl),
        pc_0 = format_args!("{:#010x}", pc_0),
        pc_1 = format_args!("{:#010x}", pc_1),
        pc_advancing,
        hs_locked,
        running = snapshot.is_running,
        halted = snapshot.cpuctl & (1 << 4) != 0,
        sctl = format_args!("{:#010x}", snapshot.sctl),
        mbox0 = format_args!("{:#010x}", snapshot.mailbox0),
        mbox1 = format_args!("{:#010x}", snapshot.mailbox1),
        pfifo = snapshot.pfifo_enabled,
        "Phase A: PMU liveness probe"
    );

    PhaseA {
        snapshot,
        pc_0,
        pc_1,
        pc_advancing,
        hs_locked,
        irqstat,
        irqmask,
        os_reg,
        fbif_ctl,
        fbif_transcfg,
        queue_heads,
        queue_tails,
        notes,
    }
}
