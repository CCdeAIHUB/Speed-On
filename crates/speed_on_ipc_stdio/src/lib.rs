use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use speed_on_core::{CoreApi, IpcRequest, JsonIpcDispatcher, SqliteStore, IPC_PROTOCOL_VERSION};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioConfig {
    pub database_path: PathBuf,
}

impl StdioConfig {
    pub fn from_args_and_env<I>(args: I, database_env: Option<String>) -> StdioResult<Self>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        let mut database_path = None;

        while let Some(arg) = args.next() {
            if arg == "--db" {
                database_path = args.next();
                break;
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
        })
    }
}

pub fn open_default_dispatcher(config: &StdioConfig) -> StdioResult<JsonIpcDispatcher<SqliteStore>> {
    let store = SqliteStore::open_migrated(&config.database_path).map_err(|error| {
        StdioTransportError::io_failure(
            format!("failed to open sqlite database: {error}"),
            "ipc_stdio::open_default_dispatcher",
        )
    })?;

    Ok(JsonIpcDispatcher::new(CoreApi::new(store)))
}

pub fn run_json_lines_transport<R, W, D>(reader: R, mut writer: W, dispatcher: &mut D) -> StdioResult<()>
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
        + speed_on_core::SearchIndexRepository
        + speed_on_core::UserOperationLogRepository,
{
    fn dispatch_request(&mut self, request: IpcRequest) -> Value {
        match serde_json::to_value(self.dispatch(request)) {
            Ok(value) => value,
            Err(error) => malformed_envelope_error(format!("failed to encode dispatch response: {error}")),
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
