use std::fmt;

/// Unified error structure for all critical backend failure paths.
///
/// The core must never hide platform, schema, permission, or storage failures as
/// success. Frontends and adapters can rely on this shape to decide whether a
/// failure is recoverable and how it should be displayed to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    pub error_code: String,
    pub message: String,
    pub module: String,
    pub recoverable: bool,
    pub cause: Option<String>,
    pub suggestion: Option<String>,
    pub trace_id: Option<String>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(
        error_code: impl Into<String>,
        message: impl Into<String>,
        module: impl Into<String>,
        recoverable: bool,
    ) -> Self {
        Self {
            error_code: error_code.into(),
            message: message.into(),
            module: module.into(),
            recoverable,
            cause: None,
            suggestion: None,
            trace_id: None,
        }
    }

    pub fn invalid_argument(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self::new("CORE_INVALID_ARGUMENT", message, module, true)
    }

    pub fn storage_failure(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self::new("CORE_STORAGE_FAILURE", message, module, true)
    }

    pub fn platform_unsupported(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self::new("CORE_PLATFORM_UNSUPPORTED", message, module, false)
    }

    /// Platform command was recognized and attempted but failed at runtime.
    ///
    /// Unlike `platform_unsupported` (which means the platform cannot do this
    /// at all), `platform_failure` means the platform *can* do it but the
    /// command exited non-zero or similar. These errors are recoverable.
    pub fn platform_failure(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self::new("CORE_PLATFORM_FAILURE", message, module, true)
    }

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} in {}: {}",
            self.error_code, self.module, self.message
        )
    }
}

impl std::error::Error for AppError {}
