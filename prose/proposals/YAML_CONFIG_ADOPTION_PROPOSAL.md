# YAML Configuration Format Adoption Proposal

**Date:** 2026-01-21
**Status:** Proposed

---

## Overview

**Change:** Replace `Quill.toml` with `Quill.yaml` as the sole configuration format.

**Rationale:**
1. TOML's dot-notation is verbose for nested structures (arrays of objects, nested properties)
2. YAML enables rich IDE tooling via JSON Schema (autocomplete, validation, hover docs)
3. YAML aligns with web ecosystem (Docker, K8s, GitHub Actions, OpenAPI)

**Note on Typst:** Using `Quill.yaml` alongside `typst.toml` is standard separation of concerns. They configure different tools (Quillmark vs Typst compiler). Mixed formats are normal (e.g., `Cargo.toml` + `.github/workflows/*.yaml`).

---

## Format Comparison

**TOML (Current) - Nested structures are verbose:**
```toml
[fields.cells]
type = "array"
cells.items.type = "object"
cells.items.properties.category.type = "string"
cells.items.properties.category.required = true
cells.items.properties.skills.type = "string"
cells.items.properties.skills.required = true
```

**YAML (Proposed) - Natural hierarchy:**
```yaml
fields:
  cells:
    type: array
    items:
      type: object
      properties:
        category:
          type: string
          required: true
        skills:
          type: string
          required: true
```

**IDE Support:**
- YAML: Real-time validation, autocomplete, hover docs (via JSON Schema)
- TOML: Syntax highlighting only

**Tradeoffs:**
- YAML requires indentation (mitigated by IDE auto-formatting)
- YAML has implicit typing (`version: 1.0` = number; use `"1.0"` for strings)
- JSON Schema catches type errors immediately

---

## Example Config

```yaml
# Quill.yaml
Quill:
  name: classic_resume
  backend: typst
  version: 1.0.0
  plate_file: plate.typ

fields:
  name:
    type: string
    title: Full Name
    required: true

  contacts:
    type: array
    items:
      type: string
    minItems: 1

cards:
  experience_section:
    title: Experience Entry
    fields:
      company:
        type: string
        required: true
      role:
        type: string
      dates:
        type: string
        pattern: '^[A-Z][a-z]+ [0-9]{4}( – ([A-Z][a-z]+ [0-9]{4}|Present))?$'
```

---

## Implementation Plan

**Phase 1: Core Parser**
1. Add `serde_yaml` dependency to `Cargo.toml`
2. Implement `QuillConfig::from_yaml()`
3. Update `Quill::from_tree()` to look for `Quill.yaml` instead of `Quill.toml`
4. Remove `toml` and `toml_edit` dependencies
5. Remove `QuillConfig::from_toml()` and `QuillValue::from_toml()`

**Phase 2: Tests**
1. Convert test fixtures from TOML to YAML (`tests/fixtures/**/*.toml` → `*.yaml`)
2. Update integration tests
3. Update WASM/Python binding tests

**Phase 3: Tooling**
1. Create `schema/quill-v1.schema.json` JSON Schema
2. Host at `https://quillmark.dev/schema/quill-v1.json`
3. Document IDE setup in README:
   ```json
   // .vscode/settings.json
   {
     "yaml.schemas": {
       "https://quillmark.dev/schema/quill-v1.json": ["Quill.yaml"]
     }
   }
   ```

**Phase 4: Documentation**
1. Update all code examples to YAML
2. Update error messages (`Quill.toml` → `Quill.yaml`)
3. Add migration note in release notes

---

## Migration Guide

**Quick conversion:**

```toml
# Quill.toml
[Quill]
name = "my_template"
backend = "typst"

[fields.title]
type = "string"
required = true
```

```yaml
# Quill.yaml
Quill:
  name: my_template
  backend: typst

fields:
  title:
    type: string
    required: true
```

**Key changes:**
- `[section]` → `section:`
- `key = "value"` → `key: value`
- Remove quotes from strings (unless needed for special chars or type disambiguation)
- Use 2-space indentation for nesting

**Note:** Pre-1.0 breaking change. TOML support removed in this release.

---

## Success Criteria

- [ ] YAML parser functional
- [ ] All tests pass with YAML configs
- [ ] JSON Schema published
- [ ] IDE validation working (VSCode, IntelliJ)
- [ ] Documentation updated (no TOML references)
- [ ] TOML dependencies removed from `Cargo.toml`
