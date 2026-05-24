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
- **Discovery hierarchy** (primalSpring cross-cutting): Songbird `ipc.resolve` → biomeOS `capability.discover` → UDS filesystem convention → socket registry → TCP probing. toadStool implements tiers 1–4; TCP probing (tier 5) not used for local IPC
- **Wave 8 Compute Trio** (S235–S263): `compute.dispatch.submit` trio-standard IPC contract. Phase A–D complete. NV VFIO e2e dispatch validated on Titan V (S263). **87 JSON-RPC methods.**
- **Deep Debt** (S240–S266): All Duration literals → named constants. Zero production mocks/TODO/FIXME/unreachable!(). All unsafe SAFETY-documented. All `#[allow]`/`#[expect]` have `reason`. `cargo deny check bans` clean. Stale socket hygiene (S264). sporePrint Wave 28 (S265). Sandbox `working_dir` production + `data_dependencies` validation (S266).
- **Tests**: 23,000+ (9,122+ lib-only, 0 failures, unlimited parallelism)
- **Unsafe**: 46 blocks (all in hw-safe/GPU/VFIO/display/plugin containment, all SAFETY-documented; reconciled S221); workspace `unsafe_code = "deny"`, 41 crates `forbid` + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`; all lint attrs have `reason =` (S211+S213)
- **async-trait**: DEPRECATED — fully removed and banned in `deny.toml` (S203r); transitive only via axum/config/wiggle
- **deny.toml**: `ring` + `async-trait` + `zstd-sys` + `aws-lc-sys` bans active (ecoBin v3 compliant, `SOVEREIGNTY_STANDARDS.md` dark forest gate). `ring` present only as conditional transitive dep via quinn-proto/rustls-webpki (not on default build path)
- **Display Phase 2**: `display.present`, `display.subscribe_input`, `display.poll_events` (petalTongue IPC)
- **Encrypted compute dispatch** (Phase 55): Tower `crypto.encrypt`/`crypto.decrypt` for payloads; `DISCOVERY_SOCKET` highest-precedence capability resolution
- **Self-registration** (S207): `ipc.register` to Songbird via `DISCOVERY_SOCKET` at startup — dynamic NUCLEUS membership without restart
- **Health probes**: `health.liveness` always returns `{"status":"alive"}` — liveness means the socket is reachable (S272, per `DEPLOYMENT_BEHAVIOR_STANDARD.md`). `health.readiness` returns `{"status":"starting"}` during boot, `{"status":"ready"}` once fully initialized. Callers should use **>= 3 second** probe timeouts (PG-62, S225)
- **MethodGate JH-0** (S229): Pre-dispatch capability gate. Methods classified Public/Protected. `GateMode::Permissive` (default) or `GateMode::Enforcing` (via `TOADSTOOL_AUTH_MODE` env var). Error codes: `-32000 UNAUTHORIZED`, `-32001 PERMISSION_DENIED` (ecosystem standard). `auth.check`, `auth.mode`, `auth.peer_info` introspection methods
- **JH-2 Resource Envelope Enforcement — FULLY RESOLVED** (S231–S232, audited S238): `ResourceEnvelope` (`mem_mb`, `cpu_cores`, `max_timeout_ms`, `method_allowlist`) enforced at all dispatch paths (`submit`, `shader.dispatch`, pipeline stages). Pipeline stages inherit `CallerContext`. All 3 dimensions confirmed enforced
- **BTSP**: Phase 3 encrypted channel (ChaCha20-Poly1305, S215); transport switch verified (S218); 13/13 converged JSON-line relay + NDJSON post-handshake (primalSpring Phase 45c); PG-46 resolved (connection-reused handshake, S214)
- **Dep hygiene**: `test-mocks` off by default (S206); all workspace deps unified
- **Monitoring**: Real host queries via `toadstool_sysmon` + `rustix::fs::statvfs`
- **Logging**: All production code uses `tracing` (structured logging standard); `println!`/`eprintln!` retained only in standalone CLI binaries and test code (S233, S240)
- **Config**: All `TOADSTOOL_*` env vars interned to `socket_env` constants. `TOADSTOOL_AUTH_MODE` controls gate mode. Discovery/config defaults use named constants (S236, S238). 20+ duplicated magic numbers consolidated into `common::defaults` module and per-struct constants (S238)

## Not Included

- No telemetry or phone-home
- No cloud provider SDK dependencies
- No PII collection

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
