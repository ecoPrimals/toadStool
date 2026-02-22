# Migration: manual_jsonrpc → pure_jsonrpc

## Status

`manual_jsonrpc` is **deprecated** as of 2.2.0. The canonical JSON-RPC implementation is
`pure_jsonrpc`, which provides:

- **SemanticMethodRegistry** — semantic routing (e.g. `runtime.workload.submit` → `submit_workload`)
- **Proper error types** — `JsonRpcError` with constructors (`invalid_params`, `method_not_found`, etc.)
- **Cow<'static, str>** — zero-copy JSON-RPC version strings
- **Unified JsonRpcResponse** — single type for success/error (no separate JsonRpcErrorResponse)

## What manual_jsonrpc has that pure_jsonrpc does not (yet)

| Capability                    | manual_jsonrpc | pure_jsonrpc |
|-------------------------------|----------------|--------------|
| Unix socket serving           | ✓              | ✗            |
| TCP serving                   | ✓              | ✗            |
| HTTP/JSON-RPC hybrid          | ✓              | ✗            |
| `toadstool.*`, `compute.*`    | ✓              | ✓            |
| `resources.estimate/validate/suggest` | ✓      | ✗            |
| `gpu.info`, `gpu.memory`      | ✓              | ✗            |
| `ollama.list_models/inference/load/unload` | ✓ | ✗       |
| `gate.update/remove/list/route` | ✓            | ✗            |
| `compute.health/version/capabilities/discover_capabilities` | ✓ | ✗ |

## Migration path (for callers)

1. **Unibin** (primary caller): Continue using `ManualJsonRpcServer` until `pure_jsonrpc` gains:
   - Connection/serving layer (Unix socket, optional TCP)
   - Port of resources, gpu, ollama, gate handlers

2. **New code**: Use `pure_jsonrpc::JsonRpcHandler` for request handling. If you need serving,
   build a thin connection layer that:
   - Accepts connections (Unix/TCP)
   - Parses `JsonRpcRequest` (use `pure_jsonrpc::types::JsonRpcRequest`)
   - Calls `JsonRpcHandler::handle_request(&request)`
   - Serializes `JsonRpcResponse` (use `pure_jsonrpc::types::JsonRpcResponse`)

3. **Shared types**: Prefer `pure_jsonrpc::JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`
   when adding new JSON-RPC types.

## Do not delete

The `manual_jsonrpc` module must remain until all callers migrate. Current callers:

- `crates/server/src/unibin/` (mod.rs, execution.rs)
- `crates/server/tests/manual_jsonrpc_tests.rs`
