# ADR: JSON IPC transport contract

## 背景

Speed-On 的后端 Core 已经定义了稳定的 Core API v1 DTO，包括 `search`、`recommend` 和 `record_selection`。下一步需要让 Windows、macOS、Linux 原生前端能够调用这些 API。

真实传输方式仍需后续结合平台前端选择，例如 Windows named pipe、Unix domain socket、本地 HTTP、stdio JSON-RPC 或 FFI。但在确定具体传输实现之前，需要先固定一个与传输方式无关的 JSON IPC envelope，避免未来每个平台各自定义一套命令格式。

## 决策

1. 新增 `ipc` 模块，定义 `speed-on-ipc-v1`。
2. IPC 层只定义 JSON envelope 和命令分发，不实现具体 pipe/socket/http server。
3. IPC request 字段固定为：
   - `protocol_version`
   - `request_id`
   - `command`
   - `payload`
4. IPC response 字段固定为：
   - `protocol_version`
   - `request_id`
   - `command`
   - `response`
5. `response` 复用 Core API v1 的 `ApiResponse<Value>`，保持统一成功/失败结构。
6. 当前支持命令：
   - `search`
   - `recommend`
   - `record_selection`
7. `JsonIpcDispatcher` 只负责：
   - 校验 protocol_version；
   - 校验 request_id；
   - 解码 payload；
   - 调用 `CoreApi`；
   - 把 typed API response 转成 JSON response。
8. payload 解码失败必须返回结构化错误，不能 panic。
9. 不支持的 protocol version 必须返回结构化错误，不能静默接受。

## 原因

- 先固定 JSON envelope，可以让不同平台前端共享同一套命令格式。
- 将 envelope 与具体 transport 分离，后续可以在 named pipe、Unix socket、本地 HTTP 或 FFI 之间切换。
- 复用 Core API v1 DTO，避免 IPC 层重新定义搜索/推荐/选择记录字段。
- request_id 透传可以让前端并发请求时关联响应。
- version 校验为后续协议升级预留边界。

## 替代方案

### 方案 A：直接实现某个平台的 IPC

例如优先实现 Windows named pipe。短期可以跑通一个平台，但会让其他平台跟随 Windows 约束，且命令 envelope 容易在实现过程中临时成型。

### 方案 B：本地 HTTP server

开发和调试简单，但需要额外考虑端口冲突、防火墙、本地安全边界和跨进程访问控制。

### 方案 C：先固定 JSON IPC envelope 和 dispatcher

这是当前选择。它让平台传输层只负责收发 JSON，而 Core 负责稳定命令分发。

## 影响范围

- 新增 `ipc.rs`。
- `serde_json` 从测试依赖提升为运行时依赖。
- `lib.rs` 导出 IPC 类型。
- 新增 IPC contract 测试。
- 更新 API 文档和 README。

## 风险

- 当前还没有真实传输层，因此不能直接被前端进程调用。
- JSON payload 未来需要在具体 transport 层增加大小限制和超时控制。
- 目前命令只有 search/recommend/record_selection，打开资源、状态查询和日志查询需要后续扩展。
- 未来如果 IPC version 升级，需要保留兼容策略或迁移文档。

## 未来演进

1. 比较 Windows named pipe、Unix domain socket、本地 HTTP、stdio JSON-RPC 和 FFI 的实现成本。
2. 增加最小 transport：优先选择一个跨平台调试友好的方式。
3. 增加 `open_resource` IPC command。
4. 增加 `get_system_status` IPC command。
5. 增加 request timeout、payload size limit、trace id 和结构化系统日志。
6. 为各平台前端生成或维护对应 DTO 类型。
