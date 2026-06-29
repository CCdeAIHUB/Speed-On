# ADR: search alias builder after application refresh

## 背景

`refresh_applications` 已经可以把平台扫描到的应用写入 `indexed_resources`。为了让这些新应用立即可以被搜索，需要在刷新后同步写入搜索别名。

## 决策

1. 新增 `alias` 模块。
2. 新增 `SearchAliasBuilder`。
3. 为资源生成 `title`、`target`、`pinyin_full` 和 `pinyin_initials` alias。
4. 新增 `PinyinAliasProvider` 边界。
5. 新增 `PinyinCrateAliasProvider`，基于 Rust `pinyin` crate 生成无声调全拼和首字母。
6. 新增 `SearchAliasRepository` port。
7. `SqliteStore` 实现 `SearchAliasRepository`。
8. `CoreApi::refresh_applications_with` 在资源写入成功后生成并写入 alias。
9. `ApiRefreshApplicationsResponse` 增加 `alias_count`。
10. alias 写入失败时返回结构化错误，不沉默失败。

## 原因

- 应用刷新后应立即可搜索。
- 中文应用标题需要支持全拼和首字母搜索。
- Core 不应依赖 SQLite 表细节。
- `pinyin` crate 当前 docs.rs 显示版本为 `0.11.0`，提供 `ToPinyin`、`plain()` 和 `first_letter()`，满足当前无声调全拼与首字母 alias 需求。
- provider 边界保留了未来替换拼音实现或处理多音字的空间。
- `alias_count` 能帮助前端和测试确认刷新是否建立了搜索索引。

## 替代方案

### 方案 A：只写资源表

实现简单，但扫描后的应用无法立即通过搜索找到。

### 方案 B：只做 title / target alias

能解决英文标题和路径搜索，但中文拼音体验不完整。

### 方案 C：alias builder + pinyin provider + pinyin crate

这是当前选择。它补齐扫描后可搜索闭环，同时将拼音转换隔离在 provider 后。

## 影响范围

- `crates/speed_on_core/src/alias.rs`
- `crates/speed_on_core/src/pinyin_alias.rs`
- `crates/speed_on_core/src/ports.rs`
- `crates/speed_on_core/src/storage/search_alias_repository.rs`
- `crates/speed_on_core/src/api.rs`
- `crates/speed_on_core/Cargo.toml`
- API / IPC / README / protocol 文档

## 风险

- 当前 provider 使用单一默认读音，多音字准确性后续需要单独增强。
- alias 数量随资源数量增加，后续需要批量写入优化。
- 后续需要 alias 清理策略。

## 未来演进

1. 增强多音字和应用名词典。
2. 增加浏览器标题 alias builder。
3. 增加文件和文件夹 alias builder。
4. 增加批量写入和性能测试。
5. 增加旧 alias 清理策略。
