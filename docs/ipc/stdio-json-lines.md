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

## Enabling real command opener

By default, `open_resource` is disabled in the stdio binary. To enable the command-based opener, pass the explicit flag:

```bash
speed-on-ipc-stdio --db ./speed-on.db --enable-command-opener
```

This mode wires `JsonIpcDispatcherWithOpener` to `speed_on_platform::CommandResourceOpener`. The opener validates targets and browser URL schemes before invoking platform commands. The first command-based strategy is:

- macOS: `open <target>`
- Linux: `xdg-open <target>`
- Windows: `explorer <target>`

The command is executed through structured process arguments, not shell string concatenation.

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
- `open_resource`

Command payloads are the same as `docs/api/core-api-v1.md`.

## Open resource behavior

Without `--enable-command-opener`, an `open_resource` request returns `CORE_PLATFORM_UNSUPPORTED` rather than pretending to open the resource.

```json
{"protocol_version":"speed-on-ipc-v1","request_id":"open-1","command":"open_resource","response":{"ok":false,"data":null,"error":{"error_code":"CORE_PLATFORM_UNSUPPORTED","message":"open_resource requires a platform ResourceOpener adapter","module":"ipc::JsonIpcDispatcher::open_resource","recoverable":false,"suggestion":null,"trace_id":null}}}
```

With `--enable-command-opener`, successful `open_resource` responses include `activity_recorded: true` after Core writes the open action into activity/usage stats.

```json
{"protocol_version":"speed-on-ipc-v1","request_id":"open-1","command":"open_resource","response":{"ok":true,"data":{"api_version":"core-api-v1","opened":true,"activity_recorded":true,"resource_id":"app-terminal","kind":"application","target":"/apps/terminal","opened_at_millis":300},"error":null}}
```

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
