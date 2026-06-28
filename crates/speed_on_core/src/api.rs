use serde::{Deserialize, Serialize};

use crate::domain::{IndexedResource, Recommendation, RecommendationRequest, ResourceKind};
use crate::error::AppError;
use crate::ports::{ResourceRepository, SearchIndexRepository, UserOperationLogRepository};
use crate::search::{SearchMatchKind, SearchRequest, SearchResult, SearchService};
use crate::service::RecommendationService;

pub const CORE_API_VERSION: &str = "core-api-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorResponse>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: AppError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ApiErrorResponse::from(error)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error_code: String,
    pub message: String,
    pub module: String,
    pub recoverable: bool,
    pub suggestion: Option<String>,
    pub trace_id: Option<String>,
}

impl From<AppError> for ApiErrorResponse {
    fn from(error: AppError) -> Self {
        // `cause` is intentionally not exposed to the frontend contract because
        // storage, platform, and IPC causes may contain paths or implementation
        // details. System logs can store sanitized diagnostic summaries instead.
        Self {
            error_code: error.error_code,
            message: error.message,
            module: error.module,
            recoverable: error.recoverable,
            suggestion: error.suggestion,
            trace_id: error.trace_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResource {
    pub id: String,
    pub kind: ApiResourceKind,
    pub title: String,
    pub target: String,
    pub icon_path: Option<String>,
}

impl From<IndexedResource> for ApiResource {
    fn from(resource: IndexedResource) -> Self {
        Self {
            id: resource.id,
            kind: ApiResourceKind::from(resource.kind),
            title: resource.title,
            target: resource.target,
            icon_path: resource.icon_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiResourceKind {
    Application,
    File,
    Folder,
    BrowserUrl,
}

impl From<ResourceKind> for ApiResourceKind {
    fn from(kind: ResourceKind) -> Self {
        match kind {
            ResourceKind::Application => Self::Application,
            ResourceKind::File => Self::File,
            ResourceKind::Folder => Self::Folder,
            ResourceKind::BrowserUrl => Self::BrowserUrl,
        }
    }
}

impl From<ApiResourceKind> for ResourceKind {
    fn from(kind: ApiResourceKind) -> Self {
        match kind {
            ApiResourceKind::Application => Self::Application,
            ApiResourceKind::File => Self::File,
            ApiResourceKind::Folder => Self::Folder,
            ApiResourceKind::BrowserUrl => Self::BrowserUrl,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSearchRequest {
    pub query: String,
    pub limit: usize,
    pub kinds: Option<Vec<ApiResourceKind>>,
    pub now_millis: u64,
}

impl From<ApiSearchRequest> for SearchRequest {
    fn from(request: ApiSearchRequest) -> Self {
        let mut search_request = SearchRequest::new(request.query, request.limit, request.now_millis);
        if let Some(kinds) = request.kinds {
            search_request = search_request.with_kinds(
                kinds.into_iter().map(ResourceKind::from).collect::<Vec<_>>(),
            );
        }
        search_request
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSearchResponse {
    pub api_version: String,
    pub results: Vec<ApiSearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSearchResult {
    pub resource: ApiResource,
    pub score: u64,
    pub match_kind: ApiSearchMatchKind,
    pub reason: String,
}

impl From<SearchResult> for ApiSearchResult {
    fn from(result: SearchResult) -> Self {
        Self {
            resource: ApiResource::from(result.resource),
            score: result.score,
            match_kind: ApiSearchMatchKind::from(result.match_kind),
            reason: result.reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiSearchMatchKind {
    UserHistory,
    Title,
    Target,
    BrowserTitle,
    PinyinFull,
    PinyinInitials,
    CustomAlias,
}

impl From<SearchMatchKind> for ApiSearchMatchKind {
    fn from(kind: SearchMatchKind) -> Self {
        match kind {
            SearchMatchKind::UserHistory => Self::UserHistory,
            SearchMatchKind::Title => Self::Title,
            SearchMatchKind::Target => Self::Target,
            SearchMatchKind::BrowserTitle => Self::BrowserTitle,
            SearchMatchKind::PinyinFull => Self::PinyinFull,
            SearchMatchKind::PinyinInitials => Self::PinyinInitials,
            SearchMatchKind::CustomAlias => Self::CustomAlias,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRecommendationRequest {
    pub limit: usize,
    pub kinds: Option<Vec<ApiResourceKind>>,
    pub now_millis: u64,
}

impl From<ApiRecommendationRequest> for RecommendationRequest {
    fn from(request: ApiRecommendationRequest) -> Self {
        let mut recommendation_request = RecommendationRequest::new(request.limit, request.now_millis);
        if let Some(kinds) = request.kinds {
            recommendation_request = recommendation_request.with_kinds(
                kinds.into_iter().map(ResourceKind::from).collect::<Vec<_>>(),
            );
        }
        recommendation_request
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRecommendationResponse {
    pub api_version: String,
    pub results: Vec<ApiRecommendationResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRecommendationResult {
    pub resource: ApiResource,
    pub score: u64,
    pub reason: String,
}

impl From<Recommendation> for ApiRecommendationResult {
    fn from(recommendation: Recommendation) -> Self {
        Self {
            resource: ApiResource::from(recommendation.resource),
            score: recommendation.score,
            reason: recommendation.reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRecordSelectionRequest {
    pub query: String,
    pub selected_resource: ApiResource,
    pub selected_rank: usize,
    pub opened_at_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRecordSelectionResponse {
    pub api_version: String,
    pub recorded: bool,
}

pub struct CoreApi<R>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
{
    repository: R,
}

impl<R> CoreApi<R>
where
    R: ResourceRepository + SearchIndexRepository + UserOperationLogRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn search(&mut self, request: ApiSearchRequest) -> ApiResponse<ApiSearchResponse> {
        let mut service = SearchService::new(&mut self.repository);
        match service.search(SearchRequest::from(request)) {
            Ok(results) => ApiResponse::success(ApiSearchResponse {
                api_version: CORE_API_VERSION.to_owned(),
                results: results.into_iter().map(ApiSearchResult::from).collect(),
            }),
            Err(error) => ApiResponse::failure(error),
        }
    }

    pub fn recommend(
        &self,
        request: ApiRecommendationRequest,
    ) -> ApiResponse<ApiRecommendationResponse> {
        let service = RecommendationService::new(&self.repository);
        match service.recommend(RecommendationRequest::from(request)) {
            Ok(results) => ApiResponse::success(ApiRecommendationResponse {
                api_version: CORE_API_VERSION.to_owned(),
                results: results
                    .into_iter()
                    .map(ApiRecommendationResult::from)
                    .collect(),
            }),
            Err(error) => ApiResponse::failure(error),
        }
    }

    pub fn record_selection(
        &mut self,
        request: ApiRecordSelectionRequest,
    ) -> ApiResponse<ApiRecordSelectionResponse> {
        let selected_resource = IndexedResource {
            id: request.selected_resource.id,
            kind: ResourceKind::from(request.selected_resource.kind),
            title: request.selected_resource.title,
            target: request.selected_resource.target,
            icon_path: request.selected_resource.icon_path,
            source: "api_selection".to_owned(),
            first_seen_at_millis: request.opened_at_millis,
            last_seen_at_millis: request.opened_at_millis,
        };
        let mut service = SearchService::new(&mut self.repository);

        match service.record_selection(
            request.query,
            &selected_resource,
            request.selected_rank,
            request.opened_at_millis,
        ) {
            Ok(()) => ApiResponse::success(ApiRecordSelectionResponse {
                api_version: CORE_API_VERSION.to_owned(),
                recorded: true,
            }),
            Err(error) => ApiResponse::failure(error),
        }
    }
}

impl<T> ResourceRepository for &mut T
where
    T: ResourceRepository,
{
    fn upsert_resources(&mut self, resources: &[IndexedResource]) -> crate::error::AppResult<()> {
        (**self).upsert_resources(resources)
    }

    fn record_activity(&mut self, activity: &crate::domain::ActivityRecord) -> crate::error::AppResult<()> {
        (**self).record_activity(activity)
    }

    fn load_recommendation_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> crate::error::AppResult<Vec<crate::domain::CandidateResource>> {
        (**self).load_recommendation_candidates(kinds)
    }
}

impl<T> SearchIndexRepository for &mut T
where
    T: SearchIndexRepository,
{
    fn load_search_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> crate::error::AppResult<Vec<crate::search::SearchCandidate>> {
        (**self).load_search_candidates(kinds)
    }
}

impl<T> UserOperationLogRepository for &mut T
where
    T: UserOperationLogRepository,
{
    fn record_user_search(&mut self, entry: &crate::logging::UserSearchLogEntry) -> crate::error::AppResult<()> {
        (**self).record_user_search(entry)
    }

    fn record_user_selection(&mut self, entry: &crate::logging::UserSelectionLogEntry) -> crate::error::AppResult<()> {
        (**self).record_user_selection(entry)
    }
}

impl<T> ResourceRepository for &T
where
    T: ResourceRepository,
{
    fn upsert_resources(&mut self, _resources: &[IndexedResource]) -> crate::error::AppResult<()> {
        Err(AppError::invalid_argument(
            "cannot write through shared repository reference",
            "api::CoreApi",
        ))
    }

    fn record_activity(&mut self, _activity: &crate::domain::ActivityRecord) -> crate::error::AppResult<()> {
        Err(AppError::invalid_argument(
            "cannot write through shared repository reference",
            "api::CoreApi",
        ))
    }

    fn load_recommendation_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> crate::error::AppResult<Vec<crate::domain::CandidateResource>> {
        (**self).load_recommendation_candidates(kinds)
    }
}
