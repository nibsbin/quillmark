# Quill Versioning System

> **Status**: Implemented
> **Implementation**: `crates/core/src/version.rs`, `crates/quillmark/src/orchestration/engine.rs`

## TL;DR

Quill templates support semantic versioning (`MAJOR.MINOR.PATCH`). Documents specify versions in QUILL tags with `@` syntax. The engine maintains a version registry and resolves version selectors at workflow creation time.

## When to Use

- **Template authors**: Bump version in `Quill.toml` when releasing changes
- **Document authors**: Pin versions in QUILL tags for reproducibility
- **Engine consumers**: Register multiple versions of the same template

## Version Format

Semantic versioning: `MAJOR.MINOR.PATCH`

| Increment | When |
|-----------|------|
| **MAJOR** | Breaking changes: layout changes, removed fields, incompatible types |
| **MINOR** | New features: new optional fields, enhancements (backward-compatible) |
| **PATCH** | Bug fixes: corrections, improvements (backward-compatible) |

### Pre-1.0 Versioning

Versions below `1.0.0` (e.g., `0.1.0`, `0.2.0`) indicate **pre-release** Quills that are still in development. Pre-1.0 guidelines:

- **Start at `0.1.0`** for new Quills under development
- **Increment minor** (e.g., `0.1.0` → `0.2.0` → `0.3.0`) for new features during pre-release
- **Increment patch** (e.g., `0.1.0` → `0.1.1`) for bug fixes during pre-release
- **Do not increment major** until the Quill is production-ready
- **Graduate to `1.0.0`** when the Quill is stable and ready for production use

Pre-1.0 Quills may have breaking changes between any minor version. Document authors should pin to exact versions (e.g., `@0.2.0`) for stability during this phase.

## Document Syntax

```yaml
---
QUILL: "template@2.1.0"    # Exact version
QUILL: "template@2.1"      # Latest 2.1.x (minor selector)
QUILL: "template@2"        # Latest 2.x.x (major selector)
QUILL: "template@latest"   # Latest overall (explicit)
QUILL: "template"          # Latest overall (default)
---
```

## Resolution Semantics

Given versions `[1.0.0, 1.0.1, 1.1.0, 2.0.0, 2.1.0, 2.1.1, 3.0.0]`:

| Selector | Resolves To |
|----------|-------------|
| `@3` | `3.0.0` (latest 3.x.x) |
| `@2` | `2.1.1` (latest 2.x.x) |
| `@2.1` | `2.1.1` (latest 2.1.x) |
| `@2.1.0` | `2.1.0` (exact match) |
| `@latest` | `3.0.0` (highest overall) |
| (none) | `3.0.0` (highest overall) |

## Template Requirements

`Quill.toml` must include a `version` field:

```toml
[Quill]
name = "my_template"
version = "2.1.0"         # Required (semver format)
backend = "typst"
description = "..."
```

Note: Two-segment versions (e.g., `"2.1"`) are also supported for backward compatibility and will be interpreted as `"2.1.0"`.

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

## Usage Examples

### Registering Multiple Versions

```rust
use quillmark::Quillmark;
use quillmark_core::Quill;

let mut engine = Quillmark::new();

// Load different versions of the same template
let resume_v1 = Quill::from_path("templates/resume/v1.0")?;
let resume_v2 = Quill::from_path("templates/resume/v2.0")?;
let resume_v2_1 = Quill::from_path("templates/resume/v2.1")?;

// All versions coexist in the registry
engine.register_quill(resume_v1)?;
engine.register_quill(resume_v2)?;
engine.register_quill(resume_v2_1)?;
```

### Creating Workflows with Version Syntax

```rust
// Exact version
let workflow = engine.workflow("resume_template@2.1.0")?;

// Latest in minor version series
let workflow = engine.workflow("resume_template@2.1")?;

// Latest in major version
let workflow = engine.workflow("resume_template@2")?;

// Latest overall (implicit)
let workflow = engine.workflow("resume_template")?;

// Latest overall (explicit)
let workflow = engine.workflow("resume_template@latest")?;
```

### Parsing Versioned Documents

```rust
use quillmark_core::ParsedDocument;

let markdown = r#"
---
QUILL: resume_template@2.1.0
name: John Doe
email: john@example.com
---
# Professional Experience
..."#;

let doc = ParsedDocument::from_markdown(markdown)?;
let workflow = engine.workflow(&doc)?;

// The workflow uses resume_template version 2.1.0
```

### Example Documents

See fixture examples in `crates/fixtures/resources/`:

