# Speed-On Core API v1

API version: `core-api-v1`

IPC protocol version: `speed-on-ipc-v1`

This document defines the stable contract between native frontends and the Rust backend core. It is transport-agnostic: Windows, macOS, and Linux frontends may call it through IPC, local HTTP, FFI, or another adapter later, but the request and response payloads must keep this shape.

## Unified response envelope

Every frontend-facing API returns the same envelope.

### Success

```json
{
  "ok": true,
  "data": {},
  "error": null
}
```

### Failure

```json
{
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
```

The frontend error response intentionally does not expose internal `cause` values. Causes may contain database paths, platform details, or other implementation-specific information. Sanitized diagnostics should go to system logs instead.

## JSON IPC envelope

The minimal IPC adapter wraps Core API payloads in a stable JSON envelope. The envelope is independent from the future transport choice: named pipe, Unix domain socket, local HTTP, stdio, or FFI adapter can all carry the same JSON shape.

### IPC request

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "request-1",
  "command": "search",
  "payload": {
    "query": "term",
    "limit": 5,
    "kinds": ["application"],
    "now_millis": 100
  }
}
```

Fields:

- `protocol_version`: must be `speed-on-ipc-v1`.
- `request_id`: frontend-generated request id. Must not be empty.
- `command`: one of `search`, `recommend`, `record_selection`, `open_resource`.
- `payload`: command-specific Core API request payload.

### IPC response

```json
{
  "protocol_version": "speed-on-ipc-v1",
  "request_id": "request-1",
  "command": "search",
  "response": {
    "ok": true,
    "data": {
      "api_version": "core-api-v1",
      "results": []
    },
    "error": null
  }
}
```

Important behavior:

- `request_id` is copied from request to response.
- Unsupported `protocol_version` returns a structured error.
- Invalid payload returns a structured error.
- IPC dispatcher must not panic on malformed payloads.
- `open_resource` requires a platform `ResourceOpener` adapter. Dispatchers without an opener return `CORE_PLATFORM_UNSUPPORTED`.

## Common resource DTO

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

## Search

Search applications, files, folders, browser URLs, browser page titles, full pinyin aliases, pinyin-initial aliases, and user-history signals.

### Request

```json
{
  "query": "wx",
  "limit": 5,
  "kinds": ["application", "browser_url"],
  "now_millis": 100
}
```

Fields:

- `query`: raw frontend search text. Empty or whitespace-only values are invalid.
- `limit`: maximum result count. Must be greater than zero.
- `kinds`: optional resource type filter. `null` or missing means all resource kinds.
- `now_millis`: caller-provided current timestamp in milliseconds.

### Success response data

```json
{
  "api_version": "core-api-v1",
  "results": [
    {
      "resource": {
        "id": "app-wechat",
        "kind": "application",
        "title": "微信",
        "target": "/apps/wechat",
        "icon_path": "wechat.png"
      },
      "score": 2180,
      "match_kind": "user_history",
      "reason": "previously selected 3 times for a similar query; score 2180"
    }
  ]
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

Important behavior:

- User-history matches rank above normal title/path/browser/pinyin matches.
- The same resource must not appear twice even if it matches through both user history and normal search aliases.
- Search queries are recorded as user operation logs.

## Recommend

Recommend likely next resources without a raw search query.

### Request

```json
{
  "limit": 3,
  "kinds": ["application"],
  "now_millis": 100
}
```

Fields:

- `limit`: maximum result count. Must be greater than zero.
- `kinds`: optional resource type filter. `null` or missing means all resource kinds.
- `now_millis`: caller-provided current timestamp in milliseconds.

### Success response data

```json
{
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
      "reason": "opened 3 times; last opened at 100; score 300"
    }
  ]
}
```

## Record selection

Record which search result the user finally opened.

### Request

```json
{
  "query": "term",
  "selected_resource": {
    "id": "app-terminal",
    "kind": "application",
    "title": "Terminal",
    "target": "/apps/terminal",
    "icon_path": "terminal.png"
  },
  "selected_rank": 1,
  "opened_at_millis": 200
}
```

Fields:

- `query`: the raw search query that produced the selected result.
- `selected_resource`: the resource object returned by `search`.
- `selected_rank`: one-based rank in the result list. Must be greater than zero.
- `opened_at_millis`: timestamp in milliseconds when the resource was opened.

### Success response data

```json
{
  "api_version": "core-api-v1",
  "recorded": true
}
```

Important behavior:

- The selection is stored in user operation logs.
- The query-resource aggregate is updated so future similar searches can prioritize the selected resource.
- The request must not be silently ignored.

## Open resource

Open an application, file, folder, or browser URL through a platform adapter.

### Request

```json
{
  "resource": {
    "id": "app-terminal",
    "kind": "application",
    "title": "Terminal",
    "target": "/apps/terminal",
    "icon_path": "terminal.png"
  },
  "requested_at_millis": 300
}
```

Fields:

- `resource`: the resource object returned by `search` or `recommend`.
- `requested_at_millis`: timestamp in milliseconds when the frontend requested opening the resource.

### Success response data

```json
{
  "api_version": "core-api-v1",
  "opened": true,
  "resource_id": "app-terminal",
  "kind": "application",
  "target": "/apps/terminal",
  "opened_at_millis": 300
}
```

Important behavior:

- Core API does not directly execute OS commands.
- Opening must go through a `ResourceOpener` platform adapter.
- Dispatchers without a platform opener must return `CORE_PLATFORM_UNSUPPORTED`.
- Platform adapters must validate targets and apply permission checks before invoking OS APIs.

## Privacy boundary

User operation logs may contain search text, file names, folder paths, browser URLs, and browser page titles. System logs are separate and must only contain sanitized runtime diagnostics.

Frontend adapters must not write raw query text, full browser URLs, tokens, passwords, or private file contents into system logs.
