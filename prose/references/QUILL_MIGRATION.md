# Quill Template Migration Guide

> **Version**: 0.30.0 (Versioning System)

This guide covers changes for Quill template developers.

## Required Change: Add `version` Field

All Quill.toml files must now include a `version` field:

```diff
  [Quill]
  name = "my_template"
+ version = "1.0"
  backend = "typst"
  description = "My template description"
```

**Without this field, registration will fail:**
```
Error: Missing required field 'version' in Quill.toml
```

## Version Format

Two-segment version: `MAJOR.MINOR`

| Increment | When |
|-----------|------|
| **MAJOR** | Breaking changes (layout, removed fields, incompatible types) |
| **MINOR** | Compatible changes (bug fixes, new optional fields) |

## Choosing Initial Version

| Status | Version | Notes |
|--------|---------|-------|
| Pre-release/experimental | `0.1` | Breaking changes allowed between minors |
| Production-ready | `1.0` | Standard semantic expectations |
| Stable/mature | `2.0+` | Use current major |

## Quill Name Requirements

Names must now follow strict format: `[a-z_][a-z0-9_]*`

```diff
  [Quill]
- name = "my-template"     # ❌ Hyphens not allowed
+ name = "my_template"     # ✅ Underscores only
```

**Valid examples:** `resume`, `usaf_memo`, `_private`, `template2`

**Invalid examples:** `My-Template`, `resume-v2`, `template.name`

## Document Version Syntax

Documents can now pin template versions:

```yaml
---
QUILL: my_template@2.1      # Exact version
QUILL: my_template@2        # Latest 2.x
QUILL: my_template@latest   # Latest (explicit)
QUILL: my_template          # Latest (default, unchanged)
---
```

## Supporting Multiple Versions

To maintain multiple versions, organize by version directory:

```
templates/
  my_template/
    v1.0/
      Quill.toml    # version = "1.0"
      plate.typ
    v2.0/
      Quill.toml    # version = "2.0"
      plate.typ
```

Register all versions with the engine—they coexist under the same name.

## Migration Checklist

- [ ] Add `version = "X.Y"` to all Quill.toml files
- [ ] Rename templates using hyphens to underscores
- [ ] Update any references in documentation/examples
- [ ] Test with `@version` syntax in sample documents

## Related Documentation

- [VERSIONING.md](../../../prose/designs/VERSIONING.md) - Full versioning design
- [QUILL.md](../../../prose/designs/QUILL.md) - Quill structure reference
