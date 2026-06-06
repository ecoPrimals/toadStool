# toadStool S295 — Wave 79b: Headless Mode + akida-setup Graceful Skip

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: P0 VPS deployment regression FIXED.

## Root Cause Analysis

The VPS binary failed with `Error: Setup("No Akida devices found. Check lspci output.")`.

This error comes from the `akida-setup` binary (a systemd oneshot), NOT from the toadStool server. The `akida-driver.service` was installed on the VPS and hard-failed because VPS has no BrainChip PCIe hardware. The toadStool server itself has no Akida dependency in its startup path.

## Fixes

### akida-setup: graceful hardware skip

`akida-setup` now exits 0 with a warning when no Akida devices are found, instead of returning `Err(SetupError::Setup(...))`. This makes the systemd oneshot idempotent on hardware-less systems.

### CLI --headless flag

Added `--headless` to both `Server` and `Daemon` commands:
- Threads through `run_server_daemon` → `run_server_main` → `UnibinExecutionConfig.headless`
- When headless, `create_executor` calls `query_baseline_only()` (CPU-only capabilities, no GPU/NPU probes)
- `TOADSTOOL_HEADLESS=1` env var also honored (read from `UnibinExecutionConfig::from_env`)

### Systemd unit hardening

`akida-driver.service` template updated with `ConditionPathIsDirectory=/sys/bus/pci` to skip on systems without PCI bus.

## VPS Deployment Command

```bash
toadstool server --socket /run/membrane/toadstool.sock --headless
```

## Files Changed

| File | Change |
|------|--------|
| `neuromorphic/akida-setup/src/main.rs` | Graceful exit when no devices |
| `cli/commands/definitions.rs` | `--headless` flag on Server + Daemon |
| `cli/commands/dispatch/mod.rs` | Thread headless to run_server_daemon |
| `cli/commands/dispatch/server.rs` | Accept + forward headless param |
| `server/unibin/mod.rs` | Accept headless, set on config |
| `server/unibin/execution.rs` | `headless` field on UnibinExecutionConfig |
| `server/unibin/capabilities.rs` | `query_baseline_only()` for headless mode |
| `scripts/install-akida-driver.sh` | ConditionPathIsDirectory on systemd unit |
| Tests | 3 new headless flag tests + existing test updates |

## Metrics

- **8,952** lib tests, 0 failed
- Zero clippy workspace-wide

## VPS Status After This Fix

toadStool binary can now be deployed port-free on VPS:
```
toadstool server --socket /run/membrane/toadstool.sock --headless
```
This starts the IPC server in pure-compute mode without any hardware enumeration.
