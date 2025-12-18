# Quill.toml Configuration Enhancement Proposal
## Support for Scope Schema Annotations (Collections)

**Date:** 2025-12-16 (Updated)
**Context:** Enable schema annotation for SCOPE blocks in quills (e.g., `indorsements`)
**Design Focus:** Ergonomic, scalable, implementation-agnostic

---

## Problem Statement

Currently, Quill.toml supports schema annotation only for main document fields via `[fields.*]` sections. However, many quills use the SCOPE feature to create **Collections** (arrays of objects), such as:
   - **Endorsements** (`indorsements`) in USAF/USSF memos
   - **Products** in catalogs
   - **Authors** in multi-author documents
   - **Sections** in structured reports

These scoped blocks have their own field schemas, but there's currently no way to:

1. ✗ Define validation rules for scope fields
2. ✗ Specify default values for scope fields
3. ✗ Provide UI metadata (groups, ordering) for scope wizards
4. ✗ Generate JSON Schema for scopes
5. ✗ Auto-document available fields for scopes

---

## Final Design Summary

**Chosen Approach:** `[scopes.*]` section for Collections

### Core Semantics
- **Section name**: `[scopes.*]`
- **Behavior**: All scopes defined in `[scopes.*]` are treated as **Collections** (arrays of objects).
- **Multiple Blocks**: Multiple SCOPE blocks with the same name in Markdown become items in the array.
- **Unknown scopes**: Allowed (lenient mode) - defaults to collection behavior (array of objects with unvalidated fields).
- **Collection constraints**: No `min_items` or `max_items` requirement supported in v1.

### Quick Example
```toml
# Collection scope: Multiple blocks → array
[scopes.indorsements]
description = "Endorsements"

[scopes.indorsements.fields.from]
type = "string"
```

---

## Behavior Rules

### 1. Default Behavior (No Schema Defined)

**Markdown:**
```markdown
---
SCOPE: unknown_scope
foo: bar
---
Content
```

**Behavior:**
- ✅ Allowed (lenient mode)
- Defaults to **collection scope** (array)
- No validation applied
- Result: `unknown_scope = [{foo: "bar", BODY: "Content"}]`

### 2. Collection Scope (Array)

**TOML:**
```toml
[scopes.products]
description = "Product listings"

[scopes.products.fields.name]
type = "string"

[scopes.products.fields.price]
type = "number"
```

**Markdown:**
```markdown
---
SCOPE: products
name: Widget
price: 19.99
---
Widget description

---
SCOPE: products
name: Gadget
price: 29.99
---
Gadget description
```

**Result:**
```json
{
  "products": [
    {"name": "Widget", "price": 19.99, "BODY": "Widget description"},
    {"name": "Gadget", "price": 29.99, "BODY": "Gadget description"}
  ]
}
```

**Edge Cases:**
- ✅ **Zero blocks**: `products = []` (empty array, valid)
- ✅ **One block**: `products = [{...}]` (array with one item)
- ✅ **Many blocks**: `products = [{...}, {...}, ...]` (array with N items)

---

## Validation Rules

**Collection Scopes:**
- ✅ Validate each item in the array against the field schema defined in `[scopes.X.fields.*]`
- ✅ Apply defaults to each item
- ❌ No `min_items` / `max_items` constraint (not supported in v1)

---

## Complete Example: USAF Memo

