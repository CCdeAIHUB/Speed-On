//! Speed-On backend core.
//!
//! This crate owns platform-independent domain logic only. Concrete Windows,
//! macOS, Linux, SQLite, and frontend bindings must be implemented behind the
//! ports defined here so that business rules stay testable and portable.

pub mod alias;
pub mod api;
pub mod domain;
pub mod error;
pub mod ipc;
pub mod logging;
pub mod pinyin_alias;
pub mod ports;
pub mod search;
pub mod service;
pub mod storage;

pub use alias::{
    DefaultPinyinAliasProvider, PinyinAliasProvider, PinyinAliases, SearchAliasBuilder,
};
pub use api::{
    ApiErrorResponse, ApiOpenResourceRequest, ApiOpenResourceResponse, ApiRecommendationRequest,
    ApiRecommendationResponse, ApiRecommendationResult, ApiRecordSelectionRequest,
    ApiRecordSelectionResponse, ApiRefreshApplicationsRequest, ApiRefreshApplicationsResponse,
    ApiResource, ApiResourceKind, ApiResponse, ApiSearchMatchKind, ApiSearchRequest,
    ApiSearchResponse, ApiSearchResult, CoreApi, CORE_API_VERSION,
};
pub use domain::{
    ActivityRecord, CandidateResource, IndexedResource, OpenResourceOutcome, OpenResourceRequest,
    Recommendation, RecommendationRequest, ResourceKind,
};
pub use error::{AppError, AppResult};
pub use ipc::{
    IpcCommand, IpcRequest, IpcResponse, JsonIpcDispatcher, JsonIpcDispatcherWithOpener,
    JsonIpcDispatcherWithScanner, JsonIpcDispatcherWithScannerAndOpener, IPC_PROTOCOL_VERSION,
};
pub use logging::{LogLevel, SystemLogEntry, UserSearchLogEntry, UserSelectionLogEntry};
pub use pinyin_alias::PinyinCrateAliasProvider;
pub use ports::{
    BrowserHistoryReader, FileActivityReader, InstalledApplicationScanner, ResourceOpener,
    ResourceRepository, SearchAliasRepository, SearchIndexRepository, SystemLogSink,
    UserOperationLogRepository,
};
pub use search::{
    normalize_search_query, SearchAlias, SearchAliasKind, SearchCandidate, SearchMatchKind,
    SearchRequest, SearchResult, SearchService, UserSelectionSignal,
};
pub use service::{IndexService, RecommendationService};
pub use storage::SqliteStore;
