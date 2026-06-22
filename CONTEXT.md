# ToadStool — Context

> Per `PUBLIC_SURFACE_STANDARD.md` from wateringHole.

## What is ToadStool?

ToadStool is the **hardware infrastructure primal** ("WHERE") in the ecoPrimals sovereign compute stack. It discovers, routes, and manages GPUs, NPUs, and CPUs across a mesh of nodes — exposing them as JSON-RPC 2.0 capabilities over Unix sockets and TCP.

## Role in ecoPrimals

| Primal | Role |
|--------|------|
| **barraCuda** | Universal Math / "WHAT" — GPU shader dispatch, sovereign math |
| **toadStool** | Universal Hardware / "WHERE" — hardware discovery, workload routing, device lifecycle |
| **coralReef** | Sovereign Compiler / "HOW" — GPU compilation, ISA tables, shader pipeline |

ToadStool is the **Layer 0** hardware substrate that other primals and springs depend on for compute capability discovery and job execution. The glowPlug/ember subsystem provides hardware-agnostic device lifecycle management.

## Key Facts

- **License**: AGPL-3.0-or-later
- **Language**: Rust (edition 2024, MSRV 1.85)
- **IPC**: JSON-RPC 2.0 (primary) + tarpc (optional hot-path), newline-delimited over Unix sockets / TCP
- **Binary**: `toadstool` (UniBin standard — single binary, subcommands)
- **ecoBin grade**: v3.0 (zero application-level C dependencies)
- **Sockets** (dual-socket pattern):
  - `$XDG_RUNTIME_DIR/biomeos/compute.sock` — JSON-RPC 2.0 primary (biomeOS routes here)
  - `$XDG_RUNTIME_DIR/biomeos/compute-tarpc.sock` — tarpc hot-path (Rust-to-Rust peers)
  - Override: `TOADSTOOL_SOCKET` / `TOADSTOOL_TARPC_SOCKET` env vars
  - Family: `compute-{family_id}.sock` / `compute-{family_id}-tarpc.sock`
