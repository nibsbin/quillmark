# Quill YAML Configuration Migration

> **Version**: 0.31.0
> **Status**: Required

This guide covers the mandatory migration from `Quill.toml` to `Quill.yaml` for all Quill template configurations.

## Overview

We have replaced TOML with YAML to support richer validation, better IDE tooling (via JSON Schema), and standard web ecosystem practices.

**Deadline**: `Quill.toml` support is removed in version 0.31.0. You must migrate to `Quill.yaml`.

## Key Changes

### 1. File Rename
Rename your configuration file:
- ❌ `Quill.toml`
- ✅ `Quill.yaml`

### 2. Version Field Required
The `version` field is now **mandatory** in the `Quill` block.

```yaml
Quill:
  name: my_template
  version: 1.0.0  # Required
  backend: typst
```

### 3. Structural Syntax
Replace TOML's dotted keys with YAML's nested hierarchy.

**Before (TOML):**
```toml
[fields.author]
type = "string"
ui.order = 1
```

**After (YAML):**
```yaml
fields:
  author:
    type: string
    ui:
      order: 1
```

### 4. Implicit Field Ordering
In `Quill.yaml`, the order of fields in the file **determines the UI order**. You generally no longer need `ui.order` unless you want to force a specific sort order different from the file structure.

## Migration Example

### Before: `Quill.toml`

```toml
[Quill]
name = "classic_resume"
backend = "typst"
description = "A classic resume template"

[fields.name]
type = "string"
required = true

[fields.experience]
type = "array"
items.type = "object"
items.properties.company.type = "string"
```

### After: `Quill.yaml`

```yaml
Quill:
  name: classic_resume
  version: 1.0.0
  backend: typst
  description: A classic resume template

fields:
  name:
    type: string
    required: true

  experience:
    type: array
    items:
      type: object
      properties:
        company:
          type: string
```

## IDE Setup

To get autocomplete and validation in VS Code:

1. Install the [YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml).
2. Add this to your `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "https://quillmark.dev/schema/quill-v1.json": ["Quill.yaml"]
  }
}
```
