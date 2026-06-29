use std::io::Cursor;

use serde_json::{json, Value};
use speed_on_core::{
    AppResult, CoreApi, IndexedResource, IpcRequest, JsonIpcDispatcherWithOpener, ResourceKind,
    ResourceRepository, SearchAlias, SearchAliasKind, SqliteStore, IPC_PROTOCOL_VERSION,
};
use speed_on_ipc_stdio::{run_json_lines_transport, IpcDispatcher, StdioConfig};
use speed_on_platform::{CommandPlan, CommandResourceOpener, CommandRunner};

fn ok<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
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

fn parse_json_line(output: &[u8]) -> Value {
    let text = match std::str::from_utf8(output) {
        Ok(text) => text,
        Err(error) => panic!("output was not utf-8: {error}"),
    };
    let first_line = some(text.lines().next());
    ok(serde_json::from_str::<Value>(first_line))
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

fn store_with_terminal() -> SqliteStore {
    let mut store = ok(SqliteStore::open_in_memory_migrated());
    ok(store.upsert_resources(&[resource()]));
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

fn real_dispatcher() -> speed_on_core::JsonIpcDispatcher<SqliteStore> {
    speed_on_core::JsonIpcDispatcher::new(CoreApi::new(store_with_terminal()))
}

#[derive(Default)]
struct RecordingRunner;

impl CommandRunner for RecordingRunner {
    fn run(&mut self, _plan: &CommandPlan) -> AppResult<()> {
        Ok(())
    }
}

fn command_opener_dispatcher(
) -> JsonIpcDispatcherWithOpener<SqliteStore, CommandResourceOpener<RecordingRunner>> {
    JsonIpcDispatcherWithOpener::new(
        CoreApi::new(store_with_terminal()),
        CommandResourceOpener::new(RecordingRunner),
    )
}

struct EchoDispatcher;

impl IpcDispatcher for EchoDispatcher {
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        json!({
            "protocol_version": IPC_PROTOCOL_VERSION,
            "request_id": request.request_id,
            "command": request.command,
            "response": {
                "ok": true,
                "data": request.payload,
                "error": null
            }
        })
    }
}

#[test]
fn stdio_config_reads_database_path_from_db_arg() {
    // 场景：前端启动 Core 子进程时，可以通过 --db 显式传入 SQLite 数据库路径。
    let config = ok(StdioConfig::from_args_and_env(
        ["--db", "speed-on.db"],
        None,
    ));

    assert_eq!(config.database_path.to_string_lossy(), "speed-on.db");
    assert!(!config.enable_command_opener);
}

#[test]
fn stdio_config_uses_environment_database_path_when_arg_is_missing() {
    // 场景：调试或打包环境可以通过 SPEED_ON_DB 提供数据库路径。
    let config = ok(StdioConfig::from_args_and_env(
        std::iter::empty::<&str>(),
        Some("env-speed-on.db".to_owned()),
    ));

    assert_eq!(config.database_path.to_string_lossy(), "env-speed-on.db");
    assert!(!config.enable_command_opener);
}

#[test]
fn stdio_config_requires_explicit_flag_to_enable_command_opener() {
    // 场景：真实平台 command opener 默认关闭，必须显式传 --enable-command-opener 才允许接入。
    let config = ok(StdioConfig::from_args_and_env(
        ["--enable-command-opener", "--db", "speed-on.db"],
        None,
    ));

    assert_eq!(config.database_path.to_string_lossy(), "speed-on.db");
    assert!(config.enable_command_opener);
}

#[test]
fn stdio_config_rejects_unknown_arguments() {
    // 场景：未知启动参数不能被静默忽略，避免前端以为某个安全开关已经生效。
    let result = StdioConfig::from_args_and_env(["--unknown", "--db", "speed-on.db"], None);

    let error = match result {
        Ok(_) => panic!("unknown argument should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "IPC_STDIO_INVALID_INPUT");
    assert_eq!(error.module, "ipc_stdio::StdioConfig");
}

#[test]
fn stdio_config_rejects_missing_database_path() {
    // 场景：没有数据库路径时，transport 不能静默创建未知位置的数据库。
    let result = StdioConfig::from_args_and_env(std::iter::empty::<&str>(), None);

    let error = match result {
        Ok(_) => panic!("missing database path should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "IPC_STDIO_INVALID_INPUT");
    assert_eq!(error.module, "ipc_stdio::StdioConfig");
}

#[test]
fn stdio_config_rejects_missing_db_argument_value() {
    // 场景：--db 后面必须是路径，不能把下一个 flag 当成数据库路径。
    let result = StdioConfig::from_args_and_env(["--db", "--enable-command-opener"], None);

    let error = match result {
        Ok(_) => panic!("missing --db value should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "IPC_STDIO_INVALID_INPUT");
    assert_eq!(error.module, "ipc_stdio::StdioConfig");
}

#[test]
fn json_lines_transport_writes_one_response_per_request_line() {
    // 场景：stdio transport 使用一行请求对应一行响应，方便前端按行读取。
    let input = br#"{"protocol_version":"speed-on-ipc-v1","request_id":"r1","command":"search","payload":{"query":"term","limit":5,"kinds":["application"],"now_millis":100}}
"#;
    let mut output = Vec::new();
    let mut dispatcher = EchoDispatcher;

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let response = parse_json_line(&output);
    assert_eq!(response["request_id"], json!("r1"));
    assert_eq!(response["command"], json!("search"));
    assert_eq!(response["response"]["ok"], json!(true));
}

#[test]
fn json_lines_transport_skips_empty_lines() {
    // 场景：前端或调试工具可能发送空行，transport 应跳过空行而不是返回伪错误。
    let input = br#"

{"protocol_version":"speed-on-ipc-v1","request_id":"r1","command":"recommend","payload":{"limit":5,"kinds":["application"],"now_millis":100}}
"#;
    let mut output = Vec::new();
    let mut dispatcher = EchoDispatcher;

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let text = match std::str::from_utf8(&output) {
        Ok(text) => text,
        Err(error) => panic!("output was not utf-8: {error}"),
    };
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn json_lines_transport_returns_transport_error_for_malformed_envelope() {
    // 场景：连 IPC envelope 都不是合法 JSON 时，transport 必须返回 transport-level 错误，不 panic。
    let input = b"not-json\n";
    let mut output = Vec::new();
    let mut dispatcher = EchoDispatcher;

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let response = parse_json_line(&output);
    assert_eq!(response["request_id"], json!(null));
    assert_eq!(response["command"], json!(null));
    assert_eq!(response["response"]["ok"], json!(false));
    assert_eq!(
        response["response"]["error"]["error_code"],
        json!("IPC_STDIO_MALFORMED_REQUEST")
    );
}

#[test]
fn json_lines_transport_can_drive_real_core_search() {
    // 场景：最小 transport 必须能承载真实 Core 搜索流程，而不只是 echo 测试。
    let input = br#"{"protocol_version":"speed-on-ipc-v1","request_id":"search-1","command":"search","payload":{"query":"term","limit":5,"kinds":["application"],"now_millis":100}}
"#;
    let mut output = Vec::new();
    let mut dispatcher = real_dispatcher();

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let response = parse_json_line(&output);
    assert_eq!(response["request_id"], json!("search-1"));
    assert_eq!(response["response"]["ok"], json!(true));
    assert_eq!(
        response["response"]["data"]["results"][0]["resource"]["id"],
        json!("app-terminal")
    );
}

#[test]
fn json_lines_transport_reports_open_resource_unsupported_when_command_opener_is_disabled() {
    // 场景：stdio binary 默认没有真实平台 opener，open_resource 必须返回 unsupported，不能假装打开成功。
    let input = br#"{"protocol_version":"speed-on-ipc-v1","request_id":"open-1","command":"open_resource","payload":{"resource":{"id":"app-terminal","kind":"application","title":"Terminal","target":"/apps/terminal","icon_path":"terminal.png"},"requested_at_millis":300}}
"#;
    let mut output = Vec::new();
    let mut dispatcher = real_dispatcher();

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let response = parse_json_line(&output);
    assert_eq!(response["request_id"], json!("open-1"));
    assert_eq!(response["response"]["ok"], json!(false));
    assert_eq!(
        response["response"]["error"]["error_code"],
        json!("CORE_PLATFORM_UNSUPPORTED")
    );
}

#[test]
fn json_lines_transport_can_drive_open_resource_when_command_opener_is_enabled() {
    // 场景：显式接入 command opener 后，open_resource 可通过 stdio transport 成功并记录 activity。
    let input = br#"{"protocol_version":"speed-on-ipc-v1","request_id":"open-1","command":"open_resource","payload":{"resource":{"id":"app-terminal","kind":"application","title":"Terminal","target":"/apps/terminal","icon_path":"terminal.png"},"requested_at_millis":300}}
"#;
    let mut output = Vec::new();
    let mut dispatcher = command_opener_dispatcher();

    ok(run_json_lines_transport(Cursor::new(input), &mut output, &mut dispatcher));

    let response = parse_json_line(&output);
    assert_eq!(response["request_id"], json!("open-1"));
    assert_eq!(response["response"]["ok"], json!(true));
    assert_eq!(response["response"]["data"]["opened"], json!(true));
    assert_eq!(response["response"]["data"]["activity_recorded"], json!(true));
}
