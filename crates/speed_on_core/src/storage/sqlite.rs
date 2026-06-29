use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, params_from_iter, Connection};

use crate::domain::{ActivityRecord, CandidateResource, IndexedResource, ResourceKind};
use crate::error::{AppError, AppResult};
use crate::logging::{SystemLogEntry, UserSearchLogEntry, UserSelectionLogEntry};
use crate::ports::{
    ResourceRepository, SearchIndexRepository, SystemLogSink, UserOperationLogRepository,
};
use crate::search::{
    normalize_search_query, SearchAlias, SearchAliasKind, SearchCandidate, UserSelectionSignal,
};
use crate::storage::schema;

pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let connection =
            Connection::open(path).map_err(|error| sqlite_error(error, "storage::SqliteStore"))?;
        Ok(Self { connection })
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let connection = Connection::open_in_memory()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore"))?;
        Ok(Self { connection })
    }

    pub fn open_migrated(path: impl AsRef<Path>) -> AppResult<Self> {
        let mut store = Self::open(path)?;
        store.apply_migrations()?;
        Ok(store)
    }

    pub fn open_in_memory_migrated() -> AppResult<Self> {
        let mut store = Self::open_in_memory()?;
        store.apply_migrations()?;
        Ok(store)
    }

    pub fn apply_migrations(&mut self) -> AppResult<()> {
        let tx = self
            .connection
            .transaction()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::apply_migrations"))?;

        // Migrations are executed in a transaction so a partial schema cannot be
        // left behind if one statement fails. `user_version` is the lightweight
        // SQLite-native marker used by later migration runners to determine the
        // current database contract version.
        tx.execute_batch(&schema::MIGRATIONS.join("\n"))
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::apply_migrations"))?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", schema::SCHEMA_VERSION))
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::apply_migrations"))?;
        tx.commit()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::apply_migrations"))
    }

    pub fn schema_version(&self) -> AppResult<u32> {
        let version: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::schema_version"))?;
        i64_to_u32(version, "storage::SqliteStore::schema_version")
    }

    pub fn upsert_search_aliases(
        &mut self,
        resource_id: &str,
        aliases: &[SearchAlias],
        created_at_millis: u64,
    ) -> AppResult<()> {
        let tx = self
            .connection
            .transaction()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_search_aliases"))?;
        let created_at = u64_to_i64(
            created_at_millis,
            "storage::SqliteStore::upsert_search_aliases",
        )?;

        for alias in aliases {
            let normalized_alias = normalize_search_query(&alias.value);
            if normalized_alias.is_empty() {
                return Err(AppError::invalid_argument(
                    "search alias must not normalize to empty text",
                    "storage::SqliteStore::upsert_search_aliases",
                ));
            }

            let alias_id = build_alias_id(resource_id, alias.kind, &normalized_alias);
            tx.execute(
                r#"
                INSERT INTO resource_search_aliases (
                    id, resource_id, alias_kind, alias_text, normalized_alias_text, created_at_millis
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(resource_id, alias_kind, normalized_alias_text) DO UPDATE SET
                    alias_text = excluded.alias_text
                "#,
                params![
                    alias_id.as_str(),
                    resource_id,
                    alias.kind.as_str(),
                    alias.value.as_str(),
                    normalized_alias.as_str(),
                    created_at,
                ],
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_search_aliases"))?;
        }

        tx.commit()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_search_aliases"))
    }

    pub fn count_user_search_logs(&self) -> AppResult<u64> {
        self.count_rows("user_search_logs")
    }

    pub fn count_user_selection_logs(&self) -> AppResult<u64> {
        self.count_rows("user_selection_logs")
    }

    pub fn count_system_logs(&self) -> AppResult<u64> {
        self.count_rows("system_logs")
    }

    pub fn load_selection_signal(
        &self,
        normalized_query: &str,
        resource_id: &str,
    ) -> AppResult<Option<UserSelectionSignal>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT normalized_query, selection_count, last_selected_at_millis
                FROM query_resource_selection_stats
                WHERE normalized_query = ?1 AND resource_id = ?2
                "#,
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::load_selection_signal"))?;

        let mut rows = statement
            .query(params![normalized_query, resource_id])
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::load_selection_signal"))?;

        match rows
            .next()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::load_selection_signal"))?
        {
            Some(row) => Ok(Some(UserSelectionSignal {
                normalized_query: row.get(0).map_err(|error| {
                    sqlite_error(error, "storage::SqliteStore::load_selection_signal")
                })?,
                selection_count: i64_to_u64(
                    row.get(1).map_err(|error| {
                        sqlite_error(error, "storage::SqliteStore::load_selection_signal")
                    })?,
                    "storage::SqliteStore::load_selection_signal",
                )?,
                last_selected_at_millis: i64_to_u64(
                    row.get(2).map_err(|error| {
                        sqlite_error(error, "storage::SqliteStore::load_selection_signal")
                    })?,
                    "storage::SqliteStore::load_selection_signal",
                )?,
            })),
            None => Ok(None),
        }
    }

    fn count_rows(&self, table_name: &'static str) -> AppResult<u64> {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        let count: i64 = self
            .connection
            .query_row(&sql, [], |row| row.get(0))
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::count_rows"))?;
        i64_to_u64(count, "storage::SqliteStore::count_rows")
    }

    fn load_resource_usage_rows(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<ResourceUsageRow>> {
        let kind_values = kind_filter_values(kinds);
        let mut sql = String::from(
            r#"
            SELECT
                r.id,
                r.kind,
                r.title,
                r.target,
                r.icon_path,
                r.source,
                r.first_seen_at_millis,
                r.last_seen_at_millis,
                COALESCE(u.open_count, 0),
                u.last_opened_at_millis
            FROM indexed_resources r
            LEFT JOIN resource_usage_stats u ON u.resource_id = r.id
            "#,
        );
        append_kind_filter(&mut sql, &kind_values);

        let mut statement = self.connection.prepare(&sql).map_err(|error| {
            sqlite_error(error, "storage::SqliteStore::load_resource_usage_rows")
        })?;
        let rows = statement
            .query_map(params_from_iter(kind_values.iter()), |row| {
                Ok(ResourceUsageRow {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    title: row.get(2)?,
                    target: row.get(3)?,
                    icon_path: row.get(4)?,
                    source: row.get(5)?,
                    first_seen_at_millis: row.get(6)?,
                    last_seen_at_millis: row.get(7)?,
                    open_count: row.get(8)?,
                    last_opened_at_millis: row.get(9)?,
                })
            })
            .map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_resource_usage_rows")
            })?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push(row.map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_resource_usage_rows")
            })?);
        }

        Ok(resources)
    }

    #[allow(dead_code)]
    fn load_aliases_for_resource(&self, resource_id: &str) -> AppResult<Vec<SearchAlias>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT alias_kind, alias_text
                FROM resource_search_aliases
                WHERE resource_id = ?1
                ORDER BY alias_kind, alias_text
                "#,
            )
            .map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_aliases_for_resource")
            })?;
        let rows = statement
            .query_map(params![resource_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_aliases_for_resource")
            })?;

        let mut aliases = Vec::new();
        for row in rows {
            let (kind, value) = row.map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_aliases_for_resource")
            })?;
            aliases.push(SearchAlias::new(
                SearchAliasKind::try_from(kind.as_str())?,
                value,
            ));
        }

        Ok(aliases)
    }

    #[allow(dead_code)]
    fn load_selection_signals_for_resource(
        &self,
        resource_id: &str,
    ) -> AppResult<Vec<UserSelectionSignal>> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT normalized_query, selection_count, last_selected_at_millis
                FROM query_resource_selection_stats
                WHERE resource_id = ?1
                ORDER BY selection_count DESC, last_selected_at_millis DESC
                "#,
            )
            .map_err(|error| {
                sqlite_error(
                    error,
                    "storage::SqliteStore::load_selection_signals_for_resource",
                )
            })?;
        let rows = statement
            .query_map(params![resource_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|error| {
                sqlite_error(
                    error,
                    "storage::SqliteStore::load_selection_signals_for_resource",
                )
            })?;

        let mut signals = Vec::new();
        for row in rows {
            let (normalized_query, selection_count, last_selected_at_millis) =
                row.map_err(|error| {
                    sqlite_error(
                        error,
                        "storage::SqliteStore::load_selection_signals_for_resource",
                    )
                })?;
            signals.push(UserSelectionSignal {
                normalized_query,
                selection_count: i64_to_u64(
                    selection_count,
                    "storage::SqliteStore::load_selection_signals_for_resource",
                )?,
                last_selected_at_millis: i64_to_u64(
                    last_selected_at_millis,
                    "storage::SqliteStore::load_selection_signals_for_resource",
                )?,
            });
        }

        Ok(signals)
    }

    /// Batch-load aliases for multiple resources in a single query.
    ///
    /// This avoids the N+1 query problem where `load_search_candidates` would
    /// otherwise issue one alias query per resource.
    fn load_aliases_for_resources(
        &self,
        resource_ids: &[String],
    ) -> AppResult<HashMap<String, Vec<SearchAlias>>> {
        let mut map: HashMap<String, Vec<SearchAlias>> = HashMap::new();
        if resource_ids.is_empty() {
            return Ok(map);
        }

        let placeholders = build_placeholders(resource_ids.len());
        let sql = format!(
            r#"
            SELECT resource_id, alias_kind, alias_text
            FROM resource_search_aliases
            WHERE resource_id IN ({placeholders})
            ORDER BY resource_id, alias_kind, alias_text
            "#
        );
        let mut statement = self.connection.prepare(&sql).map_err(|error| {
            sqlite_error(error, "storage::SqliteStore::load_aliases_for_resources")
        })?;
        let rows = statement
            .query_map(params_from_iter(resource_ids.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_aliases_for_resources")
            })?;

        for row in rows {
            let (resource_id, kind, value) = row.map_err(|error| {
                sqlite_error(error, "storage::SqliteStore::load_aliases_for_resources")
            })?;
            let alias = SearchAlias::new(SearchAliasKind::try_from(kind.as_str())?, value);
            map.entry(resource_id).or_default().push(alias);
        }

        Ok(map)
    }

    /// Batch-load selection signals for multiple resources in a single query.
    fn load_selection_signals_for_resources(
        &self,
        resource_ids: &[String],
    ) -> AppResult<HashMap<String, Vec<UserSelectionSignal>>> {
        let mut map: HashMap<String, Vec<UserSelectionSignal>> = HashMap::new();
        if resource_ids.is_empty() {
            return Ok(map);
        }

        let placeholders = build_placeholders(resource_ids.len());
        let sql = format!(
            r#"
            SELECT resource_id, normalized_query, selection_count, last_selected_at_millis
            FROM query_resource_selection_stats
            WHERE resource_id IN ({placeholders})
            ORDER BY resource_id, selection_count DESC, last_selected_at_millis DESC
            "#
        );
        let mut statement = self.connection.prepare(&sql).map_err(|error| {
            sqlite_error(
                error,
                "storage::SqliteStore::load_selection_signals_for_resources",
            )
        })?;
        let rows = statement
            .query_map(params_from_iter(resource_ids.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| {
                sqlite_error(
                    error,
                    "storage::SqliteStore::load_selection_signals_for_resources",
                )
            })?;

        for row in rows {
            let (resource_id, normalized_query, selection_count, last_selected_at_millis) = row
                .map_err(|error| {
                    sqlite_error(
                        error,
                        "storage::SqliteStore::load_selection_signals_for_resources",
                    )
                })?;
            map.entry(resource_id)
                .or_default()
                .push(UserSelectionSignal {
                    normalized_query,
                    selection_count: i64_to_u64(
                        selection_count,
                        "storage::SqliteStore::load_selection_signals_for_resources",
                    )?,
                    last_selected_at_millis: i64_to_u64(
                        last_selected_at_millis,
                        "storage::SqliteStore::load_selection_signals_for_resources",
                    )?,
                });
        }

        Ok(map)
    }
}

