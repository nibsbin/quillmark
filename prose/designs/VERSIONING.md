# Quill Versioning System

> **Status**: Implemented
> **Implementation**: `crates/core/src/version.rs`, `crates/quillmark/src/orchestration/engine.rs`

## TL;DR

Quill templates support two-segment versioning (`MAJOR.MINOR`). Documents specify versions in QUILL tags with `@` syntax. The engine maintains a version registry and resolves version selectors at workflow creation time.

## When to Use

- **Template authors**: Bump version in `Quill.toml` when releasing changes
- **Document authors**: Pin versions in QUILL tags for reproducibility
- **Engine consumers**: Register multiple versions of the same template

## Version Format

Two-segment versioning: `MAJOR.MINOR`

| Increment | When |
|-----------|------|
| **MAJOR** | Breaking changes: layout changes, removed fields, incompatible types |
| **MINOR** | Compatible changes: bug fixes, new optional fields, improvements |

### Pre-1.0 Versioning

Versions below `1.0` (e.g., `0.1`, `0.2`) indicate **pre-release** Quills that are still in development. Pre-1.0 guidelines:

- **Start at `0.1`** for new Quills under development
- **Increment minor only** (e.g., `0.1` → `0.2` → `0.3`) during pre-release
- **Do not increment major** until the Quill is production-ready
- **Graduate to `1.0`** when the Quill is stable and ready for production use

Pre-1.0 Quills may have breaking changes between any minor version. Document authors should pin to exact versions (e.g., `@0.2`) for stability during this phase.

## Document Syntax

```yaml
---
QUILL: "template@2.1"      # Exact version
QUILL: "template@2"        # Latest 2.x
QUILL: "template@latest"   # Latest overall (explicit)
QUILL: "template"          # Latest overall (default)
---
```

## Resolution Semantics

Given versions `[1.0, 1.1, 2.0, 2.1, 2.2, 3.0]`:

| Selector | Resolves To |
|----------|-------------|
| `@3` | `3.0` (latest 3.x) |
| `@2` | `2.2` (latest 2.x) |
| `@2.1` | `2.1` (exact match) |
| `@latest` | `3.0` (highest overall) |
| (none) | `3.0` (highest overall) |

## Template Requirements

`Quill.toml` must include a `version` field:

```toml
[Quill]
name = "my_template"
version = "2.1"           # Required
backend = "typst"
description = "..."
```

## Error Handling

Version errors provide actionable diagnostics:

```
Error: Version not found
  Template: resume_template
  Requested: @2.3
  Available: 3.0, 2.2, 2.1, 2.0, 1.0

  Suggestion: Use @2 for latest 2.x (currently 2.2)
```

See [ERROR.md](ERROR.md) for error handling patterns.

## Links

- **Quill structure**: [QUILL.md](QUILL.md)
- **Parsing**: [PARSE.md](PARSE.md) (QUILL tag extraction)
- **Error patterns**: [ERROR.md](ERROR.md)
