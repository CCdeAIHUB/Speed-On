pub const SCHEMA_VERSION: u32 = 2;

/// SQLite table for normalized resources that can be opened by the frontend.
///
/// The `(kind, target)` uniqueness rule prevents duplicated application paths,
/// file paths, folders, and browser URLs from splitting usage statistics across
/// multiple resource IDs.
pub const CREATE_INDEXED_RESOURCES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS indexed_resources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('application', 'file', 'folder', 'browser_url')),
    title TEXT NOT NULL,
    target TEXT NOT NULL,
    icon_path TEXT,
    source TEXT NOT NULL,
    first_seen_at_millis INTEGER NOT NULL,
    last_seen_at_millis INTEGER NOT NULL,
    created_at_millis INTEGER NOT NULL,
    updated_at_millis INTEGER NOT NULL,
    UNIQUE(kind, target)
);
"#;

/// SQLite table for immutable activity events.
///
/// Activity records stay append-only so the recommendation model can be rebuilt
/// if scoring rules change later. Aggregated counters belong in
/// `resource_usage_stats`, not in this event table.
pub const CREATE_ACTIVITY_RECORDS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS activity_records (
    id TEXT PRIMARY KEY,
    resource_id TEXT,
    kind TEXT NOT NULL CHECK (kind IN ('application', 'file', 'folder', 'browser_url')),
    target TEXT NOT NULL,
    opened_at_millis INTEGER NOT NULL,
    source TEXT NOT NULL,
    created_at_millis INTEGER NOT NULL,
    FOREIGN KEY(resource_id) REFERENCES indexed_resources(id)
);
"#;

/// SQLite table for recommendation-friendly usage aggregates.
///
/// Keeping this aggregate separate avoids repeatedly scanning the full activity
/// log when the frontend asks for a small number of recommendations.
pub const CREATE_RESOURCE_USAGE_STATS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS resource_usage_stats (
    resource_id TEXT PRIMARY KEY,
    open_count INTEGER NOT NULL DEFAULT 0,
    last_opened_at_millis INTEGER,
    updated_at_millis INTEGER NOT NULL,
    FOREIGN KEY(resource_id) REFERENCES indexed_resources(id)
);
"#;

/// SQLite table for search aliases.
///
/// Pinyin and pinyin-initial matching are stored as aliases instead of being
/// hardcoded inside the search algorithm. This keeps future dictionary or
/// tokenizer choices replaceable without changing frontend search behavior.
pub const CREATE_RESOURCE_SEARCH_ALIASES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS resource_search_aliases (
    id TEXT PRIMARY KEY,
    resource_id TEXT NOT NULL,
    alias_kind TEXT NOT NULL CHECK (alias_kind IN ('title', 'target', 'browser_title', 'pinyin_full', 'pinyin_initials', 'custom')),
    alias_text TEXT NOT NULL,
    normalized_alias_text TEXT NOT NULL,
    created_at_millis INTEGER NOT NULL,
    FOREIGN KEY(resource_id) REFERENCES indexed_resources(id),
    UNIQUE(resource_id, alias_kind, normalized_alias_text)
);
"#;

/// SQLite table for frontend search query logs.
///
/// This belongs to user-operation data, not sanitized system diagnostics. Storage
/// implementations must apply privacy settings before exporting or syncing it.
pub const CREATE_USER_SEARCH_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS user_search_logs (
    id TEXT PRIMARY KEY,
    raw_query TEXT NOT NULL,
    normalized_query TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    searched_at_millis INTEGER NOT NULL
);
"#;

/// SQLite table for the final resource opened from a search result.
///
/// This records the user's actual choice so future searches can put previously
/// selected resources before ordinary title/path/url matches while still
/// deduplicating by resource id.
pub const CREATE_USER_SELECTION_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS user_selection_logs (
    id TEXT PRIMARY KEY,
    raw_query TEXT NOT NULL,
    normalized_query TEXT NOT NULL,
    selected_resource_id TEXT NOT NULL,
    selected_kind TEXT NOT NULL CHECK (selected_kind IN ('application', 'file', 'folder', 'browser_url')),
    selected_title TEXT NOT NULL,
    selected_target TEXT NOT NULL,
    selected_rank INTEGER NOT NULL,
    opened_at_millis INTEGER NOT NULL,
    FOREIGN KEY(selected_resource_id) REFERENCES indexed_resources(id)
);
"#;

