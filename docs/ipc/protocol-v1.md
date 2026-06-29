# Speed-On IPC Protocol v1

Protocol version: `speed-on-ipc-v1`

Core API version: `core-api-v1`

This document is the authoritative IPC communication protocol reference for Speed-On v1. It describes every current IPC command, the correct request shape, successful responses, common error responses, and the reason each error can happen.

The protocol is transport-agnostic. The same JSON envelope can be carried over stdio JSON Lines, Named Pipe, Unix Domain Socket, local HTTP, or future FFI adapters. The current runnable transport is `speed-on-ipc-stdio`, which sends one JSON request per stdin line and receives one JSON response per stdout line.

## 1. Common IPC envelope

### 1.1 Request envelope

Every request must use this shape:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "request-1",
  "command": "search",
  "payload": {}
}
```

Fields:

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `protocol_version` | string | yes | Must be `speed-on-ipc-v1`. |
| `request_id` | string | yes | Frontend-generated id. Must not be empty. |
| `command` | string | yes | One of the supported command names. |
| `payload` | object | yes | Command-specific request payload. |

Supported commands:

- `search`
- `recommend`
- `record_selection`
- `open_resource`
- `refresh_applications`

### 1.2 Response envelope

Every valid request receives this shape:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "request-1",
  "command": "search",
  "response": {
    "ok": true,
    "data": {},
    "error": null
  }
}
```

Response fields:

| Field | Type | Description |
| --- | --- | --- |
| `protocol_version` | string | Always `speed-on-ipc-v1` for v1 responses. |
| `request_id` | string or null | Copied from request if the request envelope was valid. |
| `command` | string or null | Copied from request if the request envelope was valid. |
| `response.ok` | boolean | `true` on success, `false` on failure. |
| `response.data` | object or null | Command-specific response body when successful. |
| `response.error` | object or null | Structured error when failed. |

### 1.3 Error object

```json
{
  "error_code": "CORE_INVALID_ARGUMENT",
  "message": "invalid IPC payload",
  "module": "ipc::JsonIpcDispatcher::search",
  "recoverable": true,
  "suggestion": null,
  "trace_id": null
}
```

Error fields:

| Field | Type | Description |
| --- | --- | --- |
| `error_code` | string | Stable error code for frontend branching. |
| `message` | string | Human-readable summary. |
| `module` | string | Module that generated the error. |
| `recoverable` | boolean | Whether the caller may fix input or retry. |
| `suggestion` | string or null | Optional remediation hint. |
| `trace_id` | string or null | Optional trace id for logs. |

Internal `cause` values are intentionally not exposed through IPC responses because they may contain database paths, platform details, or private file paths.

## 2. Common DTOs

### 2.1 Resource DTO

```json
{
  "id": "app-terminal",
  "kind": "application",
  "title": "Terminal",
  "target": "/apps/terminal",
  "icon_path": "terminal.png"
}
```

Allowed `kind` values:

- `application`
- `file`
- `folder`
- `browser_url`

## 3. Command: search

Search indexed resources by title, target, browser title, pinyin aliases, pinyin initials, and user-history signals.

### 3.1 Correct request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-1",
  "command": "search",
  "payload": {
    "query": "term",
    "limit": 5,
    "kinds": ["application"],
    "now_millis": 1000
  }
}
```

Payload fields:

| Field | Type | Required | Rule |
| --- | --- | --- | --- |
| `query` | string | yes | Must not be empty after normalization. |
| `limit` | number | yes | Must be greater than `0`. |
| `kinds` | array or null | no | Optional resource kind filter. |
| `now_millis` | number | yes | Caller timestamp in milliseconds. |

### 3.2 Success response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-1",
  "command": "search",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "results": [
        {
          "resource": {
            "id": "app-terminal",
            "kind": "application",
            "title": "Terminal",
            "target": "/apps/terminal",
            "icon_path": "terminal.png"
          },
          "score": 700,
          "match_kind": "title",
          "reason": "matched title; score 700"
        }
      ]
    },
    "error": null
  }
}
```

Allowed `match_kind` values:

- `user_history`
- `title`
- `target`
- `browser_title`
- `pinyin_full`
- `pinyin_initials`
- `custom_alias`

### 3.3 Error example: empty query

