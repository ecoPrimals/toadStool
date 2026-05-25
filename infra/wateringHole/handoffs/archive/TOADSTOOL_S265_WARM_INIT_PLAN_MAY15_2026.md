# Handoff: WarmInitPlan — Multi-Stage Driver Injection for Sovereign GPU Access

**From:** toadStool S265 (revised)
**Date:** May 15, 2026
**Scope:** glowplug warm_init.rs, sysfs_executor.rs — containment architecture

## Summary

`WarmInitPlan` in glowplug provides two containment strategies for GPU initialization:
- **Bare-metal**: host-safe drivers (nouveau, amdgpu) swap directly on the host kernel
- **Contained**: hazardous drivers (nvidia-470) run inside agentReagents VMs via VFIO passthrough

## Problem

Volta+ GPUs (GV100/Titan V) have FECS in Heavy Secure (HS) mode, blocking unsigned code execution. The only path to full FECS boot is nvidia-470, but it conflicts with the host's nvidia-580 (same `nvidia.ko` module name). Unloading nvidia-580 destroys the RTX 5060 display — a hard constraint.

## Architecture Decision

**Host DRM is sacred.** Conflicting drivers are hazardous material and must be contained:

```text
┌─────────────────────────────────────────────────────────┐
│  HOST KERNEL                                            │
│  nvidia-580 → RTX 5060 (DRM, never touched)             │
│  vfio-pci   → Titan V (passthrough to VM or warm swap)  │
│  nouveau    → safe bare-metal seeder (no conflict)       │
└─────────────────────────────────────────────────────────┘
         │                    │
         │                    ▼
         │  ┌──────────────────────────────────────┐
         │  │  agentReagents VM (containment)       │
         │  │  nvidia-470 → Titan V (VFIO guest)    │
         │  │  SEC2→ACR→FECS→COMPUTE                │
         │  └──────────────────────────────────────┘
         │
         ▼
  bare-metal warm swap
  nouveau → vfio-pci (FLR disabled)
  cylinder BAR0/BAR1/BAR3 access
```

## New Types (`warm_init.rs`)

- `SeederContainment` — enum: `BareMetal` or `Contained { reagent_template }`
- `WarmInitPlan` — BDF, seeder, containment strategy, settle time, final target
- `SeederDriver` — name, module, initializes list, limitations list
- `WarmInitResult` / `WarmInitStep` — per-step outcomes with timing

**Removed from previous version:** `ModuleSwap`, `ProtectedDevice`, `rmmod()`, `modprobe()`, `maybe_rollback()`. Host kernel module manipulation is no longer part of glowplug.

## Factory Methods

```rust
// Bare-metal: safe for host kernel
let plan = WarmInitPlan::nouveau_titanv("0000:02:00.0");
assert!(plan.is_bare_metal());

// Contained: HAZARDOUS, runs in agentReagents VM
let plan = WarmInitPlan::nvidia470_titanv("0000:02:00.0");
assert!(plan.requires_containment());
assert_eq!(plan.reagent_template(), Some("reagent-nvidia470-titanv"));

// Bare-metal: PLX bridge requires SwapGuard
let plan = WarmInitPlan::nouveau_k80("0000:4b:00.0");
assert!(plan.is_bare_metal());
```

## Execution

### Bare-metal: `SysfsSwapExecutor::execute_warm_init()`

5-step sequence: unbind → seeder_bind → seeder_settle → prepare_warm_swap → warm_swap

Panics if given a contained plan — contained plans MUST go through agentReagents.

### Contained: agentReagents VM lifecycle

1. glowplug swaps GPU to vfio-pci on host
2. agentReagents launches VM from reagent template (e.g., `reagent-nvidia470-titanv`)
3. GPU passed through to VM via VFIO
4. VM's nvidia-470 performs full init (SEC2→ACR→FECS)
5. Compute dispatched inside VM via IPC

## Hardware Validation (Titan V, May 15)

| Capability | nouveau bare-metal | nvidia-470 contained VM |
|-----------|-------------------|------------------------|
| BAR0 registers | 13/15 alive | Full (via VM) |
| BAR1 VRAM | 256MB R/W | Full |
| BAR3 RAMIN | 32MB R/W | Full |
| PRI Ring | Alive (0x5bfff5ff) | Full |
| FECS | HRESET (no PMU fw) | Running (ACR boot) |
| Host DRM impact | None | None |

## Integration Points

- **cylinder**: Receives warm GPU after bare-metal swap; VM access via IPC for contained plans
- **ember**: Manages VFIO fd after warm swap; VM lifecycle via benchScale
- **agentReagents**: `reagent-nvidia470-titanv.yaml` — validated full Volta compute (80 SMs, HBM2)
- **toadstool-server**: `SwapGuard` burst mode protects PLX bridges during bare-metal swaps

## Test Results

77 glowplug tests pass (11 warm_init tests). All existing swap, orchestrator, and sysfs_executor tests unaffected.

## Files Changed

- `crates/core/glowplug/src/warm_init.rs` — refactored: SeederContainment enum, removed ModuleSwap/ProtectedDevice
- `crates/core/glowplug/src/sysfs_executor.rs` — simplified: removed rmmod/modprobe/maybe_rollback, bare-metal only
- `crates/core/glowplug/src/lib.rs` — warm_init module + WarmInitPlan re-export
