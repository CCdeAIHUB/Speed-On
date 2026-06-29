use serde_json::{json, Value};
use speed_on_core::{
    ApiErrorResponse, ApiResource, ApiResourceKind, AppResult, CoreApi, IndexedResource,
    InstalledApplicationScanner, IpcCommand, IpcRequest, JsonIpcDispatcher,
    JsonIpcDispatcherWithOpener, JsonIpcDispatcherWithScanner, OpenResourceOutcome,
    OpenResourceRequest, ResourceKind, ResourceOpener, ResourceRepository, SearchAlias,
    SearchAliasKind, SqliteStore, IPC_PROTOCOL_VERSION,
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

fn some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected Some value"),
    }
}

fn resource() -> IndexedResource {
    IndexedResource {
        id: "app-terminal".to_owned(),
        kind: ResourceKind::Application,
        title: "Terminal".to_owned(),
        target: "/apps/terminal".to_owned(),
        icon_path: Some("terminal.png".to_owned()),
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 2,
    }
}

fn scanned_resource() -> IndexedResource {
    IndexedResource {
        id: "app-notes".to_owned(),
        kind: ResourceKind::Application,
        title: "Notes".to_owned(),
        target: "/apps/notes".to_owned(),
        icon_path: Some("notes.png".to_owned()),
        source: "mock_scanner".to_owned(),
        first_seen_at_millis: 10,
        last_seen_at_millis: 10,
    }
}

fn store_with_terminal() -> SqliteStore {
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    let indexed_resource = resource();
    ok(store.upsert_resources(&[indexed_resource]));
    ok(store.upsert_search_aliases(
        "app-terminal",
        &[
            SearchAlias::new(SearchAliasKind::Title, "Terminal"),
            SearchAlias::new(SearchAliasKind::Target, "/apps/terminal"),
        ],
        10,
    ));
    store
}

fn dispatcher_with_terminal() -> JsonIpcDispatcher<SqliteStore> {
    JsonIpcDispatcher::new(CoreApi::new(store_with_terminal()))
}

struct MockOpener;

impl ResourceOpener for MockOpener {
    fn open_resource(&mut self, request: &OpenResourceRequest) -> AppResult<OpenResourceOutcome> {
        Ok(OpenResourceOutcome {
            resource_id: request.resource.id.clone(),
            kind: request.resource.kind,
            target: request.resource.target.clone(),
            opened_at_millis: request.requested_at_millis,
        })
    }
}

struct MockScanner;

impl InstalledApplicationScanner for MockScanner {
    fn scan_installed_applications(&self) -> AppResult<Vec<IndexedResource>> {
        Ok(vec![scanned_resource()])
    }
}

#[test]
fn ipc_request_contract_uses_stable_json_envelope() {
    // 场景：真实 pipe/socket/http 传输层之前，IPC 请求 envelope 必须先稳定下来。
    let request = IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-1".to_owned(),
        command: IpcCommand::Search,
        payload: json!({
            "query": "term",
            "limit": 5,
            "kinds": ["application"],
            "now_millis": 100
        }),
    };

    assert_eq!(
        to_json(&request),
        json!({
            "protocol_version": "speed-on-ipc-v1",
            "request_id": "request-1",
            "command": "search",
            "payload": {
                "query": "term",
                "limit": 5,
                "kinds": ["application"],
                "now_millis": 100
            }
        })
    );
}

#[test]
fn ipc_open_resource_command_uses_stable_snake_case_name() {
    // 场景：打开资源命令必须在 IPC envelope 中稳定命名为 open_resource。
    let request = IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-open".to_owned(),
        command: IpcCommand::OpenResource,
        payload: json!({
            "resource": ApiResource::from(resource()),
            "requested_at_millis": 300
        }),
    };

    assert_eq!(to_json(&request)["command"], json!("open_resource"));
}

#[test]
fn ipc_refresh_applications_command_uses_stable_snake_case_name() {
    // 场景：应用扫描命令必须在 IPC envelope 中稳定命名为 refresh_applications。
    let request = IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-refresh".to_owned(),
        command: IpcCommand::RefreshApplications,
        payload: json!({
            "requested_at_millis": 400
        }),
    };

    assert_eq!(to_json(&request)["command"], json!("refresh_applications"));
}

#[test]
fn ipc_dispatcher_executes_search_command() {
    // 场景：前端发送 search 命令时，dispatcher 必须解码 payload、调用 CoreApi 并保留 request_id。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-search".to_owned(),
        command: IpcCommand::Search,
        payload: json!({
            "query": "term",
            "limit": 5,
            "kinds": ["application"],
            "now_millis": 100
        }),
    });

    assert_eq!(response.request_id, "request-search");
    assert_eq!(response.protocol_version, IPC_PROTOCOL_VERSION);
    assert!(response.response.ok);
    let data = some(response.response.data);
    assert_eq!(data["api_version"], json!("core-api-v1"));
    assert_eq!(data["results"][0]["resource"]["id"], json!("app-terminal"));
}

#[test]
fn ipc_dispatcher_executes_recommend_command() {
    // 场景：前端发送 recommend 命令时，dispatcher 必须返回统一 envelope，即使当前还没有 activity 也不能失败。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-recommend".to_owned(),
        command: IpcCommand::Recommend,
        payload: json!({
            "limit": 5,
            "kinds": ["application"],
            "now_millis": 100
        }),
    });

    assert_eq!(response.request_id, "request-recommend");
    assert!(response.response.ok);
    let data = some(response.response.data);
    assert_eq!(data["api_version"], json!("core-api-v1"));
}

