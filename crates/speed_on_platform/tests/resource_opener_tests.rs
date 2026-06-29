use speed_on_core::{
    AppError, AppResult, IndexedResource, OpenResourceOutcome, OpenResourceRequest, ResourceKind,
    ResourceOpener,
};
use speed_on_platform::{
    opener::plan_for_platform, CommandPlan, CommandResourceOpener, CommandRunner,
    OpenTargetValidator, PlatformCommandPlanner,
};

fn ok<T>(result: AppResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("operation failed unexpectedly: {error}"),
    }
}

fn resource(kind: ResourceKind, target: &str) -> IndexedResource {
    IndexedResource {
        id: "resource-1".to_owned(),
        kind,
        title: "Resource".to_owned(),
        target: target.to_owned(),
        icon_path: None,
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 2,
    }
}

fn request(kind: ResourceKind, target: &str) -> OpenResourceRequest {
    OpenResourceRequest::new(resource(kind, target), 300)
}

#[derive(Debug, Default)]
struct RecordingRunner {
    fail: bool,
    plans: Vec<CommandPlan>,
}

impl RecordingRunner {
    fn failing() -> Self {
        Self {
            fail: true,
            plans: Vec::new(),
        }
    }
}

impl CommandRunner for RecordingRunner {
    fn run(&mut self, plan: &CommandPlan) -> AppResult<()> {
        if self.fail {
            return Err(AppError::platform_unsupported(
                "recording runner failed",
                "tests::RecordingRunner",
            ));
        }

        self.plans.push(plan.clone());
        Ok(())
    }
}

#[test]
fn validator_rejects_empty_target() {
    // 场景：打开资源不能接收空 target，否则平台 adapter 可能打开未知默认位置。
    let error = match OpenTargetValidator::validate(&request(ResourceKind::File, "   ")) {
        Ok(()) => panic!("empty target should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "platform::OpenTargetValidator");
}

#[test]
fn validator_rejects_control_characters() {
    // 场景：target 内含控制字符时必须拒绝，避免跨平台命令参数边界异常。
    let error = match OpenTargetValidator::validate(&request(ResourceKind::File, "/tmp/a\nb")) {
        Ok(()) => panic!("control character target should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "platform::OpenTargetValidator");
}

#[test]
fn validator_allows_http_https_and_file_browser_urls() {
    // 场景：浏览器地址第一版只允许 http、https 和 file，禁止任意 scheme。
    ok(OpenTargetValidator::validate(&request(
        ResourceKind::BrowserUrl,
        "https://example.test",
    )));
    ok(OpenTargetValidator::validate(&request(
        ResourceKind::BrowserUrl,
        "http://example.test",
    )));
    ok(OpenTargetValidator::validate(&request(
        ResourceKind::BrowserUrl,
        "file:///tmp/example.html",
    )));
}

#[test]
fn validator_rejects_dangerous_browser_url_schemes() {
    // 场景：javascript/data 等 scheme 不能被浏览器 URL 打开入口接受。
    for target in ["javascript:alert(1)", "data:text/html,hello", "ftp://example.test"] {
        let error = match OpenTargetValidator::validate(&request(ResourceKind::BrowserUrl, target)) {
            Ok(()) => panic!("dangerous browser URL scheme should fail"),
            Err(error) => error,
        };
        assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
        assert_eq!(error.module, "platform::OpenTargetValidator");
    }
}

#[test]
fn command_planner_uses_os_specific_open_commands() {
    // 场景：第一版平台 opener 使用无 shell 拼接的系统打开命令计划。
    assert_eq!(
        plan_for_platform("macos", "/apps/terminal"),
        CommandPlan::new("open", vec!["/apps/terminal".to_owned()])
    );
    assert_eq!(
        plan_for_platform("linux", "/apps/terminal"),
        CommandPlan::new("xdg-open", vec!["/apps/terminal".to_owned()])
    );
    assert_eq!(
        plan_for_platform("windows", "C:\\Apps\\Terminal.exe"),
        CommandPlan::new("explorer", vec!["C:\\Apps\\Terminal.exe".to_owned()])
    );
}

#[test]
fn platform_command_planner_validates_before_planning() {
    // 场景：planner 不能绕过 validator，非法 URL 必须在生成命令计划前失败。
    let error = match PlatformCommandPlanner::plan(&request(
        ResourceKind::BrowserUrl,
        "javascript:alert(1)",
    )) {
        Ok(_) => panic!("invalid browser URL should not produce a command plan"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
}

#[test]
fn command_resource_opener_runs_planned_command_and_returns_outcome() {
    // 场景：opener 成功执行 runner 后，必须返回统一 OpenResourceOutcome。
    let runner = RecordingRunner::default();
    let mut opener = CommandResourceOpener::new(runner);
    let outcome: OpenResourceOutcome = ok(opener.open_resource(&request(ResourceKind::File, "/tmp/file.txt")));

    assert_eq!(outcome.resource_id, "resource-1");
    assert_eq!(outcome.kind, ResourceKind::File);
    assert_eq!(outcome.target, "/tmp/file.txt");
    assert_eq!(outcome.opened_at_millis, 300);
}

#[test]
fn command_resource_opener_returns_runner_error() {
    // 场景：底层平台命令启动失败时，opener 必须返回结构化错误，不能假装成功。
    let runner = RecordingRunner::failing();
    let mut opener = CommandResourceOpener::new(runner);
    let error = match opener.open_resource(&request(ResourceKind::File, "/tmp/file.txt")) {
        Ok(_) => panic!("runner failure should fail opener"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_PLATFORM_UNSUPPORTED");
    assert_eq!(error.module, "tests::RecordingRunner");
}
