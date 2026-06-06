# toadStool S296 — Coverage Push II + Musl Build

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Coverage push + VPS binary ready.

## Coverage Push (+35 tests → 8,987 total)

### ember.rs (10 tests)
- `ember.list` — device listing response shape
- `ember.status` — uptime + device array
- `device.get` — missing/unknown BDF error paths
- `ember.reacquire`, `device.swap`, `device.warm_catch`, `device.experiment_lifecycle` — param validation

### dispatch/submit.rs (17 tests)
- `resolve_binary_param` — missing binary, invalid base64
- `resolve_workgroup_size`, `resolve_buffers`, `resolve_shader_info` — defaults and alias handling
- `detect_dispatch_mode` — explicit mode + non-string fallback
- Thermal gate — `ThermalCritical` rejection + passthrough recording
- `dispatch_submit_with_context` — CPU cores envelope enforcement
- Integration — missing params, missing binary, custom dispatch mode

### Background services (5 tests)
- `cleanup::find_timed_out_execution_ids` — none expired, detects expired, boundary
- `resource::update_stats_on_tick` — uptime increment, peak concurrency tracking

### CLI start.rs (3 tests)
- Unsupported Git workload source → error
- WASM missing file → error
- WASM checksum mismatch → error

## Musl-Static Binary

```
-rwxrwxr-x 14M target/x86_64-unknown-linux-musl/release/toadstool
ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped
```

Ready for VPS redeployment via:
```bash
scp target/x86_64-unknown-linux-musl/release/toadstool root@157.230.3.183:/opt/toadstool/bin/
# Then on VPS: systemctl restart toadstool
# systemd unit should use: toadstool server --socket /run/membrane/toadstool.sock --headless
```

## Metrics

- **8,987** lib tests passed (up from 8,952), 0 failed
- Full workspace clippy `-D warnings` clean
- Musl build: 14MB static binary

## Remaining Debt

1. **Coverage**: ~85% estimated → target 90%. Next: `dispatch/wgpu_dispatch.rs`, `transport.rs` deeper coverage, CLI command modules
2. **CallerContext full threading**: identity + envelope still not populated from ionic tokens
3. **LEGACY env staged removal**: 23 reads now have deprecation tracing
4. **Files 750L+**: `bar_cartography.rs` (782L), `pmu.rs` (771L) in cylinder
