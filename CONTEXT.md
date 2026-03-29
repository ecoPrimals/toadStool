# ToadStool — Context

> Per `PUBLIC_SURFACE_STANDARD.md` from wateringHole.

## What is ToadStool?

ToadStool is the **hardware infrastructure primal** ("WHERE") in the ecoPrimals sovereign compute stack. It discovers, routes, and manages GPUs, NPUs, and CPUs across a mesh of nodes — exposing them as JSON-RPC 2.0 capabilities over Unix sockets and TCP.

## Role in ecoPrimals

| Primal | Role |
|--------|------|
| **barraCuda** | Compiler / "HOW" — GPU shader compilation, sovereign math |
| **toadStool** | Infrastructure / "WHERE" — hardware discovery, workload routing |
| **coralReef** | Driver / "BRIDGE" — VFIO passthrough, kernel-level dispatch |

ToadStool is the **Layer 0** hardware substrate that other primals and springs depend on for compute capability discovery and job execution.

## Key Facts

- **License**: AGPL-3.0-only
- **Language**: Rust (edition 2024, MSRV 1.85)
- **IPC**: JSON-RPC 2.0 (primary) + tarpc (optional high-perf), newline-delimited over Unix sockets / TCP
- **Binary**: `toadstool` (UniBin standard — single binary, subcommands)
- **ecoBin grade**: v3.0 (zero application-level C dependencies)
- **Socket**: `$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock`

## Not Included

- No telemetry or phone-home
- No cloud provider SDK dependencies
- No PII collection

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
