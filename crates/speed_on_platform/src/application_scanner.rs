use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use speed_on_core::{AppError, AppResult, IndexedResource, InstalledApplicationScanner, ResourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationScanRoots {
    pub os: String,
    pub roots: Vec<PathBuf>,
    pub now_millis: u64,
}

impl ApplicationScanRoots {
    pub fn for_current_platform(now_millis: u64) -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            roots: default_roots_for_os(std::env::consts::OS),
            now_millis,
        }
    }

    pub fn new(os: impl Into<String>, roots: Vec<PathBuf>, now_millis: u64) -> Self {
        Self {
            os: os.into(),
            roots,
            now_millis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformApplicationScanner {
    roots: ApplicationScanRoots,
}

impl PlatformApplicationScanner {
    pub fn new(roots: ApplicationScanRoots) -> Self {
        Self { roots }
    }

    pub fn for_current_platform(now_millis: u64) -> Self {
        Self::new(ApplicationScanRoots::for_current_platform(now_millis))
    }
}

impl InstalledApplicationScanner for PlatformApplicationScanner {
    fn scan_installed_applications(&self) -> AppResult<Vec<IndexedResource>> {
        scan_applications_from_roots(&self.roots)
    }
}

pub fn scan_applications_from_roots(roots: &ApplicationScanRoots) -> AppResult<Vec<IndexedResource>> {
    let mut resources = Vec::new();
    let mut seen_targets = HashSet::new();

    for root in &roots.roots {
        if !root.exists() {
            continue;
        }
        scan_root(root, roots, &mut seen_targets, &mut resources)?;
    }

    resources.sort_by(|left, right| left.title.cmp(&right.title).then(left.target.cmp(&right.target)));
    Ok(resources)
}

fn scan_root(
    root: &Path,
    roots: &ApplicationScanRoots,
    seen_targets: &mut HashSet<String>,
    resources: &mut Vec<IndexedResource>,
) -> AppResult<()> {
    let entries = fs::read_dir(root).map_err(|error| {
        AppError::platform_unsupported(
            format!("failed to read application scan root: {}", root.display()),
            "platform::ApplicationScanner",
        )
        .with_cause(error.to_string())
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::platform_unsupported(
                "failed to read application scan entry",
                "platform::ApplicationScanner",
            )
            .with_cause(error.to_string())
        })?;
        let path = entry.path();
        if path.is_dir() {
            if roots.os == "macos" && has_extension(&path, "app") {
                push_unique(make_macos_app_resource(&path, roots.now_millis), seen_targets, resources);
            } else {
                scan_root(&path, roots, seen_targets, resources)?;
            }
            continue;
        }

        if let Some(resource) = resource_from_file(&path, roots) {
            push_unique(resource, seen_targets, resources);
        }
    }

    Ok(())
}

fn resource_from_file(path: &Path, roots: &ApplicationScanRoots) -> Option<IndexedResource> {
    match roots.os.as_str() {
        "linux" if has_extension(path, "desktop") => parse_linux_desktop_entry(path, roots.now_millis),
        "windows" if has_extension(path, "lnk") || has_extension(path, "exe") => {
            Some(make_file_resource(path, "windows_app_file", roots.now_millis))
        }
        _ => None,
    }
}

fn parse_linux_desktop_entry(path: &Path, now_millis: u64) -> Option<IndexedResource> {
    let content = fs::read_to_string(path).ok()?;
    if desktop_value(&content, "NoDisplay").is_some_and(|value| value.eq_ignore_ascii_case("true")) {
        return None;
    }
    if desktop_value(&content, "Hidden").is_some_and(|value| value.eq_ignore_ascii_case("true")) {
        return None;
    }

    let title = desktop_value(&content, "Name").unwrap_or_else(|| title_from_path(path));
    let exec = desktop_value(&content, "Exec")
        .map(|value| clean_desktop_exec(&value))
        .filter(|value| !value.trim().is_empty())?;
    let icon_path = desktop_value(&content, "Icon");

    Some(IndexedResource {
        id: stable_application_id("linux_desktop", &exec),
        kind: ResourceKind::Application,
        title,
        target: exec,
        icon_path,
        source: "linux_desktop_entry".to_owned(),
        first_seen_at_millis: now_millis,
        last_seen_at_millis: now_millis,
    })
}

fn desktop_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        if let Some((line_key, value)) = trimmed.split_once('=') {
            if line_key == key {
                return Some(value.trim().to_owned());
            }
        }
    }
    None
}

fn clean_desktop_exec(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|part| !part.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}

fn make_macos_app_resource(path: &Path, now_millis: u64) -> IndexedResource {
    IndexedResource {
        id: stable_application_id("macos_app", &path.to_string_lossy()),
        kind: ResourceKind::Application,
        title: title_from_path(path).trim_end_matches(".app").to_owned(),
        target: path.to_string_lossy().to_string(),
        icon_path: None,
        source: "macos_app_bundle".to_owned(),
        first_seen_at_millis: now_millis,
        last_seen_at_millis: now_millis,
    }
}

fn make_file_resource(path: &Path, source: &str, now_millis: u64) -> IndexedResource {
    IndexedResource {
        id: stable_application_id(source, &path.to_string_lossy()),
        kind: ResourceKind::Application,
        title: title_from_path(path),
        target: path.to_string_lossy().to_string(),
        icon_path: None,
        source: source.to_owned(),
        first_seen_at_millis: now_millis,
        last_seen_at_millis: now_millis,
    }
}

fn push_unique(
    resource: IndexedResource,
    seen_targets: &mut HashSet<String>,
    resources: &mut Vec<IndexedResource>,
) {
    if seen_targets.insert(resource.target.clone()) {
        resources.push(resource);
    }
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn title_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Unknown Application")
        .to_owned()
}

fn stable_application_id(source: &str, target: &str) -> String {
    format!("app-{}-{:016x}", source, fnv1a64(target.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn default_roots_for_os(os: &str) -> Vec<PathBuf> {
    match os {
        "macos" => vec![PathBuf::from("/Applications"), home_join("Applications")],
        "linux" => vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            home_join(".local/share/applications"),
        ],
        "windows" => vec![
            env_path("ProgramData", "Microsoft/Windows/Start Menu/Programs"),
            env_path("APPDATA", "Microsoft/Windows/Start Menu/Programs"),
        ],
        _ => Vec::new(),
    }
    .into_iter()
    .filter(|path| !path.as_os_str().is_empty())
    .collect()
}

fn home_join(relative: &str) -> PathBuf {
    match std::env::var("HOME") {
        Ok(home) => PathBuf::from(home).join(relative),
        Err(_) => PathBuf::new(),
    }
}

fn env_path(var_name: &str, relative: &str) -> PathBuf {
    match std::env::var(var_name) {
        Ok(value) => PathBuf::from(value).join(relative),
        Err(_) => PathBuf::new(),
    }
}
