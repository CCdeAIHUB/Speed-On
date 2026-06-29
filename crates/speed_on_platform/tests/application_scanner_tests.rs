use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use speed_on_core::{InstalledApplicationScanner, ResourceKind};
use speed_on_platform::{
    scan_applications_from_roots, ApplicationScanRoots, PlatformApplicationScanner,
};

fn ok<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("operation failed unexpectedly: {error}"),
    }
}

fn test_root(name: &str) -> PathBuf {
    let millis = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis(),
        Err(error) => panic!("system time failed unexpectedly: {error}"),
    };
    let path = std::env::temp_dir().join(format!("speed-on-{name}-{millis}"));
    ok(fs::create_dir_all(&path));
    path
}

#[test]
fn linux_desktop_scanner_reads_visible_desktop_entries() {
    // 场景：Linux 应用扫描第一版读取 .desktop 的 Name、Exec、Icon，并清理 Exec 参数占位符。
    let root = test_root("linux-desktop");
    let desktop_path = root.join("code.desktop");
    ok(fs::write(
        &desktop_path,
        "[Desktop Entry]\nName=Code\nExec=/usr/bin/code %U\nIcon=code\n",
    ));

    let roots = ApplicationScanRoots::new("linux", vec![root.clone()], 500);
    let resources = ok(scan_applications_from_roots(&roots));

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].kind, ResourceKind::Application);
    assert_eq!(resources[0].title, "Code");
    assert_eq!(resources[0].target, "/usr/bin/code");
    assert_eq!(resources[0].icon_path, Some("code".to_owned()));
    assert_eq!(resources[0].source, "linux_desktop_entry");
    assert_eq!(resources[0].first_seen_at_millis, 500);

    ok(fs::remove_dir_all(root));
}

#[test]
fn linux_desktop_scanner_skips_hidden_entries() {
    // 场景：NoDisplay/Hidden 的 desktop entry 不应进入推荐索引。
    let root = test_root("linux-hidden");
    ok(fs::write(
        root.join("hidden.desktop"),
        "[Desktop Entry]\nName=Hidden App\nExec=/usr/bin/hidden\nNoDisplay=true\n",
    ));
    ok(fs::write(
        root.join("visible.desktop"),
        "[Desktop Entry]\nName=Visible App\nExec=/usr/bin/visible\n",
    ));

    let resources = ok(scan_applications_from_roots(&ApplicationScanRoots::new(
        "linux",
        vec![root.clone()],
        500,
    )));

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].title, "Visible App");

    ok(fs::remove_dir_all(root));
}

#[test]
fn macos_scanner_reads_app_bundles() {
    // 场景：macOS 第一版扫描 .app bundle 目录，并以 bundle 路径作为 launch target。
    let root = test_root("macos-app");
    let app_dir = root.join("Notes.app");
    ok(fs::create_dir_all(&app_dir));

    let resources = ok(scan_applications_from_roots(&ApplicationScanRoots::new(
        "macos",
        vec![root.clone()],
        600,
    )));

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].title, "Notes");
    assert_eq!(resources[0].target, app_dir.to_string_lossy().to_string());
    assert_eq!(resources[0].source, "macos_app_bundle");

    ok(fs::remove_dir_all(root));
}

#[test]
fn windows_scanner_reads_lnk_and_exe_files() {
    // 场景：Windows 第一版扫描 Start Menu / roots 下的 .lnk 和 .exe 文件，不解析 .lnk 内部格式。
    let root = test_root("windows-app");
    ok(fs::write(root.join("Terminal.lnk"), "shortcut"));
    ok(fs::write(root.join("Tool.exe"), "binary"));

    let resources = ok(scan_applications_from_roots(&ApplicationScanRoots::new(
        "windows",
        vec![root.clone()],
        700,
    )));

    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].kind, ResourceKind::Application);
    assert!(resources.iter().any(|resource| resource.title == "Terminal.lnk"));
    assert!(resources.iter().any(|resource| resource.title == "Tool.exe"));

    ok(fs::remove_dir_all(root));
}

#[test]
fn platform_application_scanner_implements_core_scanner_port() {
    // 场景：platform scanner 必须实现 InstalledApplicationScanner，供 Core API / IPC 注入。
    let root = test_root("scanner-port");
    ok(fs::write(
        root.join("app.desktop"),
        "[Desktop Entry]\nName=Scanner App\nExec=/usr/bin/scanner-app\n",
    ));
    let scanner = PlatformApplicationScanner::new(ApplicationScanRoots::new(
        "linux",
        vec![root.clone()],
        800,
    ));

    let resources = ok(scanner.scan_installed_applications());

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].title, "Scanner App");

    ok(fs::remove_dir_all(root));
}
