# ADR: Core backend boundaries

## 背景

Speed-On 的核心目标是记录用户打开软件、文件、文件夹和浏览器地址的行为，并根据历史使用数据给前端提供推荐结果。项目需要支持 Windows、macOS、Linux 三个 PC 桌面平台，并覆盖 x86_64 与 ARM64/aarch64 架构。

这些平台在应用扫描、图标提取、系统日志、文件活动监听、浏览器历史读取和权限边界上差异很大。如果把平台实现直接写进业务层，后续会导致高耦合、难测试和临时 hotfix 腐化。

## 决策

后端 Core 采用以下边界：

1. `domain` 只包含平台无关的业务模型。
2. `ports` 定义平台扫描、浏览器历史、文件活动和资源存储接口。
3. `service` 只编排索引与推荐业务逻辑，依赖抽象接口。
4. `storage/schema` 固定 SQLite 初始 schema 契约。
5. Windows、macOS、Linux 的具体实现后续必须放在 adapter/provider/gateway 层。
6. 前端暂不实现，后续通过稳定 Core API 或 IPC 绑定接入。

## 原因

- 保持高模块化、低耦合。
- 避免平台差异污染推荐算法和领域模型。
- 允许用内存 repository 进行 TDD 测试。
- SQLite 实现可以独立演进，不影响推荐服务接口。
- 后续新增平台能力时，可以单独测试和回滚。

## 替代方案

### 方案 A：直接实现所有平台逻辑

优点是短期可见功能更多，但会在项目早期形成强耦合，尤其容易把权限、路径解析、日志监听和推荐算法混在一起。

### 方案 B：先写前端再补后端

前端可以更早看到效果，但会导致接口不稳定，后续 Core 改动会频繁破坏 UI。

### 方案 C：先固定 Core 边界和测试

这是当前选择。短期功能较克制，但更适合长期由 Codex/AI Agent 持续迭代。

## 影响范围

- 新增 Rust workspace。
- 新增 `speed_on_core` crate。
- 新增领域模型、端口接口、推荐服务和 SQLite schema。
- 后续平台实现必须遵守本 ADR 边界。

## 风险

- 第一阶段没有真实扫描系统软件，也没有真实监听系统日志。
- SQLite repository 还没有实现，仅固定 schema 和接口。
- 浏览器历史读取涉及隐私和浏览器文件锁，后续必须单独设计权限与脱敏策略。

## 未来演进

1. 增加 SQLite repository 实现和迁移执行器。
2. 增加 Windows 应用扫描 adapter。
3. 增加 macOS 应用扫描 adapter。
4. 增加 Linux desktop entry 扫描 adapter。
5. 增加文件/文件夹活动监听。
6. 增加浏览器历史 reader。
7. 增加前端可调用 API / IPC 层。
8. 增加跨平台集成测试和 CI。
