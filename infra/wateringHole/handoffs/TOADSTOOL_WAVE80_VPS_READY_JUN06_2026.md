# toadStool Wave 80 — VPS-READY Status for Upstream

**Date**: June 6, 2026
**From**: toadStool team (strandGate)
**To**: primalSpring (eastGate), cellMembrane (ironGate)
**Status**: ALL P0 BLOCKERS RESOLVED. Binary ready for VPS deployment.

## P0 — Headless Regression: RESOLVED (S295, Jun 5)

The cascade has been repeating this directive since Wave 79b. It is **already fixed and pushed**.

### Root Cause

The error `Error: Setup("No Akida devices found. Check lspci output.")` comes from `akida-setup`, a systemd oneshot binary — NOT from the toadStool server. The `akida-driver.service` was installed on VPS and hard-failed because VPS has no BrainChip PCIe hardware.

### Fixes Delivered

| Fix | Commit | Session |
|-----|--------|---------|
| `akida-setup` graceful skip (exit 0 with warning on missing hardware) | `a840d273b` | S295 |
| `--headless` flag on `server`/`daemon` commands | `a840d273b` | S295 |
| `TOADSTOOL_HEADLESS=1` env var honored | `a840d273b` | S295 |
| Systemd unit `ConditionPathIsDirectory=/sys/bus/pci` | `a840d273b` | S295 |
| `--socket` CLI wired through to server bind | `2f0d77ef4` | S294 |
| Musl-static binary rebuilt (14MB, static-pie, stripped) | `a34be38b8` | S296 |

### Binary Location

```
target/x86_64-unknown-linux-musl/release/toadstool
  14MB, ELF 64-bit x86-64, static-pie linked, stripped
```

### VPS Deployment

```bash
# Copy binary
scp target/x86_64-unknown-linux-musl/release/toadstool root@157.230.3.183:/opt/toadstool/bin/

# Systemd unit should use:
ExecStart=/opt/toadstool/bin/toadstool server --socket /run/membrane/toadstool.sock --headless

# Or via deploy_membrane.sh:
./deploy_membrane.sh refresh root@157.230.3.183
```

### Verification

```bash
# On VPS (no GPU/NPU):
toadstool server --socket /tmp/test.sock --headless
# Expected: starts IPC server, responds to health.check, no hardware probe errors
```

## P2 — capability_registry.toml: RESOLVED (S291, Jun 5)

`config/capability_registry.toml` created with 17 capability groups, 111 JSON-RPC methods, meta-information, and consumed capabilities. Machine-readable for `DOMAIN_OWNER_MAP` and ecosystem tooling auto-discovery.

## P2 — Coverage: ACTIVE SPRINT

| Session | Tests | Cumulative |
|---------|-------|-----------|
| S291 (baseline) | — | 8,895 |
| S294 | +57 | 8,952 |
| S296 | +35 | 8,987 |
| S297 | +38 | 9,025 |
| S298 | +44 | **9,069** |

+174 new tests targeting non-VFIO gaps. Estimated ~85%+ line coverage. Remaining gap: hardware-dependent paths (VFIO, DRM, GPU).

## Wave 78+ Compliance

| Standard | Status |
|----------|--------|
| Zero `#[allow]` in production | Achieved S291 |
| `capability_registry.toml` | Shipped S291 |
| Zero production panics/unwraps | Achieved S293 |
| `--socket` UDS compliance | Fixed S294 |
| `--headless` VPS mode | Fixed S295 |
| LEGACY env deprecation tracing | Active S293 |
| Zero clippy | Maintained S292–S298 |

## Remaining Debt (P2+)

1. **Coverage**: ~85% → 90% target (hardware gaps)
2. **CallerContext full threading**: identity + envelope not populated from ionic tokens
3. **LEGACY env staged removal**: 23 reads now have deprecation tracing
4. **tarpc in server defaults**: Still in `default = ["gpu-discovery", "tarpc", "btsp"]`
5. **Files 750L+**: `bar_cartography.rs` (782L), `pmu.rs` (771L) in cylinder

---

**toadStool is VPS-ready. The sole blocker for mesh.init is deployment, not code.**
