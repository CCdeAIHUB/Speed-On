use serde_json::{json, Value};
use speed_on_core::{
    ApiRecommendationRequest, ApiRecommendationResult, ApiRecordSelectionRequest, ApiResource,
    ApiResourceKind, ApiResponse, ApiSearchMatchKind, ApiSearchRequest, ApiSearchResult,
    AppError, CoreApi, IndexedResource, Recommendation, ResourceKind, ResourceRepository,
    SearchAlias, SearchAliasKind, SearchMatchKind, SearchResult, SqliteStore,
};

fn to_json<T>(value: &T) -> Value
where
    T: serde::Serialize,
{
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(error) => panic!("serialization failed unexpectedly: {error}"),
    }
}

fn ok<T>(result: speed_on_core::AppResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("operation failed unexpectedly: {error}"),
    }
}

fn resource() -> IndexedResource {
    IndexedResource {
        id: "app-terminal".to_owned(),
        kind: ResourceKind::Application,
        title: "Terminal".to_owned(),
        target: "/System/Applications/Utilities/Terminal.app".to_owned(),
        icon_path: Some("terminal.png".to_owned()),
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 2,
    }
}

#[test]
fn api_success_response_has_stable_json_shape() {
    // 场景：前端依赖统一响应包裹结构，成功响应必须包含 ok、data、error 三个稳定字段。
    let response = ApiResponse::success(json!({ "value": 1 }));

    assert_eq!(
        to_json(&response),
        json!({
            "ok": true,
            "data": { "value": 1 },
            "error": null
        })
    );
}

#[test]
fn api_error_response_hides_internal_cause() {
    // 场景：底层 SQLite 或平台错误 cause 可能包含路径/实现细节，API 错误响应不能暴露 cause 字段。
    let error = AppError::storage_failure("sqlite operation failed", "storage::SqliteStore")
        .with_cause("database is locked at /private/path/speed-on.db")
        .with_trace_id("trace-1");
    let response: ApiResponse<Value> = ApiResponse::failure(error);
    let serialized = to_json(&response);

    assert_eq!(serialized["ok"], json!(false));
    assert_eq!(serialized["data"], json!(null));
    assert_eq!(serialized["error"]["error_code"], json!("CORE_STORAGE_FAILURE"));
    assert_eq!(serialized["error"]["module"], json!("storage::SqliteStore"));
    assert_eq!(serialized["error"]["trace_id"], json!("trace-1"));
    assert!(serialized["error"].get("cause").is_none());
}

#[test]
fn search_request_contract_uses_snake_case_resource_kinds() {
    // 场景：前端搜索请求中的资源类型必须稳定为 snake_case，方便各平台原生语言解析。
    let request = ApiSearchRequest {
        query: "wx".to_owned(),
        limit: 5,
        kinds: Some(vec![ApiResourceKind::Application, ApiResourceKind::BrowserUrl]),
        now_millis: 100,
    };

    assert_eq!(
        to_json(&request),
        json!({
            "query": "wx",
            "limit": 5,
            "kinds": ["application", "browser_url"],
            "now_millis": 100
        })
    );
}

#[test]
fn recommendation_request_contract_uses_stable_fields() {
    // 场景：推荐接口不依赖搜索 query，只接收数量、资源类型和当前时间。
    let request = ApiRecommendationRequest {
        limit: 3,
        kinds: Some(vec![ApiResourceKind::Application]),
        now_millis: 100,
    };

    assert_eq!(
        to_json(&request),
        json!({
            "limit": 3,
            "kinds": ["application"],
            "now_millis": 100
        })
    );
}

#[test]
fn api_resource_converts_from_internal_resource_without_source_metadata() {
    // 场景：外部 API 只暴露前端需要展示和打开的字段，不能把内部 source/seen 时间泄漏成契约。
    let api_resource = ApiResource::from(resource());

    assert_eq!(
        to_json(&api_resource),
        json!({
            "id": "app-terminal",
            "kind": "application",
            "title": "Terminal",
            "target": "/System/Applications/Utilities/Terminal.app",
            "icon_path": "terminal.png"
        })
    );
}

#[test]
fn search_result_contract_preserves_match_kind() {
    // 场景：前端需要知道结果来自用户历史、标题、URL 还是拼音首字母，用于后续 UI 提示和调试。
    let result = ApiSearchResult::from(SearchResult {
        resource: resource(),
        score: 2180,
        match_kind: SearchMatchKind::UserHistory,
        reason: "previously selected".to_owned(),
    });

    assert_eq!(result.match_kind, ApiSearchMatchKind::UserHistory);
    assert_eq!(to_json(&result)["match_kind"], json!("user_history"));
}

#[test]
fn recommendation_result_contract_preserves_score_and_reason() {
    // 场景：推荐结果需要暴露分数和原因，方便前端调试展示，但不暴露内部候选结构。
    let result = ApiRecommendationResult::from(Recommendation {
        resource: resource(),
        score: 300,
        reason: "opened 3 times".to_owned(),
    });

    assert_eq!(to_json(&result)["score"], json!(300));
    assert_eq!(to_json(&result)["reason"], json!("opened 3 times"));
}

#[test]
fn record_selection_request_contract_contains_query_resource_rank_and_time() {
    // 场景：用户最终打开哪个内容必须可由前端传回 Core，字段必须包含 query、资源、排名和打开时间。
    let request = ApiRecordSelectionRequest {
        query: "term".to_owned(),
        selected_resource: ApiResource::from(resource()),
        selected_rank: 1,
        opened_at_millis: 200,
    };

    let serialized = to_json(&request);
    assert_eq!(serialized["query"], json!("term"));
    assert_eq!(serialized["selected_resource"]["id"], json!("app-terminal"));
    assert_eq!(serialized["selected_rank"], json!(1));
    assert_eq!(serialized["opened_at_millis"], json!(200));
}

#[test]
fn core_api_facade_executes_search_recommend_and_record_selection() {
    // 场景：前端最终会调用 CoreApi facade，因此契约测试必须验证 facade 能组合 SQLite、搜索和推荐服务。
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let indexed_resource = resource();
    ok(store.upsert_resources(&[indexed_resource.clone()]));
    ok(store.upsert_search_aliases(
        "app-terminal",
        &[
            SearchAlias::new(SearchAliasKind::Title, "Terminal"),
            SearchAlias::new(SearchAliasKind::Target, "/System/Applications/Utilities/Terminal.app"),
        ],
        10,
    ));

    let mut api = CoreApi::new(store);
    let search_response = api.search(ApiSearchRequest {
        query: "term".to_owned(),
        limit: 5,
        kinds: Some(vec![ApiResourceKind::Application]),
        now_millis: 100,
    });
    assert!(search_response.ok);
    let search_data = match search_response.data {
        Some(data) => data,
        None => panic!("expected search data"),
    };
    assert_eq!(search_data.results.len(), 1);
    assert_eq!(search_data.results[0].resource.id, "app-terminal");

    let recommend_response = api.recommend(ApiRecommendationRequest {
        limit: 5,
        kinds: Some(vec![ApiResourceKind::Application]),
        now_millis: 100,
    });
    assert!(recommend_response.ok);

    let selection_response = api.record_selection(ApiRecordSelectionRequest {
        query: "term".to_owned(),
        selected_resource: ApiResource::from(indexed_resource),
        selected_rank: 1,
        opened_at_millis: 200,
    });
    assert!(selection_response.ok);
    assert_eq!(selection_response.data.expect("selection data").recorded, true);
}
