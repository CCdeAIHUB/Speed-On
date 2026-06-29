# ADR: installed application scanner and refresh_applications

## 背景

Speed-On 的核心目标之一是在安装或首次运行时检索电脑上已安装的软件，整理启动路径和图标信息，写入 SQLite，并为搜索/推荐建立索引。

此前 Core 已经有 `InstalledApplicationScanner` port 和 `IndexService`，但还没有前端可触发的 API / IPC 命令，也没有第一版平台 scanner adapter。应用扫描属于平台差异逻辑：macOS、Linux、Windows 的应用发现位置和元数据格式都不同，不能直接写进 Core API 或 IPC。

## 决策

1. 新增 Core API DTO：`ApiRefreshApplicationsRequest` / `ApiRefreshApplicationsResponse`。
2. 新增 `CoreApi::refresh_applications_with`，显式接收 `InstalledApplicationScanner`。
3. 新增 IPC command：`refresh_applications`。
4. 默认 `JsonIpcDispatcher` 不带 scanner，收到 `refresh_applications` 时返回 `CORE_PLATFORM_UNSUPPORTED`。
5. 新增 `JsonIpcDispatcherWithScanner`，用于只接入应用扫描能力。
6. 新增 `JsonIpcDispatcherWithScannerAndOpener`，用于同时接入应用扫描和资源打开能力。
7. `speed_on_platform` 新增 `PlatformApplicationScanner`。
8. 第一版扫描策略：
   - macOS：扫描 `.app` bundle 目录；
   - Linux：扫描 `.desktop` 文件，读取 `Name`、`Exec`、`Icon`，跳过 `NoDisplay=true` / `Hidden=true`；
   - Windows：扫描 `.lnk` 和 `.exe` 文件，第一版不解析 `.lnk` 内部格式。
9. `speed-on-ipc-stdio` 默认不启用真实应用扫描；必须显式传 `--enable-application-scan` 才接入 `PlatformApplicationScanner`。
10. 扫描结果通过既有 `IndexService` upsert 到 SQLite，Core API 不复制索引写入逻辑。

## 原因

- Core 只依赖抽象 scanner，保持平台扫描逻辑隔离。
- API / IPC 增加 `refresh_applications` 后，前端可以在安装、首次启动或用户手动刷新时触发应用索引重建。
- stdio 显式 opt-in 避免调试或早期集成时意外扫描用户系统目录。
- 第一版 scanner 不引入第三方依赖，先建立稳定边界和测试保护。
- `.lnk`、`.desktop`、`.app` 的深度解析可以后续按平台独立增强。

## 替代方案

### 方案 A：直接在 Core API 中扫描系统目录

短期实现简单，但会把 macOS/Linux/Windows 文件系统规则混进 Core，破坏边界。

### 方案 B：先只实现 Linux desktop entry scanner

可以更快，但无法支撑跨平台目标。当前选择至少为三大平台建立第一版策略。

### 方案 C：平台 scanner adapter + refresh_applications contract

这是当前选择。它把触发契约、索引写入和平台扫描拆开，便于测试和后续演进。

## 影响范围

- 扩展 `api.rs`。
- 扩展 `ipc.rs`。
- 扩展 `lib.rs` 导出。
- 扩展 `speed_on_ipc_stdio` 启动参数和 dispatcher 模式。
- 新增 `speed_on_platform::application_scanner`。
- 新增 API、IPC、stdio、platform scanner 测试。
- 更新 README、API 文档和 stdio 文档。

## 风险

- Windows `.lnk` 第一版只把 shortcut 文件本身作为 target，不解析真实目标。
- macOS 第一版不解析 `Info.plist`，标题来自 `.app` bundle 名。
- Linux 第一版只做保守 `.desktop` 解析，Exec 字段清理能力有限。
- 图标路径第一版只记录原始字符串或为空，不复制或规范化图标资源。
- 应用扫描可能触碰用户系统目录，必须保持显式启用和后续权限策略。

## 未来演进

1. 增加 Windows `.lnk` 解析或原生 shell API scanner。
2. 增加 macOS `Info.plist` 解析，读取更准确的显示名和 icon。
3. 增加 Linux desktop entry locale name、TryExec、OnlyShowIn / NotShowIn 规则。
4. 增加图标资源规范化和缓存。
5. 增加扫描耗时、扫描数量、失败原因的脱敏 system log。
6. 增加首次运行自动扫描策略和用户隐私开关。
7. 增加扫描后的搜索 alias / pinyin alias builder。
