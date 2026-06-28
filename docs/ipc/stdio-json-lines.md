# Stdio JSON Lines IPC Transport

Binary: `speed-on-ipc-stdio`

This is the first runnable IPC transport for Speed-On. It is intentionally simple and cross-platform: the native frontend can start the Rust Core process as a child process, write one JSON request per line to stdin, and read one JSON response per line from stdout.

This transport carries the existing `speed-on-ipc-v1` envelope. It is not HTTP and it is not QUIC.

## Start command

Use an explicit database path:

```bash
speed-on-ipc-stdio --db ./speed-on.db
```

Or use the environment variable:

```bash
SPEED_ON_DB=./speed-on.db speed-on-ipc-stdio
```

If no database path is provided, startup fails with a structured stderr error instead of silently creating a database in an unknown location.

## Request format

Every stdin line must contain one complete `IpcRequest` JSON object.

```json
{"protocol_version":"speed-on-ipc-v1","request_id":"request-1","command":"search","payload":{"query":"term","limit":5,"kinds":["application"],"now_millis":100}}
```

## Response format

Every stdout line contains one complete JSON response.

```json
{"protocol_version":"speed-on-ipc-v1","request_id":"request-1","command":"search","response":{"ok":true,"data":{"api_version":"core-api-v1","results":[]},"error":null}}
```

## Supported commands

- `search`
- `recommend`
- `record_selection`

Command payloads are the same as `docs/api/core-api-v1.md`.

## Malformed request behavior

If the JSON envelope cannot be decoded at all, the transport returns a transport-level error because `request_id` and `command` may be unavailable.

```json
{"protocol_version":"speed-on-ipc-v1","request_id":null,"command":null,"response":{"ok":false,"data":null,"error":{"error_code":"IPC_STDIO_MALFORMED_REQUEST","message":"invalid IPC request envelope","module":"ipc_stdio::run_json_lines_transport","recoverable":true,"suggestion":null,"trace_id":null}}}
```

If the envelope is valid but the payload is invalid, the normal Core IPC response is used and the original `request_id` is preserved.

## Intended use

This transport is for:

- early native frontend integration;
- debugging;
- script-based testing;
- cross-platform fallback while Named Pipe / Unix Socket transports are not implemented.

It does not prevent adding more native transports later.
