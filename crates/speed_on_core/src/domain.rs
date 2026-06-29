use std::fmt;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Application,
    File,
    Folder,
    BrowserUrl,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::File => "file",
            Self::Folder => "folder",
            Self::BrowserUrl => "browser_url",
        }
    }
}

impl TryFrom<&str> for ResourceKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, AppError> {
        match value {
            "application" => Ok(Self::Application),
            "file" => Ok(Self::File),
            "folder" => Ok(Self::Folder),
            "browser_url" => Ok(Self::BrowserUrl),
            _ => Err(AppError::invalid_argument(
                format!("unknown resource kind: {value}"),
                "domain::ResourceKind",
            )),
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedResource {
    pub id: String,
    pub kind: ResourceKind,
    pub title: String,
    pub target: String,
    pub icon_path: Option<String>,
    pub source: String,
    pub first_seen_at_millis: u64,
    pub last_seen_at_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRecord {
    pub id: String,
    pub resource_id: Option<String>,
    pub kind: ResourceKind,
    pub target: String,
    pub opened_at_millis: u64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateResource {
    pub resource: IndexedResource,
    pub open_count: u64,
    pub last_opened_at_millis: Option<u64>,
}

impl CandidateResource {
    pub fn new(
        resource: IndexedResource,
        open_count: u64,
        last_opened_at_millis: Option<u64>,
    ) -> Self {
        Self {
            resource,
            open_count,
            last_opened_at_millis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecommendationRequest {
    pub limit: usize,
    pub kinds: Option<Vec<ResourceKind>>,
    pub now_millis: u64,
}

impl RecommendationRequest {
    pub fn new(limit: usize, now_millis: u64) -> Self {
        Self {
            limit,
            kinds: None,
            now_millis,
        }
    }

    pub fn with_kinds(mut self, kinds: Vec<ResourceKind>) -> Self {
        self.kinds = Some(kinds);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub resource: IndexedResource,
    pub score: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenResourceRequest {
    pub resource: IndexedResource,
    pub requested_at_millis: u64,
}

impl OpenResourceRequest {
    pub fn new(resource: IndexedResource, requested_at_millis: u64) -> Self {
        Self {
            resource,
            requested_at_millis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenResourceOutcome {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub target: String,
    pub opened_at_millis: u64,
}
