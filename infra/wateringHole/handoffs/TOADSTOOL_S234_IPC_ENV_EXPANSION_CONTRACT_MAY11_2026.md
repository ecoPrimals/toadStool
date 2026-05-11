# ToadStool S234 — IPC Env Var Expansion Contract

**Date**: May 11, 2026
**Session**: S234
**Scope**: primalSpring stadial audit response — IPC contract documentation

---

## Audit Finding

primalSpring stadial gate audit identified that the CLI workload loader
(`load_workload_file`) expands `${VAR}` / `$VAR` in TOML/JSON workload specs,
but the JSON-RPC `compute.execute` server path takes pre-structured JSON
without expansion. Callers going through IPC (not CLI) see no env var expansion.

## Resolution: Pre-Resolved Contract (Option 2)

The IPC contract is explicitly documented as **"pre-resolved only"**:

- All JSON-RPC methods expect fully resolved parameter values
- `${VAR}` / `$VAR` expansion is a CLI-only convenience in `load_workload_file`
- The server does not expand env vars — this is intentional:
  - In cross-primal composition, the server's env differs from the caller's
  - Implicit expansion would create ambiguity about whose env applies
  - Security concern: callers could probe server env state

This matches the de facto behavior since inception — no code change needed,
only documentation.

## Changes

### Code-Level Documentation
- `crates/server/src/pure_jsonrpc/handler/workload.rs`: Doc comment on
  `submit_workload` explaining pre-resolved contract
- `crates/server/src/pure_jsonrpc/handler/dispatch/submit.rs`: Doc comment on
  `dispatch_submit_with_context` explaining pre-resolved contract

### Method Reference
- `crates/server/src/pure_jsonrpc/METHODS.md`: Added `compute.execute` to
  method table (was missing). Added "IPC Contract: Pre-Resolved Values"
  section with expansion table (CLI vs JSON-RPC paths).

### README
- Added "IPC Contract: Pre-Resolved Values" section under Dispatch Timeouts
- Added `compute.execute` to methods table

### NEXT_STEPS.md
- Updated header and latest session to S234

## Guidance for Graph Specs and Composition Callers

- Use absolute paths in graph spec TOML files, not `$HOME` or `$WETSPRING_DIR`
- Pre-expand variables on the client side before sending JSON-RPC requests
- The `metadata` HashMap in `JsonWorkloadSubmission` is passed through as
  literal string key-value pairs — no server-side interpretation

## Open Items from Audit (Other Primals)

Items tracked for documentation only — toadStool has no action items beyond
the IPC contract doc:

- **NestGate** (CRITICAL): `content.*` transport routing parity — blocks 3 primals
- **skunkBat** (MEDIUM): JH-5 Phase 3 forwarding
- **squirrel** (MEDIUM): Compute delegation to toadStool IPC — `COMPUTE_ENDPOINT`
  env + capability discovery not wired
- **barraCuda** (LOW): Crypto dedup to bearDog IPC
- **songbird, coralReef**: CLEAN (stadial only)

toadStool's sole item from this audit is **RESOLVED**.
