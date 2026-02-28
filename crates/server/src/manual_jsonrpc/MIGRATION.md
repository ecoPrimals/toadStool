# Migration: manual_jsonrpc → pure_jsonrpc

## Status

`manual_jsonrpc` is **fully deprecated** as of 2.2.0. All capabilities have been ported to
`pure_jsonrpc`. The canonical JSON-RPC implementation is `pure_jsonrpc`, which provides:

- **SemanticMethodRegistry** — semantic routing (e.g. `runtime.workload.submit` → `submit_workload`)
- **Proper error types** — `JsonRpcError` with constructors (`invalid_params`, `method_not_found`, etc.)
- **Cow<'static, str>** — zero-copy JSON-RPC version strings
- **Unified JsonRpcResponse** — single type for success/error (no separate JsonRpcErrorResponse)
- **Unix socket + TCP serving** — `pure_jsonrpc::connection::serve_unix`, `serve_tcp`
- **HTTP/JSON-RPC hybrid** — connection layer supports both raw JSON and HTTP-wrapped requests

## Capability parity (all ported)

| Capability                    | manual_jsonrpc | pure_jsonrpc |
|-------------------------------|----------------|--------------|
| Unix socket serving           | ✓              | ✓            |
| TCP serving                   | ✓              | ✓            |
| HTTP/JSON-RPC hybrid          | ✓              | ✓            |
| `toadstool.*`, `compute.*`    | ✓              | ✓            |
| `resources.estimate/validate/suggest` | ✓      | ✓            |
| `gpu.info`, `gpu.memory`      | ✓              | ✓            |
| `ollama.list_models/inference/load/unload` | ✓ | ✓       |
| `gate.update/remove/list/route` | ✓            | ✓            |
| `compute.health/version/capabilities/discover_capabilities` | ✓ | ✓ |

## Migration path (for callers)

1. **Unibin** (primary caller): Now uses `pure_jsonrpc::JsonRpcHandler` with
   `pure_jsonrpc::connection::serve_unix` and `serve_tcp`.

2. **New code**: Use `pure_jsonrpc::JsonRpcHandler` for request handling. If you need serving,
   build a thin connection layer that:
   - Accepts connections (Unix/TCP)
   - Parses `JsonRpcRequest` (use `pure_jsonrpc::types::JsonRpcRequest`)
   - Calls `JsonRpcHandler::handle_request(&request)`
   - Serializes `JsonRpcResponse` (use `pure_jsonrpc::types::JsonRpcResponse`)

3. **Shared types**: Prefer `pure_jsonrpc::JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`
   when adding new JSON-RPC types.

## Do not delete

The `manual_jsonrpc` module must remain for backward compatibility and tests. Remaining references:

- `crates/server/tests/manual_jsonrpc_tests.rs` (if any)
- Manual JSON-RPC connection tests in `manual_jsonrpc/connection/`
