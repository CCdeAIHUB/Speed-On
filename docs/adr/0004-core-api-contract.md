# ADR: Core API v1 contract

## 背景

Speed-On 已经具备后端 Core 的领域模型、搜索服务、推荐服务、SQLite repository、用户操作日志和系统日志。下一阶段需要让 Windows、macOS、Linux 原生前端可以稳定调用 Core，但目前还不应该直接实现 GUI 或具体 IPC server。

如果前端直接绑定内部 domain/service/storage 类型，后续内部重构会频繁破坏前端。因此需要先定义稳定的 API DTO 和统一响应结构。

## 决策

1. 新增 `api` 模块，作为前端/IPC 的稳定契约层。
2. API version 固定为 `core-api-v1`。
3. 所有 API 返回统一 `ApiResponse<T>`：
   - `ok`
   - `data`
   - `error`
4. 错误响应使用 `ApiErrorResponse`，不暴露内部 `cause` 字段。
5. 外部资源使用 `ApiResource`，不暴露内部 `source`、`first_seen_at_millis`、`last_seen_at_millis`。
6. 资源类型和搜索匹配类型使用 snake_case JSON 枚举。
7. 当前固定三个前端调用契约：
   - `search`
   - `recommend`
   - `record_selection`
8. 新增 `CoreApi<R>` facade，组合已有 `SearchService`、`RecommendationService` 和 repository traits。
9. 暂不实现 IPC/HTTP/FFI 传输层，只固定可序列化 DTO 和行为边界。
10. 使用 TDD 测试锁定 JSON 字段，防止后续无意改名。

## 原因

- API DTO 与内部模型隔离，可以允许 Core 内部演进。
- 统一响应结构可以简化前端错误处理。
- 不暴露 `cause` 可以减少路径、数据库、平台细节泄漏。
- 使用 snake_case 枚举便于不同原生平台解析。
- 先固定 contract，再实现 IPC，符合渐进式可验证开发流程。

## 替代方案

### 方案 A：前端直接调用内部 Rust domain/service 类型

短期代码少，但会把内部模型变成事实公共 API，后续重构成本高。

### 方案 B：先实现 IPC server，再补 DTO 测试

容易在传输层实现时临时决定字段名和错误结构，导致契约不稳定。

### 方案 C：先定义可序列化 API DTO 和测试

这是当前选择。它让前端、IPC、CLI 或本地 HTTP adapter 都可以共享同一套 contract。

## 影响范围

- 新增 `api.rs`。
- 新增 API contract 测试。
- 新增 API 文档。
- 新增 `serde` 和测试用 `serde_json` 依赖。
- `lib.rs` 导出 API 类型。

## 风险

- API DTO 和内部 domain 存在字段重复，需要维护转换逻辑。
- 当前还没有真实 IPC/HTTP/FFI 传输层。
- `CoreApi<R>` facade 目前只覆盖 search/recommend/record_selection，后续 API 需要单独扩展并测试。
- 前端传入的大 JSON payload 后续需要在传输层做大小限制。

## 未来演进

1. 增加 IPC transport ADR，决定 Windows/macOS/Linux 前端如何调用 Core。
2. 增加 `open_resource` API，统一打开应用、文件、文件夹、浏览器 URL。
3. 增加 `get_system_status` API，供前端展示索引状态、日志状态和数据库状态。
4. 增加 API schema 导出，供非 Rust 前端语言生成类型。
5. 增加 API 兼容性测试，锁定 JSON fixture。
