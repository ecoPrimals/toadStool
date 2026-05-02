# ToadStool S216+S217 Handoff — Deep Debt Evolution + Flaky Test Fix + Orphan Recovery

**From**: toadStool team
**Date**: 2026-05-02
**Sessions**: S216, S217

---

## S216 — Production Stub Evolution + Dependency Hygiene + Lock Safety

### Changes

1. **Message queue transport stub evolved**: `submit_via_message_queue` no longer
   fabricates a synthetic `CoordinationJobResponse::Success`. Now returns
   `ToadStoolError::NotSupported` with capability-based guidance, matching the
   HTTP and gRPC paths. No production code fabricates success without real work.

2. **ResourceOrchestrator lock safety**: All 12 `.expect("lock poisoned")` calls
   evolved to `Result<_, OrchestrationError::LockPoisoned>`. Methods affected:
   `register_tenant`, `release`, `tenant_usage`, `all_usage`, `device_count`,
   `allocate_local_direct`, `allocate_local_multi`, `check_quota`. Combined with
   S213's `WorkloadOrchestrator` evolution, zero lock-poisoning panics remain in
   the orchestration layer.

3. **Dependency hygiene**:
   - `tar` 0.4.44 → 0.4.45 (RUSTSEC advisory resolved)
   - `drm` 0.14.2 → 0.14.1 (yanked version resolved)
   - `deny.toml` stale skip entries removed (naga, wgpu — both single-version)
   - `ring` confirmed absent from dependency tree
   - `cargo deny check` all four gates pass clean

### Comprehensive Audit Results

| Dimension | Finding |
|-----------|---------|
| Large files (>800L) | None in production code |
| Unsafe code (49 blocks) | All legitimate hw containment, SAFETY-documented |
| Hardcoded paths | All env-configurable or kernel-standard |
| Box<dyn Trait> | All open-by-design (runtime registration) |
| Production stubs | MQ stub was last synthetic success — now fixed |

---

## S217 — Flaky Test Fix + Orphan Module Recovery + Coverage Expansion

### Changes

1. **Flaky `primal_sockets` test fixed**: Long-standing env-var race condition
   where parallel tests setting `DISCOVERY_SOCKET` polluted the environment for
   `get_socket_path_for_capability("coordination")`. Fixed by wrapping convenience
   API tests in `temp_env::with_vars` to ensure clean env state.

2. **Orphan module recovery**: 6 source files in `integration-primals` were
   discovered to be completely orphaned (not in `lib.rs` module tree, never
   compiled). All wired in:
   - `error.rs` — `PrimalError` / `PrimalResult` types with ToadStoolError conversions
   - `client.rs` — Unix JSON-RPC client for primal communication
   - `orchestrator.rs` — Biome deployment orchestrator with manifest validation
   - `services.rs` — Service manager with capability-based discovery
   - `manifest/` — Rich BiomeManifest with storage/agents/security/federation
   - `types/` — PrimalRegistry, PrimalCapability descriptor

3. **Coverage expansion**: 35+ new inline tests across 5 previously-untested modules.

### Quality Gates

- 22,429+ tests, 0 failures
- `cargo clippy` — 0 warnings
- `cargo fmt --check` — 0 diffs
- `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok

---

## Impact on Downstream

- **primalSpring**: No wire format changes. BTSP Phase 3 (S215) encrypted channel
  remains compatible. The MQ transport stub change has no downstream impact since
  no springs used it.
- **All primals**: `cargo deny check` advisories now clean (tar/drm resolved).
  Teams should run `cargo update -p tar -p drm` if they share these transitive deps.

---

## Remaining Debt

- **Test coverage**: ~83.6% line coverage, target 90%. Remaining gaps are
  hardware-dependent paths (VFIO, DRM, V4L2, akida). Require mock hardware infra.
- **integration-primals docs**: Newly wired modules have `allow(missing_docs)` —
  docs will be added incrementally.
- **BTSP Phase 3 tarpc**: Encrypted framing not yet integrated with tarpc codec
  (uses different stream abstraction).
