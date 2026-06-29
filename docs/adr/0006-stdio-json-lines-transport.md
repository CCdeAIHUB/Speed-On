# ADR: stdio JSON Lines IPC transport

## 背景

Speed-On 已经定义了 Core API v1 和 `speed-on-ipc-v1` JSON envelope。下一步需要一个最小可运行的 IPC transport，让前端或调试工具可以真实发送请求并接收响应。

最终桌面版本可能会使用 Windows Named Pipe、macOS/Linux Unix Domain Socket、本地 HTTP 或 FFI。但这些选择涉及平台差异、权限、安全边界、打包方式和生命周期管理。为了先跑通跨平台调用链，需要一个不绑定具体桌面系统 API 的 transport。

## 决策

1. 新增 `speed_on_ipc_stdio` crate。
2. 新增二进制 `speed-on-ipc-stdio`。
3. 使用 stdio JSON Lines 作为最小可运行 transport：
   - stdin 每一行是一个 `IpcRequest` JSON；
   - stdout 每一行是一个 `IpcResponse` JSON；
   - 空行会被跳过。
4. 数据库路径通过 `--db <path>` 或 `SPEED_ON_DB` 指定。
5. transport 使用 `JsonIpcDispatcher` / `JsonIpcDispatcherWithOpener`，不复制 search/recommend/record_selection/open_resource 业务逻辑。
6. envelope 无法解析时，返回 transport-level 错误对象，避免 panic。
7. 该 transport 作为跨平台调试和最小集成路径，不阻止后续增加 Named Pipe / Unix Socket / HTTP adapter。
8. stdio 默认不启用真实 command opener；只有显式传入 `--enable-command-opener` 时才接入 `speed_on_platform::CommandResourceOpener`。
9. 未知启动参数会返回结构化错误，避免前端误以为某个安全开关已生效。

## 原因

- stdio 是 Windows、macOS、Linux 都可用的进程通信基础能力。
- 前端可以先以子进程方式启动 Core，避免端口、防火墙和平台 socket 差异。
- JSON Lines 容易调试，也便于脚本或测试工具直接发送请求。
- 保持 transport 层薄，业务逻辑仍然在 Core API / dispatcher 中。
- 最小可运行 transport 能让后续前端更早验证 search/recommend/record_selection/open_resource 流程。
- command opener 是真实本机动作，必须显式启用，不能默认开启。

## 替代方案

### 方案 A：Windows Named Pipe + Unix Domain Socket

这是长期更接近桌面原生的方案，但需要分别实现平台差异、权限、路径、生命周期和测试。

### 方案 B：本地 HTTP server

便于调试，但会引入端口冲突、本地访问控制、防火墙和 CSRF/跨进程访问问题。

### 方案 C：先做 stdio JSON Lines + command opener opt-in

这是当前选择。它最小、跨平台、易测试，默认安全关闭真实打开动作，并且不会阻止后续替换为更原生的 IPC transport。

## 影响范围

- Workspace 新增 `crates/speed_on_ipc_stdio`。
- 新增 `run_json_lines_transport`。
- 新增 `speed-on-ipc-stdio` binary。
- 新增 `--enable-command-opener`。
- 新增 stdio transport tests。
- 更新 README 和 API/IPC 文档。

## 风险

- stdio transport 需要前端负责启动和管理 Core 子进程。
- 如果前端进程崩溃，Core 子进程生命周期需要后续管理策略。
- JSON Lines 不适合高频大流量数据流，但当前 search/recommend/record_selection/open_resource 足够。
- malformed envelope 无法提取 request_id/command，只能返回 transport-level error。
- 启用 command opener 后会触发真实系统打开行为，前端必须只在用户明确操作后发送 open_resource。

## 未来演进

1. 为前端启动 Core 子进程定义生命周期管理策略。
2. 增加 request timeout 和 payload size limit。
3. 增加 trace id。
4. 增加 Windows Named Pipe transport。
5. 增加 macOS/Linux Unix Domain Socket transport。
6. 评估是否保留 stdio 作为 debug/dev transport。
7. 增加权限开关和首次打开确认策略。