```toml
[Quill]
name = "usaf_memo"
backend = "typst"
description = "USAF Official Memorandum with endorsements"

# ===========================================
# MAIN DOCUMENT FIELDS
# ===========================================

[fields.subject]
type = "string"
title = "Subject line"
ui.group = "Essentials"
description = "Brief, clear subject"

[fields.memo_for]
type = "array"
title = "Recipient organization(s)"
ui.group = "Essentials"

# ===========================================
# SCOPES (COLLECTIONS)
# ===========================================

# Collection scope: Multiple endorsements
[scopes.indorsements]
description = "Endorsements (1st Ind, 2d Ind, etc.) per AFH 33-337"
ui.group = "Endorsements"
ui.icon = "mail-forward"
ui.add_button_text = "Add Endorsement"
ui.item_label = "{{ordinal}} Indorsement"

[scopes.indorsements.fields.from]
title = "Endorsing organization"
type = "string"
examples = ["HQ USAF/A1"]
ui.group = "Header"

[scopes.indorsements.fields.to]
title = "Recipient organization"
type = "string"
examples = ["INSTALLATION/CC"]
ui.group = "Header"

[scopes.indorsements.fields.signature_block]
title = "Signature block"
type = "array"
examples = [["JANE DOE, Col, USAF", "Commander"]]
ui.group = "Signature"

[scopes.indorsements.fields.new_page]
title = "Start on new page"
type = "boolean"
default = false
ui.group = "Formatting"
```

**Markdown Usage:**
```markdown
---
QUILL: usaf_memo
subject: Request for Equipment Authorization
memo_for: ["INSTALLATION/CC"]
---

# Request Background

This memo requests authorization for...

---
SCOPE: indorsements
from: INSTALLATION/CC
to: SQUADRON/CC
signature_block: ["JANE SMITH, Col, USAF", "Installation Commander"]
new_page: true
---

I approve this request and forward to MAJCOM/CC.

---
SCOPE: indorsements
from: MAJCOM/CC
to: INSTALLATION/CC
signature_block: ["ROBERT JONES, Brig Gen, USAF", "Commander"]
---

Request approved.
```

**Parsed Result:**
```json
{
  "subject": "Request for Equipment Authorization",
  "memo_for": ["INSTALLATION/CC"],
  "BODY": "# Request Background\n\nThis memo requests authorization for...",

  "indorsements": [
    {
      "from": "INSTALLATION/CC",
      "to": "SQUADRON/CC",
      "signature_block": ["JANE SMITH, Col, USAF", "Installation Commander"],
      "new_page": true,
      "BODY": "I approve this request and forward to MAJCOM/CC."
    },
    {
      "from": "MAJCOM/CC",
      "to": "INSTALLATION/CC",
      "signature_block": ["ROBERT JONES, Brig Gen, USAF", "Commander"],
      "new_page": false,
      "BODY": "Request approved."
    }
  ]
}
```

---

## JSON Schema Generation

### Collection Scope Output
```toml
[scopes.indorsements]
description = "Endorsements"
```

→

```json
{
  "indorsements": {
    "type": "array",
    "description": "Endorsements",
    "items": {
      "type": "object",
      "properties": {
        "from": { "type": "string" },
        "to": { "type": "string" }
      }
    }
  }
}
```

---

## Implementation Notes

### Parser Changes

1. **Load `[scopes.*]` from Quill.toml**
   - Build field schemas for each scope.

2. **During Markdown Parsing**
   - Group SCOPE blocks by name.
   - Always accumulate blocks into an array (for now, as singular scopes are not supported).
   - Build result: `name = [ {fields...}, {fields...} ]`

3. **Validation**
   - Validate collection items against `scopes.X.fields.*`.
   - Apply defaults.

### JSON Schema Generation

```rust
fn generate_scope_schema(scope_config: &ScopeConfig) -> JsonValue {
    let item_schema = build_object_schema(&scope_config.fields);

    // Collection scope: type = array
    json!({
        "type": "array",
        "description": scope_config.description,
        "items": item_schema,
        "x-ui": build_ui_metadata(&scope_config.ui)
    })
}
```

---

## Migration Path

### Phase 1: Rename `[collections]` → `[scopes]`
- Adopt `[scopes.*]` naming convention immediately.
- Default behavior: collection (array).

### Phase 2: Update existing quills
- Migrate quills (like USAF memo) to use `[scopes.indorsements]`.
