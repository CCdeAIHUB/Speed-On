use std::fmt;

use crate::domain::ResourceKind;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl TryFrom<&str> for LogLevel {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, AppError> {
        match value {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(AppError::invalid_argument(
                format!("unknown log level: {value}"),
                "logging::LogLevel",
            )),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSearchLogEntry {
    pub id: String,
    pub raw_query: String,
    pub normalized_query: String,
    pub result_count: usize,
    pub searched_at_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSelectionLogEntry {
    pub id: String,
    pub raw_query: String,
    pub normalized_query: String,
    pub selected_resource_id: String,
    pub selected_kind: ResourceKind,
    pub selected_title: String,
    pub selected_target: String,
    pub selected_rank: usize,
    pub opened_at_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemLogEntry {
    pub id: String,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
    pub context_summary: Option<String>,
    pub trace_id: Option<String>,
    pub occurred_at_millis: u64,
}

impl SystemLogEntry {
    pub fn new(
        id: impl Into<String>,
        level: LogLevel,
        module: impl Into<String>,
        message: impl Into<String>,
        occurred_at_millis: u64,
    ) -> Self {
        Self {
            id: id.into(),
            level,
            module: module.into(),
            message: message.into(),
            context_summary: None,
            trace_id: None,
            occurred_at_millis,
        }
    }

    pub fn with_context_summary(mut self, context_summary: impl Into<String>) -> Self {
        self.context_summary = Some(context_summary.into());
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}
