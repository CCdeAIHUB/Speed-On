use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{
    ApiOpenResourceRequest, ApiRecommendationRequest, ApiRecordSelectionRequest,
    ApiRefreshApplicationsRequest, ApiResponse, ApiSearchRequest, CoreApi,
};
use crate::error::{AppError, AppResult};
use crate::ports::{
    InstalledApplicationScanner, ResourceOpener, ResourceRepository, SearchAliasRepository,
    SearchIndexRepository, UserOperationLogRepository,
};

pub const IPC_PROTOCOL_VERSION: &str = "speed-on-ipc-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcCommand {
    Search,
    Recommend,
    RecordSelection,
    OpenResource,
    RefreshApplications,
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
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
{
    api: CoreApi<R>,
}

impl<R> JsonIpcDispatcher<R>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
{
    pub fn new(api: CoreApi<R>) -> Self {
        Self { api }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        build_response(&request, self.dispatch_inner(&request))
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => dispatch_search(&mut self.api, request, "ipc::JsonIpcDispatcher::search"),
            IpcCommand::Recommend => dispatch_recommend(&self.api, request, "ipc::JsonIpcDispatcher::recommend"),
            IpcCommand::RecordSelection => dispatch_record_selection(
                &mut self.api,
                request,
                "ipc::JsonIpcDispatcher::record_selection",
            ),
            IpcCommand::OpenResource => ApiResponse::failure(AppError::platform_unsupported(
                "open_resource requires a platform ResourceOpener adapter",
                "ipc::JsonIpcDispatcher::open_resource",
            )),
            IpcCommand::RefreshApplications => ApiResponse::failure(AppError::platform_unsupported(
                "refresh_applications requires a platform InstalledApplicationScanner adapter",
                "ipc::JsonIpcDispatcher::refresh_applications",
            )),
        }
    }
}

pub struct JsonIpcDispatcherWithOpener<R, O>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    O: ResourceOpener,
{
    api: CoreApi<R>,
    opener: O,
}

impl<R, O> JsonIpcDispatcherWithOpener<R, O>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    O: ResourceOpener,
{
    pub fn new(api: CoreApi<R>, opener: O) -> Self {
        Self { api, opener }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        build_response(&request, self.dispatch_inner(&request))
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => dispatch_search(&mut self.api, request, "ipc::JsonIpcDispatcherWithOpener::search"),
            IpcCommand::Recommend => dispatch_recommend(&self.api, request, "ipc::JsonIpcDispatcherWithOpener::recommend"),
            IpcCommand::RecordSelection => dispatch_record_selection(
                &mut self.api,
                request,
                "ipc::JsonIpcDispatcherWithOpener::record_selection",
            ),
            IpcCommand::OpenResource => match decode_payload::<ApiOpenResourceRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithOpener::open_resource",
            ) {
                Ok(payload) => api_response_to_json(self.api.open_resource_with(&mut self.opener, payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::RefreshApplications => ApiResponse::failure(AppError::platform_unsupported(
                "refresh_applications requires a platform InstalledApplicationScanner adapter",
                "ipc::JsonIpcDispatcherWithOpener::refresh_applications",
            )),
        }
    }
}

pub struct JsonIpcDispatcherWithScanner<R, S>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    S: InstalledApplicationScanner,
{
    api: CoreApi<R>,
    scanner: S,
}

impl<R, S> JsonIpcDispatcherWithScanner<R, S>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    S: InstalledApplicationScanner,
{
    pub fn new(api: CoreApi<R>, scanner: S) -> Self {
        Self { api, scanner }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        build_response(&request, self.dispatch_inner(&request))
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => dispatch_search(&mut self.api, request, "ipc::JsonIpcDispatcherWithScanner::search"),
            IpcCommand::Recommend => dispatch_recommend(&self.api, request, "ipc::JsonIpcDispatcherWithScanner::recommend"),
            IpcCommand::RecordSelection => dispatch_record_selection(
                &mut self.api,
                request,
                "ipc::JsonIpcDispatcherWithScanner::record_selection",
            ),
            IpcCommand::OpenResource => ApiResponse::failure(AppError::platform_unsupported(
                "open_resource requires a platform ResourceOpener adapter",
                "ipc::JsonIpcDispatcherWithScanner::open_resource",
            )),
            IpcCommand::RefreshApplications => match decode_payload::<ApiRefreshApplicationsRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithScanner::refresh_applications",
            ) {
                Ok(payload) => api_response_to_json(self.api.refresh_applications_with(&self.scanner, payload)),
                Err(error) => ApiResponse::failure(error),
            },
        }
    }
}

