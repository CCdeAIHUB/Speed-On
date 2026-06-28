//! Speed-On backend core.
//!
//! This crate owns platform-independent domain logic only. Concrete Windows,
//! macOS, Linux, SQLite, and frontend bindings must be implemented behind the
//! ports defined here so that business rules stay testable and portable.

pub mod api;
pub mod domain;
pub mod error;
pub mod logging;
pub mod ports;
pub mod search;
pub mod service;
pub mod storage;

pub use api::{
    ApiErrorResponse, ApiRecommendationRequest, ApiRecommendationResponse,
    ApiRecommendationResult, ApiRecordSelectionRequest, ApiRecordSelectionResponse, ApiResource,
    ApiResourceKind, ApiResponse, ApiSearchMatchKind, ApiSearchRequest, ApiSearchResponse,
    ApiSearchResult, CoreApi, CORE_API_VERSION,
};
pub use domain::{
    ActivityRecord, CandidateResource, IndexedResource, Recommendation, RecommendationRequest,
    ResourceKind,
};
pub use error::{AppError, AppResult};
pub use logging::{LogLevel, SystemLogEntry, UserSearchLogEntry, UserSelectionLogEntry};
pub use ports::{
    BrowserHistoryReader, FileActivityReader, InstalledApplicationScanner, ResourceRepository,
    SearchIndexRepository, SystemLogSink, UserOperationLogRepository,
};
pub use search::{
    normalize_search_query, SearchAlias, SearchAliasKind, SearchCandidate, SearchMatchKind,
    SearchRequest, SearchResult, SearchService, UserSelectionSignal,
};
pub use service::{IndexService, RecommendationService};
pub use storage::SqliteStore;
