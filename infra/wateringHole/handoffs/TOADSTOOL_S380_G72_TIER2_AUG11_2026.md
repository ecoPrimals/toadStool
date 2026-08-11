# ToadStool S380 — G72 Dependency Pandemic Tier 2 Quick Wins

**Date**: Aug 11, 2026
**Sprint**: S380
**Wave**: 157i PANDEMIC RESPONDS

## Completed (Tier 2 Quick Wins)

- **`uuid` workspace promotion** — 15 crates migrated from inline `version = "1.7"` to `{ workspace = true }`. Zero version fragmentation.
- **`tracing-subscriber` feature-gated** — `toadstool` crate now has `logging = ["dep:tracing-subscriber"]` (non-default). CLI unaffected (uses own subscriber).
- **`tokio-serde` workspace aligned** — server inline → `{ workspace = true, features = ["json"] }`.

## Blocked Tier 2 Items

| Item | Blocker | Effort post-unblock |
|------|---------|---------------------|
| **wgpu 22→28** | **MSRV 1.92** (current: 1.85). wgpu 28 requires Rust 1.92. | 3–4 days |
| **Gossip injection (0/17)** | **swarmVine socket discovery** (157e fix). Spec exists (`GOSSIP_EVENTS.md`), manifest schema exists, zero production injection code. | 2.5–3 days |
| **axum excision** | Independent but medium effort (BYOB rewrite to UDS JSON-RPC) | 1–2 days |

## Verification

- `cargo check --workspace` — 0 errors
- `cargo test --workspace --lib` — 8,446 passed, 0 failed
