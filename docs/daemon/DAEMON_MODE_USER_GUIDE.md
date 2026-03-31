# ToadStool Daemon Mode

> **Fossilized (S170).** The HTTP-based daemon guide has been archived to
> `ecoPrimals/infra/wateringHole/fossilRecord/TOADSTOOL_DAEMON_MODE_USER_GUIDE_S169_DEPRECATED.md`.
>
> Since S169, the daemon exposes **JSON-RPC 2.0 over Unix sockets only**
> (no HTTP, no REST, no `curl`). See:
>
> - [`docs/reference/SERVER_METHODS.md`](../reference/SERVER_METHODS.md) — all JSON-RPC methods
> - [`CONTEXT.md`](../../CONTEXT.md) — primal role and IPC details
> - `$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock` — daemon socket path

## Starting the Daemon

```bash
toadstool daemon start
```

The daemon binds a Unix socket at `$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock`.
If `--port <PORT>` is specified, it also listens on TCP (JSON-RPC, not HTTP).

## Interacting with the Daemon

Use any JSON-RPC 2.0 client over the Unix socket:

```bash
echo '{"jsonrpc":"2.0","method":"health.check","id":1}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock
```

## Stopping the Daemon

```bash
toadstool daemon stop
# or send SIGINT/SIGTERM to the process
```
