# pub-minifier

`pub-minifier` 是一个基于 `rustc_private` 的静态分析工具，用于遍历 crate 的 HIR，统计“模块内 item 的可达性使用情况（reachability）”以及对应源码 `span`，并输出结构化 JSON。

## 功能概览

- 递归遍历 crate 中的所有模块。
- 对每个模块记录 item 定义（`Definition`）。
- 在 item 内部遍历 HIR，记录以下使用情景：
  - `Import` / `Export`：来自 `use` / `pub use`
  - `TypeAnnotation`：类型位置路径引用
  - `Call`：函数调用与方法调用
  - `Construct`：结构体/构造器使用
- 为每个使用情景收集对应源码位置（`Span`）。
- 以稳定排序的 JSON 输出分析结果。

## 输出结构

输出是一个模块列表，每个模块包含：

- `level`：模块层级（根模块为 1）
- `name`：模块路径
- `parent_mod`：父模块路径
- `items`：该模块内出现的 item 使用信息

每个 `items` 条目包含：

- `item`：被使用 item 的完整路径
- `usages`：不同 reachability 分类
  - `reachability`：如 `Definition` / `Call` / `TypeAnnotation` 等
  - `spans`：该分类下出现位置列表

## 实现思路

### 1. 模块级聚合（`src/collector.rs`）

- 从 crate root 模块开始递归遍历。
- 为每个模块维护 `FxHashMap<DefId, ItemUsage>`。
- 每个 item 先记录一次 `Definition`。
- 再调用 `reachability::collect_item_usages` 获取该 item 内的命中，并按模块归档。
- 最后统一转换成 `OutModule` 结构并排序输出，保证结果稳定。

### 2. HIR 可达性采集（`src/reachability.rs`）

- 实现一个 `intravisit::Visitor`，在 item 作用域内扫描表达式、类型和路径。
- `ItemKind::Use` 单独处理导入/导出。
- `visit_ty` 识别 `TypeAnnotation`。
- `visit_expr` 识别 `Call`（含方法调用）与 `Construct`。
- `visit_path` 补充捕获构造器路径场景。
- 将每次命中封装为 `UsageHit { item, reachability, span }` 返回给聚合层。

### 3. 输出与格式化（`src/out.rs`）

- `def_path_str` 负责把 `DefId` 转为稳定的全路径字符串，并做缓存。
- `span_to_string` 将 `Span` 转为可读位置。
- 最终通过 `serde_json::to_writer_pretty` 打印 JSON。

## 运行方式

该工具依赖 nightly 和 `rustc_private`。

```bash
cargo run -- <RUSTC_ARGS...>
```

例如（分析某个 Rust 文件）：

```bash
cargo run -- tests/ui/modules.rs
```

## 测试

```bash
cargo test
```

当前仓库使用 `compiletest` 做 UI 快照测试，输出变化可通过更新 `tests/ui/*.stdout` 快照同步。
