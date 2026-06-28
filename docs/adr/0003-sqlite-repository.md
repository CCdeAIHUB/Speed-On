# ADR: SQLite repository implementation

## 背景

Speed-On 已经定义了资源索引、活动记录、搜索别名、用户操作日志、系统日志和推荐/搜索聚合 schema。为了让后端 Core 从契约阶段进入可运行阶段，需要实现真实 SQLite repository 和 migration runner。

项目目标平台是 Windows、macOS、Linux，并覆盖 x86_64 与 ARM64/aarch64。SQLite 层必须保持在 storage 模块内，不能反向污染 domain、search、service 或平台 adapter。

## 决策

1. 使用 `rusqlite` 实现 SQLite repository。
2. 在 Core crate 中启用 `rusqlite` 的 `bundled` feature，降低不同系统 SQLite 版本差异。
3. 新增 `SqliteStore`，集中实现：
   - `ResourceRepository`
   - `SearchIndexRepository`
   - `UserOperationLogRepository`
   - `SystemLogSink`
4. 新增 migration runner：执行 `schema::MIGRATIONS`，并通过 `PRAGMA user_version` 写入当前 schema version。
5. 所有 SQLite 错误统一转换成 `AppError::storage_failure`，保留 cause 字符串，禁止吞掉错误。
6. 所有写入使用参数绑定，禁止拼接用户输入进 SQL。
7. 用户搜索日志、用户选择日志、系统日志继续分表存储。
8. 用户选择日志写入时同步更新 `query_resource_selection_stats`，供下一次搜索排序使用。

## 原因

- `rusqlite` 是成熟的 SQLite Rust 封装，足够适合当前本地桌面数据库需求。
- `bundled` 可减少用户机器 SQLite 版本差异导致的问题。
- `SqliteStore` 实现 ports trait，可以保持业务服务不依赖 SQLite 细节。
- 使用 `PRAGMA user_version` 可以给后续迁移提供明确版本锚点。
- 参数绑定能降低 SQL 注入和特殊字符破坏 SQL 的风险。

## 替代方案

### 方案 A：继续只保留 schema，不实现 repository

风险是后续平台扫描和前端 API 无法真实落库，项目无法进入可运行阶段。

### 方案 B：使用更重的 ORM

ORM 可以提供更完整抽象，但当前项目只需要可控的本地 SQLite 读写。引入 ORM 会增加依赖复杂度和 schema 控制成本。

### 方案 C：直接使用底层 SQLite FFI

可以减少封装层，但会增加 unsafe 边界和维护成本，不符合当前稳定工程目标。

## 影响范围

- 新增 `rusqlite` 依赖。
- 新增 `storage/sqlite.rs`。
- 扩展 `storage/mod.rs` 和 `lib.rs` 导出。
- 新增 SQLite persistence 测试。
- README 更新为 Stage 3。

## 风险

- `bundled` 会增加编译时间和产物体积。
- 当前没有真实迁移历史表，只有 `PRAGMA user_version`；后续复杂迁移可能需要增加 migrations 元数据表。
- 当前搜索候选加载会按资源加载 aliases/signals，后续大量数据时需要优化为批量 join 或 FTS 索引。
- 当前还没有用户日志保留周期和隐私开关。

## 未来演进

1. 增加正式 migration metadata 表，记录每个 migration 的执行时间和 checksum。
2. 增加 SQLite FTS5 或自定义搜索索引评估。
3. 增加 pinyin alias builder。
4. 增加用户隐私设置和日志保留策略。
5. 增加跨平台文件路径规范化。
6. 增加前端 IPC/API 调用层。