pub struct JsonIpcDispatcherWithScannerAndOpener<R, S, O>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    S: InstalledApplicationScanner,
    O: ResourceOpener,
{
    api: CoreApi<R>,
    scanner: S,
    opener: O,
}

impl<R, S, O> JsonIpcDispatcherWithScannerAndOpener<R, S, O>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
    S: InstalledApplicationScanner,
    O: ResourceOpener,
{
    pub fn new(api: CoreApi<R>, scanner: S, opener: O) -> Self {
        Self { api, scanner, opener }
    }

    pub fn dispatch(&mut self, request: IpcRequest) -> IpcResponse {
        build_response(&request, self.dispatch_inner(&request))
    }

    fn dispatch_inner(&mut self, request: &IpcRequest) -> ApiResponse<Value> {
        if let Some(error) = validate_request(request) {
            return ApiResponse::failure(error);
        }

        match request.command {
            IpcCommand::Search => dispatch_search(&mut self.api, request, "ipc::JsonIpcDispatcherWithScannerAndOpener::search"),
            IpcCommand::Recommend => dispatch_recommend(&self.api, request, "ipc::JsonIpcDispatcherWithScannerAndOpener::recommend"),
            IpcCommand::RecordSelection => dispatch_record_selection(
                &mut self.api,
                request,
                "ipc::JsonIpcDispatcherWithScannerAndOpener::record_selection",
            ),
            IpcCommand::OpenResource => match decode_payload::<ApiOpenResourceRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithScannerAndOpener::open_resource",
            ) {
                Ok(payload) => api_response_to_json(self.api.open_resource_with(&mut self.opener, payload)),
                Err(error) => ApiResponse::failure(error),
            },
            IpcCommand::RefreshApplications => match decode_payload::<ApiRefreshApplicationsRequest>(
                request.payload.clone(),
                "ipc::JsonIpcDispatcherWithScannerAndOpener::refresh_applications",
            ) {
                Ok(payload) => api_response_to_json(self.api.refresh_applications_with(&self.scanner, payload)),
                Err(error) => ApiResponse::failure(error),
            },
        }
    }
}

fn build_response(request: &IpcRequest, response: ApiResponse<Value>) -> IpcResponse {
    IpcResponse {
        protocol_version: IPC_PROTOCOL_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        command: request.command,
        response,
    }
}

fn dispatch_search<R>(api: &mut CoreApi<R>, request: &IpcRequest, module: &'static str) -> ApiResponse<Value>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
{
    match decode_payload::<ApiSearchRequest>(request.payload.clone(), module) {
        Ok(payload) => api_response_to_json(api.search(payload)),
        Err(error) => ApiResponse::failure(error),
    }
}

fn dispatch_recommend<R>(api: &CoreApi<R>, request: &IpcRequest, module: &'static str) -> ApiResponse<Value>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
{
    match decode_payload::<ApiRecommendationRequest>(request.payload.clone(), module) {
        Ok(payload) => api_response_to_json(api.recommend(payload)),
        Err(error) => ApiResponse::failure(error),
    }
}

fn dispatch_record_selection<R>(api: &mut CoreApi<R>, request: &IpcRequest, module: &'static str) -> ApiResponse<Value>
where
    R: ResourceRepository + SearchAliasRepository + SearchIndexRepository + UserOperationLogRepository,
{
    match decode_payload::<ApiRecordSelectionRequest>(request.payload.clone(), module) {
        Ok(payload) => api_response_to_json(api.record_selection(payload)),
        Err(error) => ApiResponse::failure(error),
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
