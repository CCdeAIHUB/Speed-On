# ADR: open_resource contract and platform opener boundary

## 背景

Speed-On 前端已经可以通过 Core API / IPC 完成搜索、推荐和记录用户选择。下一步需要统一“打开资源”的能力：用户点击搜索结果后，系统应能打开应用、文件、文件夹或浏览器地址。

打开资源属于高风险平台行为。不同系统有不同实现方式：Windows 可能使用 ShellExecute 或相关系统 API，macOS 可能使用 NSWorkspace/open，Linux 可能使用 xdg-open、desktop entry 或桌面环境 API。若直接在 Core API 或 IPC 层写平台命令，会造成高耦合和安全边界不清晰。

## 决策

1. 新增 `OpenResourceRequest` 和 `OpenResourceOutcome` 领域模型。
2. 新增 `ResourceOpener` trait，作为平台打开能力边界。
3. 新增 `ApiOpenResourceRequest` 和 `ApiOpenResourceResponse`。
4. 新增 `CoreApi::open_resource_with`，显式接收 `ResourceOpener`。
5. 新增 IPC command：`open_resource`。
6. 默认 `JsonIpcDispatcher` 没有平台 opener，收到 `open_resource` 时返回 `CORE_PLATFORM_UNSUPPORTED`。
7. 新增 `JsonIpcDispatcherWithOpener`，用于后续平台 adapter 或测试注入 opener。
8. `speed-on-ipc-stdio` 默认不接入真实平台 opener；显式传 `--enable-command-opener` 时接入 command opener。
9. 真实平台 opener 必须作为 Windows/macOS/Linux adapter 单独实现和测试。
10. opener 成功后，Core 会写入 `activity_records` 并更新 `resource_usage_stats`。
11. 若 opener 成功但 activity 写入失败，API 返回结构化错误，不沉默隐藏数据记录失败。

## 原因

- 打开资源会启动应用、打开文件、显示文件夹或访问 URL，必须隔离权限和平台差异。
- API 层只应表达用户意图，不应直接执行系统命令。
- 默认 unsupported 比假装成功更安全。
- `JsonIpcDispatcherWithOpener` 允许后续平台 transport 接入真实 opener，同时保持当前 dispatcher 可用于无 opener 场景。
- TDD 测试可以验证契约和错误路径，防止 open_resource 被误接成沉默成功。
- 打开成功后写入 activity/usage stats，可以让推荐系统学习用户的真实打开行为。

## 替代方案

### 方案 A：直接在 Core API 里用系统命令打开

短期实现简单，但会把平台行为、路径 escaping、URL 验证和权限策略混进 Core，后续难以维护。

### 方案 B：让前端自己打开资源

前端可以直接调用平台 API，但会导致 Windows/macOS/Linux 三套前端重复实现打开规则，且难以统一日志、权限和错误码。

### 方案 C：Core 定义契约，平台 adapter 实现 opener，并由 Core 统一记录 activity

这是当前选择。Core 统一 API、错误结构和打开后的使用统计，真实平台 opener 独立实现。

## 影响范围

- 扩展 `domain.rs`。
- 扩展 `ports.rs`。
- 扩展 `api.rs`。
- 扩展 `ipc.rs`。
- 扩展 `lib.rs` 导出。
- 扩展 API、IPC、stdio transport 测试。
- 更新 API 和 IPC 文档。

## 风险

- command opener 只有在显式启用时才会执行真实系统打开行为。
- 后续平台 opener 必须处理路径 escaping、URL scheme 白名单、权限提示、目标不存在、应用不存在等问题。
- 浏览器 URL 打开可能涉及隐私和安全风险，后续应增加 scheme 校验和用户设置。
- 文件/文件夹打开可能暴露敏感路径，系统日志必须避免记录完整隐私路径。
- 打开资源是不可回滚行为；如果打开成功后 activity 写入失败，调用方会收到结构化错误提示。

## 未来演进

1. 增加 Windows opener adapter。
2. 增加 macOS opener adapter。
3. 增加 Linux opener adapter。
4. 增加 open_resource 权限策略和用户开关。
5. 增加目标存在性校验和 URL scheme 白名单配置。
6. 将 opener 错误写入脱敏 system_logs。
