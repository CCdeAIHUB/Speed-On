use speed_on_core::storage::schema::{
    CREATE_ACTIVITY_RECORDS_TABLE, CREATE_INDEXED_RESOURCES_TABLE,
    CREATE_RESOURCE_USAGE_STATS_TABLE, MIGRATIONS, SCHEMA_VERSION,
};

#[test]
fn schema_version_starts_at_one() {
    // 场景：后续数据库迁移需要稳定的初始版本号，避免安装流程无法判断 schema 起点。
    assert_eq!(SCHEMA_VERSION, 1);
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
    }
}

#[test]
fn migrations_include_indexes_for_recommendation_queries() {
    // 场景：前端频繁请求推荐时，后端不能每次全表扫描活动日志。
    let migration_text = MIGRATIONS.join("\n");

    assert!(migration_text.contains("idx_indexed_resources_kind"));
    assert!(migration_text.contains("idx_activity_records_target_time"));
    assert!(migration_text.contains("idx_resource_usage_recommendation"));
}
