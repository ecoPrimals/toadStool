# PCIe Bridge Keepalive — S264 (May 15, 2026)

## Problem

PCIe switch chips (PLX PEX 8747, AMD Matisse, Broadcom PEX, etc.) can enter
D3cold when all downstream endpoint drivers are unbound. D3cold destroys the
switch's internal configuration (PLL, lane config, fabric routing), rendering
all downstream devices inaccessible. The link drops to Gen1 2.5GT/s training
loop with `DLActive-` (Data Link Layer never activates). Recovery requires a
full chassis power-on reset — no software reset is sufficient.

This was diagnosed on the Tesla K80 (GK210×2 behind PLX PEX 8747) during the
Hardware Validation Sprint (Exp 192–193). The K80's 3-level bridge hierarchy
(AMD GPP → PLX upstream → PLX downstream × 2 → K80 die × 2) made it
particularly vulnerable.

## Solution: Hierarchy Pinning + Burst Keepalive

### 1. `pin_bridge_hierarchy()` (ember/src/sysfs.rs)

Walks the **entire** PCI ancestry from device to root complex via canonical
sysfs path, setting `d3cold_allowed=0` and `power/control=on` on every
ancestor bridge. Returns the count of bridges pinned.

This replaces the previous `pin_bridge_power()` which only walked one parent —
insufficient for multi-level switch topologies.

### 2. Vendor lifecycle evolution (ember/src/vendor_lifecycle/)

All 9 vendor lifecycles (`NvidiaKepler`, `NvidiaLifecycle` (Volta+),
`NvidiaOpen`, `NvidiaOracle`, `AmdVega20`, `AmdRdna`, `IntelXe`, `Generic`,
`BrainChip`) now call `pin_bridge_hierarchy()` in `prepare_for_unbind()` and
`stabilize_after_bind()`. This is the primary defense.

### 3. `SysfsSwapExecutor` pre-unbind (glowplug/src/sysfs_executor.rs)

The production swap executor calls `pin_bridge_hierarchy()` before any unbind,
providing redundant safety independent of the vendor lifecycle.

### 4. `SwapGuard` burst mode (server/src/background/pcie_keepalive.rs)

RAII guard that switches PCIe keepalive from normal 3-second CfgRd cadence to
aggressive 10ms burst during driver swaps. Activated automatically by
`GlowPlugClient::swap()`.

### 5. Server startup auto-pin (server/src/background/pcie_keepalive.rs)

On daemon startup, the keepalive background task discovers all PCI-to-PCI
bridges with GPU endpoints downstream and pins their entire hierarchies
before any swap is ever attempted.

## Hardware Validation

Tested on biomegate compute trio after full chassis power cycle:

| Test | Result |
|------|--------|
| PLX post-POR state | Gen3 8GT/s x8, DLActive+, all config reads valid |
| Pin hierarchy → unbind vfio-pci | PLX survived: Gen3 8GT/s, DLActive+ |
| Rebind vfio-pci | Both K80 dies healthy: config 0x10de 0x102d, D0 |
| Full round trip | vfio-pci → unbound → vfio-pci: clean |

## Pattern for Other Hardware

This pattern applies to any PCIe device behind a switch or bridge:

1. **Multi-GPU workstations** with PLX/Broadcom switches
2. **AMD Threadripper/EPYC** with Matisse internal PCIe switches
3. **Intel Ice Lake/Sapphire Rapids** with CXL/PCIe switches
4. **Any add-in card** behind a PCIe riser or bridge chip

The general rule: **before unbinding any driver from a PCIe endpoint, pin
`d3cold_allowed=0` on every ancestor bridge from the device to the root
complex.**

## Files Changed

| File | Change |
|------|--------|
| `crates/core/ember/src/sysfs.rs` | `pin_bridge_hierarchy()` — full ancestry walk |
| `crates/core/ember/src/vendor_lifecycle/nvidia.rs` | All 4 NVIDIA lifecycles → hierarchy pinning |
| `crates/core/ember/src/vendor_lifecycle/amd.rs` | Both AMD lifecycles → hierarchy pinning |
| `crates/core/ember/src/vendor_lifecycle/steps.rs` | `PinBridgeHierarchy` lifecycle step |
| `crates/core/glowplug/src/sysfs_executor.rs` | Pre-unbind hierarchy pinning |
| `crates/server/src/background/pcie_keepalive.rs` | Generalized bridge keepalive, `SwapGuard`, startup auto-pin |
| `crates/server/src/background/mod.rs` | Updated module docs |
| `crates/server/src/glowplug_client.rs` | `swap()` activates `SwapGuard` |

## Quality Gates

| Gate | Result |
|------|--------|
| `cargo check --workspace` | 0 errors (1 pre-existing warning in pfifo.rs) |
| `cargo test -p toadstool-ember -p toadstool-glowplug -p toadstool-server --lib` | 821 passed, 0 failed |
| Hardware validation | K80 PLX round trip PASS |

## Upstream References

- hotSpring Exp 192: Hardware Validation Sprint — Compute Trio
- hotSpring Exp 193: PLX D3cold Keepalive — K80 Warm Swap Survival
