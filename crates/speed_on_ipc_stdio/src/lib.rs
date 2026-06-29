// AppError is ~152 bytes; see speed_on_core/lib.rs for rationale.
#![allow(clippy::result_large_err)]

use std::fmt;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use speed_on_core::{
    CoreApi, InstalledApplicationScanner, IpcRequest, JsonIpcDispatcher,
    JsonIpcDispatcherWithOpener, JsonIpcDispatcherWithScanner,
    JsonIpcDispatcherWithScannerAndOpener, ResourceOpener, SqliteStore, IPC_PROTOCOL_VERSION,
};
use speed_on_platform::{CommandResourceOpener, PlatformApplicationScanner, ProcessCommandRunner};

pub type CommandOpenerDispatcher =
    JsonIpcDispatcherWithOpener<SqliteStore, CommandResourceOpener<ProcessCommandRunner>>;
pub type ApplicationScannerDispatcher =
    JsonIpcDispatcherWithScanner<SqliteStore, PlatformApplicationScanner>;
pub type FullPlatformDispatcher = JsonIpcDispatcherWithScannerAndOpener<
    SqliteStore,
    PlatformApplicationScanner,
    CommandResourceOpener<ProcessCommandRunner>,
>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdioTransportError {
    pub ok: bool,
    pub error_code: String,
    pub message: String,
    pub module: String,
    pub recoverable: bool,
}

impl StdioTransportError {
    pub fn invalid_input(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            ok: false,
            error_code: "IPC_STDIO_INVALID_INPUT".to_owned(),
            message: message.into(),
            module: module.into(),
            recoverable: true,
        }
    }

    pub fn io_failure(message: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            ok: false,
            error_code: "IPC_STDIO_IO_FAILURE".to_owned(),
            message: message.into(),
            module: module.into(),
            recoverable: true,
        }
    }
}

pub type StdioResult<T> = Result<T, StdioTransportError>;

impl fmt::Display for StdioTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_code, self.message)
    }
}

impl std::error::Error for StdioTransportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioConfig {
    pub database_path: PathBuf,
    pub enable_command_opener: bool,
    pub enable_application_scan: bool,
}

impl StdioConfig {
    pub fn from_args_and_env<I>(args: I, database_env: Option<String>) -> StdioResult<Self>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        let mut database_path = None;
        let mut enable_command_opener = false;
        let mut enable_application_scan = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--db" => {
                    database_path = Some(read_database_path_arg(args.next())?);
                }
                "--enable-command-opener" => {
                    enable_command_opener = true;
                }
                "--enable-application-scan" => {
                    enable_application_scan = true;
                }
                _ => {
                    return Err(StdioTransportError::invalid_input(
                        format!("unknown argument: {arg}"),
                        "ipc_stdio::StdioConfig",
                    ));
                }
            }
        }

        let database_path = match database_path.or(database_env) {
            Some(path) if !path.trim().is_empty() => path,
            _ => {
                return Err(StdioTransportError::invalid_input(
                    "database path is required; pass --db <path> or set SPEED_ON_DB",
                    "ipc_stdio::StdioConfig",
                ));
            }
        };

        Ok(Self {
            database_path: PathBuf::from(database_path),
            enable_command_opener,
            enable_application_scan,
        })
    }
}

fn read_database_path_arg(value: Option<String>) -> StdioResult<String> {
    match value {
        Some(path) if !path.trim().is_empty() && !path.starts_with("--") => Ok(path),
        _ => Err(StdioTransportError::invalid_input(
            "--db requires a non-empty database path value",
            "ipc_stdio::StdioConfig",
        )),
    }
}

pub fn open_default_dispatcher(
    config: &StdioConfig,
) -> StdioResult<JsonIpcDispatcher<SqliteStore>> {
    let store = open_store(config, "ipc_stdio::open_default_dispatcher")?;
    Ok(JsonIpcDispatcher::new(CoreApi::new(store)))
}

pub fn open_command_opener_dispatcher(
    config: &StdioConfig,
) -> StdioResult<CommandOpenerDispatcher> {
    let store = open_store(config, "ipc_stdio::open_command_opener_dispatcher")?;
    Ok(JsonIpcDispatcherWithOpener::new(
        CoreApi::new(store),
        CommandResourceOpener::default(),
    ))
}

pub fn open_application_scanner_dispatcher(
    config: &StdioConfig,
) -> StdioResult<ApplicationScannerDispatcher> {
    let store = open_store(config, "ipc_stdio::open_application_scanner_dispatcher")?;
    Ok(JsonIpcDispatcherWithScanner::new(
        CoreApi::new(store),
        PlatformApplicationScanner::for_current_platform(current_millis()?),
    ))
}

pub fn open_full_platform_dispatcher(config: &StdioConfig) -> StdioResult<FullPlatformDispatcher> {
    let store = open_store(config, "ipc_stdio::open_full_platform_dispatcher")?;
    Ok(JsonIpcDispatcherWithScannerAndOpener::new(
        CoreApi::new(store),
        PlatformApplicationScanner::for_current_platform(current_millis()?),
        CommandResourceOpener::default(),
    ))
}