- **Peer primals**: Resolved at runtime via capability IDs and Unix-socket discovery (e.g. `capability.discover`, `resolve_capability_socket_fallback`) — not hardcoded URLs or legacy per-primal env manifests
- **Discovery hierarchy** (primalSpring cross-cutting): coordination service `ipc.resolve` → biomeOS `capability.discover` → UDS filesystem convention → socket registry → TCP probing. toadStool implements tiers 1–4; TCP probing (tier 5) not used for local IPC
- **Wave 8 Compute Trio** (S235–S263): `compute.dispatch.submit` trio-standard IPC contract. Phase A–D complete. **112 JSON-RPC methods** (direct, including bare `"health"` S315).
- **VFIO frontier** (Jun 2026): VFIO sovereign path validated through channel/DMA; blocked on PBDMA runlist configuration (Jun 1 RCA — `PFIFO_RUNLIST_BASE=0`, `GP_GET` stuck). Active spec: `specs/COMPUTE_DISPATCH_ENGINE.md`.
- **Deep Debt** (S240–S323+): All Duration literals → named constants (S321: `CPU_USAGE_SAMPLE_WINDOW` + 8 named settle/backoff constants). Zero production mocks/TODO/FIXME/unreachable!(). All unsafe SAFETY-documented. All `#[allow]`/`#[expect]` have `reason`; zero production `#[allow]` (S291). `cargo deny check bans` clean. Production panic surface eliminated (S273–S293). Zero production files >750L (S307+S320+S321+S322+S323, warm_swap + reagent + mmio + trials + method_gate + job + shader_dispatch + submit + cpu_resource refactored). 100% SPDX AGPL-3.0-or-later headers (S320+). Zero hardcoded cross-primal names in production logic (S320+). Dispatch hot-path zero-copy: `Arc<EncryptionKey>` cache, pipeline first-stage borrow, `binary_size` telemetry (S320+). S309 — TOADSTOOL-AUTO-REGISTER. S310 — kernel_sentinel AsFd. S315 — Wave 113 bare `"health"` + riboCipher REJECT. S318 — router split, `PRIMAL_SOCKET` deleted. S319 — gRPC + OpenCL **deleted** (−458 lines). S320 — Wave 114 MitoBeacon `0xED` acceptance on all accept loops. S321 — env literal purge (zero raw env strings), dep unification (bytes/ruzstd/serialport/ndarray workspace-unified), reagent/mod.rs smart refactor (704L→3 files). S322 — client riboCipher fix, composition graduation, ipc_watch coverage, mmio/trials test extraction, telemetry constants. S323 — test extraction (method_gate/job/shader_dispatch), submit param split, cpu_resource resilience extraction, flaky test fix, edge communication coverage.
- **Transport**: `TRANSPORT_ENDPOINT` env var accepted at all server paths (sourDough wire-compatible, S301–S302); `connect_transport()` for outbound; `IpcClient::from_transport_endpoint()` bridge; BYOB default bind `127.0.0.1`
- **Socket cleanup**: `BIOMEOS_SOCKET_DIR` wired into all socket/discovery-file resolution chains (S308, Wave 107). Zero `/tmp` writes when `BIOMEOS_SOCKET_DIR` set. `ProtectSystem=strict` compatible.
- **Tests**: 23,000+ (9,095+ lib-only, 0 failures, unlimited parallelism)
- **Unsafe**: 44 blocks (all in hw-safe/GPU/VFIO/display/plugin containment, all SAFETY-documented; S310: −2 via kernel_sentinel AsFd evolution); workspace `unsafe_code = "deny"`, 41 crates `forbid` + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`; all lint attrs have `reason =` (S211+S213)
- **async-trait**: DEPRECATED — fully removed and banned in `deny.toml` (S203r); transitive only via axum/config/wiggle
- **deny.toml**: `ring` + `async-trait` + `zstd-sys` + `aws-lc-sys` bans active (ecoBin v3 compliant, `SOVEREIGNTY_STANDARDS.md` dark forest gate). `ring` present only as conditional transitive dep via quinn-proto/rustls-webpki (not on default build path)
- **Display Phase 2**: `display.present`, `display.subscribe_input`, `display.poll_events` (petalTongue IPC)
- **Encrypted compute dispatch** (Phase 55): Tower `crypto.encrypt`/`crypto.decrypt` for payloads; `DISCOVERY_SOCKET` highest-precedence capability resolution
- **Self-registration** (S207): `ipc.register` to coordination service via `DISCOVERY_SOCKET` at startup — dynamic NUCLEUS membership without restart
- **Health probes**: `health.liveness` always returns `{"status":"alive"}` — liveness means the socket is reachable (S272, per `DEPLOYMENT_BEHAVIOR_STANDARD.md`). `health.readiness` returns `{"status":"starting"}` during boot, `{"status":"ready"}` once fully initialized. Callers should use **>= 3 second** probe timeouts (PG-62, S225)
- **MethodGate JH-0** (S229): Pre-dispatch capability gate. Methods classified Public/Protected. `GateMode::Permissive` (default) or `GateMode::Enforcing` (via `TOADSTOOL_AUTH_MODE` env var). Error codes: `-32000 UNAUTHORIZED`, `-32001 PERMISSION_DENIED` (ecosystem standard). `auth.check`, `auth.mode`, `auth.peer_info` introspection methods
- **JH-2 Resource Envelope Enforcement — FULLY RESOLVED** (S231–S232, audited S238): `ResourceEnvelope` (`mem_mb`, `cpu_cores`, `max_timeout_ms`, `method_allowlist`) enforced at all dispatch paths (`submit`, `shader.dispatch`, pipeline stages). Pipeline stages inherit `CallerContext`. All 3 dimensions confirmed enforced
- **BTSP**: Phase 3 encrypted channel (ChaCha20-Poly1305, S215); transport switch verified (S218); 13/13 converged JSON-line relay + NDJSON post-handshake (primalSpring Phase 45c); PG-46 resolved (connection-reused handshake, S214)
- **Dep hygiene**: `test-mocks` off by default (S206); all workspace deps unified
- **Monitoring**: Real host queries via `toadstool_sysmon` + `rustix::fs::statvfs`
- **Logging**: All production code uses `tracing` (structured logging standard); `println!`/`eprintln!` retained only in standalone CLI binaries and test code (S233, S240)
- **Config**: All `TOADSTOOL_*` env vars interned to `socket_env` constants (100% env centralized — zero production raw env string literals, S321; ~410+ reads via socket_env constants). `TOADSTOOL_GATE_ID` (local gate identity), `TOADSTOOL_HARDWARE_OWNER_GATE_ID` (yield-to-owner hardware owner, S286). `TOADSTOOL_AUTH_MODE` controls gate mode. Discovery/config defaults use named constants (S236, S238). 20+ duplicated magic numbers consolidated into `common::defaults` module and per-struct constants (S238). Capability-based primal references throughout CLI/dispatch (S273+S288)

## Not Included

- No telemetry or phone-home
- No cloud provider SDK dependencies
- No PII collection

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
