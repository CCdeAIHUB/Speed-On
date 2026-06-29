use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{
    ApiOpenResourceRequest, ApiRecommendationRequest, ApiRecordSelectionRequest, ApiResponse,
    ApiSearchRequest, CoreApi,
};
use crate::error::{AppError, AppResult};
use crate::ports::{
    ResourceOpener, ResourceRepository, SearchIndexRepository, UserOperationLogRepository,
};

pub const IPC_PROTOCOL_VERSION: &str = "speed-on-ipc-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcCommand {
    Search,
    Recommend,
    RecordSelection,
    OpenResource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcRequest {
    pub protocol_version: String,
    pub request_id: String,
    pub command: IpcCommand,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcResponse {
    pub protocol_version: String,
    pub request_id: String,
    pub command: IpcCommand,
    pub response: ApiResponse<Value>,
}

pub struct JsonIpcDispatcher<R>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
{
    api: CoreApi<R>,
}

impl<R> JsonIpcDispatcher<R>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
{
    pub fn new(api: CoreApi<R>) -> Self {
        Self { api }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        let response = self.dispatch_inner(&request);
        IpcResponse {
            protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
            request_id: request.request_id,
            command: request.command,
            response,
        }
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => match decode_payload::<ApiSearchRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcher::search",
            ) {
                Ok(payload) => api_response_to_json(self.api.search(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::Recommend => match decode_payload::<ApiRecommendationRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcher::recommend",
            ) {
                Ok(payload) => api_response_to_json(self.api.recommend(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::RecordSelection => match decode_payload::<ApiRecordSelectionRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcher::record_selection",
            ) {
                Ok(payload) => api_response_to_json(self.api.record_selection(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::OpenResource => ApiResponse::failure(AppError::platform_unsupported(
                "open_resource requires a platform ResourceOpener adapter",
                "ipc::JsonIpcDispatcher::open_resource",
            )),
        }
    }
}

pub struct JsonIpcDispatcherWithOpener<R, O>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
    O: ResourceOpener,
{
    api: CoreApi<R>,
    opener: O,
}

impl<R, O> JsonIpcDispatcherWithOpener<R, O>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
    O: ResourceOpener,
{
    pub fn new(api: CoreApi<R>, opener: O) -> Self {
        Self { api, opener }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        let response = self.dispatch_inner(&request);
        IpcResponse {
            protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
            request_id: request.request_id,
            command: request.command,
            response,
        }
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => match decode_payload::<ApiSearchRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithOpener::search",
            ) {
                Ok(payload) => api_response_to_json(self.api.search(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::Recommend => match decode_payload::<ApiRecommendationRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithOpener::recommend",
            ) {
                Ok(payload) => api_response_to_json(self.api.recommend(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::RecordSelection => match decode_payload::<ApiRecordSelectionRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithOpener::record_selection",
            ) {
                Ok(payload) => api_response_to_json(self.api.record_selection(payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::OpenResource => match decode_payload::<ApiOpenResourceRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithOpener::open_resource",
            ) {
                Ok(payload) => api_response_to_json(self.api.open_resource_with(&mut self.opener, payload)),
                Err(error) => ApiResponse::failure(error),
            },
        }
    }
}

fn validate_request(request: &IpcRequest) -> Option<AppError> {
    if request.protocol_version != IPC_PROTOCOL_VERSION {
        return Some(AppError::invalid_argument(
            format!(
                "unsupported IPC protocol version: {}",
                request.protocol_version
            ),
            "ipc::JsonIpcDispatcher",
        ));
    }

    if request.request_id.trim().is_empty() {
        return Some(AppError::invalid_argument(
            "IPC request_id must not be empty",
            "ipc::JsonIpcDispatcher",
        ));
    }

    None
}

fn decode_payload<T>(payload: Value, module: &'static str) -> AppResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(payload).map_err(|error| {
        AppError::invalid_argument("invalid IPC payload", module).with_cause(error.to_string())
    })
}

fn api_response_to_json<T>(response: ApiResponse<T>) -> ApiResponse<Value>
where
    T: Serialize,
{
    if let Some(error) = response.error {
        return ApiResponse {
            ok: false,
            data: None,
            error: Some(error),
        };
    }

    match response.data {
        Some(data) => match serde_json::to_value(data) {
            Ok(value) => ApiResponse::success(value),
            Err(error) => ApiResponse::failure(
                AppError::invalid_argument("failed to encode API response", "ipc::JsonIpcDispatcher")
                    .with_cause(error.to_string()),
            ),
        },
        None => ApiResponse::failure(AppError::invalid_argument(
            "successful API response must contain data",
            "ipc::JsonIpcDispatcher",
        )),
    }
}
