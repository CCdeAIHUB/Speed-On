// AppError is ~152 bytes due to rich diagnostic fields. See speed_on_core/lib.rs.
#![allow(clippy::result_large_err)]

pub mod application_scanner;
pub mod opener;

pub use application_scanner::{
    scan_applications_from_roots, ApplicationScanRoots, PlatformApplicationScanner,
};
pub use opener::{
    CommandPlan, CommandResourceOpener, CommandRunner, OpenTargetValidator, PlatformCommandPlanner,
    ProcessCommandRunner,
};
