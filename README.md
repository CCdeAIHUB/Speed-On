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

## Architecture rules

The backend follows the Codex stability and anti-corruption development skill used for this repository:

- High modularity and low coupling.
- Platform-specific behavior must live behind adapter/provider/gateway boundaries.
- Domain logic must not depend on Windows, macOS, Linux, SQLite, or frontend implementation details.
- Important behavior should be locked by tests, not only documented.
- Critical failure paths must return structured errors and must not fail silently.
- User operation logs and system logs must stay separated because user queries, file paths, and browser URLs are sensitive data.

## Current implementation stage

Stage 3 lands the SQLite repository behind the existing backend contracts:

- Rust workspace.
- Domain models.
- Repository and platform abstraction traits.
- SQLite schema contract v2.
- SQLite migration runner using `PRAGMA user_version`.
- SQLite-backed resource repository, search index repository, user operation log repository, and system log sink.
- Recommendation service.
- Search service with title, target, browser title, pinyin, pinyin-initial, and user-history ranking support.
- User operation log and sanitized system log models.
- TDD tests for recommendation behavior, search behavior, logging behavior, schema expectations, and SQLite persistence.

Platform-specific scanners, OS log listeners, browser-history readers, pinyin alias builders, and frontend bindings will be added in later stages.
