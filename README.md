# Speed-On

Speed-On is a PC desktop productivity core designed to predict what a user is likely to open next.

The project currently focuses on the Rust backend core. Native frontends for Windows, macOS, and Linux are intentionally postponed until the backend contracts are stable.

## Supported platforms

Target operating systems:

- Windows with GUI desktop environment
- macOS
- Linux desktop distributions with GUI environments

Target CPU architectures:

- x86_64
- ARM64 / aarch64

## Backend responsibilities

The Rust core is responsible for:

1. Scanning installed desktop applications during installation or first run.
2. Extracting application launch paths and icon metadata.
3. Persisting normalized resources into SQLite.
4. Building query-friendly indexes for recommendations and search.
5. Listening for recent application, file, folder, and browser-history activity through platform adapters.
6. Recording open counts and open timestamps.
7. Searching applications, files, folders, browser URLs, browser page titles, pinyin aliases, and pinyin-initial aliases from frontend queries.
8. Recording user operation logs for frontend search input and the final opened result.
9. Recording sanitized system runtime logs for errors and diagnostics.
10. Producing ranked recommendations and search results for the native frontend based on the requested result count and resource type filters.
11. Providing a stable Core API v1 contract for future native frontends and IPC adapters.
12. Providing a minimal JSON IPC v1 envelope and dispatcher that can be carried by a future platform transport.
13. Providing a runnable stdio JSON Lines IPC transport for early cross-platform frontend integration and debugging.
14. Providing an `open_resource` API / IPC contract behind a platform `ResourceOpener` boundary.
15. Providing a first command-based `ResourceOpener` adapter with shared target validation and URL scheme checks.
16. Recording successful `open_resource` actions into activity/usage stats so future recommendations learn from opened resources.
17. Allowing stdio IPC to opt into real command opening only through the explicit `--enable-command-opener` flag.
18. Providing a first installed-application scanner adapter and `refresh_applications` API / IPC command.
19. Allowing stdio IPC to opt into real application scanning only through the explicit `--enable-application-scan` flag.

## Architecture rules

The backend follows the Codex stability and anti-corruption development skill used for this repository:

- High modularity and low coupling.
- Platform-specific behavior must live behind adapter/provider/gateway boundaries.
- Domain logic must not depend on Windows, macOS, Linux, SQLite, or frontend implementation details.
- Important behavior should be locked by tests, not only documented.
- Critical failure paths must return structured errors and must not fail silently.
- User operation logs and system logs must stay separated because user queries, file paths, and browser URLs are sensitive data.
- Frontend-facing contracts must use stable DTOs instead of binding directly to internal domain/storage types.
- IPC envelopes must stay transport-agnostic until a concrete platform transport is selected.
- Concrete transports must remain thin adapters and must not duplicate Core search/recommend/selection logic.
- Opening resources must go through a platform `ResourceOpener`; Core API and IPC must not directly invoke OS commands.
- Resource openers must validate targets and URL schemes before invoking platform commands.
- Application scanning must go through a platform `InstalledApplicationScanner`; Core API and IPC must not scan OS directories directly.
- Real command opening and real application scanning must be opt-in for stdio IPC and must not be silently enabled by default.

## Current implementation stage

Stage 10 completes the first application scanning path behind an explicit opt-in flag:

- Rust workspace.
- Domain models.
- Repository and platform abstraction traits.
- SQLite schema contract v2.
- SQLite migration runner using `PRAGMA user_version`.
- SQLite-backed resource repository, search index repository, user operation log repository, and system log sink.
- Recommendation service.
- Search service with title, target, browser title, pinyin, pinyin-initial, and user-history ranking support.
- User operation log and sanitized system log models.
- Core API v1 DTOs and response envelope for search, recommend, record_selection, open_resource, and refresh_applications.
- JSON IPC v1 envelope and dispatcher for search, recommend, record_selection, open_resource, and refresh_applications.
- `speed-on-ipc-stdio` binary that reads one IPC request JSON per stdin line and writes one IPC response JSON per stdout line.
- Optional `speed-on-ipc-stdio --enable-command-opener` mode for real command-based resource opening.
- Optional `speed-on-ipc-stdio --enable-application-scan` mode for real installed application scanning.
- `speed_on_platform` crate with target validation, URL scheme checks, platform command planning, injectable command runner, and installed application scanner.
- TDD tests for recommendation behavior, search behavior, logging behavior, schema expectations, SQLite persistence, API JSON contracts, IPC JSON contracts, open_resource contracts, refresh_applications contracts, stdio transport behavior, command opener opt-in, application scan opt-in, platform opener validation/planning, and platform application scanning.

OS log listeners, browser-history readers, pinyin alias builders, native Named Pipe / Unix Socket transports, direct Windows/macOS/Linux opener implementations, and native frontend bindings will be added in later stages.

## API and IPC documentation

- `docs/api/core-api-v1.md`: frontend-facing Core API and JSON IPC envelope.
- `docs/ipc/stdio-json-lines.md`: runnable stdio JSON Lines IPC transport.
