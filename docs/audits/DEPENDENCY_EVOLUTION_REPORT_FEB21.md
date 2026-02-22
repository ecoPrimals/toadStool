# Dependency Evolution Report — February 21, 2026

**Scope**: Root `Cargo.toml` and crate-level dependencies across the toadStool workspace  
**Goal**: Identify dependencies that can be evolved to more modern, pure Rust alternatives

---

## 1. Changes Made

### thiserror 1.0 → 2.0 ✅ COMPLETED

- **Root**: `thiserror = "1.0"` → `thiserror = "2.0"`
- **Crates updated**: 26 crates (all workspace members using thiserror) unified to `{ workspace = true }`
- **Rationale**: thiserror 2.0 is the maintained line; MSRV 1.68 vs workspace `rust-version = 1.75.0` — compatible
- **Build status**: Verified — full workspace compiles successfully

**Breaking changes (1.x → 2.x)**: Raw identifier syntax in format strings (`{r#type}` → `{type}`). Audit found no `r#` usage in error derives; migration was safe.

---

## 2. Future Evolution Targets

### async-trait — KEEP (Documented)

| Aspect | Finding |
|--------|---------|
| **MSRV** | Workspace uses `rust-version = "1.75.0"`; native `async fn` in traits available since Rust 1.75 |
| **Usage** | ~65+ files use `#[async_trait]`; key traits: `ComputeExecutor`, `TensorStorage`, `WorkloadExecutor`, `ByobExecutor`, etc. |
| **dyn usage** | Heavy: `dyn ComputeExecutor`, `dyn TensorStorage`, `Arc<dyn WorkloadExecutor + Send + Sync>` |
| **Conclusion** | **Keep async-trait**. Native async fn in traits still requires boxing for `dyn Trait`—the returned `Future` must be `Sized`. Dropping async-trait would require manually changing every async method to `-> Pin<Box<dyn Future<Output = _> + Send>>`, which is more verbose than the proc-macro. No meaningful gain from migration. |

### chrono → time — FUTURE (Large Migration)

| Aspect | Finding |
|--------|---------|
| **Current** | `chrono = "0.4"` (pure Rust) |
| **Alternative** | `time` crate — pure Rust, more modern API |
| **Scope** | Used in container, display, cross-platform showcase, homomorphic showcase, ml-inference showcase, etc. |
| **Effort** | High — different API surface; requires date/time parsing and formatting changes across many modules |
| **Priority** | P3 — chrono is already pure Rust; migration improves API quality, not purity |

### serde / serde_json — NO ACTION

- **serde** `1.0`: Standard choice; `1.0` resolves to latest 1.x (e.g. 1.0.228)
- **serde_json** `1.0`: Same; no replacement needed
- Both are pure Rust and de facto standard

### tokio — NO ACTION

- **Version**: `1.35` in workspace; transitive resolution to `1.49` within 1.x
- Pure Rust, industry standard; no evolution needed

### std Library Replacements — ALREADY DONE

- **once_cell**: Evolved to `std::sync::LazyLock` (Rust 1.80+); workspace at 1.75 — consider MSRV bump when feasible
- **dirs**: Replaced with `etcetera` (pure Rust directory discovery) per root Cargo.toml

---

## 3. Dependencies with C / FFI Exposure

| Crate | Location | Reason | Mitigation |
|-------|----------|--------|------------|
| **libc** | akida-driver | VFIO ioctls (kernel-specific; not in rustix) | Required for Akida NPU; documented as minimal FFI |
| **cc** | runtime/specialty, runtime/edge | Build-time C compilation (optional bindgen) | Optional; only when `native-bindings` enabled |
| **nix** | akida-detection (showcase) | Unix syscalls | Showcase-only; evaluate removal if unused |
| **pyo3** | runtime/python | Python FFI | Inherent Python integration; no pure Rust alternative |
| **bollard** | runtime/container | Docker API (Unix socket + HTTP) | May pull in HTTP/TLS stack; used for Docker integration |
| **sysinfo** | workspace | System monitoring | Pure Rust per docs; no C deps in core |
| **wgpu** | barracuda, runtime/gpu, etc. | GPU backends (Vulkan, Metal, DX12) | System libraries; `renderdoc` feature disabled for pure build |
| **gbm** | runtime/display (optional) | Mesa GBM buffer management | C library; optional; only for zero-copy GPU rendering |
| **tfhe** | homomorphic-computing (showcase) | Homomorphic encryption | External FHE library; showcase-only |

### Intentionally Removed (Pure Rust Evolution)

- **ring** — removed (jsonrpsee, sqlx paths)
- **protobuf** — containerd-client, tonic removed
- **reqwest** — replaced with Unix JSON-RPC

---

## 4. Summary

| Action | Status |
|--------|--------|
| thiserror 1.0 → 2.0 | ✅ Done; workspace builds |
| async-trait removal | ❌ Not recommended; keep |
| chrono → time | 📋 Future (P3) |
| serde/tokio version bump | ℹ️ No action; already current |
| C dependency audit | 📋 Documented above |

---

## 5. Recommendations

1. **Keep async-trait** — Current architecture (dyn dispatch with async traits) benefits from it; native async fn doesn’t remove the boxing requirement.
2. **Consider MSRV 1.80+** — Would allow `std::sync::LazyLock` to replace any remaining `once_cell` usage.
3. **Chrono → time** — Plan as a dedicated migration; low urgency.
4. **C dependencies** — libc (akida-driver), pyo3, bollard, gbm are either required or optional; document clearly for teams prioritizing pure Rust builds.
