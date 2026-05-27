# S267: Sovereign Driver Rotation — Per-GPU Module Lifecycle via Diesel Engine

**Date:** May 20, 2026
**Scope:** cylinder + glowplug + server (3 crates, 3 new modules, 1 new RPC)
**Status:** Implemented, tested (20 new tests + 1 doctest), all 105 cylinder+glowplug tests pass
**Upstream from:** hotSpring Exp 211 binary-patch warm handoff results

## Context

Exp 211 proved that binary-patching `nouveau.ko` (NOP teardown functions at
offset +5 after ftrace call) preserves GPU hardware state through the
nouveau→vfio-pci warm handoff cycle. But the patching was done manually via
shell scripts — violating the primal principle:

> "with the primals we never need to touch the kernel of the OS ever."

This session codifies driver rotation as a first-class diesel engine operation.
The toadStool daemon manages the entire module lifecycle per-GPU via RPC.

## What Shipped

### Layer 1: `cylinder::vfio::kmod` (new module)

Kernel module lifecycle management via `std::process::Command`:

| Function | Purpose |
|----------|---------|
| `is_module_loaded(name)` | Check `/sys/module/{name}` |
| `find_stock_module(name)` | Locate `.ko` via `modinfo -n` |
| `load_module(path)` | `insmod` |
| `unload_module(name)` | `rmmod` |
| `ensure_module_loaded(name)` | Load-if-absent convenience |
| `nm_text_symbols(ko_path)` | Resolve symbol offsets for patching |

Consistent with Akida NPU precedent (`akida-setup/src/pcie.rs`).

### Layer 2: `cylinder::vfio::module_patch` (new module)

Binary NOP patcher with predefined patch sets:

- **`PatchStrategy::RetAfterFtrace`** — skip 5-byte ftrace `call` at function
  entry, write `0xC3` (ret) at offset +5. Proven on kernel 6.17.9.
- **`PatchSet::volta_warm_handoff()`** — NOPs: `gf100_gr_fini`,
  `nvkm_pmu_fini`, `nvkm_mc_disable`, `nvkm_mc_reset`, `gk104_fifo_fini`
- **`PatchSet::kepler_warm_handoff()`** — same targets for K80
- **`patch_module(source_ko, patch_set)`** — read → `nm` → patch → write to
  `/tmp/toadstool-patched-{name}.ko`

### Layer 3: `glowplug::warm_init::ModuleSource` (extension)

New enum on `SeederDriver`:

```rust
pub enum ModuleSource {
    System,                                    // stock module (K80, nvidia-470 VM)
    Patched { stock_module, patch_set },       // diesel engine patches at runtime
}
```

- `nouveau_titanv()` → `Patched { "nouveau", "volta_warm_handoff" }`
- `nouveau_k80()` → `System` (unsigned falcons, stock nouveau fully inits)
- Backward-compatible via `#[serde(default)]`

### Layer 4: `cylinder::vfio::sovereign_handoff` (new module)

8-step pipeline orchestrator:

1. Module preparation (find → patch → insmod, or ensure system module loaded)
2. Unbind current driver
3. Bind seeder via `driver_override` + `drivers_probe`
4. Settle (configurable wait for hardware init)
5. Pin bridge hierarchy + disable FLR
6. Warm swap (unbind seeder → override → bind final driver)
7. Tier classification (BAR0 probe via sysfs fallback)
8. Module cleanup (rmmod + delete tmpfile)

Returns `HandoffResult` with per-step timing, `TierEvidence`, and patch details.

### Layer 5: `sovereign.warm_handoff` RPC

New JSON-RPC method in dispatch handler:

```json
{"method": "sovereign.warm_handoff", "params": {"bdf": "0000:02:00.0", "strategy": "nouveau_titanv"}}
```

Strategies: `nouveau_titanv` (patched), `nouveau_k80` (stock).
Runs synchronously via `tokio::task::spawn_blocking`.

## Constraints Honored

- **Host DRM sacred**: Display GPU never touched; nvidia-580 stays loaded
- **No kernel module swaps for conflicting drivers**: nvidia-470 uses
  `SeederContainment::Contained` (VM path, not yet implemented)
- **nouveau safe for bare-metal**: Coexists with nvidia-580 (different module)
- **Binary patching is ephemeral**: patched `.ko` in `/tmp`, rmmod'd after use
- **Per-GPU granularity**: Each RPC targets one BDF with one strategy

## Test Results

- 20 new tests + 1 doctest pass
- All 105 existing cylinder+glowplug tests pass (backward compatible)
- Zero linter errors introduced
- Server crate compiles clean (pre-existing `compute_fan_out` test failure unrelated)

## Files Changed

### New files
- `crates/core/cylinder/src/vfio/kmod.rs`
- `crates/core/cylinder/src/vfio/module_patch.rs`
- `crates/core/cylinder/src/vfio/sovereign_handoff.rs`

### Modified files
- `crates/core/cylinder/src/vfio/mod.rs` — register 3 new modules
- `crates/core/glowplug/src/warm_init.rs` — `ModuleSource` enum + field
- `crates/core/glowplug/src/lib.rs` — export `ModuleSource`
- `crates/server/src/pure_jsonrpc/handler/mod.rs` — match arm
- `crates/server/src/pure_jsonrpc/handler/dispatch/mod.rs` — handler method

## Next Steps (upstream for hotSpring)

1. **K80 hardware validation**: When replacement arrives, run
   `sovereign.warm_handoff` with `nouveau_k80` strategy — expect Tier 2
   (unsigned falcons → full FECS init → GPC powered)
2. **PMU firmware extraction**: Extract signed PMU blobs from nvidia-470 package,
   load via toadstool sovereign pipeline (vendor atheistic approach for Volta)
3. **VBIOS interpreter completion**: True silicon deism — GPU initializes itself
4. **agentReagents VM path**: Implement `SeederContainment::Contained` for
   nvidia-470 warm handoff without DRM contamination
