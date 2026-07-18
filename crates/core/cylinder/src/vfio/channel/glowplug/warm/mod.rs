// SPDX-License-Identifier: AGPL-3.0-or-later
//! Full warm-up sequence — bring the GPU from any state to Warm.

pub(crate) mod warm_steps;

use super::super::diagnostic::interpreter::memory_probe;
use super::GlowPlug;
use super::types::{GpuThermalState, WarmResult};

use warm_steps::{
    run_step_bar2, run_step_clock_gating, run_step_d3hot_to_d0, run_step_digital_pmu,
    run_step_pfifo_reset, run_step_pmc_enable, run_step_pri_health, run_step_vram_strategies,
};

impl GlowPlug<'_> {
    /// Full warm-up sequence — bring the GPU from any state to Warm.
    pub fn warm(&self) -> WarmResult {
        let mut log = Vec::new();
        let mut step_snapshots = Vec::new();
        let initial_state = self.check_state();
        log.push(format!("initial state: {initial_state:?}"));

        if initial_state == GpuThermalState::Warm {
            return WarmResult {
                initial_state,
                final_state: GpuThermalState::Warm,
                success: true,
                memory: None,
                log,
                step_snapshots,
            };
        }

        // Step 0: D3hot → D0
        if initial_state == GpuThermalState::D3Hot {
            run_step_d3hot_to_d0(self, &mut log, &mut step_snapshots);
            if self.check_state() == GpuThermalState::D3Hot {
                return WarmResult {
                    initial_state,
                    final_state: GpuThermalState::D3Hot,
                    success: false,
                    memory: None,
                    log,
                    step_snapshots,
                };
            }
        }

        // Step 1: PMC_ENABLE — clock all engines
        if matches!(
            initial_state,
            GpuThermalState::ColdGated | GpuThermalState::EnginesClocked
        ) {
            run_step_pmc_enable(self, &mut log, &mut step_snapshots);
        }

        // Step 2: PFIFO reset cycle (bit 8)
        let state_after_pmc = self.check_state();
        if matches!(
            state_after_pmc,
            GpuThermalState::ColdGated | GpuThermalState::EnginesClocked
        ) {
            run_step_pfifo_reset(self, &mut log, &mut step_snapshots);
        }

        // Step 2.5: PRI bus health check
        run_step_pri_health(self, &mut log, &mut step_snapshots);

        // Step 2.75: Clock gating sweep
        run_step_clock_gating(self, &mut log, &mut step_snapshots);

        // Step 2.9: Digital PMU emulation
        run_step_digital_pmu(self, &mut log, &mut step_snapshots);

        // Step 3: VRAM strategies (if dead)
        let state_after_pfifo = self.check_state();
        if state_after_pfifo == GpuThermalState::PfifoAliveVramDead {
            run_step_vram_strategies(self, &mut log, &mut step_snapshots);
        }

        // Step 4: BAR2 page tables (requires VRAM)
        let state_after_fb = self.check_state();
        if matches!(
            state_after_fb,
            GpuThermalState::VramAliveBar2Dead | GpuThermalState::Warm
        ) && state_after_fb == GpuThermalState::VramAliveBar2Dead
        {
            run_step_bar2(self, &mut log);
        }

        // Step 5: Verify final state with full memory topology
        let final_state = self.check_state();
        let memory = Some(memory_probe::discover_memory_topology(
            self.bar0,
            self.container.clone(),
        ));

        let success = final_state == GpuThermalState::Warm;
        log.push(format!("final state: {final_state:?} success={success}"));

        WarmResult {
            initial_state,
            final_state,
            success,
            memory,
            log,
            step_snapshots,
        }
    }
}