#[test]
fn ipc_dispatcher_executes_record_selection_command() {
    // 场景：前端发送 record_selection 命令时，dispatcher 必须记录用户最终打开的资源。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-selection".to_owned(),
        command: IpcCommand::RecordSelection,
        payload: json!({
            "query": "term",
            "selected_resource": ApiResource::from(resource()),
            "selected_rank": 1,
            "opened_at_millis": 200
        }),
    });

    assert_eq!(response.request_id, "request-selection");
    assert!(response.response.ok);
    let data = some(response.response.data);
    assert_eq!(data["recorded"], json!(true));
}

#[test]
fn ipc_dispatcher_without_opener_rejects_open_resource() {
    // 场景：没有平台 opener adapter 时，open_resource 不能假装成功，必须返回 unsupported。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-open".to_owned(),
        command: IpcCommand::OpenResource,
        payload: json!({
            "resource": ApiResource::from(resource()),
            "requested_at_millis": 300
        }),
    });

    assert!(!response.response.ok);
    let error = some(response.response.error);
    assert_eq!(error.error_code, "CORE_PLATFORM_UNSUPPORTED");
    assert_eq!(error.module, "ipc::JsonIpcDispatcher::open_resource");
}

#[test]
fn ipc_dispatcher_with_opener_executes_open_resource() {
    // 场景：带 ResourceOpener 的 dispatcher 可以真正把 open_resource 分发给平台边界。
    let mut dispatcher = JsonIpcDispatcherWithOpener::new(CoreApi::new(store_with_terminal()), MockOpener);
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-open".to_owned(),
        command: IpcCommand::OpenResource,
        payload: json!({
            "resource": ApiResource::from(resource()),
            "requested_at_millis": 300
        }),
    });

    assert!(response.response.ok);
    let data = some(response.response.data);
    assert_eq!(data["opened"], json!(true));
    assert_eq!(data["activity_recorded"], json!(true));
    assert_eq!(data["resource_id"], json!("app-terminal"));
    assert_eq!(data["opened_at_millis"], json!(300));
}

#[test]
fn ipc_dispatcher_without_scanner_rejects_refresh_applications() {
    // 场景：没有平台 scanner adapter 时，refresh_applications 不能假装成功。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-refresh".to_owned(),
        command: IpcCommand::RefreshApplications,
        payload: json!({
            "requested_at_millis": 400
        }),
    });

    assert!(!response.response.ok);
    let error = some(response.response.error);
    assert_eq!(error.error_code, "CORE_PLATFORM_UNSUPPORTED");
    assert_eq!(error.module, "ipc::JsonIpcDispatcher::refresh_applications");
}

#[test]
fn ipc_dispatcher_with_scanner_executes_refresh_applications() {
    // 场景：带 InstalledApplicationScanner 的 dispatcher 可以扫描应用并写入 SQLite 索引。
    let mut dispatcher = JsonIpcDispatcherWithScanner::new(CoreApi::new(store_with_terminal()), MockScanner);
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-refresh".to_owned(),
        command: IpcCommand::RefreshApplications,
        payload: json!({
            "requested_at_millis": 400
        }),
    });

    assert!(response.response.ok);
    let data = some(response.response.data);
    assert_eq!(data["api_version"], json!("core-api-v1"));
    assert_eq!(data["scanned_count"], json!(1));
}

#[test]
fn ipc_dispatcher_rejects_unsupported_protocol_version() {
    // 场景：不同版本 IPC envelope 不能被静默接受，必须返回结构化错误。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: "speed-on-ipc-v0".to_owned(),
        request_id: "request-bad-version".to_owned(),
        command: IpcCommand::Search,
        payload: json!({
            "query": "term",
            "limit": 5,
            "kinds": ["application"],
            "now_millis": 100
        }),
    });

    assert!(!response.response.ok);
    let error: ApiErrorResponse = some(response.response.error);
    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "ipc::JsonIpcDispatcher");
}

#[test]
fn ipc_dispatcher_rejects_invalid_payload_without_panic() {
    // 场景：payload 缺字段或类型错误时，dispatcher 必须返回结构化错误，不能 panic 或返回成功。
    let mut dispatcher = dispatcher_with_terminal();
    let response = dispatcher.dispatch(IpcRequest {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: "request-invalid-payload".to_owned(),
        command: IpcCommand::Search,
        payload: json!({
            "query": "term",
            "limit": "five"
        }),
    });

    assert!(!response.response.ok);
    let error: ApiErrorResponse = some(response.response.error);
    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "ipc::JsonIpcDispatcher::search");
}

#[test]
fn ipc_resource_kind_stays_snake_case_inside_payload() {
    // 场景：IPC payload 复用 Core API DTO，因此 resource kind 必须继续保持 snake_case。
    let resource = ApiResource {
        id: "url-rust".to_owned(),
        kind: ApiResourceKind::BrowserUrl,
        title: "Rust".to_owned(),
        target: "browser://rust".to_owned(),
        icon_path: None,
    };

    assert_eq!(to_json(&resource)["kind"], json!("browser_url"));
}
