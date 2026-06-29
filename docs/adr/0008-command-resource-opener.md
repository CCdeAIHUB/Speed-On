# ADR: command-based ResourceOpener adapter

## 背景

Speed-On 已经定义了 `open_resource` Core API / IPC 契约，并通过 `ResourceOpener` trait 把真实平台打开行为隔离在平台 adapter 边界外。下一步需要一个第一版 opener adapter，用于后续 stdio、Named Pipe、Unix Socket 或原生前端集成。

打开资源会启动应用、打开文件/文件夹或访问浏览器 URL，属于高风险操作。它必须验证 target，限制 URL scheme，并避免通过 shell 拼接命令。

## 决策

1. 新增 `speed_on_platform` crate。
2. 新增 `OpenTargetValidator`，负责共享校验：
   - target 不能为空；
   - target 不能包含控制字符；
   - browser URL 只允许 `http://`、`https://`、`file://`。
3. 新增 `CommandPlan`，表达将要执行的程序和参数。
4. 新增 `PlatformCommandPlanner`，根据当前 OS 生成命令计划。
5. 第一版平台命令策略：
   - macOS：`open <target>`；
   - Linux：`xdg-open <target>`；
   - Windows：`explorer <target>`。
6. 新增 `CommandRunner` trait，用于将实际命令执行与 planner/opener 测试隔离。
7. 新增 `ProcessCommandRunner`，使用 `std::process::Command` 执行命令，参数以 args 传入，不使用 shell 字符串拼接。
8. 新增 `CommandResourceOpener<R>`，组合 validator、planner 和 runner，实现 Core 的 `ResourceOpener`。
9. 测试只使用 mock runner，不真实打开系统应用、文件、文件夹或 URL。
10. `CoreApi::open_resource_with` 在 opener 成功后写入 `activity_records` 并更新 `resource_usage_stats`。
11. `speed-on-ipc-stdio` 默认不启用真实 command opener；只有显式传入 `--enable-command-opener` 时才接入 `CommandResourceOpener<ProcessCommandRunner>`。

## 原因

- 独立 crate 可以避免平台打开逻辑污染 `speed_on_core`。
- validator 先于 planner 执行，可以提前阻断明显危险或非法的 target。
- URL scheme 白名单能避免 `javascript:`、`data:` 等不应由 opener 直接执行的地址。
- 不使用 shell 拼接可以降低命令注入风险。
- `CommandRunner` 让测试只验证计划和调用，不触发真实 OS 行为。
- 命令型 opener 是第一版通用方案，后续可以逐步替换成更原生的 Windows/macOS/Linux API。
- 打开成功后记录 activity/usage stats，可以让推荐系统学习用户真实打开行为。
- stdio 中真实打开能力显式 opt-in，避免调试或前端早期集成阶段意外启动本机资源。

## 替代方案

### 方案 A：直接在 `speed_on_core` 中实现打开逻辑

短期最简单，但会让 Core 依赖平台行为，破坏当前架构边界。

### 方案 B：先实现每个平台的原生 API

长期更理想，但需要分别处理 Windows、macOS、Linux API、权限、错误映射和测试环境。当前阶段先用命令型 adapter 建立可测试边界。

### 方案 C：命令型 opener + runner 注入 + stdio 显式启用

这是当前选择。它简单、跨平台、可测试、默认安全关闭，并且保持后续替换空间。

## 影响范围

- Workspace 新增 `crates/speed_on_platform`。
- 新增 target validator、command planner、runner 和 command opener。
- `speed_on_ipc_stdio` 新增 `--enable-command-opener`。
- `open_resource` 成功后会写入 activity/usage stats。
- 新增/更新 opener、API、IPC、stdio transport tests。
- README 更新为 Stage 9。

## 风险

- `open` / `xdg-open` / `explorer` 是通用命令，不是最终最强平台集成方案。
- Linux 环境可能缺少 `xdg-open` 或桌面环境不可用。
- Windows 使用 `explorer` 打开应用/文件/URL 的行为可能需要后续细分。
- URL whitelist 当前只允许 http/https/file，后续是否支持其他 scheme 必须通过单独 ADR 评估。
- 打开资源是不可回滚的系统行为；若打开成功但 activity 写入失败，Core 会返回结构化错误以避免沉默失败。

## 未来演进

1. 增加打开失败后的脱敏 system log。
2. 实现 Windows 原生 opener adapter。
3. 实现 macOS 原生 opener adapter。
4. 实现 Linux desktop-aware opener adapter。
5. 增加路径存在性校验、应用 bundle 校验和 URL scheme 配置。
6. 增加用户权限开关和首次打开确认策略。