impl ResourceRepository for SqliteStore {
    fn upsert_resources(&mut self, resources: &[IndexedResource]) -> AppResult<()> {
        let tx = self
            .connection
            .transaction()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_resources"))?;

        for resource in resources {
            tx.execute(
                r#"
                INSERT INTO indexed_resources (
                    id, kind, title, target, icon_path, source,
                    first_seen_at_millis, last_seen_at_millis, created_at_millis, updated_at_millis
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, ?8)
                ON CONFLICT(kind, target) DO UPDATE SET
                    title = excluded.title,
                    icon_path = excluded.icon_path,
                    source = excluded.source,
                    last_seen_at_millis = excluded.last_seen_at_millis,
                    updated_at_millis = excluded.updated_at_millis
                "#,
                params![
                    resource.id.as_str(),
                    resource.kind.as_str(),
                    resource.title.as_str(),
                    resource.target.as_str(),
                    resource.icon_path.as_deref(),
                    resource.source.as_str(),
                    u64_to_i64(
                        resource.first_seen_at_millis,
                        "storage::SqliteStore::upsert_resources"
                    )?,
                    u64_to_i64(
                        resource.last_seen_at_millis,
                        "storage::SqliteStore::upsert_resources"
                    )?,
                ],
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_resources"))?;
        }

        tx.commit()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::upsert_resources"))
    }

    fn record_activity(&mut self, activity: &ActivityRecord) -> AppResult<()> {
        let tx = self
            .connection
            .transaction()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_activity"))?;
        let opened_at = u64_to_i64(
            activity.opened_at_millis,
            "storage::SqliteStore::record_activity",
        )?;

        tx.execute(
            r#"
            INSERT INTO activity_records (
                id, resource_id, kind, target, opened_at_millis, source, created_at_millis
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5)
            "#,
            params![
                activity.id.as_str(),
                activity.resource_id.as_deref(),
                activity.kind.as_str(),
                activity.target.as_str(),
                opened_at,
                activity.source.as_str(),
            ],
        )
        .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_activity"))?;

        if let Some(resource_id) = activity.resource_id.as_deref() {
            tx.execute(
                r#"
                INSERT INTO resource_usage_stats (
                    resource_id, open_count, last_opened_at_millis, updated_at_millis
                ) VALUES (?1, 1, ?2, ?2)
                ON CONFLICT(resource_id) DO UPDATE SET
                    open_count = open_count + 1,
                    last_opened_at_millis = excluded.last_opened_at_millis,
                    updated_at_millis = excluded.updated_at_millis
                "#,
                params![resource_id, opened_at],
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_activity"))?;
        }

        tx.commit()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_activity"))
    }

    fn load_recommendation_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<CandidateResource>> {
        self.load_resource_usage_rows(kinds)?
            .into_iter()
            .map(|row| {
                let open_count = i64_to_u64(
                    row.open_count,
                    "storage::SqliteStore::load_recommendation_candidates",
                )?;
                let last_opened_at_millis = optional_i64_to_u64(
                    row.last_opened_at_millis,
                    "storage::SqliteStore::load_recommendation_candidates",
                )?;
                Ok(CandidateResource::new(
                    indexed_resource_from_row(row)?,
                    open_count,
                    last_opened_at_millis,
                ))
            })
            .collect()
    }
}

impl SearchIndexRepository for SqliteStore {
    fn load_search_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<SearchCandidate>> {
        let rows = self.load_resource_usage_rows(kinds)?;

        // Collect all resource IDs upfront so we can batch-load aliases and
        // selection signals instead of issuing one query per resource (N+1).
        let resource_ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
        let aliases_by_resource = self.load_aliases_for_resources(&resource_ids)?;
        let signals_by_resource = self.load_selection_signals_for_resources(&resource_ids)?;

        let mut candidates = Vec::new();
        for row in rows {
            let resource = indexed_resource_from_row(row.clone())?;
            let open_count = i64_to_u64(
                row.open_count,
                "storage::SqliteStore::load_search_candidates",
            )?;
            let last_opened_at_millis = optional_i64_to_u64(
                row.last_opened_at_millis,
                "storage::SqliteStore::load_search_candidates",
            )?;
            let aliases = aliases_by_resource.remove(&resource.id).unwrap_or_default();
            let signals = signals_by_resource.remove(&resource.id).unwrap_or_default();

            candidates.push(
                SearchCandidate::new(resource)
                    .with_aliases(aliases)
                    .with_user_selection_signals(signals)
                    .with_usage(open_count, last_opened_at_millis),
            );
        }

        Ok(candidates)
    }
}

impl UserOperationLogRepository for SqliteStore {
    fn record_user_search(&mut self, entry: &UserSearchLogEntry) -> AppResult<()> {
        self.connection
            .execute(
                r#"
                INSERT INTO user_search_logs (
                    id, raw_query, normalized_query, result_count, searched_at_millis
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    entry.id.as_str(),
                    entry.raw_query.as_str(),
                    entry.normalized_query.as_str(),
                    usize_to_i64(
                        entry.result_count,
                        "storage::SqliteStore::record_user_search"
                    )?,
                    u64_to_i64(
                        entry.searched_at_millis,
                        "storage::SqliteStore::record_user_search"
                    )?,
                ],
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_user_search"))?;
        Ok(())
    }

    fn record_user_selection(&mut self, entry: &UserSelectionLogEntry) -> AppResult<()> {
        let tx = self
            .connection
            .transaction()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_user_selection"))?;
        let opened_at = u64_to_i64(
            entry.opened_at_millis,
            "storage::SqliteStore::record_user_selection",
        )?;

        tx.execute(
            r#"
            INSERT INTO user_selection_logs (
                id, raw_query, normalized_query, selected_resource_id, selected_kind,
                selected_title, selected_target, selected_rank, opened_at_millis
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                entry.id.as_str(),
                entry.raw_query.as_str(),
                entry.normalized_query.as_str(),
                entry.selected_resource_id.as_str(),
                entry.selected_kind.as_str(),
                entry.selected_title.as_str(),
                entry.selected_target.as_str(),
                usize_to_i64(
                    entry.selected_rank,
                    "storage::SqliteStore::record_user_selection"
                )?,
                opened_at,
            ],
        )
        .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_user_selection"))?;

        tx.execute(
            r#"
            INSERT INTO query_resource_selection_stats (
                normalized_query, resource_id, selection_count, last_selected_at_millis, updated_at_millis
            ) VALUES (?1, ?2, 1, ?3, ?3)
            ON CONFLICT(normalized_query, resource_id) DO UPDATE SET
                selection_count = selection_count + 1,
                last_selected_at_millis = excluded.last_selected_at_millis,
                updated_at_millis = excluded.updated_at_millis
            "#,
            params![
                entry.normalized_query.as_str(),
                entry.selected_resource_id.as_str(),
                opened_at,
            ],
        )
        .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_user_selection"))?;

        tx.commit()
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_user_selection"))
    }
}

impl SystemLogSink for SqliteStore {
    fn record_system_log(&mut self, entry: &SystemLogEntry) -> AppResult<()> {
        self.connection
            .execute(
                r#"
                INSERT INTO system_logs (
                    id, level, module, message, context_summary, trace_id, occurred_at_millis
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    entry.id.as_str(),
                    entry.level.as_str(),
                    entry.module.as_str(),
                    entry.message.as_str(),
                    entry.context_summary.as_deref(),
                    entry.trace_id.as_deref(),
                    u64_to_i64(
                        entry.occurred_at_millis,
                        "storage::SqliteStore::record_system_log"
                    )?,
                ],
            )
            .map_err(|error| sqlite_error(error, "storage::SqliteStore::record_system_log"))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ResourceUsageRow {
    id: String,
    kind: String,
    title: String,
    target: String,
    icon_path: Option<String>,
    source: String,
    first_seen_at_millis: i64,
    last_seen_at_millis: i64,
    open_count: i64,
    last_opened_at_millis: Option<i64>,
}

fn indexed_resource_from_row(row: ResourceUsageRow) -> AppResult<IndexedResource> {
    Ok(IndexedResource {
        id: row.id,
        kind: ResourceKind::try_from(row.kind.as_str())?,
        title: row.title,
        target: row.target,
        icon_path: row.icon_path,
        source: row.source,
        first_seen_at_millis: i64_to_u64(
            row.first_seen_at_millis,
            "storage::SqliteStore::indexed_resource_from_row",
        )?,
        last_seen_at_millis: i64_to_u64(
            row.last_seen_at_millis,
            "storage::SqliteStore::indexed_resource_from_row",
        )?,
    })
}

fn append_kind_filter(sql: &mut String, kind_values: &[String]) {
    if kind_values.is_empty() {
        return;
    }

    sql.push_str(" WHERE r.kind IN (");
    for index in 0..kind_values.len() {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push('?');
    }
    sql.push(')');
}

/// Build a comma-separated list of `?` placeholders for use in `IN (...)` clauses.
fn build_placeholders(count: usize) -> String {
    (0..count)
        .map(|index| {
            if index == 0 {
                "?".to_owned()
            } else {
                ", ?".to_owned()
            }
        })
        .collect()
}

fn kind_filter_values(kinds: Option<&[ResourceKind]>) -> Vec<String> {
    match kinds {
        Some(kinds) => kinds.iter().map(|kind| kind.as_str().to_owned()).collect(),
        None => Vec::new(),
    }
}

fn build_alias_id(
    resource_id: &str,
    alias_kind: SearchAliasKind,
    normalized_alias: &str,
) -> String {
    format!(
        "alias-{resource_id}-{}-{normalized_alias}",
        alias_kind.as_str()
    )
}

fn sqlite_error(error: rusqlite::Error, module: &str) -> AppError {
    AppError::storage_failure("sqlite operation failed", module).with_cause(error.to_string())
}

fn u64_to_i64(value: u64, module: &str) -> AppResult<i64> {
    i64::try_from(value)
        .map_err(|_| AppError::invalid_argument("value is too large for sqlite integer", module))
}

fn usize_to_i64(value: usize, module: &str) -> AppResult<i64> {
    i64::try_from(value)
        .map_err(|_| AppError::invalid_argument("value is too large for sqlite integer", module))
}

fn i64_to_u64(value: i64, module: &str) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| {
        AppError::storage_failure("sqlite integer must not be negative", module)
            .with_cause(format!("value={value}"))
    })
}

fn i64_to_u32(value: i64, module: &str) -> AppResult<u32> {
    u32::try_from(value).map_err(|_| {
        AppError::storage_failure("sqlite integer is outside u32 range", module)
            .with_cause(format!("value={value}"))
    })
}

fn optional_i64_to_u64(value: Option<i64>, module: &str) -> AppResult<Option<u64>> {
    value.map(|inner| i64_to_u64(inner, module)).transpose()
}