**Exact version pinning** (`versioned_resume_exact.md`):
```markdown
---
QUILL: classic_resume@2.1.0
name: John Doe
---
# Resume content
```

**Minor version selector** (`versioned_resume_minor.md`):
```markdown
---
QUILL: classic_resume@2.1
name: Jane Smith
---
# Resume content
```

**Major version selector** (`versioned_resume_major.md`):
```markdown
---
QUILL: classic_resume@2
name: Jane Smith
---
# Resume content
```

**Explicit latest** (`versioned_letter_latest.md`):
```markdown
---
QUILL: business_letter@latest
---
# Letter content
```

## Migration Guide

### Adding Versions to Existing Quills

#### Step 1: Add Version Field to Quill.toml

Edit your `Quill.toml`:

```toml
[Quill]
name = "my_template"
version = "1.0.0"            # Add this field (semver format)
backend = "typst"
description = "..."
```

**Choosing the initial version:**
- **Pre-release/experimental**: Start at `0.1.0`
- **Production-ready**: Start at `1.0.0`
- **Stable/mature**: Use current major version (e.g., `2.0.0`)

#### Step 2: Update Registration Code

Before (single version):
```rust
let quill = Quill::from_path("templates/my_template")?;
engine.register_quill(quill)?;
```

After (still works the same):
```rust
let quill = Quill::from_path("templates/my_template")?;
engine.register_quill(quill)?;  // Now version 1.0.0 is registered
```

#### Step 3: Supporting Multiple Versions

To support multiple versions, organize by version:

```
templates/
  my_template/
    v1.0.0/
      Quill.toml    # version = "1.0.0"
      plate.typ
    v1.1.0/
      Quill.toml    # version = "1.1.0"
      plate.typ
    v2.0.0/
      Quill.toml    # version = "2.0.0"
      plate.typ
```

Register all versions:
```rust
for version in ["1.0.0", "1.1.0", "2.0.0"] {
    let path = format!("templates/my_template/v{}", version);
    let quill = Quill::from_path(path)?;
    engine.register_quill(quill)?;
}
```

#### Step 4: Pinning Existing Documents

Existing documents without version syntax continue to work (they use latest):

```markdown
---
QUILL: my_template    # Uses latest version
---
```

To pin to a specific version, add `@version`:

```markdown
---
QUILL: my_template@1.0.0    # Pinned to 1.0.0
---
```

To allow patch updates within a minor version:

```markdown
---
QUILL: my_template@2.1    # Uses latest 2.1.x
---
```

To allow minor/patch updates within a major version:

```markdown
---
QUILL: my_template@2    # Uses latest 2.x.x
---
```

### Version Bumping Workflow

When releasing a new version of your template:

1. **Decide increment type:**
   - Breaking change? Increment MAJOR (e.g., `2.1.0` → `3.0.0`)
   - New feature? Increment MINOR (e.g., `2.1.0` → `2.2.0`)
   - Bug fix? Increment PATCH (e.g., `2.1.0` → `2.1.1`)

2. **Create new version directory:**
   ```bash
   cp -r templates/my_template/v2.1.0 templates/my_template/v2.1.1
   ```

3. **Update Quill.toml:**
   ```toml
   version = "2.1.1"  # Update version number
   ```

4. **Register the new version:**
   ```rust
   let new_version = Quill::from_path("templates/my_template/v2.1.1")?;
   engine.register_quill(new_version)?;
   ```

5. **Document authors update at their own pace:**
   - Documents with `@2.1` automatically get `2.1.1`
   - Documents with `@2.1.0` stay on `2.1.0`
   - Documents with `@2` get `2.1.1` (latest 2.x.x)
   - Documents with no version get `2.1.1` (latest)

### Backward Compatibility

The versioning system maintains backward compatibility:

- **Unversioned Quill.toml**: Error - `version` field is now required
- **Unversioned QUILL tags**: Work fine - resolve to latest version
- **Old documents**: Continue working without modification

## Links

- **Quill structure**: [QUILL.md](QUILL.md)
- **Parsing**: [PARSE.md](PARSE.md) (QUILL tag extraction)
- **Error patterns**: [ERROR.md](ERROR.md)
- **Implementation plan**: [../plans/completed/QUILL_VERSIONING_IMPLEMENTATION.md](../plans/completed/QUILL_VERSIONING_IMPLEMENTATION.md)
- **Completion summary**: [../plans/completed/VERSIONING_COMPLETION_SUMMARY.md](../plans/completed/VERSIONING_COMPLETION_SUMMARY.md)