Wrong request:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-empty",
  "command": "search",
  "payload": {
    "query": "   ",
    "limit": 5,
    "kinds": ["application"],
    "now_millis": 1000
  }
}
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-empty",
  "command": "search",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_INVALID_ARGUMENT",
      "message": "search query must not be empty",
      "module": "search::SearchService",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: `query` becomes empty after normalization. The frontend should not send empty search requests.

### 3.4 Error example: invalid payload type

Wrong request:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-invalid-limit",
  "command": "search",
  "payload": {
    "query": "term",
    "limit": "five",
    "kinds": ["application"],
    "now_millis": 1000
  }
}
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "search-invalid-limit",
  "command": "search",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_INVALID_ARGUMENT",
      "message": "invalid IPC payload",
      "module": "ipc::JsonIpcDispatcher::search",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: `limit` must be a number, not a string.

## 4. Command: recommend

Return likely next resources without a raw search query.

### 4.1 Correct request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "recommend-1",
  "command": "recommend",
  "payload": {
    "limit": 5,
    "kinds": ["application"],
    "now_millis": 2000
  }
}
```

Payload fields:

| Field | Type | Required | Rule |
| --- | --- | --- | --- |
| `limit` | number | yes | Must be greater than `0`. |
| `kinds` | array or null | no | Optional resource kind filter. |
| `now_millis` | number | yes | Caller timestamp in milliseconds. |

### 4.2 Success response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "recommend-1",
  "command": "recommend",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "results": [
        {
          "resource": {
            "id": "app-terminal",
            "kind": "application",
            "title": "Terminal",
            "target": "/apps/terminal",
            "icon_path": "terminal.png"
          },
          "score": 300,
          "reason": "opened 3 times; last opened at 1000; score 300"
        }
      ]
    },
    "error": null
  }
}
```

### 4.3 Error example: zero limit

Wrong request:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "recommend-zero",
  "command": "recommend",
  "payload": {
    "limit": 0,
    "kinds": ["application"],
    "now_millis": 2000
  }
}
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "recommend-zero",
  "command": "recommend",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_INVALID_ARGUMENT",
      "message": "recommendation limit must be greater than zero",
      "module": "service::RecommendationService",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: `limit` must be at least `1`.

## 5. Command: record_selection

Record which search result the user finally opened.

### 5.1 Correct request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "selection-1",
  "command": "record_selection",
  "payload": {
    "query": "term",
    "selected_resource": {
      "id": "app-terminal",
      "kind": "application",
      "title": "Terminal",
      "target": "/apps/terminal",
      "icon_path": "terminal.png"
    },
    "selected_rank": 1,
    "opened_at_millis": 3000
  }
}
```

Payload fields:

| Field | Type | Required | Rule |
| --- | --- | --- | --- |
| `query` | string | yes | Must not be empty after normalization. |
| `selected_resource` | object | yes | Resource returned by `search`. |
| `selected_rank` | number | yes | One-based rank. Must be greater than `0`. |
| `opened_at_millis` | number | yes | Timestamp when the resource was opened. |

### 5.2 Success response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "selection-1",
  "command": "record_selection",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "recorded": true
    },
    "error": null
  }
}
```

### 5.3 Error example: selected_rank is zero

Wrong request:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "selection-zero-rank",
  "command": "record_selection",
  "payload": {
    "query": "term",
    "selected_resource": {
      "id": "app-terminal",
      "kind": "application",
      "title": "Terminal",
      "target": "/apps/terminal",
      "icon_path": "terminal.png"
    },
    "selected_rank": 0,
    "opened_at_millis": 3000
  }
}
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "selection-zero-rank",
  "command": "record_selection",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_INVALID_ARGUMENT",
      "message": "selected rank must be one-based and greater than zero",
      "module": "search::SearchService",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: `selected_rank` is one-based. The first item is `1`, not `0`.

## 6. Command: open_resource

Open an application, file, folder, or browser URL through a platform `ResourceOpener` adapter.

### 6.1 Correct request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "open-1",
  "command": "open_resource",
  "payload": {
    "resource": {
      "id": "app-terminal",
      "kind": "application",
      "title": "Terminal",
      "target": "/apps/terminal",
      "icon_path": "terminal.png"
    },
    "requested_at_millis": 4000
  }
}
```

Payload fields:

| Field | Type | Required | Rule |
| --- | --- | --- | --- |
| `resource` | object | yes | Resource returned by `search` or `recommend`. |
| `requested_at_millis` | number | yes | Timestamp when the frontend requested opening. |

### 6.2 Success response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "open-1",
  "command": "open_resource",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "opened": true,
      "activity_recorded": true,
      "resource_id": "app-terminal",
      "kind": "application",
      "target": "/apps/terminal",
      "opened_at_millis": 4000
    },
    "error": null
  }
}
```

