use speed_on_core::{LogLevel, SystemLogEntry};

#[test]
fn system_log_entry_keeps_sanitized_runtime_context() {
    // 场景：系统日志用于记录运行期错误和诊断信息，只保存模块、级别、消息、脱敏上下文和 trace id。
    let log = SystemLogEntry::new(
        "log-1",
        LogLevel::Error,
        "search::SearchService",
        "failed to load search candidates",
        1_000_000,
    )
    .with_context_summary("candidate_count=unknown; storage=sqlite")
    .with_trace_id("trace-1");

    assert_eq!(log.id, "log-1");
    assert_eq!(log.level, LogLevel::Error);
    assert_eq!(log.module, "search::SearchService");
    assert_eq!(log.message, "failed to load search candidates");
    assert_eq!(log.context_summary.as_deref(), Some("candidate_count=unknown; storage=sqlite"));
    assert_eq!(log.trace_id.as_deref(), Some("trace-1"));
}

#[test]
fn log_level_rejects_unknown_values() {
    // 场景：系统日志级别必须是稳定枚举，未知级别不能静默降级成 info。
    let result = LogLevel::try_from("verbose");

    let error = match result {
        Ok(_) => panic!("unknown log level should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "logging::LogLevel");
}
