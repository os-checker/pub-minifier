# pub-minifier

`pub-minifier` is a static analysis tool built on `rustc_private`. It walks crate HIR, collects item reachability usage inside modules, records source `Span`s, and emits structured JSON.

## Features

- Recursively traverses all modules in the crate.
- Records item definitions per module (`Definition`).
- Walks HIR inside each item and collects these usage scenarios:
  - `Import` / `Export`: from `use` / `pub use`
  - `TypeAnnotation`: path usage in type positions
  - `Call`: function calls and method calls
  - `Construct`: struct/constructor usage
- Records source locations (`Span`) for each usage.
- Produces deterministically sorted JSON output.

## Output Shape

The output is a list of modules. Each module includes:

- `level`: module depth (`1` for crate root)
- `name`: module path
- `parent_mod`: parent module path
- `items`: item usage information seen in this module

Each `items` entry includes:

- `item`: fully qualified item path
- `usages`: grouped by reachability kind
  - `reachability`: e.g. `Definition`, `Call`, `TypeAnnotation`
  - `spans`: source location strings for that kind

## Implementation Overview

### 1. Module-level aggregation (`src/collector.rs`)

- Starts from crate root and recursively visits modules.
- Maintains `FxHashMap<DefId, ItemUsage>` for each module.
- Records `Definition` for each item.
- Calls `reachability::collect_item_usages` to get non-definition hits, then stores them by module.
- Converts internal state to sorted `OutModule` output.

### 2. HIR reachability collection (`src/reachability.rs`)

- Implements an `intravisit::Visitor` to scan expressions, types, and paths inside an item.
- Handles `ItemKind::Use` separately for import/export classification.
- `visit_ty`: records `TypeAnnotation`.
- `visit_expr`: records `Call` (including method calls) and `Construct`.
- `visit_path`: captures constructor-path cases not covered by `ExprKind::Struct`.
- Emits each match as `UsageHit { item, reachability, span }`.

### 3. Output and formatting (`src/out.rs`)

- `def_path_str`: resolves `DefId` to full path and caches the result.
- `span_to_string`: converts `Span` to readable location text.
- Uses `serde_json::to_writer_pretty` for final output.

## Usage

This tool requires nightly and `rustc_private`.

```bash
cargo run -- <RUSTC_ARGS...>
```

For example, analyze a Rust file:

```bash
cargo run -- tests/ui/modules.rs
```

## Tests

```bash
cargo test
```

This repository uses compiletest UI snapshots. If output changes intentionally, update `tests/ui/*.stdout` snapshots accordingly.
