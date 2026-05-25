# Handoff: Driver Lab — Multi-Seeder Comparison for Sovereign GPU Init

**From:** toadStool S265r + hotSpring Exp 195
**Date:** May 15, 2026
**Scope:** glowplug warm_init.rs (DriverLabPlan), cylinder bar_cartography.rs, hardware validation

## Summary

Added **Driver Laboratory** capability to glowplug — cycle different drivers through the same GPU, capture BAR0 register snapshots after each, diff to understand exactly what each driver initializes. First trial executed on Titan V: cold/vfio vs nouveau-warm.

## New Types

- `DriverTrial` — one seeder to test (label, WarmInitPlan, scan ranges, power cycle flag)
- `DriverLabPlan` — ordered sequence of trials for the same GPU, with output directory
- `NV_BAR0_DOMAINS` — 25 NVIDIA BAR0 domain ranges for cartography labeling

Factory: `DriverLabPlan::standard_titanv(bdf, output_dir)` — 3 trials:
1. cold-vfio (baseline)
2. nouveau-warm (bare-metal)
3. nvidia470-vm (contained in agentReagents)

## Hardware Results — Trial 1→2 (cold vs nouveau)

| Domain | Woke Up | Changed | Same | Insight |
|--------|---------|---------|------|---------|
| PGRAPH_GPC | 0 | 92 | 4658 | GPC cluster config (TPC routing, SM setup) |
| SEC2 | 0 | 0 | 192 | Completely untouched — root FECS blocker |
| FECS | 0 | 2 | 603 | HS-locked (SCTL=0x20204080), PC=0, no firmware |
| PRI_RING | 0 | 2 | 2303 | Stable (status counters only) |
| PMU | 0 | 7 | 765 | Halted, no firmware (nouveau lacks GV100 PMU fw) |

### Security Boundary Mapped

```
FECS_SCTL = 0x20204080  →  HS mode, production fuse, debug disabled
FECS_SSTAT = 0x00000001  →  ACR lockdown active
SEC2_SCTL = 0x00000000   →  Not initialized (root blocker)
```

PGRAPH hub splits into:
- FECS-independent (alive): GR_STATUS, GR_INTR, GR_ACTIVITY
- FECS-gated (dead at 0xbadf5040): GR_FECS_CTXSW, GR_PRI_STATUS

## Integration

- Leverages existing `bar_cartography::scan_bar0`, `diff_bar_maps`, `BarMapDiff`
- `SysfsSwapExecutor::execute_warm_init()` handles bare-metal trials
- Contained trials (nvidia-470) dispatch through agentReagents VM lifecycle
- All snapshots persistable as JSON via `BarMap::to_json_value()`

## Test Results

86 glowplug tests pass (9 new driver lab tests). All existing tests unaffected.

## Files Changed

- `crates/core/glowplug/src/warm_init.rs` — added DriverTrial, DriverLabPlan, NV_BAR0_DOMAINS
- `crates/core/glowplug/src/lib.rs` — re-exports DriverLabPlan, DriverTrial
- `hotSpring/experiments/195_DRIVER_LAB_MESA_VS_VENDOR.md` — experiment writeup
