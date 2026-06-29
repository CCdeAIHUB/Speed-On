use speed_on_core::storage::schema::SCHEMA_VERSION;
use speed_on_core::{
    ActivityRecord, AppResult, IndexedResource, LogLevel, ResourceKind, ResourceRepository,
    SearchAlias, SearchAliasKind, SearchIndexRepository, SqliteStore, SystemLogEntry,
    SystemLogSink, UserOperationLogRepository, UserSelectionLogEntry,
};

fn ok<T>(result: AppResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("operation failed unexpectedly: {error}"),
    }
}

fn some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected Some value"),
    }
}

fn resource(id: &str, kind: ResourceKind, title: &str, target: &str) -> IndexedResource {
    IndexedResource {
        id: id.to_owned(),
        kind,
        title: title.to_owned(),
        target: target.to_owned(),
        icon_path: Some(format!("{id}.png")),
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 2,
    }
}

#[test]
fn migration_runner_creates_schema_and_sets_user_version() {
    // 场景：首次启动或安装后，SQLite migration runner 必须创建 v2 schema 并写入 user_version。
    let store = ok(SqliteStore::open_in_memory_migrated());

    assert_eq!(ok(store.schema_version()), SCHEMA_VERSION);
}

#[test]
fn sqlite_repository_upserts_resources_and_keeps_unique_kind_target() {
    // 场景：同一个应用路径被重复扫描时，不能产生重复资源，只能更新标题和最近发现时间。
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let first = resource(
        "app-terminal-1",
        ResourceKind::Application,
        "Terminal",
        "/System/Applications/Utilities/Terminal.app",
    );
    let mut updated = resource(
        "app-terminal-2",
        ResourceKind::Application,
        "Terminal Updated",
        "/System/Applications/Utilities/Terminal.app",
    );
    updated.last_seen_at_millis = 9;

    ok(store.upsert_resources(&[first, updated]));

    let kinds = [ResourceKind::Application];
    let candidates = ok(store.load_recommendation_candidates(Some(&kinds)));
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].resource.title, "Terminal Updated");
    assert_eq!(candidates[0].resource.last_seen_at_millis, 9);
}

#[test]
fn sqlite_repository_records_activity_and_updates_usage_stats() {
    // 场景：打开应用时必须同时保留不可变活动日志，并更新推荐读取所需的聚合计数。
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let app = resource(
        "app-terminal",
        ResourceKind::Application,
        "Terminal",
        "/System/Applications/Utilities/Terminal.app",
    );
    ok(store.upsert_resources(&[app]));

    ok(store.record_activity(&ActivityRecord {
        id: "activity-1".to_owned(),
        resource_id: Some("app-terminal".to_owned()),
        kind: ResourceKind::Application,
        target: "/System/Applications/Utilities/Terminal.app".to_owned(),
        opened_at_millis: 100,
        source: "test".to_owned(),
    }));
    ok(store.record_activity(&ActivityRecord {
        id: "activity-2".to_owned(),
        resource_id: Some("app-terminal".to_owned()),
        kind: ResourceKind::Application,
        target: "/System/Applications/Utilities/Terminal.app".to_owned(),
        opened_at_millis: 200,
        source: "test".to_owned(),
    }));

    let kinds = [ResourceKind::Application];
    let candidates = ok(store.load_recommendation_candidates(Some(&kinds)));
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].open_count, 2);
    assert_eq!(candidates[0].last_opened_at_millis, Some(200));
}

#[test]
fn sqlite_search_candidates_include_aliases_usage_and_user_selection_signals() {
    // 场景：搜索候选必须从 SQLite 同时加载标题/拼音别名、打开次数和用户历史选择信号。
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let app = resource(
        "app-wechat",
        ResourceKind::Application,
        "微信",
        "/Applications/WeChat.app",
    );
    ok(store.upsert_resources(&[app]));
    ok(store.upsert_search_aliases(
        "app-wechat",
        &[
            SearchAlias::new(SearchAliasKind::Title, "微信"),
            SearchAlias::new(SearchAliasKind::PinyinFull, "weixin"),
            SearchAlias::new(SearchAliasKind::PinyinInitials, "wx"),
        ],
        10,
    ));
    ok(store.record_activity(&ActivityRecord {
        id: "activity-wechat".to_owned(),
        resource_id: Some("app-wechat".to_owned()),
        kind: ResourceKind::Application,
        target: "/Applications/WeChat.app".to_owned(),
        opened_at_millis: 100,
        source: "test".to_owned(),
    }));
    ok(store.record_user_selection(&UserSelectionLogEntry {
        id: "selection-wechat".to_owned(),
        raw_query: "wx".to_owned(),
        normalized_query: "wx".to_owned(),
        selected_resource_id: "app-wechat".to_owned(),
        selected_kind: ResourceKind::Application,
        selected_title: "微信".to_owned(),
        selected_target: "/Applications/WeChat.app".to_owned(),
        selected_rank: 1,
        opened_at_millis: 110,
    }));

    let candidates = ok(store.load_search_candidates(None));
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].resource.id, "app-wechat");
    assert_eq!(candidates[0].open_count, 1);
    assert_eq!(candidates[0].aliases.len(), 3);
    assert_eq!(candidates[0].user_selection_signals.len(), 1);
    assert_eq!(
        candidates[0].user_selection_signals[0].normalized_query,
        "wx"
    );
}

#[test]
fn sqlite_user_selection_log_updates_query_resource_stats() {
    // 场景：用户连续用同一个 query 打开同一资源时，必须聚合为 selection_count，供下次搜索优先排序。
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let app = resource(
        "app-terminal",
        ResourceKind::Application,
        "Terminal",
        "/System/Applications/Utilities/Terminal.app",
    );
    ok(store.upsert_resources(&[app]));

    for index in 0..2 {
        ok(store.record_user_selection(&UserSelectionLogEntry {
            id: format!("selection-terminal-{index}"),
            raw_query: "term".to_owned(),
            normalized_query: "term".to_owned(),
            selected_resource_id: "app-terminal".to_owned(),
            selected_kind: ResourceKind::Application,
            selected_title: "Terminal".to_owned(),
            selected_target: "/System/Applications/Utilities/Terminal.app".to_owned(),
            selected_rank: 1,
            opened_at_millis: 100 + index,
        }));
    }

    let signal = some(ok(store.load_selection_signal("term", "app-terminal")));
    assert_eq!(ok(store.count_user_selection_logs()), 2);
    assert_eq!(signal.selection_count, 2);
    assert_eq!(signal.last_selected_at_millis, 101);
}

#[test]
fn sqlite_logs_user_searches_and_system_events() {
    // 场景：用户搜索日志和系统日志必须真实落库，并且分表统计。
    let mut store = ok(SqliteStore::open_in_memory_migrated());

    ok(
        store.record_user_search(&speed_on_core::UserSearchLogEntry {
            id: "search-1".to_owned(),
            raw_query: " Term ".to_owned(),
            normalized_query: "term".to_owned(),
            result_count: 3,
            searched_at_millis: 100,
        }),
    );
    ok(store.record_system_log(&SystemLogEntry::new(
        "system-1",
        LogLevel::Info,
        "storage::SqliteStore",
        "migration completed",
        101,
    )));

    assert_eq!(ok(store.count_user_search_logs()), 1);
    assert_eq!(ok(store.count_system_logs()), 1);
}
