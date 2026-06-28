//! Speed-On backend core.
//!
//! This crate owns platform-independent domain logic only. Concrete Windows,
//! macOS, Linux, SQLite, and frontend bindings must be implemented behind the
//! ports defined here so that business rules stay testable and portable.

pub mod domain;
pub mod error;
pub mod ports;
pub mod service;
pub mod storage;

pub use domain::{
    ActivityRecord, CandidateResource, IndexedResource, Recommendation, RecommendationRequest,
    ResourceKind,
};
pub use error::{AppError, AppResult};
pub use ports::{
    BrowserHistoryReader, FileActivityReader, InstalledApplicationScanner, ResourceRepository,
};
pub use service::{IndexService, RecommendationService};
