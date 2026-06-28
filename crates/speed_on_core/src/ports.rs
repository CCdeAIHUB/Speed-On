use crate::domain::{ActivityRecord, CandidateResource, IndexedResource, ResourceKind};
use crate::error::AppResult;

/// Scans installed applications from the current desktop operating system.
///
/// Platform-specific implementations must stay behind this trait. This prevents
/// Windows registry, macOS bundle, or Linux desktop-entry details from leaking
/// into recommendation and indexing business logic.
pub trait InstalledApplicationScanner {
    fn scan_installed_applications(&self) -> AppResult<Vec<IndexedResource>>;
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
