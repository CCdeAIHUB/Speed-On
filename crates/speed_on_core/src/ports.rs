use crate::domain::{
    ActivityRecord, CandidateResource, IndexedResource, OpenResourceOutcome, OpenResourceRequest,
    ResourceKind,
};
use crate::error::AppResult;
use crate::logging::{SystemLogEntry, UserSearchLogEntry, UserSelectionLogEntry};
use crate::search::SearchCandidate;

/// Scans installed applications from the current desktop operating system.
///
/// Platform-specific implementations must stay behind this trait. This prevents
/// Windows registry, macOS bundle, or Linux desktop-entry details from leaking
/// into recommendation and indexing business logic.
pub trait InstalledApplicationScanner {
    fn scan_installed_applications(&self) -> AppResult<Vec<IndexedResource>>;
}

impl<T> InstalledApplicationScanner for &T
where
    T: InstalledApplicationScanner,
{
    fn scan_installed_applications(&self) -> AppResult<Vec<IndexedResource>> {
        (**self).scan_installed_applications()
    }
}

/// Reads browser history entries that can become recommendation resources.
///
/// Implementations must handle browser privacy, locked profile files, and user
/// permission boundaries explicitly. Returning success with missing data is not
/// allowed unless the user intentionally opted out of browser-history indexing.
pub trait BrowserHistoryReader {
    fn read_recent_browser_resources(&self, since_millis: u64) -> AppResult<Vec<IndexedResource>>;
}

/// Receives recently opened files and folders from platform-specific event APIs.
///
/// Real implementations may use OS logs, file-system watchers, or desktop
/// integration APIs, but those mechanisms must remain outside the domain layer.
pub trait FileActivityReader {
    fn read_recent_file_activity(&self, since_millis: u64) -> AppResult<Vec<ActivityRecord>>;
}

/// Opens an indexed resource through the current desktop operating system.
///
/// This is a high-risk platform boundary because it can launch applications,
/// reveal folders, open files, or navigate to browser URLs. Implementations must
/// live in platform adapters and must perform validation, permission checks, and
/// system-specific escaping before invoking OS APIs.
pub trait ResourceOpener {
    fn open_resource(&mut self, request: &OpenResourceRequest) -> AppResult<OpenResourceOutcome>;
}

/// Persistence boundary for indexed resources and usage signals.
///
/// SQLite is the first planned implementation, but core services depend on this
/// contract so that tests can use in-memory repositories and future migrations do
/// not rewrite business rules.
pub trait ResourceRepository {
    fn upsert_resources(&mut self, resources: &[IndexedResource]) -> AppResult<()>;

    fn record_activity(&mut self, activity: &ActivityRecord) -> AppResult<()>;

    fn load_recommendation_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<CandidateResource>>;
}

/// Query boundary for frontend search.
///
/// Implementations should load normalized aliases, browser title metadata, pinyin
/// aliases, usage counters, and user-selection signals from SQLite. The search
/// service consumes this prebuilt view and does not know how SQLite stores it.
pub trait SearchIndexRepository {
    fn load_search_candidates(&self, kinds: Option<&[ResourceKind]>) -> AppResult<Vec<SearchCandidate>>;
}

/// User operation log boundary.
///
/// User queries and final selections are intentionally separated from system
/// logs. They are product-behavior data and may contain sensitive user intent,
/// file paths, URLs, or document names, so storage implementations must apply
/// the user's privacy settings before persistence or export.
pub trait UserOperationLogRepository {
    fn record_user_search(&mut self, entry: &UserSearchLogEntry) -> AppResult<()>;

    fn record_user_selection(&mut self, entry: &UserSelectionLogEntry) -> AppResult<()>;
}

/// System runtime log boundary.
///
/// System logs should describe modules, stages, errors, timings, and sanitized
/// context summaries. They must not copy raw search queries, passwords, tokens,
/// private file contents, or full browser URLs unless a future privacy policy
/// explicitly allows it.
pub trait SystemLogSink {
    fn record_system_log(&mut self, entry: &SystemLogEntry) -> AppResult<()>;
}
