# toadStool — Wave 49 "Primals on the Mountain" Response

**Date**: May 25, 2026
**Session**: S275
**From**: toadStool team
**To**: primalSpring (downstream audit)
**Audit ref**: Wave 49 — Primals on the Mountain (May 25, 2026)

---

## toadStool Items — Both Resolved

### 1. Archive hygiene — RESOLVED (this session)

**Audit**: `infra/wateringHole/` has 37 flat handoffs, no `archive/` subdir.

**Fix**: Created `archive/` subdir. 28 superseded handoffs (S243–S266) moved
to `archive/`. 9 active handoffs remain at top level (S267–S275).

Split boundary: S267+ = active (deep debt evolution, primalSpring wave
responses). S243–S266 = Phase C absorption, diesel engine, hotspring
exchanges — all resolved work.

### 2. Slow startup (>8s cold) — ALREADY RESOLVED (S275, earlier this session)

**Audit**: Cold launch >8s, health probes can time out.

**Fix** (shipped in S275 Wave 49 ecosystem tightening commit):

1. **Deferred wgpu GPU enumeration** — `query_local_capabilities()` returns
   fast baseline (cpu, memory, orchestration) immediately. wgpu
   `enumerate_adapters()` (1–5s Vulkan driver init) runs in background
   `tokio::spawn`. Full GPU capabilities populate `OnceLock` asynchronously.

2. **Pre-bound JSON-RPC socket** — `prebind_unix_listener()` binds the
   socket before `create_executor()` runs. Health probes can connect during
   initialization. `serve_unix_prebound()` accepts pre-bound listener once
   handler is ready.

3. Default `LocalDirect` deployment skips orchestrator overhead entirely.

**Result**: Socket is listening within ~1s of startup. Health probes no
longer time out during cold launch.

---

## Ecosystem Notes

### S4 Auth Shadow
No action needed from toadStool — membrane consumes through bearDog.
Acknowledged.

### Cross-Gate `discovery.peers`
toadStool does not gate this — Songbird owns `mesh.init` + `discovery.peers`.
We'll consume the capability when it lands.

---

## Verification

- [x] `infra/wateringHole/handoffs/archive/` exists with 28 superseded files
- [x] 9 active handoffs at top level (S267–S275)
- [x] Startup latency fix shipped (S275)
- [x] 0 clippy warnings, 9,149+ lib tests passing

---

Both toadStool items from Wave 49 Mountain audit: **RESOLVED**.
