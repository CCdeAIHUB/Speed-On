# ADR: search alias builder after application refresh

## 背景

`refresh_applications` 已经可以把平台扫描到的应用写入 `indexed_resources`。为了让这些新应用立即可以被搜索，需要在刷新后同步写入搜索别名。

## 决策

1. 新增 `alias` 模块。
2. 新增 `SearchAliasBuilder`。
3. 第一版为资源生成 `title` 和 `target` 两类 alias。
4. 新增 `PinyinAliasProvider` 边界，后续真实拼音转换通过该 provider 接入。
5. 新增 `SearchAliasRepository` port。
6. `SqliteStore` 实现 `SearchAliasRepository`。
7. `CoreApi::refresh_applications_with` 在资源写入成功后生成并写入 alias。
8. `ApiRefreshApplicationsResponse` 增加 `alias_count`。
9. alias 写入失败时返回结构化错误，不沉默失败。

## 原因

- 应用刷新后应立即可搜索。
- Core 不应依赖 SQLite 表细节。
- 真实拼音转换涉及依赖选择、字典质量和多音字问题，后续单独评估。
- `alias_count` 能帮助前端和测试确认刷新是否建立了搜索索引。

## 替代方案

### 方案 A：只写资源表

实现简单，但扫描后的应用无法立即通过搜索找到。

### 方案 B：直接引入拼音依赖

体验更完整，但依赖风险需要单独评估。

### 方案 C：先做 alias builder 和 provider 边界

这是当前选择。先补齐搜索闭环，再扩展真实拼音转换。

## 影响范围

- `crates/speed_on_core/src/alias.rs`
- `crates/speed_on_core/src/ports.rs`
- `crates/speed_on_core/src/storage/search_alias_repository.rs`
- `crates/speed_on_core/src/api.rs`
- API / IPC / README / protocol 文档

## 风险

- 当前默认 provider 不生成真实中文拼音 alias。
- alias 数量随资源数量增加，后续需要批量写入优化。
- 后续需要 alias 清理策略。

## 未来演进

1. 增加真实拼音 provider。
2. 增加浏览器标题 alias builder。
3. 增加文件和文件夹 alias builder。
4. 增加批量写入和性能测试。
5. 增加旧 alias 清理策略。
