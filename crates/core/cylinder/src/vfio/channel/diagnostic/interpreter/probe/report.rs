// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(missing_docs, reason = "probe report types; full docs planned")]
//! Probe report — result container and analysis.

use std::fmt::Write as FmtWrite;

use crate::vfio::memory::MemoryTopology;

use super::super::layers::*;

/// Full probe report — contains either a successful layer output or
/// the failure point with all evidence collected up to that layer.
#[derive(Debug)]
pub struct ProbeReport {
    pub bar: Option<BarTopology>,
    pub identity: Option<GpuIdentity>,
    pub power: Option<PowerState>,
    pub engines: Option<EngineTopology>,
    pub memory: Option<MemoryTopology>,
    pub dma: Option<DmaCapability>,
    pub channel: Option<ChannelConfig>,
    pub dispatch: Option<DispatchCapability>,
    pub failures: Vec<ProbeFailure>,
    pub elapsed_ms: u128,
}

impl ProbeReport {
    /// Highest layer reached successfully.
    pub fn depth(&self) -> u8 {
        if self.dispatch.is_some() {
            8
        } else if self.channel.is_some() {
            7
        } else if self.dma.is_some() {
            6
        } else if self.memory.is_some() {
            5
        } else if self.engines.is_some() {
            4
        } else if self.power.is_some() {
            3
        } else if self.identity.is_some() {
            2
        } else if self.bar.is_some() {
            1
        } else {
            0
        }
    }

    /// Emit a human-readable probe report via `tracing`.
    pub fn print_summary(&self) {
        let mut s = String::new();
        let _ = writeln!(
            &mut s,
            "╔══ INTERPRETER PROBE REPORT ════════════════════════════════╗"
        );
        let _ = writeln!(
            &mut s,
            "║ Depth: Layer {}/8 in {}ms",
            self.depth(),
            self.elapsed_ms
        );

        if let Some(bar) = &self.bar {
            let _ = writeln!(
                &mut s,
                "║ L0 BAR:     read={} write={} d3hot={} BOOT0={:#010x}",
                bar.bar0_readable, bar.bar0_writable, bar.in_d3hot, bar.boot0_raw
            );
        }
        if let Some(id) = &self.identity {
            let _ = writeln!(
                &mut s,
                "║ L1 ID:      {} impl={} rev={} BOOT0={:#010x}",
                id.architecture, id.implementation, id.revision, id.boot0
            );
        }
        if let Some(pwr) = &self.power {
            let _ = writeln!(
                &mut s,
                "║ L2 POWER:   {:?} pfifo={} ptimer={} PMC={:#010x}",
                pwr.method, pwr.pfifo_enabled, pwr.ptimer_ticking, pwr.pmc_enable_final
            );
        }
        if let Some(eng) = &self.engines {
            let _ = writeln!(
                &mut s,
                "║ L3 ENGINES: pbdma_map={:#010x} gr_rl={:?} gr_pbdma={:?}",
                eng.pbdma_map, eng.gr_runlist, eng.gr_pbdma
            );
            let _ = writeln!(
                &mut s,
                "║             BAR1={:#010x} BAR2={:#010x} bar2_setup={}",
                eng.bar1_block, eng.bar2_block, eng.bar2_setup_needed
            );
        }
        if let Some(mem) = &self.memory {
            let pramin_ok = mem.pramin_works();
            let dma_ok = mem.dma_works();
            let working = mem.paths.iter().filter(|p| p.status.is_working()).count();
            let total = mem.paths.len();
            let _ = writeln!(
                &mut s,
                "║ L3.5 MEM:   vram={} pramin={pramin_ok} dma={dma_ok} bar2={} paths={working}/{total}",
                mem.vram_accessible, mem.bar2_configured
            );
        }
        if let Some(dma) = &self.dma {
            let _ = writeln!(
                &mut s,
                "║ L4 DMA:     read={} write={} iommu={} pt={} inst={}",
                dma.gpu_can_read_sysmem,
                dma.gpu_can_write_sysmem,
                dma.iommu_mapping_ok,
                dma.page_tables_ok,
                dma.instance_block_accessible
            );
            if !dma.ctx_evidence.is_empty() {
                let _ = write!(&mut s, "║             CTX:");
                for (name, val) in &dma.ctx_evidence {
                    let _ = write!(&mut s, " {name}={val:#010x}");
                }
                let _ = writeln!(&mut s);
            }
        }
        if let Some(ch) = &self.channel {
            let _ = writeln!(
                &mut s,
                "║ L5 CHANNEL: inst_tgt={} userd_tgt={} vram_inst={} method={:?}",
                ch.working_inst_target,
                ch.working_userd_target,
                ch.instance_requires_vram,
                ch.scheduling_method
            );
        }
        if let Some(disp) = &self.dispatch {
            let _ = writeln!(
                &mut s,
                "║ L6 DISPATCH: consumed={} nop={} ready={}",
                disp.gpfifo_consumed, disp.nop_executed, disp.dispatch_ready
            );
            for b in &disp.blockers {
                let _ = writeln!(&mut s, "║   BLOCKER: {b}");
            }
        }

        if !self.failures.is_empty() {
            let _ = writeln!(
                &mut s,
                "╠══ FAILURES ═══════════════════════════════════════════════╣"
            );
            for f in &self.failures {
                let _ = writeln!(&mut s, "║ {f}");
            }
        }
        let _ = writeln!(
            &mut s,
            "╚═══════════════════════════════════════════════════════════╝"
        );
        tracing::info!(summary = %s, "interpreter probe report");
    }
}
