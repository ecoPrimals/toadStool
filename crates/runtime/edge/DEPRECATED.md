# DEPRECATED — `toadstool-runtime-edge`

This crate is **orphaned** and **excluded from the workspace**. It is not built by
default, has zero workspace dependents, and is preserved only as a fossil record of
early edge/IoT runtime work.

## Status

| Property | Value |
|----------|-------|
| Crate name | `toadstool-runtime-edge` |
| Workspace member | No — commented out in root `Cargo.toml` members list |
| Workspace exclude | Yes — `exclude = ["fuzz", "crates/runtime/edge"]` (S378) |
| Dependents | None |
| Default build | Never |

## Intended purpose (never completed)

The crate was meant to provide a **Edge/IoT runtime engine** for ToadStool: universal
compute orchestration for edge devices, IoT platforms, and embedded systems (Arduino,
ESP32, Raspberry Pi, and similar). Modules include:

- `communication` — device communication protocols
- `deployment` — deployment coordination
- `discovery` — USB, serial, Bluetooth, mDNS, and network discovery
- `platforms` — platform-specific adapters (e.g. ESP32)
- `serial_transport` — USB-UART serial (optional `serial-transport` feature)
- `toolchain` — cross-compilation helpers
- `udev_pure` — pure-Rust `/sys/class` parsing (Linux only)

The `RuntimeEngine` implementation in `src/lib.rs` was started but never wired into
production server/CLI paths. Discovery stubs were partially filled (S203m), but end-to-end
edge deployment and orchestration were never finished.

## Why it was excluded (S378)

During **S378 — Tokio Vestigial Segmentation** (Aug 10, 2026), this crate was moved to
`[workspace].exclude` because:

- Zero workspace crates depend on it
- It is never built in CI or coverage runs
- Optional deps (`reqwest`, `serialport`) conflict with the project's pure-Rust / ecoBin
  policy when enabled (`http-downloads`, `serial-transport` features)
- Edge/IoT orchestration is not on the current product roadmap

Code is **preserved, not deleted** — it may contain useful patterns for future edge support.

## Policy exceptions

`deny.toml` documents that `reqwest` is allowed **only** behind the optional
`http-downloads` feature on this excluded crate. No core, server, or CLI crate may
depend on `reqwest`.

## Future action required

In a future sprint, either:

1. **Fully implement** edge/IoT runtime support — re-add to workspace, complete
   `RuntimeEngine` wiring, resolve pure-Rust policy for serial/HTTP deps, and add
   integration tests; or
2. **Remove** the crate entirely if edge support is permanently out of scope.

Do not leave this crate in limbo indefinitely.

## References

### CHANGELOG.md

- **Session S378 (Aug 10, 2026)** — `runtime/edge` excluded from workspace; orphaned
  crate with zero dependents moved to workspace `exclude`.
- Earlier history: ESP32 module decomposition, test extraction (`lib.rs` 636→404 LOC),
  workspace dep unification (`tokio`, `serde`, `uuid`), mdns removal, cache path fix.

### DEBT.md

- **S378 (strandGate Tokio Vestigial Segmentation — Aug 10, 2026)** — `runtime/edge`
  excluded from workspace (orphaned); code preserved as fossil record, not deleted.
- **D-EDGE-DISCOVERY-STUBS — RESOLVED S203m** — USB/Bluetooth/IPv6 discovery stubs
  replaced with real sysfs/proc enumeration (scope: `runtime/edge`, 3 modules).
- **D-LARGE-FILE-REFACTOR-S203C — RESOLVED S203c** — `runtime/edge/lib.rs` tests
  extracted to `lib_tests.rs` (636→404 LOC).
- **S211** — workspace dependency unification for `tokio`, `serde`, `uuid` in
  `runtime/edge`.

### Other

- `scripts/run-coverage.sh` — explicitly excludes `toadstool-runtime-edge` from coverage
  (serialport → libudev C dep when `serial-transport` is enabled).
- `infra/wateringHole/handoffs/archive/TOADSTOOL_S378_VESTIGIAL_SEGMENTATION_AUG10_2026.md`
  — handoff notes for the exclusion decision.
