# Quillmark AI Agent Instructions

## Architecture

**Template-first Markdown rendering**: Parse (Markdown + YAML) → Template (MiniJinja) → Compile (backend glue) → Artifacts (PDF/SVG)

**Workspace structure** (publish order `core` → `typst` → `quillmark`, synchronized versions):
- `quillmark-core/` - Foundation (parsing, templating, Backend trait)
- `quillmark-typst/` - Typst backend implementation
- `quillmark/` - Orchestration layer (Quillmark engine and workflows)
- `quillmark-fixtures/` - Test resources (not published)

See `designs/DESIGN.md` for complete architecture.

## Documentation Pattern (Critical)

**Hybrid strategy** - minimal inline docs + external markdown via `#[doc = include_str!("../docs/{module}.md")]`

**Gotcha**: Intra-doc links MUST use module-qualified paths:
- ✅ `` [`compile::compile_to_pdf()`] ``
- ❌ `` [`compile_to_pdf`] `` (breaks when included in lib.rs)

Always run `cargo doc --no-deps` after doc changes.

## Build & Test

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo doc --no-deps --workspace --all-features
cargo run --example appreciated_letter
```

Tests: unit (in-file), integration (`tests/*.rs` with `common.rs`), doc (external `.md`), examples (use fixtures).

## Extended YAML Metadata (Non-Obvious)

Supports **inline metadata sections** with tag directives:

```markdown
---
!products
name: Widget
---
Description here.
```

Creates `products` array with metadata + `body` field. Rules:
- Tag `!name` on first line after `---`
- Blocks must be contiguous (no blank lines)
- `---` + blank line = horizontal rule (NOT metadata)

See `quillmark-core/docs/designs/PARSE.md`.

## Filter API (Stable Abstraction)

**Never import MiniJinja directly** - use `quillmark_core::templating::filter_api`:

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
```

Common filters: `String`, `Lines`, `Date`, `Dict`, `Content`, `Asset` (prefixed with `DYNAMIC_ASSET__`).

## Backend Implementation

Implement `Backend` trait (see `designs/DESIGN.md`). Typst gotchas:
- Escape `* _ # $ @ [ ] < > \`` in text
- Convert markdown `-` to Typst `+` for lists
- Virtual paths: use forward slashes, not `Path::join()`
- Fonts: `assets/fonts/` → `assets/` → system (lazy load via `typst-kit`)

## Error Handling

**Use `Diagnostic` everywhere** - never stringify prematurely:

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub primary: Option<Location>, // File, line, column
    pub hint: Option<String>,
}
```

Map external errors (MiniJinja, Typst) to preserve context.

## Releases

```bash
cargo release minor              # Preview
cargo release minor --execute    # Execute
```

All crates share version (automated via `release.toml`). See `designs/CI_CD.md`.

## Common Pitfalls

1. Doc links without module qualification
2. Virtual paths with backslashes or `Path::join()`
3. Missing character escaping for Typst
4. Non-contiguous metadata blocks
5. Hardcoded fixture paths
6. Unsynchronized versions
7. Direct MiniJinja imports

## Reference

- `designs/` - Complete architecture, workflows, specs
- `CONTRIBUTING.md` - Documentation standards
- `release.toml` - Release configuration
