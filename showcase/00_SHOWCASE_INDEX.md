# ToadStool Showcase Index

**Progressive Learning Path: From Local Compute to Inter-Primal Orchestration**

---

## Learning Path

```
Level 00: Local Primal           "What can toadStool do on its own?"
    |
    v
Level 01: Shader Pipeline        "How does shader compilation work?"
    |
    v
Level 02: Compute Patterns       "How do toadStool, barraCuda, and coralReef work together?"
    |
    v
Level 03: Ecosystem Integration  "How does compute fit into the full ecosystem?"
```

---

## Prerequisites by Level

| Level | Rust | toadStool Server | coralReef | barraCuda | songbird/beardog/nestgate |
|-------|------|------------------|-----------|-----------|---------------------------|
| 00    | Yes  | No               | No        | No        | No                        |
| 01    | Yes  | Yes              | Optional  | No        | No                        |
| 02    | Yes  | Yes              | Optional  | Optional  | No                        |
| 03    | Yes  | Yes              | Optional  | Optional  | Optional                  |

All demos gracefully degrade when optional services are unavailable, showing
what the interaction would look like and which capabilities were discovered.

---

## Demo Index

### 00-local-primal/

| # | Demo | Description | Duration | Difficulty |
|---|------|-------------|----------|------------|
| 01 | hello-compute | Health, version, capability enumeration | 30s | Beginner |
| 02 | hardware-discovery | CPU/GPU/NPU substrate probing | 60s | Beginner |
| 03 | workload-lifecycle | Submit, poll status, get result, cancel | 60s | Intermediate |
| 04 | resource-management | Estimation, validation, optimization | 30s | Intermediate |
| 05 | gpu-job-queue | GPU dispatch and queue management | 60s | Intermediate |

### 01-shader-pipeline/

| # | Demo | Description | Duration | Difficulty |
|---|------|-------------|----------|------------|
| 01 | naga-fallback | WGSL -> SPIR-V via naga (standalone) | 30s | Beginner |
| 02 | coralreef-compile | Shader compilation via coralReef | 60s | Intermediate |
| 03 | compile-status | Async compilation status polling | 30s | Intermediate |

### 02-compute-patterns/

| # | Demo | Description | Duration | Difficulty |
|---|------|-------------|----------|------------|
| 01 | capability-discovery | Runtime discovery of compute primals | 30s | Intermediate |
| 02 | science-dispatch | GPU compute job submission | 60s | Advanced |
| 03 | deploy-graph | Capability-based routing to barraCuda | 60s | Advanced |
| 04 | shader-to-gpu | Full compile -> dispatch -> execute triangle | 120s | Advanced |

### 03-ecosystem-integration/

| # | Demo | Description | Duration | Difficulty |
|---|------|-------------|----------|------------|
| 01 | coordination-registration | Register capabilities for cross-tower discovery | 60s | Advanced |
| 02 | security-secured-compute | Signed workload submission | 60s | Advanced |
| 03 | storage-artifact-pipeline | Store/retrieve compute artifacts | 60s | Advanced |

---

## Key Concepts

- **Capability-based discovery**: Primals find each other by capability (e.g. "compute"),
  not by name. No hardcoded addresses.
- **Compute triangle**: toadStool (WHERE to run), barraCuda (WHAT to compute),
  coralReef (HOW to compile shaders).
- **Graceful degradation**: Every demo works standalone and shows what *would* happen
  when optional services are present.
- **JSON-RPC 2.0**: All inter-primal communication uses JSON-RPC over Unix sockets.
  No REST, no gRPC, no embedded code.