fn current_millis() -> StdioResult<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            StdioTransportError::io_failure(
                format!("failed to read system time: {error}"),
                "ipc_stdio::current_millis",
            )
        })?;
    Ok(duration.as_millis() as u64)
}

fn open_store(config: &StdioConfig, module: &'static str) -> StdioResult<SqliteStore> {
    SqliteStore::open_migrated(&config.database_path).map_err(|error| {
        StdioTransportError::io_failure(format!("failed to open sqlite database: {error}"), module)
    })
}

pub fn run_json_lines_transport<R, W, D>(
    reader: R,
    mut writer: W,
    dispatcher: &mut D,
) -> StdioResult<()>
where
    R: BufRead,
    W: Write,
    D: IpcDispatcher,
{
    for line in reader.lines() {
        let line = line.map_err(|error| {
            StdioTransportError::io_failure(
                format!("failed to read IPC line: {error}"),
                "ipc_stdio::run_json_lines_transport",
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = dispatch_line(trimmed, dispatcher);
        let serialized = serde_json::to_string(&response).map_err(|error| {
            StdioTransportError::io_failure(
                format!("failed to encode IPC response: {error}"),
                "ipc_stdio::run_json_lines_transport",
            )
        })?;
        writeln!(writer, "{serialized}").map_err(|error| {
            StdioTransportError::io_failure(
                format!("failed to write IPC response: {error}"),
                "ipc_stdio::run_json_lines_transport",
            )
        })?;
        writer.flush().map_err(|error| {
            StdioTransportError::io_failure(
                format!("failed to flush IPC response: {error}"),
                "ipc_stdio::run_json_lines_transport",
            )
        })?;
    }

    Ok(())
}

pub trait IpcDispatcher {
    fn dispatch_request(&mut self, request: IpcRequest) -> Value;
}

impl<R> IpcDispatcher for JsonIpcDispatcher<R>
where
    R: speed_on_core::ResourceRepository
        + speed_on_core::SearchAliasRepository
        + speed_on_core::SearchIndexRepository
        + speed_on_core::UserOperationLogRepository,
{
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        dispatch_to_value(self.dispatch(request))
    }
}

impl<R, O> IpcDispatcher for JsonIpcDispatcherWithOpener<R, O>
where
    R: speed_on_core::ResourceRepository
        + speed_on_core::SearchAliasRepository
        + speed_on_core::SearchIndexRepository
        + speed_on_core::UserOperationLogRepository,
    O: ResourceOpener,
{
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        dispatch_to_value(self.dispatch(request))
    }
}

impl<R, S> IpcDispatcher for JsonIpcDispatcherWithScanner<R, S>
where
    R: speed_on_core::ResourceRepository
        + speed_on_core::SearchAliasRepository
        + speed_on_core::SearchIndexRepository
        + speed_on_core::UserOperationLogRepository,
    S: InstalledApplicationScanner,
{
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        dispatch_to_value(self.dispatch(request))
    }
}

impl<R, S, O> IpcDispatcher for JsonIpcDispatcherWithScannerAndOpener<R, S, O>
where
    R: speed_on_core::ResourceRepository
        + speed_on_core::SearchAliasRepository
        + speed_on_core::SearchIndexRepository
        + speed_on_core::UserOperationLogRepository,
    S: InstalledApplicationScanner,
    O: ResourceOpener,
{
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        dispatch_to_value(self.dispatch(request))
    }
}

fn dispatch_to_value<T>(response: T) -> Value
where
    T: serde::Serialize,
{
    match serde_json::to_value(response) {
        Ok(value) => value,
        Err(error) => {
            malformed_envelope_error(format!("failed to encode dispatch response: {error}"))
        }
    }
}

fn dispatch_line<D>(line: &str, dispatcher: &mut D) -> Value
where
    D: IpcDispatcher,
{
    match serde_json::from_str::<IpcRequest>(line) {
        Ok(request) => dispatcher.dispatch_request(request),
        Err(error) => malformed_envelope_error(format!("invalid IPC request envelope: {error}")),
    }
}

fn malformed_envelope_error(message: String) -> Value {
    // This transport-level response is only used when the JSON envelope cannot
    // be decoded at all, so request_id and command may be unavailable. Valid IPC
    // requests still receive normal `IpcResponse` objects from the core dispatcher.
    serde_json::json!({
        "protocol_version": IPC_PROTOCOL_VERSION,
        "request_id": null,
        "command": null,
        "response": {
            "ok": false,
            "data": null,
            "error": {
                "error_code": "IPC_STDIO_MALFORMED_REQUEST",
                "message": message,
                "module": "ipc_stdio::run_json_lines_transport",
                "recoverable": true,
                "suggestion": null,
                "trace_id": null
            }
        }
    })
}
