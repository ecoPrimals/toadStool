# toadStool S291 — Wave 78 Parity: Capability Registry + Zero Production #[allow]

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Wave 78 parity items complete. Software-only.

## Changes

### Capability Registry

Created `config/capability_registry.toml` — machine-readable capability declaration per Wave 78 ecosystem standard:
- 17 capability groups covering all 111 JSON-RPC methods
- Consumed capabilities documented (security, storage, coordination)
- Meta section with primal identity, version, protocol, transport, sockets

### Zero Production #[allow]

Eliminated all 77 production `#[allow]` attributes across 54 files:

| Category | Count | Action |
|----------|-------|--------|
| Stale (lint no longer fires) | 14 | Deleted |
| Convert to `#[expect]` | 58 | Converted with `reason` |
| `unsafe_code` justified | 4 | Converted to `#[expect]` |
| cfg-gated | 4 | Converted to `#[cfg_attr(..., expect(...))]` |

Remaining `#[allow]` in codebase: 13 — all in `#[cfg(test)]` blocks (test-only, Wave 78 compliant).

## Metrics

- **8,895** lib tests passed (default), 0 failed
- Full workspace clippy `-D warnings` clean
- 55 files changed, 92 insertions, 196 deletions (net -104 lines)
- Zero production `#[allow]` — Wave 78 compliant

## Wave 78 Compliance

| Standard | Status |
|----------|--------|
| Zero clippy (pedantic + nursery) | ✓ |
| Zero `#[allow]` in production | ✓ (S291) |
| `capability_registry.toml` | ✓ (S291) |
| BTSP Phase 3 | ✓ |
| Wire Standard L2+ | ✓ (L3 partial) |
| MethodGate pre-dispatch | ✓ |
| plasmidBin ecoBin compliant | ✓ |
| `forbid(unsafe_code)` or justified | ✓ (41 forbid + 5 justified opt-out) |
| 90% line coverage | Pending (84% → 90%) |

## Remaining for Wave 78

1. **Coverage push**: 84% → 90% — focus on non-VFIO paths (server handlers, CLI, distributed)
