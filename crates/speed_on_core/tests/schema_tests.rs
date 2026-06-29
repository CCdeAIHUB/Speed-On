use speed_on_core::storage::schema::{
    CREATE_ACTIVITY_RECORDS_TABLE, CREATE_INDEXED_RESOURCES_TABLE,
    CREATE_QUERY_RESOURCE_SELECTION_STATS_TABLE, CREATE_RESOURCE_SEARCH_ALIASES_TABLE,
    CREATE_RESOURCE_USAGE_STATS_TABLE, CREATE_SYSTEM_LOGS_TABLE, CREATE_USER_SEARCH_LOGS_TABLE,
    CREATE_USER_SELECTION_LOGS_TABLE, MIGRATIONS, SCHEMA_VERSION,
};

#[test]
fn schema_version_is_two_after_search_and_logging_contracts() {
    // 场景：新增搜索索引、用户操作日志、系统日志后，数据库 schema 必须升级到 v2。
    assert_eq!(SCHEMA_VERSION, 2);
}

#[test]
fn schema_contains_resource_activity_and_usage_tables() {
    // 场景：推荐系统必须同时保存资源、不可变活动日志和聚合使用统计，不能只靠临时内存排序。
    assert!(CREATE_INDEXED_RESOURCES_TABLE.contains("indexed_resources"));
    assert!(CREATE_ACTIVITY_RECORDS_TABLE.contains("activity_records"));
    assert!(CREATE_RESOURCE_USAGE_STATS_TABLE.contains("resource_usage_stats"));
}

#[test]
fn indexed_resources_schema_keeps_launch_target_and_icon_path() {
    // 场景：应用扫描结果必须保留可打开目标和图标路径，供前端展示与启动调用使用。
    assert!(CREATE_INDEXED_RESOURCES_TABLE.contains("target TEXT NOT NULL"));
    assert!(CREATE_INDEXED_RESOURCES_TABLE.contains("icon_path TEXT"));
    assert!(CREATE_INDEXED_RESOURCES_TABLE.contains("UNIQUE(kind, target)"));
}

#[test]
fn schema_supports_all_planned_resource_kinds() {
    // 场景：软件、文件、文件夹、浏览器地址都必须进入同一推荐模型，但类型需要可过滤。
    for resource_kind in ["application", "file", "folder", "browser_url"] {
        assert!(CREATE_INDEXED_RESOURCES_TABLE.contains(resource_kind));
        assert!(CREATE_ACTIVITY_RECORDS_TABLE.contains(resource_kind));
        assert!(CREATE_USER_SELECTION_LOGS_TABLE.contains(resource_kind));
    }
}

#[test]
fn schema_contains_search_aliases_for_pinyin_and_browser_titles() {
    // 场景：搜索必须支持浏览器标题、完整拼音和拼音首字母，不能只匹配原始路径或 URL。
    assert!(CREATE_RESOURCE_SEARCH_ALIASES_TABLE.contains("resource_search_aliases"));
    assert!(CREATE_RESOURCE_SEARCH_ALIASES_TABLE.contains("browser_title"));
    assert!(CREATE_RESOURCE_SEARCH_ALIASES_TABLE.contains("pinyin_full"));
    assert!(CREATE_RESOURCE_SEARCH_ALIASES_TABLE.contains("pinyin_initials"));
    assert!(CREATE_RESOURCE_SEARCH_ALIASES_TABLE.contains("normalized_alias_text"));
}

#[test]
fn schema_contains_user_operation_logs_and_selection_stats() {
    // 场景：前端搜索内容和最终打开选择必须被记录，并聚合为下次搜索排序信号。
    assert!(CREATE_USER_SEARCH_LOGS_TABLE.contains("raw_query TEXT NOT NULL"));
    assert!(CREATE_USER_SELECTION_LOGS_TABLE.contains("selected_resource_id TEXT NOT NULL"));
    assert!(CREATE_USER_SELECTION_LOGS_TABLE.contains("selected_target TEXT NOT NULL"));
    assert!(CREATE_QUERY_RESOURCE_SELECTION_STATS_TABLE.contains("selection_count"));
    assert!(CREATE_QUERY_RESOURCE_SELECTION_STATS_TABLE
        .contains("PRIMARY KEY(normalized_query, resource_id)"));
}

#[test]
fn schema_contains_sanitized_system_logs() {
    // 场景：运行期错误和诊断信息必须进入系统日志，但与用户搜索/选择日志分表隔离。
    assert!(CREATE_SYSTEM_LOGS_TABLE.contains("system_logs"));
    assert!(CREATE_SYSTEM_LOGS_TABLE.contains("level TEXT NOT NULL"));
    assert!(CREATE_SYSTEM_LOGS_TABLE.contains("module TEXT NOT NULL"));
    assert!(CREATE_SYSTEM_LOGS_TABLE.contains("context_summary TEXT"));
    assert!(CREATE_SYSTEM_LOGS_TABLE.contains("trace_id TEXT"));
}

#[test]
fn migrations_include_indexes_for_recommendation_search_and_logs() {
    // 场景：前端频繁请求推荐和搜索时，后端不能每次全表扫描活动日志、别名表或选择日志。
    let migration_text = MIGRATIONS.join("\n");

    assert!(migration_text.contains("idx_indexed_resources_kind"));
    assert!(migration_text.contains("idx_activity_records_target_time"));
    assert!(migration_text.contains("idx_resource_usage_recommendation"));
    assert!(migration_text.contains("idx_resource_search_aliases_lookup"));
    assert!(migration_text.contains("idx_user_search_logs_query_time"));
    assert!(migration_text.contains("idx_query_resource_selection_stats_lookup"));
    assert!(migration_text.contains("idx_system_logs_level_time"));
}
