# Speed-On Core API v1

API version: `core-api-v1`

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

## Common resource DTO

```json
{
  "id": "app-terminal",
  "kind": "application",
  "title": "Terminal",
  "target": "/System/Applications/Utilities/Terminal.app",
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
        "target": "/Applications/WeChat.app",
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
        "target": "/System/Applications/Utilities/Terminal.app",
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
    "target": "/System/Applications/Utilities/Terminal.app",
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

## Privacy boundary

User operation logs may contain search text, file names, folder paths, browser URLs, and browser page titles. System logs are separate and must only contain sanitized runtime diagnostics.

Frontend adapters must not write raw query text, full browser URLs, tokens, passwords, or private file contents into system logs.
