# ToadStool Daemon Mode

> Since S169, the daemon exposes **JSON-RPC 2.0 over Unix sockets only**
> (no HTTP, no REST, no `curl`). See:
>
> - [`docs/reference/PRODUCTION_DEPLOYMENT_GUIDE.md`](../reference/PRODUCTION_DEPLOYMENT_GUIDE.md) — full deployment guide
> - [`docs/reference/SERVER_METHODS.md`](../reference/SERVER_METHODS.md) — all JSON-RPC methods
> - [`CONTEXT.md`](../../CONTEXT.md) — primal role and IPC details

## Starting the Server

The recommended command is `toadstool server` (UniBin standard naming).
`toadstool daemon` is a backward-compatible alias that calls the same code path.

```bash
# Recommended
toadstool server

# With options
toadstool server --port 9090 --register --family-id lab01

# Backward-compatible alias
toadstool daemon
```

The server binds two Unix sockets:

| Socket | Path | Protocol |
|--------|------|----------|
| JSON-RPC | `$XDG_RUNTIME_DIR/biomeos/compute.sock` | JSON-RPC 2.0 (newline-delimited) |
| tarpc | `$XDG_RUNTIME_DIR/biomeos/compute-tarpc.sock` | tarpc binary codec |

If `--port <PORT>` is specified, it also listens on TCP (JSON-RPC, not HTTP).

## Verifying the Server

Use any JSON-RPC 2.0 client over the Unix socket:

```bash
# Health check
echo '{"jsonrpc":"2.0","method":"health.liveness","id":1}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock

# List capabilities (used by biomeOS Neural API for routing)
echo '{"jsonrpc":"2.0","method":"capabilities.list","id":2}' | \
  socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/compute.sock
```

## Stopping the Server

Send `SIGINT` (Ctrl+C) or `SIGTERM` to the process:

```bash
kill -SIGTERM $(pgrep -f "toadstool server")
```

## CLI Flags

See [`docs/reference/SERVER_METHODS.md`](../reference/SERVER_METHODS.md#cli-flags-server-mode)
for the full list of `--register`, `--port`, `--socket`, `--config`,
`--max-workloads`, `--biomeos-socket`, and `--family-id` options.
