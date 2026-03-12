// SPDX-License-Identifier: AGPL-3.0-only
//! Classify trace events by register function.
//!
//! Uses known register offset ranges from envytools (NVIDIA),
//! amd-gfx headers (AMD), and intel-gfx specs (Intel) to tag
//! each register write with its functional purpose.

use crate::distiller::RegFunction;
use crate::observer::{TraceEvent, TraceEventKind};

/// A classified trace event with its inferred function.
#[derive(Debug, Clone)]
pub struct ClassifiedEvent {
    pub event: TraceEvent,
    pub function: RegFunction,
}

/// Classify a stream of trace events.
pub fn classify_events(events: &[TraceEvent]) -> Vec<ClassifiedEvent> {
    events
        .iter()
        .map(|e| {
            let function = classify_single(e);
            ClassifiedEvent {
                event: e.clone(),
                function,
            }
        })
        .collect()
}

fn classify_single(event: &TraceEvent) -> RegFunction {
    match &event.kind {
        TraceEventKind::RegisterWrite { offset, .. } => classify_register(*offset),
        TraceEventKind::FirmwareLoad { .. } => RegFunction::PowerGate,
        TraceEventKind::IoctlCall { ioctl_nr, .. } => classify_ioctl(*ioctl_nr),
        _ => RegFunction::Unknown,
    }
}

/// Classify a register write by offset range.
///
/// These ranges are approximate — refined by the knowledge store's
/// envytools/amd-gfx integration as recipes accumulate.
fn classify_register(offset: u64) -> RegFunction {
    // NVIDIA (from envytools rnndb): approximate ranges for GV100+
    // 0x00020000..0x00020FFF — PMC (Power Management Controller)
    // 0x00060000..0x00060FFF — PTIMER (clock)
    // 0x00100000..0x00100FFF — PFB (framebuffer/memory controller)
    // 0x00400000..0x004FFFFF — PGRAPH (graphics/compute engine)
    // 0x00800000..0x008FFFFF — CE (copy engine)
    //
    // AMD (from amdgpu headers): approximate ranges for GFX10+
    // 0x00002000..0x00002FFF — GC (graphics compute)
    // 0x0000D000..0x0000DFFF — SMN/NBIO (power)
    // 0x00015000..0x00015FFF — SDMA
    //
    // Intel (from i915 headers): approximate ranges
    // 0x00002000..0x00002FFF — RENDER ring
    // 0x00012000..0x00012FFF — Compute engine

    match offset {
        // NVIDIA PMC / power management
        0x00020000..=0x00020FFF => RegFunction::PowerGate,
        // NVIDIA PTIMER / clock
        0x00060000..=0x00060FFF => RegFunction::ClockEnable,
        // NVIDIA PFB / memory
        0x00100000..=0x00100FFF => RegFunction::MemoryConfig,
        // NVIDIA PGRAPH compute engine
        0x00400000..=0x004FFFFF => RegFunction::EngineReset,
        // NVIDIA interrupts
        0x00000100..=0x000001FF => RegFunction::InterruptEnable,
        // AMD GC (graphics compute) register range
        0x00002000..=0x00002FFF => RegFunction::EngineReset,
        // AMD power/SMN
        0x0000D000..=0x0000DFFF => RegFunction::PowerGate,
        // AMD thermal
        0x00016000..=0x00016FFF => RegFunction::ThermalConfig,
        _ => RegFunction::Unknown,
    }
}

fn classify_ioctl(ioctl_nr: u64) -> RegFunction {
    // nouveau new UAPI ioctl numbers (from drm/nouveau_drm.h)
    let cmd = (ioctl_nr & 0xFF) as u8;
    match cmd {
        0x00 => RegFunction::ContextAlloc, // VM_INIT
        0x01 => RegFunction::MemoryConfig, // VM_BIND
        0x02 => RegFunction::ChannelBind,  // CHANNEL_ALLOC (EXEC)
        _ => RegFunction::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_nvidia_power() {
        assert_eq!(classify_register(0x00020010), RegFunction::PowerGate);
    }

    #[test]
    fn classify_nvidia_clock() {
        assert_eq!(classify_register(0x00060000), RegFunction::ClockEnable);
    }

    #[test]
    fn classify_nvidia_pgraph() {
        assert_eq!(classify_register(0x00400000), RegFunction::EngineReset);
    }

    #[test]
    fn classify_unknown_offset() {
        assert_eq!(classify_register(0xFFFFFFFF), RegFunction::Unknown);
    }

    #[test]
    fn classify_amd_gc() {
        assert_eq!(classify_register(0x00002100), RegFunction::EngineReset);
    }
}