### 6.3 Error example: opener disabled

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "open-1",
  "command": "open_resource",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_PLATFORM_UNSUPPORTED",
      "message": "open_resource requires a platform ResourceOpener adapter",
      "module": "ipc::JsonIpcDispatcher::open_resource",
      "recoverable": false,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: the dispatcher was created without a `ResourceOpener`. In stdio, launch with `--enable-command-opener` to enable the first command-based opener.

### 6.4 Error example: unsafe browser URL scheme

Wrong request:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "open-bad-url",
  "command": "open_resource",
  "payload": {
    "resource": {
      "id": "url-bad",
      "kind": "browser_url",
      "title": "Bad URL",
      "target": "javascript:alert(1)",
      "icon_path": null
    },
    "requested_at_millis": 4000
  }
}
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "open-bad-url",
  "command": "open_resource",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_INVALID_ARGUMENT",
      "message": "browser URL target must use http, https, or file scheme",
      "module": "platform::OpenTargetValidator",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: the command opener only allows `http://`, `https://`, and `file://` for `browser_url` resources.

## 7. Command: refresh_applications

Scan installed desktop applications through a platform `InstalledApplicationScanner` adapter, write resources to SQLite, and generate search aliases.

### 7.1 Correct request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "refresh-1",
  "command": "refresh_applications",
  "payload": {
    "requested_at_millis": 5000
  }
}
```

Payload fields:

| Field | Type | Required | Rule |
| --- | --- | --- | --- |
| `requested_at_millis` | number | yes | Timestamp when the frontend requested refreshing the application index. |

### 7.2 Success response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "refresh-1",
  "command": "refresh_applications",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "scanned_count": 12,
      "alias_count": 24
    },
    "error": null
  }
}
```

### 7.3 Error example: scanner disabled

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "refresh-1",
  "command": "refresh_applications",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_PLATFORM_UNSUPPORTED",
      "message": "refresh_applications requires a platform InstalledApplicationScanner adapter",
      "module": "ipc::JsonIpcDispatcher::refresh_applications",
      "recoverable": false,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: the dispatcher was created without an `InstalledApplicationScanner`. In stdio, launch with `--enable-application-scan` to enable the first platform scanner.

### 7.4 Error example: scan root cannot be read

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "refresh-root-error",
  "command": "refresh_applications",
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "CORE_PLATFORM_UNSUPPORTED",
      "message": "failed to read application scan root: /restricted/apps",
      "module": "platform::ApplicationScanner",
      "recoverable": false,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: the configured scan root could not be read because it is missing, inaccessible, or blocked by OS permissions. Missing default roots are skipped, but unreadable existing roots return an error.

## 8. Transport-level malformed request

If the input line is not valid JSON or cannot be decoded as an IPC request envelope, stdio transport returns a transport-level error. Because `request_id` and `command` may be unavailable, they are `null`.

Wrong input line:

```text
not-json
```

Failure response:

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": null,
  "command": null,
  "response": {
    "ok": false,
    "data": null,
    "error": {
      "error_code": "IPC_STDIO_MALFORMED_REQUEST",
      "message": "invalid IPC request envelope",
      "module": "ipc_stdio::run_json_lines_transport",
      "recoverable": true,
      "suggestion": null,
      "trace_id": null
    }
  }
}
```

Reason: the request cannot be parsed as an `IpcRequest`, so normal command dispatch cannot run.

## 9. Stdio startup examples

Default, no platform actions:

```bash
speed-on-ipc-stdio --db ./speed-on.db
```

Enable application scanning:

```bash
speed-on-ipc-stdio --db ./speed-on.db --enable-application-scan
```

Enable opening resources:

```bash
speed-on-ipc-stdio --db ./speed-on.db --enable-command-opener
```

Enable both platform capabilities:

```bash
speed-on-ipc-stdio --db ./speed-on.db --enable-application-scan --enable-command-opener
```

If `--db` is missing and `SPEED_ON_DB` is not set, startup fails on stderr with `IPC_STDIO_INVALID_INPUT`.