/// SQLite table for query-to-resource selection aggregates.
///
/// Search uses this table as a compact user-behavior signal instead of scanning
/// the full `user_selection_logs` table on every keystroke.
pub const CREATE_QUERY_RESOURCE_SELECTION_STATS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS query_resource_selection_stats (
    normalized_query TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    selection_count INTEGER NOT NULL DEFAULT 0,
    last_selected_at_millis INTEGER NOT NULL,
    updated_at_millis INTEGER NOT NULL,
    PRIMARY KEY(normalized_query, resource_id),
    FOREIGN KEY(resource_id) REFERENCES indexed_resources(id)
);
"#;

/// SQLite table for sanitized runtime diagnostics.
///
/// System logs must describe runtime stages, errors, and sanitized context. Raw
/// user search text, full browser URLs, tokens, passwords, and private file
/// contents must not be copied here by adapters.
pub const CREATE_SYSTEM_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS system_logs (
    id TEXT PRIMARY KEY,
    level TEXT NOT NULL CHECK (level IN ('debug', 'info', 'warn', 'error')),
    module TEXT NOT NULL,
    message TEXT NOT NULL,
    context_summary TEXT,
    trace_id TEXT,
    occurred_at_millis INTEGER NOT NULL
);
"#;

pub const CREATE_INDEXED_RESOURCES_KIND_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_indexed_resources_kind
ON indexed_resources(kind);
"#;

pub const CREATE_ACTIVITY_TARGET_TIME_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_activity_records_target_time
ON activity_records(kind, target, opened_at_millis DESC);
"#;

pub const CREATE_USAGE_RECOMMENDATION_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_resource_usage_recommendation
ON resource_usage_stats(open_count DESC, last_opened_at_millis DESC);
"#;

pub const CREATE_SEARCH_ALIAS_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_resource_search_aliases_lookup
ON resource_search_aliases(normalized_alias_text, alias_kind);
"#;

pub const CREATE_USER_SEARCH_LOG_QUERY_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_user_search_logs_query_time
ON user_search_logs(normalized_query, searched_at_millis DESC);
"#;

pub const CREATE_USER_SELECTION_STATS_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_query_resource_selection_stats_lookup
ON query_resource_selection_stats(normalized_query, selection_count DESC, last_selected_at_millis DESC);
"#;

pub const CREATE_SYSTEM_LOGS_LEVEL_TIME_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_system_logs_level_time
ON system_logs(level, occurred_at_millis DESC);
"#;

pub const MIGRATIONS: &[&str] = &[
    CREATE_INDEXED_RESOURCES_TABLE,
    CREATE_ACTIVITY_RECORDS_TABLE,
    CREATE_RESOURCE_USAGE_STATS_TABLE,
    CREATE_RESOURCE_SEARCH_ALIASES_TABLE,
    CREATE_USER_SEARCH_LOGS_TABLE,
    CREATE_USER_SELECTION_LOGS_TABLE,
    CREATE_QUERY_RESOURCE_SELECTION_STATS_TABLE,
    CREATE_SYSTEM_LOGS_TABLE,
    CREATE_INDEXED_RESOURCES_KIND_INDEX,
    CREATE_ACTIVITY_TARGET_TIME_INDEX,
    CREATE_USAGE_RECOMMENDATION_INDEX,
    CREATE_SEARCH_ALIAS_INDEX,
    CREATE_USER_SEARCH_LOG_QUERY_INDEX,
    CREATE_USER_SELECTION_STATS_INDEX,
    CREATE_SYSTEM_LOGS_LEVEL_TIME_INDEX,
];
