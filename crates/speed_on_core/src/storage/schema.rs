pub const SCHEMA_VERSION: u32 = 1;

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

pub const MIGRATIONS: &[&str] = &[
    CREATE_INDEXED_RESOURCES_TABLE,
    CREATE_ACTIVITY_RECORDS_TABLE,
    CREATE_RESOURCE_USAGE_STATS_TABLE,
    CREATE_INDEXED_RESOURCES_KIND_INDEX,
    CREATE_ACTIVITY_TARGET_TIME_INDEX,
    CREATE_USAGE_RECOMMENDATION_INDEX,
];
